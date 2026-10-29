import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../project/project_gateway.dart';
import '../widgets/or_widgets.dart';
import 'editor_frame_rate.dart';
import 'editor_panel_header.dart';
import 'inspector_audio_validation.dart';

/// Human-facing unit conversion for the inspector. Canonical state stays in the
/// engine's milli / basis-point integers; these helpers are the only place the
/// UI translates between the two, so a field never exposes `1/4` or `(bp)`.
String _plainNumber(double value, {int decimals = 3}) {
  if (!value.isFinite) return '0';
  var fixed = value.toStringAsFixed(decimals);
  if (fixed.contains('.')) {
    fixed = fixed.replaceFirst(RegExp(r'0+$'), '');
    fixed = fixed.replaceFirst(RegExp(r'\.$'), '');
  }
  return fixed.isEmpty ? '0' : fixed;
}

String _percentLabel(int milli) => _plainNumber(milli / 10);
String _basisPercentLabel(int basisPoints) => _plainNumber(basisPoints / 100);
String _degreesLabel(int milliDegrees) => _plainNumber(milliDegrees / 1000);
String _pixelsLabel(int milliPixels) => _plainNumber(milliPixels / 1000);
String _secondsLabel(ProjectRationalTime time) =>
    _plainNumber(time.numerator.toDouble() / time.denominator);

double? _finiteNumber(String text) {
  final value = double.tryParse(text.trim());
  return value != null && value.isFinite ? value : null;
}

int? _percentToMilli(
  String text, {
  int minimum = -100000,
  int maximum = 100000,
}) {
  final value = _finiteNumber(text);
  if (value == null) return null;
  final milli = (value * 10).round();
  return milli >= minimum && milli <= maximum ? milli : null;
}

int? _basisPercentToBasisPoints(String text) {
  final value = _finiteNumber(text);
  if (value == null) return null;
  final points = (value * 100).round();
  return points >= 0 && points <= 10000 ? points : null;
}

int? _degreesToMilliDegrees(String text) {
  final value = _finiteNumber(text);
  if (value == null) return null;
  final milli = (value * 1000).round();
  return milli >= -360000 && milli <= 360000 ? milli : null;
}

int? _pixelsToMilliPixels(String text) {
  final value = _finiteNumber(text);
  if (value == null || value < 0) return null;
  final milli = (value * 1000).round();
  return milli <= 128000 ? milli : null;
}

class EditorInspectorPanel extends StatefulWidget {
  const EditorInspectorPanel({
    super.key,
    required this.isProjectWorkspace,
    required this.project,
    required this.gateway,
    required this.session,
    required this.trackId,
    required this.clipId,
    this.clip,
    required this.isVisualTrack,
    required this.isAudioTrack,
    required this.trackLocked,
    required this.busy,
    this.sequenceFrameRate,
    this.contentEnd,
    this.trackCount = 0,
    this.clipCount = 0,
    this.onSetSequenceFrameRate,
    required this.onUpdate,
    required this.onUpdateAudio,
    this.onUpdatePlaybackRate,
    this.onSetReversed,
    this.onSetFrozen,
    this.onClose,
    this.showHeader = true,
  });

  final bool showHeader;
  final bool isProjectWorkspace;
  final ProjectReadModel? project;
  final ProjectGateway? gateway;
  final ProjectSessionHandle? session;
  final String? trackId;
  final String? clipId;
  final ProjectTimelineClip? clip;
  final bool isVisualTrack;
  final bool isAudioTrack;
  final bool trackLocked;
  final bool busy;

  /// Sequence facts the inspector states when no clip is selected, so the
  /// right panel always answers a question instead of going blank.
  final ProjectRationalRate? sequenceFrameRate;
  final ProjectRationalTime? contentEnd;
  final ValueChanged<ProjectRationalRate?>? onSetSequenceFrameRate;
  final int trackCount;
  final int clipCount;
  final Future<ProjectReadModel?> Function(
    ProjectReadModel,
    String,
    String,
    ProjectTimelineVisualSettings,
  )?
  onUpdate;
  final Future<ProjectReadModel?> Function(
    ProjectReadModel,
    String,
    String,
    ProjectTimelineAudioSettings,
  )?
  onUpdateAudio;
  final Future<ProjectReadModel?> Function(
    ProjectReadModel,
    String,
    String,
    ProjectRationalRate,
  )?
  onUpdatePlaybackRate;
  final Future<ProjectReadModel?> Function(
    ProjectReadModel expected,
    String trackId,
    String clipId,
    bool reversed,
  )?
  onSetReversed;
  final Future<ProjectReadModel?> Function(
    ProjectReadModel expected,
    String trackId,
    String clipId,
    bool frozen,
  )?
  onSetFrozen;
  final VoidCallback? onClose;

  @override
  State<EditorInspectorPanel> createState() => _EditorInspectorPanelState();
}

class _EditorInspectorPanelState extends State<EditorInspectorPanel> {
  final Map<String, TextEditingController> _fields = {
    for (final key in [..._visualFieldKeys, ..._visualTimeFieldKeys])
      key: TextEditingController(),
  };
  final Map<String, TextEditingController> _audioFields = {
    for (final key in _audioFieldKeys) key: TextEditingController(),
  };
  bool _loading = false;
  bool _saving = false;
  String? _error;
  int _loadGeneration = 0;
  final Set<ProjectTimelineEffectKind> _modifiedEffects = {};
  bool _updateTransitionIn = false;
  bool _updateTransitionOut = false;

