import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../project/project_gateway.dart';
import 'editor_timeline_shared.dart';

class TimelineTextClipEdit {
  const TimelineTextClipEdit({required this.content, required this.duration});

  final ProjectTimelineTextContent content;
  final ProjectRationalTime duration;
}

Future<void> editTimelineTextClip({
  required BuildContext context,
  required ProjectReadModel? project,
  required ProjectTimelineTrack track,
  required ProjectTimelineClip clip,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineClip,
    ProjectRationalTime,
    ProjectTimelineTextContent,
  )?
  onUpdate,
}) async {
  if (project == null || onUpdate == null) return;
  final edit = await showTimelineTextClipDialog(
    context: context,
    contentKind: clip.contentKind,
    text: clip.text ?? '',
    duration: clip.timelineDuration,
    formatting: clip.formatting ?? ProjectTextFormatting.defaults,
  );
  if (edit != null) {
    await onUpdate(project, track, clip, edit.duration, edit.content);
  }
}

int greatestCommonDivisor(int left, int right) {
  var a = left.abs();
  var b = right.abs();
  while (b != 0) {
    final remainder = a % b;
    a = b;
    b = remainder;
  }
  return a == 0 ? 1 : a;
}

String playbackSpeedLabel(ProjectRationalRate rate) {
  final value = rate.numerator / rate.denominator;
  if (rate.numerator == rate.denominator) return '1×';
  return '${value.toStringAsFixed(value == value.roundToDouble() ? 0 : 2)}×';
}

Future<ProjectRationalRate?> showPlaybackSpeedDialog(
  BuildContext context,
  ProjectRationalRate initial,
) async {
  const presets = [
    ProjectRationalRate(1, 4),
    ProjectRationalRate(1, 2),
    ProjectRationalRate(1, 1),
    ProjectRationalRate(3, 2),
    ProjectRationalRate(2, 1),
    ProjectRationalRate(4, 1),
  ];
  var value = initial.numerator / initial.denominator;
  value = value.clamp(0.25, 4.0).toDouble();
  return showDialog<ProjectRationalRate>(
    context: context,
    builder: (dialogContext) => StatefulBuilder(
      builder: (dialogContext, setDialogState) => AlertDialog(
        title: const Text('Clip speed'),
        content: SizedBox(
          width: 360,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                '${value.toStringAsFixed(2)}×',
                key: const ValueKey('timeline-speed-value'),
                textAlign: TextAlign.center,
                style: const TextStyle(
                  fontSize: 24,
                  fontWeight: FontWeight.w600,
                ),
              ),
              Slider(
                key: const ValueKey('timeline-speed-slider'),
                value: value,
                min: 0.25,
                max: 4,
                divisions: 75,
                label: '${value.toStringAsFixed(2)}×',
                onChanged: (next) => setDialogState(() => value = next),
              ),
              Wrap(
                spacing: OrSpacing.x1,
                alignment: WrapAlignment.center,
                children: [
                  for (final rate in presets)
                    ChoiceChip(
                      key: ValueKey(
                        'timeline-speed-preset-${rate.numerator}-${rate.denominator}',
                      ),
                      label: Text(playbackSpeedLabel(rate)),
                      selected:
                          (rate.numerator / rate.denominator - value).abs() <
                          0.001,
                      onSelected: (_) => setDialogState(
                        () => value = rate.numerator / rate.denominator,
                      ),
                    ),
                ],
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const ValueKey('timeline-speed-save'),
            onPressed: () {
              final hundredths = (value * 100).round();
              final divisor = greatestCommonDivisor(hundredths, 100);
              Navigator.of(
                dialogContext,
              ).pop(ProjectRationalRate(hundredths ~/ divisor, 100 ~/ divisor));
            },
            child: const Text('Apply'),
          ),
        ],
      ),
    ),
  );
}

