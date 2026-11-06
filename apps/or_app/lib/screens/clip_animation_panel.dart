import 'dart:async';

import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../project/project_gateway.dart';

/// Keyframes for the selected clip, edited at the playhead.
///
/// The panel is deliberately a small table rather than a graph: adding the
/// value the fields already hold at the playhead, then moving the playhead and
/// adding another, is the whole beginner workflow. The easing list names what
/// it does, and every keyframe can be edited or removed in place.
class ClipAnimationPanel extends StatefulWidget {
  const ClipAnimationPanel({
    super.key,
    required this.gateway,
    required this.session,
    required this.project,
    required this.trackId,
    required this.clipId,
    required this.clipStart,
    required this.clipDuration,
    this.playhead,
    this.staticSettings,
    this.enabled = true,
  });

  final ProjectGateway gateway;
  final ProjectSessionHandle session;
  final ProjectReadModel project;
  final String trackId;
  final String clipId;
  final ProjectRationalTime clipStart;
  final ProjectRationalTime clipDuration;
  final ProjectRationalTime? playhead;
  final ProjectTimelineVisualSettings? staticSettings;
  final bool enabled;

  @override
  State<ClipAnimationPanel> createState() => _ClipAnimationPanelState();
}

class _ClipAnimationPanelState extends State<ClipAnimationPanel> {
  ProjectClipAnimation _animation = ProjectClipAnimation.empty;
  bool _loading = false;
  bool _saving = false;
  String? _error;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(covariant ClipAnimationPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.clipId != widget.clipId ||
        oldWidget.trackId != widget.trackId ||
        oldWidget.project.projectId != widget.project.projectId ||
        oldWidget.project.projectInstanceId !=
            widget.project.projectInstanceId ||
        oldWidget.project.revision != widget.project.revision) {
      _load();
    }
  }

  @override
  void dispose() {
    _generation++;
    super.dispose();
  }

  Future<void> _load() async {
    final generation = ++_generation;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final animation = await widget.gateway.getTimelineClipAnimation(
        widget.session,
        widget.project,
        trackId: widget.trackId,
        clipId: widget.clipId,
      );
      if (!mounted || generation != _generation) return;
      setState(() {
        _animation = animation;
        _loading = false;
      });
    } on ProjectGatewayException catch (error) {
      if (!mounted || generation != _generation) return;
      setState(() {
        _error = error.message;
        _loading = false;
      });
    }
  }

  Future<void> _save(ProjectClipAnimation next) async {
    if (_saving) return;
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      final result = await widget.gateway.updateTimelineClipAnimation(
        widget.session,
        widget.project,
        trackId: widget.trackId,
        clipId: widget.clipId,
        animation: next,
      );
      if (!mounted) return;
      if (!result.succeeded) {
        setState(() {
          _saving = false;
          _error = result.message.isEmpty
              ? 'The keyframes were not saved.'
              : result.message;
        });
        return;
      }
      setState(() {
        _animation = next;
        _saving = false;
      });
    } on ProjectGatewayException catch (error) {
      if (!mounted) return;
      setState(() {
        _saving = false;
        _error = error.message;
      });
    }
  }

  double get _clipSeconds => widget.clipDuration.secondsForDisplay;

  double get _elapsedSeconds {
    final playhead = widget.playhead;
    if (playhead == null) return 0;
    final elapsed =
        playhead.secondsForDisplay - widget.clipStart.secondsForDisplay;
    return elapsed.clamp(0, _clipSeconds);
  }

  ProjectRationalTime _timeAt(double seconds) =>
      ProjectRationalTime(BigInt.from((seconds * 1000).round()), 1000);

  void _addKeyframe(_AnimationChannel channel) {
    final elapsed = _timeAt(_elapsedSeconds);
    final existing = channel
        .read(_animation)
        .where(
          (keyframe) =>
              (keyframe.at.secondsForDisplay - elapsed.secondsForDisplay)
                  .abs() <
              0.002,
        )
        .toList();
    if (existing.isNotEmpty) {
      // A keyframe already sits on this frame: edit it instead of adding a
      // second one at the same time, which the model rejects.
      unawaited(_editKeyframe(channel, existing.first));
      return;
    }
    final value = channel.valueFromStatic(widget.staticSettings);
    final next = channel.write(
      _animation,
      [...channel.read(_animation), ProjectKeyframe(at: elapsed, value: value)]
        ..sort((a, b) => a.at.compareTo(b.at)),
    );
    unawaited(_save(next));
  }

  void _removeKeyframe(_AnimationChannel channel, ProjectKeyframe keyframe) {
    final next = channel.write(
      _animation,
      channel
          .read(_animation)
          .where((candidate) => !identical(candidate, keyframe))
          .toList(),
    );
    unawaited(_save(next));
  }

  Future<void> _applyPreset(_MotionPreset preset) async {
    final span = widget.clipDuration.secondsForDisplay.clamp(0.05, 0.7);
    await _save(preset.build(_animation, span));
  }

  @override
  Widget build(BuildContext context) {
    final channels = _channelDefinitions;
    final hasKeyframes = !_animation.isEmpty;
    return Column(
      key: const ValueKey('animation-section'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                'ANIMATION',
                style: TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            if (hasKeyframes)
              TextButton(
                key: const ValueKey('animation-clear'),
                onPressed: widget.enabled && !_saving
                    ? () => unawaited(_save(ProjectClipAnimation.empty))
                    : null,
                child: const Text('Clear'),
              ),
          ],
        ),
        Text(
          _loading
              ? 'Reading the keyframes on this clip…'
              : hasKeyframes
              ? 'Keyframes override the values above for the frames they cover. '
                    'Add another at a different playhead position to animate.'
              : 'Add a keyframe at the playhead to start animating this clip. '
                    'Move the playhead, change the value above, then add again.',
          style: TextStyle(color: OrColors.textMuted, fontSize: OrType.tinySize),
        ),
        const SizedBox(height: OrSpacing.x1),
        Row(
          children: [
            Expanded(
              child: Text(
                'MOTION PRESETS',
                style: TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            TextButton(
              key: const ValueKey('animation-copy'),
              onPressed: _animation.isEmpty
                  ? null
                  : () {
                      ProjectClipAnimation.copied = _animation;
                      setState(() {});
                    },
              child: const Text('Copy'),
            ),
            TextButton(
              key: const ValueKey('animation-paste'),
              onPressed:
                  widget.enabled &&
                      !_saving &&
                      ProjectClipAnimation.copied != null
                  ? () => unawaited(
                      _save(
                        _animation.pasteChannelsFrom(
                          ProjectClipAnimation.copied!,
                        ),
                      ),
                    )
                  : null,
              child: const Text('Paste'),
            ),
          ],
        ),
        const SizedBox(height: OrSpacing.x1),
        Wrap(
          spacing: OrSpacing.x1,
          runSpacing: OrSpacing.x1,
          children: [
            for (final preset in _motionPresets)
              ActionChip(
                key: ValueKey('animation-preset-${preset.wire}'),
                label: Text(preset.label),
                onPressed: widget.enabled && !_saving
                    ? () => unawaited(_applyPreset(preset))
                    : null,
              ),
          ],
        ),
        Text(
          'A preset replaces the keyframes on the channels it animates.',
          style: TextStyle(color: OrColors.textMuted, fontSize: OrType.tinySize),
        ),
        if (_error != null) ...[
          const SizedBox(height: OrSpacing.x1),
          Text(
            _error!,
            key: const ValueKey('animation-error'),
            style: TextStyle(color: OrColors.danger, fontSize: OrType.tinySize),
          ),
        ],
        const SizedBox(height: OrSpacing.x1),
        for (final channel in channels) ...[
          _channelRow(channel),
          const SizedBox(height: OrSpacing.x1),
        ],
      ],
    );
  }

  Widget _channelRow(_AnimationChannel channel) {
    final keyframes = channel.read(_animation);
    final canAdd = widget.enabled && !_saving;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                channel.label,
                style: TextStyle(
                  color: OrColors.text,
                  fontSize: OrType.metadataSize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            TextButton(
              key: ValueKey('animation-add-${channel.wire}'),
              onPressed: canAdd ? () => _addKeyframe(channel) : null,
              child: Text(
                keyframes.isEmpty
                    ? 'Add at ${_secondsLabel(_elapsedSeconds)}'
                    : 'Add',
              ),
            ),
          ],
        ),
        if (keyframes.isNotEmpty)
          Wrap(
            spacing: OrSpacing.x1,
            runSpacing: OrSpacing.x1,
            children: [
              for (final (index, keyframe) in keyframes.indexed)
                InputChip(
                  key: ValueKey('animation-keyframe-${channel.wire}-$index'),
                  label: Text(
                    '${_secondsLabel(keyframe.at.secondsForDisplay)} · '
                    '${channel.display(keyframe.value)} · '
                    '${_easingLabel(keyframe.easing)}',
                  ),
                  onPressed: widget.enabled && !_saving
                      ? () => _editKeyframe(channel, keyframe)
                      : null,
                  onDeleted: widget.enabled && !_saving
                      ? () => _removeKeyframe(channel, keyframe)
                      : null,
                ),
            ],
          ),
      ],
    );
  }

  Future<void> _editKeyframe(
    _AnimationChannel channel,
    ProjectKeyframe keyframe,
  ) async {
    final result = await showDialog<_KeyframeEdit>(
      context: context,
      builder: (context) => _KeyframeDialog(
        channel: channel,
        keyframe: keyframe,
        clipSeconds: _clipSeconds,
      ),
    );
    if (result == null || !mounted) return;
    if (result.delete) {
      _removeKeyframe(channel, keyframe);
      return;
    }
    final edited = keyframe.copyWith(
      at: result.at,
      value: result.value,
      easing: result.easing,
    );
    final next = channel.write(_animation, [
      for (final candidate in channel.read(_animation))
        if (identical(candidate, keyframe)) edited else candidate,
    ]..sort((a, b) => a.at.compareTo(b.at)));
    unawaited(_save(next));
  }
}

