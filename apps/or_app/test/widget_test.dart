import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/core_gateway.dart';
import 'package:or_app/design/or_colors.dart';
import 'package:or_app/design/or_spacing.dart';
import 'package:or_app/main.dart';
import 'package:or_app/shell/app_navigation.dart';

void main() {
  testWidgets('wide shell shows OR navigation and the top bar', (tester) async {
    _setViewport(tester, const Size(1440, 900));
    await tester.pumpWidget(OrApp(gateway: _FakeCoreGateway()));
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('app-top-bar')), findsOneWidget);
    expect(find.byKey(const ValueKey('compact-navigation')), findsNothing);
    expect(find.byKey(const ValueKey('nav-home')), findsOneWidget);
    expect(find.text('Opencut Reinforced'), findsOneWidget);
    expect(find.text('Home'), findsWidgets);

    await tester.tap(find.byKey(const ValueKey('nav-projects')));
    await tester.pumpAndSettle();
    expect(
      find.text('No projects opened from the production UI yet'),
      findsOneWidget,
    );
  });

  testWidgets(
    'compact shell reaches all five primary destinations without overflow',
    (tester) async {
      _setViewport(tester, const Size(390, 844));
      await tester.pumpWidget(OrApp(gateway: _FakeCoreGateway()));
      await tester.pumpAndSettle();

      expect(find.byKey(const ValueKey('compact-navigation')), findsOneWidget);
      expect(find.byKey(const ValueKey('nav-home')), findsOneWidget);

      final destinations = [
        (
          AppDestination.projects,
          'No projects opened from the production UI yet',
        ),
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
    },
  );

  testWidgets('medium desktop page header fits beside the navigation rail', (
    tester,
  ) async {
    _setViewport(tester, const Size(760, 900));
    await tester.pumpWidget(OrApp(gateway: _FakeCoreGateway()));
    await tester.pumpAndSettle();
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

  testWidgets(
    'Home has honest empty state and unavailable actions explain why',
    (tester) async {
      _setViewport(tester, const Size(1440, 900));
      await tester.pumpWidget(OrApp(gateway: _FakeCoreGateway()));
      await tester.pumpAndSettle();

      expect(find.text('Home'), findsWidgets);
      expect(find.text('New Project'), findsOneWidget);
      expect(find.text('Open Project'), findsOneWidget);
      expect(find.text('Recent Projects'), findsOneWidget);
      expect(find.text('No recent projects yet'), findsOneWidget);
      expect(find.text('Untitled Project'), findsNothing);

      await tester.tap(find.byKey(const ValueKey('home-view-projects')));
      await tester.pumpAndSettle();
      expect(
        find.text('No projects opened from the production UI yet'),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const ValueKey('nav-home')));
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const ValueKey('home-new-project')));
      await tester.pumpAndSettle();
      expect(
        find.text('Project creation is unavailable in this Developer Preview.'),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const ValueKey('home-open-project')));
      await tester.pumpAndSettle();
      expect(
        find.text(
          'Opening projects from the production UI is not available in this Developer Preview.',
        ),
        findsOneWidget,
      );
    },
  );

  testWidgets('Settings diagnostics display only values from CoreGateway', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    await tester.pumpWidget(OrApp(gateway: _FakeCoreGateway()));
    await tester.pumpAndSettle();
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

  testWidgets('Ctrl+K opens command palette and navigates to editor preview', (
    tester,
  ) async {
    _setViewport(tester, const Size(1280, 800));
    await tester.pumpWidget(OrApp(gateway: _FakeCoreGateway()));
    await tester.pumpAndSettle();

    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.keyK);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.keyK);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('command-palette-query')), findsOneWidget);
    await tester.enterText(
      find.byKey(const ValueKey('command-palette-query')),
      'Editor Shell Preview',
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('command-editorPreview')));
    await tester.pumpAndSettle();
    expect(find.text('Timeline engine not implemented'), findsOneWidget);
  });

  testWidgets('desktop Editor Shell Preview shows honest structural regions', (
    tester,
  ) async {
    _setViewport(tester, const Size(1440, 900));
    await tester.pumpWidget(OrApp(gateway: _FakeCoreGateway()));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('home-editor-preview')));
    await tester.pumpAndSettle();

    expect(find.text('Viewer'), findsOneWidget);
    expect(find.text('Timeline'), findsOneWidget);
    expect(find.text('Inspector'), findsOneWidget);
    expect(find.text('No media loaded'), findsOneWidget);
    expect(find.text('No selection'), findsOneWidget);
    expect(find.text('Timeline engine not implemented'), findsOneWidget);
    expect(find.textContaining('.mp4'), findsNothing);
    expect(find.byKey(const ValueKey('nav-editorPreview')), findsNothing);
  });

  testWidgets(
    'compact Editor Shell Preview uses a tool sheet, not desktop panels',
    (tester) async {
      _setViewport(tester, const Size(390, 844));
      await tester.pumpWidget(OrApp(gateway: _FakeCoreGateway()));
      await tester.pumpAndSettle();
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
      expect(
        find.text('Unavailable in this Developer Preview'),
        findsOneWidget,
      );
      await tester.tap(find.byTooltip('Close tool panel'));
      await tester.pumpAndSettle();
      expect(find.text('Timeline engine not implemented'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
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
