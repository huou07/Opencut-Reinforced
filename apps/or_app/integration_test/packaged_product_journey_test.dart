import 'dart:io';
import 'dart:ui' show Size;

import 'package:flutter/foundation.dart' show ValueKey;
import 'package:flutter/material.dart'
    show
        IconButton,
        OutlinedButton,
        PopupMenuButton,
        SnackBar,
        TextButton,
        Widget;
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:or_app/main.dart';
import 'package:or_app/project/project_file_picker.dart';
import 'package:or_app/project/rust_project_gateway.dart';
import 'package:or_app/rust_core_gateway.dart';
import 'package:or_app_bridge/or_app_bridge.dart' show RustLib;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(RustLib.init);

  testWidgets('packaged desktop product journey', (tester) async {
    final phase = Platform.environment['OR_PACKAGED_JOURNEY_PHASE'];
    const forbiddenEnvironment = [
      'OR_FFMPEG_PATH',
      'LD_LIBRARY_PATH',
      'DYLD_LIBRARY_PATH',
      'DYLD_FALLBACK_LIBRARY_PATH',
      'DYLD_FRAMEWORK_PATH',
      'DYLD_ROOT_PATH',
      'DYLD_INSERT_LIBRARIES',
      'FFMPEG_DIR',
      'FFMPEG_INSTALL_PREFIX',
      'PKG_CONFIG_PATH',
    ];
    for (final name in forbiddenEnvironment) {
      expect(Platform.environment.containsKey(name), isFalse, reason: name);
    }
    expect(
      Platform.environment.containsKey('OR_FFPROBE_PATH'),
      phase == 'failures',
    );

    final projectPath = _requiredEnvironment('OR_PACKAGED_JOURNEY_PROJECT');
    final mediaPath = _requiredEnvironment('OR_PACKAGED_JOURNEY_MEDIA');
    final unsupportedMediaPath = _requiredEnvironment(
      'OR_PACKAGED_JOURNEY_UNSUPPORTED_MEDIA',
    );
    final missingMediaPath = _requiredEnvironment(
      'OR_PACKAGED_JOURNEY_MISSING_MEDIA',
    );
    final badProjectPath = _requiredEnvironment(
      'OR_PACKAGED_JOURNEY_BAD_PROJECT',
    );
    final exportPath = _requiredEnvironment('OR_PACKAGED_JOURNEY_EXPORT');
    final picker = _JourneyPicker(
      projectPath: projectPath,
      mediaPaths: [mediaPath, unsupportedMediaPath, missingMediaPath],
      exportPath: exportPath,
      badProjectPath: badProjectPath,
    );
    const gateway = RustProjectGateway();

    tester.view.physicalSize = const Size(1440, 900);
    addTearDown(tester.view.resetPhysicalSize);
    await tester.pumpWidget(
      OrApp(
        gateway: const RustCoreGateway(),
        projectGateway: gateway,
        projectFilePicker: picker,
      ),
    );

    switch (phase) {
      case 'create':
        await _createAndEdit(tester, picker);
        break;
      case 'reopen':
        await _reopenAndExport(tester, picker);
        break;
      case 'failures':
        await _preserveProjectAcrossFailures(tester, picker);
        break;
      default:
        fail('Unknown OR_PACKAGED_JOURNEY_PHASE: $phase');
    }
    expect(tester.takeException(), isNull);
  });
}