class _KeyframeEdit {
  const _KeyframeEdit({
    required this.at,
    required this.value,
    required this.easing,
    this.delete = false,
  });

  final ProjectRationalTime at;
  final int value;
  final ProjectKeyframeEasing easing;
  final bool delete;
}

class _KeyframeDialog extends StatefulWidget {
  const _KeyframeDialog({
    required this.channel,
    required this.keyframe,
    required this.clipSeconds,
  });

  final _AnimationChannel channel;
  final ProjectKeyframe keyframe;
  final double clipSeconds;

  @override
  State<_KeyframeDialog> createState() => _KeyframeDialogState();
}

class _KeyframeDialogState extends State<_KeyframeDialog> {
  late final TextEditingController _time = TextEditingController(
    text: widget.keyframe.at.secondsForDisplay.toStringAsFixed(3),
  );
  late final TextEditingController _value = TextEditingController(
    text: widget.channel
        .toHuman(widget.keyframe.value)
        .toStringAsFixed(widget.channel.decimals),
  );
  late ProjectKeyframeEasing _easing = widget.keyframe.easing;
  String? _error;

  @override
  void dispose() {
    _time.dispose();
    _value.dispose();
    super.dispose();
  }

  void _submit() {
    final seconds = double.tryParse(_time.text.trim());
    final human = double.tryParse(_value.text.trim());
    if (seconds == null || seconds < 0 || seconds > widget.clipSeconds) {
      setState(() => _error = 'Time must be between 0 and the clip end.');
      return;
    }
    final value = human == null ? null : widget.channel.fromHuman(human);
    if (value == null ||
        value < widget.channel.minimum ||
        value > widget.channel.maximum) {
      setState(
        () => _error =
            '${widget.channel.label} must be ${widget.channel.humanRange}.',
      );
      return;
    }
    Navigator.of(context).pop(
      _KeyframeEdit(
        at: ProjectRationalTime(BigInt.from((seconds * 1000).round()), 1000),
        value: value,
        easing: _easing,
      ),
    );
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text('${widget.channel.label} keyframe'),
    content: Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        TextField(
          key: const ValueKey('animation-dialog-time'),
          controller: _time,
          decoration: const InputDecoration(labelText: 'Seconds from clip start'),
          keyboardType: const TextInputType.numberWithOptions(decimal: true),
        ),
        const SizedBox(height: OrSpacing.x1),
        TextField(
          key: const ValueKey('animation-dialog-value'),
          controller: _value,
          decoration: InputDecoration(
            labelText: '${widget.channel.label} (${widget.channel.unit})',
          ),
          keyboardType: const TextInputType.numberWithOptions(
            signed: true,
            decimal: true,
          ),
        ),
        const SizedBox(height: OrSpacing.x1),
        DropdownButtonFormField<ProjectKeyframeEasing>(
          key: const ValueKey('animation-dialog-easing'),
          initialValue: _easing,
          items: [
            for (final easing in ProjectKeyframeEasing.values)
              DropdownMenuItem(
                value: easing,
                child: Text(_easingLabel(easing)),
              ),
          ],
          onChanged: (value) => setState(() => _easing = value ?? _easing),
        ),
        if (_error != null) ...[
          const SizedBox(height: OrSpacing.x1),
          Text(
            _error!,
            style: TextStyle(color: OrColors.danger, fontSize: OrType.tinySize),
          ),
        ],
      ],
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.of(context).pop(
          _KeyframeEdit(
            at: widget.keyframe.at,
            value: widget.keyframe.value,
            easing: widget.keyframe.easing,
            delete: true,
          ),
        ),
        child: const Text('Remove'),
      ),
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: const Text('Cancel'),
      ),
      FilledButton(
        onPressed: _submit,
        child: const Text('Save'),
      ),
    ],
  );
}

