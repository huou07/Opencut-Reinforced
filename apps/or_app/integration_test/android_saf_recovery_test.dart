import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:or_app/main.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/project/rust_project_gateway.dart';
import 'package:or_app/rust_core_gateway.dart';
import 'package:or_app_bridge/or_app_bridge.dart' as rust;

class _RecoveryGateway extends RustProjectGateway {
  ProjectSessionHandle? session;
  String? openedPath;
  String? inspectedPath;
  ProjectRecoveryInspection? inspection;
  ProjectPreviewState? playResult;
  int playCalls = 0;

  @override
  Future<ProjectRecoveryInspection> inspectRecovery(String path) async {
    inspectedPath = path;
    return inspection = await super.inspectRecovery(path);
  }

  @override
  Future<ProjectSessionHandle> openProject(String path) async {
    openedPath = path;
    return session = await super.openProject(path);
  }

  @override
  Future<ProjectPreviewState> previewPlay(ProjectSessionHandle session) async {
    try {
      return playResult = await super.previewPlay(session);
    } finally {
      playCalls++;
    }
  }
}

const _fixtureChannel = MethodChannel('or_saf_acceptance_fixture');

Future<Map<String, Object?>> _fixtureStatus() async => (await _fixtureChannel
    .invokeMapMethod<String, Object?>('control', {'operation': 'status'}))!;

