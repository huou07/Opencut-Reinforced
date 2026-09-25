import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/core_gateway.dart';
import 'package:or_app/design/or_colors.dart';
import 'package:or_app/design/or_spacing.dart';
import 'package:or_app/main.dart';
import 'package:or_app/project/project_file_picker.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/shell/app_navigation.dart';

void main() {
  testWidgets('wide shell shows OR navigation and project destinations', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    await _mount(tester);

    expect(find.byKey(const ValueKey('app-top-bar')), findsOneWidget);
    expect(find.byKey(const ValueKey('compact-navigation')), findsNothing);
    expect(find.byKey(const ValueKey('nav-home')), findsOneWidget);
    expect(find.text('Opencut Reinforced'), findsOneWidget);
    expect(find.text('Home'), findsWidgets);
    expect(find.text('New Project'), findsOneWidget);
    expect(find.text('Open Project'), findsOneWidget);
    expect(find.text('No recent projects yet'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('nav-projects')));
    await tester.pumpAndSettle();
    expect(find.text('No active project'), findsOneWidget);
  });

  testWidgets('compact shell reaches all five primary destinations', (
    tester,
  ) async {
    _setViewport(tester, const Size(390, 844));
    await _mount(tester);
    expect(find.byKey(const ValueKey('compact-navigation')), findsOneWidget);

    final destinations = [
      (AppDestination.projects, 'No active project'),
      (AppDestination.templates, 'No templates available'),
      (AppDestination.assets, 'No assets available'),
      (AppDestination.settings, 'Focused Monochrome'),
      (AppDestination.home, 'No recent projects yet'),
    ];
    for (final (destination, expectedText) in destinations) {
      await tester.tap(find.byKey(ValueKey('nav-${destination.name}')));
      await tester.pumpAndSettle();
      expect(find.text(expectedText), findsOneWidget);
      expect(tester.takeException(), isNull);
    }
  });

  testWidgets('medium desktop page header fits beside the navigation rail', (
    tester,
  ) async {
    _setViewport(tester, const Size(760, 900));
    await _mount(tester);
    await tester.tap(find.byKey(const ValueKey('nav-projects')));
    await tester.pumpAndSettle();
    expect(find.text('Projects'), findsWidgets);
    expect(tester.takeException(), isNull);
  });

  test('Focused Monochrome tokens and responsive policy are centralized', () {
    expect(OrColors.background.toARGB32(), 0xFF090909);
    expect(OrColors.surface.toARGB32(), 0xFF151516);
    expect(OrColors.border.toARGB32(), 0xFF2A2A2D);
    expect(OrColors.text.toARGB32(), 0xFFF5F5F5);
    expect(OrColors.selection.toARGB32(), 0xFF3FC7FF);
    expect(OrColors.ai.toARGB32(), 0xFF9B7CFF);
    expect(OrSpacing.x1, 4);
    expect(OrSpacing.x12, 48);
    expect(OrRadii.control, 8);
    expect(OrRadii.panel, 12);
    expect(OrBreakpoints.isCompact(759), isTrue);
    expect(OrBreakpoints.isCompact(760), isFalse);
    expect(OrBreakpoints.isWide(1179), isFalse);
    expect(OrBreakpoints.isWide(1180), isTrue);
  });

  testWidgets('settings diagnostics remain sourced from CoreGateway', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    await _mount(tester);
    await tester.tap(find.byKey(const ValueKey('nav-settings')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('settings-section-advanced')));
    await tester.pumpAndSettle();

    expect(find.text('Fixture Core App'), findsOneWidget);
    expect(find.text('9.8.7-test'), findsOneWidget);
    expect(find.text('healthy-from-gateway'), findsOneWidget);
    expect(find.text('core.app_info'), findsOneWidget);
    expect(find.text('core.health'), findsOneWidget);
    expect(find.text('core.capabilities'), findsOneWidget);
    expect(find.text('Hardcoded Flutter diagnostics'), findsNothing);
  });

  testWidgets('new project asks for a name and opens the created workspace', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/or-widget-demo';
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('new-project-name')), findsOneWidget);
    expect(gateway.createCalls, 0);
    await tester.enterText(
      find.byKey(const ValueKey('new-project-name')),
      'Demo Project ',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
    await tester.pumpAndSettle();

    expect(picker.saveCalls, 1);
    expect(gateway.lastCreatePath, '/tmp/or-widget-demo.orproj');
    expect(gateway.lastCreateName, 'Demo Project ');
    expect(gateway.inspectCalls, 1);
    expect(gateway.createCalls, 1);
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
    expect(find.text('Revision 0'), findsOneWidget);
    expect(find.text('Saved'), findsOneWidget);
    expect(find.text('Timeline engine not implemented'), findsOneWidget);
    expect(find.text('No media loaded'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('canceling the new-project name or location creates nothing', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = null;
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(picker.saveCalls, 0);
    expect(gateway.createCalls, 0);

    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('new-project-name')),
      'Cancelled at picker',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
    await tester.pumpAndSettle();
    expect(picker.saveCalls, 1);
    expect(gateway.createCalls, 0);
    expect(find.text('Editor layout preview'), findsNothing);
  });

  testWidgets('open project uses the picker and Rust read model', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..openPath = '/tmp/opened.orproj';
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    expect(picker.openCalls, 1);
    expect(gateway.lastOpenPath, '/tmp/opened.orproj');
    expect(gateway.openCalls, 1);
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('workspace-project-revision')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'Projects shows one active project row and only saves when dirty',
    (tester) async {
      _setViewport(tester, const Size(1440, 900));
      final gateway = _FakeProjectGateway();
      final picker = _FakeProjectPicker()..savePath = '/tmp/active.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Active');
      await tester.tap(find.byKey(const ValueKey('or-brand-home')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('nav-projects')));
      await tester.pumpAndSettle();

      expect(find.byKey(const ValueKey('active-project-card')), findsOneWidget);
      expect(find.text('Active Project'), findsOneWidget);
      expect(find.text('Revision 0'), findsOneWidget);
      expect(find.byKey(const ValueKey('active-project-save')), findsNothing);
      expect(find.text('Recent Project'), findsNothing);
    },
  );

  testWidgets('rename marks dirty and explicit save keeps the revision', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/save.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Before');

    await tester.tap(find.byKey(const ValueKey('workspace-rename')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('rename-project-name')),
      'After',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-rename-project')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
    expect(find.text('Unsaved changes'), findsOneWidget);
    expect(find.text('Revision 1'), findsOneWidget);
    final revisionBeforeSave = gateway.lastSession!.view.revision;

    await tester.tap(find.byKey(const ValueKey('workspace-save')));
    await tester.pumpAndSettle();
    expect(gateway.saveCalls, 1);
    expect(gateway.lastSession!.view.dirty, isFalse);
    expect(gateway.lastSession!.view.revision, revisionBeforeSave);
    expect(find.text('Saved'), findsOneWidget);
  });

  testWidgets('save failure leaves the project open and dirty', (tester) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()
      ..nextSaveFailure = 'PROJECT_FILE_CHANGED';
    final picker = _FakeProjectPicker()..savePath = '/tmp/fail-save.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Dirty');
    await tester.tap(find.byKey(const ValueKey('workspace-rename')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('rename-project-name')),
      'Changed',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-rename-project')));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('workspace-save')));
    await tester.pumpAndSettle();
    expect(gateway.lastSession!.view.dirty, isTrue);
    expect(gateway.closeCalls, 0);
    expect(find.text('Unsaved changes'), findsOneWidget);
    expect(find.textContaining('save was blocked'), findsOneWidget);
  });

  testWidgets('close guard supports cancel, discard, and save', (tester) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/close.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Close me');
    await _renameActiveProject(tester, 'Unsaved');

    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    expect(find.text('Save your changes before continuing?'), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(gateway.closeCalls, 0);
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Discard'));
    await tester.pumpAndSettle();
    expect(gateway.closeCalls, 1);
    expect(gateway.lastCloseDiscard, isTrue);
    expect(find.text('No recent projects yet'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('new-project-name')),
      'Save to close',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
    await tester.pumpAndSettle();
    await _renameActiveProject(tester, 'Save me');
    await tester.tap(find.byKey(const ValueKey('workspace-close')));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Save'));
    await tester.pumpAndSettle();
    expect(gateway.saveCalls, 1);
    expect(gateway.closeCalls, 2);
    expect(gateway.lastCloseDiscard, isFalse);
    expect(find.text('No recent projects yet'), findsOneWidget);
  });

  testWidgets('revision conflict refreshes once and does not retry rename', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()..nextRenameConflict = true;
    final picker = _FakeProjectPicker()..savePath = '/tmp/conflict.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Flutter name');

    await tester.tap(find.byKey(const ValueKey('workspace-rename')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('rename-project-name')),
      'Unsent rename',
    );
    await tester.tap(find.byKey(const ValueKey('confirm-rename-project')));
    await tester.pumpAndSettle();

    expect(gateway.renameCalls, 1);
    expect(gateway.summaryCalls, greaterThanOrEqualTo(2));
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
    expect(find.textContaining('this action was not retried'), findsOneWidget);
  });

  testWidgets(
    'dirty project switch can cancel or save before opening another',
    (tester) async {
      _setViewport(tester, const Size(1440, 900));
      final gateway = _FakeProjectGateway();
      final picker = _FakeProjectPicker()..savePath = '/tmp/switch.orproj';
      await _mount(tester, gateway: gateway, picker: picker);
      await _createProject(tester, 'Current project');
      await _renameActiveProject(tester, 'Current dirty project');
      picker.openPath = '/tmp/next.orproj';
      await tester.tap(find.byKey(const ValueKey('or-brand-home')));
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const ValueKey('home-open-project')));
      await tester.pumpAndSettle();
      expect(find.text('Save your changes before continuing?'), findsOneWidget);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(gateway.openCalls, 0);
      expect(gateway.closeCalls, 0);
      expect(gateway.lastSession!.closed, isFalse);

      await tester.tap(find.byKey(const ValueKey('home-open-project')));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, 'Save'));
      await tester.pumpAndSettle();
      expect(gateway.saveCalls, 1);
      expect(gateway.closeCalls, 1);
      expect(gateway.lastCloseDiscard, isFalse);
      expect(gateway.openCalls, 1);
      expect(
        tester
            .widget<Text>(find.byKey(const ValueKey('workspace-project-name')))
            .data,
        'Opened Project',
      );
    },
  );

  testWidgets('attached-session invalidations refresh from summary', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/events.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Initial');
    final before = gateway.lastSession!.view.revision;
    gateway.externalRename('From attached CLI');
    await tester.pumpAndSettle();

    expect(gateway.lastSession!.view.revision, before + BigInt.one);
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
    expect(find.text('Revision 1'), findsOneWidget);
    expect(find.text('Unsaved changes'), findsOneWidget);
  });

  testWidgets('candidate recovery offers recover discard and cancel', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()
      ..recoveryInspection = _inspection(ProjectRecoveryKind.candidate);
    final picker = _FakeProjectPicker()..openPath = '/tmp/candidate.orproj';
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    expect(find.text('Saved revision: 4'), findsOneWidget);
    expect(find.text('Recovery revision: 6'), findsOneWidget);
    expect(find.text('Recover'), findsOneWidget);
    expect(find.text('Discard'), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(gateway.applyRecoveryCalls, 0);
    expect(gateway.discardRecoveryCalls, 0);
    expect(gateway.openCalls, 0);

    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Discard'));
    await tester.pumpAndSettle();
    expect(gateway.discardRecoveryCalls, 1);
    expect(gateway.openCalls, 1);
    expect(
      find.byKey(const ValueKey('workspace-project-name')),
      findsOneWidget,
    );
  });

  testWidgets(
    'recovery conflict needs confirmed discard and never offers recover',
    (tester) async {
      _setViewport(tester, const Size(1440, 900));
      final gateway = _FakeProjectGateway()
        ..recoveryInspection = _inspection(ProjectRecoveryKind.conflict);
      final picker = _FakeProjectPicker()..openPath = '/tmp/conflicting.orproj';
      await _mount(tester, gateway: gateway, picker: picker);

      await tester.tap(find.byKey(const ValueKey('home-open-project')));
      await tester.pumpAndSettle();
      expect(find.text('Recovery checkpoint conflict'), findsOneWidget);
      expect(find.text('Recover'), findsNothing);
      await tester.tap(find.text('Discard checkpoint'));
      await tester.pumpAndSettle();
      expect(find.text('Discard recovery data?'), findsOneWidget);
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();
      expect(gateway.discardRecoveryCalls, 0);
      expect(gateway.openCalls, 0);

      await tester.tap(find.byKey(const ValueKey('home-open-project')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Discard checkpoint'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Discard'));
      await tester.pumpAndSettle();
      expect(gateway.discardRecoveryCalls, 1);
      expect(gateway.openCalls, 1);
    },
  );

  testWidgets('invalid recovery requires explicit confirmation', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()
      ..recoveryInspection = _inspection(ProjectRecoveryKind.invalid);
    final picker = _FakeProjectPicker()..openPath = '/tmp/invalid.orproj';
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    expect(find.text('Invalid recovery checkpoint'), findsOneWidget);
    expect(
      find.textContaining('malformed or failed validation'),
      findsOneWidget,
    );
    await tester.tap(find.text('Discard checkpoint'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Discard'));
    await tester.pumpAndSettle();
    expect(gateway.discardRecoveryCalls, 1);
    expect(gateway.openCalls, 1);
  });

  testWidgets('stale recovery opens canonical project and can be discarded', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway()
      ..recoveryInspection = _inspection(ProjectRecoveryKind.stale);
    final picker = _FakeProjectPicker()..openPath = '/tmp/stale.orproj';
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    expect(gateway.openCalls, 1);
    expect(gateway.applyRecoveryCalls, 0);
    expect(gateway.discardRecoveryCalls, 0);
    expect(
      find.textContaining('stale recovery checkpoint is present'),
      findsOneWidget,
    );
    await tester.tap(find.byKey(const ValueKey('discard-stale-recovery')));
    await tester.pumpAndSettle();
    expect(gateway.discardRecoveryCalls, 1);
    expect(find.byKey(const ValueKey('project-notice')), findsNothing);
  });

  testWidgets('Android lifecycle remains unavailable without calling picker', (
    tester,
  ) async {
    _setViewport(tester, const Size(390, 844));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker(supported: false);
    await _mount(tester, gateway: gateway, picker: picker);

    await tester.tap(find.byKey(const ValueKey('home-new-project')));
    await tester.pumpAndSettle();
    expect(
      find.text(
        'Project file access on Android requires Storage Access Framework integration and is not available in this Developer Preview.',
      ),
      findsOneWidget,
    );
    await tester.tap(find.byKey(const ValueKey('home-open-project')));
    await tester.pumpAndSettle();
    expect(picker.saveCalls, 0);
    expect(picker.openCalls, 0);
    expect(gateway.createCalls, 0);
    expect(gateway.openCalls, 0);
  });

  testWidgets('keyboard shortcuts use the same live gateway operations', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/keys.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Key project');
    await _renameActiveProject(tester, 'Renamed');

    await _sendShortcut(tester, LogicalKeyboardKey.keyZ);
    expect(gateway.lastSession!.view.name, 'Key project');
    await _sendShortcut(tester, LogicalKeyboardKey.keyZ, shift: true);
    expect(gateway.lastSession!.view.name, 'Renamed');
    await _sendShortcut(tester, LogicalKeyboardKey.keyS);
    expect(gateway.saveCalls, 1);
    expect(gateway.lastSession!.view.dirty, isFalse);
  });

  testWidgets('dirty application exit can cancel and clean exit closes host', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/exit.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Exit project');
    await _renameActiveProject(tester, 'Exit dirty');

    final cancelledExit = WidgetsBinding.instance.handleRequestAppExit();
    await tester.pumpAndSettle();
    expect(find.text('Unsaved project changes'), findsOneWidget);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(await cancelledExit, ui.AppExitResponse.cancel);
    expect(gateway.closeCalls, 0);

    await tester.tap(find.byKey(const ValueKey('workspace-save')));
    await tester.pumpAndSettle();
    expect(
      await WidgetsBinding.instance.handleRequestAppExit(),
      ui.AppExitResponse.exit,
    );
    expect(gateway.closeCalls, 1);
    expect(gateway.lastCloseDiscard, isFalse);
  });

  testWidgets('command palette exposes lifecycle operations while active', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    final gateway = _FakeProjectGateway();
    final picker = _FakeProjectPicker()..savePath = '/tmp/palette.orproj';
    await _mount(tester, gateway: gateway, picker: picker);
    await _createProject(tester, 'Palette');

    await _sendShortcut(tester, LogicalKeyboardKey.keyK);
    await tester.enterText(
      find.byKey(const ValueKey('command-palette-query')),
      'Close Project',
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('command-close-project')), findsOneWidget);
  });

  testWidgets('desktop Editor Shell Preview stays distinct without a project', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    await _mount(tester);
    await tester.tap(find.byKey(const ValueKey('home-editor-preview')));
    await tester.pumpAndSettle();

    expect(find.text('Editor layout preview'), findsOneWidget);
    expect(find.text('Viewer'), findsOneWidget);
    expect(find.text('Timeline'), findsOneWidget);
    expect(find.text('Inspector'), findsOneWidget);
    expect(find.text('No media loaded'), findsOneWidget);
    expect(find.text('No selection'), findsOneWidget);
    expect(find.text('Timeline engine not implemented'), findsOneWidget);
    expect(find.textContaining('.mp4'), findsNothing);
    expect(find.byKey(const ValueKey('nav-editorPreview')), findsNothing);
  });

  testWidgets('compact Editor Shell Preview uses a tool sheet', (tester) async {
    _setViewport(tester, const Size(390, 844));
    await _mount(tester);
    await tester.ensureVisible(
      find.byKey(const ValueKey('home-editor-preview')),
    );
    await tester.tap(find.byKey(const ValueKey('home-editor-preview')));
    await tester.pumpAndSettle();

    expect(find.text('No media loaded'), findsOneWidget);
    expect(find.text('Timeline engine not implemented'), findsOneWidget);
    expect(find.byKey(const ValueKey('preview-play')), findsOneWidget);
    expect(
      find.byKey(const ValueKey('mobile-editor-tool-dock')),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('editor-tool-media')), findsNothing);
    expect(
      find.byKey(const ValueKey('mobile-editor-tool-media')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);

    await tester.tap(find.byKey(const ValueKey('mobile-editor-tool-media')));
    await tester.pumpAndSettle();
    expect(find.text('Unavailable in this Developer Preview'), findsOneWidget);
    await tester.tap(find.byTooltip('Close tool panel'));
    await tester.pumpAndSettle();
    expect(find.text('Timeline engine not implemented'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

ProjectRecoveryInspection _inspection(ProjectRecoveryKind kind) =>
    ProjectRecoveryInspection(
      kind: kind,
      projectId: 'project-id',
      baseRevision: BigInt.from(4),
      recoveryRevision: BigInt.from(6),
      recoveryName: 'Recovered Demo',
      conflictReason: 'project_id_mismatch',
      message: 'internal fixture details are not shown',
    );

Future<void> _mount(
  WidgetTester tester, {
  _FakeProjectGateway? gateway,
  _FakeProjectPicker? picker,
}) async {
  await tester.pumpWidget(
    OrApp(
      gateway: _FakeCoreGateway(),
      projectGateway: gateway ?? _FakeProjectGateway(),
      projectFilePicker: picker ?? _FakeProjectPicker(),
    ),
  );
  await tester.pumpAndSettle();
}

Future<void> _createProject(WidgetTester tester, String name) async {
  await tester.tap(find.byKey(const ValueKey('home-new-project')));
  await tester.pumpAndSettle();
  await tester.enterText(find.byKey(const ValueKey('new-project-name')), name);
  await tester.tap(find.byKey(const ValueKey('confirm-new-project')));
  await tester.pumpAndSettle();
}

Future<void> _renameActiveProject(WidgetTester tester, String name) async {
  await tester.tap(find.byKey(const ValueKey('workspace-rename')));
  await tester.pumpAndSettle();
  await tester.enterText(
    find.byKey(const ValueKey('rename-project-name')),
    name,
  );
  await tester.tap(find.byKey(const ValueKey('confirm-rename-project')));
  await tester.pumpAndSettle();
}

Future<void> _sendShortcut(
  WidgetTester tester,
  LogicalKeyboardKey key, {
  bool shift = false,
}) async {
  await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
  if (shift) await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
  await tester.sendKeyDownEvent(key);
  await tester.sendKeyUpEvent(key);
  if (shift) await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
  await tester.pumpAndSettle();
}

void _setViewport(WidgetTester tester, Size size) {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
}

class _FakeCoreGateway implements CoreGateway {
  @override
  Future<AppInfo> appInfo() async => const AppInfo(
    name: 'Fixture Core App',
    version: '9.8.7-test',
    coreApiVersion: 4,
  );

  @override
  Future<HealthStatus> health() async =>
      const HealthStatus(status: 'healthy-from-gateway');

  @override
  Future<List<Capability>> capabilities() async => const [
    Capability(id: 'core.app_info', version: 2),
    Capability(id: 'core.health', version: 3),
    Capability(id: 'core.capabilities', version: 4),
  ];
}

class _FakeProjectPicker implements ProjectFilePicker {
  _FakeProjectPicker({this.supported = true});

  final bool supported;
  String? openPath;
  String? savePath;
  int openCalls = 0;
  int saveCalls = 0;

  @override
  bool get isSupported => supported;

  @override
  Future<String?> openProjectPath() async {
    openCalls++;
    return openPath;
  }

  @override
  Future<String?> saveProjectPath({required String suggestedName}) async {
    saveCalls++;
    return savePath;
  }
}

class _FakeProjectGateway implements ProjectGateway {
  final Map<String, _FakeSession> _sessionsByPath = {};
  final Map<String, ProjectReadModel> _savedByPath = {};
  _FakeSession? lastSession;
  ProjectRecoveryInspection recoveryInspection = _inspection(
    ProjectRecoveryKind.none,
  );
  String? nextSaveFailure;
  bool nextRenameConflict = false;
  String? lastCreatePath;
  String? lastCreateName;
  String? lastOpenPath;
  int createCalls = 0;
  int openCalls = 0;
  int summaryCalls = 0;
  int renameCalls = 0;
  int saveCalls = 0;
  int closeCalls = 0;
  bool? lastCloseDiscard;
  int inspectCalls = 0;
  int applyRecoveryCalls = 0;
  int discardRecoveryCalls = 0;

  @override
  Future<ProjectSessionHandle> createProject(String path, String name) async {
    createCalls++;
    lastCreatePath = path;
    lastCreateName = name;
    final view = ProjectReadModel(
      projectId: 'project-id',
      projectInstanceId: 'instance-$createCalls',
      revision: BigInt.zero,
      name: name,
      dirty: false,
      descriptorPath: '/tmp/or-session-$createCalls.json',
    );
    final session = _FakeSession(path, view);
    _sessionsByPath[path] = session;
    _savedByPath[path] = view;
    lastSession = session;
    return session;
  }

  @override
  Future<ProjectSessionHandle> openProject(String path) async {
    openCalls++;
    lastOpenPath = path;
    final saved =
        _savedByPath[path] ??
        ProjectReadModel(
          projectId: 'project-id',
          projectInstanceId: 'instance-open-$openCalls',
          revision: BigInt.zero,
          name: 'Opened Project',
          dirty: false,
          descriptorPath: '/tmp/or-session-open-$openCalls.json',
        );
    final session = _FakeSession(
      path,
      ProjectReadModel(
        projectId: saved.projectId,
        projectInstanceId: 'instance-open-$openCalls',
        revision: saved.revision,
        name: saved.name,
        dirty: false,
        descriptorPath: '/tmp/or-session-open-$openCalls.json',
      ),
    );
    _sessionsByPath[path] = session;
    lastSession = session;
    return session;
  }

  @override
  Future<ProjectReadModel> summary(ProjectSessionHandle handle) async {
    summaryCalls++;
    return _session(handle).view;
  }

  @override
  Future<ProjectActionResult> rename(
    ProjectSessionHandle handle,
    ProjectReadModel current,
    String name,
  ) async {
    renameCalls++;
    final session = _session(handle);
    if (nextRenameConflict) {
      nextRenameConflict = false;
      session.view = _copyView(
        session.view,
        name: 'Changed in CLI',
        revision: session.view.revision + BigInt.one,
        dirty: true,
      );
      session.emit('project_changed');
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'REVISION_CONFLICT',
        message: 'revision changed',
      );
    }
    if (current.revision != session.view.revision) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'REVISION_CONFLICT',
        message: 'revision changed',
      );
    }
    if (name == session.view.name) {
      return ProjectActionResult(succeeded: true, view: session.view);
    }
    session.undo.add(session.view.name);
    session.redo.clear();
    session.view = _copyView(
      session.view,
      name: name,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> undo(
    ProjectSessionHandle handle,
    ProjectReadModel current,
  ) async {
    final session = _session(handle);
    if (current.revision != session.view.revision) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'REVISION_CONFLICT',
      );
    }
    if (session.undo.isEmpty) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'NOTHING_TO_UNDO',
      );
    }
    session.redo.add(session.view.name);
    final name = session.undo.removeLast();
    session.view = _copyView(
      session.view,
      name: name,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> redo(
    ProjectSessionHandle handle,
    ProjectReadModel current,
  ) async {
    final session = _session(handle);
    if (current.revision != session.view.revision) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'REVISION_CONFLICT',
      );
    }
    if (session.redo.isEmpty) {
      return const ProjectActionResult(
        succeeded: false,
        errorCode: 'NOTHING_TO_REDO',
      );
    }
    session.undo.add(session.view.name);
    final name = session.redo.removeLast();
    session.view = _copyView(
      session.view,
      name: name,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<ProjectActionResult> save(ProjectSessionHandle handle) async {
    saveCalls++;
    final session = _session(handle);
    final failure = nextSaveFailure;
    nextSaveFailure = null;
    if (failure != null) {
      return ProjectActionResult(
        succeeded: false,
        errorCode: failure,
        message: 'external file changed',
      );
    }
    session.view = _copyView(session.view, dirty: false);
    _savedByPath[session.path] = session.view;
    session.emit('project_saved');
    return ProjectActionResult(succeeded: true, view: session.view);
  }

  @override
  Future<void> close(
    ProjectSessionHandle handle, {
    required bool discardUnsaved,
  }) async {
    final session = _session(handle);
    if (session.view.dirty && !discardUnsaved) {
      throw const ProjectGatewayException('UNSAVED_CHANGES', 'dirty');
    }
    closeCalls++;
    lastCloseDiscard = discardUnsaved;
    session.closed = true;
    session.emit('session_closing');
  }

  @override
  Stream<ProjectHostEvent> watch(ProjectSessionHandle handle) =>
      _session(handle).events.stream;

  @override
  Future<ProjectRecoveryInspection> inspectRecovery(String path) async {
    inspectCalls++;
    return recoveryInspection;
  }

  @override
  Future<ProjectRecoveryActionResult> applyRecovery(String path) async {
    applyRecoveryCalls++;
    return const ProjectRecoveryActionResult(
      succeeded: true,
      changed: true,
      message: 'Recovery applied.',
    );
  }

  @override
  Future<ProjectRecoveryActionResult> discardRecovery(String path) async {
    discardRecoveryCalls++;
    return const ProjectRecoveryActionResult(
      succeeded: true,
      changed: true,
      message: 'Recovery checkpoint discarded.',
    );
  }

  void externalRename(String name) {
    final session = lastSession!;
    session.view = _copyView(
      session.view,
      name: name,
      revision: session.view.revision + BigInt.one,
      dirty: true,
    );
    session.emit('project_changed');
  }

  _FakeSession _session(ProjectSessionHandle handle) {
    if (handle is _FakeSession) return handle;
    throw ArgumentError.value(handle);
  }
}

ProjectReadModel _copyView(
  ProjectReadModel view, {
  String? name,
  BigInt? revision,
  bool? dirty,
}) => ProjectReadModel(
  projectId: view.projectId,
  projectInstanceId: view.projectInstanceId,
  revision: revision ?? view.revision,
  name: name ?? view.name,
  dirty: dirty ?? view.dirty,
  descriptorPath: view.descriptorPath,
);

class _FakeSession implements ProjectSessionHandle {
  _FakeSession(this.path, this.view);

  final String path;
  ProjectReadModel view;
  final List<String> undo = [];
  final List<String> redo = [];
  final StreamController<ProjectHostEvent> events =
      StreamController<ProjectHostEvent>.broadcast(sync: true);
  int sequence = 0;
  bool closed = false;

  void emit(String kind) {
    events.add(
      ProjectHostEvent(
        sequence: BigInt.from(++sequence),
        kind: kind,
        projectId: view.projectId,
        projectInstanceId: view.projectInstanceId,
        revision: view.revision,
        dirty: view.dirty,
      ),
    );
  }
}
