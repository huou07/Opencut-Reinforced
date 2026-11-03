import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// The application menu. Every item routes to the same callback the on-screen
/// control uses, so the menu can never drift from the visible surface.
///
/// macOS owns a system menu bar and Flutter exposes it as `PlatformMenuBar`.
/// Windows and Linux have no such channel, so the same menu description is
/// rendered as an in-window bar with the Material `MenuBar` widget instead of
/// leaving those platforms without menus. One description, two renderers: an
/// item added below appears on all three platforms.
class PlatformMenus extends StatelessWidget {
  const PlatformMenus({
    super.key,
    required this.isEditor,
    required this.textInputFocused,
    required this.canExport,
    required this.canEditProject,
    required this.onNewProject,
    required this.onOpenProject,
    required this.onSaveProject,
    required this.onExport,
    required this.onUndo,
    required this.onRedo,
    required this.onCloseProject,
    required this.onHome,
    required this.onProjects,
    required this.onSettings,
    required this.onCommandPalette,
    required this.onSplit,
    required this.onDuplicate,
    required this.onCopy,
    required this.onPaste,
    required this.onDelete,
    required this.onRippleDelete,
    required this.onAddMarker,
    required this.onToggleSnapping,
    required this.onReverseClip,
    required this.onFreezeClip,
    required this.canImport,
    required this.onImportMedia,
    required this.onImportFolder,
    required this.child,
  });

  final bool isEditor;
  final bool textInputFocused;
  final bool canExport;
  final bool canEditProject;
  final VoidCallback onNewProject;
  final VoidCallback onOpenProject;
  final VoidCallback onSaveProject;
  final VoidCallback onExport;
  final VoidCallback onUndo;
  final VoidCallback onRedo;
  final VoidCallback onCloseProject;
  final VoidCallback onHome;
  final VoidCallback onProjects;
  final VoidCallback onSettings;
  final VoidCallback onCommandPalette;
  final VoidCallback onSplit;
  final VoidCallback onDuplicate;
  final VoidCallback onCopy;
  final VoidCallback onPaste;
  final VoidCallback onDelete;
  final VoidCallback onRippleDelete;
  final VoidCallback onAddMarker;
  final VoidCallback onToggleSnapping;
  final VoidCallback onReverseClip;