Widget textStyleSlider({
  required String keyName,
  required String label,
  required double value,
  required double min,
  required double max,
  required String display,
  required ValueChanged<double> onChanged,
}) => Row(
  children: [
    SizedBox(
      width: 104,
      child: Text(label, style: const TextStyle(fontSize: OrType.tinySize)),
    ),
    Expanded(
      child: Slider(
        key: ValueKey(keyName),
        value: value.clamp(min, max),
        min: min,
        max: max,
        divisions: (max - min).round(),
        onChanged: onChanged,
      ),
    ),
    SizedBox(
      width: 48,
      child: Text(
        display,
        textAlign: TextAlign.end,
        style: TextStyle(
          color: OrColors.textSecondary,
          fontSize: OrType.tinySize,
        ),
      ),
    ),
  ],
);

Future<TimelineTextClipEdit?> showTimelineTextClipDialog({
  required BuildContext context,
  required ProjectTimelineClipContentKind contentKind,
  String text = '',
  required ProjectRationalTime duration,
  required ProjectTextFormatting formatting,
}) async {
  if (contentKind == ProjectTimelineClipContentKind.media) return null;
  final textController = TextEditingController(text: text);
  final durationController = TextEditingController(text: duration.canonical);
  final sizeController = TextEditingController(
    text: (formatting.sizeMilliPoints / 1000)
        .toStringAsFixed(3)
        .replaceFirst(RegExp(r'\.?0+$'), ''),
  );
  var font = formatting.font;
  var weight = formatting.weight;
  var alignment = formatting.alignment;
  final colors = <ProjectTextColor>[
    ProjectTextColor.white,
    ProjectTextColor(red: 255, green: 220, blue: 90, alpha: 255),
    ProjectTextColor(red: 110, green: 210, blue: 255, alpha: 255),
  ];
  var colorIndex = colors.indexWhere(
    (color) =>
        color.red == formatting.color.red &&
        color.green == formatting.color.green &&
        color.blue == formatting.color.blue &&
        color.alpha == formatting.color.alpha,
  );
  if (colorIndex < 0) {
    colors.add(formatting.color);
    colorIndex = colors.length - 1;
  }
  var anchor = formatting.anchor;
  var offsetX = formatting.offsetXBasisPoints.toDouble();
  var offsetY = formatting.offsetYBasisPoints.toDouble();
  var opacityPercent = formatting.opacityBasisPoints / 100.0;
  var backgroundEnabled = formatting.background != null;
  var backgroundPercent = (formatting.background?.color.alpha ?? 170) / 2.55;
  bool fieldsAreValid() {
    final durationValue = ProjectRationalTime.tryParse(durationController.text);
    final sizePoints = double.tryParse(sizeController.text.trim());
    final maxTextBytes = contentKind == ProjectTimelineClipContentKind.caption
        ? 4096
        : 65536;
    return textController.text.trim().isNotEmpty &&
        utf8.encode(textController.text).length <= maxTextBytes &&
        durationValue != null &&
        durationValue.numerator > BigInt.zero &&
        sizePoints != null &&
        sizePoints.isFinite &&
        sizePoints >= 4 &&
        sizePoints <= 256;
  }

  final canSave = ValueNotifier(fieldsAreValid());
  void updateValidity() => canSave.value = fieldsAreValid();
  textController.addListener(updateValidity);
  durationController.addListener(updateValidity);
  sizeController.addListener(updateValidity);
  final title = switch (contentKind) {
    ProjectTimelineClipContentKind.text =>
      text.isEmpty ? 'Add Title' : 'Edit Title',
    ProjectTimelineClipContentKind.caption =>
      text.isEmpty ? 'Add Manual Caption' : 'Edit Manual Caption',
    ProjectTimelineClipContentKind.media => '',
  };
  try {
    return await showDialog<TimelineTextClipEdit>(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (dialogContext, setDialogState) => AlertDialog(
          title: Text(title),
          content: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                TextField(
                  key: const ValueKey('timeline-text-content'),
                  controller: textController,
                  autofocus: true,
                  minLines: 2,
                  maxLines: 4,
                  decoration: InputDecoration(
                    labelText:
                        contentKind == ProjectTimelineClipContentKind.text
                        ? 'Title text'
                        : 'Caption text',
                    hintText: 'Enter text',
                    isDense: true,
                  ),
                ),
                ExactRationalField(
                  fieldKey: const ValueKey('timeline-text-duration'),
                  label: 'Duration (NUM/DEN seconds)',
                  controller: durationController,
                  onChanged: updateValidity,
                ),
                Padding(
                  padding: const EdgeInsets.only(top: OrSpacing.x2),
                  child: TextField(
                    key: const ValueKey('timeline-text-size'),
                    controller: sizeController,
                    keyboardType: const TextInputType.numberWithOptions(
                      decimal: true,
                    ),
                    decoration: const InputDecoration(
                      labelText: 'Font size (pt)',
                      isDense: true,
                    ),
                  ),
                ),
                const SizedBox(height: OrSpacing.x2),
                DropdownButtonFormField<ProjectFontIdentity>(
                  isExpanded: true,
                  key: const ValueKey('timeline-text-font'),
                  initialValue: font,
                  decoration: const InputDecoration(
                    labelText: 'Font',
                    isDense: true,
                  ),
                  items: const [
                    DropdownMenuItem(
                      value: ProjectFontIdentity.bundledInter,
                      child: Text('Inter'),
                    ),
                    DropdownMenuItem(
                      value: ProjectFontIdentity.beVietnamPro,
                      child: Text('Be Vietnam Pro'),
                    ),
                  ],
                  onChanged: (value) {
                    if (value == null) return;
                    setDialogState(() => font = value);
                  },
                ),
                const SizedBox(height: OrSpacing.x2),
                DropdownButtonFormField<ProjectTextWeight>(
                  isExpanded: true,
                  key: const ValueKey('timeline-text-weight'),
                  initialValue: weight,
                  decoration: const InputDecoration(
                    labelText: 'Weight',
                    isDense: true,
                  ),
                  items: const [
                    DropdownMenuItem(
                      value: ProjectTextWeight.regular,
                      child: Text('Regular'),
                    ),
                    DropdownMenuItem(
                      value: ProjectTextWeight.medium,
                      child: Text('Medium'),
                    ),
                    DropdownMenuItem(
                      value: ProjectTextWeight.semibold,
                      child: Text('Semibold'),
                    ),
                    DropdownMenuItem(
                      value: ProjectTextWeight.bold,
                      child: Text('Bold'),
                    ),
                  ],
                  onChanged: (value) {
                    if (value == null) return;
                    setDialogState(() => weight = value);
                    updateValidity();
                  },
                ),
                const SizedBox(height: OrSpacing.x2),
                DropdownButtonFormField<ProjectTextAlignment>(
                  isExpanded: true,
                  key: const ValueKey('timeline-text-alignment'),
                  initialValue: alignment,
                  decoration: const InputDecoration(
                    labelText: 'Alignment',
                    isDense: true,
                  ),
                  items: const [
                    DropdownMenuItem(
                      value: ProjectTextAlignment.start,
                      child: Text('Left'),
                    ),
                    DropdownMenuItem(
                      value: ProjectTextAlignment.center,
                      child: Text('Center'),
                    ),
                    DropdownMenuItem(
                      value: ProjectTextAlignment.end,
                      child: Text('Right'),
                    ),
                  ],
                  onChanged: (value) {
                    if (value == null) return;
                    setDialogState(() => alignment = value);
                    updateValidity();
                  },
                ),
                const SizedBox(height: OrSpacing.x2),
                DropdownButtonFormField<int>(
                  isExpanded: true,
                  key: const ValueKey('timeline-text-color'),
                  initialValue: colorIndex,
                  decoration: const InputDecoration(
                    labelText: 'Color',
                    isDense: true,
                  ),
                  items: [
                    const DropdownMenuItem(value: 0, child: Text('White')),
                    const DropdownMenuItem(value: 1, child: Text('Yellow')),
                    const DropdownMenuItem(value: 2, child: Text('Cyan')),
                    if (colors.length > 3)
                      const DropdownMenuItem(
                        value: 3,
                        child: Text('Current custom color'),
                      ),
                  ],
                  onChanged: (value) {
                    if (value == null) return;
                    setDialogState(() => colorIndex = value);
                    updateValidity();
                  },
                ),
                const SizedBox(height: OrSpacing.x2),
                DropdownButtonFormField<ProjectTextVerticalAnchor>(
                  isExpanded: true,
                  key: const ValueKey('timeline-text-anchor'),
                  initialValue: anchor,
                  decoration: const InputDecoration(
                    labelText: 'Position',
                    isDense: true,
                  ),
                  items: const [
                    DropdownMenuItem(
                      value: ProjectTextVerticalAnchor.top,
                      child: Text('Top'),
                    ),
                    DropdownMenuItem(
                      value: ProjectTextVerticalAnchor.center,
                      child: Text('Middle'),
                    ),
                    DropdownMenuItem(
                      value: ProjectTextVerticalAnchor.bottom,
                      child: Text('Bottom'),
                    ),
                  ],
                  onChanged: (value) {
                    if (value == null) return;
                    setDialogState(() => anchor = value);
                  },
                ),
                textStyleSlider(
                  keyName: 'timeline-text-offset-x',
                  label: 'Horizontal',
                  value: offsetX,
                  min: -100,
                  max: 100,
                  display: '${offsetX.round()}%',
                  onChanged: (value) => setDialogState(() => offsetX = value),
                ),
                textStyleSlider(
                  keyName: 'timeline-text-offset-y',
                  label: 'Vertical',
                  value: offsetY,
                  min: -100,
                  max: 100,
                  display: '${offsetY.round()}%',
                  onChanged: (value) => setDialogState(() => offsetY = value),
                ),
                textStyleSlider(
                  keyName: 'timeline-text-opacity',
                  label: 'Text opacity',
                  value: opacityPercent,
                  min: 0,
                  max: 100,
                  display: '${opacityPercent.round()}%',
                  onChanged: (value) =>
                      setDialogState(() => opacityPercent = value),
                ),
                SwitchListTile(
                  key: const ValueKey('timeline-text-background'),
                  contentPadding: EdgeInsets.zero,
                  dense: true,
                  title: const Text('Background box'),
                  value: backgroundEnabled,
                  onChanged: (value) =>
                      setDialogState(() => backgroundEnabled = value),
                ),
                if (backgroundEnabled)
                  textStyleSlider(
                    keyName: 'timeline-text-background-opacity',
                    label: 'Box opacity',
                    value: backgroundPercent,
                    min: 0,
                    max: 100,
                    display: '${backgroundPercent.round()}%',
                    onChanged: (value) =>
                        setDialogState(() => backgroundPercent = value),
                  ),
                const SizedBox(height: OrSpacing.x1),
                const Text(
                  'New clips start at the preview playhead.',
                  style: TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: OrType.tinySize,
                  ),
                ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: const Text('Cancel'),
            ),
            ValueListenableBuilder<bool>(
              valueListenable: canSave,
              builder: (context, valid, _) => FilledButton(
                key: const ValueKey('timeline-save-text'),
                onPressed: valid
                    ? () {
                        final parsedDuration = ProjectRationalTime.tryParse(
                          durationController.text,
                        );
                        final sizePoints = double.tryParse(
                          sizeController.text.trim(),
                        );
                        if (parsedDuration == null || sizePoints == null) {
                          return;
                        }
                        final content = ProjectTimelineTextContent(
                          kind: contentKind,
                          text: textController.text,
                          formatting: ProjectTextFormatting(
                            font: font,
                            sizeMilliPoints: (sizePoints * 1000).round(),
                            weight: weight,
                            alignment: alignment,
                            color: colors[colorIndex],
                            anchor: anchor,
                            offsetXBasisPoints: offsetX.round(),
                            offsetYBasisPoints: offsetY.round(),
                            opacityBasisPoints: opacityPercent.round() * 100,
                            background: backgroundEnabled
                                ? ProjectTextBackground(
                                    color: ProjectTextColor(
                                      red: 0,
                                      green: 0,
                                      blue: 0,
                                      alpha: (backgroundPercent * 2.55).round(),
                                    ),
                                  )
                                : null,
                          ),
                        );
                        Navigator.of(dialogContext).pop(
                          TimelineTextClipEdit(
                            content: content,
                            duration: parsedDuration,
                          ),
                        );
                      }
                    : null,
                child: Text(text.isEmpty ? 'Add' : 'Save'),
              ),
            ),
          ],
        ),
      ),
    );
  } finally {
    await Future<void>.delayed(const Duration(milliseconds: 250));
    textController.removeListener(updateValidity);
    durationController.removeListener(updateValidity);
    sizeController.removeListener(updateValidity);
    textController.dispose();
    durationController.dispose();
    sizeController.dispose();
    canSave.dispose();
  }
}

