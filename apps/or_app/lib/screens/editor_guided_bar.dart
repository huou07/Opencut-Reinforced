import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';

/// The one next action the guided row offers.
///
/// The order is the order a first video is actually made in: something to show,
/// a clip on the timeline, a title, sound, then the file. A step disappears when
/// the state proves it was done, so the row never asks for work already
/// finished and never becomes a static tutorial.
enum GuidedStepKind {
  importMedia,
  addFirstClip,
  addTitle,
  addAudio,
  exportVideo,
}

class GuidedStep {
  const GuidedStep({
    required this.kind,
    required this.title,
    required this.detail,
    required this.actionLabel,
  });

  final GuidedStepKind kind;
  final String title;
  final String detail;
  final String actionLabel;
}

/// The next step, or null when the project has already produced a file.
///
/// Pure on purpose: the rule is the product behaviour, so it is tested without
/// a widget tree, and the row cannot drift from the state it claims to read.
GuidedStep? guidedNextStep({
  required bool hasMedia,
  required int clipCount,
  required bool hasText,
  required bool hasAudio,
  required bool hasExported,
}) {
  if (hasExported) return null;
  if (clipCount == 0) {
    if (!hasMedia) {
      return const GuidedStep(
        kind: GuidedStepKind.importMedia,
        title: 'Start with something to show',
        detail:
            'Import a video, a photo or a recording. It stays in its folder.',
        actionLabel: 'Import media',
      );
    }
    return const GuidedStep(
      kind: GuidedStepKind.addFirstClip,
      title: 'Put your first clip on the timeline',
      detail: 'The clip appears at the start; drag or trim it after that.',
      actionLabel: 'Add first clip',
    );
  }
  if (!hasText) {
    return const GuidedStep(
      kind: GuidedStepKind.addTitle,
      title: 'Give the video a title',
      detail:
          'A title names the story and is the fastest way to look finished.',
      actionLabel: 'Add a title',
    );
  }
  if (!hasAudio) {
    return const GuidedStep(
      kind: GuidedStepKind.addAudio,
      title: 'Add sound',
      detail: 'A voiceover or a music bed carries the pace of the edit.',
      actionLabel: 'Add sound',
    );
  }
  return const GuidedStep(
    kind: GuidedStepKind.exportVideo,
    title: 'Export your video',
    detail: 'One file, made on this machine. You can export again after edits.',
    actionLabel: 'Export video',
  );
}

/// The guided step as one inline toolbar item.
///
/// It lives inside the timeline toolbar instead of taking its own row, so the
/// panes keep the height they would have without guidance: a next step can
/// never shrink the inspector a user is scrolling or push a clip under a
/// transient snackbar. The toolbar already scrolls, so the item costs nothing
/// when a wide window has room and still fits a narrow one.
class EditorGuidedBar extends StatelessWidget {
  const EditorGuidedBar({
    super.key,
    required this.step,
    required this.onAction,
    required this.onDismiss,
    this.busy = false,
  });

  final GuidedStep step;
  final VoidCallback onAction;
  final VoidCallback onDismiss;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    return Padding(
      key: const ValueKey('editor-guided-bar'),
      padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x1),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Tooltip(
            message: step.detail,
            child: const Icon(
              Icons.tips_and_updates_outlined,
              size: 15,
              color: OrColors.selection,
            ),
          ),
          const SizedBox(width: OrSpacing.x1),
          Text(
            step.title,
            key: const ValueKey('editor-guided-title'),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(
              color: OrColors.text,
              fontSize: OrType.tinySize,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(width: OrSpacing.x2),
          TextButton(
            key: ValueKey('editor-guided-action-${step.kind.name}'),
            onPressed: busy ? null : onAction,
            // A compact density would shrink this below the 24 px pointer
            // minimum the design language requires, so the row keeps its
            // small type without shrinking the target.
            style: TextButton.styleFrom(
              minimumSize: const Size(0, 26),
              padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x2),
              textStyle: const TextStyle(fontSize: OrType.tinySize),
            ),
            child: Text(step.actionLabel),
          ),
          IconButton(
            key: const ValueKey('editor-guided-dismiss'),
            tooltip: 'Hide next-step tips',
            constraints: const BoxConstraints.tightFor(width: 26, height: 26),
            padding: EdgeInsets.zero,
            onPressed: onDismiss,
            icon: const Icon(Icons.close, size: 14),
            color: OrColors.textMuted,
          ),
        ],
      ),
    );
  }
}