  @override
  void initState() {
    super.initState();
    _loadSettings();
  }

  @override
  void didUpdateWidget(covariant EditorInspectorPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.project?.projectId != widget.project?.projectId ||
        oldWidget.project?.projectInstanceId !=
            widget.project?.projectInstanceId ||
        oldWidget.project?.revision != widget.project?.revision ||
        oldWidget.trackId != widget.trackId ||
        oldWidget.clipId != widget.clipId ||
        oldWidget.isVisualTrack != widget.isVisualTrack ||
        oldWidget.isAudioTrack != widget.isAudioTrack) {
      _loadSettings();
    }
  }

  @override
  void dispose() {
    _loadGeneration++;
    for (final field in _fields.values) {
      field.dispose();
    }
    for (final field in _audioFields.values) {
      field.dispose();
    }
    super.dispose();
  }

  Widget _presetRow({
    required String keyPrefix,
    required List<(String, String)> presets,
    required void Function(String preset) onPick,
  }) => Wrap(
    spacing: OrSpacing.x1,
    runSpacing: OrSpacing.x1,
    children: [
      for (final (id, label) in presets)
        ActionChip(
          key: ValueKey('$keyPrefix-$id'),
          label: Text(
            label,
            style: const TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.tinySize,
            ),
          ),
          visualDensity: VisualDensity.compact,
          materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
          backgroundColor: OrColors.backgroundRaised,
          side: const BorderSide(color: OrColors.border),
          onPressed: () => onPick(id),
        ),
    ],
  );

  /// Presets fill the numeric fields; the apply step stays explicit so a
  /// preset is reviewable before it becomes a command. A second "apply preset"
  /// command path would hide the value that was written.
  void _setFieldText(String name, String value) {
    final field = _fields[name];
    if (field == null) return;
    field.text = value;
  }

  void _applyEffectPreset(String preset) {
    switch (preset) {
      case 'none':
        _setFieldText('brightness', '0');
        _setFieldText('contrast', '0');
        _setFieldText('saturation', '0');
        _setFieldText('blur', '0');
      case 'bright':
        _setFieldText('brightness', '10');
      case 'punch':
        _setFieldText('contrast', '20');
        _setFieldText('saturation', '15');
      case 'muted':
        _setFieldText('saturation', '-30');
      case 'soft':
        _setFieldText('blur', '2');
    }
    setState(() {});
  }

  void _applyTransitionPreset(String preset) {
    final kind = switch (preset) {
      'black' => ProjectTimelineTransition.fadeThroughBlack,
      'wipe' => ProjectTimelineTransition.wipe,
      _ => ProjectTimelineTransition.crossDissolve,
    };
    _setFieldText('transition_in_kind', '${kind.index}');
    _setFieldText('transition_in_duration', preset == 'none' ? '0' : '0.5');
    setState(() {});
  }

  Future<void> _loadSettings() async {
    final generation = ++_loadGeneration;
    final project = widget.project;
    final gateway = widget.gateway;
    final session = widget.session;
    final trackId = widget.trackId;
    final clipId = widget.clipId;
    if (!widget.isProjectWorkspace ||
        (!widget.isVisualTrack && !widget.isAudioTrack) ||
        project == null ||
        gateway == null ||
        session == null ||
        trackId == null ||
        clipId == null) {
      setState(() {
        _loading = false;
        _error = null;
      });
      return;
    }
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      if (widget.isAudioTrack) {
        final settings = await gateway.getTimelineClipAudioSettings(
          session,
          project,
          trackId: trackId,
          clipId: clipId,
        );
        if (!mounted || generation != _loadGeneration) return;
        _writeAudioSettings(settings);
      } else {
        final settings = await gateway.getTimelineClipVisualSettings(
          session,
          project,
          trackId: trackId,
          clipId: clipId,
        );
        if (!mounted || generation != _loadGeneration) return;
        _writeSettings(settings);
      }
    } on ProjectGatewayException catch (error) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() => _error = error.message);
    } catch (_) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() => _error = 'Video settings could not be loaded.');
    } finally {
      if (mounted && generation == _loadGeneration) {
        setState(() => _loading = false);
      }
    }
  }

  void _writeSettings(ProjectTimelineVisualSettings settings) {
    _fields['x']!.text = _percentLabel(settings.xMilliCanvas);
    _fields['y']!.text = _percentLabel(settings.yMilliCanvas);
    _fields['scale_x']!.text = _percentLabel(settings.scaleXMilli);
    _fields['scale_y']!.text = _percentLabel(settings.scaleYMilli);
    _fields['rotation']!.text = _degreesLabel(settings.rotationMilliDegrees);
    _fields['anchor_x']!.text = _basisPercentLabel(settings.anchorXBasisPoints);
    _fields['anchor_y']!.text = _basisPercentLabel(settings.anchorYBasisPoints);
    _fields['crop_left']!.text = _basisPercentLabel(
      settings.cropLeftBasisPoints,
    );
    _fields['crop_top']!.text = _basisPercentLabel(settings.cropTopBasisPoints);
    _fields['crop_right']!.text = _basisPercentLabel(
      settings.cropRightBasisPoints,
    );
    _fields['crop_bottom']!.text = _basisPercentLabel(
      settings.cropBottomBasisPoints,
    );
    _fields['opacity']!.text = _basisPercentLabel(settings.opacityBasisPoints);
    _fields['brightness']!.text = _percentLabel(settings.brightnessAmountMilli);
    _fields['contrast']!.text = _percentLabel(settings.contrastAmountMilli);
    _fields['saturation']!.text = _percentLabel(settings.saturationAmountMilli);
    _fields['blur']!.text = _pixelsLabel(settings.gaussianBlurRadiusMilli);
    _fields['transition_in_kind']!.text = '${settings.transitionIn.index}';
    _fields['transition_in_duration']!.text =
        settings.transitionInDuration == null
        ? '0'
        : _secondsLabel(settings.transitionInDuration!);
    _fields['transition_out_kind']!.text = '${settings.transitionOut.index}';
    _fields['transition_out_duration']!.text =
        settings.transitionOutDuration == null
        ? '0'
        : _secondsLabel(settings.transitionOutDuration!);
    _modifiedEffects.clear();
    _updateTransitionIn = false;
    _updateTransitionOut = false;
  }

  void _writeAudioSettings(ProjectTimelineAudioSettings settings) {
    _audioFields['gain_db']!.text = (settings.gainMilliDecibels / 1000)
        .toStringAsFixed(3);
    _audioFields['pan_percent']!.text = (settings.panBasisPoints / 100)
        .toStringAsFixed(2);
    _audioFields['fade_in']!.text = _secondsLabel(settings.fadeIn);
    _audioFields['fade_out']!.text = _secondsLabel(settings.fadeOut);
  }

  ProjectTimelineVisualSettings? _readSettings() {
    final x = _percentToMilli(_fields['x']!.text);
    final y = _percentToMilli(_fields['y']!.text);
    final scaleX = _percentToMilli(
      _fields['scale_x']!.text,
      minimum: 1,
      maximum: 100000,
    );
    final scaleY = _percentToMilli(
      _fields['scale_y']!.text,
      minimum: 1,
      maximum: 100000,
    );
    final rotation = _degreesToMilliDegrees(_fields['rotation']!.text);
    final anchorX = _basisPercentToBasisPoints(_fields['anchor_x']!.text);
    final anchorY = _basisPercentToBasisPoints(_fields['anchor_y']!.text);
    final cropLeft = _basisPercentToBasisPoints(_fields['crop_left']!.text);
    final cropTop = _basisPercentToBasisPoints(_fields['crop_top']!.text);
    final cropRight = _basisPercentToBasisPoints(_fields['crop_right']!.text);
    final cropBottom = _basisPercentToBasisPoints(_fields['crop_bottom']!.text);
    final opacity = _basisPercentToBasisPoints(_fields['opacity']!.text);
    final brightness = _percentToMilli(
      _fields['brightness']!.text,
      minimum: -1000,
      maximum: 1000,
    );
    final contrast = _percentToMilli(
      _fields['contrast']!.text,
      minimum: 0,
      maximum: 4000,
    );
    final saturation = _percentToMilli(
      _fields['saturation']!.text,
      minimum: 0,
      maximum: 4000,
    );
    final blur = _pixelsToMilliPixels(_fields['blur']!.text);
    final transitionInKind = int.tryParse(_fields['transition_in_kind']!.text);
    final transitionOutKind = int.tryParse(
      _fields['transition_out_kind']!.text,
    );
    final transitionInDuration = parseSecondsOrRational(
      _fields['transition_in_duration']!.text,
    );
    final transitionOutDuration = parseSecondsOrRational(
      _fields['transition_out_duration']!.text,
    );
    if (x == null ||
        y == null ||
        scaleX == null ||
        scaleY == null ||
        rotation == null ||
        anchorX == null ||
        anchorY == null ||
        cropLeft == null ||
        cropTop == null ||
        cropRight == null ||
        cropBottom == null ||
        opacity == null ||
        brightness == null ||
        contrast == null ||
        saturation == null ||
        blur == null ||
        transitionInKind == null ||
        transitionInKind < 0 ||
        transitionInKind > 3 ||
        transitionOutKind == null ||
        transitionOutKind < 0 ||
        transitionOutKind > 3 ||
        transitionInDuration == null ||
        transitionOutDuration == null ||
        cropLeft + cropRight >= 10000 ||
        cropTop + cropBottom >= 10000 ||
        (transitionInKind != 0 && !transitionInDuration.isPositive) ||
        (transitionOutKind != 0 && !transitionOutDuration.isPositive)) {
      return null;
    }
    return ProjectTimelineVisualSettings(
      xMilliCanvas: x,
      yMilliCanvas: y,
      scaleXMilli: scaleX,
      scaleYMilli: scaleY,
      rotationMilliDegrees: rotation,
      anchorXBasisPoints: anchorX,
      anchorYBasisPoints: anchorY,
      cropLeftBasisPoints: cropLeft,
      cropTopBasisPoints: cropTop,
      cropRightBasisPoints: cropRight,
      cropBottomBasisPoints: cropBottom,
      opacityBasisPoints: opacity,
      brightnessAmountMilli: brightness,
      contrastAmountMilli: contrast,
      saturationAmountMilli: saturation,
      gaussianBlurRadiusMilli: blur,
      transitionIn: ProjectTimelineTransition.values[transitionInKind],
      transitionInDuration: transitionInDuration,
      transitionOut: ProjectTimelineTransition.values[transitionOutKind],
      transitionOutDuration: transitionOutDuration,
      modifiedEffects: Set.unmodifiable(_modifiedEffects),
      updateTransitionIn: _updateTransitionIn,
      updateTransitionOut: _updateTransitionOut,
    );
  }

  ProjectTimelineAudioSettings? _readAudioSettings() => audioSettingsFromTexts(
    gain: _audioFields['gain_db']!.text,
    pan: _audioFields['pan_percent']!.text,
    fadeIn: _audioFields['fade_in']!.text,
    fadeOut: _audioFields['fade_out']!.text,
  );

  Future<void> _apply(ProjectTimelineVisualSettings settings) async {
    final project = widget.project;
    final trackId = widget.trackId;
    final clipId = widget.clipId;
    final onUpdate = widget.onUpdate;
    if (project == null ||
        trackId == null ||
        clipId == null ||
        onUpdate == null) {
      return;
    }
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      await onUpdate(project, trackId, clipId, settings);
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  Future<void> _applyAudio(ProjectTimelineAudioSettings settings) async {
    final project = widget.project;
    final trackId = widget.trackId;
    final clipId = widget.clipId;
    final onUpdate = widget.onUpdateAudio;
    if (project == null ||
        trackId == null ||
        clipId == null ||
        onUpdate == null) {
      return;
    }
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      await onUpdate(project, trackId, clipId, settings);
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  Future<void> _applyPlaybackRate(ProjectRationalRate rate) async {
    final project = widget.project;
    final trackId = widget.trackId;
    final clipId = widget.clipId;
    final onUpdate = widget.onUpdatePlaybackRate;
    if (project == null ||
        trackId == null ||
        clipId == null ||
        onUpdate == null) {
      return;
    }
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      await onUpdate(project, trackId, clipId, rate);
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  Future<void> _applyReversed(bool reversed) async {
    final project = widget.project;
    final trackId = widget.trackId;
    final clipId = widget.clipId;
    final onSet = widget.onSetReversed;
    if (project == null || trackId == null || clipId == null || onSet == null) {
      return;
    }
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      await onSet(project, trackId, clipId, reversed);
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  Widget _reverseRow(bool enabled) {
    final clip = widget.clip;
    if (clip == null || widget.onSetReversed == null) {
      return const SizedBox.shrink();
    }
    return Padding(
      padding: const EdgeInsets.only(top: OrSpacing.x1),
      child: Row(
        children: [
          Expanded(
            child: Text(
              'Play backwards',
              style: TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.tinySize,
              ),
            ),
          ),
          Switch(
            key: const ValueKey('inspector-clip-reversed'),
            value: clip.reversed,
            onChanged: enabled ? _applyReversed : null,
          ),
        ],
      ),
    );
  }

  Future<void> _applyFrozen(bool frozen) async {
    final project = widget.project;
    final trackId = widget.trackId;
    final clipId = widget.clipId;
    final onSet = widget.onSetFrozen;
    if (project == null || trackId == null || clipId == null || onSet == null) {
      return;
    }
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      await onSet(project, trackId, clipId, frozen);
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  Widget _freezeRow(bool enabled) {
    final clip = widget.clip;
    if (clip == null || widget.onSetFrozen == null) {
      return const SizedBox.shrink();
    }
    return Padding(
      padding: const EdgeInsets.only(top: OrSpacing.x1),
      child: Row(
        children: [
          Expanded(
            child: Text(
              'Hold this frame',
              style: TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.tinySize,
              ),
            ),
          ),
          Switch(
            key: const ValueKey('inspector-clip-frozen'),
            value: clip.frozen,
            onChanged: enabled ? _applyFrozen : null,
          ),
        ],
      ),
    );
  }

  Widget _playbackRateEditor() {
    final clip = widget.clip;
    if (clip == null ||
        clip.contentKind != ProjectTimelineClipContentKind.media ||
        widget.onUpdatePlaybackRate == null) {
      return const SizedBox.shrink();
    }
    const presets = [
      ProjectRationalRate(1, 4),
      ProjectRationalRate(1, 2),
      ProjectRationalRate(3, 4),
      ProjectRationalRate(1, 1),
      ProjectRationalRate(5, 4),
      ProjectRationalRate(3, 2),
      ProjectRationalRate(2, 1),
      ProjectRationalRate(4, 1),
    ];
    final current = clip.playbackRate;
    final currentValue = '${current.numerator}/${current.denominator}';
    final choices =
        presets.any(
          (rate) =>
              rate.numerator == current.numerator &&
              rate.denominator == current.denominator,
        )
        ? presets
        : [current, ...presets];
    final enabled =
        !widget.busy && !_saving && !_loading && !widget.trackLocked;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          'CLIP',
          style: TextStyle(
            color: OrColors.textMuted,
            fontSize: OrType.tinySize,
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: OrSpacing.x1),
        InputDecorator(
          decoration: const InputDecoration(
            labelText: 'Playback speed',
            isDense: true,
            contentPadding: EdgeInsets.symmetric(
              horizontal: OrSpacing.x2,
              vertical: OrSpacing.x1,
            ),
          ),
          child: DropdownButtonHideUnderline(
            child: DropdownButton<String>(
              key: const ValueKey('inspector-playback-rate'),
              value: currentValue,
              isExpanded: true,
              items: choices
                  .map(
                    (rate) => DropdownMenuItem<String>(
                      value: '${rate.numerator}/${rate.denominator}',
                      child: Text(
                        _playbackRateLabel(rate),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  )
                  .toList(growable: false),
              onChanged: enabled
                  ? (value) {
                      if (value == null) return;
                      final rate = choices.firstWhere(
                        (candidate) =>
                            '${candidate.numerator}/${candidate.denominator}' ==
                            value,
                      );
                      unawaited(_applyPlaybackRate(rate));
                    }
                  : null,
            ),
          ),
        ),
        _reverseRow(enabled),
        _freezeRow(enabled),
        const SizedBox(height: OrSpacing.x3),
      ],
    );
  }

  String _playbackRateLabel(ProjectRationalRate rate) =>
      switch ('${rate.numerator}/${rate.denominator}') {
        '1/4' => '0.25×',
        '1/2' => '0.5×',
        '3/4' => '0.75×',
        '1/1' => '1×',
        '5/4' => '1.25×',
        '3/2' => '1.5×',
        '2/1' => '2×',
        '4/1' => '4×',
        _ => '${rate.numerator}/${rate.denominator}×',
      };

  Widget _numberField(String key, String label, {required bool signed}) {
    final expression = RegExp(
      signed ? r'^-?\d{0,7}(?:\.\d{0,3})?$' : r'^\d{0,7}(?:\.\d{0,3})?$',
    );
    final limit = switch (key) {
      'x' || 'y' => 100000,
      'rotation' => 360000,
      'scale_x' || 'scale_y' => 100000,
      'brightness' => 1000,
      'blur' => 128000,
      'contrast' || 'saturation' => 4000,
      _ => 10000,
    };
    return TextField(
      key: ValueKey('inspector-visual-$key'),
      controller: _fields[key],
      enabled: !widget.busy && !_saving && !_loading && !widget.trackLocked,
      onChanged: (value) {
        final effect = switch (key) {
          'brightness' => ProjectTimelineEffectKind.brightness,
          'contrast' => ProjectTimelineEffectKind.contrast,
          'saturation' => ProjectTimelineEffectKind.saturation,
          'blur' => ProjectTimelineEffectKind.gaussianBlur,
          _ => null,
        };
        if (effect != null) _modifiedEffects.add(effect);
      },
      keyboardType: TextInputType.numberWithOptions(
        signed: signed,
        decimal: true,
      ),
      inputFormatters: [
        TextInputFormatter.withFunction((oldValue, newValue) {
          if (!expression.hasMatch(newValue.text)) return oldValue;
          if (newValue.text.isEmpty ||
              newValue.text == '-' ||
              newValue.text == '.' ||
              newValue.text == '-.') {
            return newValue;
          }
          final value = double.tryParse(newValue.text);
          if (value == null || !value.isFinite) return oldValue;
          final minimum = switch (key) {
            'x' || 'y' || 'rotation' => -limit.toDouble(),
            'brightness' => -limit.toDouble(),
            'scale_x' || 'scale_y' => 0.1,
            _ => 0.0,
          };
          return value >= minimum && value <= limit ? newValue : oldValue;
        }),
      ],
      decoration: InputDecoration(
        labelText: label,
        isDense: true,
        counterText: '',
        contentPadding: const EdgeInsets.symmetric(
          horizontal: OrSpacing.x2,
          vertical: OrSpacing.x4,
        ),
      ),
      style: const TextStyle(fontSize: OrType.tinySize),
    );
  }

  Widget _visualTimeField(String key, String label) => TextField(
    key: ValueKey('inspector-visual-$key'),
    controller: _fields[key],
    enabled: !widget.busy && !_saving && !_loading && !widget.trackLocked,
    onChanged: (_) {
      if (key == 'transition_in_duration') {
        _updateTransitionIn = true;
      } else {
        _updateTransitionOut = true;
      }
    },
    decoration: InputDecoration(
      labelText: label,
      isDense: true,
      counterText: '',
      helperText: 'Seconds, e.g. 0.5',
      helperMaxLines: 1,
      contentPadding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x2,
        vertical: OrSpacing.x4,
      ),
    ),
    style: const TextStyle(fontSize: OrType.tinySize),
  );

  Widget _transitionField(String key, String label) =>
      DropdownButtonFormField<int>(
        isExpanded: true,
        key: ValueKey('inspector-visual-$key'),
        initialValue: int.tryParse(_fields[key]!.text) ?? 0,
        items: const [
          DropdownMenuItem(value: 0, child: Text('None')),
          DropdownMenuItem(value: 1, child: Text('Cross dissolve')),
          DropdownMenuItem(value: 2, child: Text('Fade through black')),
          DropdownMenuItem(value: 3, child: Text('Wipe')),
        ],
        onChanged: widget.busy || _saving || _loading || widget.trackLocked
            ? null
            : (value) {
                if (value != null) {
                  setState(() {
                    _fields[key]!.text = '$value';
                    if (key == 'transition_in_kind') {
                      _updateTransitionIn = true;
                    } else {
                      _updateTransitionOut = true;
                    }
                  });
                }
              },
        decoration: InputDecoration(labelText: label, isDense: true),
      );

  Widget _audioNumberField(
    String key,
    String label, {
    required bool decimal,
    required int minimum,
    required int maximum,
  }) => TextField(
    key: ValueKey('inspector-audio-$key'),
    controller: _audioFields[key],
    enabled: !widget.busy && !_saving && !_loading && !widget.trackLocked,
    keyboardType: TextInputType.numberWithOptions(
      signed: minimum < 0,
      decimal: decimal,
    ),
    inputFormatters: [
      TextInputFormatter.withFunction((oldValue, newValue) {
        final expression = RegExp(
          decimal ? r'^-?\d{0,3}(\.\d{0,3})?$' : r'^-?\d{0,3}$',
        );
        if (!expression.hasMatch(newValue.text) ||
            newValue.text.isEmpty ||
            newValue.text == '-') {
          return expression.hasMatch(newValue.text) ? newValue : oldValue;
        }
        final parsed = double.tryParse(newValue.text);
        if (parsed == null) return oldValue;
        return parsed >= minimum && parsed <= maximum ? newValue : oldValue;
      }),
    ],
    decoration: InputDecoration(
      labelText: label,
      isDense: true,
      counterText: '',
      contentPadding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x2,
        vertical: OrSpacing.x4,
      ),
    ),
    style: const TextStyle(fontSize: OrType.tinySize),
  );

  Widget _audioTimeField(String key, String label) => TextField(
    key: ValueKey('inspector-audio-$key'),
    controller: _audioFields[key],
    enabled: !widget.busy && !_saving && !_loading && !widget.trackLocked,
    decoration: InputDecoration(
      labelText: label,
      isDense: true,
      counterText: '',
      helperText: 'Seconds, e.g. 0.5',
      helperMaxLines: 1,
      contentPadding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x2,
        vertical: OrSpacing.x4,
      ),
    ),
    style: const TextStyle(fontSize: OrType.tinySize),
  );

  /// What the inspector shows when no clip is selected: the sequence the
  /// edits belong to, plus the facts that explain an empty viewer.
  Widget _projectInspector() {
    final project = widget.project;
    final rate = widget.sequenceFrameRate;
    final duration = widget.contentEnd;
    return ListView(
      key: const ValueKey('inspector-project'),
      padding: const EdgeInsets.all(OrSpacing.x3),
      children: [
        _inspectorSectionLabel('SEQUENCE'),
        if (widget.onSetSequenceFrameRate == null)
          _inspectorFactRow(
            'Frame rate',
            rate == null ? 'Not set yet' : frameRateLabel(rate),
          )
        else
          _inspectorFrameRateRow(rate),
        _inspectorFactRow(
          'Duration',
          duration == null
              ? 'Empty'
              : '${duration.secondsForDisplay.toStringAsFixed(2)} s',
        ),
        _inspectorFactRow(
          'Timeline',
          '${widget.trackCount} track${widget.trackCount == 1 ? '' : 's'} · '
              '${widget.clipCount} clip${widget.clipCount == 1 ? '' : 's'}',
        ),
        const SizedBox(height: OrSpacing.x4),
        _inspectorSectionLabel('PROJECT'),
        _inspectorFactRow('Name', project?.name ?? 'No project open'),
        _inspectorFactRow(
          'Save state',
          project == null
              ? '—'
              : (project.dirty ? 'Unsaved edits' : 'Saved to disk'),
        ),
        const SizedBox(height: OrSpacing.x4),
        Text(
          widget.clipCount == 0
              ? "Import media, then drag it onto the timeline. Selecting a clip "
                    "replaces these sequence facts with that clip's controls."
              : 'Select a clip on the timeline to edit its settings. These '
                    'sequence facts stay visible until you do.',
          style: TextStyle(
            color: OrColors.textMuted,
            fontSize: OrType.tinySize,
            height: 1.5,
          ),
        ),
      ],
    );
  }

  /// The sequence frame rate is a project setting, so the inspector row that
  /// states it is also the row that changes it. A second home for the same
  /// value would let the two drift apart.
  Widget _inspectorFrameRateRow(ProjectRationalRate? rate) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 88,
            child: Text(
              'Frame rate',
              style: const TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.tinySize,
              ),
            ),
          ),
          Expanded(
            child: PopupMenuButton<EditorFrameRateChoice>(
              key: const ValueKey('inspector-sequence-frame-rate'),
              tooltip: 'Sequence frame rate',
              onSelected: (choice) =>
                  widget.onSetSequenceFrameRate?.call(choice.rate),
              itemBuilder: (context) => [
                for (final choice in editorSequenceRateChoices)
                  PopupMenuItem(value: choice, child: Text(choice.label)),
              ],
              child: Text(
                rate == null ? 'Not set yet' : frameRateLabel(rate),
                style: const TextStyle(
                  color: OrColors.selection,
                  fontSize: OrType.tinySize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _inspectorSectionLabel(String text) => Padding(
    padding: const EdgeInsets.only(bottom: OrSpacing.x2),
    child: Text(
      text,
      style: TextStyle(
        color: OrColors.textMuted,
        fontSize: OrType.tinySize,
        fontWeight: FontWeight.w600,
        letterSpacing: 0.6,
      ),
    ),
  );

  Widget _inspectorFactRow(String label, String value) => Padding(
    padding: const EdgeInsets.only(bottom: OrSpacing.x2),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 88,
          child: Text(
            label,
            style: TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.tinySize,
            ),
          ),
        ),
        Expanded(
          child: Text(
            value,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: TextStyle(color: OrColors.text, fontSize: OrType.tinySize),
          ),
        ),
      ],
    ),
  );

  Widget _audioEditor() => ListView(
    key: const ValueKey('inspector-scroll'),
    padding: const EdgeInsets.all(OrSpacing.x2),
    children: [
      _playbackRateEditor(),
      Text(
        'AUDIO',
        style: TextStyle(
          color: OrColors.textMuted,
          fontSize: OrType.tinySize,
          fontWeight: FontWeight.w600,
        ),
      ),
      const SizedBox(height: OrSpacing.x1),
      _audioNumberField(
        'gain_db',
        'Gain (dB)',
        decimal: true,
        minimum: -96,
        maximum: 24,
      ),
      const SizedBox(height: OrSpacing.x1),
      _audioNumberField(
        'pan_percent',
        'Pan (%)',
        decimal: true,
        minimum: -100,
        maximum: 100,
      ),
      const SizedBox(height: OrSpacing.x1),
      _audioTimeField('fade_in', 'Fade in (s)'),
      const SizedBox(height: OrSpacing.x1),
      _audioTimeField('fade_out', 'Fade out (s)'),
      if (_error != null) ...[
        const SizedBox(height: OrSpacing.x2),
        Semantics(
          liveRegion: true,
          child: Text(
            _error!,
            key: const ValueKey('inspector-error'),
            style: TextStyle(
              color: Theme.of(context).colorScheme.error,
              fontSize: OrType.tinySize,
            ),
          ),
        ),
      ],
    ],
  );

  Widget _pair(
    String leftKey,
    String leftLabel,
    bool leftSigned,
    String rightKey,
    String rightLabel,
    bool rightSigned,
  ) {
    return Row(
      children: [
        Expanded(child: _numberField(leftKey, leftLabel, signed: leftSigned)),
        const SizedBox(width: OrSpacing.x1),
        Expanded(
          child: _numberField(rightKey, rightLabel, signed: rightSigned),
        ),
      ],
    );
  }

  /// The commit bar is pinned to the panel instead of the end of a 20-field
  /// scroll: at 800x600 Apply sat below the fold and could not be reached
  /// without discovering that the inspector scrolls.
  Widget _inspectorFooter() {
    final audio = widget.isAudioTrack;
    final disabled = widget.busy || _saving || widget.trackLocked || _loading;
    return Container(
      key: const ValueKey('inspector-commit-bar'),
      padding: const EdgeInsets.all(OrSpacing.x2),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(top: BorderSide(color: OrColors.border)),
      ),
      child: Row(
        children: [
          Expanded(
            child: OutlinedButton(
              key: ValueKey(
                audio ? 'inspector-audio-reset' : 'inspector-visual-reset',
              ),
              onPressed: disabled
                  ? null
                  : () => audio
                        ? _applyAudio(ProjectTimelineAudioSettings.identity)
                        : _apply(ProjectTimelineVisualSettings.identity),
              child: const Text('Reset'),
            ),
          ),
          const SizedBox(width: OrSpacing.x2),
          Expanded(
            child: FilledButton(
              key: ValueKey(
                audio ? 'inspector-audio-apply' : 'inspector-visual-apply',
              ),
              onPressed: disabled
                  ? null
                  : () {
                      if (audio) {
                        final settings = _readAudioSettings();
                        if (settings == null) {
                          setState(
                            () => _error = inspectorAudioValidationMessage,
                          );
                          return;
                        }
                        _applyAudio(settings);
                        return;
                      }
                      final settings = _readSettings();
                      if (settings == null) {
                        setState(
                          () => _error = 'Enter values within the supported ranges. Percentages may use decimals.',
                        );
                        return;
                      }
                      _apply(settings);
                    },
              child: const Text('Apply'),
            ),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final hasSelection = widget.clipId != null && widget.trackId != null;
    final canEdit =
        widget.isProjectWorkspace &&
        (widget.isVisualTrack || widget.isAudioTrack) &&
        hasSelection &&
        !widget.trackLocked;
    return ColoredBox(
      color: OrColors.backgroundRaised,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (widget.showHeader)
            EditorPanelHeader(
              title: 'Inspector',
              trailing: widget.onClose == null
                  ? null
                  : IconButton(
                      key: const ValueKey('mobile-tool-sheet-close'),
                      tooltip: 'Close tool panel',
                      onPressed: widget.onClose,
                      icon: const Icon(Icons.close_outlined),
                    ),
            ),
          const Divider(height: 1),
          if (!canEdit && widget.isProjectWorkspace && !hasSelection)
            Expanded(child: _projectInspector())
          else if (!canEdit)
            Expanded(
              child: OrEmptyState(
                title: hasSelection ? 'Clip settings' : 'No selection',
                message: !widget.isProjectWorkspace
                    ? 'Inspector controls are unavailable in this preview.'
                    : widget.trackLocked
                    ? 'Unlock the selected track to edit clip settings.'
                    : 'Select a clip on the timeline to edit its settings.',
                icon: Icons.tune_outlined,
              ),
            )
          else if (_loading)
            const Expanded(
              child: Center(child: CircularProgressIndicator(strokeWidth: 2)),
            )
          else if (widget.isAudioTrack)
            Expanded(child: _audioEditor())
          else
            Expanded(
              child: ListView(
                key: const ValueKey('inspector-scroll'),
                padding: const EdgeInsets.all(OrSpacing.x2),
                children: [
                  _playbackRateEditor(),
                  Text(
                    'TRANSFORM',
                    style: TextStyle(
                      color: OrColors.textMuted,
                      fontSize: OrType.tinySize,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _pair('x', 'X (%)', true, 'y', 'Y (%)', true),
                  const SizedBox(height: OrSpacing.x1),
                  _pair(
                    'scale_x',
                    'Width (%)',
                    false,
                    'scale_y',
                    'Height (%)',
                    false,
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _numberField('rotation', 'Rotation (°)', signed: true),
                  const SizedBox(height: OrSpacing.x1),
                  _pair(
                    'anchor_x',
                    'Anchor X (%)',
                    false,
                    'anchor_y',
                    'Anchor Y (%)',
                    false,
                  ),
                  const SizedBox(height: OrSpacing.x3),
                  Text(
                    'CROP',
                    style: TextStyle(
                      color: OrColors.textMuted,
                      fontSize: OrType.tinySize,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _pair(
                    'crop_left',
                    'Left crop (%)',
                    false,
                    'crop_right',
                    'Right crop (%)',
                    false,
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _pair(
                    'crop_top',
                    'Top crop (%)',
                    false,
                    'crop_bottom',
                    'Bottom crop (%)',
                    false,
                  ),
                  const SizedBox(height: OrSpacing.x3),
                  Text(
                    'OPACITY',
                    style: TextStyle(
                      color: OrColors.textMuted,
                      fontSize: OrType.tinySize,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _numberField('opacity', 'Opacity (%)', signed: false),
                  const SizedBox(height: OrSpacing.x3),
                  Text(
                    'EFFECTS',
                    style: TextStyle(
                      color: OrColors.textMuted,
                      fontSize: OrType.tinySize,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _presetRow(
                    keyPrefix: 'effect-preset',
                    presets: const [
                      ('none', 'None'),
                      ('bright', 'Bright'),
                      ('punch', 'Punch'),
                      ('muted', 'Muted'),
                      ('soft', 'Soft blur'),
                    ],
                    onPick: _applyEffectPreset,
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _numberField('brightness', 'Brightness (%)', signed: true),
                  const SizedBox(height: OrSpacing.x1),
                  _pair(
                    'contrast',
                    'Contrast (%)',
                    false,
                    'saturation',
                    'Saturation (%)',
                    false,
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _numberField('blur', 'Blur (px)', signed: false),
                  const SizedBox(height: OrSpacing.x3),
                  Text(
                    'TRANSITIONS',
                    style: TextStyle(
                      color: OrColors.textMuted,
                      fontSize: OrType.tinySize,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _presetRow(
                    keyPrefix: 'transition-preset',
                    presets: const [
                      ('none', 'None'),
                      ('cross', 'Cross dissolve'),
                      ('black', 'Fade to black'),
                      ('wipe', 'Wipe'),
                    ],
                    onPick: _applyTransitionPreset,
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _transitionField('transition_in_kind', 'Transition in'),
                  const SizedBox(height: OrSpacing.x1),
                  _visualTimeField('transition_in_duration', 'Duration in (s)'),
                  const SizedBox(height: OrSpacing.x1),
                  _transitionField('transition_out_kind', 'Transition out'),
                  const SizedBox(height: OrSpacing.x1),
                  _visualTimeField(
                    'transition_out_duration',
                    'Duration out (s)',
                  ),
                  if (_error != null) ...[
                    const SizedBox(height: OrSpacing.x2),
                    Semantics(
                      liveRegion: true,
                      child: Text(
                        _error!,
                        key: const ValueKey('inspector-error'),
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.error,
                          fontSize: OrType.tinySize,
                        ),
                      ),
                    ),
                  ],
                ],
              ),
            ),
          if (canEdit && !_loading) _inspectorFooter(),
        ],
      ),
    );
  }
}

const _visualFieldKeys = [
  'x',
  'y',
  'scale_x',
  'scale_y',
  'rotation',
  'anchor_x',
  'anchor_y',
  'crop_left',
  'crop_top',
  'crop_right',
  'crop_bottom',
  'opacity',
  'brightness',
  'contrast',
  'saturation',
  'blur',
  'transition_in_kind',
  'transition_out_kind',
];

const _visualTimeFieldKeys = [
  'transition_in_duration',
  'transition_out_duration',
];

const _audioFieldKeys = ['gain_db', 'pan_percent', 'fade_in', 'fade_out'];

/// A horizontal toolbar that makes its overflow visible.
///
/// The timeline actions do not fit beside the viewer at every desktop width.
/// A bare `SingleChildScrollView` hid the rightmost actions with no affordance:
/// measured 2026-10-20 at 1280x800, "Export Captions" and "Add Caption" were
/// off-screen. A persistent scrollbar keeps the remaining actions discoverable
/// and draggable with a pointer.