ProjectRationalTime _secondsTime(double seconds) =>
    ProjectRationalTime(BigInt.from((seconds * 1000).round()), 1000);

/// One-tap animation built from ordinary keyframes.
///
/// A preset is only a starting point: it writes the same command the manual
/// controls use, so the chips can be edited or cleared afterwards and the
/// project file stores keyframes, not a preset name.
class _MotionPreset {
  const _MotionPreset({
    required this.wire,
    required this.label,
    required this.build,
  });

  final String wire;
  final String label;
  final ProjectClipAnimation Function(ProjectClipAnimation current, double span)
  build;
}

ProjectKeyframe _keyframe(double at, int value, ProjectKeyframeEasing easing) =>
    ProjectKeyframe(at: _secondsTime(at), value: value, easing: easing);

const List<_MotionPreset> _motionPresets = [
  _MotionPreset(
    wire: 'fade-in',
    label: 'Fade in',
    build: _fadeIn,
  ),
  _MotionPreset(
    wire: 'slide-up',
    label: 'Slide up',
    build: _slideUp,
  ),
  _MotionPreset(
    wire: 'zoom-in',
    label: 'Zoom in',
    build: _zoomIn,
  ),
  _MotionPreset(
    wire: 'pop',
    label: 'Pop',
    build: _pop,
  ),
];

