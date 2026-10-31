import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:or_viewer_texture/or_viewer_texture.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../project/project_gateway.dart';
import 'editor_frame_rate.dart';
import 'editor_labels.dart';
import 'editor_panel_header.dart';
import 'editor_rational_time.dart';
import 'viewer_caption_overlay.dart';
import 'viewer_transform_overlay.dart';

class EditorViewerPanel extends StatefulWidget {
  const EditorViewerPanel({
    super.key,
    required this.compact,
    required this.gateway,
    required this.session,
    required this.project,
    required this.previewSeekPosition,
    required this.onPreviewSeekConsumed,
    required this.onStateChanged,
    this.hasTimelineContent = false,
    this.selectedVisualSettings,
    this.onCommitTransform,
    this.selectedTextClip,
    this.onCommitTextFormatting,
    this.onProjectChanged,
  });

  final bool compact;
  final ProjectGateway? gateway;
  final ProjectSessionHandle? session;
  final ProjectReadModel? project;

  /// Whether any clip exists. An empty viewer placeholder says what to do
  /// instead of naming an internal state.
  final bool hasTimelineContent;

  /// The selected clip's transform, or null when the selection is not a clip
  /// the viewer can manipulate. A non-null value puts handles on the stage.
  final ProjectTimelineVisualSettings? selectedVisualSettings;
  final ValueChanged<ProjectTimelineVisualSettings>? onCommitTransform;

  /// The selected title or caption, if the selection is one. A text selection
  /// shows the caption overlay instead of the transform box.
  final ProjectTimelineClip? selectedTextClip;
  final void Function(ProjectTimelineClip, ProjectTextFormatting)?
  onCommitTextFormatting;

  /// Reports a project setting this panel changed (canvas, frame rate) so the
  /// shell adopts the new revision and later edits do not conflict.
  final ValueChanged<ProjectReadModel>? onProjectChanged;
  final ProjectRationalTime? previewSeekPosition;
  final VoidCallback? onPreviewSeekConsumed;
  final ValueChanged<ProjectPreviewState> onStateChanged;

  @override
  State<EditorViewerPanel> createState() => EditorViewerPanelState();
}

/// The transport commands the editor's key layer dispatches through.
///
/// Implemented by the viewer panel, which owns the preview state, so a key and
/// the transport buttons call the same code.
mixin _EditorViewerCommands {
  void togglePlayPause();

  void stepFrame(bool backwards);

  void seekBy(double seconds);

  /// Moves the playhead to an exact time chosen by a timeline scrub gesture.
  ///
  /// Distinct from [seekBy] because a drag emits dozens of targets per second:
  /// the viewer coalesces them instead of queueing one preview render each.
  void scrubTo(ProjectRationalTime position);
}

