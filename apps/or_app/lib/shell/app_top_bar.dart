import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_typography.dart';
import '../design/or_spacing.dart';
import '../widgets/or_widgets.dart';

class AppTopBar extends StatelessWidget {
  const AppTopBar({
    super.key,
    required this.title,
    required this.compact,
    required this.onOpenCommandPalette,
    required this.onHome,
    this.onExitEditorPreview,
    this.onExport,
    this.onCancelExport,
    this.exportIsActive = false,
    this.statusLabel,
    this.statusIsError = false,
    this.statusIcon,
    this.projectDirty,
    this.onUndo,
    this.onRedo,
    this.onRenameProject,
    this.onSaveProject,
    this.onCloseProject,
  });

  final String title;
  final bool compact;
  final VoidCallback onOpenCommandPalette;
  final VoidCallback onHome;
  final VoidCallback? onExitEditorPreview;
  final VoidCallback? onExport;
  final VoidCallback? onCancelExport;
  final bool exportIsActive;
  final String? statusLabel;
  final bool statusIsError;

  /// Overrides the status glyph for a transient statement that is not a save or
  /// export message (an import batch reports progress through the same slot).
  final IconData? statusIcon;

  /// Non-null while a project is open. The bar then owns the project's save
  /// state, its history controls and the project menu, so the workspace below
  /// does not need a second title row (docs/archive/legacy/EDITOR_WORKSPACE_DESIGN.md).
  final bool? projectDirty;
  final VoidCallback? onUndo;
  final VoidCallback? onRedo;
  final VoidCallback? onRenameProject;
  final VoidCallback? onSaveProject;
  final VoidCallback? onCloseProject;

