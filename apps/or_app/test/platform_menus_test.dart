import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/shell/platform_menus.dart';

Widget _build(VoidCallback onNewProject) {
  return MaterialApp(
    home: Scaffold(
      body: PlatformMenus(
        isEditor: true,
        textInputFocused: false,
        canExport: true,
        canEditProject: true,
        onNewProject: onNewProject,
        onOpenProject: () {},
        onSaveProject: () {},
        onExport: () {},
        onUndo: () {},
        onRedo: () {},
        onCloseProject: () {},
        onHome: () {},
        onProjects: () {},
        onSettings: () {},
        onCommandPalette: () {},
        onSplit: () {},
        onDuplicate: () {},
        onCopy: () {},
        onPaste: () {},
        onDelete: () {},
        onRippleDelete: () {},
        onAddMarker: () {},
        onToggleSnapping: () {},
        onReverseClip: () {},
        onFreezeClip: () {},
        canImport: true,
        onImportMedia: () {},
        onImportFolder: () {},
        child: const SizedBox.expand(),
      ),
    ),
  );
}

void main() {
  testWidgets('an in-window bar carries the same menus and commands', (
    tester,
  ) async {
    var created = 0;
    PlatformMenus.debugUseInWindowMenuBar = true;
    addTearDown(() => PlatformMenus.debugUseInWindowMenuBar = false);

    await tester.pumpWidget(_build(() => created++));

    expect(find.text('File'), findsOneWidget);
    expect(find.text('Edit'), findsOneWidget);
    expect(find.text('View'), findsOneWidget);

    await tester.tap(find.text('File'));
    await tester.pumpAndSettle();
    expect(find.text('New Project…'), findsOneWidget);

    await tester.tap(find.text('New Project…'));
    await tester.pumpAndSettle();
    expect(created, 1);
  });

  testWidgets('a disabled verb stays visible and inert', (tester) async {
    PlatformMenus.debugUseInWindowMenuBar = true;
    addTearDown(() => PlatformMenus.debugUseInWindowMenuBar = false);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: PlatformMenus(
            isEditor: false,
            textInputFocused: true,
            canExport: false,
            canEditProject: false,
            onNewProject: () {},
            onOpenProject: () {},
            onSaveProject: () {},
            onExport: () {},
            onUndo: () {},
            onRedo: () {},
            onCloseProject: () {},
            onHome: () {},
            onProjects: () {},
            onSettings: () {},
            onCommandPalette: () {},
            onSplit: () {},
            onDuplicate: () {},
            onCopy: () {},
            onPaste: () {},
            onDelete: () {},
            onRippleDelete: () {},
            onAddMarker: () {},
            onToggleSnapping: () {},
            onReverseClip: () {},
            onFreezeClip: () {},
            canImport: false,
            onImportMedia: null,
            onImportFolder: null,
            child: const SizedBox.expand(),
          ),
        ),
      ),
    );

    await tester.tap(find.text('Edit'));
    await tester.pumpAndSettle();
    expect(find.text('Undo'), findsOneWidget);
    final undo = tester.widget<MenuItemButton>(
      find.ancestor(
        of: find.text('Undo'),
        matching: find.byType(MenuItemButton),
      ),
    );
    expect(undo.onPressed, isNull);
  });
}