Future<void> _until(WidgetTester tester, bool Function() ready) async {
  for (var i = 0; i < 600 && !ready(); i++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
  expect(ready(), isTrue, reason: 'The recovery user action did not complete.');
}

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(rust.RustLib.init);

  testWidgets('Android SAF recovery survives a real app process restart', (
    tester,
  ) async {
    final gateway = _RecoveryGateway();
    await tester.pumpWidget(
      OrApp(gateway: const RustCoreGateway(), projectGateway: gateway),
    );
    await _until(
      tester,
      () =>
          find.byKey(const ValueKey('home-open-project')).evaluate().isNotEmpty,
    );
    final providerBeforeOpen = await _fixtureStatus();
    expect(
      providerBeforeOpen['persistedMediaUris'],
      contains(
        'content://dev.opencut.saffixture.documents/document/relink-replacement',
      ),
      reason: 'The relinked media grant must survive process recovery.',
    );
    debugPrint(
      'ANDROID_SAF_PROVIDER_BEFORE_RECOVERY_OPEN '
      'projectSha256=${providerBeforeOpen['projectSha256']} '
      'projectBytes=${providerBeforeOpen['projectBytes']}',
    );
    debugPrint('ANDROID_SAF_DOCUMENTS_UI_READY');
    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    try {
      await _until(
        tester,
        () => find.text('Recovery checkpoint found').evaluate().isNotEmpty,
      );
    } catch (_) {
      final visibleText = find
          .byType(Text)
          .evaluate()
          .map((element) => (element.widget as Text).data)
          .whereType<String>()
          .join(' | ');
      debugPrint(
        'ANDROID_SAF_RECOVERY_DIAGNOSTIC '
        'inspectedPath=${gateway.inspectedPath} '
        'kind=${gateway.inspection?.kind} '
        'baseRevision=${gateway.inspection?.baseRevision} '
        'recoveryRevision=${gateway.inspection?.recoveryRevision} '
        'recoveryName=${gateway.inspection?.recoveryName} '
        'visibleText=$visibleText',
      );
      rethrow;
    }
    expect(find.textContaining('Process recovery acceptance'), findsOneWidget);
    await tester.tap(find.text('Recover'));
    try {
      await _until(
        tester,
        () =>
            gateway.session != null &&
            find.byType(Texture).evaluate().isNotEmpty,
      );
    } catch (_) {
      final visibleText = find
          .byType(Text)
          .evaluate()
          .map((element) => (element.widget as Text).data)
          .whereType<String>()
          .join(' | ');
      ProjectReadModel? openedProject;
      if (gateway.session != null) {
        try {
          openedProject = await gateway.summary(gateway.session!);
        } catch (_) {
          // Keep the recovery-state diagnostic even if the session is stale.
        }
      }
      debugPrint(
        'ANDROID_SAF_RECOVERY_AFTER_APPLY_DIAGNOSTIC '
        'openedPath=${gateway.openedPath} '
        'sessionOpened=${gateway.session != null} '
        'projectName=${openedProject?.name} '
        'projectRevision=${openedProject?.revision} '
        'textureVisible=${find.byType(Texture).evaluate().isNotEmpty} '
        'visibleText=$visibleText',
      );
      try {
        await binding.takeScreenshot('saf-recovery-after-apply-failure');
      } catch (_) {
        // The log diagnostic remains useful when the platform cannot capture.
      }
      rethrow;
    }

    expect(gateway.openedPath, contains('/files/or-projects/'));
    final summary = await gateway.summary(gateway.session!);
    expect(summary.name, 'Process recovery acceptance');
    expect(summary.revision, greaterThan(BigInt.zero));
    final relinked = (await gateway.listMediaPage(
      gateway.session!,
      offset: 64,
      limit: 1,
    )).items.single;
    expect(relinked.mediaId, '00000041-2222-4222-8222-222222222222');
    expect(
      relinked.sourceUri,
      'content://dev.opencut.saffixture.documents/document/relink-replacement',
    );
    final videoTrack = (await gateway.listTimelineTracks(gateway.session!))
        .items
        .singleWhere((track) => track.kind == ProjectTimelineTrackKind.video);
    final clips = await gateway.listTimelineClips(
      gateway.session!,
      trackId: videoTrack.trackId,
      offset: 0,
      limit: 20,
    );
    expect(clips.items.any((clip) => clip.mediaId == relinked.mediaId), isTrue);
    final recovery = await gateway.inspectRecovery(gateway.openedPath!);
    expect(recovery.kind, ProjectRecoveryKind.none);

    await tester.tap(find.byKey(const ValueKey('preview-play')));
    await _until(tester, () => gateway.playCalls > 0);
    expect(gateway.playResult!.frameSequence, greaterThan(BigInt.zero));
    expect(gateway.playResult!.errorCode, isNull);

    final noFlutterError = tester.takeException() == null;
    expect(noFlutterError, isTrue);
    final report = {
      'androidSafRecoveryAcceptance': {
        'checks': {
          'nativeDocumentsUiReopenedSameSafProject': true,
          'recoveryCandidateShownAfterProcessRestart': true,
          'explicitRecoveryApplied': true,
          'recoverySidecarRemovedAfterApply': true,
          'previewPlaybackResumedFromRecoveredProject': true,
          'relinkedMediaAndTimelineRecovered':
              relinked.mediaId == '00000041-2222-4222-8222-222222222222' &&
              relinked.sourceUri ==
                  'content://dev.opencut.saffixture.documents/document/relink-replacement' &&
              clips.items.any((clip) => clip.mediaId == relinked.mediaId),
          'relinkedMediaGrantSurvivedProcessRestart':
              (providerBeforeOpen['persistedMediaUris'] as List).contains(
                relinked.sourceUri,
              ),
          'noFlutterException': noFlutterError,
        },
        'relinkedMediaId': relinked.mediaId,
        'relinkedMediaSource': relinked.sourceUri,
        'projectPath': gateway.openedPath,
        'providerProjectSha256BeforeOpen': providerBeforeOpen['projectSha256'],
        'projectName': summary.name,
        'projectRevision': summary.revision.toString(),
        'recoveryKindAfterApply': recovery.kind.name,
        'frameSequence': gateway.playResult!.frameSequence.toString(),
      },
    };
    binding.reportData = {...?binding.reportData, ...report};
    debugPrint('ANDROID_SAF_RECOVERY_ACCEPTANCE_COMPLETE $report');
  });
}
