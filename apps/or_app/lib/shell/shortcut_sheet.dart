import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';

/// One row of the shortcut sheet.
class OrShortcut {
  const OrShortcut(this.keys, this.action);

  /// How the key is written on screen, in the order a user presses it.
  final String keys;
  final String action;
}

/// One named group of rows.
class OrShortcutGroup {
  const OrShortcutGroup(this.title, this.rows);

  final String title;
  final List<OrShortcut> rows;
}

/// Everything the app binds, written the way a beginner reads a keyboard.
///
/// The source of truth for what a key does is the two maps in
/// `lib/shell/app_shell.dart` (`_alwaysShortcuts`, `_editingShortcuts`); this
/// list is the readable mirror of them, so a new binding belongs in both places
/// and `test/shortcut_sheet_test.dart` keeps the count honest.
const List<OrShortcutGroup> orShortcutGroups = [
  OrShortcutGroup('Start', [
    OrShortcut('Ctrl/⌘ K', 'Command palette'),
    OrShortcut('Ctrl/⌘ N', 'New project'),
    OrShortcut('Ctrl/⌘ O', 'Open project'),
    OrShortcut('Ctrl/⌘ I', 'Import media'),
    OrShortcut('Ctrl/⌘ ⇧ I', 'Import a folder'),
  ]),
  OrShortcutGroup('While editing', [
    OrShortcut('Ctrl/⌘ S', 'Save'),
    OrShortcut('Ctrl/⌘ E', 'Export'),
    OrShortcut('Ctrl/⌘ Z', 'Undo'),
    OrShortcut('Ctrl/⌘ ⇧ Z', 'Redo'),
    OrShortcut('Ctrl/⌘ D', 'Duplicate the selected clip'),
    OrShortcut('Ctrl/⌘ ⇧ A', 'Show the assistant'),
    OrShortcut('Ctrl/⌘ B  or  S', 'Split at the playhead'),
    OrShortcut('Delete', 'Delete the selected clip'),
    OrShortcut('⇧ Delete', 'Ripple delete the selected clip'),
    OrShortcut('C  /  V', 'Blade mode  /  select mode'),
    OrShortcut('M', 'Add a marker at the playhead'),
    OrShortcut('Esc', 'Clear the selection'),
  ]),
  OrShortcutGroup('Playback', [
    OrShortcut('Space', 'Play or pause'),
    OrShortcut('←  /  →', 'Move one second'),
    OrShortcut('⇧ ←  /  ⇧ →', 'Step one frame'),
    OrShortcut('[  /  ]', 'Move five seconds'),
    OrShortcut(',  /  .', 'Step one frame'),
  ]),
  OrShortcutGroup('Timeline and view', [
    OrShortcut('+  /  −', 'Zoom the timeline'),
    OrShortcut('F5', 'Fit the timeline'),
    OrShortcut('G', 'Composition guides'),
    OrShortcut('Alt ← ↑ → ↓', 'Nudge the selected clip'),
    OrShortcut('1 2 3 4', 'Media, Text, Audio, Motion'),
    OrShortcut('Shift + scroll', 'Scroll the timeline sideways'),
  ]),
  OrShortcutGroup('Help', [OrShortcut('?  or  Ctrl/⌘ /', 'This list')]),
];

/// Shows the shortcut sheet.
Future<void> showOrShortcutSheet(BuildContext context) => showDialog<void>(
  context: context,
  builder: (_) => const _ShortcutSheetDialog(),
);

class _ShortcutSheetDialog extends StatelessWidget {
  const _ShortcutSheetDialog();

  @override
  Widget build(BuildContext context) => AlertDialog(
    key: const ValueKey('shortcut-sheet'),
    title: const Text('Keyboard shortcuts'),
    content: SizedBox(
      width: 520,
      height: 460,
      child: ListView(
        children: [
          for (final group in orShortcutGroups) ...[
            Padding(
              padding: const EdgeInsets.only(
                top: OrSpacing.x2,
                bottom: OrSpacing.x1,
              ),
              child: Text(group.title, style: OrType.control),
            ),
            for (final row in group.rows)
              Padding(
                key: ValueKey('shortcut-sheet-row-${row.action}'),
                padding: const EdgeInsets.symmetric(vertical: 3),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: 150,
                      child: Text(
                        row.keys,
                        style: OrType.metadata.copyWith(
                          color: OrColors.textSecondary,
                        ),
                      ),
                    ),
                    const SizedBox(width: OrSpacing.x2),
                    Expanded(child: Text(row.action, style: OrType.body)),
                  ],
                ),
              ),
          ],
        ],
      ),
    ),
    actions: [
      TextButton(
        key: const ValueKey('shortcut-sheet-close'),
        onPressed: () => Navigator.of(context).pop(),
        child: const Text('Close'),
      ),
    ],
  );
}
