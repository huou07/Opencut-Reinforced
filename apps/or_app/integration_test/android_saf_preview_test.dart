import 'dart:convert';
import 'dart:async';
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

Future<T> _stage<T>(String name, Future<T> Function() operation) async {
  debugPrint('ANDROID_SAF_STAGE_START $name');
  try {
    final value = await operation().timeout(
      const Duration(seconds: 60),
      onTimeout: () => throw TimeoutException(
        'Android SAF acceptance stage timed out: $name',
      ),
    );
    debugPrint('ANDROID_SAF_STAGE_COMPLETE $name');
    return value;
  } catch (error) {
    debugPrint('ANDROID_SAF_STAGE_FAILED $name: $error');
    rethrow;
  }
}

Future<Map<String, dynamic>> _control(
  String operation, {
  String? uri,
  String? projectJson,
}) => _stage('fixture-control:$operation', () async {
  return Map<String, dynamic>.from(
    (await _fixture.invokeMapMethod<String, dynamic>('control', {
      'operation': operation,
      'uri': uri,
      'projectJson': projectJson,
    }))!,
  );
});
Future<Map<String, int>> _resources() => _stage('resource-snapshot', () async {
  return (await _presenter.invokeMapMethod<String, num>('resourceSnapshot'))!
      .map((key, value) => MapEntry(key, value.toInt()));
});
Future<bool?> _frameAvailable(String stage) => _stage(
  'frame-available:$stage',
  () => _presenter.invokeMethod<bool>('frameAvailable'),
);

