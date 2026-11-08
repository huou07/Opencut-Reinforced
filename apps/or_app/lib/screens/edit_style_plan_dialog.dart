import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../library/creative_directions.dart';
import '../library/edit_styles.dart';

/// The cut a style would build, shown before anything is written.
///
/// The list is the plan object itself, so what a person approves is exactly
/// what runs. Every line carries its reason, because a style is a claim about
/// pacing and the claim should be checkable.
class EditStylePlanDialog extends StatelessWidget {
  const EditStylePlanDialog({
    super.key,
    required this.plan,
    required this.title,
    required this.labels,
    required this.appendsAfter,
  });

  final EditStylePlan plan;

  /// The title text that will go on the text track, or null for no title.
  final String? title;

  /// Media id to the name a person recognises.
  final Map<String, String> labels;

  /// True when the cut lands after the clips already on the timeline.
  final bool appendsAfter;

  static const _applyKey = ValueKey('edit-style-apply');

  @override
  Widget build(BuildContext context) {
    final style = plan.style;
    return AlertDialog(
      title: Text('Build a ${style.name} cut'),
      content: SizedBox(
        width: OrDialogSizes.width(context, 460),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                plan.summary,
                key: const ValueKey('edit-style-summary'),
                style: const TextStyle(
                  color: OrColors.text,
                  fontSize: OrType.bodySize,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: OrSpacing.x1),
              Text(
                style.pacing,
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
              const SizedBox(height: OrSpacing.x3),
              for (final (index, shot) in plan.shots.indexed) ...[
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: 24,
                      child: Text(
                        '${index + 1}',
                        style: const TextStyle(
                          color: OrColors.textMuted,
                          fontSize: OrType.metadataSize,
                        ),
                      ),
                    ),
                    Expanded(
                      child: Text(
                        '${labels[shot.mediaId] ?? shot.mediaId} · '
                        '${shot.durationLabel(plan.frameRate)}',
                        style: const TextStyle(
                          color: OrColors.text,
                          fontSize: OrType.metadataSize,
                        ),
                      ),
                    ),
                  ],
                ),
                Padding(
                  padding: const EdgeInsets.only(
                    left: 24,
                    bottom: OrSpacing.x2,
                  ),
                  child: Text(
                    shot.why,
                    key: ValueKey('edit-style-shot-why-$index'),
                    style: const TextStyle(
                      color: OrColors.textMuted,
                      fontSize: OrType.metadataSize,
                    ),
                  ),
                ),
              ],
              if (plan.notes.isNotEmpty) ...[
                const SizedBox(height: OrSpacing.x1),
                for (final (index, note) in plan.notes.indexed)
                  Padding(
                    padding: const EdgeInsets.only(bottom: OrSpacing.x1),
                    child: Text(
                      note,
                      key: ValueKey('edit-style-note-$index'),
                      style: const TextStyle(
                        color: OrColors.warning,
                        fontSize: OrType.metadataSize,
                      ),
                    ),
                  ),
              ],
              const SizedBox(height: OrSpacing.x2),
              Text(
                [
                  if (title != null)
                    'A title clip "${title!}" goes on a text track above the cut.',
                  if (appendsAfter)
                    'The cut is added after the clips already on the timeline.',
                  'These are ordinary clips: trim, move, relabel or delete them '
                      'like any other. Each insert is its own step, so Undo '
                      'removes them one at a time.',
                ].join(' '),
                key: const ValueKey('edit-style-effect'),
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: _applyKey,
          onPressed: () => Navigator.of(context).pop(true),
          child: const Text('Build the cut'),
        ),
      ],
    );
  }
}

/// A creative direction as a brief: readable, copyable, and editable by hand.
class CreativeDirectionDialog extends StatelessWidget {
  const CreativeDirectionDialog({
    super.key,
    required this.direction,
    required this.onCopy,
    required this.onAddPinned,
  });

  final CreativeDirection direction;
  final VoidCallback onCopy;

  /// Keeps the direction in the library as one of the person's own.
  final VoidCallback? onAddPinned;

  @override
  Widget build(BuildContext context) {
    final sections = <(String, String)>[
      ('Voice and narration', direction.voice),
      ('Script shape', direction.script),
      ('Storytelling', direction.storytelling),
      ('Graphics', direction.graphics),
      ('Captions', direction.captions),
      ('Editing', direction.editing),
      ('Audio', direction.audio),
    ];
    return AlertDialog(
      title: Text(direction.name),
      content: SizedBox(
        width: OrDialogSizes.width(context, 460),
        child: SingleChildScrollView(
          key: const ValueKey('creative-direction-brief'),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                direction.summary,
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
              const SizedBox(height: OrSpacing.x3),
              for (final (name, body) in sections) ...[
                Text(
                  name.toUpperCase(),
                  style: const TextStyle(
                    color: OrColors.textMuted,
                    fontSize: OrType.metadataSize,
                    letterSpacing: 0.6,
                  ),
                ),
                const SizedBox(height: OrSpacing.x1),
                Text(
                  body,
                  style: const TextStyle(
                    color: OrColors.text,
                    fontSize: OrType.metadataSize,
                  ),
                ),
                const SizedBox(height: OrSpacing.x3),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          key: const ValueKey('creative-direction-copy'),
          onPressed: () {
            onCopy();
            Navigator.of(context).pop();
          },
          child: const Text('Copy brief'),
        ),
        if (onAddPinned != null)
          TextButton(
            key: const ValueKey('creative-direction-pin'),
            onPressed: () {
              onAddPinned!();
              Navigator.of(context).pop();
            },
            child: const Text('Keep a copy'),
          ),
        TextButton(
          key: const ValueKey('creative-direction-close'),
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
      ],
    );
  }
}