  @override
  Widget build(BuildContext context) {
    // One shortcut, named the way the current platform spells it. Showing the
    // macOS Command glyph on Linux or Windows told those users to press a key
    // their keyboard does not have.
    final commandShortcut = defaultTargetPlatform == TargetPlatform.macOS
        ? '⌘ K'
        : 'Ctrl+K';
    return Container(
      key: const ValueKey('app-top-bar'),
      height: OrSpacing.topBarHeight,
      padding: EdgeInsets.symmetric(
        horizontal: compact ? OrSpacing.x3 : OrSpacing.x5,
      ),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final showFullBrand = OrBreakpoints.isWide(constraints.maxWidth);
          // Measured 2026-10-25 at 800 px: the project extras plus a labelled
          // Export button overflowed the row by 63 px. Labels go before
          // controls do.
          final labelledExport = constraints.maxWidth >= 1000;
          // Five project icons need ~200 px. A phone-width bar collapses them
          // into one menu; an 800 px window keeps every action literal.
          final literalProjectActions = constraints.maxWidth >= 760;
          final showSaveStateText = constraints.maxWidth >= 1100;
          // A project workspace names the project once, here. The developer
          // preview chip is product scaffolding, not editing information, so it
          // stays out of that mode.
          final showPreviewBadge = showFullBrand && projectDirty == null;
          final dirty = projectDirty;

          return Row(
            children: [
              if (onExitEditorPreview != null) ...[
                Tooltip(
                  message: 'Exit Editor Shell Preview',
                  child: IconButton(
                    key: const ValueKey('exit-editor-preview'),
                    onPressed: onExitEditorPreview,
                    icon: const Icon(Icons.arrow_back_outlined, size: 19),
                    color: OrColors.textSecondary,
                  ),
                ),
                const SizedBox(width: OrSpacing.x2),
              ],
              OrFocusRing(
                radius: 8,
                child: InkWell(
                  key: const ValueKey('or-brand-home'),
                  onTap: onHome,
                  borderRadius: BorderRadius.circular(OrRadii.small),
                  child: Container(
                    width: 30,
                    height: 30,
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      border: Border.all(color: OrColors.borderStrong),
                      borderRadius: BorderRadius.circular(7),
                    ),
                    child: const Text(
                      'OR',
                      style: TextStyle(
                        fontSize: OrType.metadataCompactSize,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                ),
              ),
              if (showFullBrand) ...[
                const SizedBox(width: OrSpacing.x3),
                const Text(
                  'Opencut Reinforced',
                  style: TextStyle(
                    fontSize: OrType.metadataSize,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
              const SizedBox(width: OrSpacing.x4),
              Container(width: 1, height: 24, color: OrColors.border),
              const SizedBox(width: OrSpacing.x4),
              Expanded(
                child: Row(
                  children: [
                    Flexible(
                      child: Text(
                        title,
                        // The workspace names its project here, so this is the
                        // one place a test or harness reads that name.
                        key: projectDirty != null
                            ? const ValueKey('workspace-project-name')
                            : null,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                          color: OrColors.textSecondary,
                          fontSize: OrType.metadataSize,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ),
                    if (dirty != null) ...[
                      const SizedBox(width: OrSpacing.x2),
                      Tooltip(
                        message: dirty
                            ? 'Unsaved changes · ${defaultTargetPlatform == TargetPlatform.macOS ? '⌘S' : 'Ctrl+S'} saves'
                            : 'Every change is on disk',
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Container(
                              key: const ValueKey('project-save-dot'),
                              width: 7,
                              height: 7,
                              decoration: BoxDecoration(
                                shape: BoxShape.circle,
                                color: dirty
                                    ? OrColors.warning
                                    : OrColors.textMuted,
                              ),
                            ),
                            // One save-state statement at a time: when the bar
                            // already carries a transient status (export or a
                            // recovery problem) the dot alone stands for the
                            // dirty state.
                            if (!compact &&
                                showSaveStateText &&
                                statusLabel == null) ...[
                              const SizedBox(width: OrSpacing.x1),
                              Text(
                                dirty ? 'Unsaved' : 'Saved',
                                key: const ValueKey('project-save-state'),
                                style: TextStyle(
                                  color: dirty
                                      ? OrColors.warning
                                      : OrColors.textMuted,
                                  fontSize: OrType.tinySize,
                                ),
                              ),
                            ],
                          ],
                        ),
                      ),
                    ],
                  ],
                ),
              ),
              // All five project actions stay as icons in the one bar. At
              // 800 px the previous design painted Undo and Close off-screen,
              // so the icons are the floor and the labels above them are what
              // collapse first.
              if (dirty != null && !literalProjectActions)
                PopupMenuButton<String>(
                  key: const ValueKey('workspace-actions'),
                  tooltip: 'Project actions',
                  icon: const Icon(Icons.more_horiz, size: 19),
                  iconColor: OrColors.textSecondary,
                  onSelected: (choice) {
                    switch (choice) {
                      case 'rename':
                        onRenameProject?.call();
                      case 'save':
                        onSaveProject?.call();
                      case 'undo':
                        onUndo?.call();
                      case 'redo':
                        onRedo?.call();
                      case 'close':
                        onCloseProject?.call();
                    }
                  },
                  itemBuilder: (context) => [
                    const PopupMenuItem(
                      value: 'rename',
                      child: Text('Rename project…'),
                    ),
                    PopupMenuItem(
                      value: 'save',
                      enabled: dirty,
                      child: const Text('Save now'),
                    ),
                    PopupMenuItem(
                      value: 'undo',
                      enabled: onUndo != null,
                      child: const Text('Undo'),
                    ),
                    PopupMenuItem(
                      value: 'redo',
                      enabled: onRedo != null,
                      child: const Text('Redo'),
                    ),
                    const PopupMenuDivider(),
                    const PopupMenuItem(
                      value: 'close',
                      child: Text('Close project'),
                    ),
                  ],
                ),
              if (dirty != null && literalProjectActions) ...[
                _TopBarIcon(
                  buttonKey: 'workspace-rename',
                  tooltip: 'Rename project',
                  icon: Icons.drive_file_rename_outline,
                  onPressed: onRenameProject,
                ),
                _TopBarIcon(
                  buttonKey: 'workspace-save',
                  tooltip: dirty ? 'Save now (⌘S)' : 'Saved',
                  icon: Icons.save_outlined,
                  onPressed: dirty ? onSaveProject : null,
                ),
                _TopBarIcon(
                  buttonKey: 'workspace-undo',
                  tooltip: 'Undo (⌘Z)',
                  icon: Icons.undo_outlined,
                  onPressed: onUndo,
                ),
                _TopBarIcon(
                  buttonKey: 'workspace-redo',
                  tooltip: 'Redo (⇧⌘Z)',
                  icon: Icons.redo_outlined,
                  onPressed: onRedo,
                ),
                _TopBarIcon(
                  buttonKey: 'workspace-close',
                  tooltip: 'Close project',
                  icon: Icons.close,
                  onPressed: onCloseProject,
                ),
                const SizedBox(width: OrSpacing.x2),
              ],
              // One transient statement at a time: an export or recovery
              // message is more urgent than the dirty dot's label, and the
              // text is dropped before the icon so the bar never overflows.
              if (statusLabel != null) ...[
                Tooltip(
                  key: const ValueKey('export-status'),
                  message: statusLabel!,
                  child: Icon(
                    statusIsError
                        ? Icons.warning_amber_outlined
                        : (statusIcon ?? Icons.save_outlined),
                    size: 15,
                    color: statusIsError
                        ? OrColors.warning
                        : OrColors.textMuted,
                  ),
                ),
                if (!compact && showSaveStateText) ...[
                  const SizedBox(width: OrSpacing.x1),
                  Text(
                    statusLabel!,
                    style: TextStyle(
                      color: statusIsError
                          ? OrColors.warning
                          : OrColors.textMuted,
                      fontSize: OrType.tinySize,
                    ),
                  ),
                  const SizedBox(width: OrSpacing.x3),
                ] else
                  const SizedBox(width: OrSpacing.x1),
              ],
              if (onExport != null) ...[
                if (compact || !labelledExport)
                  Tooltip(
                    message: 'Export project',
                    child: IconButton(
                      key: const ValueKey('export-project'),
                      onPressed: onExport,
                      icon: const Icon(Icons.ios_share_outlined, size: 18),
                      color: OrColors.textSecondary,
                    ),
                  )
                else
                  OutlinedButton.icon(
                    key: const ValueKey('export-project'),
                    onPressed: onExport,
                    icon: const Icon(Icons.ios_share_outlined, size: 17),
                    label: const Text('Export'),
                    style: OutlinedButton.styleFrom(
                      minimumSize: const Size(0, OrControlSizes.button),
                      padding: const EdgeInsets.symmetric(
                        horizontal: OrSpacing.x3,
                      ),
                      textStyle: const TextStyle(
                        fontSize: OrType.metadataCompactSize,
                      ),
                    ),
                  ),
                const SizedBox(width: OrSpacing.x1),
              ],
              if (exportIsActive && onCancelExport != null) ...[
                Tooltip(
                  message: 'Cancel export',
                  child: IconButton(
                    key: const ValueKey('cancel-export'),
                    onPressed: onCancelExport,
                    icon: const Icon(Icons.close_outlined, size: 17),
                    color: OrColors.textSecondary,
                  ),
                ),
                const SizedBox(width: OrSpacing.x1),
              ],
              if (showPreviewBadge) ...[
                const _PreviewBadge(),
                const SizedBox(width: OrSpacing.x3),
              ],
              if (showFullBrand)
                OutlinedButton.icon(
                  key: const ValueKey('open-command-palette'),
                  onPressed: onOpenCommandPalette,
                  icon: const Icon(Icons.search_outlined, size: 17),
                  label: Row(
                    children: [
                      const Text('Search commands'),
                      const SizedBox(width: OrSpacing.x2),
                      Text(
                        commandShortcut,
                        style: const TextStyle(
                          color: OrColors.textMuted,
                          fontSize: OrType.tinySize,
                        ),
                      ),
                    ],
                  ),
                  style: OutlinedButton.styleFrom(
                    minimumSize: const Size(0, OrControlSizes.button),
                    padding: const EdgeInsets.symmetric(
                      horizontal: OrSpacing.x3,
                    ),
                    textStyle: const TextStyle(
                      fontSize: OrType.metadataCompactSize,
                    ),
                  ),
                )
              else
                Tooltip(
                  message: 'Search commands ($commandShortcut)',
                  child: IconButton(
                    key: const ValueKey('open-command-palette'),
                    onPressed: onOpenCommandPalette,
                    icon: const Icon(Icons.search_outlined),
                    color: OrColors.textSecondary,
                  ),
                ),
            ],
          );
        },
      ),
    );
  }
}

class _TopBarIcon extends StatelessWidget {
  const _TopBarIcon({
    required this.buttonKey,
    required this.tooltip,
    required this.icon,
    required this.onPressed,
  });

  final String buttonKey;
  final String tooltip;
  final IconData icon;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return IconButton(
      key: ValueKey(buttonKey),
      tooltip: tooltip,
      onPressed: onPressed,
      icon: Icon(icon, size: 18),
      color: OrColors.textSecondary,
      visualDensity: VisualDensity.compact,
    );
  }
}

class _PreviewBadge extends StatelessWidget {
  const _PreviewBadge();
  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x2,
        vertical: 5,
      ),
      decoration: BoxDecoration(
        border: Border.all(color: OrColors.border),
        borderRadius: BorderRadius.circular(OrRadii.small),
      ),
      child: const Text(
        'Developer Preview',
        style: TextStyle(
          color: OrColors.textSecondary,
          fontSize: OrType.tinySize,
        ),
      ),
    );
  }
}
