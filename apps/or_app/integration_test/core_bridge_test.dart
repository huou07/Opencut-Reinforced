import 'dart:convert';
import 'dart:io';
import 'dart:ui' show Size;

import 'package:flutter/foundation.dart' show ValueKey;
import 'package:flutter/material.dart' show SnackBar, Text;
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:or_app/main.dart';
import 'package:or_app/project/project_file_picker.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/project/rust_project_gateway.dart';
import 'package:or_app/rust_core_gateway.dart';
import 'package:or_app_bridge/or_app_bridge.dart' show RustLib;

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

  testWidgets('native Flutter project lifecycle uses one Rust host', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 900);
    addTearDown(tester.view.resetPhysicalSize);
    final directory = Directory.systemTemp.createTempSync('or-flutter-cli-');
    addTearDown(() => directory.deleteSync(recursive: true));
    final projectPath = '${directory.path}/native-project.orproj';
    final picker = _NativeProjectPicker(projectPath);
    final gateway = _ObservedRustProjectGateway();
    const coreGateway = RustCoreGateway();

    await tester.pumpWidget(
      OrApp(
        gateway: coreGateway,
        projectGateway: gateway,
        projectFilePicker: picker,
      ),
    );
    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('new-project-name')),
      'Native Project',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
    await tester.pumpAndSettle();

    final session = gateway.activeSession;
    final errorMessage = tester
        .widgetList<Text>(
          find.descendant(
            of: find.byType(SnackBar),
            matching: find.byType(Text),
          ),
        )
        .map((text) => text.data)
        .whereType<String>()
        .join(' | ');
    expect(
      session,
      isNotNull,
      reason: '$errorMessage; native bridge error: ${gateway.lastError}',
    );
    final initial = await gateway.summary(session!);
    expect(initial.name, 'Native Project');
    expect(initial.revision, BigInt.zero);
    expect(initial.dirty, isFalse);
    final descriptorPath = initial.descriptorPath;

    await tester.tap(find.byKey(const ValueKey('or-brand-home')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('nav-settings')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('settings-section-advanced')));
    await tester.pumpAndSettle();
    expect(find.text('Local CLI session'), findsOneWidget);
    expect(find.text('Available'), findsOneWidget);
    expect(find.text(descriptorPath), findsOneWidget);
    expect(find.textContaining('token'), findsNothing);
    expect(find.textContaining('--attach "<descriptor-path>"'), findsOneWidget);
    await tester.tap(
      find.byKey(const ValueKey('settings-open-editor-preview')),
    );
    await tester.pumpAndSettle();

    final events = <ProjectHostEvent>[];
    final eventSubscription = gateway.watch(session).listen(events.add);
    addTearDown(eventSubscription.cancel);

    await _renameProject(tester, 'From Flutter');
    expect((await gateway.summary(session)).revision, BigInt.one);
    expect(find.text('Unsaved changes'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('workspace-undo')));
    await tester.pumpAndSettle();
    expect((await gateway.summary(session)).name, 'Native Project');
    expect((await gateway.summary(session)).revision, BigInt.two);

    await tester.tap(find.byKey(const ValueKey('workspace-redo')));
    await tester.pumpAndSettle();
    expect((await gateway.summary(session)).name, 'From Flutter');
    expect((await gateway.summary(session)).revision, BigInt.from(3));

    await tester.tap(find.byKey(const ValueKey('workspace-save')));
    await tester.pumpAndSettle();
    expect((await gateway.summary(session)).dirty, isFalse);
    expect((await gateway.summary(session)).revision, BigInt.from(3));

    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    expect(File(descriptorPath).existsSync(), isFalse);

    picker.openPath = projectPath;
    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    final reopened = await gateway.summary(gateway.activeSession!);
    expect(reopened.projectId, initial.projectId);
    expect(reopened.projectInstanceId, isNot(initial.projectInstanceId));
    expect(reopened.name, 'From Flutter');
    expect(reopened.revision, BigInt.from(3));
    expect(reopened.dirty, isFalse);
    expect(find.text('Timeline engine not implemented'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 150)),
    );
    expect(events.map((event) => event.sequence).toList(), [
      BigInt.one,
      BigInt.two,
      BigInt.from(3),
      BigInt.from(4),
      BigInt.from(5),
    ]);
    expect(events.map((event) => event.kind).toList(), [
      'project_changed',
      'project_changed',
      'project_changed',
      'project_saved',
      'session_closing',
    ]);
  });

  testWidgets(
    'native media bridge persists offline media through undo and save',
    (tester) async {
      final directory = Directory.systemTemp.createTempSync('or-media-bridge-');
      final projectPath = '${directory.path}/media-project.orproj';
      final missingSourcePath = '${directory.path}/offline-source.mkv';
      await File(projectPath).writeAsString(
        jsonEncode(
          _offlineMediaProject(Uri.file(missingSourcePath).toString()),
        ),
      );
      final gateway = _ObservedRustProjectGateway();
      addTearDown(() async {
        final session = gateway.activeSession;
        if (session != null) {
          await gateway.close(session, discardUnsaved: true);
        }
        directory.deleteSync(recursive: true);
      });

      final session = await gateway.openProject(projectPath);
      final original = await gateway.listMediaPage(
        session,
        offset: 0,
        limit: 50,
      );
      expect(original.items, hasLength(1));
      expect(
        original.items.single.mediaId,
        '22222222-2222-4222-8222-222222222222',
      );
      expect(original.items.single.formatNames, ['matroska']);
      expect(
        original.items.single.sourceUri,
        Uri.file(missingSourcePath).toString(),
      );
      expect(File(missingSourcePath).existsSync(), isFalse);

      final removed = await gateway.removeMedia(
        session,
        await gateway.summary(session),
        original.items.single.mediaId,
      );
      expect(removed.succeeded, isTrue);
      expect(
        (await gateway.listMediaPage(session, offset: 0, limit: 50)).items,
        isEmpty,
      );

      final undone = await gateway.undo(session, removed.view!);
      expect(undone.succeeded, isTrue);
      final restored = await gateway.listMediaPage(
        session,
        offset: 0,
        limit: 50,
      );
      expect(restored.items.single.mediaId, original.items.single.mediaId);
      expect(restored.items.single.sourceUri, original.items.single.sourceUri);
      expect(
        restored.items.single.formatNames,
        original.items.single.formatNames,
      );

      final saved = await gateway.save(session);
      expect(saved.succeeded, isTrue);
      expect(saved.view?.dirty, isFalse);
      await gateway.close(session, discardUnsaved: false);

      final reopenedSession = await gateway.openProject(projectPath);
      final reopened = await gateway.listMediaPage(
        reopenedSession,
        offset: 0,
        limit: 50,
      );
      expect(reopened.items, hasLength(1));
      expect(reopened.items.single.mediaId, original.items.single.mediaId);
      expect(reopened.items.single.sourceUri, original.items.single.sourceUri);
      expect(reopened.items.single.formatNames, ['matroska']);
      expect((await gateway.summary(reopenedSession)).dirty, isFalse);
      expect(File(missingSourcePath).existsSync(), isFalse);
    },
  );
}