ProjectClipAnimation _fadeIn(ProjectClipAnimation current, double span) =>
    current.copyWith(
      opacityBasisPoints: [
        _keyframe(0, 0, ProjectKeyframeEasing.easeOut),
        _keyframe(span, 10000, ProjectKeyframeEasing.linear),
      ],
    );

ProjectClipAnimation _slideUp(ProjectClipAnimation current, double span) =>
    current.copyWith(
      yMilliCanvas: [
        _keyframe(0, 120, ProjectKeyframeEasing.easeOut),
        _keyframe(span, 0, ProjectKeyframeEasing.linear),
      ],
    );

ProjectClipAnimation _zoomIn(ProjectClipAnimation current, double span) =>
    current.copyWith(
      scaleXMilli: [
        _keyframe(0, 850, ProjectKeyframeEasing.easeOut),
        _keyframe(span, 1000, ProjectKeyframeEasing.linear),
      ],
      scaleYMilli: [
        _keyframe(0, 850, ProjectKeyframeEasing.easeOut),
        _keyframe(span, 1000, ProjectKeyframeEasing.linear),
      ],
    );

ProjectClipAnimation _pop(ProjectClipAnimation current, double span) => current
    .copyWith(
      scaleXMilli: [
        _keyframe(0, 1000, ProjectKeyframeEasing.easeOut),
        _keyframe(span / 2, 1120, ProjectKeyframeEasing.easeInOut),
        _keyframe(span, 1000, ProjectKeyframeEasing.linear),
      ],
      scaleYMilli: [
        _keyframe(0, 1000, ProjectKeyframeEasing.easeOut),
        _keyframe(span / 2, 1120, ProjectKeyframeEasing.easeInOut),
        _keyframe(span, 1000, ProjectKeyframeEasing.linear),
      ],
    );