Future<void> showTrimTimelineClipDialog({
  required BuildContext context,
  required ProjectReadModel? project,
  required ProjectTimelineClip clip,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectTimelineTrimEdge,
    ProjectRationalTime,
    bool,
  )?
  onTrim,
}) async {
  if (project == null || onTrim == null) return;
  var selectedEdge = ProjectTimelineTrimEdge.start;
  // Presentation-local: the ripple choice travels with the command and is never
  // written into the project. It only means anything for an end edge, because a
  // ripple-in has two defensible meanings and the command refuses it.
  var ripple = false;
  var userEditedTimelineEdge = false;
  final timelineEdgeController = TextEditingController(
    text: clip.timelineStart.canonical,
  );
  bool edgeIsValid() {
    final target = ProjectRationalTime.tryParse(timelineEdgeController.text);
    if (target == null || target.numerator < BigInt.zero) return false;
    return selectedEdge == ProjectTimelineTrimEdge.start
        ? compareProjectRational(target, clip.timelineEnd) < 0
        : compareProjectRational(target, clip.timelineStart) > 0;
  }

  final canTrim = ValueNotifier(edgeIsValid());
  void updateTrimValidity() => canTrim.value = edgeIsValid();

  timelineEdgeController.addListener(updateTrimValidity);
  try {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (dialogContext, setDialogState) => AlertDialog(
          title: const Text('Trim Clip'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                'Current timing: ${clip.timelineStart.canonical} – ${clip.timelineEnd.canonical}',
                style: const TextStyle(color: OrColors.textSecondary),
              ),
              const SizedBox(height: OrSpacing.x2),
              DropdownButtonFormField<ProjectTimelineTrimEdge>(
                isExpanded: true,
                key: const ValueKey('timeline-trim-edge'),
                initialValue: selectedEdge,
                decoration: const InputDecoration(labelText: 'Edge'),
                items: const [
                  DropdownMenuItem(
                    value: ProjectTimelineTrimEdge.start,
                    child: Text('Start'),
                  ),
                  DropdownMenuItem(
                    value: ProjectTimelineTrimEdge.end,
                    child: Text('End'),
                  ),
                ],
                onChanged: (value) {
                  if (value == null) return;
                  setDialogState(() {
                    selectedEdge = value;
                    if (!userEditedTimelineEdge) {
                      timelineEdgeController.text =
                          value == ProjectTimelineTrimEdge.start
                          ? clip.timelineStart.canonical
                          : clip.timelineEnd.canonical;
                    }
                  });
                },
              ),
              CheckboxListTile(
                key: const ValueKey('timeline-trim-ripple'),
                value: ripple && selectedEdge == ProjectTimelineTrimEdge.end,
                onChanged: selectedEdge == ProjectTimelineTrimEdge.end
                    ? (value) => setDialogState(() => ripple = value ?? false)
                    : null,
                contentPadding: EdgeInsets.zero,
                dense: true,
                title: const Text('Ripple later clips'),
                subtitle: Text(
                  selectedEdge == ProjectTimelineTrimEdge.end
                      ? 'Move every later clip on this track by the same amount.'
                      : 'Only an end trim can ripple.',
                ),
              ),
              ExactRationalField(
                fieldKey: const ValueKey('timeline-trim-time'),
                label: 'Timeline edge',
                controller: timelineEdgeController,
                onChanged: () {
                  userEditedTimelineEdge = true;
                  setDialogState(() {});
                },
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: const Text('Cancel'),
            ),
            ValueListenableBuilder<bool>(
              valueListenable: canTrim,
              builder: (context, valid, child) => FilledButton(
                key: const ValueKey('timeline-confirm-trim'),
                onPressed: valid
                    ? () {
                        final target = ProjectRationalTime.tryParse(
                          timelineEdgeController.text,
                        );
                        if (target == null || !edgeIsValid()) return;
                        Navigator.of(dialogContext).pop();
                        unawaited(
                          onTrim(
                            project,
                            clip,
                            selectedEdge,
                            target,
                            ripple &&
                                selectedEdge == ProjectTimelineTrimEdge.end,
                          ),
                        );
                      }
                    : null,
                child: const Text('Trim'),
              ),
            ),
          ],
        ),
      ),
    );
  } finally {
    await Future<void>.delayed(const Duration(milliseconds: 250));
    timelineEdgeController.removeListener(updateTrimValidity);
    timelineEdgeController.dispose();
    canTrim.dispose();
  }
}