  /// Holds the selected clip's first frame, or releases it.
  final VoidCallback onFreezeClip;
  final bool canImport;
  final VoidCallback? onImportMedia;
  final VoidCallback? onImportFolder;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (Platform.isMacOS && !debugUseInWindowMenuBar) {
      return PlatformMenuBar(menus: _macOSMenus(), child: child);
    }
    return Column(
      children: [
        _inWindowMenuBar(context),
        Expanded(child: child),
      ],
    );
  }

  /// Forces the in-window bar while a test runs on a macOS host.
  @visibleForTesting
  static bool debugUseInWindowMenuBar = false;

  /// The one description of the menu, rendered by whichever bar the host has.
  List<_OrMenu> get _menus => [
    _OrMenu('File', [
      _OrMenuAction(
        'New Project…',
        shortcut: const SingleActivator(LogicalKeyboardKey.keyN, meta: true),
        onSelected: onNewProject,
      ),
      _OrMenuAction(
        'Open Project…',
        shortcut: const SingleActivator(LogicalKeyboardKey.keyO, meta: true),
        onSelected: onOpenProject,
      ),
      const _OrMenuSeparator(),
      _OrMenuAction(
        'Save',
        shortcut: const SingleActivator(LogicalKeyboardKey.keyS, meta: true),
        onSelected: canEditProject ? onSaveProject : null,
      ),
      _OrMenuAction(
        'Import Media…',
        shortcut: const SingleActivator(LogicalKeyboardKey.keyI, meta: true),
        onSelected: onImportMedia,
      ),
      _OrMenuAction(
        'Import Folder…',
        shortcut: const SingleActivator(
          LogicalKeyboardKey.keyI,
          meta: true,
          shift: true,
        ),
        onSelected: onImportFolder,
      ),
      _OrMenuAction(
        'Export Video…',
        shortcut: const SingleActivator(LogicalKeyboardKey.keyE, meta: true),
        onSelected: canExport ? onExport : null,
      ),
      const _OrMenuSeparator(),
      _OrMenuAction(
        'Close Project',
        onSelected: canEditProject ? onCloseProject : null,
      ),
    ]),
    _OrMenu('Edit', [
      // Editing verbs carry no key equivalent. A native menu equivalent is
      // matched before the focused text field, so a shortcut here would steal
      // text editing; the Flutter shortcut map owns these keys and disables
      // them while a field is focused.
      _OrMenuAction(
        'Undo',
        onSelected: !textInputFocused && canEditProject ? onUndo : null,
      ),
      _OrMenuAction(
        'Redo',
        onSelected: !textInputFocused && canEditProject ? onRedo : null,
      ),
      const _OrMenuSeparator(),
      _OrMenuAction(
        'Split at Playhead',
        onSelected: !textInputFocused && isEditor ? onSplit : null,
      ),
      _OrMenuAction(
        'Duplicate Clip',
        onSelected: !textInputFocused && isEditor ? onDuplicate : null,
      ),
      _OrMenuAction(
        'Copy Clip',
        onSelected: !textInputFocused && isEditor ? onCopy : null,
      ),
      _OrMenuAction(
        'Paste Clip',
        onSelected: !textInputFocused && isEditor ? onPaste : null,
      ),
      _OrMenuAction(
        'Delete Clip',
        onSelected: !textInputFocused && isEditor ? onDelete : null,
      ),
      _OrMenuAction(
        'Ripple Delete Clip',
        onSelected: !textInputFocused && isEditor ? onRippleDelete : null,
      ),
      _OrMenuAction(
        'Add Marker',
        onSelected: !textInputFocused && isEditor ? onAddMarker : null,
      ),
      _OrMenuAction(
        'Toggle Snapping',
        onSelected: isEditor ? onToggleSnapping : null,
      ),
      _OrMenuAction(
        'Reverse Selected Clip',
        onSelected: isEditor ? onReverseClip : null,
      ),
      _OrMenuAction(
        'Freeze Selected Clip',
        onSelected: isEditor ? onFreezeClip : null,
      ),
    ]),
    _OrMenu('View', [
      _OrMenuAction('Home', onSelected: onHome),
      _OrMenuAction('Projects', onSelected: onProjects),
      _OrMenuAction(
        'Command Palette…',
        shortcut: const SingleActivator(LogicalKeyboardKey.keyK, meta: true),
        onSelected: onCommandPalette,
      ),
    ]),
    _OrMenu('OR', [
      _OrMenuAction(
        'Settings…',
        shortcut: const SingleActivator(LogicalKeyboardKey.comma, meta: true),
        onSelected: onSettings,
      ),
    ]),
  ];

  List<PlatformMenu> _macOSMenus() {
    return [
      // macOS owns the first slot and titles it with the application name.
      // `PlatformProvidedMenuItem` refuses to exist on a target without that
      // built-in item, and `flutter test` reports an android target even on a
      // macOS host, so the app menu is added only for a real macOS target.
      if (defaultTargetPlatform == TargetPlatform.macOS)
        PlatformMenu(
          label: 'Opencut Reinforced',
          menus: [
            PlatformProvidedMenuItem(type: PlatformProvidedMenuItemType.about),
            PlatformMenuItemGroup(
              members: [
                PlatformProvidedMenuItem(
                  type: PlatformProvidedMenuItemType.hide,
                ),
                PlatformProvidedMenuItem(
                  type: PlatformProvidedMenuItemType.hideOtherApplications,
                ),
                PlatformProvidedMenuItem(
                  type: PlatformProvidedMenuItemType.showAllApplications,
                ),
              ],
            ),
            PlatformMenuItemGroup(
              members: [
                PlatformProvidedMenuItem(
                  type: PlatformProvidedMenuItemType.quit,
                ),
              ],
            ),
          ],
        ),
      for (final menu in _menus)
        PlatformMenu(label: menu.label, menus: _platformMenus(menu.entries)),
    ];
  }

  /// A native group draws its own separator, so each run becomes one group.
  List<PlatformMenuItem> _platformMenus(List<_OrMenuEntry> entries) {
    final result = <PlatformMenuItem>[];
    var run = <PlatformMenuItem>[];
    void flush() {
      if (run.isEmpty) return;
      result.add(
        PlatformMenuItemGroup(members: List<PlatformMenuItem>.of(run)),
      );
      run = [];
    }

    for (final entry in entries) {
      switch (entry) {
        case _OrMenuSeparator():
          flush();
        case _OrMenuAction():
          run.add(
            PlatformMenuItem(
              label: entry.label,
              shortcut: entry.shortcut,
              onSelected: entry.onSelected,
            ),
          );
      }
    }
    flush();
    return result;
  }

  Widget _inWindowMenuBar(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: theme.colorScheme.surface,
      child: SizedBox(
        height: 40,
        child: Align(
          alignment: Alignment.centerLeft,
          child: MenuBar(
            style: const MenuStyle(
              elevation: WidgetStatePropertyAll(0),
              padding: WidgetStatePropertyAll(
                EdgeInsets.symmetric(horizontal: 6),
              ),
            ),
            children: [
              for (final menu in _menus)
                SubmenuButton(
                  menuChildren: _materialMenus(menu.entries),
                  child: Text(menu.label),
                ),
            ],
          ),
        ),
      ),
    );
  }

  List<Widget> _materialMenus(List<_OrMenuEntry> entries) {
    return [
      for (final entry in entries)
        switch (entry) {
          _OrMenuSeparator() => const Divider(height: 1),
          _OrMenuAction() => MenuItemButton(
            shortcut: entry.shortcut,
            onPressed: entry.onSelected,
            child: Text(entry.label),
          ),
        },
    ];
  }
}

/// One menu, independent of the toolkit that draws it.
class _OrMenu {
  const _OrMenu(this.label, this.entries);

  final String label;
  final List<_OrMenuEntry> entries;
}

sealed class _OrMenuEntry {
  const _OrMenuEntry();
}

class _OrMenuAction extends _OrMenuEntry {
  const _OrMenuAction(this.label, {this.shortcut, this.onSelected});

  final String label;
  final SingleActivator? shortcut;
  final VoidCallback? onSelected;
}

class _OrMenuSeparator extends _OrMenuEntry {
  const _OrMenuSeparator();
}
