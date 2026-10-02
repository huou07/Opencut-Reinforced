import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/project/rust_project_gateway.dart';
import 'package:or_app_bridge/or_app_bridge.dart' show RustLib;
import 'package:or_viewer_texture/or_viewer_texture.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(RustLib.init);

  testWidgets('Android publishes a bounded Rust preview to a Flutter surface', (
    tester,
  ) async {
    // Flutter drive installs between targets; app data outlives the cache.
    final fixture = File(
      '${Directory.systemTemp.parent.path}/files/or-android-preview-tiny.mkv',
    );
    addTearDown(() {
      if (fixture.existsSync()) fixture.deleteSync();
    });
    expect(
      fixture.existsSync(),
      isTrue,
      reason: 'Expected CI to stage the preview fixture at ${fixture.path}.',
    );
    final directory = Directory.systemTemp.createTempSync(
      'or-android-preview-',
    );
    const gateway = RustProjectGateway();
    final projectPath = '${directory.path}/android-preview.orproj';
    await File(projectPath).writeAsString(
      jsonEncode(
        _videoProject(Uri.file(fixture.path).toString(), fixture.lengthSync()),
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

    final presented = await const MethodChannel('or_viewer_texture')
        .invokeMethod<bool>('frameAvailable');
    expect(presented, isTrue);
    debugPrint(
      'ANDROID_PREVIEW_TELEMETRY frameSequence=${preview.frameSequence} '
      'dimensions=${preview.width}x${preview.height} '
      'previewError=${preview.errorCode ?? 'none'} '
      'surfacePresented=$presented revisionUnchanged=$revisionUnchanged',
    );
    expect(tester.takeException(), isNull);
  });
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
