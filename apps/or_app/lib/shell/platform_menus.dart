import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// The macOS menu bar. Every item routes to the same callback the on-screen
/// control uses, so the menu can never drift from the visible surface.
/// Non-Apple platforms get the child unchanged: a menu bar that only exists on
/// one platform should not change the widget tree anywhere else.
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
    if (!Platform.isMacOS) return child;
    return PlatformMenuBar(
      menus: [
        // macOS owns the first menu slot and titles it with the application
        // name; without an app menu of our own the File menu is swallowed into
        // that slot and never appears (observed: bar read "or_app, Edit, View,
        // OR" while File's items lived under "or_app").
        // `PlatformProvidedMenuItem` refuses to exist on a target without that
        // built-in item, and `flutter test` reports an android target even on a
        // macOS host, so the app menu is added only for a real macOS target.
        if (defaultTargetPlatform == TargetPlatform.macOS)
          PlatformMenu(
            label: 'Opencut Reinforced',
            menus: [
              PlatformProvidedMenuItem(
                type: PlatformProvidedMenuItemType.about,
              ),
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
        PlatformMenu(
          label: 'File',
          menus: [
            PlatformMenuItem(
              label: 'New Project…',
              shortcut: const SingleActivator(
                LogicalKeyboardKey.keyN,
                meta: true,
              ),
              onSelected: onNewProject,
            ),
            PlatformMenuItem(
              label: 'Open Project…',
              shortcut: const SingleActivator(
                LogicalKeyboardKey.keyO,
                meta: true,
              ),
              onSelected: onOpenProject,
            ),
            PlatformMenuItemGroup(
              members: [
                PlatformMenuItem(
                  label: 'Save',
                  shortcut: const SingleActivator(
                    LogicalKeyboardKey.keyS,
                    meta: true,
                  ),
                  onSelected: canEditProject ? onSaveProject : null,
                ),
                PlatformMenuItem(
                  label: 'Import Media…',
                  shortcut: const SingleActivator(
                    LogicalKeyboardKey.keyI,
                    meta: true,
                  ),
                  onSelected: onImportMedia,
                ),
                PlatformMenuItem(
                  label: 'Import Folder…',
                  shortcut: const SingleActivator(
                    LogicalKeyboardKey.keyI,
                    meta: true,
                    shift: true,
                  ),
                  onSelected: onImportFolder,
                ),
                PlatformMenuItem(
                  label: 'Export Video…',
                  shortcut: const SingleActivator(
                    LogicalKeyboardKey.keyE,
                    meta: true,
                  ),
                  onSelected: canExport ? onExport : null,
                ),
              ],
            ),
            PlatformMenuItemGroup(
              members: [
                PlatformMenuItem(
                  label: 'Close Project',
                  onSelected: canEditProject ? onCloseProject : null,
                ),
              ],
            ),
          ],
        ),
        PlatformMenu(
          label: 'Edit',
          menus: [
            // Editing verbs carry no native key equivalent. macOS routes a
            // menu equivalent before the focused text field, so ⌘Z/⌘C/⌘V in a
            // menu would steal text editing. The Flutter shortcut map owns
            // these keys and disables them while a field is focused.
            PlatformMenuItem(
              label: 'Undo',
              onSelected: !textInputFocused && canEditProject ? onUndo : null,
            ),
            PlatformMenuItem(
              label: 'Redo',
              onSelected: !textInputFocused && canEditProject ? onRedo : null,
            ),
            PlatformMenuItemGroup(
              members: [
                PlatformMenuItem(
                  label: 'Split at Playhead',
                  onSelected: !textInputFocused && isEditor ? onSplit : null,
                ),
                PlatformMenuItem(
                  label: 'Duplicate Clip',
                  onSelected: !textInputFocused && isEditor
                      ? onDuplicate
                      : null,
                ),
                PlatformMenuItem(
                  label: 'Copy Clip',
                  onSelected: !textInputFocused && isEditor ? onCopy : null,
                ),
                PlatformMenuItem(
                  label: 'Paste Clip',
                  onSelected: !textInputFocused && isEditor ? onPaste : null,
                ),
                // No key equivalent: macOS treats a modifier-less menu
                // equivalent as a typing key (Delete is Backspace, M is "m"),
                // so the menu would steal text editing from every field. The
                // Flutter shortcut map owns these verbs and is disabled while a
                // text field is focused.
                PlatformMenuItem(
                  label: 'Delete Clip',
                  onSelected: !textInputFocused && isEditor ? onDelete : null,
                ),
                PlatformMenuItem(
                  label: 'Ripple Delete Clip',
                  onSelected: !textInputFocused && isEditor
                      ? onRippleDelete
                      : null,
                ),
                PlatformMenuItem(
                  label: 'Add Marker',
                  onSelected: !textInputFocused && isEditor
                      ? onAddMarker
                      : null,
                ),
                PlatformMenuItem(
                  label: 'Toggle Snapping',
                  onSelected: isEditor ? onToggleSnapping : null,
                ),
                PlatformMenuItem(
                  label: 'Reverse Selected Clip',
                  onSelected: isEditor ? onReverseClip : null,
                ),
                PlatformMenuItem(
                  label: 'Freeze Selected Clip',
                  onSelected: isEditor ? onFreezeClip : null,
                ),
              ],
            ),
          ],
        ),
        PlatformMenu(
          label: 'View',
          menus: [
            PlatformMenuItem(label: 'Home', onSelected: onHome),
            PlatformMenuItem(label: 'Projects', onSelected: onProjects),
            PlatformMenuItem(
              label: 'Command Palette…',
              shortcut: const SingleActivator(
                LogicalKeyboardKey.keyK,
                meta: true,
              ),
              onSelected: onCommandPalette,
            ),
          ],
        ),
        PlatformMenu(
          label: 'OR',
          menus: [
            PlatformMenuItem(
              label: 'Settings…',
              shortcut: const SingleActivator(
                LogicalKeyboardKey.comma,
                meta: true,
              ),
              onSelected: onSettings,
            ),
          ],
        ),
      ],
      child: child,
    );
  }
}
