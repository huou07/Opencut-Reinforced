import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:or_app/main.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/project/rust_project_gateway.dart';
import 'package:or_app/rust_core_gateway.dart';
import 'package:or_app_bridge/or_app_bridge.dart' as rust;
import 'package:or_viewer_texture/or_viewer_texture.dart';

const _fixture = MethodChannel('or_saf_acceptance_fixture');
const _presenter = MethodChannel('or_viewer_texture');
String _source(String id) =>
    'content://dev.opencut.saffixture.documents/document/$id';

Future<Map<String, dynamic>> _control(
  String operation, {
  String? uri,
  String? projectJson,
}) async => Map<String, dynamic>.from(
  (await _fixture.invokeMapMethod<String, dynamic>('control', {
    'operation': operation,
    'uri': uri,
    'projectJson': projectJson,
  }))!,
);
Future<Map<String, int>> _resources() async =>
    (await _presenter.invokeMapMethod<String, num>('resourceSnapshot'))!
        .map((key, value) => MapEntry(key, value.toInt()));

// This counts real capabilities in OR's process, including duplicates which
// remain readable after the provider revokes an Android URI grant.
int _providerFds() {
  var count = 0;
  for (final entry in Directory('/proc/self/fd').listSync(followLinks: false)) {
    try {
      final target = Link(entry.path).targetSync();
      if (target.contains('/dev.opencut.saffixture/') &&
          target.endsWith('/tiny.mkv')) {
        count++;
      }
    } on FileSystemException {
      /* FD may have closed during the snapshot. */
    }
  }
  return count;
}

Future<String?> _bindingFailure(Future<bool> operation) async {
  try {
    expect(await operation, isTrue);
    return null;
  } on PlatformException catch (error) {
    return error.code;
  }
}