/// A signed delta in timeline units, which is all a roll and a slide ask for.
Future<void> showBoundaryMoveDialog({
  required BuildContext context,
  required ProjectReadModel? project,
  required ProjectTimelineClip clip,
  required String title,
  required String helper,
  required Key confirmKey,
  required Key fieldKey,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onConfirm,
}) async {
  if (project == null || onConfirm == null) return;
  final deltaController = TextEditingController(text: '1/4');
  bool deltaIsValid() {
    final delta = ProjectRationalTime.tryParse(deltaController.text);
    return delta != null && delta.numerator != BigInt.zero;
  }

  final canConfirm = ValueNotifier(deltaIsValid());
  void updateValidity() => canConfirm.value = deltaIsValid();
  deltaController.addListener(updateValidity);
  try {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (dialogContext, setDialogState) => AlertDialog(
          title: Text(title),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(helper),
              ExactRationalField(
                fieldKey: fieldKey,
                label: 'Delta',
                controller: deltaController,
                onChanged: () => setDialogState(() {}),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: const Text('Cancel'),
            ),
            ValueListenableBuilder<bool>(
              valueListenable: canConfirm,
              builder: (context, valid, child) => FilledButton(
                key: confirmKey,
                onPressed: valid
                    ? () {
                        final delta = ProjectRationalTime.tryParse(
                          deltaController.text,
                        );
                        if (delta == null) return;
                        Navigator.of(dialogContext).pop();
                        unawaited(onConfirm(project, clip, delta));
                      }
                    : null,
                child: const Text('Apply'),
              ),
            ),
          ],
        ),
      ),
    );
  } finally {
    await Future<void>.delayed(const Duration(milliseconds: 250));
    deltaController.removeListener(updateValidity);
    deltaController.dispose();
    canConfirm.dispose();
  }
}

