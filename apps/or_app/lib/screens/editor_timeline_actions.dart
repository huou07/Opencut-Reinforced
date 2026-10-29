import 'dart:async';

import 'package:flutter/material.dart';

import '../project/project_gateway.dart';
import 'editor_timeline_dialogs.dart';
import 'editor_timeline_shared.dart';

Future<void> showRenameTimelineMarkerDialog({
  required BuildContext context,
  required ProjectReadModel project,
  required ProjectTimelineMarker marker,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineMarker,
    String,
  )
  onRename,
}) async {
  final controller = TextEditingController(text: marker.label);
  bool isValid() =>
      controller.text.trim().isNotEmpty && controller.text.trim().length <= 256;
  final valid = ValueNotifier(isValid());
  void updateValidity() => valid.value = isValid();
  controller.addListener(updateValidity);
  try {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text('Rename ${marker.label}'),
        content: TextField(
          key: ValueKey('timeline-marker-rename-label-${marker.markerId}'),
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(labelText: 'Label'),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          ValueListenableBuilder<bool>(
            valueListenable: valid,
            builder: (context, enabled, child) => FilledButton(
              key: ValueKey('timeline-confirm-rename-${marker.markerId}'),
              onPressed: enabled
                  ? () {
                      final label = controller.text.trim();
                      if (label.isEmpty || label.length > 256) return;
                      Navigator.of(dialogContext).pop();
                      unawaited(onRename(project, marker, label));
                    }
                  : null,
              child: const Text('Rename'),
            ),
          ),
        ],
      ),
    );
  } finally {
    await Future<void>.delayed(const Duration(milliseconds: 250));
    controller.removeListener(updateValidity);
    controller.dispose();
    valid.dispose();
  }
}

Future<void> confirmDeleteTimelineMarker({
  required BuildContext context,
  required ProjectReadModel project,
  required ProjectTimelineMarker marker,
  required Future<void> Function(ProjectReadModel, ProjectTimelineMarker)
  onDelete,
}) async {
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (dialogContext) => AlertDialog(
      title: const Text('Delete marker?'),
      content: Text('Delete marker "${marker.label}"?'),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(dialogContext).pop(false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: ValueKey('timeline-confirm-delete-marker-${marker.markerId}'),
          onPressed: () => Navigator.of(dialogContext).pop(true),
          child: const Text('Delete'),
        ),
      ],
    ),
  );
  if (confirmed == true) await onDelete(project, marker);
}

