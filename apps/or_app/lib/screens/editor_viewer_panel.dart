import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:or_viewer_texture/or_viewer_texture.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../project/project_gateway.dart';
import '../widgets/or_widgets.dart';
import 'editor_frame_rate.dart';
import 'editor_labels.dart';
import 'editor_panel_header.dart';
import 'editor_rational_time.dart';
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
      if (!_isCurrent(epoch)) return;
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

  Future<void> _setFrameRate(ProjectRationalRate? rate) async {
    final project = widget.project;
    if (project == null) return;
    await _run((gateway, session) async {
      final result = await gateway.setTimelineSequenceFrameRate(
        session,
        project,
        rate,
      );
      if (!result.succeeded) {
        throw ProjectGatewayException(result.errorCode, result.message);
      }
      return gateway.previewState(session);
    });
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
                  if (widget.selectedVisualSettings != null &&
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
    final rulerMaximum = hasRange ? endSeconds : 1.0;
    final sliderValue = (shownPosition?.secondsForDisplay ?? 0)
        .clamp(0.0, rulerMaximum)
        .toDouble();

    return ColoredBox(
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
              EditorPanelHeader(
                title: 'Viewer',
                // The short panel keeps two pixels for the surface's remainder: with
                // the strip at its minimum the sum came to one pixel more than the
                // panel, and a one-pixel overflow is still a clipped control.
                height: shortPanel ? 32 : (widget.compact ? 34 : 38),
                trailing: OrBadge(
                  preview == null || preview.width == 0
                      ? 'Preview'
                      : '${preview.width} × ${preview.height}',
                ),
              ),
              Expanded(child: _buildViewerSurface()),
              Container(
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
                    Row(
                      children: [
                        IconButton(
                          key: const ValueKey('preview-previous-frame'),
                          tooltip: 'Previous frame',
                          visualDensity: VisualDensity.compact,
                          onPressed: canStep
                              ? () => unawaited(
                                  _run(
                                    (gateway, session) => gateway.previewStep(
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
                          tooltip: preview?.playing == true ? 'Pause' : 'Play',
                          visualDensity: VisualDensity.compact,
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
                          visualDensity: VisualDensity.compact,
                          onPressed: canStep
                              ? () => unawaited(
                                  _run(
                                    (gateway, session) => gateway.previewStep(
                                      session,
                                      ProjectPreviewFrameStep.next,
                                    ),
                                  ),
                                )
                              : null,
                          icon: const Icon(Icons.skip_next_outlined, size: 18),
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
                                _connected && widget.project != null && !_busy,
                            onSelected: (choice) =>
                                unawaited(_setFrameRate(choice.rate)),
                            itemBuilder: (context) => [
                              for (final choice in _rateChoices)
                                PopupMenuItem(
                                  value: choice,
                                  child: Text(choice.label),
                                ),
                            ],
                            child: Padding(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 6,
                              ),
                              child: Text(
                                frameRate == null
                                    ? 'Set frame rate'
                                    : frameRateLabel(frameRate),
                                key: const ValueKey('preview-rate-label'),
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
                      ],
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