// This counts real capabilities in OR's process, including duplicates which
// remain readable after the provider revokes an Android URI grant.
int _providerFds() {
  var count = 0;
  for (final entry in Directory('/proc/self/fd').listSync(followLinks: false)) {
    try {
      final target = Link(entry.path).targetSync();
      if (target.contains('/dev.opencut.saffixture/') &&
          (target.endsWith('/tiny.mkv') ||
              target.endsWith('/tiny-second.mkv') ||
              target.endsWith('/relink-replacement.mkv'))) {
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

String? _exportStatus(WidgetTester tester) {
  final status = find.byKey(const ValueKey('export-status'));
  if (status.evaluate().isEmpty) return null;
  return tester.widget<Tooltip>(status).message;
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

Future<void> _dragSeekUi(
  WidgetTester tester,
  _ObservedGateway gateway,
  double fraction,
) async {
  final slider = find.byKey(const ValueKey('preview-scrub-ruler'));
  final play = find.byKey(const ValueKey('preview-play'));
  await _until(
    tester,
    () =>
        slider.evaluate().isNotEmpty &&
        tester.widget<IconButton>(play).onPressed != null,
  );
  final before = gateway.seekCalls;
  await tester.ensureVisible(slider);
  final rect = tester.getRect(slider);
  await tester.dragFrom(
    Offset(rect.left + 24, rect.center.dy),
    Offset((rect.width - 48) * fraction, 0),
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
  final bytes = await _stage(
    'screenshot:$name',
    () => binding.takeScreenshot(name),
  );
  final codec = await ui.instantiateImageCodec(Uint8List.fromList(bytes));
  final frame = await codec.getNextFrame();
  final image = frame.image;
  final rgba = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
  final offset = (center.dy.floor() * image.width + center.dx.floor()) * 4;
  final pixel = List.generate(4, (i) => rgba.getUint8(offset + i));
  image.dispose();
  codec.dispose();
  expect(pixel[0], greaterThanOrEqualTo(200), reason: 'Red channel of $pixel');
  expect(pixel[1], lessThanOrEqualTo(40), reason: 'Green channel of $pixel');
  expect(pixel[2], lessThanOrEqualTo(40), reason: 'Blue channel of $pixel');
  expect(pixel[3], 255, reason: 'Alpha channel of $pixel');
  return pixel;
}

// The two editor captures above are of a texture that had been presenting for
// seconds, so a single posted frame was already on screen. A brand-new
// SurfaceTexture is different: the engine has to acquire the first buffer the
// plugin posts before the layer carries it, and this SwiftShader guest drops
// hundreds of frames doing that. Keep asking the real plugin for real frames
// and let the platform settle, so the capture measures the surface and not how
// fast this guest composites. Bounded, so a genuinely blank surface still
// fails on the unchanged pixel assertion.
Future<void> _settleRecreatedSurface(WidgetTester tester) async {
  for (var attempt = 0; attempt < 8; attempt++) {
    expect(
      await _frameAvailable('recreated-surface'),
      isTrue,
      reason: 'A recreated Android surface must present a real frame.',
    );
    await tester.pump(const Duration(milliseconds: 500));
  }
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
              'duration': {'numerator': 1, 'denominator': 2},
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
                    'duration': {'numerator': 1, 'denominator': 2},
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
  final insertedActiveClip = await gateway.insertTimelineClip(
    session,
    current,
    trackId: track.trackId,
    mediaId: media.mediaId,
    timelineStart: ProjectRationalTime(BigInt.zero, 1),
    sourceStart: ProjectRationalTime(BigInt.zero, 1),
    duration: ProjectRationalTime(BigInt.one, 2),
  );
  expect(insertedActiveClip.succeeded, isTrue);
  current = insertedActiveClip.view!;
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
  int importCalls = 0, importMicros = 0;
  int relinkCalls = 0;
  String? lastRelinkMediaId, lastRelinkSource;
  ProjectActionResult? lastRelinkResult;
  int saveCalls = 0;
  int captionImportCalls = 0,
      captionExportStartedCalls = 0,
      captionExportCalls = 0;
  bool closed = false;
  @override
  Future<ProjectSessionHandle> openProject(String path) async {
    openedPath = path;
    return session = await super.openProject(path);
  }

  @override
  Future<ProjectActionResult> save(ProjectSessionHandle session) async {
    try {
      return await super.save(session);
    } finally {
      saveCalls++;
    }
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
  Future<ProjectActionResult> importMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String source,
  ) async {
    final watch = Stopwatch()..start();
    try {
      return await super.importMedia(session, current, source);
    } finally {
      importCalls++;
      importMicros = watch.elapsedMicroseconds;
    }
  }

  @override
  Future<ProjectActionResult> relinkMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String mediaId,
    String source,
  ) async {
    try {
      return lastRelinkResult = await super.relinkMedia(
        session,
        current,
        mediaId,
        source,
      );
    } finally {
      relinkCalls++;
      lastRelinkMediaId = mediaId;
      lastRelinkSource = source;
    }
  }

  @override
  Future<ProjectActionResult> importTimelineCaptions(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String path,
  ) async {
    try {
      return await super.importTimelineCaptions(session, current, path);
    } finally {
      captionImportCalls++;
    }
  }

  @override
  Future<ProjectActionResult> exportTimelineCaptions(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String path,
    required ProjectCaptionFileFormat format,
  }) async {
    captionExportStartedCalls++;
    debugPrint('ANDROID_SAF_CAPTION_EXPORT_GATEWAY_STARTED');
    try {
      return await super.exportTimelineCaptions(
        session,
        current,
        path: path,
        format: format,
      );
    } finally {
      captionExportCalls++;
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
      final projectJson = await File(path).readAsString();
      final provider = await _control('seedProject', projectJson: projectJson);
      expect(provider['providerUid'], isNot(provider['appUid']));
      expect(provider['mediaBytes'], 9045);
      expect(
        provider['projectBytes'],
        utf8.encode(projectJson).length,
        reason:
            'The provider must serve the exact document OR saved, not a '
            'truncated or substituted fixture project.',
      );
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
      // Prepare Flutter's screenshot surface before the editor registers its
      // Android SurfaceProducer texture. The prior hosted run stalled while
      // rebinding the activity render surface with the preview texture active.
      debugPrint('ANDROID_SAF_SURFACE_CONVERSION_START');
      await binding.convertFlutterSurfaceToImage();
      debugPrint('ANDROID_SAF_SURFACE_CONVERSION_COMPLETE');
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
      debugPrint('ANDROID_SAF_PROJECT_OPENED');
      expect(gateway.openedPath, contains('/files/or-projects/'));
      debugPrint('ANDROID_SAF_INITIAL_TEXTURE_CAPTURE_START');
      final session = gateway.session!;
      final revision = (await _stage(
        'project-summary',
        () => gateway.summary(session),
      )).revision;
      debugPrint('ANDROID_SAF_PROJECT_SUMMARY_READ');
      expect(
        (await _stage(
          'project-media-page',
          () => gateway.listMediaPage(session, offset: 64, limit: 1),
        )).items.single.sourceUri,
        source,
      );
      debugPrint('ANDROID_SAF_PROJECT_MEDIA_PAGE_READ');

      // The first explicit user transport action is Play, followed by real seek.
      debugPrint('ANDROID_SAF_PLAY_TAP_START');
      await tester.tap(find.byKey(const ValueKey('preview-play')));
      debugPrint('ANDROID_SAF_PLAY_TAP_COMPLETE');
      await _until(tester, () => gateway.playCalls == 1);
      debugPrint('ANDROID_SAF_PLAY_COMMAND_COMPLETE');
      expect(gateway.playResult!.frameSequence, greaterThan(BigInt.zero));
      expect(gateway.playResult!.errorCode, isNull);
      if (tester
              .widget<IconButton>(find.byKey(const ValueKey('preview-play')))
              .tooltip ==
          'Pause') {
        await tester.tap(find.byKey(const ValueKey('preview-play')));
      }
      await _until(tester, () => gateway.lastPreview?.playing == false);
      debugPrint('ANDROID_SAF_PLAYBACK_PAUSED');
      await _seekUi(tester, gateway, .25);
      debugPrint('ANDROID_SAF_FIRST_SEEK_COMPLETE');
      expect(gateway.seekError, isNull);
      expect(gateway.lastPreview!.width, 16);
      expect(gateway.lastPreview!.height, 16);
      expect(gateway.lastPreview!.position.numerator, greaterThan(BigInt.zero));
      expect(await _frameAvailable('after-first-seek'), isTrue);
      debugPrint('ANDROID_SAF_PROVIDER_FD_SNAPSHOT_START');
      expect(_providerFds(), 1);
      debugPrint('ANDROID_SAF_PROVIDER_FD_SNAPSHOT_COMPLETE');
      final registeredBefore = (await _resources())['registrationCount'];
      final opensBefore = (await _control('status'))['providerOpens'];
      for (final position in [.3, .35, .45]) {
        await _seekUi(tester, gateway, position);
        expect(gateway.seekError, isNull);
        expect(await _frameAvailable('after-repeat-seek'), isTrue);
      }
      final registeredAfter = (await _resources())['registrationCount'];
      final opensAfter = (await _control('status'))['providerOpens'];
      expect(registeredAfter, registeredBefore);
      expect(
        opensAfter,
        opensBefore,
        reason: 'Unchanged active-source seeks must reuse the duplicated capability.',
      );

      final playCallsBeforeBackground = gateway.playCalls;
      await tester.tap(find.byKey(const ValueKey('preview-play')));
      await _until(
        tester,
        () => gateway.playCalls == playCallsBeforeBackground + 1,
      );
      expect(gateway.lastPreview!.playing, isTrue);
      debugPrint('ANDROID_SAF_BACKGROUND_CONTROL_START');
      expect((await _control('backgroundAndResume'))['backgrounded'], isTrue);
      debugPrint('ANDROID_SAF_BACKGROUND_CONTROL_COMPLETE');
      await _stage(
        'wait-for-background-pause',
        () => _until(tester, () => gateway.lastPreview?.playing == false),
      );
      debugPrint('ANDROID_SAF_BACKGROUND_PAUSE_OBSERVED');
      await _stage(
        'wait-for-activity-resume',
        () => _until(
          tester,
          () => tester.binding.lifecycleState == AppLifecycleState.resumed,
        ),
      );
      debugPrint('ANDROID_SAF_ACTIVITY_RESUME_OBSERVED');
      expect(_providerFds(), 1);
      expect((await gateway.summary(session)).revision, revision);
      expect(await _frameAvailable('after-background-resume'), isTrue);
      await _settleRecreatedSurface(tester);
      final backgroundPixels = await _redTexture(
        tester,
        binding,
        'saf-background-resumed-texture',
      );

      final pixels = await _redTexture(tester, binding, 'saf-editor-texture');
      debugPrint('ANDROID_SAF_INITIAL_TEXTURE_CAPTURE_COMPLETE');

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
      expect(await _frameAvailable('after-permission-regrant'), isTrue);
      final recoveredPixels = await _redTexture(
        tester,
        binding,
        'saf-editor-permission-recovered',
      );
      expect((await gateway.summary(session)).revision, revision);

      final exportButton = find.byKey(const ValueKey('export-project'));
      await tester.ensureVisible(exportButton);
      await tester.tap(exportButton);
      await tester.pump();
      debugPrint('ANDROID_SAF_EXPORT_DOCUMENTS_UI_READY');
      await _until(tester, () {
        final status = _exportStatus(tester);
        return status != null &&
            status != 'Export queued' &&
            status != 'Exporting' &&
            !RegExp(r'^Export \d+/\d+$').hasMatch(status) &&
            status != 'Saving export to the selected location…';
      });
      final exportStatus = _exportStatus(tester);
      expect(
        exportStatus,
        'Export complete',
        reason: 'Android export ended with status: $exportStatus',
      );
      final exported = await _control('exportStatus');
      expect(exported['exportBytes'], greaterThan(4));
      expect(exported['validMatroska'], isTrue);
      expect((await gateway.summary(session)).revision, revision);

      await tester.tap(find.byKey(const ValueKey('mobile-editor-tool-media')));
      await _until(
        tester,
        () => find
            .byKey(const ValueKey('project-media-panel'))
            .evaluate()
            .isNotEmpty,
      );
      await tester.tap(find.byKey(const ValueKey('media-import')));
      debugPrint('ANDROID_SAF_MEDIA_IMPORT_DOCUMENTS_UI_READY');
      await _until(tester, () => gateway.importCalls == 2);
      final imported = await gateway.listMediaPage(
        session,
        offset: 65,
        limit: 2,
      );
      expect(imported.items.map((item) => item.sourceUri), [
        _source('media'),
        _source('media-second'),
      ]);
      expect(
        imported.items.every((item) => item.formatNames.contains('matroska')),
        isTrue,
      );
      final importedRevision = (await gateway.summary(session)).revision;
      expect(importedRevision, greaterThan(revision));
      // Keep the granted source descriptors alive while the project revision
      // reconnects the viewer; project close below owns their release.
      await tester.tap(find.byKey(const ValueKey('mobile-tool-sheet-close')));
      // The mobile tool sheet is a route. Do not search for or tap timeline
      // controls while its dismissal barrier is still intercepting input.
      await _until(
        tester,
        () => find
            .byKey(const ValueKey('project-media-panel'))
            .evaluate()
            .isEmpty,
      );

      final importCaptionsButton = find.byKey(
        const ValueKey('timeline-import-captions'),
      );
      await tester.ensureVisible(importCaptionsButton);
      await _until(
        tester,
        () => importCaptionsButton.hitTestable().evaluate().isNotEmpty,
      );
      await tester.tap(importCaptionsButton.hitTestable());
      debugPrint('ANDROID_SAF_CAPTION_IMPORT_DOCUMENTS_UI_READY');
      await _until(
        tester,
        () => find.text('Import SubRip captions?').evaluate().isNotEmpty,
      );
      await tester.tap(find.text('Import').last);
      await _until(tester, () => gateway.captionImportCalls == 1);
      var captionTracks = (await gateway.listTimelineTracks(session)).items
          .where((track) => track.kind == ProjectTimelineTrackKind.caption)
          .toList();
      expect(captionTracks, hasLength(1));
      var captionPage = await gateway.listTimelineClips(
        session,
        trackId: captionTracks.single.trackId,
        offset: 0,
        limit: 10,
      );
      expect(captionPage.items.single.text, 'Imported SAF caption');
      final captionImportRevision = (await gateway.summary(session)).revision;

      final exportCaptionsButton = find.byKey(
        const ValueKey('timeline-export-captions'),
      );
      await tester.ensureVisible(exportCaptionsButton);
      await _until(tester, () {
        if (exportCaptionsButton.evaluate().isEmpty) return false;
        return tester.widget<TextButton>(exportCaptionsButton).onPressed !=
            null;
      });
      await tester.tap(exportCaptionsButton);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Continue'));
      debugPrint('ANDROID_SAF_CAPTION_EXPORT_DOCUMENTS_UI_READY');
      await _until(tester, () => gateway.captionExportStartedCalls == 1);
      await _until(tester, () => gateway.captionExportCalls == 1);
      await _until(
        tester,
        () => find.text('Caption file exported.').evaluate().isNotEmpty,
      );
      final captionExport = await _control('status');
      expect(captionExport['captionExportBytes'], greaterThan(0));
      expect(captionExport['validSrtCaption'], isTrue);

      captionTracks = (await gateway.listTimelineTracks(session)).items
          .where((track) => track.kind == ProjectTimelineTrackKind.caption)
          .toList();
      expect(captionTracks, hasLength(1));
      captionPage = await gateway.listTimelineClips(
        session,
        trackId: captionTracks.single.trackId,
        offset: 0,
        limit: 10,
      );
      expect(captionPage.items.single.text, 'Imported SAF caption');

      final activeMedia = (await gateway.listMediaPage(
        session,
        offset: 64,
        limit: 1,
      )).items.single;
      expect(activeMedia.mediaId, '00000041-2222-4222-8222-222222222222');
      expect(activeMedia.sourceUri, source);
      await tester.tap(find.byKey(const ValueKey('mobile-editor-tool-media')));
      await _until(
        tester,
        () => find
            .byKey(const ValueKey('project-media-panel'))
            .evaluate()
            .isNotEmpty,
      );
      final mediaPanel = find.byKey(const ValueKey('project-media-panel'));
      final mediaList = find.descendant(
        of: mediaPanel,
        matching: find.byType(ListView),
      );
      await _until(tester, () => mediaList.evaluate().isNotEmpty);
      final loadMore = find.byKey(const ValueKey('media-load-more'));
      for (
        var attempt = 0;
        attempt < 20 && loadMore.hitTestable().evaluate().isEmpty;
        attempt++
      ) {
        await tester.drag(mediaList, const Offset(0, -600));
        await tester.pumpAndSettle();
      }
      expect(loadMore.hitTestable(), findsOneWidget);
      await tester.tap(loadMore);
      await _until(tester, () => loadMore.evaluate().isEmpty);
      final mediaActions = find.byKey(
        ValueKey('media-actions-${activeMedia.mediaId}'),
      );
      for (
        var attempt = 0;
        attempt < 20 && mediaActions.hitTestable().evaluate().isEmpty;
        attempt++
      ) {
        await tester.drag(mediaList, const Offset(0, -600));
        await tester.pumpAndSettle();
      }
      expect(mediaActions.hitTestable(), findsOneWidget);
      await tester.tap(mediaActions);
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(ValueKey('media-relink-${activeMedia.mediaId}')),
      );
      debugPrint('ANDROID_SAF_MEDIA_RELINK_DOCUMENTS_UI_READY');
      await _until(tester, () => gateway.lastRelinkResult != null);
      final relinkResult = gateway.lastRelinkResult!;
      debugPrint(
        'ANDROID_SAF_MEDIA_RELINK_RESULT '
        'succeeded=${relinkResult.succeeded} '
        'errorCode=${relinkResult.errorCode} '
        'message=${relinkResult.message} '
        'revision=${relinkResult.view?.revision}',
      );
      expect(
        relinkResult.succeeded,
        isTrue,
        reason:
            'The real SAF relink command must succeed: '
            '${relinkResult.errorCode} ${relinkResult.message}',
      );
      debugPrint('ANDROID_SAF_MEDIA_RELINK_COMPLETE');
      final relinkedMedia = (await gateway.listMediaPage(
        session,
        offset: 64,
        limit: 1,
      )).items.single;
      expect(gateway.lastRelinkMediaId, activeMedia.mediaId);
      expect(gateway.lastRelinkSource, _source('relink-replacement'));
      expect(relinkedMedia.mediaId, activeMedia.mediaId);
      expect(relinkedMedia.sourceUri, _source('relink-replacement'));
      final videoTrack = (await gateway.listTimelineTracks(session)).items
          .singleWhere((track) => track.kind == ProjectTimelineTrackKind.video);
      final relinkedClips = await gateway.listTimelineClips(
        session,
        trackId: videoTrack.trackId,
        offset: 0,
        limit: 20,
      );
      expect(
        relinkedClips.items.any((clip) => clip.mediaId == activeMedia.mediaId),
        isTrue,
        reason: 'Relinking must retain the timeline clip identity.',
      );
      final mediaRelinkRevision = (await gateway.summary(session)).revision;
      expect(mediaRelinkRevision, greaterThan(captionImportRevision));
      await tester.tap(find.byKey(const ValueKey('mobile-tool-sheet-close')));
      await _until(
        tester,
        () => find
            .byKey(const ValueKey('project-media-panel'))
            .evaluate()
            .isEmpty,
      );

      // Compact transport and editing tools must remain reachable by touch.
      await _dragSeekUi(tester, gateway, .65);
      expect(gateway.seekError, isNull);
      expect(gateway.lastPreview!.position.numerator, greaterThan(BigInt.zero));
      await tester.tap(find.byKey(const ValueKey('mobile-editor-tool-media')));
      await _until(
        tester,
        () => find
            .byKey(const ValueKey('project-media-panel'))
            .evaluate()
            .isNotEmpty,
      );
      expect(find.byKey(const ValueKey('media-import')), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('mobile-tool-sheet-close')));
      await _until(
        tester,
        () => find
            .byKey(const ValueKey('project-media-panel'))
            .evaluate()
            .isEmpty,
      );

      final clip = find.byWidgetPredicate((widget) {
        final key = widget.key;
        return key is ValueKey<String> &&
            key.value.startsWith('timeline-clip-') &&
            !key.value.startsWith('timeline-clip-tooltip-');
      });
      await _until(tester, () => clip.evaluate().isNotEmpty);
      await tester.ensureVisible(clip.first);
      await tester.tap(clip.first);
      await _until(tester, () => find.text('Clip').evaluate().isNotEmpty);
      await tester.tap(
        find.descendant(
          of: find.byType(AlertDialog).last,
          matching: find.text('Close'),
        ),
      );
      await _until(tester, () => find.byType(AlertDialog).evaluate().isEmpty);
      await _until(
        tester,
        () => find
            .byKey(const ValueKey('mobile-editor-tool-inspector'))
            .evaluate()
            .isNotEmpty,
      );
      await tester.tap(
        find.byKey(const ValueKey('mobile-editor-tool-inspector')),
      );
      await _until(
        tester,
        () => find
            .byKey(const ValueKey('inspector-visual-x'))
            .evaluate()
            .isNotEmpty,
      );
      expect(
        tester
            .widget<TextField>(find.byKey(const ValueKey('inspector-visual-x')))
            .controller!
            .text,
        '0',
      );
      await tester.tap(find.byKey(const ValueKey('mobile-tool-sheet-close')));
      await _until(
        tester,
        () =>
            find.byKey(const ValueKey('inspector-visual-x')).evaluate().isEmpty,
      );

      // Importing media and seeking dirties the project. Save through the
      // editor before closing so the journey verifies provider cleanup while
      // preserving the normal unsaved-work guard.
      final saveButton = find.byKey(const ValueKey('workspace-save'));
      await tester.ensureVisible(saveButton);
      await _until(
        tester,
        () =>
            saveButton.evaluate().isNotEmpty &&
            saveButton.hitTestable().evaluate().isNotEmpty,
      );
      await tester.tap(saveButton);
      await _until(tester, () => gateway.saveCalls == 1);
      Map<String, dynamic>? syncedProject;
      for (var attempt = 0; attempt < 60; attempt++) {
        syncedProject = await _control('status');
        if (syncedProject['projectSha256'] != provider['projectSha256']) {
          break;
        }
        await tester.pump(const Duration(milliseconds: 100));
      }
      expect(
        syncedProject?['projectSha256'],
        isNot(provider['projectSha256']),
        reason:
            'Save must sync the edited project to its writable SAF document.',
      );

      final journey = await _resources();
      // The close control lives on the Projects workspace. Tapping nav-home
      // landed on Home, which has no active-project card, so the close step
      // could never complete.
      await tester.tap(find.byKey(const ValueKey('nav-projects')));
      await _until(
        tester,
        () => find
            .byKey(const ValueKey('active-project-close'))
            .evaluate()
            .isNotEmpty,
      );
      // The action row sits below the fold at this height; a real user scrolls
      // to it before tapping.
      await tester.ensureVisible(
        find.byKey(const ValueKey('active-project-close')),
      );
      await tester.pump();
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
      final reopenedRelink = (await secondGateway.listMediaPage(
        second,
        offset: 64,
        limit: 1,
      )).items.single;
      expect(reopenedRelink.mediaId, activeMedia.mediaId);
      expect(reopenedRelink.sourceUri, _source('relink-replacement'));
      final reopenedTrack = (await secondGateway.listTimelineTracks(second))
          .items
          .singleWhere((track) => track.kind == ProjectTimelineTrackKind.video);
      final reopenedClips = await secondGateway.listTimelineClips(
        second,
        trackId: reopenedTrack.trackId,
        offset: 0,
        limit: 20,
      );
      expect(
        reopenedClips.items.any((clip) => clip.mediaId == activeMedia.mediaId),
        isTrue,
      );
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
        expect(await _frameAvailable('surface-stress'), isTrue);
        expect((await _resources())['inFlightLeases'], 0);
        expect(_providerFds(), 1);
      }
      final presentations = await Future.wait(
        List.generate(64, (_) => _frameAvailable('parallel-stress')),
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
      await _settleRecreatedSurface(tester);
      final recreatedPixels = await _redTexture(
        tester,
        binding,
        'saf-surface-recreated',
      );
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
      final retainedMedia = await _control('status');
      expect(
        retainedMedia['persistedMediaUris'],
        contains(relinkedMedia.sourceUri),
      );
      // Leave an unsaved, valid project change in the app-private working copy
      // and persist it using the same recovery checkpoint API the shell uses.
      // The host runner force-stops this process after the drive completes.
      final recoveryGateway = RustProjectGateway();
      final recoverySession = await recoveryGateway.openProject(
        gateway.openedPath!,
      );
      final recoveryBase = await recoveryGateway.summary(recoverySession);
      expect(
        (await recoveryGateway.rename(
          recoverySession,
          recoveryBase,
          'Process recovery acceptance',
        )).succeeded,
        isTrue,
      );
      expect(
        (await recoveryGateway.autosaveCheckpoint(recoverySession)).succeeded,
        isTrue,
      );
      final recoveryPath = gateway.openedPath!;
      final recoveryBeforeClose = await recoveryGateway.inspectRecovery(
        recoveryPath,
      );
      expect(recoveryBeforeClose.kind, ProjectRecoveryKind.candidate);
      expect(recoveryBeforeClose.recoveryName, 'Process recovery acceptance');
      await recoveryGateway.close(recoverySession, discardUnsaved: true);
      final recoveryAfterClose = await recoveryGateway.inspectRecovery(
        recoveryPath,
      );
      expect(
        recoveryAfterClose.kind,
        ProjectRecoveryKind.candidate,
        reason: 'Closing the live session must preserve its recovery sidecar.',
      );
      expect(
        recoveryAfterClose.recoveryRevision,
        recoveryBeforeClose.recoveryRevision,
      );
      debugPrint('ANDROID_SAF_RECOVERY_CHECKPOINT_READY');
      final captures =
          (binding.reportData?['screenshots'] as List?) ?? const [];
      expect(
        captures.map((entry) => (entry as Map)['screenshotName']).toSet(),
        {
          'saf-editor-texture',
          'saf-background-resumed-texture',
          'saf-editor-permission-recovered',
          'saf-surface-recreated',
        },
        reason: 'Every asserted capture must reach the acceptance artifact.',
      );
      // takeScreenshot accumulates each captured PNG in this map and the driver
      // writes those bytes to the acceptance output. Assigning a fresh map
      // discarded them, so the screenshots only existed while the test failed.
      binding.reportData = {
        ...?binding.reportData,
        'androidSafAcceptance': {
          'checks': {
            for (final name in [
              'nativeDocumentsUiAndEditorControls',
              'safMediaImportThroughDocumentsUi',
              'visibleTexturePixels',
              'foregroundBackgroundPlaybackPausesAndSurfaceRecovers',
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
              'mobileTouchScrubbingAndTransport',
              'mobileMediaLibrarySheet',
              'mobileSelectedClipInspectorSheet',
              'androidSafExportToDocumentsUi',
              'androidSafCaptionImportAndExportThroughDocumentsUi',
              'androidSafMediaRelinkThroughDocumentsUi',
              'recoveryCheckpointPersistedBeforeProcessStop',
            ])
              name: true,
          },
          'sourceUri': source,
          'providerUid': provider['providerUid'],
          'appUid': provider['appUid'],
          'projectRevision': revision.toString(),
          'mediaImportRevision': importedRevision.toString(),
          'captionImportRevision': captionImportRevision.toString(),
          'captionImportCalls': gateway.captionImportCalls,
          'captionExportCalls': gateway.captionExportCalls,
          'captionExportBytes': captionExport['captionExportBytes'],
          'captionExportValidSrt': captionExport['validSrtCaption'],
          'mediaImportMicros': gateway.importMicros,
          'mediaImportSourceUris': imported.items
              .map((item) => item.sourceUri)
              .toList(growable: false),
          'mediaRelinkCalls': gateway.relinkCalls,
          'mediaRelinkSucceeded': relinkResult.succeeded,
          'mediaRelinkErrorCode': relinkResult.errorCode,
          'mediaRelinkResultRevision': relinkResult.view?.revision.toString(),
          'mediaRelinkMediaIdBefore': activeMedia.mediaId,
          'mediaRelinkMediaIdAfter': relinkedMedia.mediaId,
          'mediaRelinkSourceBefore': activeMedia.sourceUri,
          'mediaRelinkSourceAfter': relinkedMedia.sourceUri,
          'mediaRelinkTimelineReferencePreserved': relinkedClips.items.any(
            (clip) => clip.mediaId == activeMedia.mediaId,
          ),
          'mediaRelinkRevision': mediaRelinkRevision.toString(),
          'exportBytes': exported['exportBytes'],
          'exportValidMatroska': exported['validMatroska'],
          'visiblePixelRgba': pixels,
          'backgroundResumePixelRgba': backgroundPixels,
          'recoveredPixelRgba': recoveredPixels,
          'recreatedPixelRgba': recreatedPixels,
          'journeyResources': journey,
          'stressResources': stress,
          'finalResources': finalResources,
          'providerOpens': providerStats['providerOpens'],
          'providerProjectSha256AtSeed': provider['projectSha256'],
          'providerProjectSha256BeforeRestart': providerStats['projectSha256'],
          'finalOsMediaFds': _providerFds(),
          'recoveryName': recoveryBeforeClose.recoveryName,
          'recoveryBaseRevision': recoveryBeforeClose.baseRevision.toString(),
          'recoveryRevision': recoveryBeforeClose.recoveryRevision.toString(),
          'recoveryKindAfterClose': recoveryAfterClose.kind.name,
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
