import 'package:flutter/material.dart';
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
    await _until(
      tester,
      () =>
          gateway.session != null && find.byType(Texture).evaluate().isNotEmpty,
    );

    expect(gateway.openedPath, contains('/files/or-projects/'));
    final summary = await gateway.summary(gateway.session!);
    expect(summary.name, 'Process recovery acceptance');
    expect(summary.revision, greaterThan(BigInt.zero));
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
          'noFlutterException': noFlutterError,
        },
        'projectPath': gateway.openedPath,
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
