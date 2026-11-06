import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/design/or_theme.dart';
import 'package:or_app/library/project_templates.dart';
import 'package:or_app/library/sound_library.dart';
import 'package:or_app/library/text_presets.dart';
import 'package:or_app/library/usage_guide.dart';
import 'package:or_app/screens/library_screen.dart';
import 'package:or_app/support/host_platform.dart';

HostPlatform _host({required bool macOS}) => HostPlatform(
  isWindows: !macOS,
  isMacOS: macOS,
  isLinux: false,
  environment: const {},
  executablePath: '/or',
);

Future<void> _pump(
  WidgetTester tester, {
  bool projectOpen = false,
  bool canPickProjects = true,
  LibraryTab initialTab = LibraryTab.templates,
  HostPlatform? host,
  Key? key,
}) async {
  tester.view.physicalSize = const Size(1600, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      theme: OrTheme.dark,
      home: Scaffold(
        body: LibraryScreen(
          key: key,
          canPickProjects: canPickProjects,
          projectOpen: projectOpen,
          onUseTemplate: (_) {},
          onAddTextPreset: (_) {},
          onAddSoundEffect: (_) {},
          onOpenEditor: () {},
          initialTab: initialTab,
          hostPlatform: host ?? _host(macOS: true),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('the guide tab says where every shortcut lives', (tester) async {
    await _pump(tester, initialTab: LibraryTab.guide);
    final hint = find.byKey(const ValueKey('library-shortcut-hint'));
    await tester.scrollUntilVisible(
      hint,
      200,
      scrollable: find.byType(Scrollable).last,
    );
    expect(hint, findsOneWidget);
    expect(find.textContaining('Press ? anywhere'), findsOneWidget);
  });

  testWidgets('the library opens on templates and lists every template', (
    tester,
  ) async {
    await _pump(tester);
    expect(find.text('Library'), findsWidgets);
    expect(find.byKey(const ValueKey('library-tabs')), findsOneWidget);
    for (final tab in LibraryTab.values) {
      expect(find.byKey(ValueKey('library-tab-${tab.name}')), findsOneWidget);
    }
    for (final template in projectTemplates) {
      // The card may be below the fold; scrolling to one key proves the list
      // really contains it rather than only its first screen.
      await tester.scrollUntilVisible(
        find.byKey(ValueKey('library-template-use-${template.id}')),
        200,
        scrollable: find.byType(Scrollable).first,
      );
      expect(
        find.byKey(ValueKey('library-template-use-${template.id}')),
        findsOneWidget,
      );
    }
  });

  testWidgets('a template with no project still shows its action', (
    tester,
  ) async {
    await _pump(tester, canPickProjects: false);
    expect(
      find.byKey(ValueKey('library-template-use-${projectTemplates.first.id}')),
      findsOneWidget,
    );
  });

  testWidgets('the text tab shows the exact formatting of every look', (
    tester,
  ) async {
    await _pump(tester, initialTab: LibraryTab.text);
    for (final preset in textPresets) {
      await tester.scrollUntilVisible(
        find.byKey(ValueKey('library-text-spec-${preset.id}')),
        200,
        scrollable: find.byType(Scrollable).first,
      );
      expect(
        find.byKey(ValueKey('library-text-add-${preset.id}')),
        findsOneWidget,
      );
    }
    // No project: the reason is attached to the control instead of the control
    // being silently inert.
    expect(find.byTooltip('Open a project to add a text clip.'), findsWidgets);
  });

  testWidgets('the text tab adds a clip when a project is open', (
    tester,
  ) async {
    var added = 0;
    tester.view.physicalSize = const Size(1600, 1400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        theme: OrTheme.dark,
        home: Scaffold(
          body: LibraryScreen(
            canPickProjects: true,
            projectOpen: true,
            onUseTemplate: (_) {},
            onAddTextPreset: (_) => added++,
            onAddSoundEffect: (_) {},
            onOpenEditor: () {},
            initialTab: LibraryTab.text,
            hostPlatform: _host(macOS: true),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final first = textPresets.first;
    final button = find.byKey(ValueKey('library-text-add-${first.id}'));
    await tester.ensureVisible(button);
    await tester.pumpAndSettle();
    await tester.tap(button);
    await tester.pumpAndSettle();
    expect(added, 1);
    expect(find.byTooltip('Open a project to add a text clip.'), findsNothing);
  });

  testWidgets('the sound tab offers every effect and the music entry point', (
    tester,
  ) async {
    await _pump(tester, initialTab: LibraryTab.sound);
    expect(
      find.byKey(const ValueKey('library-sound-open-music')),
      findsOneWidget,
    );
    for (final effect in soundEffects) {
      await tester.scrollUntilVisible(
        find.byKey(ValueKey('library-sound-add-${effect.id}')),
        200,
        scrollable: find.byType(Scrollable).first,
      );
      expect(find.byKey(ValueKey('library-sound-add-${effect.id}')), findsOne);
    }
  });

  testWidgets('the guide renders the shortcut modifier of the host', (
    tester,
  ) async {
    await _pump(tester, initialTab: LibraryTab.guide);
    expect(find.byKey(const ValueKey('library-guide-start')), findsOneWidget);
    expect(find.text('Cmd+K'), findsWidgets);

    await _pump(
      tester,
      initialTab: LibraryTab.guide,
      host: _host(macOS: false),
      key: const ValueKey('windows-guide'),
    );
    expect(find.text('Cmd+K'), findsNothing);
    expect(find.text('Ctrl+K'), findsWidgets);
  });

  testWidgets('every guide section is reachable', (tester) async {
    await _pump(tester, initialTab: LibraryTab.guide);
    for (final section in usageGuide) {
      await tester.scrollUntilVisible(
        find.byKey(ValueKey('library-guide-${section.id}')),
        300,
        scrollable: find.byType(Scrollable).first,
      );
      expect(
        find.byKey(ValueKey('library-guide-${section.id}')),
        findsOneWidget,
      );
    }
  });

  testWidgets('the guide ends with the licence of every bundled asset', (
    tester,
  ) async {
    await _pump(tester, initialTab: LibraryTab.guide);
    await tester.scrollUntilVisible(
      find.byKey(const ValueKey('library-credits')),
      400,
      scrollable: find.byType(Scrollable).first,
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('library-credits')), findsOneWidget);
    expect(
      tester
          .widget<Text>(
            find.byKey(const ValueKey('library-credit-licence-inter')),
          )
          .data,
      'SIL Open Font License 1.1',
    );
    expect(
      tester
          .widget<Text>(
            find.byKey(const ValueKey('library-credit-licence-sound-effects')),
          )
          .data,
      'Generated on this device',
    );
  });

  testWidgets('the text and sound tabs say where their licences are', (
    tester,
  ) async {
    await _pump(tester, initialTab: LibraryTab.text);
    expect(
      tester
          .widget<Text>(find.byKey(const ValueKey('library-text-font-licence')))
          .data,
      contains('SIL Open Font License 1.1'),
    );

    // A new key so the screen state is created fresh; otherwise the previous
    // tab would survive and the sound tab would never build.
    await _pump(
      tester,
      initialTab: LibraryTab.sound,
      key: const ValueKey('library-sound-tab'),
    );
    expect(
      tester
          .widget<Text>(find.byKey(const ValueKey('library-sound-licence')))
          .data,
      contains('no third-party audio'),
    );
  });
}