Future<void> _until(WidgetTester tester, bool Function() ready) async {
  for (var i = 0; i < 600 && !ready(); i++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
  expect(ready(), isTrue, reason: 'The real product action did not complete.');
}

Future<void> _seekUi(
  WidgetTester tester,
  _ObservedGateway gateway,
  double fraction,
) async {
  final before = gateway.seekCalls;
  final slider = find.byKey(const ValueKey('preview-scrub-ruler'));
  await tester.ensureVisible(slider);
  final rect = tester.getRect(slider);
  await tester.tapAt(
    Offset(rect.left + 24 + (rect.width - 48) * fraction, rect.center.dy),
  );
  await _until(tester, () => gateway.seekCalls > before);
}

Future<List<int>> _redTexture(
  WidgetTester tester,
  IntegrationTestWidgetsFlutterBinding binding,
  String name,
) async {
  final texture = find.byType(Texture);
  expect(texture, findsOneWidget);
  final center = tester.getCenter(texture) * tester.view.devicePixelRatio;
  await tester.pump(const Duration(milliseconds: 100));
  final bytes = await binding.takeScreenshot(name);
  final codec = await ui.instantiateImageCodec(Uint8List.fromList(bytes));
  final frame = await codec.getNextFrame();
  final image = frame.image;
  final rgba = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
  final offset = (center.dy.floor() * image.width + center.dx.floor()) * 4;
  final pixel = List.generate(4, (i) => rgba.getUint8(offset + i));
  image.dispose();
  codec.dispose();
  expect(pixel[0], greaterThanOrEqualTo(200));
  expect(pixel[1], lessThanOrEqualTo(40));
  expect(pixel[2], lessThanOrEqualTo(40));
  expect(pixel[3], 255);
  return pixel;
}

// Fixture preparation uses canonical commands. It does not replace the native
// project picker, source permission enforcement, decoder or editor controls.
Future<String> _project(Directory directory, String name, String source) async {
  final path = '${directory.path}/$name.orproj';
  await File(path).writeAsString(
    jsonEncode({
      'format': 'opencut-reinforced-project',
      'schema_version': 7,
      'project': {
        'id': '01234567-89ab-4def-8123-456789abcdef',
        'revision': 0,
        'name': name,
        'media': List.generate(
          65,
          (index) => {
            'id':
                '${(index + 1).toRadixString(16).padLeft(8, '0')}-2222-4222-8222-222222222222',
            'source': {
              'kind': 'android_saf_document_uri',
              'uri': index == 64 ? source : _source('unused-$index'),
            },
            'metadata': {
              'format_names': ['matroska'],
              'duration': {'numerator': 1, 'denominator': 1},
              'file_size_bytes': 9045,
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
        ),
        'timeline': {
          'tracks': [],
          'markers': [],
          'sequence_frame_rate': {'numerator': 4, 'denominator': 1},
        },
      },
    }),
  );
  const gateway = RustProjectGateway();
  final session = await gateway.openProject(path);
  var current = await gateway.summary(session);
  final added = await gateway.addTimelineTrack(
    session,
    current,
    ProjectTimelineTrackKind.video,
  );
  expect(added.succeeded, isTrue);
  current = added.view!;
  final track = (await gateway.listTimelineTracks(session)).items.single;
  final media = (await gateway.listMediaPage(
    session,
    offset: 64,
    limit: 1,
  )).items.single;
  expect(
    (await gateway.insertTimelineClip(
      session,
      current,
      trackId: track.trackId,
      mediaId: media.mediaId,
      timelineStart: ProjectRationalTime(BigInt.zero, 1),
      sourceStart: ProjectRationalTime(BigInt.zero, 1),
      duration: ProjectRationalTime(BigInt.one, 1),
    )).succeeded,
    isTrue,
  );
  expect((await gateway.save(session)).succeeded, isTrue);
  await gateway.close(session, discardUnsaved: false);
  return path;
}

// Every operation delegates to the shipped gateway. Observation only allows
// the test to wait for actual UI callbacks and inspect their real results.
class _ObservedGateway extends RustProjectGateway {
  ProjectSessionHandle? session;
  String? openedPath;
  ProjectPreviewState? lastPreview;
  ProjectPreviewState? playResult;
  ProjectGatewayException? seekError;
  int seekCalls = 0, playCalls = 0, playMicros = 0;
  bool closed = false;
  @override
  Future<ProjectSessionHandle> openProject(String path) async {
    openedPath = path;
    return session = await super.openProject(path);
  }

  @override
  Future<ProjectPreviewState> previewSeek(
    ProjectSessionHandle session,
    ProjectRationalTime position,
  ) async {
    seekError = null;
    try {
      return lastPreview = await super.previewSeek(session, position);
    } on ProjectGatewayException catch (error) {
      seekError = error;
      rethrow;
    } finally {
      seekCalls++;
    }
  }

  @override
  Future<ProjectPreviewState> previewPlay(ProjectSessionHandle session) async {
    final watch = Stopwatch()..start();
    try {
      return playResult = lastPreview = await super.previewPlay(session);
    } finally {
      playCalls++;
      playMicros = watch.elapsedMicroseconds;
    }
  }

  @override
  Future<ProjectPreviewState> previewPause(
    ProjectSessionHandle session,
  ) async => lastPreview = await super.previewPause(session);
  @override
  Future<ProjectPreviewState> previewTick(ProjectSessionHandle session) async =>
      lastPreview = await super.previewTick(session);
  @override
  Future<void> close(
    ProjectSessionHandle session, {
    required bool discardUnsaved,
  }) async {
    await super.close(session, discardUnsaved: discardUnsaved);
    closed = true;
  }
}

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(rust.RustLib.init);
  testWidgets(
    'Android SAF preview works through DocumentsUI and real editor controls',
    (tester) async {
      final directory = Directory.systemTemp.createTempSync('or-saf-preview-');
      addTearDown(() => directory.deleteSync(recursive: true));
      final source = _source('late65');
      final path = await _project(directory, 'SAF preview acceptance', source);
      final provider = await _control(
        'seedProject',
        projectJson: await File(path).readAsString(),
      );
      expect(provider['providerUid'], isNot(provider['appUid']));
      expect(provider['mediaBytes'], 9045);
      expect(_providerFds(), 0);
      final gateway = _ObservedGateway();
      await tester.pumpWidget(
        OrApp(gateway: const RustCoreGateway(), projectGateway: gateway),
      );
      await _until(
        tester,
        () => find
            .byKey(const ValueKey('home-open-project'))
            .evaluate()
            .isNotEmpty,
      );
      debugPrint('ANDROID_SAF_DOCUMENTS_UI_READY');
      await tester.tap(find.byKey(const ValueKey('home-open-project')));
      // A hosted UIAutomator helper selects the separate provider's document in
      // the real Android DocumentsUI. No fixture method returns a picker result.
      await _until(
        tester,
        () =>
            gateway.session != null &&
            find.byType(Texture).evaluate().isNotEmpty,
      );
      expect(gateway.openedPath, contains('/files/or-projects/'));
      final session = gateway.session!;
      final revision = (await gateway.summary(session)).revision;
      expect(
        (await gateway.listMediaPage(
          session,
          offset: 64,
          limit: 1,
        )).items.single.sourceUri,
        source,
      );

      // The first explicit user transport action is Play, followed by real seek.
      await tester.tap(find.byKey(const ValueKey('preview-play')));
      await _until(tester, () => gateway.playCalls == 1);
      expect(gateway.playResult!.frameSequence, greaterThan(BigInt.zero));
      expect(gateway.playResult!.errorCode, isNull);
      if (tester
              .widget<IconButton>(find.byKey(const ValueKey('preview-play')))
              .tooltip ==
          'Pause') {
        await tester.tap(find.byKey(const ValueKey('preview-play')));
      }
      await _until(tester, () => gateway.lastPreview?.playing == false);
      await _seekUi(tester, gateway, .25);
      expect(gateway.seekError, isNull);
      expect(gateway.lastPreview!.width, 16);
      expect(gateway.lastPreview!.height, 16);
      expect(gateway.lastPreview!.position.numerator, greaterThan(BigInt.zero));
      expect(await _presenter.invokeMethod<bool>('frameAvailable'), isTrue);
      expect(_providerFds(), 1);
      final registeredBefore = (await _resources())['registrationCount'];
      final opensBefore = (await _control('status'))['providerOpens'];
      for (final position in [.3, .35, .45]) {
        await _seekUi(tester, gateway, position);
        expect(gateway.seekError, isNull);
        expect(await _presenter.invokeMethod<bool>('frameAvailable'), isTrue);
      }
      final registeredAfter = (await _resources())['registrationCount'];
      final opensAfter = (await _control('status'))['providerOpens'];
      expect(registeredAfter, registeredBefore);
      expect(
        opensAfter,
        opensBefore,
        reason: 'Unchanged active-source seeks must reuse the duplicated capability.',
      );
      await binding.convertFlutterSurfaceToImage();
      final pixels = await _redTexture(tester, binding, 'saf-editor-texture');

      await _control('revoke', uri: source);
      expect(
        _providerFds(),
        1,
      ); // Revocation does not revoke an already-open FD.
      await _seekUi(tester, gateway, .4);
      expect(gateway.seekError!.code, 'MEDIA_PERMISSION_REQUIRED');
      expect(find.byKey(const ValueKey('preview-error')), findsOneWidget);
      expect(
        tester.widget<Text>(find.byKey(const ValueKey('preview-error'))).data,
        contains('Select it again'),
      );
      expect((await _resources())['duplicatedMediaFds'], 0);
      expect(_providerFds(), 0);
      await _control('grant', uri: source);
      await _seekUi(tester, gateway, .5);
      expect(gateway.seekError, isNull);
      expect(find.byKey(const ValueKey('preview-error')), findsNothing);
      expect(await _presenter.invokeMethod<bool>('frameAvailable'), isTrue);
      final recoveredPixels = await _redTexture(
        tester,
        binding,
        'saf-editor-permission-recovered',
      );
      expect((await gateway.summary(session)).revision, revision);
      final journey = await _resources();
      await tester.tap(find.byKey(const ValueKey('nav-home')));
      await _until(
        tester,
        () => find
            .byKey(const ValueKey('active-project-close'))
            .evaluate()
            .isNotEmpty,
      );
      await tester.tap(find.byKey(const ValueKey('active-project-close')));
      await _until(tester, () => gateway.closed);
      expect(_providerFds(), 0);
      expect((await _resources())['duplicatedMediaFds'], 0);
      expect((await _resources())['inFlightLeases'], 0);
      await tester.pumpWidget(const SizedBox());

      // Actual provider failures exercise partial-open rollback and retryability.
      await _control('resetMissing');
      expect(
        await _bindingFailure(
          OrViewerTexture.setMediaSources([
            _source('good'),
            _source('missing'),
          ]),
        ),
        'MEDIA_SOURCE_MISSING',
      );
      expect(_providerFds(), 0);
      expect((await _resources())['duplicatedMediaFds'], 0);
      await _control('recoverMissing');
      expect(
        await OrViewerTexture.setMediaSources([
          _source('good'),
          _source('missing'),
        ]),
        isTrue,
      );
      expect(_providerFds(), 2);
      await OrViewerTexture.clearMediaSources();
      expect(_providerFds(), 0);
      expect(
        await _bindingFailure(
          OrViewerTexture.setMediaSources([_source('pipe')]),
        ),
        'MEDIA_SOURCE_NOT_SEEKABLE',
      );
      expect(_providerFds(), 0);
      expect((await _resources())['duplicatedMediaFds'], 0);
      await _control('armBlocked');
      final blocked = _bindingFailure(
        OrViewerTexture.setMediaSources([_source('blocked')]),
      );
      await _control('awaitBlocked');
      final clearing = OrViewerTexture.clearMediaSources();
      await _control('releaseBlocked');
      await clearing;
      expect(await blocked, 'STALE_PREVIEW_REQUEST');
      expect(_providerFds(), 0);

      // Supplemental fresh-session Play guard and bounded surface stress.
      const secondGateway = RustProjectGateway();
      final second = await secondGateway.openProject(path);
      final before = await secondGateway.summary(second);
      final played = await secondGateway.previewPlay(
        second,
      ); // No preceding seek.
      expect(played.frameSequence, greaterThan(BigInt.zero));
      expect(played.width, 16);
      expect(played.errorCode, isNull);
      await secondGateway.previewPause(second);
      expect((await secondGateway.summary(second)).revision, before.revision);
      final oldTexture = (await OrViewerTexture.textureId())!;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: Texture(textureId: oldTexture)),
        ),
      );
      for (var i = 0; i < 8; i++) {
        expect(await _presenter.invokeMethod<bool>('recreateSurface'), isTrue);
        expect(await _presenter.invokeMethod<bool>('frameAvailable'), isTrue);
        expect((await _resources())['inFlightLeases'], 0);
        expect(_providerFds(), 1);
      }
      final presentations = await Future.wait(
        List.generate(
          64,
          (_) => _presenter.invokeMethod<bool>('frameAvailable'),
        ),
      );
      expect(presentations, contains(isTrue));
      final stress = await _resources();
      expect(stress['peakPendingFrameResults'], lessThanOrEqualTo(8));
      expect(stress['peakQueuedOperations'], lessThanOrEqualTo(8));
      expect(stress['bitmapBytes'], lessThanOrEqualTo(1920 * 1080 * 4));
      expect(stress['inFlightLeases'], 0);
      final inFlightPresentation = _presenter.invokeMethod<bool>(
        'frameAvailable',
      );
      expect(await _presenter.invokeMethod<bool>('releaseTexture'), isTrue);
      expect(await inFlightPresentation, anyOf(isTrue, isFalse));
      expect((await _resources())['bitmapBytes'], 0);
      final recreated = (await OrViewerTexture.textureId())!;
      expect(recreated, isNot(oldTexture));
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(body: Texture(textureId: recreated)),
        ),
      );
      expect(await _presenter.invokeMethod<bool>('frameAvailable'), isTrue);
      await _redTexture(tester, binding, 'saf-surface-recreated');
      await secondGateway.close(second, discardUnsaved: false);
      expect(_providerFds(), 0);

      // Real blocked provider open discriminates an edit during registration.
      final blockedPath = await _project(
        directory,
        'Blocked SAF source',
        _source('blocked'),
      );
      final host = await rust.openProject(path: blockedPath);
      final prepared = await host.previewPrepare(
        action: rust.PreviewPreparationActionView.seek,
        position: rust.RationalTimeView(numerator: 0, denominator: 1),
      );
      final summary = await host.summary();
      await _control('armBlocked');
      final bindingOpen = OrViewerTexture.setMediaSources(
        prepared.sources,
        generation: prepared.requestId,
        owner: summary.projectInstanceId,
      );
      await _control('awaitBlocked');
      expect(
        (await host.setTimelineSequenceFrameRate(
          projectId: summary.projectId,
          projectInstanceId: summary.projectInstanceId,
          expectedRevision: summary.revision,
          sequenceFrameRate: rust.RationalRateView(
            numerator: 24,
            denominator: 1,
          ),
        )).succeeded,
        isTrue,
      );
      await _control('releaseBlocked');
      expect(await bindingOpen, isTrue);
      await expectLater(
        host.previewCompletePrepared(requestId: prepared.requestId!),
        throwsA(
          isA<rust.ProjectBridgeError>().having(
            (e) => e.code,
            'code',
            'STALE_PREVIEW_REQUEST',
          ),
        ),
      );
      expect((await host.previewState()).frameSequence, BigInt.zero);
      final older = await host.previewPrepare(
        action: rust.PreviewPreparationActionView.seek,
        position: rust.RationalTimeView(numerator: 0, denominator: 1),
      );
      final newer = await host.previewPrepare(
        action: rust.PreviewPreparationActionView.seek,
        position: rust.RationalTimeView(numerator: 1, denominator: 4),
      );
      expect(newer.requestId, isNot(older.requestId));
      await expectLater(
        host.previewCompletePrepared(requestId: older.requestId!),
        throwsA(
          isA<rust.ProjectBridgeError>().having(
            (e) => e.code,
            'code',
            'STALE_PREVIEW_REQUEST',
          ),
        ),
      );
      expect((await host.close(discardUnsaved: true)).succeeded, isTrue);
      await OrViewerTexture.clearMediaSources();
      final finalResources = await _resources();
      expect(finalResources['duplicatedMediaFds'], 0);
      expect(finalResources['inFlightLeases'], 0);
      expect(finalResources['latestFrameBytes'], 0);
      expect(_providerFds(), 0);
      final providerStats = await _control('status');
      expect(providerStats['providerOpens'], greaterThan(0));
      expect(tester.takeException(), isNull);
      binding.reportData = {
        'androidSafAcceptance': {
          'checks': {
            for (final name in [
              'nativeDocumentsUiAndEditorControls',
              'visibleTexturePixels',
              'externalUidPermissionEnforcement',
              'activeLateSourceBeyond64',
              'unchangedProjectRevision',
              'playWithoutSeek',
              'revokedPermissionUiRecovery',
              'missingPartialOpenRollbackAndRecovery',
              'nonseekableNativeRegistrationRejected',
              'clearDuringOpenDropsStaleBinding',
              'editAndGenerationDropPreparedFrame',
              'surfaceRecreationAndRelease',
              'osMediaFdsAndNativeLeasesReleased',
              'boundedPresentationStress',
              'sameSourceSeeksReuseProviderCapability',
            ])
              name: true,
          },
          'sourceUri': source,
          'providerUid': provider['providerUid'],
          'appUid': provider['appUid'],
          'projectRevision': revision.toString(),
          'visiblePixelRgba': pixels,
          'recoveredPixelRgba': recoveredPixels,
          'journeyResources': journey,
          'stressResources': stress,
          'finalResources': finalResources,
          'providerOpens': providerStats['providerOpens'],
          'finalOsMediaFds': _providerFds(),
          'uiPlayMicros': gateway.playMicros,
          'sameSourceRegistrations': [registeredBefore, registeredAfter],
          'sameSourceProviderOpens': [opensBefore, opensAfter],
          'softwareFallback': 'packaged_ffmpeg_shared_render_bounded_bgra',
          'hardware': 'UNVERIFIED',
        },
      };
      debugPrint(
        'ANDROID_SAF_ACCEPTANCE_COMPLETE ${jsonEncode(binding.reportData)}',
      );
    },
  );
}