Future<void> showSlipTimelineClipDialog({
  required BuildContext context,
  required ProjectReadModel? project,
  required ProjectTimelineClip clip,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onSlip,
}) async {
  if (project == null || onSlip == null) return;
  final deltaController = TextEditingController(text: '1/4');
  bool deltaIsValid() {
    final delta = ProjectRationalTime.tryParse(deltaController.text);
    return delta != null && delta.numerator != BigInt.zero;
  }

  final canSlip = ValueNotifier(deltaIsValid());
  void updateValidity() => canSlip.value = deltaIsValid();
  deltaController.addListener(updateValidity);
  try {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (dialogContext, setDialogState) => AlertDialog(
          title: const Text('Slip Clip'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text(
                'Move which part of the source this clip shows. The clip stays '
                'where it is, so no cut moves.',
              ),
              ExactRationalField(
                fieldKey: const ValueKey('timeline-slip-delta'),
                label: 'Source delta',
                controller: deltaController,
                onChanged: () => setDialogState(() {}),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: const Text('Cancel'),
            ),
            ValueListenableBuilder<bool>(
              valueListenable: canSlip,
              builder: (context, valid, child) => FilledButton(
                key: const ValueKey('timeline-confirm-slip'),
                onPressed: valid
                    ? () {
                        final delta = ProjectRationalTime.tryParse(
                          deltaController.text,
                        );
                        if (delta == null) return;
                        Navigator.of(dialogContext).pop();
                        unawaited(onSlip(project, clip, delta));
                      }
                    : null,
                child: const Text('Slip'),
              ),
            ),
          ],
        ),
      ),
    );
  } finally {
    await Future<void>.delayed(const Duration(milliseconds: 250));
    deltaController.removeListener(updateValidity);
    deltaController.dispose();
    canSlip.dispose();
  }
}