class _AnimationChannel {
  const _AnimationChannel({
    required this.wire,
    required this.label,
    required this.unit,
    required this.decimals,
    required this.minimum,
    required this.maximum,
    required this.read,
    required this.write,
    required this.toHuman,
    required this.fromHuman,
    required this.staticValue,
  });

  final String wire;
  final String label;
  final String unit;
  final int decimals;
  final int minimum;
  final int maximum;
  final List<ProjectKeyframe> Function(ProjectClipAnimation) read;
  final ProjectClipAnimation Function(
    ProjectClipAnimation,
    List<ProjectKeyframe>,
  )
  write;
  final double Function(int) toHuman;
  final int? Function(double) fromHuman;
  final int Function(ProjectTimelineVisualSettings?) staticValue;

  int valueFromStatic(ProjectTimelineVisualSettings? settings) =>
      staticValue(settings);

  String display(int value) =>
      '${toHuman(value).toStringAsFixed(decimals)}$unit';

  String get humanRange =>
      '${toHuman(minimum).toStringAsFixed(decimals)}$unit to '
      '${toHuman(maximum).toStringAsFixed(decimals)}$unit';
}

String _easingLabel(ProjectKeyframeEasing easing) => switch (easing) {
  ProjectKeyframeEasing.linear => 'Linear',
  ProjectKeyframeEasing.easeIn => 'Ease in',
  ProjectKeyframeEasing.easeOut => 'Ease out',
  ProjectKeyframeEasing.easeInOut => 'Ease in and out',
};

String _secondsLabel(double seconds) => '${seconds.toStringAsFixed(2)}s';

double _percentHuman(int basisPoints) => basisPoints / 100;
int? _percentCanonical(double value) => (value * 100).round();
double _positionHuman(int milliCanvas) => milliCanvas / 1000;
int? _positionCanonical(double value) => (value * 1000).round();
double _scaleHuman(int milli) => milli / 1000;
int? _scaleCanonical(double value) => (value * 1000).round();
double _rotationHuman(int milliDegrees) => milliDegrees / 1000;
int? _rotationCanonical(double value) => (value * 1000).round();