Future<void> _createAndEdit(WidgetTester tester, _JourneyPicker picker) async {
  await tester.tap(find.byKey(const ValueKey('home-new-project')));
  await tester.pumpAndSettle();
  await tester.enterText(
    find.byKey(const ValueKey('new-project-name')),
    'Packaged Journey',
  );
  await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
  await _pumpUntil(
    tester,
    () => find.text('No timeline tracks').evaluate().isNotEmpty,
    'new project workspace',
  );

  await tester.tap(find.byKey(const ValueKey('media-import')));
  await _pumpUntil(
    tester,
    () => find.text('tiny.mkv').evaluate().isNotEmpty,
    'supported video import',
  );
  final mediaId = _singleKeySuffix(tester, 'media-preview-');
  await _pumpUntil(
    tester,
    () => find
        .byKey(ValueKey('media-preview-image-$mediaId'))
        .evaluate()
        .isNotEmpty,
    'packaged thumbnail generation',
    attempts: 1800,
  );

  picker.nextMediaPath = picker.mediaPaths[1];
  await tester.tap(find.byKey(const ValueKey('media-import')));
  await _pumpUntil(
    tester,
    () => find
        .text('the media container is not in the import matrix')
        .evaluate()
        .isNotEmpty,
    'unsupported media feedback',
  );
  expect(find.text('unsupported.mp4'), findsNothing);

  await tester.tap(find.byKey(const ValueKey('timeline-add-video-track')));
  await _pumpUntil(
    tester,
    () => find.text('No timeline tracks').evaluate().isEmpty,
    'video track creation',
  );
  await tester.tap(find.byKey(ValueKey('media-add-timeline-$mediaId')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const ValueKey('timeline-confirm-insert')));
  await _pumpUntil(
    tester,
    () => _hasKeyPrefix(tester, 'timeline-clip-'),
    'media insertion into the timeline',
  );

  await _pumpUntil(
    tester,
    () => tester
        .widget<PopupMenuButton<dynamic>>(
          find.byKey(const ValueKey('preview-frame-rate')),
        )
        .enabled,
    'preview frame-rate control readiness',
  );
  await tester.tap(find.byKey(const ValueKey('preview-frame-rate')));
  await _pumpUntil(
    tester,
    () => find.text('24 fps').hitTestable().evaluate().isNotEmpty,
    '24 fps menu option readiness',
  );
  await tester.tap(find.text('24 fps').hitTestable());
  await _pumpUntil(
    tester,
    () =>
        tester
            .widget<IconButton>(find.byKey(const ValueKey('preview-play')))
            .onPressed !=
        null,
    'sequence frame rate configuration',
  );
  await tester.tap(find.byKey(const ValueKey('preview-play')));
  await tester.pump(const Duration(milliseconds: 300));
  await tester.tap(find.byKey(const ValueKey('preview-play')));

  await tester.tap(find.byKey(const ValueKey('workspace-rename')));
  await tester.pumpAndSettle();
  await tester.enterText(
    find.byKey(const ValueKey('rename-project-name')),
    'Packaged Journey Edited',
  );
  await tester.tap(find.byKey(const ValueKey('confirm-rename-project')));
  await _pumpUntil(
    tester,
    () => find.text('Packaged Journey Edited').evaluate().isNotEmpty,
    'project edit',
  );

  final saveButton = find.byKey(const ValueKey('workspace-save'));
  await tester.ensureVisible(saveButton);
  await _pumpUntil(
    tester,
    () =>
        tester.widget<TextButton>(saveButton).onPressed != null &&
        saveButton.hitTestable().evaluate().isNotEmpty,
    'project save control readiness',
  );
  await tester.tap(saveButton);
  await _pumpUntil(
    tester,
    () => find.text('Saved').evaluate().isNotEmpty,
    'project save',
  );
  await tester.tap(find.byKey(const ValueKey('workspace-close')));
  await _pumpUntil(
    tester,
    () => find.byKey(const ValueKey('home-open-project')).evaluate().isNotEmpty,
    'project close before application exit',
  );
}

Future<void> _reopenAndExport(
  WidgetTester tester,
  _JourneyPicker picker,
) async {
  await tester.tap(find.byKey(const ValueKey('home-open-project')));
  await _pumpUntil(
    tester,
    () =>
        find.text('Packaged Journey Edited').evaluate().isNotEmpty &&
        find.text('tiny.mkv').evaluate().isNotEmpty &&
        _hasKeyPrefix(tester, 'timeline-clip-'),
    'saved project reopen with media and timeline',
    attempts: 900,
  );

  await tester.tap(find.byKey(const ValueKey('preview-play')));
  await tester.pump(const Duration(milliseconds: 300));
  await tester.tap(find.byKey(const ValueKey('preview-play')));

  await tester.tap(find.byKey(const ValueKey('export-project')));
  await _pumpUntil(
    tester,
    () => find.text('Export complete').evaluate().isNotEmpty,
    'export to the existing destination',
    attempts: 1800,
  );
  expect(File(picker.exportPath).lengthSync(), greaterThan(0));
}