Future<void> showSplitTimelineClipDialog({
  required BuildContext context,
  required ProjectReadModel? project,
  required ProjectTimelineClip clip,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onSplit,
}) async {
  if (project == null || onSplit == null) return;
  final splitAtController = TextEditingController();
  bool splitIsValid() {
    final splitAt = ProjectRationalTime.tryParse(splitAtController.text);
    return splitAt != null &&
        splitAt.numerator >= BigInt.zero &&
        compareProjectRational(splitAt, clip.timelineStart) > 0 &&
        compareProjectRational(splitAt, clip.timelineEnd) < 0;
  }

  final canSplit = ValueNotifier(splitIsValid());
  void updateSplitValidity() => canSplit.value = splitIsValid();

  splitAtController.addListener(updateSplitValidity);
  try {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Split Clip'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'Current timing: ${clip.timelineStart.canonical} – ${clip.timelineEnd.canonical}',
              style: const TextStyle(color: OrColors.textSecondary),
            ),
            ExactRationalField(
              fieldKey: const ValueKey('timeline-split-time'),
              label: 'Split at:',
              controller: splitAtController,
              onChanged: () {},
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          ValueListenableBuilder<bool>(
            valueListenable: canSplit,
            builder: (context, valid, child) => FilledButton(
              key: const ValueKey('timeline-confirm-split'),
              onPressed: valid
                  ? () {
                      final splitAt = ProjectRationalTime.tryParse(
                        splitAtController.text,
                      );
                      if (splitAt == null || !splitIsValid()) return;
                      Navigator.of(dialogContext).pop();
                      unawaited(onSplit(project, clip, splitAt));
                    }
                  : null,
              child: const Text('Split'),
            ),
          ),
        ],
      ),
    );
  } finally {
    await Future<void>.delayed(const Duration(milliseconds: 250));
    splitAtController.removeListener(updateSplitValidity);
    splitAtController.dispose();
    canSplit.dispose();
  }
}