const List<_AnimationChannel> _channelDefinitions = [
  _AnimationChannel(
    wire: 'opacity',
    label: 'Opacity',
    unit: '%',
    decimals: 1,
    minimum: 0,
    maximum: 10000,
    read: _opacityKeyframes,
    write: _withOpacityKeyframes,
    toHuman: _percentHuman,
    fromHuman: _percentCanonical,
    staticValue: _staticOpacity,
  ),
  _AnimationChannel(
    wire: 'x',
    label: 'Position X',
    unit: '',
    decimals: 2,
    minimum: -100000,
    maximum: 100000,
    read: _xKeyframes,
    write: _withXKeyframes,
    toHuman: _positionHuman,
    fromHuman: _positionCanonical,
    staticValue: _staticX,
  ),
  _AnimationChannel(
    wire: 'y',
    label: 'Position Y',
    unit: '',
    decimals: 2,
    minimum: -100000,
    maximum: 100000,
    read: _yKeyframes,
    write: _withYKeyframes,
    toHuman: _positionHuman,
    fromHuman: _positionCanonical,
    staticValue: _staticY,
  ),
  _AnimationChannel(
    wire: 'scale_x',
    label: 'Scale X',
    unit: '×',
    decimals: 3,
    minimum: 1,
    maximum: 100000,
    read: _scaleXKeyframes,
    write: _withScaleXKeyframes,
    toHuman: _scaleHuman,
    fromHuman: _scaleCanonical,
    staticValue: _staticScaleX,
  ),
  _AnimationChannel(
    wire: 'scale_y',
    label: 'Scale Y',
    unit: '×',
    decimals: 3,
    minimum: 1,
    maximum: 100000,
    read: _scaleYKeyframes,
    write: _withScaleYKeyframes,
    toHuman: _scaleHuman,
    fromHuman: _scaleCanonical,
    staticValue: _staticScaleY,
  ),
  _AnimationChannel(
    wire: 'rotation',
    label: 'Rotation',
    unit: '°',
    decimals: 1,
    minimum: -360000,
    maximum: 360000,
    read: _rotationKeyframes,
    write: _withRotationKeyframes,
    toHuman: _rotationHuman,
    fromHuman: _rotationCanonical,
    staticValue: _staticRotation,
  ),
];

List<ProjectKeyframe> _opacityKeyframes(ProjectClipAnimation a) =>
    a.opacityBasisPoints;
List<ProjectKeyframe> _xKeyframes(ProjectClipAnimation a) => a.xMilliCanvas;
List<ProjectKeyframe> _yKeyframes(ProjectClipAnimation a) => a.yMilliCanvas;
List<ProjectKeyframe> _scaleXKeyframes(ProjectClipAnimation a) => a.scaleXMilli;
List<ProjectKeyframe> _scaleYKeyframes(ProjectClipAnimation a) => a.scaleYMilli;
List<ProjectKeyframe> _rotationKeyframes(ProjectClipAnimation a) =>
    a.rotationMilliDegrees;

ProjectClipAnimation _withOpacityKeyframes(
  ProjectClipAnimation a,
  List<ProjectKeyframe> k,
) => a.copyWith(opacityBasisPoints: k);
ProjectClipAnimation _withXKeyframes(
  ProjectClipAnimation a,
  List<ProjectKeyframe> k,
) => a.copyWith(xMilliCanvas: k);
ProjectClipAnimation _withYKeyframes(
  ProjectClipAnimation a,
  List<ProjectKeyframe> k,
) => a.copyWith(yMilliCanvas: k);
ProjectClipAnimation _withScaleXKeyframes(
  ProjectClipAnimation a,
  List<ProjectKeyframe> k,
) => a.copyWith(scaleXMilli: k);
ProjectClipAnimation _withScaleYKeyframes(
  ProjectClipAnimation a,
  List<ProjectKeyframe> k,
) => a.copyWith(scaleYMilli: k);
ProjectClipAnimation _withRotationKeyframes(
  ProjectClipAnimation a,
  List<ProjectKeyframe> k,
) => a.copyWith(rotationMilliDegrees: k);

int _staticOpacity(ProjectTimelineVisualSettings? s) =>
    s?.opacityBasisPoints ?? 10000;
int _staticX(ProjectTimelineVisualSettings? s) => s?.xMilliCanvas ?? 0;
int _staticY(ProjectTimelineVisualSettings? s) => s?.yMilliCanvas ?? 0;
int _staticScaleX(ProjectTimelineVisualSettings? s) => s?.scaleXMilli ?? 1000;
int _staticScaleY(ProjectTimelineVisualSettings? s) => s?.scaleYMilli ?? 1000;
int _staticRotation(ProjectTimelineVisualSettings? s) =>
    s?.rotationMilliDegrees ?? 0;