class EditorViewerPanelState extends State<EditorViewerPanel>
    with _EditorViewerCommands, WidgetsBindingObserver {
  static const _rateChoices = editorSequenceRateChoices;

  ProjectPreviewState? _preview;
  ProjectSequenceCanvas? _canvas;
  double _speed = 1;
  bool _speedInFlight = false;
  final ValueNotifier<ProjectPreviewState?> _previewNotifier = ValueNotifier(
    null,
  );
  final ValueNotifier<double> _speedNotifier = ValueNotifier(1);
  final ValueNotifier<ProjectSequenceCanvas?> _canvasNotifier = ValueNotifier(
    null,
  );
  ProjectRationalTime? _scrubPosition;

  /// Newest scrub target that has not been sent to the preview yet.
  ProjectRationalTime? _pendingScrubTarget;
  bool _scrubSeekInFlight = false;
  String? _error;
  Timer? _timer;
  int? _textureId;
  int _epoch = 0;
  bool _busy = false;
  bool _tickInFlight = false;
  bool _revisionRefreshInFlight = false;
  bool _pendingRevisionRefresh = false;
  bool _applicationResumed = true;
  bool _lifecyclePauseAttempted = false;

  bool get _connected => widget.gateway != null && widget.session != null;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    unawaited(_initialize());
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      _applicationResumed = true;
      _lifecyclePauseAttempted = false;
      return;
    }
    if (_applicationResumed) _lifecyclePauseAttempted = false;
    _applicationResumed = false;
    _timer?.cancel();
    _timer = null;
    unawaited(_pauseWhileInactive());
  }

  @override
  void didUpdateWidget(covariant EditorViewerPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    final oldProject = oldWidget.project;
    final project = widget.project;
    final previewInputChanged =
        oldWidget.previewSeekPosition != widget.previewSeekPosition &&
        widget.previewSeekPosition != null;
    final identityChanged =
        oldWidget.gateway != widget.gateway ||
        oldWidget.session != widget.session ||
        oldProject?.projectId != project?.projectId ||
        oldProject?.projectInstanceId != project?.projectInstanceId;
    if (identityChanged || project == null) {
      unawaited(_initialize());
      return;
    }
    // A project revision change is the common case: one clip was edited, so the
    // same texture must show the same playhead against the new state. Tearing
    // the viewer down here blanked the stage, re-bound the platform texture and
    // made two IPC round trips for every edit. Keep the picture and the texture
    // and ask for the replacement frame in one call.
    if (previewInputChanged) {
      unawaited(_seekToRequestedPosition());
      return;
    }
    if (oldProject?.revision != project.revision) {
      unawaited(_refreshAfterRevision());
    }
  }

  /// Re-renders the current playhead for a new project revision without a
  /// teardown. A newer revision that arrives while a refresh is in flight is
  /// coalesced into one follow-up call on the latest state.
  Future<void> _refreshAfterRevision() async {
    final gateway = widget.gateway;
    final session = widget.session;
    final project = widget.project;
    if (gateway == null || session == null || project == null) return;
    if (_revisionRefreshInFlight) {
      _pendingRevisionRefresh = true;
      return;
    }
    _revisionRefreshInFlight = true;
    final epoch = _epoch;
    try {
      do {
        _pendingRevisionRefresh = false;
        final position =
            _scrubPosition ??
            _preview?.position ??
            ProjectRationalTime(BigInt.zero, 1);
        final wasPlaying = _preview?.playing ?? false;
        final state = await gateway.previewSeek(session, position);
        if (!_isCurrent(epoch)) return;
        _acceptState(state, epoch, acceptRewind: true);
        if (wasPlaying && mounted && !state.playing) {
          final resumed = await gateway.previewPlay(session);
          if (_isCurrent(epoch)) {
            _acceptState(resumed, epoch, acceptRewind: true);
          }
        }
      } while (_pendingRevisionRefresh);
    } catch (error) {
      if (_isCurrent(epoch)) setState(() => _error = _errorMessage(error));
    } finally {
      _revisionRefreshInFlight = false;
    }
  }

  /// Consumes an explicit seek request from the editor without rebinding the
  /// texture. The full initialise path is still used before the first frame.
  Future<void> _seekToRequestedPosition() async {
    final gateway = widget.gateway;
    final session = widget.session;
    final target = widget.previewSeekPosition;
    if (gateway == null || session == null || target == null) return;
    if (_preview == null || _textureId == null) {
      await _initialize();
      return;
    }
    final epoch = _epoch;
    try {
      final state = await gateway.previewSeek(session, target);
      if (!_isCurrent(epoch)) return;
      _acceptState(state, epoch, acceptRewind: true);
      widget.onPreviewSeekConsumed?.call();
    } catch (error) {
      if (_isCurrent(epoch)) setState(() => _error = _errorMessage(error));
    }
  }

  @override
  void dispose() {
    _previewNotifier.dispose();
    _speedNotifier.dispose();
    _canvasNotifier.dispose();
    _epoch++;
    WidgetsBinding.instance.removeObserver(this);
    _timer?.cancel();
    super.dispose();
  }

  Future<void> _initialize() async {
    final epoch = ++_epoch;
    _timer?.cancel();
    _timer = null;
    _tickInFlight = false;
    if (!mounted) return;
    setState(() {
      _preview = null;
      _scrubPosition = null;
      _textureId = null;
      _error = null;
      _canvas = null;
      _canvasNotifier.value = null;
      _speed = 1;
      _speedNotifier.value = 1;
      _busy = _connected;
    });
    final gateway = widget.gateway;
    final session = widget.session;
    if (gateway == null || session == null) {
      if (mounted) setState(() => _busy = false);
      return;
    }

    try {
      final textureIdFuture = OrViewerTexture.textureId();
      final state = await gateway.previewState(session);
      final settings = await gateway.timelineSequenceSettings(session);
      if (!_isCurrent(epoch)) return;
      setState(() => _canvas = settings.sequenceCanvas);
      _acceptState(state, epoch);
      final requestedPosition = widget.previewSeekPosition;
      final rendered = await gateway.previewSeek(
        session,
        requestedPosition ?? state.position,
      );
      _acceptState(rendered, epoch);
      if (requestedPosition != null) widget.onPreviewSeekConsumed?.call();
      if (_isCurrent(epoch)) setState(() => _busy = false);
      final textureId = await textureIdFuture;
      if (!_isCurrent(epoch)) return;
      setState(() => _textureId = textureId);
      final preview = _preview;
      if (textureId != null &&
          preview != null &&
          preview.frameSequence > BigInt.zero) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (_isCurrent(epoch) && _textureId == textureId) {
            unawaited(OrViewerTexture.frameAvailable());
          }
        });
      }
    } catch (error) {
      if (_isCurrent(epoch)) {
        setState(() {
          _busy = false;
          _error = _errorMessage(error);
        });
      }
    }
  }

  bool _isCurrent(int epoch) => mounted && epoch == _epoch;

  void _acceptState(
    ProjectPreviewState state,
    int epoch, {
    bool acceptRewind = false,
  }) {
    if (!_isCurrent(epoch)) return;
    if (acceptRewind ? _isStaleGeneration(state) : _isOlder(state)) return;
    final previous = _preview;
    setState(() {
      _preview = state;
      _scrubPosition = null;
      _error = state.errorMessage;
    });
    _previewNotifier.value = state;
    widget.onStateChanged(state);
    if (_textureId != null &&
        (previous == null || state.frameSequence > previous.frameSequence)) {
      unawaited(OrViewerTexture.frameAvailable());
    }
    if (state.playing) {
      _startTimer(epoch);
    } else {
      _timer?.cancel();
      _timer = null;
    }
  }

  bool _isOlder(ProjectPreviewState incoming) {
    final current = _preview;
    if (current == null) return false;
    final generationOrder = incoming.generation.compareTo(current.generation);
    if (generationOrder != 0) return generationOrder < 0;
    if (incoming.frameSequence < current.frameSequence ||
        incoming.position.compareTo(current.position) < 0) {
      return true;
    }
    return !current.playing && incoming.playing;
  }

  /// A deliberate seek answers an obsolete request even when its position is
  /// earlier than the one on screen; only a stale generation may be dropped.
  bool _isStaleGeneration(ProjectPreviewState incoming) {
    final current = _preview;
    if (current == null) return false;
    return incoming.generation.compareTo(current.generation) < 0;
  }

  void _startTimer(int epoch) {
    if (_timer != null || !_applicationResumed) return;
    _timer = Timer.periodic(const Duration(milliseconds: 16), (_) {
      if (!_isCurrent(epoch) || _tickInFlight || _busy) return;
      unawaited(_tick(epoch));
    });
  }

  Future<void> _tick(int epoch) async {
    final gateway = widget.gateway;
    final session = widget.session;
    if (gateway == null || session == null || _tickInFlight) return;
    _tickInFlight = true;
    try {
      _acceptState(await gateway.previewTick(session), epoch);
    } catch (error) {
      if (_isCurrent(epoch)) setState(() => _error = _errorMessage(error));
    } finally {
      _tickInFlight = false;
      if (!_applicationResumed) unawaited(_pauseWhileInactive());
    }
  }

  /// The transport surface a keyboard command dispatches through.
  ///
  /// Implemented by the viewer panel so the key handler calls exactly the
  /// code the transport buttons call, and nothing else.
  @override
  void togglePlayPause() {
    if (widget.gateway == null ||
        widget.session == null ||
        _busy ||
        _preview == null) {
      return;
    }
    if (_preview!.playing) {
      unawaited(_run((gateway, session) => gateway.previewPause(session)));
    } else if (_preview!.contentEnd != null) {
      unawaited(_run((gateway, session) => gateway.previewPlay(session)));
    }
  }

  /// Step one frame. Presentation only.
  @override
  void stepFrame(bool backwards) {
    if (widget.gateway == null || widget.session == null || _busy) return;
    unawaited(
      _run(
        (gateway, session) => gateway.previewStep(
          session,
          backwards
              ? ProjectPreviewFrameStep.previous
              : ProjectPreviewFrameStep.next,
        ),
      ),
    );
  }

  /// Seek by a signed number of seconds, clamped to the loaded content.
  /// Presentation only: no project revision is involved.
  @override
  void seekBy(double seconds) {
    if (widget.gateway == null ||
        widget.session == null ||
        _busy ||
        _preview == null) {
      return;
    }
    final end = _preview!.contentEnd;
    if (end == null) return;
    final target = _preview!.position.secondsForDisplay + seconds;
    if (target.isNaN || target.isInfinite) return;
    final clamped = target.clamp(0.0, end.secondsForDisplay).toDouble();
    unawaited(
      _run(
        (gateway, session) => gateway.previewSeek(
          session,
          ProjectRationalTime(BigInt.from((clamped * 1000).round()), 1000),
        ),
      ),
    );
  }

  /// Seek to an exact scrubbed time. Presentation only, like [seekBy].
  ///
  /// A pointer drag outruns the decoder, so the newest target replaces any
  /// target still waiting instead of queueing: the renderer always works on the
  /// frame the pointer is at now, and a released drag ends on the final frame.
  @override
  void scrubTo(ProjectRationalTime position) {
    if (widget.gateway == null || widget.session == null) return;
    _pendingScrubTarget = position.numerator < BigInt.zero
        ? ProjectRationalTime(BigInt.zero, 1)
        : position;
    if (_scrubSeekInFlight) return;
    unawaited(_drainScrubSeeks());
  }

  Future<void> _drainScrubSeeks() async {
    final gateway = widget.gateway;
    final session = widget.session;
    if (gateway == null || session == null) return;
    _scrubSeekInFlight = true;
    final epoch = _epoch;
    try {
      while (_pendingScrubTarget != null) {
        final target = _pendingScrubTarget!;
        _pendingScrubTarget = null;
        final state = await gateway.previewSeek(session, target);
        if (!_isCurrent(epoch)) return;
        _acceptState(state, epoch);
      }
    } catch (error) {
      if (_isCurrent(epoch)) setState(() => _error = _errorMessage(error));
    } finally {
      _scrubSeekInFlight = false;
    }
  }

  Future<void> _pauseWhileInactive() async {
    if (_applicationResumed ||
        _lifecyclePauseAttempted ||
        _preview?.playing != true) {
      return;
    }
    if (_busy || _tickInFlight) return;
    _lifecyclePauseAttempted = true;
    await _run((gateway, session) => gateway.previewPause(session));
  }

  Future<void> _run(
    Future<ProjectPreviewState> Function(
      ProjectGateway gateway,
      ProjectSessionHandle session,
    )
    action,
  ) async {
    final gateway = widget.gateway;
    final session = widget.session;
    if (gateway == null || session == null || _busy) return;
    final epoch = _epoch;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      _acceptState(await action(gateway, session), epoch);
    } catch (error) {
      if (_isCurrent(epoch)) setState(() => _error = _errorMessage(error));
    } finally {
      if (_isCurrent(epoch)) {
        setState(() => _busy = false);
        if (!_applicationResumed) unawaited(_pauseWhileInactive());
      }
    }
  }

  String _errorMessage(Object error) => switch (error) {
    ProjectGatewayException(:final message) => message,
    _ => error.toString(),
  };

  void _onScrubChanged(double seconds) {
    final position = rationalTimeFromSeconds(seconds);
    setState(() => _scrubPosition = position);
    final state = _preview;
    if (state != null) {
      widget.onStateChanged(
        ProjectPreviewState(
          position: position,
          presentedTime: state.presentedTime,
          sequenceFrameRate: state.sequenceFrameRate,
          contentEnd: state.contentEnd,
          playing: state.playing,
          generation: state.generation,
          frameSequence: state.frameSequence,
          width: state.width,
          height: state.height,
          errorCode: state.errorCode,
          errorMessage: state.errorMessage,
        ),
      );
    }
  }

  /// Applies a composition canvas. `null` returns to sizing from the first
  /// clip, which is what projects made before canvases existed use.
  Future<void> _setCanvas(ProjectSequenceCanvas? canvas) async {
    final gateway = widget.gateway;
    final session = widget.session;
    final project = widget.project;
    if (gateway == null ||
        session == null ||
        project == null ||
        _busy ||
        canvas == _canvas) {
      return;
    }
    final epoch = _epoch;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final result = await gateway.setTimelineSequenceCanvas(
        session,
        project,
        canvas,
      );
      if (!result.succeeded) {
        throw ProjectGatewayException(result.errorCode, result.message);
      }
      final view = result.view;
      if (view != null) widget.onProjectChanged?.call(view);
      final settings = await gateway.timelineSequenceSettings(session);
      final state = await gateway.previewState(session);
      if (!_isCurrent(epoch)) return;
      setState(() => _canvas = settings.sequenceCanvas);
      _canvasNotifier.value = settings.sequenceCanvas;
      _acceptState(state, epoch, acceptRewind: true);
    } catch (error) {
      if (_isCurrent(epoch)) setState(() => _error = _errorMessage(error));
    } finally {
      if (_isCurrent(epoch)) setState(() => _busy = false);
    }
  }

  Future<void> _setSpeed(double speed) async {
    final gateway = widget.gateway;
    final session = widget.session;
    if (gateway == null || session == null || _speedInFlight || _busy) return;
    if (speed == _speed) return;
    _speedInFlight = true;
    final epoch = _epoch;
    final previous = _speed;
    setState(() {
      _speed = speed;
      _error = null;
    });
    _speedNotifier.value = speed;
    try {
      final state = await gateway.previewSetSpeed(session, speed);
      if (!_isCurrent(epoch)) return;
      _acceptState(state, epoch, acceptRewind: true);
    } catch (error) {
      if (_isCurrent(epoch)) {
        setState(() {
          _speed = previous;
          _error = _errorMessage(error);
        });
        _speedNotifier.value = previous;
      }
    } finally {
      _speedInFlight = false;
    }
  }

  Future<void> _setFrameRate(ProjectRationalRate? rate) async {
    final gateway = widget.gateway;
    final session = widget.session;
    final project = widget.project;
    if (gateway == null || session == null || project == null || _busy) return;
    final epoch = _epoch;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final result = await gateway.setTimelineSequenceFrameRate(
        session,
        project,
        rate,
      );
      if (!result.succeeded) {
        throw ProjectGatewayException(result.errorCode, result.message);
      }
      final view = result.view;
      if (view != null) widget.onProjectChanged?.call(view);
      final state = await gateway.previewState(session);
      if (!_isCurrent(epoch)) return;
      _acceptState(state, epoch, acceptRewind: true);
    } catch (error) {
      if (_isCurrent(epoch)) setState(() => _error = _errorMessage(error));
    } finally {
      if (_isCurrent(epoch)) setState(() => _busy = false);
    }
  }

  /// Placeholder content for the viewer surface.
  ///
  /// A short compact viewport can squeeze the surface below the height the
  /// icon-and-text column needs. Keep the icon and expose the message through
  /// semantics instead of letting the column overflow.
  Widget _viewerPlaceholder({required IconData icon, required String message}) {
    return Center(
      child: LayoutBuilder(
        builder: (context, constraints) {
          if (constraints.maxHeight < 56 || constraints.maxWidth < 120) {
            return Semantics(
              label: message,
              liveRegion: true,
              child: Icon(icon, size: 18, color: OrColors.textMuted),
            );
          }
          return Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 22, color: OrColors.textMuted),
              const SizedBox(height: OrSpacing.x2),
              Text(
                message,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  /// The project canvas, named by what people publish to and by its size.
  ///
  /// The old badge showed the texture size, which looked editable and was not;
  /// it also changed when the window did. This button is the real control.
  Widget _canvasButton() => _CanvasPicker(
    buttonKey: const ValueKey('preview-canvas'),
    labelKey: const ValueKey('preview-canvas-label'),
    canvas: _canvas,
    enabled: _connected && widget.project != null && !_busy,
    onSelected: (canvas) => unawaited(_setCanvas(canvas)),
  );

  Future<void> _openFullscreen() async {
    final preview = _preview;
    final textureId = _textureId;
    if (preview == null || textureId == null) return;
    await showDialog<void>(
      context: context,
      barrierColor: Colors.black,
      builder: (dialogContext) => Dialog.fullscreen(
        backgroundColor: Colors.black,
        child: _FullscreenPreview(
          textureId: textureId,
          preview: _previewNotifier,
          speed: _speedNotifier,
          canvas: _canvasNotifier,
          onClose: () => Navigator.of(dialogContext).pop(),
          onPlayPause: togglePlayPause,
          onSeekBy: seekBy,
          onScrub: (seconds) {
            final position = rationalTimeFromSeconds(seconds);
            setState(() => _scrubPosition = position);
            scrubTo(position);
          },
          onScrubEnd: _seekToSeconds,
          onSpeed: (speed) => unawaited(_setSpeed(speed)),
          onCanvas: (canvas) => unawaited(_setCanvas(canvas)),
        ),
      ),
    );
  }

  void _seekToSeconds(double seconds) {
    setState(() => _scrubPosition = null);
    unawaited(
      _run(
        (gateway, session) =>
            gateway.previewSeek(session, rationalTimeFromSeconds(seconds)),
      ),
    );
  }

  double get _transportControlSize =>
      widget.compact ? OrTouchTargets.comfortable : 40.0;

  ButtonStyle get _transportButtonStyle => IconButton.styleFrom(
    minimumSize: Size.square(_transportControlSize),
    fixedSize: Size.square(_transportControlSize),
    padding: EdgeInsets.zero,
    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
  );

  Widget _buildViewerSurface() {
    final textureId = _textureId;
    final preview = _preview;
    final hasSession = _connected;
    final width = preview?.width ?? 0;
    final height = preview?.height ?? 0;
    final aspectRatio = width > 0 && height > 0 ? width / height : 16 / 9;
    return Container(
      key: const ValueKey('editor-viewer-surface'),
      alignment: Alignment.center,
      color: OrColors.background,
      child: LayoutBuilder(
        builder: (context, constraints) {
          // Fit the preview box to *both* available dimensions. `AspectRatio`
          // inside an `Expanded` receives a tight height and sizes itself from
          // the width, so at a narrow short viewport it overflows: measured
          // 2026-10-18, the 320x440 compact editor overflowed the viewer by 25 px
          // as soon as the scrub strip appeared, and the same shape overflows at
          // any short viewport. Fitting explicitly letterboxes the preview
          // instead, which is what a viewer should do.
          // A desktop viewer uses the space it is given. The old 660x420 cap
          // left a 1440-wide window previewing at less than a third of its
          // panel, which reads as a thumbnail rather than as the edit surface.
          final limitWidth = widget.compact ? 520.0 : constraints.maxWidth;
          final limitHeight = widget.compact ? 360.0 : constraints.maxHeight;
          final availableWidth = constraints.maxWidth.isFinite
              ? constraints.maxWidth
              : limitWidth;
          final availableHeight = constraints.maxHeight.isFinite
              ? constraints.maxHeight
              : limitHeight;
          var fittedWidth = math.min(availableWidth, limitWidth);
          var fittedHeight = fittedWidth / aspectRatio;
          if (fittedHeight > math.min(availableHeight, limitHeight)) {
            fittedHeight = math.min(availableHeight, limitHeight);
            fittedWidth = fittedHeight * aspectRatio;
          }
          return SizedBox(
            key: const ValueKey('editor-viewer-preview'),
            width: fittedWidth,
            height: fittedHeight,
            child: Container(
              decoration: BoxDecoration(
                color: OrColors.backgroundRecessed,
                border: Border.all(color: OrColors.borderStrong),
                borderRadius: BorderRadius.circular(OrRadii.small),
              ),
              clipBehavior: Clip.antiAlias,
              child: Stack(
                fit: StackFit.expand,
                children: [
                  if (preview?.contentEnd == null)
                    _viewerPlaceholder(
                      icon: Icons.movie_outlined,
                      message: widget.hasTimelineContent
                          ? 'No clip at the playhead'
                          : 'Drag a clip to the timeline to preview it',
                    )
                  else if (textureId != null)
                    Texture(
                      key: ValueKey('or-viewer-texture-$textureId'),
                      textureId: textureId,
                      filterQuality: FilterQuality.medium,
                    )
                  else
                    _viewerPlaceholder(
                      icon: hasSession
                          ? Icons.error_outline
                          : Icons.movie_outlined,
                      message: hasSession
                          ? 'Viewer texture unavailable'
                          : 'Open a project to preview',
                    ),
                  if (widget.selectedTextClip != null &&
                      widget.onCommitTextFormatting != null)
                    ViewerCaptionOverlay(
                      key: const ValueKey('viewer-caption-overlay'),
                      canvasSize: Size(
                        (preview?.width ?? 0).toDouble(),
                        (preview?.height ?? 0).toDouble(),
                      ),
                      clip: widget.selectedTextClip!,
                      onCommit: (formatting) => widget.onCommitTextFormatting!(
                        widget.selectedTextClip!,
                        formatting,
                      ),
                      enabled: !_busy,
                    )
                  else if (widget.selectedVisualSettings != null &&
                      widget.onCommitTransform != null)
                    ViewerTransformOverlay(
                      key: const ValueKey('viewer-transform-overlay'),
                      canvasSize: Size(
                        (preview?.width ?? 0).toDouble(),
                        (preview?.height ?? 0).toDouble(),
                      ),
                      settings: widget.selectedVisualSettings!,
                      onCommit: widget.onCommitTransform!,
                      enabled: !_busy,
                    ),
                  if (_busy)
                    const Center(
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          color: OrColors.backgroundRaised,
                          border: Border.fromBorderSide(
                            BorderSide(color: OrColors.border),
                          ),
                          borderRadius: BorderRadius.all(
                            Radius.circular(OrRadii.small),
                          ),
                        ),
                        child: Padding(
                          padding: EdgeInsets.symmetric(
                            horizontal: OrSpacing.x2,
                            vertical: OrSpacing.x1,
                          ),
                          child: Text(
                            'Updating preview',
                            style: TextStyle(
                              color: OrColors.textSecondary,
                              fontSize: OrType.tinySize,
                            ),
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final preview = _preview;
    final endSeconds = preview?.contentEnd?.secondsForDisplay ?? 0;
    final hasRange = endSeconds.isFinite && endSeconds > 0;
    final shownPosition = _scrubPosition ?? preview?.position;
    final frameRate = preview?.sequenceFrameRate;
    final hasPlayableFrames = hasRange;
    final canStep =
        _connected && frameRate != null && hasPlayableFrames && !_busy;
    final canPlay =
        _connected && frameRate != null && hasPlayableFrames && !_busy;
    final canSeek = _connected && hasRange && !_busy;
    final rulerMaximum = hasRange ? endSeconds : 1.0;
    final sliderValue = (shownPosition?.secondsForDisplay ?? 0)
        .clamp(0.0, rulerMaximum)
        .toDouble();

    return ColoredBox(
      key: const ValueKey('editor-viewer-panel'),
      color: OrColors.background,
      child: LayoutBuilder(
        builder: (context, constraints) {
          // A short panel cannot hold the header, the surface, the ruler, the
          // ruler's clock labels, the scrub row and the transport row. Measured
          // 2026-10-18 at 320x440: the viewer received 149 px, the surface
          // collapsed to zero height and the column still overflowed by 25 px.
          // A short panel keeps what the user acts with — the surface, the scrub
          // control and the transport — and drops the ruler and its clock labels,
          // which the live position readout and the timeline's own ruler still
          // convey. It never clips and never squeezes away a control.
          final shortPanel = constraints.maxHeight < 200;
          return Column(
            children: [
              if (!widget.compact)
                EditorPanelHeader(
                  key: const ValueKey('editor-viewer-header'),
                  title: 'Viewer',
                  // The short panel keeps two pixels for the surface's remainder: with
                  // the strip at its minimum the sum came to one pixel more than the
                  // panel, and a one-pixel overflow is still a clipped control.
                  height: shortPanel ? 32 : (widget.compact ? 34 : 38),
                  trailing: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      _canvasButton(),
                      IconButton(
                        key: const ValueKey('preview-fullscreen'),
                        tooltip: 'Full screen preview',
                        visualDensity: VisualDensity.compact,
                        onPressed: preview == null || preview.width == 0
                            ? null
                            : _openFullscreen,
                        icon: const Icon(Icons.fullscreen, size: 18),
                      ),
                    ],
                  ),
                ),
              Expanded(child: _buildViewerSurface()),
              Container(
                key: const ValueKey('editor-viewer-transport'),
                decoration: const BoxDecoration(
                  color: OrColors.backgroundRaised,
                  border: Border(top: BorderSide(color: OrColors.border)),
                ),
                padding: const EdgeInsets.fromLTRB(
                  OrSpacing.x3,
                  OrSpacing.x1,
                  OrSpacing.x3,
                  OrSpacing.x1,
                ),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (hasRange) ...[
                      if (!shortPanel)
                        SizedBox(
                          height: 8,
                          child: CustomPaint(
                            painter: _PreviewRulerPainter(),
                            size: Size.infinite,
                          ),
                        ),
                      SizedBox(
                        height: 32,
                        child: SliderTheme(
                          data: SliderTheme.of(context).copyWith(
                            trackHeight: 2,
                            activeTrackColor: OrColors.selection,
                            inactiveTrackColor: OrColors.borderStrong,
                            thumbColor: OrColors.text,
                            overlayShape: const RoundSliderOverlayShape(
                              overlayRadius: 10,
                            ),
                          ),
                          child: Slider(
                            key: const ValueKey('preview-scrub-ruler'),
                            min: 0,
                            max: rulerMaximum,
                            value: sliderValue,
                            onChanged: _busy || !_connected
                                ? null
                                : _onScrubChanged,
                            onChangeEnd: _busy || !_connected
                                ? null
                                : (seconds) {
                                    setState(() => _scrubPosition = null);
                                    unawaited(
                                      _run(
                                        (gateway, session) =>
                                            gateway.previewSeek(
                                              session,
                                              rationalTimeFromSeconds(seconds),
                                            ),
                                      ),
                                    );
                                  },
                          ),
                        ),
                      ),
                      if (!shortPanel)
                        Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            Text(
                              clockLabel(0),
                              style: const TextStyle(
                                color: OrColors.textMuted,
                                fontFamily: 'monospace',
                                fontSize: OrType.tinySize,
                              ),
                            ),
                            Text(
                              clockLabel(endSeconds),
                              style: const TextStyle(
                                color: OrColors.textMuted,
                                fontFamily: 'monospace',
                                fontSize: OrType.tinySize,
                              ),
                            ),
                          ],
                        ),
                    ],
                    // A narrow viewer keeps the controls it cannot live
                    // without (step, play, time, rate). Skip and speed are
                    // extras that would squeeze the time readout to nothing;
                    // the full-screen transport still offers both.
                    LayoutBuilder(
                      builder: (context, rowConstraints) {
                        final wide = rowConstraints.maxWidth >= 360;
                        return Row(
                          children: [
                            if (wide && !widget.compact)
                              IconButton(
                                key: const ValueKey('preview-skip-back'),
                                tooltip: 'Back 5 seconds ([)',
                                style: _transportButtonStyle,
                                onPressed: canSeek ? () => seekBy(-5) : null,
                                icon: const Icon(Icons.replay_5, size: 18),
                              ),
                            IconButton(
                              key: const ValueKey('preview-previous-frame'),
                              tooltip: 'Previous frame',
                              style: _transportButtonStyle,
                              onPressed: canStep
                                  ? () => unawaited(
                                      _run(
                                        (gateway, session) =>
                                            gateway.previewStep(
                                              session,
                                              ProjectPreviewFrameStep.previous,
                                            ),
                                      ),
                                    )
                                  : null,
                              icon: const Icon(
                                Icons.skip_previous_outlined,
                                size: 18,
                              ),
                            ),
                            IconButton(
                              key: const ValueKey('preview-play'),
                              tooltip: preview?.playing == true
                                  ? 'Pause'
                                  : 'Play',
                              style: _transportButtonStyle,
                              onPressed: preview?.playing == true
                                  ? () => unawaited(
                                      _run((gateway, session) {
                                        return gateway.previewPause(session);
                                      }),
                                    )
                                  : canPlay
                                  ? () => unawaited(
                                      _run((gateway, session) {
                                        return gateway.previewPlay(session);
                                      }),
                                    )
                                  : null,
                              icon: Icon(
                                preview?.playing == true
                                    ? Icons.pause_outlined
                                    : Icons.play_arrow_outlined,
                                size: 20,
                              ),
                            ),
                            IconButton(
                              key: const ValueKey('preview-next-frame'),
                              tooltip: 'Next frame',
                              style: _transportButtonStyle,
                              onPressed: canStep
                                  ? () => unawaited(
                                      _run(
                                        (gateway, session) =>
                                            gateway.previewStep(
                                              session,
                                              ProjectPreviewFrameStep.next,
                                            ),
                                      ),
                                    )
                                  : null,
                              icon: const Icon(
                                Icons.skip_next_outlined,
                                size: 18,
                              ),
                            ),
                            if (wide && !widget.compact)
                              IconButton(
                                key: const ValueKey('preview-skip-forward'),
                                tooltip: 'Forward 5 seconds (])',
                                style: _transportButtonStyle,
                                onPressed: canSeek ? () => seekBy(5) : null,
                                icon: const Icon(Icons.forward_5, size: 18),
                              ),
                            if (widget.compact)
                              IconButton(
                                key: const ValueKey('preview-fullscreen'),
                                tooltip: 'Full screen preview',
                                style: _transportButtonStyle,
                                onPressed: preview == null || preview.width == 0
                                    ? null
                                    : _openFullscreen,
                                icon: const Icon(Icons.fullscreen, size: 18),
                              ),
                            Expanded(
                              child: Tooltip(
                                message: shownPosition == null
                                    ? 'Playhead position'
                                    : 'Exact ${shownPosition.canonical}'
                                          '${preview?.presentedTime == null ? '' : ' · presented ${preview!.presentedTime!.canonical}'}',
                                child: Text(
                                  shownPosition == null
                                      ? '00:00:00:00'
                                      : _timeFeedback(
                                          shownPosition,
                                          preview?.presentedTime,
                                          frameRate,
                                        ),
                                  key: const ValueKey('preview-time-readout'),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  textAlign: TextAlign.center,
                                  style: const TextStyle(
                                    color: OrColors.textSecondary,
                                    fontFamily: 'monospace',
                                    fontSize: OrType.tinySize,
                                  ),
                                ),
                              ),
                            ),
                            // A 246 px viewer cannot hold three transport buttons
                            // plus a rate label. Flexible keeps the row on the
                            // panel instead of painting the label off its edge.
                            Flexible(
                              child: PopupMenuButton<EditorFrameRateChoice>(
                                key: const ValueKey('preview-frame-rate'),
                                tooltip: frameRate == null
                                    ? 'Choose a sequence frame rate to enable playback'
                                    : 'Sequence frame rate (playback and export)',
                                enabled:
                                    _connected &&
                                    widget.project != null &&
                                    !_busy,
                                onSelected: (choice) =>
                                    unawaited(_setFrameRate(choice.rate)),
                                itemBuilder: (context) => [
                                  for (final choice in _rateChoices)
                                    PopupMenuItem(
                                      value: choice,
                                      child: Text(choice.label),
                                    ),
                                ],
                                child: SizedBox(
                                  height: _transportControlSize,
                                  child: Padding(
                                    padding: const EdgeInsets.symmetric(
                                      horizontal: 6,
                                    ),
                                    child: Center(
                                      widthFactor: 1,
                                      child: Text(
                                        frameRate == null
                                            ? 'Set frame rate'
                                            : frameRateLabel(frameRate),
                                        key: const ValueKey(
                                          'preview-rate-label',
                                        ),
                                        style: TextStyle(
                                          color: frameRate == null
                                              ? OrColors.selection
                                              : OrColors.textSecondary,
                                          fontSize: OrType.tinySize,
                                          fontWeight: frameRate == null
                                              ? FontWeight.w600
                                              : FontWeight.w400,
                                        ),
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            ),
                            if (wide && !widget.compact)
                              PopupMenuButton<double>(
                                key: const ValueKey('preview-speed'),
                                tooltip: 'Playback speed',
                                enabled: _connected && !_busy,
                                onSelected: (speed) =>
                                    unawaited(_setSpeed(speed)),
                                itemBuilder: (context) => [
                                  for (final speed in _previewSpeeds)
                                    PopupMenuItem<double>(
                                      key: ValueKey(
                                        'preview-speed-${_speedLabel(speed)}',
                                      ),
                                      value: speed,
                                      child: Row(
                                        mainAxisSize: MainAxisSize.min,
                                        children: [
                                          SizedBox(
                                            width: 20,
                                            child: speed == _speed
                                                ? const Icon(
                                                    Icons.check,
                                                    size: 16,
                                                    color: OrColors.selection,
                                                  )
                                                : null,
                                          ),
                                          Text('${_speedLabel(speed)}×'),
                                        ],
                                      ),
                                    ),
                                  const PopupMenuDivider(),
                                  PopupMenuItem<double>(
                                    enabled: false,
                                    height: 30,
                                    child: Text(
                                      'Audio plays at 1× only',
                                      style: TextStyle(
                                        color: OrColors.textMuted,
                                        fontSize: OrType.tinySize,
                                      ),
                                    ),
                                  ),
                                ],
                                child: SizedBox(
                                  height: _transportControlSize,
                                  child: Padding(
                                    padding: const EdgeInsets.symmetric(
                                      horizontal: 6,
                                    ),
                                    child: Center(
                                      widthFactor: 1,
                                      child: Text(
                                        '${_speedLabel(_speed)}×',
                                        key: const ValueKey(
                                          'preview-speed-label',
                                        ),
                                        style: TextStyle(
                                          color: _speed == 1
                                              ? OrColors.textSecondary
                                              : OrColors.selection,
                                          fontSize: OrType.tinySize,
                                          fontWeight: _speed == 1
                                              ? FontWeight.w400
                                              : FontWeight.w600,
                                        ),
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                          ],
                        );
                      },
                    ),
                    if (_error != null)
                      Padding(
                        padding: const EdgeInsets.only(bottom: OrSpacing.x1),
                        child: Text(
                          _error!,
                          key: const ValueKey('preview-error'),
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          textAlign: TextAlign.center,
                          style: const TextStyle(
                            color: OrColors.danger,
                            fontSize: OrType.tinySize,
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _PreviewRulerPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final line = Paint()
      ..color = OrColors.borderStrong
      ..strokeWidth = 1;
    for (var index = 0; index <= 8; index++) {
      final x = size.width * index / 8;
      canvas.drawLine(Offset(x, 0), Offset(x, index.isEven ? 7 : 4), line);
    }
  }

  @override
  bool shouldRepaint(covariant _PreviewRulerPainter oldDelegate) => false;
}

/// `0.5×`, `1.5×`, `2×`: a speed badge reads better as a decimal than as the
/// canonical `3/2` the model stores.
/// Transport readout. A configured sequence rate renders SMPTE-style
/// `HH:MM:SS:FF`; without one it falls back to the clock so the exact rational
/// pair (`0/1 · 0/1`) never leaks into the primary label.
String _timeFeedback(
  ProjectRationalTime position,
  ProjectRationalTime? presentedTime,
  ProjectRationalRate? rate,
) {
  final time = position.secondsForDisplay;
  if (rate != null && rate.numerator > 0 && rate.denominator > 0) {
    return _timecodeLabel(time, rate.numerator / rate.denominator);
  }
  return clockLabel(time);
}

String _timecodeLabel(double seconds, double fps) {
  final rate = fps.round().clamp(1, 1000);
  final frames = (seconds * fps + 1e-6).floor();
  final totalSeconds = frames ~/ rate;
  final ff = frames % rate;
  final hh = totalSeconds ~/ 3600;
  final mm = totalSeconds ~/ 60 % 60;
  final ss = totalSeconds % 60;
  String two(int value) => value.toString().padLeft(2, '0');
  return '${two(hh)}:${two(mm)}:${two(ss)}:${two(ff)}';
}

/// One canvas preset, named by where people publish it and by its pixel size.
class _CanvasOption {
  const _CanvasOption(this.label, this.detail, this.width, this.height);

  final String label;
  final String detail;
  final int width;
  final int height;
}

class _CanvasGroup {
  const _CanvasGroup(this.label, this.options);

  final String label;
  final List<_CanvasOption> options;
}

const _matchFirstClip = _CanvasOption(
  'Match the first clip',
  'Automatic size from the first video clip',
  0,
  0,
);

const _canvasGroups = <_CanvasGroup>[
  _CanvasGroup('Landscape', [
    _CanvasOption('YouTube', '1920 × 1080 · 16:9', 1920, 1080),
    _CanvasOption('HD', '1280 × 720 · 16:9', 1280, 720),
    _CanvasOption('Classic TV', '1440 × 1080 · 4:3', 1440, 1080),
  ]),
  _CanvasGroup('Portrait', [
    _CanvasOption('TikTok, Reels, Shorts', '1080 × 1920 · 9:16', 1080, 1920),
    _CanvasOption('Instagram feed', '1080 × 1350 · 4:5', 1080, 1350),
  ]),
  _CanvasGroup('Square', [
    _CanvasOption('Square post', '1080 × 1080 · 1:1', 1080, 1080),
  ]),
];

const _previewSpeeds = <double>[0.25, 0.5, 0.75, 1, 1.5, 2];

String _speedLabel(double speed) =>
    speed == speed.roundToDouble() ? speed.toInt().toString() : '$speed';

/// Canvas control shared by the viewer header and the full-screen transport.
class _CanvasPicker extends StatelessWidget {
  const _CanvasPicker({
    required this.canvas,
    required this.enabled,
    required this.onSelected,
    required this.buttonKey,
    this.labelKey,
    this.tint,
    this.background,
  });

  final ProjectSequenceCanvas? canvas;
  final bool enabled;
  final ValueChanged<ProjectSequenceCanvas?> onSelected;
  final Key buttonKey;
  final Key? labelKey;
  final Color? tint;
  final Color? background;

  @override
  Widget build(BuildContext context) {
    final label = canvas == null
        ? 'Auto'
        : '${canvas!.width} × ${canvas!.height}';
    return PopupMenuButton<_CanvasOption>(
      key: buttonKey,
      tooltip: canvas == null
          ? 'Canvas: match the first clip'
          : 'Canvas: ${canvas!.width} × ${canvas!.height}',
      enabled: enabled,
      onSelected: (option) => onSelected(
        option.width == 0
            ? null
            : ProjectSequenceCanvas(option.width, option.height),
      ),
      itemBuilder: (context) => [
        for (final group in _canvasGroups) ...[
          PopupMenuItem<_CanvasOption>(
            enabled: false,
            height: 26,
            child: Text(
              group.label,
              style: const TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.tinySize,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
          for (final option in group.options)
            PopupMenuItem<_CanvasOption>(
              key: ValueKey('preview-canvas-${option.width}x${option.height}'),
              value: option,
              child: _canvasOptionRow(option),
            ),
        ],
        const PopupMenuDivider(),
        PopupMenuItem<_CanvasOption>(
          value: _matchFirstClip,
          child: _canvasOptionRow(_matchFirstClip),
        ),
      ],
      child: Container(
        height: 28,
        padding: const EdgeInsets.symmetric(horizontal: 8),
        margin: const EdgeInsets.only(right: OrSpacing.x1),
        decoration: BoxDecoration(
          color: background ?? OrColors.surface,
          border: Border.all(
            color: canvas == null ? OrColors.borderStrong : OrColors.selection,
          ),
          borderRadius: BorderRadius.circular(OrRadii.small),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.aspect_ratio,
              size: 14,
              color: tint ?? OrColors.textSecondary,
            ),
            const SizedBox(width: 6),
            Text(
              label,
              key: labelKey,
              style: TextStyle(
                color: tint ?? OrColors.text,
                fontSize: OrType.tinySize,
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _canvasOptionRow(_CanvasOption option) {
    final selected = option.width == 0
        ? canvas == null
        : canvas?.width == option.width && canvas?.height == option.height;
    return SizedBox(
      width: 248,
      child: Row(
        children: [
          SizedBox(
            width: 20,
            child: selected
                ? const Icon(Icons.check, size: 16, color: OrColors.selection)
                : null,
          ),
          Flexible(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  option.label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontSize: OrType.metadataSize),
                ),
                Text(
                  option.detail,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: OrColors.textMuted,
                    fontSize: OrType.tinySize,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// Full-screen preview with a transport that fades out while watching.
///
/// The old full-screen view had one close button: no pause, no scrub, no speed
/// and no time. Every control here calls the same viewer methods as the docked
/// transport, so the two can never disagree.
class _FullscreenPreview extends StatefulWidget {
  const _FullscreenPreview({
    required this.textureId,
    required this.preview,
    required this.speed,
    required this.canvas,
    required this.onClose,
    required this.onPlayPause,
    required this.onSeekBy,
    required this.onScrub,
    required this.onScrubEnd,
    required this.onSpeed,
    required this.onCanvas,
  });

  final int textureId;
  final ValueListenable<ProjectPreviewState?> preview;
  final ValueListenable<double> speed;
  final ValueListenable<ProjectSequenceCanvas?> canvas;
  final VoidCallback onClose;
  final VoidCallback onPlayPause;
  final ValueChanged<double> onSeekBy;
  final ValueChanged<double> onScrub;
  final ValueChanged<double> onScrubEnd;
  final ValueChanged<double> onSpeed;
  final ValueChanged<ProjectSequenceCanvas?> onCanvas;

  @override
  State<_FullscreenPreview> createState() => _FullscreenPreviewState();
}

class _FullscreenPreviewState extends State<_FullscreenPreview> {
  static const _hideDelay = Duration(milliseconds: 2200);

  bool _visible = true;
  bool _scrubbing = false;
  double? _scrubSeconds;
  Timer? _hideTimer;

  @override
  void initState() {
    super.initState();
    widget.preview.addListener(_onPreviewChanged);
    _scheduleHide();
  }

  @override
  void dispose() {
    widget.preview.removeListener(_onPreviewChanged);
    _hideTimer?.cancel();
    super.dispose();
  }

  /// Controls stay put while playback is paused: a paused player with no
  /// visible transport reads as a frozen picture.
  void _onPreviewChanged() {
    if (widget.preview.value?.playing == true) {
      _scheduleHide();
    } else {
      _show();
    }
  }

  void _scheduleHide() {
    _hideTimer?.cancel();
    _hideTimer = Timer(_hideDelay, () {
      if (!mounted || _scrubbing) return;
      if (widget.preview.value?.playing != true) return;
      setState(() => _visible = false);
    });
  }

  void _show() {
    _hideTimer?.cancel();
    if (!_visible) setState(() => _visible = true);
    _scheduleHide();
  }

  void _toggleControls() {
    _hideTimer?.cancel();
    if (_visible) {
      setState(() => _visible = false);
    } else {
      _show();
    }
  }

  @override
  Widget build(BuildContext context) {
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.space): widget.onPlayPause,
        const SingleActivator(LogicalKeyboardKey.escape): widget.onClose,
        const SingleActivator(LogicalKeyboardKey.arrowLeft): () =>
            widget.onSeekBy(-1),
        const SingleActivator(LogicalKeyboardKey.arrowRight): () =>
            widget.onSeekBy(1),
        const SingleActivator(LogicalKeyboardKey.arrowLeft, shift: true): () =>
            widget.onSeekBy(-5),
        const SingleActivator(LogicalKeyboardKey.arrowRight, shift: true): () =>
            widget.onSeekBy(5),
      },
      child: Focus(
        autofocus: true,
        child: MouseRegion(
          onHover: (_) => _show(),
          child: Listener(
            onPointerDown: (_) => _show(),
            child: ColoredBox(
              color: Colors.black,
              child: ValueListenableBuilder<ProjectPreviewState?>(
                valueListenable: widget.preview,
                builder: (context, preview, _) {
                  final end = preview?.contentEnd?.secondsForDisplay ?? 0;
                  final hasRange = end.isFinite && end > 0;
                  final position = preview?.position.secondsForDisplay ?? 0;
                  final shown = (_scrubSeconds ?? position).clamp(0.0, end);
                  final rate = preview?.sequenceFrameRate;
                  final endLabel = hasRange && rate != null
                      ? _timecodeLabel(end, rate.numerator / rate.denominator)
                      : clockLabel(hasRange ? end : 0);
                  return Stack(
                    fit: StackFit.expand,
                    children: [
                      if (preview != null && preview.width > 0)
                        Center(
                          child: FittedBox(
                            fit: BoxFit.contain,
                            child: SizedBox(
                              width: preview.width.toDouble(),
                              height: preview.height.toDouble(),
                              child: Texture(
                                key: ValueKey(
                                  'fullscreen-viewer-texture-${widget.textureId}',
                                ),
                                textureId: widget.textureId,
                                filterQuality: FilterQuality.medium,
                              ),
                            ),
                          ),
                        )
                      else
                        const Center(
                          child: Text(
                            'Preparing preview…',
                            style: TextStyle(color: OrColors.textSecondary),
                          ),
                        ),
                      Positioned.fill(
                        child: GestureDetector(
                          behavior: HitTestBehavior.opaque,
                          onTap: _toggleControls,
                        ),
                      ),
                      Positioned(
                        top: OrSpacing.x2,
                        right: OrSpacing.x2,
                        child: AnimatedOpacity(
                          opacity: _visible ? 1 : 0,
                          duration: const Duration(milliseconds: 150),
                          child: IgnorePointer(
                            ignoring: !_visible,
                            child: Row(
                              children: [
                                ValueListenableBuilder<ProjectSequenceCanvas?>(
                                  valueListenable: widget.canvas,
                                  builder: (context, canvas, _) =>
                                      _CanvasPicker(
                                        buttonKey: const ValueKey(
                                          'fullscreen-canvas',
                                        ),
                                        labelKey: const ValueKey(
                                          'fullscreen-canvas-label',
                                        ),
                                        canvas: canvas,
                                        enabled: true,
                                        tint: OrColors.text,
                                        background: OrColors.backgroundRaised
                                            .withValues(alpha: 0.9),
                                        onSelected: widget.onCanvas,
                                      ),
                                ),
                                IconButton(
                                  key: const ValueKey(
                                    'preview-fullscreen-close',
                                  ),
                                  tooltip: 'Exit full screen',
                                  onPressed: widget.onClose,
                                  style: IconButton.styleFrom(
                                    backgroundColor: OrColors.backgroundRaised
                                        .withValues(alpha: 0.9),
                                    foregroundColor: OrColors.text,
                                  ),
                                  icon: const Icon(Icons.close_fullscreen),
                                ),
                              ],
                            ),
                          ),
                        ),
                      ),
                      Positioned(
                        left: 0,
                        right: 0,
                        bottom: 0,
                        child: AnimatedOpacity(
                          opacity: _visible ? 1 : 0,
                          duration: const Duration(milliseconds: 150),
                          child: IgnorePointer(
                            ignoring: !_visible,
                            child: Container(
                              decoration: BoxDecoration(
                                gradient: LinearGradient(
                                  begin: Alignment.topCenter,
                                  end: Alignment.bottomCenter,
                                  colors: [
                                    Colors.transparent,
                                    Colors.black.withValues(alpha: 0.88),
                                  ],
                                ),
                              ),
                              padding: const EdgeInsets.fromLTRB(
                                OrSpacing.x4,
                                OrSpacing.x8,
                                OrSpacing.x4,
                                OrSpacing.x3,
                              ),
                              child: Column(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  SliderTheme(
                                    data: SliderTheme.of(context).copyWith(
                                      trackHeight: 3,
                                      activeTrackColor: OrColors.selection,
                                      inactiveTrackColor: OrColors.borderStrong,
                                      thumbColor: OrColors.text,
                                      overlayShape:
                                          const RoundSliderOverlayShape(
                                            overlayRadius: 12,
                                          ),
                                    ),
                                    child: Slider(
                                      key: const ValueKey('fullscreen-scrub'),
                                      min: 0,
                                      max: hasRange ? end : 1,
                                      value: hasRange ? shown : 0,
                                      onChangeStart: hasRange
                                          ? (_) {
                                              _hideTimer?.cancel();
                                              setState(() => _scrubbing = true);
                                            }
                                          : null,
                                      onChanged: hasRange
                                          ? (seconds) {
                                              setState(() {
                                                _scrubbing = true;
                                                _scrubSeconds = seconds;
                                              });
                                              widget.onScrub(seconds);
                                            }
                                          : null,
                                      onChangeEnd: hasRange
                                          ? (seconds) {
                                              setState(() {
                                                _scrubbing = false;
                                                _scrubSeconds = null;
                                              });
                                              widget.onScrubEnd(seconds);
                                              _scheduleHide();
                                            }
                                          : null,
                                    ),
                                  ),
                                  Row(
                                    children: [
                                      _fullscreenButton(
                                        key: const ValueKey(
                                          'fullscreen-skip-back',
                                        ),
                                        tooltip: 'Back 5 seconds',
                                        icon: Icons.replay_5,
                                        onPressed: () => widget.onSeekBy(-5),
                                      ),
                                      _fullscreenButton(
                                        key: const ValueKey('fullscreen-play'),
                                        tooltip: preview?.playing == true
                                            ? 'Pause (Space)'
                                            : 'Play (Space)',
                                        icon: preview?.playing == true
                                            ? Icons.pause
                                            : Icons.play_arrow,
                                        iconSize: 26,
                                        onPressed: widget.onPlayPause,
                                      ),
                                      _fullscreenButton(
                                        key: const ValueKey(
                                          'fullscreen-skip-forward',
                                        ),
                                        tooltip: 'Forward 5 seconds',
                                        icon: Icons.forward_5,
                                        onPressed: () => widget.onSeekBy(5),
                                      ),
                                      const SizedBox(width: OrSpacing.x3),
                                      Flexible(
                                        child: Text(
                                          preview == null
                                              ? '00:00:00:00'
                                              : _timeFeedback(
                                                  preview.position,
                                                  preview.presentedTime,
                                                  preview.sequenceFrameRate,
                                                ),
                                          key: const ValueKey(
                                            'fullscreen-time-readout',
                                          ),
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                          style: const TextStyle(
                                            color: Colors.white,
                                            fontFamily: 'monospace',
                                            fontSize: OrType.metadataSize,
                                          ),
                                        ),
                                      ),
                                      Text(
                                        hasRange ? ' / $endLabel' : '',
                                        style: const TextStyle(
                                          color: OrColors.textSecondary,
                                          fontFamily: 'monospace',
                                          fontSize: OrType.tinySize,
                                        ),
                                      ),
                                      const Spacer(),
                                      ValueListenableBuilder<double>(
                                        valueListenable: widget.speed,
                                        builder: (context, speed, _) =>
                                            PopupMenuButton<double>(
                                              key: const ValueKey(
                                                'fullscreen-speed',
                                              ),
                                              tooltip: 'Playback speed',
                                              onSelected: widget.onSpeed,
                                              itemBuilder: (context) => [
                                                for (final option
                                                    in _previewSpeeds)
                                                  PopupMenuItem<double>(
                                                    key: ValueKey(
                                                      'fullscreen-speed-${_speedLabel(option)}',
                                                    ),
                                                    value: option,
                                                    child: Row(
                                                      mainAxisSize:
                                                          MainAxisSize.min,
                                                      children: [
                                                        SizedBox(
                                                          width: 20,
                                                          child: option == speed
                                                              ? const Icon(
                                                                  Icons.check,
                                                                  size: 16,
                                                                  color: OrColors
                                                                      .selection,
                                                                )
                                                              : null,
                                                        ),
                                                        Text(
                                                          '${_speedLabel(option)}×',
                                                        ),
                                                      ],
                                                    ),
                                                  ),
                                              ],
                                              child: Container(
                                                height: 28,
                                                padding:
                                                    const EdgeInsets.symmetric(
                                                      horizontal: 8,
                                                    ),
                                                decoration: BoxDecoration(
                                                  color: OrColors
                                                      .backgroundRaised
                                                      .withValues(alpha: 0.9),
                                                  border: Border.all(
                                                    color: speed == 1
                                                        ? OrColors.borderStrong
                                                        : OrColors.selection,
                                                  ),
                                                  borderRadius:
                                                      BorderRadius.circular(
                                                        OrRadii.small,
                                                      ),
                                                ),
                                                child: Center(
                                                  child: Text(
                                                    '${_speedLabel(speed)}×',
                                                    key: const ValueKey(
                                                      'fullscreen-speed-label',
                                                    ),
                                                    style: TextStyle(
                                                      color: speed == 1
                                                          ? OrColors.text
                                                          : OrColors.selection,
                                                      fontSize: OrType.tinySize,
                                                    ),
                                                  ),
                                                ),
                                              ),
                                            ),
                                      ),
                                      const SizedBox(width: OrSpacing.x2),
                                      _fullscreenButton(
                                        key: const ValueKey('fullscreen-close'),
                                        tooltip: 'Exit full screen (Esc)',
                                        icon: Icons.close_fullscreen,
                                        onPressed: widget.onClose,
                                      ),
                                    ],
                                  ),
                                ],
                              ),
                            ),
                          ),
                        ),
                      ),
                    ],
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _fullscreenButton({
    required Key key,
    required String tooltip,
    required IconData icon,
    required VoidCallback onPressed,
    double iconSize = 22,
  }) {
    return IconButton(
      key: key,
      tooltip: tooltip,
      onPressed: onPressed,
      style: IconButton.styleFrom(
        minimumSize: const Size.square(44),
        foregroundColor: Colors.white,
      ),
      icon: Icon(icon, size: iconSize),
    );
  }
}