Future<void> showMoveTimelineClipDialog({
  required BuildContext context,
  required ProjectReadModel project,
  required List<ProjectTimelineTrack> tracks,
  required ProjectTimelineTrack sourceTrack,
  required ProjectTimelineClip clip,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    String,
    ProjectRationalTime,
  )
  onMove,
}) async {
  final targets = tracks
      .where(
        (candidate) =>
            candidate.kind == sourceTrack.kind && !candidate.state.locked,
      )
      .toList(growable: false);
  if (targets.isEmpty) return;
  final labels = timelineTrackLabels(tracks);
  var selectedTrackId = sourceTrack.trackId;
  final timeController = TextEditingController(
    text: clip.timelineStart.canonical,
  );
  bool timeIsValid() {
    final start = ProjectRationalTime.tryParse(timeController.text);
    return start != null && start.numerator >= BigInt.zero;
  }

  final canMove = ValueNotifier(timeIsValid());
  void updateMoveValidity() => canMove.value = timeIsValid();

  timeController.addListener(updateMoveValidity);
  try {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (dialogContext, setDialogState) {
          return AlertDialog(
            title: const Text('Move Clip'),
            content: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                DropdownButtonFormField<String>(
                  isExpanded: true,
                  key: const ValueKey('timeline-move-track'),
                  initialValue: selectedTrackId,
                  decoration: const InputDecoration(labelText: 'Target Track'),
                  items: [
                    for (final track in targets)
                      DropdownMenuItem(
                        value: track.trackId,
                        child: Text(labels[track.trackId]!),
                      ),
                  ],
                  onChanged: (value) => setDialogState(() {
                    selectedTrackId = value ?? selectedTrackId;
                  }),
                ),
                ExactRationalField(
                  fieldKey: const ValueKey('timeline-move-start'),
                  label: 'Timeline Start',
                  controller: timeController,
                  onChanged: () => setDialogState(() {}),
                ),
              ],
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(),
                child: const Text('Cancel'),
              ),
              ValueListenableBuilder<bool>(
                valueListenable: canMove,
                builder: (context, valid, child) {
                  return FilledButton(
                    key: const ValueKey('timeline-confirm-move'),
                    onPressed: valid
                        ? () {
                            final submittedStart = ProjectRationalTime.tryParse(
                              timeController.text,
                            );
                            if (submittedStart == null ||
                                submittedStart.numerator < BigInt.zero) {
                              return;
                            }
                            Navigator.of(dialogContext).pop();
                            unawaited(
                              onMove(
                                project,
                                clip,
                                selectedTrackId,
                                submittedStart,
                              ),
                            );
                          }
                        : null,
                    child: const Text('Move'),
                  );
                },
              ),
            ],
          );
        },
      ),
    );
  } finally {
    await Future<void>.delayed(const Duration(milliseconds: 250));
    timeController.removeListener(updateMoveValidity);
    timeController.dispose();
    canMove.dispose();
  }
}

