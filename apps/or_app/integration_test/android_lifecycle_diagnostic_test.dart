import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:or_app/main.dart';
import 'package:or_app/rust_core_gateway.dart';
import 'package:or_app_bridge/or_app_bridge.dart' show RustLib;

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/project/rust_project_gateway.dart';
import 'package:or_viewer_texture/or_viewer_texture.dart';

// DIAGNOSTIC ONLY: native/product source is unchanged. Not acceptance evidence.
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(RustLib.init);
  testWidgets('native bridge diagnostics match the CLI snapshot', (
    tester,
  ) async {
    const expectedJson = String.fromEnvironment('OR_CLI_BOOTSTRAP_JSON');
    expect(expectedJson, isNotEmpty);
    final expected = jsonDecode(expectedJson) as Map<String, dynamic>;
    const coreGateway = RustCoreGateway();

    final appInfo = await coreGateway.appInfo();
    expect({
      'name': appInfo.name,
      'version': appInfo.version,
      'core_api_version': appInfo.coreApiVersion,
    }, expected['app_info']);

    final health = await coreGateway.health();
    expect({'status': health.status}, expected['health']);

    final capabilities = await coreGateway.capabilities();
    expect(
      capabilities
          .map(
            (capability) => {
              'id': capability.id,
              'version': capability.version,
            },
          )
          .toList(),
      (expected['capabilities'] as Map<String, dynamic>)['capabilities'],
    );

    await tester.pumpWidget(const OrApp(gateway: coreGateway));
    await tester.tap(find.byKey(const ValueKey('nav-settings')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('settings-section-advanced')));
    await tester.pumpAndSettle();
    expect(find.text(appInfo.name), findsWidgets);
    expect(find.text(appInfo.version), findsOneWidget);
    expect(find.text(health.status), findsOneWidget);
    for (final capability in capabilities) {
      expect(find.text(capability.id), findsOneWidget);
    }
    final versions = <String, int>{};
    for (final capability in capabilities) {
      final label = 'v${capability.version}';
      versions.update(label, (count) => count + 1, ifAbsent: () => 1);
    }
    for (final entry in versions.entries) {
      expect(find.text(entry.key), findsNWidgets(entry.value));
    }
  });
  for (var repeat = 0; repeat < 2; repeat++) {
    group('same-process preview repeat $repeat', () {
      testWidgets('Android publishes a bounded Rust preview to a Flutter surface', (
        tester,
      ) async {
        // Flutter drive installs between targets; app data outlives the cache.
        final fixture = File(
          '${Directory.systemTemp.parent.path}/files/or-android-preview-tiny.mkv',
        );
        expect(
          fixture.existsSync(),
          isTrue,
          reason:
              'Expected CI to stage the preview fixture at ${fixture.path}.',
        );
        final directory = Directory.systemTemp.createTempSync(
          'or-android-preview-',
        );
        const gateway = RustProjectGateway();
        final projectPath = '${directory.path}/android-preview.orproj';
        await File(projectPath).writeAsString(
          jsonEncode(
            _videoProject(
              Uri.file(fixture.path).toString(),
              fixture.lengthSync(),
            ),
          ),
        );
        final session = await gateway.openProject(projectPath);
        addTearDown(() async {
          await gateway.close(session, discardUnsaved: true);
          directory.deleteSync(recursive: true);
        });

        var current = await gateway.summary(session);
        final addedTrack = await gateway.addTimelineTrack(
          session,
          current,
          ProjectTimelineTrackKind.video,
        );
        expect(addedTrack.succeeded, isTrue);
        current = addedTrack.view!;
        final track = (await gateway.listTimelineTracks(session)).items.single;
        final media = (await gateway.listMediaPage(
          session,
          offset: 0,
          limit: 1,
        )).items.single;
        final insertedClip = await gateway.insertTimelineClip(
          session,
          current,
          trackId: track.trackId,
          mediaId: media.mediaId,
          timelineStart: ProjectRationalTime(BigInt.zero, 1),
          sourceStart: ProjectRationalTime(BigInt.one, 4),
          duration: ProjectRationalTime(BigInt.one, 4),
        );
        expect(insertedClip.succeeded, isTrue);

        final textureId = await OrViewerTexture.textureId();
        expect(textureId, isNotNull);
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(body: Texture(textureId: textureId!)),
          ),
        );
        await tester.pumpAndSettle();

        final initial = await gateway.summary(session);
        final preview = await gateway.previewSeek(
          session,
          ProjectRationalTime(BigInt.zero, 1),
        );
        expect(preview.frameSequence, greaterThan(BigInt.zero));
        expect(preview.width, greaterThan(0));
        expect(preview.height, greaterThan(0));
        expect(preview.errorCode, isNull);
        final revisionUnchanged =
            (await gateway.summary(session)).revision == initial.revision;
        expect(revisionUnchanged, isTrue);

        const presentationChannel = MethodChannel('or_viewer_texture');
        final presentationResults = await Future.wait([
          presentationChannel.invokeMethod<bool>('frameAvailable'),
          presentationChannel.invokeMethod<bool>('frameAvailable'),
        ]);
        final surfacePresented = presentationResults.every(
          (result) => result == true,
        );
        expect(presentationResults, everyElement(isTrue));
        expect(surfacePresented, isTrue);
        debugPrint(
          'ANDROID_PREVIEW_TELEMETRY frameSequence=${preview.frameSequence} '
          'dimensions=${preview.width}x${preview.height} '
          'previewError=${preview.errorCode ?? 'none'} '
          'surfacePresented=$surfacePresented revisionUnchanged=$revisionUnchanged',
        );
        expect(tester.takeException(), isNull);
      });
    });
  }
}

Map<String, Object?> _videoProject(String sourceUri, int fileSizeBytes) => {
  'format': 'opencut-reinforced-project',
  'schema_version': 3,
  'project': {
    'id': '01234567-89ab-4def-8123-456789abcdef',
    'revision': 0,
    'name': 'Android preview fixture',
    'media': [
      {
        'id': '22222222-2222-4222-8222-222222222222',
        'source': {'kind': 'local_file', 'uri': sourceUri},
        'metadata': {
          'format_names': ['matroska'],
          'duration': {'numerator': 1, 'denominator': 1},
          'file_size_bytes': fileSizeBytes,
          'streams': [
            {
              'kind': 'video',
              'metadata': {
                'index': 0,
                'codec_name': 'ffv1',
                'width': 16,
                'height': 16,
                'pixel_format': 'bgra',
                'average_frame_rate': {'numerator': 4, 'denominator': 1},
                'duration': {'numerator': 1, 'denominator': 1},
              },
            },
          ],
        },
      },
    ],
    'timeline': {'tracks': []},
  },
};
