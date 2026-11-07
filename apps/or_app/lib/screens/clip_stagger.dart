import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../project/project_gateway.dart';
import 'clip_motion_presets.dart';

/// What the owner chose in the stagger dialog.
class StaggerRequest {
  const StaggerRequest({required this.preset, required this.delaySeconds});

  final MotionPreset preset;
  final double delaySeconds;
}

/// What a stagger actually did, so the caller can report it without inventing
/// success for the clips the plan skipped.
class StaggerOutcome {
  const StaggerOutcome({
    required this.appliedClipIds,
    required this.skippedClipIds,
    this.failure,
  });

  final List<String> appliedClipIds;
  final List<String> skippedClipIds;

  /// The first error that stopped the cascade, or null when it ran to the end.
  final String? failure;

  bool get appliedAny => appliedClipIds.isNotEmpty;
  int get clipCount => appliedClipIds.length + skippedClipIds.length;
}

/// Applies one entrance to several clips, each starting a little later than the
/// one before.
///
/// Every clip is one ordinary animation update, so the cascade lands in the same
/// history the manual keyframe editor writes; the clips after an error are left
/// untouched rather than half-staggered.
Future<StaggerOutcome> applyStagger({
  required ProjectGateway gateway,
  required ProjectSessionHandle session,
  required ProjectReadModel project,
  required List<ProjectTimelineClip> clips,
  required Map<String, String> trackIdByClipId,
  required MotionPreset preset,
  required double delaySeconds,
}) async {
  final plan = planStagger(
    orderedClipIds: [for (final clip in clips) clip.clipId],
    durationsSeconds: [
      for (final clip in clips) clip.timelineDuration.secondsForDisplay,
    ],
    delaySeconds: delaySeconds,
  );
  final applied = <String>[];
  var current = project;
  String? failure;
  for (final step in plan.steps) {
    final trackId = trackIdByClipId[step.clipId];
    if (trackId == null) {
      failure = 'The clip is no longer on a track.';
      break;
    }
    try {
      final animation = await gateway.getTimelineClipAnimation(
        session,
        current,
        trackId: trackId,
        clipId: step.clipId,
      );
      final next = preset.build(animation, step.spanSeconds, step.delaySeconds);
      final result = await gateway.updateTimelineClipAnimation(
        session,
        current,
        trackId: trackId,
        clipId: step.clipId,
        animation: next,
      );
      if (!result.succeeded) {
        failure = result.message.isEmpty
            ? 'The animation was not saved.'
            : result.message;
        break;
      }
      current = result.view ?? current;
      applied.add(step.clipId);
    } on ProjectGatewayException catch (error) {
      failure = error.message;
      break;
    }
  }
  return StaggerOutcome(
    appliedClipIds: applied,
    skippedClipIds: plan.skippedClipIds,
    failure: failure,
  );
}

/// Asks which entrance to cascade and how far apart the clips should start.
Future<StaggerRequest?> showStaggerDialog({
  required BuildContext context,
  required int clipCount,
}) => showDialog<StaggerRequest>(
  context: context,
  builder: (context) => _StaggerDialog(clipCount: clipCount),
);

class _StaggerDialog extends StatefulWidget {
  const _StaggerDialog({required this.clipCount});

  final int clipCount;

  @override
  State<_StaggerDialog> createState() => _StaggerDialogState();
}

class _StaggerDialogState extends State<_StaggerDialog> {
  MotionPreset _preset = motionPresets.first;
  double _delay = 0.2;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      key: const ValueKey('stagger-dialog'),
      title: const Text('Stagger animation'),
      content: SizedBox(
        width: 360,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'One entrance on ${widget.clipCount} clips, each starting a '
              'little later than the one before.',
              style: TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.tinySize,
              ),
            ),
            const SizedBox(height: OrSpacing.x2),
            Wrap(
              spacing: OrSpacing.x1,
              runSpacing: OrSpacing.x1,
              children: [
                for (final preset in motionPresets)
                  ChoiceChip(
                    key: ValueKey('stagger-preset-${preset.wire}'),
                    label: Text(preset.label),
                    selected: preset.wire == _preset.wire,
                    onSelected: (_) => setState(() => _preset = preset),
                  ),
              ],
            ),
            const SizedBox(height: OrSpacing.x2),
            Row(
              children: [
                Expanded(
                  child: Text(
                    'Delay between clips',
                    style: TextStyle(
                      color: OrColors.text,
                      fontSize: OrType.metadataSize,
                    ),
                  ),
                ),
                Text(
                  '${_delay.toStringAsFixed(2)} s',
                  key: const ValueKey('stagger-delay-value'),
                  style: TextStyle(
                    color: OrColors.textMuted,
                    fontSize: OrType.tinySize,
                  ),
                ),
              ],
            ),
            Slider(
              key: const ValueKey('stagger-delay'),
              value: _delay,
              min: 0,
              max: 0.5,
              divisions: 10,
              label: '${_delay.toStringAsFixed(2)} s',
              onChanged: (value) => setState(() => _delay = value),
            ),
            Text(
              'Clips too short for their delay keep their current animation.',
              style: TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.tinySize,
              ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          key: const ValueKey('stagger-cancel'),
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const ValueKey('stagger-apply'),
          onPressed: () =>
              Navigator.of(context)
                  .pop(StaggerRequest(preset: _preset, delaySeconds: _delay)),
          child: const Text('Stagger'),
        ),
      ],
    );
  }
}