Future<void> confirmDeleteTimelineClip({
  required BuildContext context,
  required ProjectReadModel project,
  required ProjectTimelineClip clip,
  required Future<void> Function(ProjectReadModel, ProjectTimelineClip)
  onDelete,
}) async {
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (dialogContext) => AlertDialog(
      title: const Text('Delete clip?'),
      content: const Text(
        'Later clips and other tracks stay in place. The media remains in the project.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(dialogContext).pop(false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: ValueKey('timeline-confirm-delete-${clip.clipId}'),
          onPressed: () => Navigator.of(dialogContext).pop(true),
          child: const Text('Delete'),
        ),
      ],
    ),
  );
  if (confirmed == true) await onDelete(project, clip);
}

Future<void> confirmRippleDeleteTimelineClip({
  required BuildContext context,
  required ProjectReadModel? project,
  required ProjectTimelineClip clip,
  required Future<void> Function(ProjectReadModel, ProjectTimelineClip)?
  onRippleDelete,
}) async {
  if (project == null || onRippleDelete == null) return;
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (dialogContext) => AlertDialog(
      title: const Text('Ripple delete clip?'),
      content: const Text(
        'Delete this clip and shift later clips on this track left by its duration? Other tracks will not move.',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(dialogContext).pop(false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: ValueKey('timeline-confirm-ripple-delete-${clip.clipId}'),
          onPressed: () => Navigator.of(dialogContext).pop(true),
          child: const Text('Ripple Delete'),
        ),
      ],
    ),
  );
  if (confirmed == true) await onRippleDelete(project, clip);
}