Future<void> _preserveProjectAcrossFailures(
  WidgetTester tester,
  _JourneyPicker picker,
) async {
  await tester.tap(find.byKey(const ValueKey('home-open-project')));
  await _pumpUntil(
    tester,
    () =>
        find.text('Packaged Journey Edited').evaluate().isNotEmpty &&
        find.text('tiny.mkv').evaluate().isNotEmpty,
    'project reopen before failure checks',
  );

  final importButton = find.byKey(const ValueKey('media-import'));
  picker.nextMediaPath = picker.mediaPaths[2];
  await _waitForImportButton(tester, importButton);
  await tester.tap(importButton);
  await _pumpUntil(
    tester,
    () => find
        .text(
          'Imported 0 of 1 selected files. 1 could not be imported. '
          'The selected media file could not be found. Check that the source still exists.',
        )
        .evaluate()
        .isNotEmpty,
    'unavailable external media feedback',
  );
  expect(find.text('tiny.mkv'), findsWidgets);

  picker.nextMediaPath = picker.mediaPaths[0];
  await _waitForImportButton(tester, importButton);
  await tester.tap(importButton);
  await _pumpUntil(
    tester,
    () => find
        .text(
          'The packaged media inspector could not start. Check the app installation and try again.',
        )
        .evaluate()
        .isNotEmpty,
    'missing packaged probe feedback',
  );
  expect(find.text('tiny.mkv'), findsWidgets);

  picker.nextExportPath = _requiredEnvironment(
    'OR_PACKAGED_JOURNEY_FAILED_EXPORT',
  );
  await tester.tap(find.byKey(const ValueKey('export-project')));
  await _pumpUntil(
    tester,
    () => find
        .textContaining('could not publish the completed export')
        .evaluate()
        .isNotEmpty,
    'failed replacement feedback',
    attempts: 1800,
  );

  await _pumpUntil(
    tester,
    () => find.byType(SnackBar).evaluate().isEmpty,
    'transient failure feedback dismissal',
  );
  await tester.tap(find.byKey(const ValueKey('or-brand-home')));
  await tester.pumpAndSettle();
  picker.openPath = picker.badProjectPath;
  await tester.tap(find.byKey(const ValueKey('home-open-project')));
  await _pumpUntil(
    tester,
    () => find.byType(SnackBar).evaluate().isNotEmpty,
    'failed project reopen feedback',
  );
  expect(File(picker.projectPath).existsSync(), isTrue);
  expect(File(picker.projectPath).lengthSync(), greaterThan(0));
}

String _requiredEnvironment(String name) {
  final value = Platform.environment[name];
  if (value == null || value.isEmpty) {
    fail('Missing environment variable $name');
  }
  return value;
}

String _singleKeySuffix(WidgetTester tester, String prefix) {
  final values = tester
      .widgetList<Widget>(
        find.byWidgetPredicate((widget) {
          final key = widget.key;
          return key is ValueKey<String> &&
              key.value.startsWith(prefix) &&
              !key.value.startsWith('media-preview-image-');
        }),
      )
      .map((widget) => (widget.key as ValueKey<String>).value)
      .where((value) => value.length > prefix.length)
      .toSet();
  expect(values, hasLength(1));
  return values.single.substring(prefix.length);
}

bool _hasKeyPrefix(WidgetTester tester, String prefix) => tester
    .widgetList<Widget>(
      find.byWidgetPredicate((widget) {
        final key = widget.key;
        return key is ValueKey<String> && key.value.startsWith(prefix);
      }),
    )
    .isNotEmpty;

Future<void> _pumpUntil(
  WidgetTester tester,
  bool Function() ready,
  String operation, {
  int attempts = 600,
}) async {
  for (var attempt = 0; attempt < attempts; attempt++) {
    if (ready()) return;
    await tester.pump(const Duration(milliseconds: 100));
  }
  fail('Timed out waiting for $operation.');
}

Future<void> _waitForImportButton(WidgetTester tester, Finder button) async {
  await tester.ensureVisible(button);
  await _pumpUntil(
    tester,
    () =>
        tester.widget<OutlinedButton>(button).onPressed != null &&
        button.hitTestable().evaluate().isNotEmpty,
    'media import control readiness',
  );
}

class _JourneyPicker implements ProjectFilePicker {
  _JourneyPicker({
    required this.projectPath,
    required this.mediaPaths,
    required this.exportPath,
    required this.badProjectPath,
  }) : nextMediaPath = mediaPaths.first,
       nextExportPath = exportPath;

  final String projectPath;
  final List<String> mediaPaths;
  final String exportPath;
  final String badProjectPath;
  String? openPath;
  String nextMediaPath;
  String nextExportPath;

  @override
  bool get isSupported => true;

  @override
  bool get supportsMediaImport => true;

  @override
  bool get supportsExport => true;

  @override
  Future<String?> openProjectPath() async => openPath ?? projectPath;

  @override
  Future<List<String>> openMediaSources() async => [nextMediaPath];

  @override
  Future<String?> saveProjectPath({required String suggestedName}) async =>
      projectPath;

  @override
  Future<String?> saveExportPath({required String suggestedName}) async =>
      nextExportPath;

  @override
  Future<void> publishExportPath(String path) async {}

  @override
  Future<void> discardExportPath(String path) async {}

  @override
  Future<void> cancelExportPublish(String path) async {}

  @override
  Future<ProjectFileSyncResult?> synchronizeProjectPath(String path) async =>
      null;
}