Future<void> showTimelineClipActions({
  required BuildContext context,
  required ProjectReadModel? project,
  required List<ProjectTimelineTrack> tracks,
  required ProjectTimelineTrack track,
  required ProjectTimelineClip clip,
  required ProjectRationalTime? playhead,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    String,
    ProjectRationalTime,
  )?
  onMove,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineClip,
  )?
  onDuplicate,
  required Future<void> Function(ProjectReadModel, ProjectTimelineClip)?
  onDelete,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectTimelineTrimEdge,
    ProjectRationalTime,
    bool ripple,
  )?
  onTrim,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onSlip,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onRoll,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onSlide,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onSplit,
  required Future<void> Function(ProjectReadModel, ProjectTimelineClip)?
  onRippleDelete,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineClip,
    ProjectRationalTime,
    ProjectTimelineTextContent,
  )?
  onEditTextClip,
  required bool busy,
}) async {
  final isMedia = clip.contentKind == ProjectTimelineClipContentKind.media;
  final isCaption = clip.contentKind == ProjectTimelineClipContentKind.caption;
  final canMove =
      project != null &&
      !track.state.locked &&
      onMove != null &&
      tracks.any(
        (candidate) => candidate.kind == track.kind && !candidate.state.locked,
      );
  final canDuplicate =
      project != null && !track.state.locked && onDuplicate != null;
  final canTrim =
      isMedia && project != null && !track.state.locked && onTrim != null;
  final canSplit =
      isMedia && project != null && !track.state.locked && onSplit != null;
  final splitAtPlayhead = playhead;
  final canSplitAtPlayhead =
      canSplit &&
      splitAtPlayhead != null &&
      compareProjectRational(splitAtPlayhead, clip.timelineStart) > 0 &&
      compareProjectRational(splitAtPlayhead, clip.timelineEnd) < 0;
  final canEditText =
      !isMedia &&
      project != null &&
      !track.state.locked &&
      onEditTextClip != null;
  final canRippleDelete =
      project != null && !track.state.locked && onRippleDelete != null;
  await showDialog<void>(
    context: context,
    builder: (dialogContext) => AlertDialog(
      // Keep the action rows compact on short screens.
      scrollable: true,
      actionsPadding: const EdgeInsets.only(
        left: 8,
        top: 0,
        right: 8,
        bottom: 8,
      ),
      buttonPadding: EdgeInsets.zero,
      title: const Text('Clip'),
      content: Text(switch (clip.contentKind) {
        ProjectTimelineClipContentKind.media =>
          'Timeline start: ${clip.timelineStart.canonical}\nSource range: ${clip.sourceStart?.canonical ?? ''} + ${clip.timelineDuration.canonical}',
        ProjectTimelineClipContentKind.text =>
          'Title: ${clip.text ?? ''}\nTimeline start: ${clip.timelineStart.canonical}\nDuration: ${clip.timelineDuration.canonical}',
        ProjectTimelineClipContentKind.caption =>
          'Caption: ${clip.text ?? ''}\nTimeline start: ${clip.timelineStart.canonical}\nDuration: ${clip.timelineDuration.canonical}',
      }),
      actions: [
        SizedBox(
          width: (MediaQuery.sizeOf(dialogContext).width - 96)
              .clamp(0.0, 512.0)
              .toDouble(),
          child: ConstrainedBox(
            constraints: BoxConstraints(
              maxHeight: MediaQuery.sizeOf(dialogContext).width < 600
                  ? (MediaQuery.sizeOf(dialogContext).height * 0.42)
                        .clamp(144.0, 220.0)
                        .toDouble()
                  : MediaQuery.sizeOf(dialogContext).height,
            ),
            child: SingleChildScrollView(
              child: OverflowBar(
                alignment: MainAxisAlignment.end,
                overflowAlignment: OverflowBarAlignment.end,
                children: [
                  TextButton(
                    onPressed: () => Navigator.of(dialogContext).pop(),
                    child: const Text('Close'),
                  ),
                  TextButton(
                    key: ValueKey('timeline-move-${clip.clipId}'),
                    onPressed: busy || !canMove
                        ? null
                        : () {
                            Navigator.of(dialogContext).pop();
                            unawaited(
                              showMoveTimelineClipDialog(
                                context: context,
                                project: project,
                                tracks: tracks,
                                sourceTrack: track,
                                clip: clip,
                                onMove: onMove,
                              ),
                            );
                          },
                    child: const Text('Move'),
                  ),
                  TextButton(
                    key: ValueKey('timeline-duplicate-${clip.clipId}'),
                    onPressed: busy || !canDuplicate
                        ? null
                        : () {
                            Navigator.of(dialogContext).pop();
                            unawaited(onDuplicate(project, track, clip));
                          },
                    child: const Text('Duplicate'),
                  ),
                  if (!isMedia)
                    TextButton(
                      key: ValueKey('timeline-edit-text-${clip.clipId}'),
                      onPressed: busy || !canEditText
                          ? null
                          : () {
                              Navigator.of(dialogContext).pop();
                              unawaited(
                                editTimelineTextClip(
                                  context: context,
                                  project: project,
                                  track: track,
                                  clip: clip,
                                  onUpdate: onEditTextClip,
                                ),
                              );
                            },
                      child: Text(isCaption ? 'Edit Caption' : 'Edit Title'),
                    ),
                  TextButton(
                    key: ValueKey('timeline-trim-${clip.clipId}'),
                    onPressed: busy || !canTrim
                        ? null
                        : () {
                            Navigator.of(dialogContext).pop();
                            unawaited(
                              showTrimTimelineClipDialog(
                                context: context,
                                project: project,
                                clip: clip,
                                onTrim: onTrim,
                              ),
                            );
                          },
                    child: const Text('Trim'),
                  ),
                  TextButton(
                    key: ValueKey('timeline-slip-${clip.clipId}'),
                    onPressed:
                        busy || !isMedia || track.state.locked || onSlip == null
                        ? null
                        : () {
                            Navigator.of(dialogContext).pop();
                            unawaited(
                              showSlipTimelineClipDialog(
                                context: context,
                                project: project,
                                clip: clip,
                                onSlip: onSlip,
                              ),
                            );
                          },
                    child: const Text('Slip'),
                  ),
                  TextButton(
                    key: ValueKey('timeline-roll-${clip.clipId}'),
                    onPressed:
                        busy || !isMedia || track.state.locked || onRoll == null
                        ? null
                        : () {
                            Navigator.of(dialogContext).pop();
                            unawaited(
                              showBoundaryMoveDialog(
                                context: context,
                                project: project,
                                clip: clip,
                                title: 'Roll Clip',
                                helper:
                                    'Move the boundary between this clip and the next one. '
                                    'Both clips stay where they are; their shared edge moves.',
                                confirmKey: const ValueKey(
                                  'timeline-confirm-roll',
                                ),
                                fieldKey: const ValueKey('timeline-roll-delta'),
                                onConfirm: onRoll,
                              ),
                            );
                          },
                    child: const Text('Roll'),
                  ),
                  TextButton(
                    key: ValueKey('timeline-slide-${clip.clipId}'),
                    onPressed:
                        busy ||
                            !isMedia ||
                            track.state.locked ||
                            onSlide == null
                        ? null
                        : () {
                            Navigator.of(dialogContext).pop();
                            unawaited(
                              showBoundaryMoveDialog(
                                context: context,
                                project: project,
                                clip: clip,
                                title: 'Slide Clip',
                                helper:
                                    'Move this clip in time; the clips either side give up or '
                                    'gain the same amount, so the track stays gapless.',
                                confirmKey: const ValueKey(
                                  'timeline-confirm-slide',
                                ),
                                fieldKey: const ValueKey(
                                  'timeline-slide-delta',
                                ),
                                onConfirm: onSlide,
                              ),
                            );
                          },
                    child: const Text('Slide'),
                  ),
                  TextButton(
                    key: ValueKey('timeline-split-${clip.clipId}'),
                    onPressed: busy || !canSplit
                        ? null
                        : () {
                            Navigator.of(dialogContext).pop();
                            unawaited(
                              showSplitTimelineClipDialog(
                                context: context,
                                project: project,
                                clip: clip,
                                onSplit: onSplit,
                              ),
                            );
                          },
                    child: const Text('Split'),
                  ),
                  if (isMedia)
                    Tooltip(
                      message: canSplitAtPlayhead
                          ? 'Split at ${splitAtPlayhead.canonical}'
                          : 'Move the playhead inside this clip to split at it',
                      child: TextButton(
                        key: ValueKey(
                          'timeline-split-at-playhead-${clip.clipId}',
                        ),
                        onPressed: busy || !canSplitAtPlayhead
                            ? null
                            : () {
                                final activeProject = project;
                                final split = onSplit;
                                final position = splitAtPlayhead;
                                Navigator.of(dialogContext).pop();
                                unawaited(split(activeProject, clip, position));
                              },
                        child: const Text('Split at Playhead'),
                      ),
                    ),
                  TextButton(
                    key: ValueKey('timeline-delete-${clip.clipId}'),
                    onPressed:
                        busy ||
                            track.state.locked ||
                            project == null ||
                            onDelete == null
                        ? null
                        : () {
                            Navigator.of(dialogContext).pop();
                            unawaited(
                              confirmDeleteTimelineClip(
                                context: context,
                                project: project,
                                clip: clip,
                                onDelete: onDelete,
                              ),
                            );
                          },
                    child: const Text('Delete'),
                  ),
                  TextButton(
                    key: ValueKey('timeline-ripple-delete-${clip.clipId}'),
                    onPressed: busy || !canRippleDelete
                        ? null
                        : () {
                            Navigator.of(dialogContext).pop();
                            unawaited(
                              confirmRippleDeleteTimelineClip(
                                context: context,
                                project: project,
                                clip: clip,
                                onRippleDelete: onRippleDelete,
                              ),
                            );
                          },
                    child: const Text('Ripple Delete'),
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    ),
  );
}
