import 'dart:math' as math;

import '../project/project_gateway.dart';

/// One-tap motion built from ordinary keyframes.
///
/// A preset is only a starting point: it writes the same animation command the
/// manual controls use, so the result can be edited or cleared afterwards and
/// the project file stores keyframes, not a preset name.
class MotionPreset {
  const MotionPreset({
    required this.wire,
    required this.label,
    required this.build,
  });

  /// Stable identifier for tests and widget keys.
  final String wire;
  final String label;

  /// `current` keeps the channels the preset does not touch, `span` is how long
  /// the entrance lasts, and `delay` moves it later from the clip's own start.
  final ProjectClipAnimation Function(
    ProjectClipAnimation current,
    double span,
    double delay,
  )
  build;
}

/// The entrance presets the inspector offers and the stagger action repeats.
const List<MotionPreset> motionPresets = [
  MotionPreset(wire: 'fade-in', label: 'Fade in', build: fadeInMotion),
  MotionPreset(wire: 'slide-up', label: 'Slide up', build: slideUpMotion),
  MotionPreset(wire: 'zoom-in', label: 'Zoom in', build: zoomInMotion),
  MotionPreset(wire: 'pop', label: 'Pop', build: popMotion),
];

MotionPreset? motionPresetByWire(String wire) {
  for (final preset in motionPresets) {
    if (preset.wire == wire) return preset;
  }
  return null;
}

/// A keyframe at [at] seconds from the clip start, in the millisecond basis the
/// animation view accepts.
ProjectKeyframe motionKeyframe(
  double at,
  int value,
  ProjectKeyframeEasing easing,
) => ProjectKeyframe(at: motionTime(at), value: value, easing: easing);

ProjectRationalTime motionTime(double seconds) =>
    ProjectRationalTime(BigInt.from((seconds * 1000).round()), 1000);

/// The entrance length that fits a clip, short enough to read on a cut and long
/// enough not to be a single-frame flash.
double motionSpanFor(double clipDurationSeconds) =>
    clipDurationSeconds.clamp(minimumStaggerSpan, maximumStaggerSpan);

/// The shortest entrance worth writing: below this the value never visibly
/// travels, so a stagger skips the clip instead of writing a stutter.
const double minimumStaggerSpan = 0.05;

/// The longest entrance a preset uses even on a long clip.
const double maximumStaggerSpan = 0.7;

ProjectClipAnimation fadeInMotion(
  ProjectClipAnimation current,
  double span,
  double delay,
) => current.copyWith(
  opacityBasisPoints: [
    motionKeyframe(delay, 0, ProjectKeyframeEasing.easeOut),
    motionKeyframe(delay + span, 10000, ProjectKeyframeEasing.linear),
  ],
);

ProjectClipAnimation slideUpMotion(
  ProjectClipAnimation current,
  double span,
  double delay,
) => current.copyWith(
  yMilliCanvas: [
    motionKeyframe(delay, 120, ProjectKeyframeEasing.easeOut),
    motionKeyframe(delay + span, 0, ProjectKeyframeEasing.linear),
  ],
);

ProjectClipAnimation zoomInMotion(
  ProjectClipAnimation current,
  double span,
  double delay,
) => current.copyWith(
  scaleXMilli: [
    motionKeyframe(delay, 850, ProjectKeyframeEasing.easeOut),
    motionKeyframe(delay + span, 1000, ProjectKeyframeEasing.linear),
  ],
  scaleYMilli: [
    motionKeyframe(delay, 850, ProjectKeyframeEasing.easeOut),
    motionKeyframe(delay + span, 1000, ProjectKeyframeEasing.linear),
  ],
);

ProjectClipAnimation popMotion(
  ProjectClipAnimation current,
  double span,
  double delay,
) => current.copyWith(
  scaleXMilli: [
    motionKeyframe(delay, 1000, ProjectKeyframeEasing.easeOut),
    motionKeyframe(delay + span / 2, 1120, ProjectKeyframeEasing.easeInOut),
    motionKeyframe(delay + span, 1000, ProjectKeyframeEasing.linear),
  ],
  scaleYMilli: [
    motionKeyframe(delay, 1000, ProjectKeyframeEasing.easeOut),
    motionKeyframe(delay + span / 2, 1120, ProjectKeyframeEasing.easeInOut),
    motionKeyframe(delay + span, 1000, ProjectKeyframeEasing.linear),
  ],
);

/// One clip's place in a staggered entrance.
class StaggerStep {
  const StaggerStep({
    required this.clipId,
    required this.delaySeconds,
    required this.spanSeconds,
  });

  final String clipId;
  final double delaySeconds;
  final double spanSeconds;
}

/// Which clips a stagger can animate and when each one starts.
///
/// Clips are listed in the order the cascade follows. A clip that cannot hold
/// the requested delay plus a minimum entrance is skipped rather than given a
/// truncated animation that would pop in mid-clip.
class StaggerPlan {
  const StaggerPlan({required this.steps, required this.skippedClipIds});

  final List<StaggerStep> steps;
  final List<String> skippedClipIds;

  bool get isEmpty => steps.isEmpty;
  int get clipCount => steps.length + skippedClipIds.length;
}

/// Spreads one entrance across clips in timeline order.
///
/// The offset is counted from each clip's own start, so the cascade reads the
/// same whether the clips are adjacent or on separate tracks.
StaggerPlan planStagger({
  required List<String> orderedClipIds,
  required List<double> durationsSeconds,
  required double delaySeconds,
}) {
  assert(orderedClipIds.length == durationsSeconds.length);
  final delay = delaySeconds.isFinite && delaySeconds > 0 ? delaySeconds : 0.0;
  final steps = <StaggerStep>[];
  final skipped = <String>[];
  for (var index = 0; index < orderedClipIds.length; index++) {
    final duration = durationsSeconds[index];
    final at = index * delay;
    if (duration + 1e-9 < at + minimumStaggerSpan) {
      skipped.add(orderedClipIds[index]);
      continue;
    }
    steps.add(
      StaggerStep(
        clipId: orderedClipIds[index],
        delaySeconds: at,
        spanSeconds: math.min(
          maximumStaggerSpan,
          math.max(minimumStaggerSpan, duration - at),
        ),
      ),
    );
  }
  return StaggerPlan(steps: steps, skippedClipIds: skipped);
}