Map<String, Object?> _offlineMediaProject(String sourceUri) => {
  'format': 'opencut-reinforced-project',
  'schema_version': 2,
  'project': {
    'id': '01234567-89ab-4def-8123-456789abcdef',
    'revision': 0,
    'name': 'Offline media fixture',
    'media': [
      {
        'id': '22222222-2222-4222-8222-222222222222',
        'source': {'kind': 'local_file', 'uri': sourceUri},
        'metadata': {
          'format_names': ['matroska'],
          'duration': null,
          'file_size_bytes': 0,
          'streams': [],
        },
      },
    ],
  },
};

Future<void> _renameProject(WidgetTester tester, String name) async {
  await tester.tap(find.byKey(const ValueKey('workspace-rename')));
  await tester.pumpAndSettle();
  await tester.enterText(
    find.byKey(const ValueKey('rename-project-name')),
    name,
  );
  await tester.tap(find.byKey(const ValueKey('confirm-rename-project')));
  await tester.pumpAndSettle();
  expect(
    tester
        .widget<Text>(find.byKey(const ValueKey('workspace-project-name')))
        .data,
    name,
  );
}

class _NativeProjectPicker implements ProjectFilePicker {
  _NativeProjectPicker(this.projectPath);

  final String projectPath;
  String? openPath;

  @override
  bool get isSupported => true;

  @override
  Future<String?> openProjectPath() async => openPath;

  @override
  Future<String?> openMediaPath() async => null;

  @override
  Future<String?> saveProjectPath({required String suggestedName}) async =>
      projectPath;
}

class _ObservedRustProjectGateway implements ProjectGateway {
  ProjectSessionHandle? activeSession;
  Object? lastError;
  static const _gateway = RustProjectGateway();

  @override
  Future<ProjectSessionHandle> createProject(String path, String name) async {
    try {
      return activeSession = await _gateway.createProject(path, name);
    } catch (error) {
      lastError = error;
      rethrow;
    }
  }

  @override
  Future<ProjectSessionHandle> openProject(String path) async =>
      activeSession = await _gateway.openProject(path);

  @override
  Future<ProjectReadModel> summary(ProjectSessionHandle session) =>
      _gateway.summary(session);

  @override
  Future<ProjectActionResult> rename(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String name,
  ) => _gateway.rename(session, current, name);

  @override
  Future<ProjectActionResult> undo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  ) => _gateway.undo(session, current);

  @override
  Future<ProjectActionResult> redo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  ) => _gateway.redo(session, current);

  @override
  Future<ProjectMediaPage> listMediaPage(
    ProjectSessionHandle session, {
    required int offset,
    required int limit,
  }) => _gateway.listMediaPage(session, offset: offset, limit: limit);

  @override
  Future<ProjectActionResult> importMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String path,
  ) => _gateway.importMedia(session, current, path);

  @override
  Future<ProjectActionResult> removeMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String mediaId,
  ) => _gateway.removeMedia(session, current, mediaId);

  @override
  Future<ProjectActionResult> save(ProjectSessionHandle session) =>
      _gateway.save(session);

  @override
  Future<void> close(
    ProjectSessionHandle session, {
    required bool discardUnsaved,
  }) async {
    await _gateway.close(session, discardUnsaved: discardUnsaved);
    if (identical(activeSession, session)) activeSession = null;
  }

  @override
  Stream<ProjectHostEvent> watch(ProjectSessionHandle session) =>
      _gateway.watch(session);

  @override
  Future<ProjectRecoveryInspection> inspectRecovery(String path) =>
      _gateway.inspectRecovery(path);

  @override
  Future<ProjectRecoveryActionResult> applyRecovery(String path) =>
      _gateway.applyRecovery(path);

  @override
  Future<ProjectRecoveryActionResult> discardRecovery(String path) =>
      _gateway.discardRecovery(path);
}
