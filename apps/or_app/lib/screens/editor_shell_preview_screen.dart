import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:or_viewer_texture/or_viewer_texture.dart';

import '../design/or_colors.dart';
import '../design/or_typography.dart';
import '../design/or_spacing.dart';
import '../project/motion_scene_snapshot.dart';
import '../project/project_gateway.dart';
import 'viewer_transform_overlay.dart';
import '../widgets/or_widgets.dart';

class EditorShellPreviewScreen extends StatefulWidget {
  const EditorShellPreviewScreen({
    super.key,
    this.isProjectWorkspace = false,
    this.project,
    this.projectGateway,
    this.projectSession,
    this.previewSeekPosition,
    this.onPreviewSeekConsumed,
    this.onSetSequenceFrameRate,
    this.notice,
    this.busy = false,
    this.mediaPage,
    this.mediaPreviews = const {},
    this.mediaLoading = false,
    this.mediaLoadingMore = false,
    this.mediaLoadError,
    this.timelineTracks,
    this.timelineClipPages = const {},
    this.timelineMarkerPage,
    this.timelineLoadingMoreTracks = const {},
    this.timelineLoadingMoreMarkers = false,
    this.timelineLoading = false,
    this.timelineLoadError,
    this.onImportMedia,
    this.onImportMediaFolder,
    this.onLoadMoreMedia,
    this.onRefreshMedia,
    this.onRemoveMedia,
    this.onRelinkMedia,
    this.onAddVideoTrack,
    this.onAddAudioTrack,
    this.onAddTextTrack,
    this.onAddCaptionTrack,
    this.onOpenMotion,
    this.onCopyClip,
    this.onPasteClip,
    this.motionScene,
    this.onInsertMotionScene,
    this.onRerenderMotionScene,
    this.onImportCaptions,
    this.onExportCaptions,
    this.onGenerateCaptions,
    this.onCleanUpAudio,
    this.onRemoveTimelineTrack,
    this.onSetTimelineTrackState,
    this.onMoveTimelineTrack,
    this.onLoadMoreTimelineClips,
    this.onLoadMoreTimelineMarkers,
    this.onRefreshTimeline,
    this.onAddMediaToTimeline,
    this.onInsertTimelineTextClip,
    this.onUpdateTimelineTextClip,
    this.onMoveTimelineClip,
    this.onDuplicateTimelineClip,
    this.onResolveTimelineSnap,
    this.onDeleteTimelineClip,
    this.onTrimTimelineClip,
    this.onSplitTimelineClip,
    this.onSlipTimelineClip,
    this.onRollTimelineClip,
    this.onSlideTimelineClip,
    this.onRippleDeleteTimelineClip,
    this.onAddTimelineMarker,
    this.onMoveTimelineMarker,
    this.onRenameTimelineMarker,
    this.onDeleteTimelineMarker,
    this.onUpdateTimelineClipVisualSettings,
    this.onUpdateTimelineClipAudioSettings,
    this.onUpdateTimelineClipPlaybackRate,
    this.onSetTimelineClipReversed,
    this.onSave,
    this.onRename,
    this.onUndo,
    this.onRedo,
    this.onClose,
    this.onDiscardRecovery,
  });

  final bool isProjectWorkspace;
  final ProjectReadModel? project;
  final ProjectGateway? projectGateway;
  final ProjectSessionHandle? projectSession;
  final ProjectRationalTime? previewSeekPosition;
  final ValueChanged<ProjectRationalRate?>? onSetSequenceFrameRate;
  final VoidCallback? onPreviewSeekConsumed;
  final String? notice;
  final bool busy;
  final ProjectMediaPage? mediaPage;
  final Map<String, ProjectMediaPreview> mediaPreviews;
  final bool mediaLoading;
  final bool mediaLoadingMore;
  final String? mediaLoadError;
  final ProjectTimelineTracks? timelineTracks;
  final Map<String, ProjectTimelineClipPage> timelineClipPages;
  final ProjectTimelineMarkerPage? timelineMarkerPage;
  final Set<String> timelineLoadingMoreTracks;
  final bool timelineLoadingMoreMarkers;
  final bool timelineLoading;
  final String? timelineLoadError;
  final VoidCallback? onImportMedia;
  final VoidCallback? onImportMediaFolder;
  final VoidCallback? onLoadMoreMedia;
  final VoidCallback? onRefreshMedia;
  final ValueChanged<ProjectMediaItem>? onRemoveMedia;
  final ValueChanged<ProjectMediaItem>? onRelinkMedia;
  final VoidCallback? onAddVideoTrack;
  final VoidCallback? onAddAudioTrack;
  final VoidCallback? onAddTextTrack;
  final VoidCallback? onAddCaptionTrack;
  final VoidCallback? onOpenMotion;
  final void Function(
    ProjectReadModel project,
    ProjectTimelineTrack track,
    ProjectTimelineClip clip,
  )?
  onCopyClip;
  final void Function(
    ProjectReadModel project,
    ProjectTimelineTrack track,
    ProjectRationalTime position,
  )?
  onPasteClip;
  /// The scene this session last rendered, so the Motion tab can report the
  /// file it produced instead of forgetting the moment the dialog closed.
  final MotionSceneSnapshot? motionScene;
  final Future<void> Function()? onInsertMotionScene;
  final Future<void> Function()? onRerenderMotionScene;
  final VoidCallback? onImportCaptions;
  final VoidCallback? onExportCaptions;
  final Future<void> Function(ProjectMediaItem media)? onGenerateCaptions;
  final Future<void> Function(ProjectMediaItem media)? onCleanUpAudio;
  final Future<void> Function(ProjectReadModel, ProjectTimelineTrack)?
  onRemoveTimelineTrack;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineTrackState,
  )?
  onSetTimelineTrackState;
  final Future<void> Function(ProjectReadModel, ProjectTimelineTrack, int)?
  onMoveTimelineTrack;
  final ValueChanged<String>? onLoadMoreTimelineClips;
  final VoidCallback? onLoadMoreTimelineMarkers;
  final VoidCallback? onRefreshTimeline;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectMediaItem media,
    String trackId,
    ProjectRationalTime timelineStart,
    ProjectRationalTime sourceStart,
    ProjectRationalTime duration,

    /// `insert` or `overwrite`; chosen in the insert dialog, sent with the
    /// command, and never stored in the project.
    String mode,
  )?
  onAddMediaToTimeline;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectRationalTime,
    ProjectRationalTime,
    ProjectTimelineTextContent,
  )?
  onInsertTimelineTextClip;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineClip,
    ProjectRationalTime,
    ProjectTimelineTextContent,
  )?
  onUpdateTimelineTextClip;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectTimelineClip clip,
    String trackId,
    ProjectRationalTime timelineStart,
  )?
  onMoveTimelineClip;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineClip,
  )?
  onDuplicateTimelineClip;
  final Future<ProjectTimelineSnapResult> Function(
    ProjectReadModel project,
    ProjectTimelineSnapOperation operation,
    String clipId,
    String? targetTrackId,
    ProjectRationalTime targetTime,
  )?
  onResolveTimelineSnap;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectTimelineClip clip,
  )?
  onDeleteTimelineClip;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectTimelineClip clip,
    ProjectTimelineTrimEdge edge,
    ProjectRationalTime timelineTime,
    bool ripple,
  )?
  onTrimTimelineClip;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectTimelineClip clip,
    ProjectRationalTime timelineTime,
  )?
  onSplitTimelineClip;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onSlipTimelineClip;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onRollTimelineClip;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onSlideTimelineClip;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectTimelineClip clip,
  )?
  onRippleDeleteTimelineClip;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectRationalTime timelineTime,
    String label,
  )?
  onAddTimelineMarker;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectTimelineMarker marker,
    ProjectRationalTime timelineTime,
  )?
  onMoveTimelineMarker;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectTimelineMarker marker,
    String label,
  )?
  onRenameTimelineMarker;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectTimelineMarker marker,
  )?
  onDeleteTimelineMarker;
  final Future<ProjectReadModel?> Function(
    ProjectReadModel project,
    String trackId,
    String clipId,
    ProjectTimelineVisualSettings settings,
  )?
  onUpdateTimelineClipVisualSettings;
  final Future<ProjectReadModel?> Function(
    ProjectReadModel project,
    String trackId,
    String clipId,
    ProjectTimelineAudioSettings settings,
  )?
  onUpdateTimelineClipAudioSettings;
  final Future<ProjectReadModel?> Function(
    ProjectReadModel project,
    String trackId,
    String clipId,
    ProjectRationalRate rate,
  )?
  onUpdateTimelineClipPlaybackRate;
  final Future<ProjectReadModel?> Function(
    ProjectReadModel expected,
    String trackId,
    String clipId,
    bool reversed,
  )?
  onSetTimelineClipReversed;
  final VoidCallback? onSave;
  final VoidCallback? onRename;
  final VoidCallback? onUndo;
  final VoidCallback? onRedo;
  final VoidCallback? onClose;
  final VoidCallback? onDiscardRecovery;

  @override
  State<EditorShellPreviewScreen> createState() =>
      EditorShellPreviewScreenState();
}

class _EditorTool {
  const _EditorTool(this.label, this.icon);

  final String label;
  final IconData icon;
}

// Only tools with a working panel are listed. The previous rail offered
// Templates, Shapes, Effects, Transitions, Filters, AI and More, every one of
// which answered "Unavailable in this Developer Preview"; a dead control costs
// more trust than a short rail. Text/Audio/Motion open real actions, and the
// remaining editing surfaces live in the inspector and the timeline toolbar.
const _editorTools = [
  _EditorTool('Media', Icons.perm_media_outlined),
  _EditorTool('Text', Icons.text_fields_outlined),
  _EditorTool('Audio', Icons.graphic_eq_outlined),
  _EditorTool('Motion', Icons.animation_outlined),
];

class EditorShellPreviewScreenState extends State<EditorShellPreviewScreen> {
  // The keyboard layer sits above both panels, so a command stays reachable
  // from whichever panel holds focus, while each command still runs through
  // the panel that owns the state it changes.
  final GlobalKey<_ViewerPanelState> _viewerKey = GlobalKey(
    debugLabel: 'editor viewer',
  );
  final GlobalKey<_TimelinePanelState> _timelineKey = GlobalKey(
    debugLabel: 'editor timeline',
  );

  String _selectedTool = 'Media';
  String? _selectedTrackId;
  String? _selectedClipId;
  ProjectTimelineTrackKind? _selectedTrackKind;
  bool _snapEnabled = true;

  /// Snapping is a mode, so it needs a verb the palette and menus can call,
  /// not only the chip in the timeline row.
  void toggleSnapping() => setState(() => _snapEnabled = !_snapEnabled);
  bool _mobileSheetRefreshScheduled = false;
  late final ValueNotifier<ProjectPreviewState?> _previewState;
  final ValueNotifier<int> _mobileSheetRevision = ValueNotifier(0);

  @override
  void initState() {
    super.initState();
    _previewState = ValueNotifier(null);
  }

  @override
  void didUpdateWidget(covariant EditorShellPreviewScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    _scheduleMobileSheetRefresh();
    final projectIdentityChanged =
        oldWidget.project?.projectId != widget.project?.projectId ||
        oldWidget.project?.projectInstanceId !=
            widget.project?.projectInstanceId ||
        oldWidget.projectSession != widget.projectSession;
    if (projectIdentityChanged) {
      _snapEnabled = true;
      _selectedTrackId = null;
      _selectedClipId = null;
      _selectedTrackKind = null;
    }
    if (projectIdentityChanged ||
        oldWidget.project?.revision != widget.project?.revision) {
      _previewState.value = null;
    }
  }

  void _scheduleMobileSheetRefresh() {
    if (_mobileSheetRefreshScheduled) return;
    _mobileSheetRefreshScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _mobileSheetRefreshScheduled = false;
      if (mounted) _mobileSheetRevision.value++;
    });
  }

  @override
  void dispose() {
    _mediaSearchController.dispose();
    _previewState.dispose();
    _mobileSheetRevision.dispose();
    super.dispose();
  }

  void _onPreviewStateChanged(ProjectPreviewState state) {
    final previous = _previewState.value;
    _previewState.value = state;
    if (mounted &&
        (previous == null || previous.contentEnd != state.contentEnd)) {
      setState(() {});
    }
  }

  void _onTimelineClipSelectionChanged(
    String? trackId,
    String? clipId,
    ProjectTimelineTrackKind? kind,
  ) {
    if (_selectedTrackId == trackId &&
        _selectedClipId == clipId &&
        _selectedTrackKind == kind) {
      return;
    }
    setState(() {
      _selectedTrackId = trackId;
      _selectedClipId = clipId;
      _selectedTrackKind = kind;
    });
    unawaited(_refreshSelectedVisualSettings());
  }

  /// Transform of the selected visual clip, kept in step with the selection so
  /// the viewer can draw handles without asking the inspector.
  /// Blade mode keeps the pointer a cutting tool until the owner leaves it,
  /// which is how every desktop editor lets you make ten cuts without
  /// re-arming a command for each one.
  bool _bladeMode = false;

  /// Owns the media search field's text so a keystroke re-filters the library
  /// without rebuilding the editor.
  final TextEditingController _mediaSearchController = TextEditingController();
  ProjectTimelineVisualSettings? _selectedVisualSettings;
  int _visualSettingsGeneration = 0;
  /// Serializes viewer gestures and nudges; see [_queueSelectedTransform].
  Future<void> _transformQueue = Future<void>.value();
  int _transformCommitSeq = 0;

  Future<void> _refreshSelectedVisualSettings() async {
    final generation = ++_visualSettingsGeneration;
    final project = widget.project;
    final gateway = widget.projectGateway;
    final session = widget.projectSession;
    final trackId = _selectedTrackId;
    final clipId = _selectedClipId;
    final kind = _selectedTrackKind;
    final isVisual =
        kind == ProjectTimelineTrackKind.video ||
        kind == ProjectTimelineTrackKind.text ||
        kind == ProjectTimelineTrackKind.caption;
    if (!widget.isProjectWorkspace ||
        !isVisual ||
        project == null ||
        gateway == null ||
        session == null ||
        trackId == null ||
        clipId == null) {
      if (_selectedVisualSettings != null) {
        setState(() => _selectedVisualSettings = null);
      }
      return;
    }
    try {
      final settings = await gateway.getTimelineClipVisualSettings(
        session,
        project,
        trackId: trackId,
        clipId: clipId,
      );
      if (!mounted || generation != _visualSettingsGeneration) return;
      setState(() => _selectedVisualSettings = settings);
    } on ProjectGatewayException {
      // A read that raced a revision bump is not a deselection: dropping the
      // settings here put the handles away mid-nudge and made the next nudge a
      // no-op. The next successful read corrects the value.
      if (!mounted || generation != _visualSettingsGeneration) return;
    } catch (_) {
      if (!mounted || generation != _visualSettingsGeneration) return;
    }
  }

  /// Moves the selected clip by one step without a pointer. The step is in
  /// milli-canvas units, so it is the same field the viewer handles write.
  Future<void> nudgeSelectedTransform(double dx, double dy) async {
    final current = _selectedVisualSettings;
    if (current == null) return;
    final next = withTransform(
      current,
      xMilliCanvas: current.xMilliCanvas + dx,
      yMilliCanvas: current.yMilliCanvas + dy,
    );
    setState(() => _selectedVisualSettings = next);
    await _commitSelectedTransform(next);
  }

  /// One committed command per viewer gesture.
  Future<void> _commitSelectedTransform(
    ProjectTimelineVisualSettings next,
  ) => _queueSelectedTransform(next);

  /// Repeating a nudge faster than a command round-trips used to send a second
  /// command built on the revision the first one had already replaced, so the
  /// host rejected it and three presses moved the clip once. Commands are
  /// serialized here: each waits for the previous one before it is built, and
  /// only the last one refreshes the panel, so the visible position is always
  /// the position the host holds.
  Future<void> _queueSelectedTransform(ProjectTimelineVisualSettings next) {
    final chained = _transformQueue.then((_) => _sendSelectedTransform(next));
    _transformQueue = chained.then((_) {}, onError: (_) {});
    return chained;
  }

  Future<void> _sendSelectedTransform(ProjectTimelineVisualSettings next) async {
    final project = widget.project;
    final trackId = _selectedTrackId;
    final clipId = _selectedClipId;
    final update = widget.onUpdateTimelineClipVisualSettings;
    if (project == null || trackId == null || clipId == null || update == null) {
      return;
    }
    final seq = ++_transformCommitSeq;
    final result = await update(project, trackId, clipId, next);
    if (!mounted || seq != _transformCommitSeq) return;
    // The commit is the newest truth we hold: paint it immediately so the
    // handles stay where the hand left them, then let a re-read correct any
    // value the host normalized.
    setState(() => _selectedVisualSettings = next);
    if (result != null) await _refreshSelectedVisualSettings();
  }

  ProjectTimelineTrack? get _selectedInspectorTrack {
    final selectedId = _selectedTrackId;
    if (selectedId == null) return null;
    for (final track
        in widget.timelineTracks?.items ?? const <ProjectTimelineTrack>[]) {
      if (track.trackId == selectedId) return track;
    }
    return null;
  }

  ProjectTimelineClip? get _selectedInspectorClip {
    final selectedId = _selectedClipId;
    final project = widget.project;
    if (selectedId == null || project == null) return null;
    for (final page in widget.timelineClipPages.values) {
      if (page.projectId != project.projectId ||
          page.projectInstanceId != project.projectInstanceId ||
          page.projectRevision != project.revision) {
        continue;
      }
      for (final clip in page.items) {
        if (clip.clipId == selectedId) return clip;
      }
    }
    return null;
  }

  // The shell's Shortcuts map drives these, because that map is the layer
  // that receives keys; each command still runs through the panel that owns
  // the state it changes.
  void togglePlayPause() => _viewerKey.currentState?.togglePlayPause();

  void stepFrame(bool backwards) =>
      _viewerKey.currentState?.stepFrame(backwards);

  void seekBy(double seconds) => _viewerKey.currentState?.seekBy(seconds);

  void zoomTimelineBy(double factor) => _timelineKey.currentState?.changeZoom(
    (_timelineKey.currentState?.timelineZoom ?? 1) * factor,
  );

  void fitTimeline() => _timelineKey.currentState?.fitToView();

  void duplicateSelectedClip() =>
      _timelineKey.currentState?.duplicateSelectedClip();

  void copySelectedClip() => _timelineKey.currentState?.copySelectedClip();

  void pasteClipboardClip() => _timelineKey.currentState?.pasteClipboardClip();

  void confirmDeleteSelectedClip() =>
      _timelineKey.currentState?.confirmDeleteSelectedClip();

  void clearSelection() => _timelineKey.currentState?.clearSelection();

  void splitSelectedClip() =>
      _timelineKey.currentState?.splitSelectedClipAtPlayhead();

  void rippleDeleteSelectedClip() =>
      _timelineKey.currentState?.rippleDeleteSelectedClip();

  /// The left panel's category, so number keys reach the same state the tab
  /// strip owns.
  void setBladeMode(bool enabled) {
    if (_bladeMode == enabled) return;
    setState(() => _bladeMode = enabled);
  }

  void toggleBladeMode() => setBladeMode(!_bladeMode);

  /// Reverse the selected media clip, wherever the owner invoked it from.
  ///
  /// The clip object lives in the timeline page, not in this state, so the lookup
  /// is the same one the panels use; a missing clip is a no-op rather than an
  /// error, because the palette can outlive a selection change by one frame.
  void toggleSelectedClipReverse() {
    final clipId = _selectedClipId;
    final onSet = widget.onSetTimelineClipReversed;
    final project = widget.project;
    if (clipId == null || onSet == null || project == null) return;
    for (final entry in widget.timelineClipPages.entries) {
      for (final clip in entry.value.items) {
        if (clip.clipId != clipId) continue;
        if (clip.contentKind != ProjectTimelineClipContentKind.media) return;
        unawaited(onSet(project, entry.key, clip.clipId, !clip.reversed));
        return;
      }
    }
  }

  void selectTool(String tool) {
    if (_selectedTool == tool) return;
    setState(() => _selectedTool = tool);
  }

  void addMarkerAtPlayhead() => _showAddTimelineMarker();

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      key: const ValueKey('editor-shell-preview'),
      builder: (context, constraints) {
        final compact = OrBreakpoints.isCompact(constraints.maxWidth);
        return ColoredBox(
          color: OrColors.background,
          child: Column(
            children: [
              // The project's name, save state, history controls and project
              // menu live in the application bar; a second workspace title row
              // repeated the same facts and cost 52 px of editing space
              // (docs/EDITOR_WORKSPACE_DESIGN.md §3).
              if (!widget.isProjectWorkspace) const _EditorPreviewHeader(),
              if (widget.notice != null)
                _ProjectNotice(
                  message: widget.notice!,
                  onDiscardRecovery: widget.onDiscardRecovery,
                ),
              Expanded(
                child: compact
                    ? _compactLayout()
                    : _desktopLayout(
                        OrBreakpoints.isWide(constraints.maxWidth),
                      ),
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _desktopLayout(bool wide) {
    return Column(
      children: [
        Expanded(
          flex: 3,
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              SizedBox(
                width: wide ? 320 : 264,
                child: _EditorToolPanel(
                  selectedTool: _selectedTool,
                  onSelectedTool: (picked) =>
                      setState(() => _selectedTool = picked),
                  mediaSearchController: _mediaSearchController,
                  isProjectWorkspace: widget.isProjectWorkspace,
                  project: widget.project,
                  previewState: _previewState.value,
                  mediaPage: widget.mediaPage,
                  timelineTracks: widget.timelineTracks,
                  mediaPreviews: widget.mediaPreviews,
                  mediaLoading: widget.mediaLoading,
                  mediaLoadingMore: widget.mediaLoadingMore,
                  mediaLoadError: widget.mediaLoadError,
                  onAddMediaToTimeline:
                      widget.isProjectWorkspace && !widget.busy
                      ? widget.onAddMediaToTimeline
                      : null,
                  onImportMedia: widget.busy ? null : widget.onImportMedia,
                  onImportMediaFolder: widget.busy
                      ? null
                      : widget.onImportMediaFolder,
                  onLoadMoreMedia: widget.onLoadMoreMedia,
                  onRefreshMedia: widget.onRefreshMedia,
                  onRemoveMedia: widget.busy ? null : widget.onRemoveMedia,
                  onRelinkMedia: widget.busy ? null : widget.onRelinkMedia,
                  onGenerateCaptions: widget.busy
                      ? null
                      : widget.onGenerateCaptions,
                  onCleanUpAudio: widget.busy ? null : widget.onCleanUpAudio,
                  onAddTextTrack: widget.busy ? null : widget.onAddTextTrack,
                  onAddCaptionTrack: widget.busy
                      ? null
                      : widget.onAddCaptionTrack,
                  onAddAudioTrack: widget.busy ? null : widget.onAddAudioTrack,
                  onImportCaptions: widget.busy
                      ? null
                      : widget.onImportCaptions,
                  onExportCaptions: widget.busy
                      ? null
                      : widget.onExportCaptions,
                  onOpenMotion: widget.busy ? null : widget.onOpenMotion,
                  motionScene: widget.motionScene,
                  onInsertMotionScene: widget.busy
                      ? null
                      : widget.onInsertMotionScene,
                  onRerenderMotionScene: widget.busy
                      ? null
                      : widget.onRerenderMotionScene,
                ),
              ),
              const VerticalDivider(width: 1),
              Expanded(
                child: _ViewerPanel(
                  key: _viewerKey,
                  compact: false,
                  gateway: widget.projectGateway,
                  session: widget.projectSession,
                  project: widget.project,
                  previewSeekPosition: widget.previewSeekPosition,
                  onPreviewSeekConsumed: widget.onPreviewSeekConsumed,
                  onStateChanged: _onPreviewStateChanged,
                  selectedVisualSettings: _selectedVisualSettings,
                  onCommitTransform: _commitSelectedTransform,
                  hasTimelineContent: widget.timelineClipPages.values.any(
                    (page) => page.totalCount > 0,
                  ),
                ),
              ),
              const VerticalDivider(width: 1),
              SizedBox(
                width: wide ? 300 : 264,
                child: _InspectorPanel(
                  isProjectWorkspace: widget.isProjectWorkspace,
                  project: widget.project,
                  gateway: widget.projectGateway,
                  session: widget.projectSession,
                  trackId: _selectedTrackId,
                  clipId: _selectedClipId,
                  clip: _selectedInspectorClip,
                  isVisualTrack:
                      _selectedTrackKind == ProjectTimelineTrackKind.video ||
                      _selectedTrackKind == ProjectTimelineTrackKind.text ||
                      _selectedTrackKind == ProjectTimelineTrackKind.caption,
                  isAudioTrack:
                      _selectedTrackKind == ProjectTimelineTrackKind.audio,
                  trackLocked: _selectedInspectorTrack?.state.locked ?? true,
                  busy: widget.busy,
                  sequenceFrameRate: _previewState.value?.sequenceFrameRate,
                  contentEnd: _previewState.value?.contentEnd,
                  onSetSequenceFrameRate: widget.onSetSequenceFrameRate,
                trackCount: widget.timelineTracks?.items.length ?? 0,
                  clipCount: widget.timelineClipPages.values.fold<int>(
                    0,
                    (total, page) => total + page.totalCount,
                  ),
                  onUpdate: widget.onUpdateTimelineClipVisualSettings,
                  onUpdateAudio: widget.onUpdateTimelineClipAudioSettings,
                  onUpdatePlaybackRate: widget.onUpdateTimelineClipPlaybackRate,
                  onSetReversed: widget.onSetTimelineClipReversed,
                ),
              ),
            ],
          ),
        ),
        const _ResizeDivider(horizontal: true),
        Expanded(
          flex: 2,
          child: _TimelinePanel(
            key: _timelineKey,
            compact: false,
            previewState: _previewState,
            isProjectWorkspace: widget.isProjectWorkspace,
            onCopyClip: widget.onCopyClip,
            onPasteClip: widget.onPasteClip,
            project: widget.project,
            tracks: widget.timelineTracks,
            clipPages: widget.timelineClipPages,
            markerPage: widget.timelineMarkerPage,
            loadingMoreTracks: widget.timelineLoadingMoreTracks,
            loadingMoreMarkers: widget.timelineLoadingMoreMarkers,
            loading: widget.timelineLoading,
            error: widget.timelineLoadError,
            busy: widget.busy,
            onImportMedia: widget.busy ? null : widget.onImportMedia,
            onAddVideoTrack: widget.onAddVideoTrack,
            onAddAudioTrack: widget.onAddAudioTrack,
            onAddTextTrack: widget.onAddTextTrack,
            onAddCaptionTrack: widget.onAddCaptionTrack,
            onRemoveTrack: widget.onRemoveTimelineTrack,
            onSetTrackState: widget.onSetTimelineTrackState,
            onMoveTrack: widget.onMoveTimelineTrack,
            onLoadMore: widget.onLoadMoreTimelineClips,
            onLoadMoreMarkers: widget.onLoadMoreTimelineMarkers,
            onRefresh: widget.onRefreshTimeline,
            onMoveClip: widget.onMoveTimelineClip,
            onDuplicateClip: widget.onDuplicateTimelineClip,
            onResolveSnap: widget.onResolveTimelineSnap,
            onDeleteClip: widget.onDeleteTimelineClip,
            onTrimClip: widget.onTrimTimelineClip,
            onSplitClip: widget.onSplitTimelineClip,
            bladeMode: _bladeMode,
            onToggleBladeMode: () => setBladeMode(!_bladeMode),
            onSlipClip: widget.onSlipTimelineClip,
            onRollClip: widget.onRollTimelineClip,
            onSlideClip: widget.onSlideTimelineClip,
            onRippleDeleteClip: widget.onRippleDeleteTimelineClip,
                  onSetClipReversed: widget.onSetTimelineClipReversed,
            onUpdateTextClip: widget.onUpdateTimelineTextClip,
            onAddMarker: widget.onAddTimelineMarker,
            onMoveMarker: widget.onMoveTimelineMarker,
            onRenameMarker: widget.onRenameTimelineMarker,
            onDeleteMarker: widget.onDeleteTimelineMarker,
            snapEnabled: _snapEnabled,
            onSnapChanged: widget.isProjectWorkspace
                ? (value) => setState(() => _snapEnabled = value)
                : null,
            onImportCaptions: widget.onImportCaptions,
            onExportCaptions: widget.onExportCaptions,
            onAddTitle: () =>
                _showAddTimelineTextClip(ProjectTimelineClipContentKind.text),
            onAddCaption: () => _showAddTimelineTextClip(
              ProjectTimelineClipContentKind.caption,
            ),
            onRequestMarker: widget.isProjectWorkspace
                ? _showAddTimelineMarker
                : null,
            onTimelineEditError: _showTimelineEditError,
            mediaItems: widget.mediaPage?.items ?? const [],
            mediaPreviews: widget.mediaPreviews,
            onAddMediaToTimeline: widget.onAddMediaToTimeline,
            onClipSelectionChanged: _onTimelineClipSelectionChanged,
          ),
        ),
      ],
    );
  }

  Widget _compactLayout() {
    return Column(
      children: [
        Expanded(
          flex: 4,
          child: _ViewerPanel(
            key: _viewerKey,
            compact: true,
            gateway: widget.projectGateway,
            session: widget.projectSession,
            project: widget.project,
            previewSeekPosition: widget.previewSeekPosition,
            onPreviewSeekConsumed: widget.onPreviewSeekConsumed,
            onStateChanged: _onPreviewStateChanged,
          ),
        ),
        Expanded(
          flex: 2,
          child: _TimelinePanel(
            key: _timelineKey,
            compact: true,
            previewState: _previewState,
            isProjectWorkspace: widget.isProjectWorkspace,
            onCopyClip: widget.onCopyClip,
            onPasteClip: widget.onPasteClip,
            project: widget.project,
            tracks: widget.timelineTracks,
            clipPages: widget.timelineClipPages,
            markerPage: widget.timelineMarkerPage,
            loadingMoreTracks: widget.timelineLoadingMoreTracks,
            loadingMoreMarkers: widget.timelineLoadingMoreMarkers,
            loading: widget.timelineLoading,
            error: widget.timelineLoadError,
            busy: widget.busy,
            onImportMedia: widget.busy ? null : widget.onImportMedia,
            onAddVideoTrack: widget.onAddVideoTrack,
            onAddAudioTrack: widget.onAddAudioTrack,
            onAddTextTrack: widget.onAddTextTrack,
            onAddCaptionTrack: widget.onAddCaptionTrack,
            onRemoveTrack: widget.onRemoveTimelineTrack,
            onSetTrackState: widget.onSetTimelineTrackState,
            onMoveTrack: widget.onMoveTimelineTrack,
            onLoadMore: widget.onLoadMoreTimelineClips,
            onLoadMoreMarkers: widget.onLoadMoreTimelineMarkers,
            onRefresh: widget.onRefreshTimeline,
            onMoveClip: widget.onMoveTimelineClip,
            onDuplicateClip: widget.onDuplicateTimelineClip,
            onResolveSnap: widget.onResolveTimelineSnap,
            onDeleteClip: widget.onDeleteTimelineClip,
            onTrimClip: widget.onTrimTimelineClip,
            onSplitClip: widget.onSplitTimelineClip,
            onSlipClip: widget.onSlipTimelineClip,
            onRollClip: widget.onRollTimelineClip,
            onSlideClip: widget.onSlideTimelineClip,
            onRippleDeleteClip: widget.onRippleDeleteTimelineClip,
                  onSetClipReversed: widget.onSetTimelineClipReversed,
            onUpdateTextClip: widget.onUpdateTimelineTextClip,
            onAddMarker: widget.onAddTimelineMarker,
            onMoveMarker: widget.onMoveTimelineMarker,
            onRenameMarker: widget.onRenameTimelineMarker,
            onDeleteMarker: widget.onDeleteTimelineMarker,
            snapEnabled: _snapEnabled,
            onSnapChanged: widget.isProjectWorkspace
                ? (value) => setState(() => _snapEnabled = value)
                : null,
            onImportCaptions: widget.onImportCaptions,
            onExportCaptions: widget.onExportCaptions,
            onAddTitle: () =>
                _showAddTimelineTextClip(ProjectTimelineClipContentKind.text),
            onAddCaption: () => _showAddTimelineTextClip(
              ProjectTimelineClipContentKind.caption,
            ),
            onTimelineEditError: _showTimelineEditError,
            mediaItems: widget.mediaPage?.items ?? const [],
            mediaPreviews: widget.mediaPreviews,
            onAddMediaToTimeline: widget.onAddMediaToTimeline,
            onClipSelectionChanged: _onTimelineClipSelectionChanged,
          ),
        ),
        _MobileToolDock(
          selected: _selectedTool,
          hasClipSelection: _selectedClipId != null,
          isProjectWorkspace: widget.isProjectWorkspace,
          onSelected: _showMobileToolSheet,
        ),
      ],
    );
  }

  Future<void> _showMobileToolSheet(String tool) async {
    setState(() => _selectedTool = tool);
    final height = MediaQuery.of(context).size.height * 0.82;
    await showModalBottomSheet<void>(
      context: context,
      useSafeArea: true,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (sheetContext) => AnimatedBuilder(
        animation: _mobileSheetRevision,
        builder: (context, child) {
          void onClose() => Navigator.of(sheetContext).pop();

          final selectedTrack = _selectedInspectorTrack;
          if (tool == 'Inspector') {
            return SizedBox(
              height: height,
              child: _InspectorPanel(
                isProjectWorkspace: widget.isProjectWorkspace,
                project: widget.project,
                gateway: widget.projectGateway,
                session: widget.projectSession,
                trackId: _selectedTrackId,
                clipId: _selectedClipId,
                clip: _selectedInspectorClip,
                isVisualTrack:
                    _selectedTrackKind == ProjectTimelineTrackKind.video ||
                    _selectedTrackKind == ProjectTimelineTrackKind.text ||
                    _selectedTrackKind == ProjectTimelineTrackKind.caption,
                isAudioTrack:
                    _selectedTrackKind == ProjectTimelineTrackKind.audio,
                trackLocked: selectedTrack?.state.locked ?? true,
                busy: widget.busy,
                sequenceFrameRate: _previewState.value?.sequenceFrameRate,
                contentEnd: _previewState.value?.contentEnd,
                trackCount: widget.timelineTracks?.items.length ?? 0,
                clipCount: widget.timelineClipPages.values.fold<int>(
                  0,
                  (total, page) => total + page.totalCount,
                ),
                onUpdate: widget.onUpdateTimelineClipVisualSettings,
                onUpdateAudio: widget.onUpdateTimelineClipAudioSettings,
                onUpdatePlaybackRate: widget.onUpdateTimelineClipPlaybackRate,
                onClose: onClose,
              ),
            );
          }
          if ((tool == 'Media' ||
                  tool == 'Text' ||
                  tool == 'Audio' ||
                  tool == 'Motion') &&
              widget.isProjectWorkspace) {
            return SizedBox(
              height: height,
              child: _EditorToolPanel(
                selectedTool: tool,
                onSelectedTool: (picked) =>
                    setState(() => _selectedTool = picked),
                mediaSearchController: _mediaSearchController,
                isProjectWorkspace: true,
                project: widget.project,
                previewState: _previewState.value,
                mediaPage: widget.mediaPage,
                timelineTracks: widget.timelineTracks,
                mediaPreviews: widget.mediaPreviews,
                mediaLoading: widget.mediaLoading,
                mediaLoadingMore: widget.mediaLoadingMore,
                mediaLoadError: widget.mediaLoadError,
                onImportMedia: widget.busy ? null : widget.onImportMedia,
                onImportMediaFolder: widget.busy
                    ? null
                    : widget.onImportMediaFolder,
                onLoadMoreMedia: widget.onLoadMoreMedia,
                onRefreshMedia: widget.onRefreshMedia,
                onRemoveMedia: widget.busy ? null : widget.onRemoveMedia,
                onRelinkMedia: widget.busy ? null : widget.onRelinkMedia,
                onGenerateCaptions: widget.busy
                    ? null
                    : widget.onGenerateCaptions,
                onCleanUpAudio: widget.busy ? null : widget.onCleanUpAudio,
                onAddMediaToTimeline: widget.busy
                    ? null
                    : widget.onAddMediaToTimeline,
                onAddTextTrack: widget.busy ? null : widget.onAddTextTrack,
                onAddCaptionTrack: widget.busy
                    ? null
                    : widget.onAddCaptionTrack,
                onAddAudioTrack: widget.busy ? null : widget.onAddAudioTrack,
                onImportCaptions: widget.busy ? null : widget.onImportCaptions,
                onExportCaptions: widget.busy ? null : widget.onExportCaptions,
                onOpenMotion: widget.busy
                    ? null
                    : () {
                        onClose();
                        widget.onOpenMotion?.call();
                      },
                motionScene: widget.motionScene,
                onInsertMotionScene: widget.onInsertMotionScene,
                onRerenderMotionScene: widget.onRerenderMotionScene,
                onClose: onClose,
              ),
            );
          }
          return _UnavailableToolSheet(tool: tool);
        },
      ),
    );
  }

  void _showTimelineEditError(String message) {
    if (!mounted) return;
    final compact = OrBreakpoints.isCompact(MediaQuery.sizeOf(context).width);
    OrSnackBar.show(
      context,
      message,
      bottomInset: compact ? OrSpacing.topBarHeight : 0,
    );
  }

  Future<void> _showAddTimelineMarker() async {
    final project = widget.project;
    final onAdd = widget.onAddTimelineMarker;
    if (!widget.isProjectWorkspace || project == null || onAdd == null) return;
    await _showAddTimelineMarkerDialog(
      context: context,
      project: project,
      onAdd: onAdd,
    );
  }

  Future<void> _showAddTimelineTextClip(
    ProjectTimelineClipContentKind contentKind,
  ) async {
    final project = widget.project;
    final onInsert = widget.onInsertTimelineTextClip;
    if (!widget.isProjectWorkspace ||
        project == null ||
        onInsert == null ||
        widget.busy) {
      return;
    }
    if (contentKind == ProjectTimelineClipContentKind.media) return;
    final trackKind = contentKind == ProjectTimelineClipContentKind.text
        ? ProjectTimelineTrackKind.text
        : ProjectTimelineTrackKind.caption;
    ProjectTimelineTrack? track;
    for (final candidate
        in widget.timelineTracks?.items ?? const <ProjectTimelineTrack>[]) {
      if (candidate.kind == trackKind && !candidate.state.locked) {
        track = candidate;
        break;
      }
    }
    if (track == null) {
      _showTimelineEditError(
        'Add an unlocked ${contentKind == ProjectTimelineClipContentKind.text ? 'text' : 'caption'} track first.',
      );
      return;
    }
    final start =
        _previewState.value?.position ?? ProjectRationalTime(BigInt.zero, 1);
    final edit = await _showTimelineTextClipDialog(
      context: context,
      contentKind: contentKind,
      duration: ProjectRationalTime(BigInt.from(5), 1),
      formatting: ProjectTextFormatting.defaults,
    );
    if (edit != null && mounted) {
      await onInsert(project, track, start, edit.duration, edit.content);
    }
  }
}

class _ProjectNotice extends StatelessWidget {
  const _ProjectNotice({required this.message, this.onDiscardRecovery});

  final String message;
  final VoidCallback? onDiscardRecovery;

  @override
  Widget build(BuildContext context) {
    final staleCheckpoint = message.toLowerCase().startsWith(
      'a stale recovery checkpoint',
    );
    return Container(
      key: const ValueKey('project-notice'),
      width: double.infinity,
      padding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x3,
        vertical: OrSpacing.x2,
      ),
      color: OrColors.surface,
      child: Row(
        children: [
          const Icon(
            Icons.info_outline,
            size: 16,
            color: OrColors.textSecondary,
          ),
          const SizedBox(width: OrSpacing.x2),
          Expanded(
            child: Text(
              message,
              style: const TextStyle(fontSize: OrType.metadataCompactSize),
            ),
          ),
          if (staleCheckpoint && onDiscardRecovery != null)
            TextButton(
              key: const ValueKey('discard-stale-recovery'),
              onPressed: onDiscardRecovery,
              child: const Text('Discard stale checkpoint'),
            ),
        ],
      ),
    );
  }
}

class _EditorPreviewHeader extends StatelessWidget {
  const _EditorPreviewHeader();

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      height: 40,
      padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x3),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: const Row(
        children: [
          Icon(
            Icons.view_quilt_outlined,
            size: 17,
            color: OrColors.textSecondary,
          ),
          SizedBox(width: OrSpacing.x2),
          Expanded(
            child: Text(
              'Editor layout preview',
              style: TextStyle(
                fontSize: OrType.metadataCompactSize,
                fontWeight: FontWeight.w500,
              ),
            ),
          ),
          OrBadge('Non-functional'),
        ],
      ),
    );
  }
}

/// Rail copy. Every listed tool opens a working panel, so the label never
/// claims it is unavailable.
String _toolSemanticsLabel(String label) =>
    label == 'Media' ? 'Media library' : label;

String _toolDescription(String label) => switch (label) {
  'Media' => 'Media library',
  'Text' => 'Titles and captions',
  'Audio' => 'Audio tools',
  'Motion' => 'Illustration video (MotionScene)',
  _ => label,
};

/// The editing tool categories, as one segmented strip at the top of the tool
/// panel. A separate icon rail repeated the same four choices in a second
/// column and cost 68 px of viewer width (docs/EDITOR_WORKSPACE_DESIGN.md §3).
class _ToolTabs extends StatelessWidget {
  const _ToolTabs({
    required this.selected,
    required this.onSelected,
    required this.compact,
  });

  final String selected;
  final ValueChanged<String> onSelected;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    return Container(
      key: const ValueKey('editor-tool-tabs'),
      height: compact ? 38 : 42,
      padding: const EdgeInsets.all(OrSpacing.x2),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Row(
        children: [
          for (final tool in _editorTools)
            Expanded(
              child: Semantics(
                button: true,
                selected: selected == tool.label,
                label: _toolSemanticsLabel(tool.label),
                child: Tooltip(
                  message: _toolDescription(tool.label),
                  waitDuration: const Duration(milliseconds: 600),
                  child: OrFocusRing(
                    radius: OrRadii.small,
                    child: InkWell(
                      key: ValueKey('editor-tool-${_toolKey(tool.label)}'),
                      onTap: () => onSelected(tool.label),
                      borderRadius: BorderRadius.circular(OrRadii.small),
                      child: Container(
                        height: double.infinity,
                        margin: const EdgeInsets.symmetric(horizontal: 1),
                        alignment: Alignment.center,
                        decoration: BoxDecoration(
                          color: selected == tool.label
                              ? OrColors.surface
                              : Colors.transparent,
                          border: Border.all(
                            color: selected == tool.label
                                ? OrColors.selection
                                : Colors.transparent,
                          ),
                          borderRadius: BorderRadius.circular(OrRadii.small),
                        ),
                        child: Text(
                          _shortToolLabel(tool.label),
                          maxLines: 1,
                          textAlign: TextAlign.center,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            color: selected == tool.label
                                ? OrColors.text
                                : OrColors.textMuted,
                            fontSize: OrType.metadataCompactSize,
                            fontWeight: selected == tool.label
                                ? FontWeight.w600
                                : FontWeight.w500,
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _EditorToolPanel extends StatelessWidget {
  const _EditorToolPanel({
    required this.selectedTool,
    required this.onSelectedTool,
    this.mediaSearchController,
    required this.isProjectWorkspace,
    required this.project,
    required this.previewState,
    required this.mediaPage,
    required this.timelineTracks,
    required this.mediaPreviews,
    required this.mediaLoading,
    required this.mediaLoadingMore,
    required this.mediaLoadError,
    required this.onImportMedia,
    required this.onImportMediaFolder,
    required this.onLoadMoreMedia,
    required this.onRefreshMedia,
    required this.onRemoveMedia,
    required this.onRelinkMedia,
    required this.onGenerateCaptions,
    required this.onCleanUpAudio,
    required this.onAddMediaToTimeline,
    this.onAddTextTrack,
    this.onAddCaptionTrack,
    this.onAddAudioTrack,
    this.onImportCaptions,
    this.onExportCaptions,
    this.onOpenMotion,
    this.motionScene,
    this.onInsertMotionScene,
    this.onRerenderMotionScene,
    this.onClose,
  });

  final String selectedTool;
  final ValueChanged<String> onSelectedTool;
  final TextEditingController? mediaSearchController;
  final bool isProjectWorkspace;
  final ProjectReadModel? project;
  final ProjectPreviewState? previewState;
  final ProjectMediaPage? mediaPage;
  final ProjectTimelineTracks? timelineTracks;
  final Map<String, ProjectMediaPreview> mediaPreviews;
  final bool mediaLoading;
  final bool mediaLoadingMore;
  final String? mediaLoadError;
  final VoidCallback? onImportMedia;
  final VoidCallback? onImportMediaFolder;
  final VoidCallback? onLoadMoreMedia;
  final VoidCallback? onRefreshMedia;
  final ValueChanged<ProjectMediaItem>? onRemoveMedia;
  final ValueChanged<ProjectMediaItem>? onRelinkMedia;
  final Future<void> Function(ProjectMediaItem media)? onGenerateCaptions;
  final Future<void> Function(ProjectMediaItem media)? onCleanUpAudio;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectMediaItem media,
    String trackId,
    ProjectRationalTime timelineStart,
    ProjectRationalTime sourceStart,
    ProjectRationalTime duration,
    String mode,
  )?
  onAddMediaToTimeline;
  final VoidCallback? onAddTextTrack;
  final VoidCallback? onAddCaptionTrack;
  final VoidCallback? onAddAudioTrack;
  final VoidCallback? onImportCaptions;
  final VoidCallback? onExportCaptions;
  final VoidCallback? onOpenMotion;
  final MotionSceneSnapshot? motionScene;
  final Future<void> Function()? onInsertMotionScene;
  final Future<void> Function()? onRerenderMotionScene;
  final VoidCallback? onClose;

  @override
  Widget build(BuildContext context) {
    final body = _toolBody(context);
    if (!isProjectWorkspace) return body;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _ToolTabs(
          selected: selectedTool,
          onSelected: onSelectedTool,
          compact: false,
        ),
        Expanded(child: body),
      ],
    );
  }

  Widget _toolBody(BuildContext context) {
    if (selectedTool == 'Media' && isProjectWorkspace) {
      return _MediaLibraryPanel(
        showTitle: false,
        searchController: mediaSearchController,
        page: mediaPage,
        project: project,
        previewState: previewState,
        tracks: timelineTracks,
        previews: mediaPreviews,
        loading: mediaLoading,
        loadingMore: mediaLoadingMore,
        error: mediaLoadError,
        onImport: onImportMedia,
        onImportFolder: onImportMediaFolder,
        onLoadMore: onLoadMoreMedia,
        onRefresh: onRefreshMedia,
        onRemove: onRemoveMedia,
        onRelink: onRelinkMedia,
        onGenerateCaptions: onGenerateCaptions,
        onCleanUpAudio: onCleanUpAudio,
        onAddMediaToTimeline: onAddMediaToTimeline,
        onClose: onClose,
      );
    }

    if (selectedTool == 'Text') {
      return _ToolActionsPanel(
        title: 'Text',
        showTitle: false,
        subtitle:
            'Titles and captions live on their own tracks. Select a clip to '
            'edit its content and formatting in the inspector.',
        actions: [
          _ToolAction(
            keyName: 'tool-add-text-track',
            label: 'Add text track',
            icon: Icons.text_fields_outlined,
            onPressed: onAddTextTrack,
          ),
          _ToolAction(
            keyName: 'tool-add-caption-track',
            label: 'Add caption track',
            icon: Icons.subtitles_outlined,
            onPressed: onAddCaptionTrack,
          ),
          _ToolAction(
            keyName: 'tool-import-captions',
            label: 'Import captions…',
            icon: Icons.file_open_outlined,
            onPressed: onImportCaptions,
          ),
          _ToolAction(
            keyName: 'tool-export-captions',
            label: 'Export captions…',
            icon: Icons.file_download_outlined,
            onPressed: onExportCaptions,
          ),
        ],
      );
    }
    if (selectedTool == 'Audio') {
      return _ToolActionsPanel(
        title: 'Audio',
        showTitle: false,
        subtitle:
            'Audio clips carry gain, pan and fade times. The inspector shows '
            'them in dB, % and seconds for the selected clip.',
        actions: [
          _ToolAction(
            keyName: 'tool-add-audio-track',
            label: 'Add audio track',
            icon: Icons.graphic_eq_outlined,
            onPressed: onAddAudioTrack,
          ),
          _ToolAction(
            keyName: 'tool-import-audio',
            label: 'Import audio or video…',
            icon: Icons.add,
            onPressed: onImportMedia,
          ),
        ],
      );
    }
    if (selectedTool == 'Motion') {
      final scene = motionScene;
      if (scene == null) {
        return _ToolActionsPanel(
          title: 'Motion',
          showTitle: false,
          subtitle:
              'A code file becomes a deterministic MotionScene, rendered locally '
              'and inserted as an ordinary clip that exports with the timeline.',
          actions: [
            _ToolAction(
              keyName: 'tool-open-motion',
              label: 'Create illustration video…',
              icon: Icons.animation_outlined,
              onPressed: onOpenMotion,
            ),
          ],
        );
      }
      return _MotionScenePanel(
        scene: scene,
        onCreate: onOpenMotion,
        onInsert: onInsertMotionScene,
        onRerender: onRerenderMotionScene,
      );
    }

    final message = selectedTool == 'Media'
        ? 'Media tools unavailable in this Developer Preview'
        : 'Unavailable in this Developer Preview';

    return ColoredBox(
      color: OrColors.backgroundRaised,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PanelHeader(title: selectedTool),
          const Divider(height: 1),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.all(OrSpacing.x4),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const OrBadge('Unavailable'),
                  const SizedBox(height: OrSpacing.x3),
                  Text(
                    message,
                    style: const TextStyle(
                      color: OrColors.textSecondary,
                      fontSize: OrType.metadataSize,
                      height: 1.4,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}

/// The scene this session rendered: what it is, what it wrote, and the two
/// things an author does next. A render is a file plus an insert, so the panel
/// keeps both facts visible instead of asking the owner to remember them.
class _MotionScenePanel extends StatelessWidget {
  const _MotionScenePanel({
    required this.scene,
    required this.onCreate,
    required this.onInsert,
    required this.onRerender,
  });

  final MotionSceneSnapshot scene;
  final VoidCallback? onCreate;
  final Future<void> Function()? onInsert;
  final Future<void> Function()? onRerender;

  @override
  Widget build(BuildContext context) {
    final size = scene.bytesWritten >= 1024 * 1024
        ? '${(scene.bytesWritten / (1024 * 1024)).toStringAsFixed(1)} MB'
        : '${(scene.bytesWritten / 1024).toStringAsFixed(0)} KB';
    return ColoredBox(
      color: OrColors.backgroundRaised,
      child: ListView(
        key: const ValueKey('motion-scene-list'),
        padding: const EdgeInsets.all(OrSpacing.x3),
        children: [
          Container(
            key: const ValueKey('motion-scene-card'),
            padding: const EdgeInsets.all(OrSpacing.x3),
            decoration: BoxDecoration(
              color: OrColors.surface,
              borderRadius: BorderRadius.circular(OrRadii.card),
              border: Border.all(color: OrColors.border),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        scene.title,
                        key: const ValueKey('motion-scene-title'),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                          color: OrColors.text,
                          fontSize: OrType.controlSize,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    OrBadge(
                      scene.inserted ? 'On timeline' : 'Rendered',
                      key: const ValueKey('motion-scene-state'),
                    ),
                  ],
                ),
                const SizedBox(height: OrSpacing.x2),
                _fact('Frames', '${scene.frameCount}'),
                _fact(
                  'Rate',
                  '${scene.frameRate} fps',
                ),
                _fact('Size', '${scene.width} × ${scene.height}'),
                _fact('File', '$size · ${_fileName(scene.destinationPath)}'),
                const SizedBox(height: OrSpacing.x2),
                Text(
                  'Source ${_fileName(scene.sourcePath)}',
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
          const SizedBox(height: OrSpacing.x2),
          FilledButton.icon(
            key: const ValueKey('motion-scene-insert'),
            onPressed: scene.inserted || onInsert == null
                ? null
                : () => onInsert!(),
            icon: const Icon(Icons.playlist_add, size: 16),
            label: Text(scene.inserted ? 'Already on the timeline' : 'Insert into timeline'),
          ),
          const SizedBox(height: OrSpacing.x1),
          OutlinedButton.icon(
            key: const ValueKey('motion-scene-rerender'),
            onPressed: onRerender == null ? null : () => onRerender!(),
            icon: const Icon(Icons.refresh, size: 16),
            label: const Text('Render again'),
          ),
          const SizedBox(height: OrSpacing.x1),
          TextButton.icon(
            key: const ValueKey('motion-scene-new'),
            onPressed: onCreate,
            icon: const Icon(Icons.add, size: 16),
            label: const Text('New scene…'),
          ),
        ],
      ),
    );
  }

  Widget _fact(String label, String value) => Padding(
    padding: const EdgeInsets.only(bottom: OrSpacing.x1),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 56,
          child: Text(
            label,
            style: const TextStyle(
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
            style: const TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.tinySize,
            ),
          ),
        ),
      ],
    ),
  );
}

String _fileName(String path) => path.split(RegExp(r'[\\/]')).last;

class _ToolAction {
  const _ToolAction({
    required this.keyName,
    required this.label,
    required this.icon,
    required this.onPressed,
  });

  final String keyName;
  final String label;
  final IconData icon;
  final VoidCallback? onPressed;
}

/// A short, honest tool panel: a sentence about what the tool does and the
/// actions it really performs. No "Unavailable" placeholder.
class _ToolActionsPanel extends StatelessWidget {
  const _ToolActionsPanel({
    required this.title,
    required this.subtitle,
    required this.actions,
    this.showTitle = true,
  });

  final String title;
  final String subtitle;
  final List<_ToolAction> actions;

  /// The tab strip above already names the category, so the panel must not
  /// repeat it as a second title row.
  final bool showTitle;

  @override
  Widget build(BuildContext context) {
    return ColoredBox(
      color: OrColors.backgroundRaised,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (showTitle) ...[
            _PanelHeader(title: title),
            const Divider(height: 1),
          ],
          Expanded(
            child: ListView(
              padding: const EdgeInsets.all(OrSpacing.x3),
              children: [
                Text(
                  subtitle,
                  style: const TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: OrType.metadataSize,
                    height: 1.45,
                  ),
                ),
                const SizedBox(height: OrSpacing.x4),
                for (final action in actions) ...[
                  SizedBox(
                    width: double.infinity,
                    child: OutlinedButton.icon(
                      key: ValueKey(action.keyName),
                      onPressed: action.onPressed,
                      icon: Icon(action.icon, size: 16),
                      label: Align(
                        alignment: Alignment.centerLeft,
                        child: Text(action.label),
                      ),
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x2),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _MediaLibraryPanel extends StatelessWidget {
  const _MediaLibraryPanel({
    required this.page,
    required this.project,
    required this.previewState,
    required this.tracks,
    required this.previews,
    required this.loading,
    required this.loadingMore,
    required this.error,
    required this.onImport,
    required this.onImportFolder,
    required this.onLoadMore,
    required this.onRefresh,
    required this.onRemove,
    required this.onRelink,
    required this.onGenerateCaptions,
    required this.onCleanUpAudio,
    required this.onAddMediaToTimeline,
    this.onClose,
    this.showTitle = true,
    this.searchController,
  });

  final ProjectMediaPage? page;
  final ProjectReadModel? project;
  final ProjectPreviewState? previewState;
  final ProjectTimelineTracks? tracks;
  final Map<String, ProjectMediaPreview> previews;
  final bool loading;
  final bool loadingMore;
  final String? error;
  final VoidCallback? onImport;
  final VoidCallback? onImportFolder;
  final VoidCallback? onLoadMore;
  final VoidCallback? onRefresh;
  final ValueChanged<ProjectMediaItem>? onRemove;
  final ValueChanged<ProjectMediaItem>? onRelink;
  final Future<void> Function(ProjectMediaItem media)? onGenerateCaptions;
  final Future<void> Function(ProjectMediaItem media)? onCleanUpAudio;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectMediaItem media,
    String trackId,
    ProjectRationalTime timelineStart,
    ProjectRationalTime sourceStart,
    ProjectRationalTime duration,
    String mode,
  )?
  onAddMediaToTimeline;
  final VoidCallback? onClose;

  /// Hidden when the tab strip already names the category.
  final bool showTitle;

  /// A library of a hundred clips is unusable without a filter. The field owns
  /// a controller so a keystroke re-filters without rebuilding the editor.
  final TextEditingController? searchController;

  @override
  Widget build(BuildContext context) {
    final controller = searchController;
    if (controller == null) return _panel(context, '');
    return ValueListenableBuilder<TextEditingValue>(
      valueListenable: controller,
      builder: (context, value, _) => _panel(context, value.text),
    );
  }

  Widget _panel(BuildContext context, String searchQuery) {
    final library = page?.items ?? const <ProjectMediaItem>[];
    final needle = searchQuery.trim().toLowerCase();
    final items = needle.isEmpty
        ? library
        : library
              .where(
                (item) =>
                    _mediaDisplayName(item.sourceUri).toLowerCase().contains(
                      needle,
                    ),
              )
              .toList(growable: false);
    final filtering = needle.isNotEmpty;
    final currentPreviewState = previewState;
    final timelineSnapshotMatches =
        page != null &&
        project != null &&
        tracks != null &&
        page!.projectId == project!.projectId &&
        page!.projectInstanceId == project!.projectInstanceId &&
        page!.projectRevision == project!.revision &&
        tracks!.projectId == project!.projectId &&
        tracks!.projectInstanceId == project!.projectInstanceId &&
        tracks!.projectRevision == project!.revision;
    return ColoredBox(
      key: const ValueKey('project-media-panel'),
      color: OrColors.backgroundRaised,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (showTitle) ...[
            _PanelHeader(
              title: 'Media',
              trailing: onClose == null
                  ? null
                  : IconButton(
                      key: const ValueKey('mobile-tool-sheet-close'),
                      tooltip: 'Close tool panel',
                      onPressed: onClose,
                      icon: const Icon(Icons.close_outlined),
                    ),
            ),
            const Divider(height: 1),
          ] else if (onClose != null)
            Align(
              alignment: Alignment.centerRight,
              child: IconButton(
                key: const ValueKey('mobile-tool-sheet-close'),
                tooltip: 'Close tool panel',
                onPressed: onClose,
                icon: const Icon(Icons.close_outlined),
              ),
            ),
          Padding(
            padding: const EdgeInsets.fromLTRB(
              OrSpacing.x2,
              OrSpacing.x2,
              OrSpacing.x2,
              0,
            ),
            child: Wrap(
              spacing: OrSpacing.x1,
              runSpacing: OrSpacing.x1,
              children: [
                OutlinedButton.icon(
                  key: const ValueKey('media-import'),
                  onPressed: onImport,
                  icon: const Icon(Icons.add, size: 16),
                  label: const Text('Import Media'),
                ),
                OutlinedButton.icon(
                  key: const ValueKey('media-import-folder'),
                  onPressed: onImportFolder,
                  icon: const Icon(Icons.create_new_folder_outlined, size: 16),
                  label: const Text('Import Folder'),
                ),
              ],
            ),
          ),
          if (page != null && library.isNotEmpty)
            Padding(
              padding: const EdgeInsets.fromLTRB(
                OrSpacing.x2,
                OrSpacing.x2,
                OrSpacing.x2,
                0,
              ),
              child: TextField(
                key: const ValueKey('media-search'),
                controller: searchController,
                textInputAction: TextInputAction.search,
                style: const TextStyle(
                  color: OrColors.text,
                  fontSize: OrType.metadataCompactSize,
                ),
                decoration: InputDecoration(
                  isDense: true,
                  hintText: 'Search media',
                  hintStyle: const TextStyle(
                    color: OrColors.textMuted,
                    fontSize: OrType.metadataCompactSize,
                  ),
                  prefixIcon: const Icon(
                    Icons.search,
                    size: 16,
                    color: OrColors.textMuted,
                  ),
                  prefixIconConstraints: const BoxConstraints(
                    minWidth: 30,
                    minHeight: 30,
                  ),
                  contentPadding: const EdgeInsets.symmetric(
                    horizontal: OrSpacing.x2,
                    vertical: OrSpacing.x1,
                  ),
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(OrRadii.small),
                    borderSide: const BorderSide(color: OrColors.border),
                  ),
                  enabledBorder: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(OrRadii.small),
                    borderSide: const BorderSide(color: OrColors.border),
                  ),
                  focusedBorder: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(OrRadii.small),
                    borderSide: const BorderSide(color: OrColors.selection),
                  ),
                ),
              ),
            ),
          if (page != null)
            Padding(
              padding: const EdgeInsets.fromLTRB(
                OrSpacing.x3,
                OrSpacing.x2,
                OrSpacing.x3,
                0,
              ),
              child: Text(
                filtering
                    ? '${items.length} of ${library.length} items'
                    : '${library.length} ${library.length == 1 ? 'item' : 'items'}',
                key: const ValueKey('media-count'),
                style: const TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                ),
              ),
            ),
          if (loading && items.isEmpty)
            const Expanded(
              child: Center(child: CircularProgressIndicator(strokeWidth: 2)),
            )
          else if (items.isEmpty && error == null && filtering)
            const Expanded(
              child: Align(
                alignment: Alignment.topCenter,
                child: Padding(
                  padding: EdgeInsets.fromLTRB(
                    OrSpacing.x3,
                    OrSpacing.x4,
                    OrSpacing.x3,
                    0,
                  ),
                  child: Text(
                    'No media matches this search.',
                    textAlign: TextAlign.center,
                    style: TextStyle(
                      color: OrColors.textMuted,
                      fontSize: OrType.metadataCompactSize,
                    ),
                  ),
                ),
              ),
            )
          else if (items.isEmpty && error == null)
            // A centred sentence in the middle of a tall panel reads as a
            // broken panel. The hint sits under the actions that resolve it.
            const Expanded(
              child: Align(
                alignment: Alignment.topCenter,
                child: Padding(
                  padding: EdgeInsets.fromLTRB(
                    OrSpacing.x3,
                    OrSpacing.x4,
                    OrSpacing.x3,
                    0,
                  ),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        Icons.video_library_outlined,
                        size: 22,
                        color: OrColors.textMuted,
                      ),
                      SizedBox(height: OrSpacing.x2),
                      Text(
                        'Import a video, image or audio file to start.',
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          color: OrColors.textMuted,
                          fontSize: OrType.metadataCompactSize,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            )
          else
            Expanded(
              child: ListView(
                key: const ValueKey('media-list'),
                padding: const EdgeInsets.all(OrSpacing.x2),
                children: [
                  for (final item in items)
                    Padding(
                      padding: const EdgeInsets.only(bottom: OrSpacing.x2),
                      child: _MediaLibraryItem(
                        item: item,
                        preview: previews[item.mediaId],
                        onAddToTimeline:
                            onAddMediaToTimeline == null ||
                                !timelineSnapshotMatches ||
                                currentPreviewState == null
                            ? null
                            : () => unawaited(
                                _showInsertTimelineDialog(
                                  context: context,
                                  project: project!,
                                  media: item,
                                  tracks: tracks!,
                                  suggestedStart:
                                      currentPreviewState.contentEnd ??
                                      ProjectRationalTime(BigInt.zero, 1),
                                  onInsert: onAddMediaToTimeline!,
                                ),
                              ),
                        onRemove: onRemove == null
                            ? null
                            : () => onRemove!(item),
                        onRelink: onRelink == null
                            ? null
                            : () => onRelink!(item),
                        onGenerateCaptions:
                            onGenerateCaptions == null ||
                                item.audioDetails == null
                            ? null
                            : () => unawaited(onGenerateCaptions!(item)),
                        onCleanUpAudio:
                            onCleanUpAudio == null || item.audioDetails == null
                            ? null
                            : () => unawaited(onCleanUpAudio!(item)),
                      ),
                    ),
                  if (error != null) ...[
                    Padding(
                      padding: const EdgeInsets.all(OrSpacing.x2),
                      child: Text(
                        error!,
                        style: const TextStyle(
                          color: OrColors.textSecondary,
                          fontSize: OrType.tinySize,
                        ),
                      ),
                    ),
                    TextButton(
                      onPressed: onRefresh,
                      child: const Text('Retry'),
                    ),
                  ],
                  if (page?.nextOffset != null)
                    TextButton.icon(
                      key: const ValueKey('media-load-more'),
                      onPressed: loadingMore ? null : onLoadMore,
                      icon: loadingMore
                          ? const SizedBox.square(
                              dimension: 14,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.expand_more, size: 17),
                      label: const Text('Load more'),
                    ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

class _MediaLibraryItem extends StatelessWidget {
  const _MediaLibraryItem({
    required this.item,
    required this.preview,
    required this.onAddToTimeline,
    required this.onRelink,
    required this.onRemove,
    required this.onGenerateCaptions,
    required this.onCleanUpAudio,
  });

  final ProjectMediaItem item;
  final ProjectMediaPreview? preview;
  final VoidCallback? onAddToTimeline;
  final VoidCallback? onRelink;
  final VoidCallback? onRemove;
  final VoidCallback? onGenerateCaptions;
  final VoidCallback? onCleanUpAudio;

  @override
  Widget build(BuildContext context) {
    final name = _mediaDisplayName(item.sourceUri);
    final supportsTimelineDrag =
        item.videoDetails != null || item.audioDetails != null;
    final details = <String>[
      if (item.formatNames.isNotEmpty) item.formatNames.join(', '),
      if (item.duration != null) item.duration!,
      if (item.videoDetails != null) item.videoDetails!,
      if (item.audioDetails != null) 'Audio · ${item.audioDetails}',
    ];
    Widget dragHandle() => Tooltip(
      message: 'Drag to Timeline',
      child: SizedBox(
        key: ValueKey('media-drag-handle-${item.mediaId}'),
        width: 40,
        height: 40,
        child: const Icon(Icons.drag_indicator, size: 18),
      ),
    );
    Widget dragAction() => Draggable<ProjectMediaItem>(
      key: ValueKey('media-drag-${item.mediaId}'),
      data: item,
      affinity: Axis.horizontal,
      dragAnchorStrategy: pointerDragAnchorStrategy,
      feedback: Material(
        color: OrColors.surface,
        borderRadius: BorderRadius.circular(OrRadii.small),
        child: SizedBox(
          width: 220,
          child: Padding(
            padding: const EdgeInsets.all(OrSpacing.x2),
            child: Text(
              'Add ${_mediaDisplayName(item.sourceUri)} to Timeline',
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(
                color: OrColors.text,
                fontSize: OrType.tinySize,
              ),
            ),
          ),
        ),
      ),
      childWhenDragging: Opacity(opacity: 0.45, child: dragHandle()),
      child: dragHandle(),
    );
    List<Widget> actionWidgets() => [
      if (supportsTimelineDrag) dragAction(),
      if (item.videoDetails != null || item.audioDetails != null)
        IconButton(
          key: ValueKey('media-add-timeline-${item.mediaId}'),
          tooltip: 'Add to Timeline',
          onPressed: onAddToTimeline,
          visualDensity: VisualDensity.compact,
          icon: const Icon(Icons.playlist_add_outlined, size: 18),
          color: OrColors.textMuted,
        ),
      PopupMenuButton<String>(
        key: ValueKey('media-actions-${item.mediaId}'),
        tooltip: 'Media actions',
        onSelected:
            onRelink == null &&
                onRemove == null &&
                onGenerateCaptions == null &&
                onCleanUpAudio == null
            ? null
            : (action) {
                if (action == 'relink') onRelink?.call();
                if (action == 'remove') onRemove?.call();
                if (action == 'transcribe') onGenerateCaptions?.call();
                if (action == 'cleanup') onCleanUpAudio?.call();
              },
        itemBuilder: (context) => [
          if (onRelink != null)
            PopupMenuItem(
              key: ValueKey('media-relink-${item.mediaId}'),
              value: 'relink',
              child: const Text('Relink Source'),
            ),
          if (onRemove != null)
            PopupMenuItem(
              key: ValueKey('media-remove-${item.mediaId}'),
              value: 'remove',
              child: const Text('Remove from Project'),
            ),
          if (onGenerateCaptions != null)
            PopupMenuItem(
              key: ValueKey('media-generate-captions-${item.mediaId}'),
              value: 'transcribe',
              child: const Text('Generate Captions'),
            ),
          // Only a file with audio can be cleaned, and the result is a new
          // item rather than a change to this one.
          if (onCleanUpAudio != null && item.audioDetails != null)
            PopupMenuItem(
              key: ValueKey('media-clean-audio-${item.mediaId}'),
              value: 'cleanup',
              child: const Text('Clean up Audio'),
            ),
        ],
        child: const SizedBox(
          width: 40,
          height: 40,
          child: Icon(Icons.more_vert, size: 17, color: OrColors.textMuted),
        ),
      ),
    ];
    Widget actions() =>
        Row(mainAxisSize: MainAxisSize.min, children: actionWidgets());
    Widget metadata() => Expanded(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: OrSpacing.x2),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              name,
              key: ValueKey('media-name-${item.mediaId}'),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(
                color: OrColors.text,
                fontSize: OrType.tinySize,
                fontWeight: FontWeight.w500,
              ),
            ),
            if (details.isNotEmpty)
              Text(
                details.join(' · '),
                maxLines: 3,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                  height: 1.35,
                ),
              ),
          ],
        ),
      ),
    );
    final card = Tooltip(
      message: 'Media ID: ${item.mediaId}\n${item.sourceUri}',
      child: Container(
        key: ValueKey('media-item-${item.mediaId}'),
        padding: const EdgeInsets.only(left: OrSpacing.x2),
        decoration: BoxDecoration(
          color: OrColors.surface,
          border: Border.all(color: OrColors.border),
          borderRadius: BorderRadius.circular(OrRadii.small),
        ),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final narrow = constraints.maxWidth < 270;
            final leading = [
              _MediaItemPreview(item: item, preview: preview),
              const SizedBox(width: OrSpacing.x2),
              metadata(),
            ];
            if (narrow) {
              return Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Row(children: leading),
                  Align(alignment: Alignment.centerRight, child: actions()),
                ],
              );
            }
            return Row(children: [...leading, ...actionWidgets()]);
          },
        ),
      ),
    );
    return card;
  }
}

class _MediaItemPreview extends StatelessWidget {
  const _MediaItemPreview({required this.item, required this.preview});

  final ProjectMediaItem item;
  final ProjectMediaPreview? preview;

  @override
  Widget build(BuildContext context) {
    final hasVideo = item.videoDetails != null;
    final kind = hasVideo
        ? ProjectMediaArtifactKind.thumbnail
        : item.audioDetails != null
        ? ProjectMediaArtifactKind.waveform
        : null;
    final bytes = preview?.kind == kind ? preview?.bytes : null;
    final label = kind == ProjectMediaArtifactKind.thumbnail
        ? 'Thumbnail preview for ${_mediaDisplayName(item.sourceUri)}'
        : 'Waveform preview for ${_mediaDisplayName(item.sourceUri)}';
    return SizedBox(
      key: ValueKey('media-preview-${item.mediaId}'),
      width: 52,
      height: 36,
      child: bytes == null
          ? Center(
              child: Icon(
                hasVideo
                    ? Icons.movie_outlined
                    : kind == ProjectMediaArtifactKind.waveform
                    ? Icons.graphic_eq
                    : Icons.perm_media_outlined,
                size: 16,
                color: OrColors.textMuted,
              ),
            )
          : ClipRRect(
              borderRadius: BorderRadius.circular(OrRadii.small),
              child: Image.memory(
                bytes,
                key: ValueKey('media-preview-image-${item.mediaId}'),
                fit: BoxFit.contain,
                semanticLabel: label,
                errorBuilder: (context, error, stackTrace) => const Center(
                  child: Icon(
                    Icons.perm_media_outlined,
                    size: 16,
                    color: OrColors.textMuted,
                  ),
                ),
              ),
            ),
    );
  }
}

String _mediaDisplayName(String sourceUri) {
  final uri = Uri.tryParse(sourceUri);
  if (uri != null && uri.pathSegments.isNotEmpty) {
    return uri.pathSegments.last;
  }
  final normalized = sourceUri.replaceAll('\\', '/');
  return normalized.split('/').last;
}

class _ViewerPanel extends StatefulWidget {
  const _ViewerPanel({
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
  State<_ViewerPanel> createState() => _ViewerPanelState();
}

/// The transport commands the editor's key layer dispatches through.
///
/// Implemented by the viewer panel, which owns the preview state, so a key and
/// the transport buttons call the same code.
mixin _ViewerCommands {
  void togglePlayPause();

  void stepFrame(bool backwards);

  void seekBy(double seconds);
}

/// Frame rates the sequence can carry. One list, because the viewer transport
/// and the inspector both offer the same choice and a second copy would drift.
const _sequenceRateChoices = [
    _FrameRateChoice(
      rate: ProjectRationalRate(24000, 1001),
      label: '23.976 (24000/1001) fps',
    ),
    _FrameRateChoice(rate: ProjectRationalRate(24, 1), label: '24 fps'),
    _FrameRateChoice(rate: ProjectRationalRate(25, 1), label: '25 fps'),
    _FrameRateChoice(rate: ProjectRationalRate(30, 1), label: '30 fps'),
    _FrameRateChoice(rate: ProjectRationalRate(50, 1), label: '50 fps'),
    _FrameRateChoice(rate: ProjectRationalRate(60, 1), label: '60 fps'),
  _FrameRateChoice.unset(),
];

class _ViewerPanelState extends State<_ViewerPanel>
    with _ViewerCommands, WidgetsBindingObserver {
  static const _rateChoices = _sequenceRateChoices;

  ProjectPreviewState? _preview;
  ProjectRationalTime? _scrubPosition;
  String? _error;
  Timer? _timer;
  int? _textureId;
  int _epoch = 0;
  bool _busy = false;
  bool _tickInFlight = false;
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
  void didUpdateWidget(covariant _ViewerPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    final oldProject = oldWidget.project;
    final project = widget.project;
    final previewInputChanged =
        oldWidget.previewSeekPosition != widget.previewSeekPosition &&
        widget.previewSeekPosition != null;
    if (oldWidget.gateway != widget.gateway ||
        oldWidget.session != widget.session ||
        oldProject?.projectId != project?.projectId ||
        oldProject?.projectInstanceId != project?.projectInstanceId ||
        oldProject?.revision != project?.revision ||
        previewInputChanged) {
      unawaited(_initialize());
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

  void _acceptState(ProjectPreviewState state, int epoch) {
    if (!_isCurrent(epoch) || _isOlder(state)) return;
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
    final position = _timeFromSeconds(seconds);
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
              _PanelHeader(
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
                                              _timeFromSeconds(seconds),
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
                              _clockLabel(0),
                              style: const TextStyle(
                                color: OrColors.textMuted,
                                fontFamily: 'monospace',
                                fontSize: OrType.tinySize,
                              ),
                            ),
                            Text(
                              _clockLabel(endSeconds),
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
                          child: PopupMenuButton<_FrameRateChoice>(
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
                                    : _frameRateLabel(frameRate),
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

class _FrameRateChoice {
  const _FrameRateChoice({required this.rate, required this.label});
  const _FrameRateChoice.unset() : rate = null, label = 'Remove frame rate';

  final ProjectRationalRate? rate;
  final String label;
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

ProjectRationalTime _timeFromSeconds(double seconds) => ProjectRationalTime(
  BigInt.from((seconds.clamp(0, double.maxFinite) * 1000000).round()),
  1000000,
);

/// `0.5×`, `1.5×`, `2×`: a speed badge reads better as a decimal than as the
/// canonical `3/2` the model stores.
String _rateLabel(ProjectRationalRate rate) {
  final value = rate.numerator / rate.denominator;
  final text = value == value.roundToDouble()
      ? value.round().toString()
      : value.toStringAsFixed(2).replaceAll(RegExp(r'0+$'), '').replaceAll(RegExp(r'\.$'), '');
  return text;
}

String _clockLabel(double seconds) {
  final milliseconds = (seconds * 1000).round();
  final minutes = milliseconds ~/ 60000;
  final wholeSeconds = milliseconds ~/ 1000 % 60;
  final fraction = milliseconds % 1000;
  return '${minutes.toString().padLeft(2, '0')}:${wholeSeconds.toString().padLeft(2, '0')}.${fraction.toString().padLeft(3, '0')}';
}

/// Platform-correct shortcut label for tooltips and menus. macOS uses the
/// command glyph; every other target keeps the explicit Ctrl prefix.
String _shortcutLabel(String key) =>
    defaultTargetPlatform == TargetPlatform.macOS ? '⌘$key' : 'Ctrl+$key';

double _rationalSeconds(ProjectRationalTime time) =>
    time.numerator.toInt() / time.denominator;

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

ProjectRationalTime? _parseSecondsOrRational(String text) {
  final trimmed = text.trim();
  if (trimmed.isEmpty) return null;
  final direct = ProjectRationalTime.tryParse(trimmed);
  if (direct != null) return direct;
  final seconds = _finiteNumber(trimmed);
  if (seconds == null || seconds < 0) return null;
  return _timeFromSeconds(seconds);
}

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
  return _clockLabel(time);
}

/// The Delete key hint follows the host keyboard, like every other shortcut.
String _deleteShortcutLabel({bool shift = false}) {
  final mac = defaultTargetPlatform == TargetPlatform.macOS;
  final key = mac ? '⌫' : 'Del';
  return shift ? '⇧$key' : key;
}

/// Human frame-rate label: `30 fps`, `23.976 fps`, never `30/1 fps`.
String _frameRateLabel(ProjectRationalRate rate) {
  final value = rate.numerator / rate.denominator;
  final text = value == value.roundToDouble()
      ? value.toStringAsFixed(0)
      : value.toStringAsFixed(3);
  return '$text fps';
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

class _InspectorPanel extends StatefulWidget {
  const _InspectorPanel({
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
    this.onClose,
  });

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
  final VoidCallback? onClose;

  @override
  State<_InspectorPanel> createState() => _InspectorPanelState();
}

class _InspectorPanelState extends State<_InspectorPanel> {
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
  void didUpdateWidget(covariant _InspectorPanel oldWidget) {
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
    final transitionInDuration = _parseSecondsOrRational(
      _fields['transition_in_duration']!.text,
    );
    final transitionOutDuration = _parseSecondsOrRational(
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

  ProjectTimelineAudioSettings? _readAudioSettings() {
    final gain = double.tryParse(_audioFields['gain_db']!.text);
    final pan = double.tryParse(_audioFields['pan_percent']!.text);
    final fadeIn = _parseSecondsOrRational(_audioFields['fade_in']!.text);
    final fadeOut = _parseSecondsOrRational(_audioFields['fade_out']!.text);
    if (gain == null ||
        !gain.isFinite ||
        gain < -96 ||
        gain > 24 ||
        pan == null ||
        !pan.isFinite ||
        pan < -100 ||
        pan > 100 ||
        fadeIn == null ||
        fadeIn.numerator < BigInt.zero ||
        fadeOut == null ||
        fadeOut.numerator < BigInt.zero) {
      return null;
    }
    return ProjectTimelineAudioSettings(
      gainMilliDecibels: (gain * 1000).round(),
      panBasisPoints: (pan * 100).round(),
      fadeIn: fadeIn,
      fadeOut: fadeOut,
    );
  }

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
              style: TextStyle(color: OrColors.textSecondary, fontSize: OrType.tinySize),
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
            rate == null ? 'Not set yet' : _frameRateLabel(rate),
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
            child: PopupMenuButton<_FrameRateChoice>(
              key: const ValueKey('inspector-sequence-frame-rate'),
              tooltip: 'Sequence frame rate',
              onSelected: (choice) =>
                  widget.onSetSequenceFrameRate?.call(choice.rate),
              itemBuilder: (context) => [
                for (final choice in _sequenceRateChoices)
                  PopupMenuItem(value: choice, child: Text(choice.label)),
              ],
              child: Text(
                rate == null ? 'Not set yet' : _frameRateLabel(rate),
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
        Text(
          _error!,
          style: TextStyle(
            color: Theme.of(context).colorScheme.error,
            fontSize: OrType.tinySize,
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
                            () => _error = 'Enter a gain between -96 and 24 dB, a pan between -100 and 100%, and fade times in seconds.',
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
          _PanelHeader(
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
                    Text(
                      _error!,
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.error,
                        fontSize: OrType.tinySize,
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
class _ToolbarScroll extends StatefulWidget {
  const _ToolbarScroll({required this.child});

  final Widget child;

  @override
  State<_ToolbarScroll> createState() => _ToolbarScrollState();
}

class _ToolbarScrollState extends State<_ToolbarScroll> {
  final ScrollController _controller = ScrollController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Scrollbar(
    controller: _controller,
    thumbVisibility: true,
    thickness: 4,
    radius: const Radius.circular(2),
    scrollbarOrientation: ScrollbarOrientation.bottom,
    child: SingleChildScrollView(
      controller: _controller,
      scrollDirection: Axis.horizontal,
      child: widget.child,
    ),
  );
}

class _TimelinePanel extends StatefulWidget {
  const _TimelinePanel({
    super.key,
    required this.compact,
    required this.previewState,
    required this.isProjectWorkspace,
    required this.project,
    required this.tracks,
    required this.clipPages,
    required this.markerPage,
    required this.loadingMoreTracks,
    required this.loadingMoreMarkers,
    required this.loading,
    required this.error,
    required this.busy,
    required this.onImportMedia,
    required this.onAddVideoTrack,
    required this.onAddAudioTrack,
    required this.onAddTextTrack,
    required this.onAddCaptionTrack,
    required this.onRemoveTrack,
    required this.onSetTrackState,
    required this.onMoveTrack,
    required this.onLoadMore,
    required this.onLoadMoreMarkers,
    required this.onRefresh,
    required this.onMoveClip,
    required this.onDuplicateClip,
    required this.onResolveSnap,
    required this.onDeleteClip,
    required this.onTrimClip,
    required this.onSplitClip,
    this.bladeMode = false,
    this.onCopyClip,
    this.onPasteClip,
    this.onToggleBladeMode,
    required this.onSlipClip,
    required this.onRollClip,
    required this.onSlideClip,
    required this.onRippleDeleteClip,
    this.onSetClipReversed,
    required this.onUpdateTextClip,
    required this.onAddMarker,
    required this.onMoveMarker,
    required this.onRenameMarker,
    required this.onDeleteMarker,
    required this.snapEnabled,
    this.onSnapChanged,
    this.onImportCaptions,
    this.onExportCaptions,
    this.onAddTitle,
    this.onAddCaption,
    this.onRequestMarker,
    required this.onTimelineEditError,
    required this.mediaItems,
    required this.mediaPreviews,
    required this.onAddMediaToTimeline,
    this.onClipSelectionChanged,
  });

  final bool compact;

  final ValueListenable<ProjectPreviewState?> previewState;
  final bool isProjectWorkspace;
  final ProjectReadModel? project;
  final ProjectTimelineTracks? tracks;
  final Map<String, ProjectTimelineClipPage> clipPages;
  final ProjectTimelineMarkerPage? markerPage;
  final Set<String> loadingMoreTracks;
  final bool loadingMoreMarkers;
  final bool loading;
  final String? error;
  final bool busy;
  final VoidCallback? onImportMedia;
  final VoidCallback? onAddVideoTrack;
  final VoidCallback? onAddAudioTrack;
  final VoidCallback? onAddTextTrack;
  final VoidCallback? onAddCaptionTrack;
  final Future<void> Function(ProjectReadModel, ProjectTimelineTrack)?
  onRemoveTrack;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineTrackState,
  )?
  onSetTrackState;
  final Future<void> Function(ProjectReadModel, ProjectTimelineTrack, int)?
  onMoveTrack;
  final ValueChanged<String>? onLoadMore;
  final VoidCallback? onLoadMoreMarkers;
  final VoidCallback? onRefresh;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    String,
    ProjectRationalTime,
  )?
  onMoveClip;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineClip,
  )?
  onDuplicateClip;
  final Future<ProjectTimelineSnapResult> Function(
    ProjectReadModel project,
    ProjectTimelineSnapOperation operation,
    String clipId,
    String? targetTrackId,
    ProjectRationalTime targetTime,
  )?
  onResolveSnap;
  final Future<void> Function(ProjectReadModel, ProjectTimelineClip)?
  onDeleteClip;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectTimelineTrimEdge,
    ProjectRationalTime,
    bool ripple,
  )?
  onTrimClip;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onSplitClip;

  /// Blade mode turns a click on a clip into a split at the pointer.
  final bool bladeMode;
  final void Function(
    ProjectReadModel project,
    ProjectTimelineTrack track,
    ProjectTimelineClip clip,
  )?
  onCopyClip;
  final void Function(
    ProjectReadModel project,
    ProjectTimelineTrack track,
    ProjectRationalTime position,
  )?
  onPasteClip;
  final VoidCallback? onToggleBladeMode;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onSlipClip;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onRollClip;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onSlideClip;
  final Future<void> Function(ProjectReadModel, ProjectTimelineClip)?
  onRippleDeleteClip;
  /// Flips the direction of one media clip; the clip menu offers it beside
  /// Duplicate so reversal does not require opening the inspector.
  final Future<ProjectReadModel?> Function(
    ProjectReadModel expected,
    String trackId,
    String clipId,
    bool reversed,
  )?
  onSetClipReversed;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineClip,
    ProjectRationalTime,
    ProjectTimelineTextContent,
  )?
  onUpdateTextClip;
  final Future<void> Function(ProjectReadModel, ProjectRationalTime, String)?
  onAddMarker;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineMarker,
    ProjectRationalTime,
  )?
  onMoveMarker;
  final Future<void> Function(ProjectReadModel, ProjectTimelineMarker, String)?
  onRenameMarker;
  final Future<void> Function(ProjectReadModel, ProjectTimelineMarker)?
  onDeleteMarker;
  final bool snapEnabled;
  final ValueChanged<String>? onTimelineEditError;
  final List<ProjectMediaItem> mediaItems;
  final Map<String, ProjectMediaPreview> mediaPreviews;
  final Future<void> Function(
    ProjectReadModel,
    ProjectMediaItem,
    String,
    ProjectRationalTime,
    ProjectRationalTime,
    ProjectRationalTime,
    String,
  )?
  onAddMediaToTimeline;
  final void Function(
    String? trackId,
    String? clipId,
    ProjectTimelineTrackKind? kind,
  )?
  onClipSelectionChanged;

  /// Snapping and the creation menu live in this zone's single control row.
  final ValueChanged<bool>? onSnapChanged;
  final VoidCallback? onImportCaptions;
  final VoidCallback? onExportCaptions;
  final VoidCallback? onAddTitle;
  final VoidCallback? onAddCaption;

  /// Opens the labelled marker dialog; the panel does not invent marker names.
  final VoidCallback? onRequestMarker;

  @override
  State<_TimelinePanel> createState() => _TimelinePanelState();
}

class _TimelinePanelState extends State<_TimelinePanel> {
  final Map<String, GlobalKey> _laneKeys = {};
  final FocusNode _timelineFocusNode = FocusNode(debugLabel: 'Timeline');

  final ScrollController _horizontalController = ScrollController();
  _TimelinePointerGesture? _gesture;
  _TimelineSnapGuide? _snapGuide;
  String? _selectedClipId;
  double _timelineZoom = 1;
  double _pixelsPerSecond = 64;
  bool _fitTimelineToView = false;
  int _gestureToken = 0;

  @override
  void didUpdateWidget(covariant _TimelinePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    final previousProject = oldWidget.project;
    final nextProject = widget.project;
    if (previousProject?.projectId != nextProject?.projectId ||
        previousProject?.projectInstanceId != nextProject?.projectInstanceId ||
        previousProject?.revision != nextProject?.revision) {
      _cancelGesture();
    }
    if (previousProject?.projectId != nextProject?.projectId ||
        previousProject?.projectInstanceId != nextProject?.projectInstanceId) {
      _selectedClipId = null;
      _timelineZoom = 1;
      _fitTimelineToView = false;
      if (_horizontalController.hasClients) _horizontalController.jumpTo(0);
    }
  }

  @override
  void dispose() {
    _gestureToken++;
    _timelineFocusNode.dispose();
    _horizontalController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<ProjectPreviewState?>(
      valueListenable: widget.previewState,
      builder: (context, preview, _) => _buildTimeline(context, preview),
    );
  }

  Widget _buildTimeline(BuildContext context, ProjectPreviewState? preview) {
    return Container(
      key: const ValueKey('editor-timeline-region'),
      width: double.infinity,
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Column(
        children: [
          if (!widget.isProjectWorkspace)
            Expanded(
              child: Column(
                children: [
                  // The frozen preview names every zone; the project
                  // workspace does not, because a working editor is identified
                  // by its content rather than by labels.
                  const _PanelHeader(
                    title: 'Timeline',
                    height: 32,
                    trailing: OrBadge('Developer Preview'),
                  ),
                  _TimelineRuler(compact: widget.compact),
                  const Expanded(
                    child: Column(
                      children: [
                        Expanded(child: _EmptyTimelineLane()),
                        Divider(height: 1),
                        Expanded(child: _EmptyTimelineLane()),
                        Divider(height: 1),
                        Expanded(child: _EmptyTimelineLane()),
                      ],
                    ),
                  ),
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: OrSpacing.x2),
                    child: Text(
                      'Timeline engine not implemented',
                      style: TextStyle(
                        color: OrColors.textMuted,
                        fontSize: OrType.tinySize,
                      ),
                    ),
                  ),
                ],
              ),
            )
          else
            Expanded(child: _buildProjectTimeline(context, preview)),
        ],
      ),
    );
  }

  Widget _buildProjectTimeline(
    BuildContext context,
    ProjectPreviewState? preview,
  ) {
    final snapshot = widget.tracks;
    if (widget.loading && snapshot == null) {
      return const Center(child: CircularProgressIndicator(strokeWidth: 2));
    }
    if (snapshot == null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              widget.error ?? 'The timeline is unavailable.',
              textAlign: TextAlign.center,
              style: const TextStyle(color: OrColors.textSecondary),
            ),
            TextButton.icon(
              onPressed: widget.onRefresh,
              icon: const Icon(Icons.refresh, size: 16),
              label: const Text('Refresh timeline'),
            ),
          ],
        ),
      );
    }
    if (snapshot.items.isEmpty && (widget.markerPage?.items.isEmpty ?? true)) {
      // This panel gets a small share of a short screen, so the empty state
      // must scroll: it previously overflowed by 154 px at 320x640 and 86 px at
      // 390x844, which is the emulator viewport.
      // The control row always renders: zoom, snapping and the creation menu
      // must stay reachable from the first empty timeline, otherwise the empty
      // state is the only place that can create anything.
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _timelineViewControls(),
          Expanded(
            child: SingleChildScrollView(
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Text(
                      'No timeline tracks',
                      style: TextStyle(
                        color: OrColors.textSecondary,
                        fontSize: OrType.metadataSize,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    const SizedBox(height: OrSpacing.x2),
                    if (widget.mediaItems.isEmpty &&
                        widget.isProjectWorkspace &&
                        widget.onImportMedia != null) ...[
                      const Text(
                        'Import a video or audio file to start editing.',
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          color: OrColors.textMuted,
                          fontSize: OrType.metadataCompactSize,
                        ),
                      ),
                      const SizedBox(height: OrSpacing.x2),
                      FilledButton.icon(
                        key: const ValueKey('timeline-empty-import-media'),
                        onPressed: widget.busy ? null : widget.onImportMedia,
                        icon: const Icon(Icons.add, size: 16),
                        label: const Text('Import Media'),
                      ),
                      const SizedBox(height: OrSpacing.x2),
                    ],
                    // One filled action, then quiet text links. Five bordered
                    // buttons competed for the same click and the `+` menu
                    // already lists the rest of the track kinds.
                    Wrap(
                      spacing: OrSpacing.x2,
                      alignment: WrapAlignment.center,
                      children: [
                        TextButton.icon(
                          key: const ValueKey('timeline-empty-add-video'),
                          onPressed: widget.busy
                              ? null
                              : widget.onAddVideoTrack,
                          icon: const Icon(Icons.movie_outlined, size: 16),
                          label: const Text('Add video track'),
                        ),
                        TextButton.icon(
                          key: const ValueKey('timeline-empty-add-audio'),
                          onPressed: widget.busy
                              ? null
                              : widget.onAddAudioTrack,
                          icon: const Icon(Icons.graphic_eq_outlined, size: 16),
                          label: const Text('Add audio track'),
                        ),
                      ],
                    ),
                    Text(
                      'More track kinds, titles, captions and markers are in '
                      'the + menu above.',
                      textAlign: TextAlign.center,
                      style: TextStyle(
                        color: OrColors.textMuted,
                        fontSize: OrType.metadataCompactSize,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ],
      );
    }

    return Focus(
      focusNode: _timelineFocusNode,
      autofocus: true,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final labels = _timelineTrackLabels(snapshot.items);
          var endSeconds = 0.0;
          for (final page in widget.clipPages.values) {
            for (final clip in page.items) {
              final start = clip.timelineStart.secondsForDisplay;
              final duration = clip.timelineDuration.secondsForDisplay;
              final end = start + duration;
              if (start.isFinite && duration.isFinite && end.isFinite) {
                endSeconds = math.max(endSeconds, end);
              }
            }
          }
          final previewEnd = preview?.contentEnd?.secondsForDisplay;
          if (previewEnd != null && previewEnd.isFinite) {
            endSeconds = math.max(endSeconds, previewEnd);
          }
          for (final marker in widget.markerPage?.items ?? const []) {
            final seconds = marker.timelineTime.secondsForDisplay;
            if (seconds.isFinite) endSeconds = math.max(endSeconds, seconds);
          }
          final displayDuration = math.max(20.0, endSeconds);
          const maximumWidth = 100000.0;
          final trackHeaderWidth = _timelineTrackHeaderWidthFor(
            constraints.maxWidth,
          );
          final viewportWidth = math.max(
            1.0,
            constraints.maxWidth - trackHeaderWidth,
          );
          final preferredScale = _fitTimelineToView
              ? viewportWidth / displayDuration
              : 64.0 * _timelineZoom;
          final scale = math.min(
            preferredScale,
            maximumWidth / displayDuration,
          );
          _pixelsPerSecond = scale;
          final canvasWidth = math.max(
            viewportWidth,
            math.min(maximumWidth, displayDuration * scale),
          );
          final tickSeconds = _timelineTickSeconds(
            displayDuration,
            canvasWidth,
          );
          final trackRows = <Widget>[];
          for (var walk = 0; walk < snapshot.items.length; walk += 1) {
            final track = snapshot.items[walk];
            final page = widget.clipPages[track.trackId];
            final loadedCount = page?.items.length ?? 0;
            final countLabel = loadedCount < track.clipCount
                ? '$loadedCount / ${track.clipCount}'
                : '${track.clipCount}';
            trackRows.add(
              _TimelineTrackHeader(
                track: track,
                label: labels[track.trackId]!,
                countLabel: countLabel,
                trackIndex: walk,
                trackCount: snapshot.items.length,
                busy: widget.busy,
                project: widget.project,
                onRemove: widget.onRemoveTrack,
                onSetState: widget.onSetTrackState,
                onMove: widget.onMoveTrack,
              ),
            );
            if (page?.nextOffset != null) {
              trackRows.add(
                _TimelineLoadMoreHeader(
                  trackId: track.trackId,
                  width: trackHeaderWidth,
                  loading: widget.loadingMoreTracks.contains(track.trackId),
                  busy: widget.busy,
                  onLoadMore: widget.onLoadMore,
                ),
              );
            }
          }

          return Column(
            children: [
              // The fixed-height view controls cannot fit a region this
              // short; drop them rather than overflow an unusable strip.
              if (constraints.maxHeight >= 60) _timelineViewControls(),
              Expanded(
                child: Scrollbar(
                  child: SingleChildScrollView(
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        SizedBox(
                          width: trackHeaderWidth,
                          child: Column(
                            children: [
                              const SizedBox(height: _timelineRulerHeight),
                              ...trackRows,
                            ],
                          ),
                        ),
                        Expanded(
                          child: Scrollbar(
                            controller: _horizontalController,
                            child: SingleChildScrollView(
                              controller: _horizontalController,
                              scrollDirection: Axis.horizontal,
                              child: SizedBox(
                                width: canvasWidth,
                                child: Column(
                                  children: [
                                    SizedBox(
                                      height: _timelineRulerHeight,
                                      child: _TimelineMarkerRuler(
                                        markers:
                                            widget.markerPage?.items ??
                                            const [],
                                        playhead: preview?.position,
                                        pixelsPerSecond: scale,
                                        tickSeconds: tickSeconds,
                                        width: canvasWidth,
                                        loadingMore: widget.loadingMoreMarkers,
                                        onLoadMore:
                                            widget.markerPage?.nextOffset ==
                                                null
                                            ? null
                                            : widget.onLoadMoreMarkers,
                                        busy: widget.busy,
                                        snapGuide: _snapGuide,
                                        onOpenMarker: (marker) => unawaited(
                                          _showTimelineMarkerActions(
                                            context: context,
                                            project: widget.project,
                                            marker: marker,
                                            onMove: widget.onMoveMarker,
                                            onRename: widget.onRenameMarker,
                                            onDelete: widget.onDeleteMarker,
                                            busy: widget.busy,
                                          ),
                                        ),
                                        onMoveMarker:
                                            widget.onMoveMarker == null
                                            ? null
                                            : (marker, timelineTime) =>
                                                  widget.onMoveMarker!(
                                                    widget.project!,
                                                    marker,
                                                    timelineTime,
                                                  ),
                                      ),
                                    ),
                                    for (final track in snapshot.items) ...[
                                      _TimelineTrackLane(
                                        laneKey: _laneKeys.putIfAbsent(
                                          track.trackId,
                                          GlobalKey.new,
                                        ),
                                        track: track,
                                        clips:
                                            widget
                                                .clipPages[track.trackId]
                                                ?.items ??
                                            const [],
                                        mediaItems: widget.mediaItems,
                                        mediaPreviews: widget.mediaPreviews,
                                        width: canvasWidth,
                                        pixelsPerSecond: scale,
                                        tickSeconds: tickSeconds,
                                        playhead: preview?.position,
                                        gesture: _gesture,
                                        snapGuide: _snapGuide,
                                        selectedClipId: _selectedClipId,
                                        bladeMode: widget.bladeMode,
                                        onBladeSplit:
                                            widget.project == null ||
                                                widget.onSplitClip == null
                                            ? null
                                            : (clip, time) => unawaited(
                                                widget.onSplitClip!(
                                                  widget.project!,
                                                  clip,
                                                  time,
                                                ),
                                              ),
                                        onDropMedia:
                                            widget.onAddMediaToTimeline ==
                                                    null ||
                                                widget.project == null ||
                                                widget.busy
                                            ? null
                                            : (media, position) =>
                                                  _dropMediaOnTrack(
                                                    track,
                                                    media,
                                                    position,
                                                    scale,
                                                  ),
                                        onOpenClip: (clip) =>
                                            _selectClip(track, clip),
                                        onOpenClipMenu: (clip, position) =>
                                            unawaited(
                                              _showClipMenu(
                                                track,
                                                clip,
                                                position,
                                              ),
                                            ),
                                        onMoveStart: track.state.locked
                                            ? null
                                            : (clip, details) =>
                                                  _beginMoveGesture(
                                                    track,
                                                    clip,
                                                    details,
                                                  ),
                                        onMoveUpdate: (clip, details) =>
                                            _updateMoveGesture(clip, details),
                                        onMoveEnd: (clip, details) =>
                                            _endMoveGesture(clip, details),
                                        onMoveCancel: _cancelPointerGesture,
                                        onTrimStart: track.state.locked
                                            ? null
                                            : (
                                                clip,
                                                details,
                                              ) => _beginTrimGesture(
                                                track,
                                                clip,
                                                ProjectTimelineTrimEdge.start,
                                                details,
                                              ),
                                        onTrimEnd: track.state.locked
                                            ? null
                                            : (clip, details) =>
                                                  _beginTrimGesture(
                                                    track,
                                                    clip,
                                                    ProjectTimelineTrimEdge.end,
                                                    details,
                                                  ),
                                        onTrimUpdate: (clip, details) =>
                                            _updateTrimGesture(clip, details),
                                        onTrimFinish: (clip, details) =>
                                            _endTrimGesture(clip, details),
                                        onTrimCancel: _cancelPointerGesture,
                                      ),
                                      if (widget
                                              .clipPages[track.trackId]
                                              ?.nextOffset !=
                                          null)
                                        _TimelineLoadMoreLane(
                                          width: canvasWidth,
                                        ),
                                    ],
                                  ],
                                ),
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  Widget _timelineViewControls() {
    final selectedClip = _selectedClip;
    final selectedTrack = _selectedTrack;
    final canDuplicate =
        selectedClip != null &&
        selectedTrack != null &&
        !selectedTrack.state.locked &&
        !widget.busy &&
        widget.project != null &&
        widget.onDuplicateClip != null;
    // Split is the most frequent trim operation; it used to live only inside
    // the clip dialog and the ⌘B shortcut. Offer it on the selection strip and
    // disable it honestly when the playhead is outside the selected clip.
    final previewPosition = widget.previewState.value?.position;
    final canSplit =
        selectedClip != null &&
        selectedTrack != null &&
        previewPosition != null &&
        !selectedTrack.state.locked &&
        !widget.busy &&
        widget.project != null &&
        widget.onSplitClip != null &&
        _compareProjectRational(previewPosition, selectedClip.timelineStart) >
            0 &&
        _compareProjectRational(previewPosition, selectedClip.timelineEnd) < 0;
    final canDelete =
        selectedClip != null &&
        selectedTrack != null &&
        !selectedTrack.state.locked &&
        !widget.busy &&
        widget.project != null &&
        widget.onDeleteClip != null;
    return Container(
      height: 34,
      padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x1),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (selectedClip != null) ...[
              Padding(
                padding: const EdgeInsets.only(left: OrSpacing.x1),
                child: Text(
                  _selectedClipStripLabel(selectedClip),
                  key: const ValueKey('timeline-selection-label'),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: OrColors.text,
                    fontSize: OrType.tinySize,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
              const SizedBox(width: OrSpacing.x2),
              Tooltip(
                message: 'Split at playhead (${_shortcutLabel('B')})',
                child: TextButton.icon(
                  key: const ValueKey('timeline-selection-split'),
                  onPressed: canSplit
                      ? () => unawaited(
                          widget.onSplitClip!(
                            widget.project!,
                            selectedClip,
                            previewPosition,
                          ),
                        )
                      : null,
                  icon: const Icon(Icons.content_cut, size: 15),
                  label: const Text('Split at playhead'),
                ),
              ),
              IconButton(
                key: const ValueKey('timeline-selection-duplicate'),
                tooltip: 'Duplicate clip (${_shortcutLabel('D')})',
                visualDensity: VisualDensity.compact,
                onPressed: canDuplicate ? _duplicateSelectedClip : null,
                icon: const Icon(Icons.content_copy_outlined, size: 15),
              ),
              IconButton(
                key: const ValueKey('timeline-selection-delete'),
                tooltip: 'Delete clip (⌫)',
                visualDensity: VisualDensity.compact,
                onPressed: canDelete
                    ? () => unawaited(
                        widget.onDeleteClip!(widget.project!, selectedClip),
                      )
                    : null,
                icon: const Icon(Icons.delete_outline, size: 16),
              ),
              Builder(
                builder: (buttonContext) => IconButton(
                  key: const ValueKey('timeline-selection-actions'),
                  tooltip: 'Clip actions',
                  visualDensity: VisualDensity.compact,
                  onPressed: selectedTrack == null
                      ? null
                      : () {
                          final box =
                              buttonContext.findRenderObject() as RenderBox?;
                          final position = box == null
                              ? Offset.zero
                              : box.localToGlobal(box.size.center(Offset.zero));
                          unawaited(
                            _showClipMenu(
                              selectedTrack,
                              selectedClip,
                              position,
                            ),
                          );
                        },
                  icon: const Icon(Icons.more_horiz, size: 16),
                ),
              ),
              IconButton(
                key: const ValueKey('timeline-selection-clear'),
                tooltip: 'Clear selection',
                visualDensity: VisualDensity.compact,
                onPressed: _clearSelection,
                icon: const Icon(Icons.close, size: 15),
              ),
            ],
            const SizedBox(width: OrSpacing.x2),
            if (widget.isProjectWorkspace)
              Padding(
                padding: const EdgeInsets.only(right: OrSpacing.x1),
                child: Tooltip(
                  message: widget.bladeMode
                      ? 'Blade tool on · click a clip to cut it (V selects)'
                      : 'Blade tool (C) · click a clip to cut it',
                  child: IconButton(
                    key: const ValueKey('timeline-blade-toggle'),
                    isSelected: widget.bladeMode,
                    visualDensity: VisualDensity.compact,
                    style: IconButton.styleFrom(
                      backgroundColor: widget.bladeMode
                          ? OrColors.surfaceSelected
                          : null,
                      foregroundColor: widget.bladeMode
                          ? OrColors.selection
                          : OrColors.textSecondary,
                    ),
                    onPressed: widget.onToggleBladeMode,
                    icon: const Icon(Icons.content_cut, size: 16),
                  ),
                ),
              ),
            IconButton(
              key: const ValueKey('timeline-zoom-out'),
              tooltip: 'Zoom out',
              visualDensity: VisualDensity.compact,
              onPressed: _fitTimelineToView || _timelineZoom <= 0.125
                  ? null
                  : () => _changeTimelineZoom(_timelineZoom / 1.25),
              icon: const Icon(Icons.zoom_out, size: 16),
            ),
            SizedBox(
              key: const ValueKey('timeline-zoom-label'),
              width: 42,
              child: Text(
                _fitTimelineToView
                    ? 'Fit'
                    : '${(_timelineZoom * 100).round()}%',
                textAlign: TextAlign.center,
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.tinySize,
                ),
              ),
            ),
            IconButton(
              key: const ValueKey('timeline-zoom-in'),
              tooltip: 'Zoom in',
              visualDensity: VisualDensity.compact,
              onPressed: !_fitTimelineToView && _timelineZoom >= 8
                  ? null
                  : () => _changeTimelineZoom(
                      _fitTimelineToView ? 1.25 : _timelineZoom * 1.25,
                    ),
              icon: const Icon(Icons.zoom_in, size: 16),
            ),
            TextButton(
              key: const ValueKey('timeline-fit-zoom'),
              onPressed: _fitTimelineToView
                  ? null
                  : () {
                      setState(() => _fitTimelineToView = true);
                      WidgetsBinding.instance.addPostFrameCallback((_) {
                        if (_horizontalController.hasClients) {
                          _horizontalController.jumpTo(0);
                        }
                      });
                    },
              child: const Text('Fit'),
            ),
            if (widget.onSnapChanged != null) ...[
              Container(
                width: 1,
                height: 16,
                margin: const EdgeInsets.symmetric(horizontal: OrSpacing.x1),
                color: OrColors.border,
              ),
              // A FilterChip states its own selection to a screen reader and
              // to a test, which an ad-hoc InkWell does not.
              FilterChip(
                key: const ValueKey('timeline-snap-toggle'),
                selected: widget.snapEnabled,
                onSelected: widget.busy
                    ? null
                    : (value) => widget.onSnapChanged!(value),
                tooltip: widget.snapEnabled
                    ? 'Snapping on · hold Option to bypass'
                    : 'Snapping off',
                avatar: Icon(
                  Icons.straighten_outlined,
                  size: 14,
                  color: widget.snapEnabled
                      ? OrColors.selection
                      : OrColors.textMuted,
                ),
                label: const Text('Snap'),
                labelStyle: TextStyle(
                  fontSize: OrType.tinySize,
                  color: widget.snapEnabled
                      ? OrColors.text
                      : OrColors.textMuted,
                ),
                visualDensity: VisualDensity.compact,
                materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                showCheckmark: false,
                side: BorderSide(
                  color: widget.snapEnabled
                      ? OrColors.selection
                      : OrColors.border,
                ),
                backgroundColor: Colors.transparent,
                selectedColor: OrColors.surface,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(OrRadii.small),
                ),
              ),
            ],
            if (widget.isProjectWorkspace) ...[
              const SizedBox(width: OrSpacing.x1),
              PopupMenuButton<String>(
                key: const ValueKey('timeline-create-menu'),
                tooltip: 'Add a track, title, caption or marker',
                enabled: !widget.busy,
                icon: const Icon(Icons.add, size: 17),
                onSelected: (choice) {
                  switch (choice) {
                    case 'video':
                      widget.onAddVideoTrack?.call();
                    case 'audio':
                      widget.onAddAudioTrack?.call();
                    case 'text':
                      widget.onAddTextTrack?.call();
                    case 'caption':
                      widget.onAddCaptionTrack?.call();
                    case 'title':
                      widget.onAddTitle?.call();
                    case 'caption-clip':
                      widget.onAddCaption?.call();
                    case 'marker':
                      widget.onRequestMarker?.call();
                    case 'import-captions':
                      widget.onImportCaptions?.call();
                    case 'export-captions':
                      widget.onExportCaptions?.call();
                  }
                },
                itemBuilder: (context) => [
                  const PopupMenuItem(
                    value: 'video',
                    child: Text('Add video track'),
                  ),
                  const PopupMenuItem(
                    value: 'audio',
                    child: Text('Add audio track'),
                  ),
                  const PopupMenuItem(
                    value: 'text',
                    child: Text('Add text track'),
                  ),
                  const PopupMenuItem(
                    value: 'caption',
                    child: Text('Add caption track'),
                  ),
                  const PopupMenuDivider(),
                  const PopupMenuItem(value: 'title', child: Text('Add title')),
                  const PopupMenuItem(
                    value: 'caption-clip',
                    child: Text('Add caption'),
                  ),
                  const PopupMenuItem(
                    value: 'marker',
                    child: Text('Add marker at playhead'),
                  ),
                  const PopupMenuDivider(),
                  const PopupMenuItem(
                    value: 'import-captions',
                    child: Text('Import captions (SRT)…'),
                  ),
                  const PopupMenuItem(
                    value: 'export-captions',
                    child: Text('Export captions (SRT)…'),
                  ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }

  /// Current zoom, so a zoom key steps from where the timeline already is.
  double get timelineZoom => _timelineZoom;

  void changeZoom(double zoom) => _changeTimelineZoom(zoom);

  void fitToView() {
    if (_fitTimelineToView) return;
    setState(() => _fitTimelineToView = true);
    if (_horizontalController.hasClients) {
      _horizontalController.jumpTo(0);
    }
  }

  void copySelectedClip() {
    final project = widget.project;
    final clip = _selectedClip;
    final track = _selectedTrack;
    if (project == null || clip == null || track == null) return;
    final onCopy = widget.onCopyClip;
    if (onCopy == null) return;
    onCopy(project, track, clip);
  }

  /// Paste at the playhead on the selected track, which is where the eye is:
  /// the owner moved the playhead to the place the clip belongs.
  void pasteClipboardClip() {
    final project = widget.project;
    final track = _selectedTrack;
    final onPaste = widget.onPasteClip;
    var position = widget.previewState.value?.position;
    // A paste at the very start would sit on top of the first clip. When the
    // playhead is still at zero and a clip is selected, the clip the owner is
    // working with is the better anchor: paste lands right after it.
    final selected = _selectedClip;
    if (position != null &&
        position.numerator == BigInt.zero &&
        selected != null) {
      position = ProjectRationalTime.tryParse(selected.timelineEnd.canonical);
    }
    if (project == null ||
        track == null ||
        position == null ||
        track.state.locked ||
        widget.busy ||
        onPaste == null) {
      return;
    }
    onPaste(project, track, position);
  }

  void duplicateSelectedClip() {
    final project = widget.project;
    final clip = _selectedClip;
    final track = _selectedTrack;
    if (project == null ||
        clip == null ||
        track == null ||
        track.state.locked ||
        widget.busy ||
        widget.onDuplicateClip == null) {
      return;
    }
    unawaited(widget.onDuplicateClip!(project, track, clip));
  }

  void confirmDeleteSelectedClip() {
    final clip = _selectedClip;
    final track = _selectedTrack;
    final project = widget.project;
    final onDelete = widget.onDeleteClip;
    if (clip == null ||
        track == null ||
        track.state.locked ||
        project == null ||
        onDelete == null ||
        widget.busy) {
      return;
    }
    unawaited(
      _confirmDeleteTimelineClip(
        context: context,
        project: project,
        clip: clip,
        onDelete: onDelete,
      ),
    );
  }

  void clearSelection() {
    if (_selectedClipId == null) return;
    _clearSelection();
  }

  /// Split at the playhead. Shares the enablement rules with the selection
  /// strip so the keyboard and the button cannot disagree.
  void splitSelectedClipAtPlayhead() {
    final project = widget.project;
    final clip = _selectedClip;
    final track = _selectedTrack;
    final position = widget.previewState.value?.position;
    if (project == null ||
        clip == null ||
        track == null ||
        position == null ||
        track.state.locked ||
        widget.busy ||
        widget.onSplitClip == null) {
      return;
    }
    if (_compareProjectRational(position, clip.timelineStart) <= 0 ||
        _compareProjectRational(position, clip.timelineEnd) >= 0) {
      return;
    }
    unawaited(widget.onSplitClip!(project, clip, position));
  }

  /// Ripple delete: removes the clip and closes the gap in one command.
  void rippleDeleteSelectedClip() {
    final project = widget.project;
    final clip = _selectedClip;
    final track = _selectedTrack;
    if (project == null ||
        clip == null ||
        track == null ||
        track.state.locked ||
        widget.busy ||
        widget.onRippleDeleteClip == null) {
      return;
    }
    unawaited(widget.onRippleDeleteClip!(project, clip));
  }

  void _changeTimelineZoom(double zoom) {
    final leftTime = _horizontalController.hasClients && _pixelsPerSecond > 0
        ? _horizontalController.offset / _pixelsPerSecond
        : 0.0;
    setState(() {
      _fitTimelineToView = false;
      _timelineZoom = zoom.clamp(0.125, 8).toDouble();
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !_horizontalController.hasClients) return;
      final offset = (leftTime * _pixelsPerSecond)
          .clamp(0.0, _horizontalController.position.maxScrollExtent)
          .toDouble();
      _horizontalController.jumpTo(offset);
    });
  }

  ProjectTimelineClip? get _selectedClip {
    final id = _selectedClipId;
    if (id == null) return null;
    for (final page in widget.clipPages.values) {
      for (final clip in page.items) {
        if (clip.clipId == id) return clip;
      }
    }
    return null;
  }

  ProjectTimelineTrack? get _selectedTrack {
    final id = _selectedClipId;
    if (id == null) return null;
    for (final track
        in widget.tracks?.items ?? const <ProjectTimelineTrack>[]) {
      if (widget.clipPages[track.trackId]?.items.any(
            (clip) => clip.clipId == id,
          ) ??
          false) {
        return track;
      }
    }
    return null;
  }

  /// The full clip dialog, kept for the operations that need exact numbers.
  /// Reached from the context menu's "Advanced clip actions…" item.
  Future<void> _openClipDialog(
    ProjectTimelineTrack track,
    ProjectTimelineClip clip,
  ) async {
    final tracks = widget.tracks?.items ?? const <ProjectTimelineTrack>[];
    await _showTimelineClipActions(
      context: context,
      project: widget.project,
      tracks: tracks,
      track: track,
      clip: clip,
      playhead: widget.previewState.value?.position,
      onMove: widget.onMoveClip,
      onDuplicate: widget.onDuplicateClip,
      onDelete: widget.onDeleteClip,
      onTrim: widget.onTrimClip,
      onSplit: widget.onSplitClip,
      onSlip: widget.onSlipClip,
      onRoll: widget.onRollClip,
      onSlide: widget.onSlideClip,
      onRippleDelete: widget.onRippleDeleteClip,
      onEditTextClip: widget.onUpdateTextClip,
      busy: widget.busy,
    );
    if (mounted) _timelineFocusNode.requestFocus();
  }

  /// Context menu for the frequent clip operations. Right-clicking a clip or
  /// the selection strip's "…" button opens it; the old modal-on-tap is gone.
  Future<void> _showClipMenu(
    ProjectTimelineTrack track,
    ProjectTimelineClip clip,
    Offset globalPosition,
  ) async {
    final project = widget.project;
    if (project == null) return;
    if (track.state.locked) {
      _showLockedTrackNotice();
      return;
    }
    final playhead = widget.previewState.value?.position;
    final canSplit =
        playhead != null &&
        _compareProjectRational(playhead, clip.timelineStart) > 0 &&
        _compareProjectRational(playhead, clip.timelineEnd) < 0 &&
        widget.onSplitClip != null &&
        !widget.busy;
    final overlay =
        Overlay.of(context).context.findRenderObject()! as RenderBox;
    final choice = await showMenu<_ClipMenuChoice>(
      context: context,
      position: RelativeRect.fromRect(
        Rect.fromPoints(globalPosition, globalPosition),
        Offset.zero & overlay.size,
      ),
      color: OrColors.backgroundRaised,
      items: [
        PopupMenuItem<_ClipMenuChoice>(
          enabled: false,
          height: 34,
          child: Text(
            _selectedClipStripLabel(clip),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.tinySize,
            ),
          ),
        ),
        const PopupMenuDivider(),
        PopupMenuItem<_ClipMenuChoice>(
          value: _ClipMenuChoice.split,
          enabled: canSplit,
          child: _ClipMenuItem('Split at playhead', _shortcutLabel('B')),
        ),
        PopupMenuItem<_ClipMenuChoice>(
          value: _ClipMenuChoice.duplicate,
          enabled: widget.onDuplicateClip != null && !widget.busy,
          child: _ClipMenuItem('Duplicate', _shortcutLabel('D')),
        ),
        PopupMenuItem<_ClipMenuChoice>(
          value: _ClipMenuChoice.reverse,
          enabled:
              widget.onSetClipReversed != null &&
              !widget.busy &&
              clip.contentKind == ProjectTimelineClipContentKind.media,
          child: _ClipMenuItem(
            clip.reversed ? 'Play forwards' : 'Play backwards',
            '',
          ),
        ),
        PopupMenuItem<_ClipMenuChoice>(
          value: _ClipMenuChoice.delete,
          enabled: widget.onDeleteClip != null && !widget.busy,
          child: _ClipMenuItem(
            'Delete',
            _deleteShortcutLabel(),
            destructive: true,
          ),
        ),
        PopupMenuItem<_ClipMenuChoice>(
          value: _ClipMenuChoice.rippleDelete,
          enabled: widget.onRippleDeleteClip != null && !widget.busy,
          child: const _ClipMenuItem('Ripple delete', '⇧⌫', destructive: true),
        ),
        const PopupMenuDivider(),
        PopupMenuItem<_ClipMenuChoice>(
          value: _ClipMenuChoice.advanced,
          child: const _ClipMenuItem('Advanced clip actions…', ''),
        ),
      ],
    );
    switch (choice) {
      case null:
        return;
      case _ClipMenuChoice.split:
        if (playhead != null) {
          await widget.onSplitClip!(project, clip, playhead);
        }
      case _ClipMenuChoice.duplicate:
        await widget.onDuplicateClip!(project, track, clip);
      case _ClipMenuChoice.reverse:
        await widget.onSetClipReversed!(
          project,
          track.trackId,
          clip.clipId,
          !clip.reversed,
        );
      case _ClipMenuChoice.delete:
        await widget.onDeleteClip!(project, clip);
      case _ClipMenuChoice.rippleDelete:
        await widget.onRippleDeleteClip!(project, clip);
      case _ClipMenuChoice.advanced:
        await _openClipDialog(track, clip);
    }
  }

  void _showLockedTrackNotice() {
    OrSnackBar.show(
      context,
      'This track is locked. Unlock it to edit its clips.',
      bottomInset: 0,
    );
  }

  String _selectedClipStripLabel(ProjectTimelineClip clip) {
    final text = clip.text?.trim() ?? '';
    var name = '';
    if (text.isNotEmpty) {
      name = text.length > 40 ? '${text.substring(0, 40)}…' : text;
    }
    if (name.isEmpty) {
      name = switch (clip.contentKind) {
        ProjectTimelineClipContentKind.text => 'Text clip',
        ProjectTimelineClipContentKind.caption => 'Caption clip',
        ProjectTimelineClipContentKind.media => 'Clip',
      };
      final mediaId = clip.mediaId;
      if (mediaId != null) {
        for (final item in widget.mediaItems) {
          if (item.mediaId == mediaId) {
            name = _mediaDisplayName(item.sourceUri);
            break;
          }
        }
      }
    }
    final start = _clockLabel(_rationalSeconds(clip.timelineStart));
    final end = _clockLabel(_rationalSeconds(clip.timelineEnd));
    return '$name · $start – $end';
  }

  void _duplicateSelectedClip() {
    final project = widget.project;
    final clip = _selectedClip;
    final track = _selectedTrack;
    final onDuplicate = widget.onDuplicateClip;
    if (project == null ||
        clip == null ||
        track == null ||
        onDuplicate == null) {
      return;
    }
    unawaited(onDuplicate(project, track, clip));
  }

  void _beginMoveGesture(
    ProjectTimelineTrack track,
    ProjectTimelineClip clip,
    DragStartDetails details,
  ) {
    final project = widget.project;
    if (project == null || widget.busy || widget.onMoveClip == null) return;
    final token = ++_gestureToken;
    setState(() {
      _selectedClipId = clip.clipId;
      _gesture = _TimelinePointerGesture.move(
        token: token,
        project: project,
        sourceTrack: track,
        clip: clip,
        startGlobalPosition: details.globalPosition,
        pixelsPerSecond: _pixelsPerSecondForGesture(),
      );
    });
    widget.onClipSelectionChanged?.call(track.trackId, clip.clipId, track.kind);
  }

  void _updateMoveGesture(ProjectTimelineClip clip, DragUpdateDetails details) {
    final gesture = _gesture;
    if (gesture == null ||
        gesture.kind != _TimelinePointerEditKind.move ||
        gesture.clip.clipId != clip.clipId) {
      return;
    }
    gesture.deltaPixels =
        details.globalPosition.dx - gesture.startGlobalPosition.dx;
    final target = _trackAt(details.globalPosition);
    if (target == null) {
      gesture.targetTrackId = gesture.sourceTrack.trackId;
      gesture.invalidLane = false;
    } else if (target.kind == gesture.sourceTrack.kind) {
      gesture.targetTrackId = target.trackId;
      gesture.invalidLane = false;
    } else {
      gesture.targetTrackId = gesture.sourceTrack.trackId;
      gesture.invalidLane = true;
    }
    setState(() {});
  }

  void _endMoveGesture(ProjectTimelineClip clip, DragEndDetails details) {
    final gesture = _gesture;
    if (gesture == null ||
        gesture.kind != _TimelinePointerEditKind.move ||
        gesture.clip.clipId != clip.clipId) {
      return;
    }
    _gesture = null;
    setState(() {});
    if (gesture.invalidLane) return;
    unawaited(_commitPointerGesture(gesture));
  }

  void _beginTrimGesture(
    ProjectTimelineTrack track,
    ProjectTimelineClip clip,
    ProjectTimelineTrimEdge edge,
    DragStartDetails details,
  ) {
    final project = widget.project;
    if (project == null || widget.busy || widget.onTrimClip == null) return;
    final token = ++_gestureToken;
    setState(() {
      _selectedClipId = clip.clipId;
      _gesture = _TimelinePointerGesture.trim(
        token: token,
        project: project,
        sourceTrack: track,
        clip: clip,
        edge: edge,
        startGlobalPosition: details.globalPosition,
        pixelsPerSecond: _pixelsPerSecondForGesture(),
      );
    });
    widget.onClipSelectionChanged?.call(track.trackId, clip.clipId, track.kind);
  }

  void _selectClip(ProjectTimelineTrack track, ProjectTimelineClip clip) {
    setState(() => _selectedClipId = clip.clipId);
    widget.onClipSelectionChanged?.call(track.trackId, clip.clipId, track.kind);
  }

  void _clearSelection() {
    setState(() => _selectedClipId = null);
    widget.onClipSelectionChanged?.call(null, null, null);
  }

  void _updateTrimGesture(ProjectTimelineClip clip, DragUpdateDetails details) {
    final gesture = _gesture;
    if (gesture == null ||
        gesture.kind == _TimelinePointerEditKind.move ||
        gesture.clip.clipId != clip.clipId) {
      return;
    }
    gesture.deltaPixels =
        details.globalPosition.dx - gesture.startGlobalPosition.dx;
    setState(() {});
  }

  void _endTrimGesture(ProjectTimelineClip clip, DragEndDetails details) {
    final gesture = _gesture;
    if (gesture == null ||
        gesture.kind == _TimelinePointerEditKind.move ||
        gesture.clip.clipId != clip.clipId) {
      return;
    }
    _gesture = null;
    setState(() {});
    unawaited(_commitPointerGesture(gesture));
  }

  void _cancelPointerGesture(ProjectTimelineClip clip) {
    if (_gesture?.clip.clipId != clip.clipId) return;
    setState(_cancelGesture);
  }

  Future<void> _commitPointerGesture(_TimelinePointerGesture gesture) async {
    final rawTargetTime = gesture.rawTargetTime;
    var resolvedTargetTime = rawTargetTime;
    try {
      final operation = gesture.snapOperation;
      if (widget.snapEnabled && widget.onResolveSnap != null) {
        final snap = await widget.onResolveSnap!(
          gesture.project,
          operation,
          gesture.clip.clipId,
          gesture.kind == _TimelinePointerEditKind.move
              ? gesture.targetTrackId
              : null,
          rawTargetTime,
        );
        if (!_snapResultIsCurrent(gesture, snap)) {
          if (mounted) widget.onRefresh?.call();
          return;
        }
        if (snap.snapped) {
          resolvedTargetTime = snap.resolvedTargetTime;
          _showSnapGuide(gesture, snap);
        }
      }
      if (!_gestureCommitIsCurrent(gesture)) {
        if (mounted) widget.onRefresh?.call();
        return;
      }
      if (gesture.kind == _TimelinePointerEditKind.move) {
        final onMove = widget.onMoveClip;
        if (onMove == null) return;
        await onMove(
          gesture.project,
          gesture.clip,
          gesture.targetTrackId,
          resolvedTargetTime,
        );
      } else {
        final onTrim = widget.onTrimClip;
        if (onTrim == null) return;
        await onTrim(
          gesture.project,
          gesture.clip,
          gesture.edge,
          resolvedTargetTime,
          false,
        );
      }
    } on ProjectGatewayException catch (error) {
      if (!mounted) return;
      widget.onTimelineEditError?.call(
        error.message.isEmpty ? 'The timeline edit failed.' : error.message,
      );
      widget.onRefresh?.call();
    } catch (_) {
      if (!mounted) return;
      widget.onTimelineEditError?.call('The timeline edit failed.');
      widget.onRefresh?.call();
    }
  }

  bool _snapResultIsCurrent(
    _TimelinePointerGesture gesture,
    ProjectTimelineSnapResult result,
  ) =>
      _gestureCommitIsCurrent(gesture) &&
      result.projectId == gesture.project.projectId &&
      result.projectInstanceId == gesture.project.projectInstanceId &&
      result.projectRevision == gesture.project.revision &&
      result.rawTargetTime.canonical == gesture.rawTargetTime.canonical;

  bool _gestureCommitIsCurrent(_TimelinePointerGesture gesture) =>
      mounted &&
      _gestureToken == gesture.token &&
      widget.project?.projectId == gesture.project.projectId &&
      widget.project?.projectInstanceId == gesture.project.projectInstanceId &&
      widget.project?.revision == gesture.project.revision;

  void _showSnapGuide(
    _TimelinePointerGesture gesture,
    ProjectTimelineSnapResult snap,
  ) {
    if (!mounted || !_gestureCommitIsCurrent(gesture)) return;
    setState(() {
      _snapGuide = _TimelineSnapGuide(
        trackId: gesture.targetTrackId,
        time: snap.targetTime,
        targetKind: snap.targetKind,
        markerId: snap.targetMarkerId,
      );
    });
    final token = gesture.token;
    Future<void>.delayed(const Duration(milliseconds: 650), () {
      if (!mounted || _gestureToken != token) return;
      setState(() => _snapGuide = null);
    });
  }

  void _cancelGesture() {
    _gestureToken++;
    _gesture = null;
    _snapGuide = null;
  }

  void _dropMediaOnTrack(
    ProjectTimelineTrack track,
    ProjectMediaItem media,
    Offset globalPosition,
    double pixelsPerSecond,
  ) {
    final project = widget.project;
    final tracks = widget.tracks;
    final insert = widget.onAddMediaToTimeline;
    if (project == null ||
        tracks == null ||
        insert == null ||
        widget.busy ||
        track.state.locked ||
        !_mediaSupportsTrack(media, track.kind) ||
        tracks.projectId != project.projectId ||
        tracks.projectInstanceId != project.projectInstanceId ||
        tracks.projectRevision != project.revision ||
        pixelsPerSecond <= 0) {
      widget.onRefresh?.call();
      return;
    }
    final renderObject = _laneKeys[track.trackId]?.currentContext
        ?.findRenderObject();
    if (renderObject is! RenderBox) return;
    final localX = renderObject.globalToLocal(globalPosition).dx;
    if (!localX.isFinite) return;
    final milliseconds =
        (localX.clamp(0.0, renderObject.size.width) / pixelsPerSecond * 1000)
            .round();
    final timelineStart = ProjectRationalTime.tryParse('$milliseconds/1000');
    final duration = _defaultTimelineDuration(media, track.kind);
    if (timelineStart == null || duration == null) return;
    unawaited(
      insert(
        project,
        media,
        track.trackId,
        timelineStart,
        ProjectRationalTime(BigInt.zero, 1),
        duration,
        'insert',
      ),
    );
  }

  ProjectTimelineTrack? _trackAt(Offset globalPosition) {
    final tracks = widget.tracks?.items ?? const <ProjectTimelineTrack>[];
    for (final track in tracks) {
      final context = _laneKeys[track.trackId]?.currentContext;
      final renderObject = context?.findRenderObject();
      if (renderObject is! RenderBox) continue;
      final origin = renderObject.localToGlobal(Offset.zero);
      final rect = origin & renderObject.size;
      if (rect.contains(globalPosition)) return track;
    }
    return null;
  }

  double _pixelsPerSecondForGesture() {
    return _pixelsPerSecond;
  }
}

/// Width of the track-header gutter beside each timeline lane.
///
/// The gutter has to hold two 11 px labels plus the track controls inside a
/// fixed-height lane, and the label column carries a "loaded / total" count
/// that wraps when it is squeezed, overflowing the lane. The wide layout gets
/// enough room for a three-digit count on one line; compact keeps the legacy
/// width so the gutter still fits a small screen.
/// The track header must hold its name, its clip count and its four controls: the
/// enabled, lock and solo toggles and the move menu. The desktop width grew by the
/// move control's footprint, because squeezing the count is worse than giving the
/// timeline 32 fewer pixels; the compact width fits the same controls at compact
/// density and is guarded at 320x440 and 320x480.
double _timelineTrackHeaderWidthFor(double availableWidth) =>
    OrBreakpoints.isCompact(availableWidth) ? 272 : 368;
const double _timelineRulerHeight = 38;
const double _timelineLaneHeight = 58;

Map<String, String> _timelineTrackLabels(List<ProjectTimelineTrack> tracks) {
  var video = 0;
  var audio = 0;
  var text = 0;
  var caption = 0;
  return {
    for (final track in tracks)
      track.trackId: switch (track.kind) {
        ProjectTimelineTrackKind.video => 'V${++video}',
        ProjectTimelineTrackKind.audio => 'A${++audio}',
        ProjectTimelineTrackKind.text => 'T${++text}',
        ProjectTimelineTrackKind.caption => 'C${++caption}',
      },
  };
}

double _timelineTickSeconds(double duration, double width) {
  final marks = (width / 100).ceil().clamp(1, 1000);
  final needed = duration / marks;
  return math.max(5, (needed / 5).ceil() * 5).toDouble();
}

enum _TimelinePointerEditKind { move, trim }

class _TimelinePointerGesture {
  _TimelinePointerGesture.move({
    required this.token,
    required this.project,
    required this.sourceTrack,
    required this.clip,
    required this.startGlobalPosition,
    required this.pixelsPerSecond,
  }) : kind = _TimelinePointerEditKind.move,
       edge = ProjectTimelineTrimEdge.start,
       targetTrackId = sourceTrack.trackId;

  _TimelinePointerGesture.trim({
    required this.token,
    required this.project,
    required this.sourceTrack,
    required this.clip,
    required this.edge,
    required this.startGlobalPosition,
    required this.pixelsPerSecond,
  }) : kind = _TimelinePointerEditKind.trim,
       targetTrackId = sourceTrack.trackId;

  final int token;
  final ProjectReadModel project;
  final ProjectTimelineTrack sourceTrack;
  final ProjectTimelineClip clip;
  final _TimelinePointerEditKind kind;
  final ProjectTimelineTrimEdge edge;
  final Offset startGlobalPosition;
  final double pixelsPerSecond;
  String targetTrackId;
  double deltaPixels = 0;
  bool invalidLane = false;

  double get deltaSeconds => deltaPixels / pixelsPerSecond;

  ProjectRationalTime get pointerDelta {
    final milliseconds = (deltaSeconds * 1000).round();
    return ProjectRationalTime(BigInt.from(milliseconds), 1000);
  }

  ProjectRationalTime get rawTargetTime {
    final delta = pointerDelta;
    return switch (kind) {
      _TimelinePointerEditKind.move => clip.timelineStart.add(delta),
      _TimelinePointerEditKind.trim =>
        edge == ProjectTimelineTrimEdge.start
            ? clip.timelineStart.add(delta)
            : clip.timelineEnd.add(delta),
    };
  }

  ProjectTimelineSnapOperation get snapOperation => switch (kind) {
    _TimelinePointerEditKind.move => ProjectTimelineSnapOperation.move,
    _TimelinePointerEditKind.trim =>
      edge == ProjectTimelineTrimEdge.start
          ? ProjectTimelineSnapOperation.trimStart
          : ProjectTimelineSnapOperation.trimEnd,
  };

  double get visualStartSeconds => switch (kind) {
    _TimelinePointerEditKind.move =>
      clip.timelineStart.secondsForDisplay + deltaSeconds,
    _TimelinePointerEditKind.trim =>
      clip.timelineStart.secondsForDisplay +
          (edge == ProjectTimelineTrimEdge.start ? deltaSeconds : 0),
  };

  double get visualDurationSeconds => switch (kind) {
    _TimelinePointerEditKind.move => clip.timelineDuration.secondsForDisplay,
    _TimelinePointerEditKind.trim =>
      clip.timelineDuration.secondsForDisplay +
          (edge == ProjectTimelineTrimEdge.start
              ? -deltaSeconds
              : deltaSeconds),
  };
}

class _TimelineSnapGuide {
  const _TimelineSnapGuide({
    required this.trackId,
    required this.time,
    required this.targetKind,
    this.markerId,
  });

  final String trackId;
  final ProjectRationalTime time;
  final ProjectTimelineSnapTargetKind targetKind;
  final String? markerId;
}

String _timelineTimeLabel(double seconds) {
  final value = BigInt.tryParse(seconds.toStringAsFixed(0)) ?? BigInt.zero;
  final hours = value ~/ BigInt.from(3600);
  final minutes = (value % BigInt.from(3600)) ~/ BigInt.from(60);
  final remainder = value % BigInt.from(60);
  final mm = minutes.toString().padLeft(2, '0');
  final ss = remainder.toString().padLeft(2, '0');
  return hours == BigInt.zero
      ? '$mm:$ss'
      : '${hours.toString().padLeft(2, '0')}:$mm:$ss';
}

int _compareProjectRational(
  ProjectRationalTime left,
  ProjectRationalTime right,
) => (left.numerator * BigInt.from(right.denominator)).compareTo(
  right.numerator * BigInt.from(left.denominator),
);

class _TimelineRulerPainter extends CustomPainter {
  const _TimelineRulerPainter({
    required this.pixelsPerSecond,
    required this.tickSeconds,
  });

  final double pixelsPerSecond;
  final double tickSeconds;

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = OrColors.surface);
    final line = Paint()
      ..color = OrColors.border
      ..strokeWidth = 1;
    final textStyle = const TextStyle(
      color: OrColors.textMuted,
      fontFamily: 'monospace',
      fontSize: OrType.tinySize,
    );
    for (var index = 0; index <= 1000; index++) {
      final seconds = index * tickSeconds;
      final x = seconds * pixelsPerSecond;
      if (!x.isFinite || x > size.width) break;
      canvas.drawLine(Offset(x, size.height - 8), Offset(x, size.height), line);
      final label = _timelineTimeLabel(seconds);
      final painter = TextPainter(
        text: TextSpan(text: label, style: textStyle),
        textDirection: TextDirection.ltr,
      )..layout();
      painter.paint(canvas, Offset(x + 4, 2));
    }
    canvas.drawLine(
      Offset(0, size.height - 0.5),
      Offset(size.width, size.height - 0.5),
      line,
    );
  }

  @override
  bool shouldRepaint(_TimelineRulerPainter oldDelegate) =>
      oldDelegate.pixelsPerSecond != pixelsPerSecond ||
      oldDelegate.tickSeconds != tickSeconds;
}

class _TimelineMarkerRuler extends StatefulWidget {
  const _TimelineMarkerRuler({
    required this.markers,
    required this.playhead,
    required this.pixelsPerSecond,
    required this.tickSeconds,
    required this.width,
    required this.loadingMore,
    required this.onLoadMore,
    required this.busy,
    required this.snapGuide,
    required this.onOpenMarker,
    required this.onMoveMarker,
  });

  final List<ProjectTimelineMarker> markers;
  final ProjectRationalTime? playhead;
  final double pixelsPerSecond;
  final double tickSeconds;
  final double width;
  final bool loadingMore;
  final VoidCallback? onLoadMore;
  final bool busy;
  final _TimelineSnapGuide? snapGuide;
  final ValueChanged<ProjectTimelineMarker> onOpenMarker;
  final Future<void> Function(ProjectTimelineMarker, ProjectRationalTime)?
  onMoveMarker;

  @override
  State<_TimelineMarkerRuler> createState() => _TimelineMarkerRulerState();
}

class _TimelineMarkerRulerState extends State<_TimelineMarkerRuler> {
  ProjectTimelineMarker? _draggingMarker;
  Offset? _dragStart;
  double _dragDeltaPixels = 0;

  @override
  Widget build(BuildContext context) {
    final markerWidgets = <Widget>[
      Positioned.fill(
        child: CustomPaint(
          painter: _TimelineRulerPainter(
            pixelsPerSecond: widget.pixelsPerSecond,
            tickSeconds: widget.tickSeconds,
          ),
        ),
      ),
      if (widget.playhead != null)
        Positioned(
          key: const ValueKey('timeline-playhead-ruler'),
          left: _xFor(widget.playhead!),
          top: 0,
          bottom: 0,
          width: 2,
          child: const IgnorePointer(
            child: ColoredBox(color: OrColors.selection),
          ),
        ),
      for (final marker in widget.markers) ..._markerWidgets(marker),
      if (widget.snapGuide?.targetKind == ProjectTimelineSnapTargetKind.marker)
        Positioned(
          key: const ValueKey('timeline-snap-feedback'),
          left: _xFor(widget.snapGuide!.time),
          top: 0,
          bottom: 0,
          width: 2,
          child: ColoredBox(color: OrColors.selection),
        ),
      if (widget.snapGuide?.targetKind == ProjectTimelineSnapTargetKind.marker)
        Positioned(
          top: 1,
          right: widget.onLoadMore == null ? 4 : 92,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: OrColors.selection,
              borderRadius: BorderRadius.circular(OrRadii.small),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 2),
              child: Text(
                'Snap: marker',
                style: const TextStyle(
                  color: OrColors.primaryText,
                  fontSize: OrType.tinySize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
        ),
      if (widget.onLoadMore != null)
        Positioned(
          top: 0,
          right: 2,
          child: TextButton.icon(
            key: const ValueKey('timeline-marker-load-more'),
            onPressed: widget.busy || widget.loadingMore
                ? null
                : widget.onLoadMore,
            icon: widget.loadingMore
                ? const SizedBox.square(
                    dimension: 11,
                    child: CircularProgressIndicator(strokeWidth: 1.5),
                  )
                : const Icon(Icons.expand_more, size: 13),
            label: const Text('Markers'),
            style: TextButton.styleFrom(
              minimumSize: const Size(0, 28),
              padding: const EdgeInsets.symmetric(horizontal: 4),
              textStyle: const TextStyle(fontSize: OrType.tinySize),
            ),
          ),
        ),
    ];
    return DecoratedBox(
      decoration: const BoxDecoration(
        color: OrColors.surface,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Stack(clipBehavior: Clip.hardEdge, children: markerWidgets),
    );
  }

  List<Widget> _markerWidgets(ProjectTimelineMarker marker) {
    final moving = _draggingMarker?.markerId == marker.markerId;
    final time = moving ? _dragTargetTime(marker) : marker.timelineTime;
    final x = _xFor(time);
    final labelWidth = math.min(110.0, math.max(54.0, widget.width - 8));
    final labelLeft = (x - labelWidth / 2).clamp(
      2.0,
      math.max(2.0, widget.width - labelWidth - 2),
    );
    final label = marker.label.isEmpty ? 'Marker' : marker.label;
    final handle = Semantics(
      button: true,
      label: 'Marker $label at ${time.canonical}',
      hint: 'Drag to move. Activate for marker actions.',
      child: GestureDetector(
        key: ValueKey('timeline-marker-${marker.markerId}'),
        behavior: HitTestBehavior.opaque,
        onTap: () => widget.onOpenMarker(marker),
        onPanStart: widget.busy || widget.onMoveMarker == null
            ? null
            : (details) {
                setState(() {
                  _draggingMarker = marker;
                  _dragStart = details.globalPosition;
                  _dragDeltaPixels = 0;
                });
              },
        onPanUpdate: widget.busy || widget.onMoveMarker == null
            ? null
            : (details) {
                if (_draggingMarker?.markerId != marker.markerId ||
                    _dragStart == null) {
                  return;
                }
                setState(() {
                  _dragDeltaPixels = details.globalPosition.dx - _dragStart!.dx;
                });
              },
        onPanEnd: widget.busy || widget.onMoveMarker == null
            ? null
            : (_) {
                if (_draggingMarker?.markerId != marker.markerId) return;
                final target = _dragTargetTime(marker);
                setState(() {
                  _draggingMarker = null;
                  _dragStart = null;
                  _dragDeltaPixels = 0;
                });
                unawaited(widget.onMoveMarker!(marker, target));
              },
        onPanCancel: widget.busy || widget.onMoveMarker == null
            ? null
            : () {
                if (_draggingMarker?.markerId != marker.markerId) return;
                setState(() {
                  _draggingMarker = null;
                  _dragStart = null;
                  _dragDeltaPixels = 0;
                });
              },
        child: Container(
          width: labelWidth,
          height: _timelineRulerHeight,
          alignment: Alignment.topCenter,
          padding: const EdgeInsets.only(top: 2),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: moving ? OrColors.selection : OrColors.backgroundRaised,
              border: Border.all(
                color: moving ? OrColors.selection : OrColors.borderStrong,
              ),
              borderRadius: BorderRadius.circular(OrRadii.small),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  color: moving ? OrColors.primaryText : OrColors.text,
                  fontSize: OrType.tinySize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
        ),
      ),
    );
    return [
      Positioned(
        left: x - 1,
        top: 0,
        bottom: 0,
        width: 2,
        child: IgnorePointer(
          child: ColoredBox(
            color: moving ? OrColors.selection : OrColors.borderStrong,
          ),
        ),
      ),
      Positioned(left: labelLeft.toDouble(), top: 0, child: handle),
    ];
  }

  double _xFor(ProjectRationalTime time) =>
      (time.secondsForDisplay * widget.pixelsPerSecond)
          .clamp(0.0, math.max(0.0, widget.width - 1))
          .toDouble();

  ProjectRationalTime _dragTargetTime(ProjectTimelineMarker marker) {
    final milliseconds = (_dragDeltaPixels / widget.pixelsPerSecond * 1000)
        .round();
    final target = marker.timelineTime.add(
      ProjectRationalTime(BigInt.from(milliseconds), 1000),
    );
    return target.numerator < BigInt.zero
        ? ProjectRationalTime(BigInt.zero, 1)
        : target;
  }
}

class _TimelineGridPainter extends CustomPainter {
  const _TimelineGridPainter({
    required this.pixelsPerSecond,
    required this.tickSeconds,
  });

  final double pixelsPerSecond;
  final double tickSeconds;

  @override
  void paint(Canvas canvas, Size size) {
    final line = Paint()
      ..color = OrColors.border.withValues(alpha: 0.65)
      ..strokeWidth = 1;
    for (var index = 0; index <= 1000; index++) {
      final x = index * tickSeconds * pixelsPerSecond;
      if (!x.isFinite || x > size.width) break;
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), line);
    }
    canvas.drawLine(
      Offset(0, size.height - 0.5),
      Offset(size.width, size.height - 0.5),
      Paint()
        ..color = OrColors.border
        ..strokeWidth = 1,
    );
  }

  @override
  bool shouldRepaint(_TimelineGridPainter oldDelegate) =>
      oldDelegate.pixelsPerSecond != pixelsPerSecond ||
      oldDelegate.tickSeconds != tickSeconds;
}

class _TimelineTrackHeader extends StatelessWidget {
  const _TimelineTrackHeader({
    required this.track,
    required this.label,
    required this.countLabel,
    required this.trackIndex,
    required this.trackCount,
    required this.busy,
    required this.project,
    required this.onRemove,
    required this.onSetState,
    required this.onMove,
  });

  final ProjectTimelineTrack track;
  final String label;
  final String countLabel;
  final int trackIndex;
  final int trackCount;
  final bool busy;
  final ProjectReadModel? project;
  final Future<void> Function(ProjectReadModel, ProjectTimelineTrack)? onRemove;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineTrackState,
  )?
  onSetState;
  final Future<void> Function(ProjectReadModel, ProjectTimelineTrack, int)?
  onMove;

  @override
  Widget build(BuildContext context) {
    final visual = track.kind != ProjectTimelineTrackKind.audio;
    final kind = switch (track.kind) {
      ProjectTimelineTrackKind.video => 'Video',
      ProjectTimelineTrackKind.audio => 'Audio',
      ProjectTimelineTrackKind.text => 'Text',
      ProjectTimelineTrackKind.caption => 'Caption',
    };
    final icon = switch (track.kind) {
      ProjectTimelineTrackKind.video => Icons.movie_outlined,
      ProjectTimelineTrackKind.audio => Icons.graphic_eq_outlined,
      ProjectTimelineTrackKind.text => Icons.title,
      ProjectTimelineTrackKind.caption => Icons.subtitles_outlined,
    };
    final enabled = visual ? track.state.visible : !track.state.muted;
    final removable = track.clipCount == 0 && !busy && project != null;
    void submit(ProjectTimelineTrackState state) {
      final handler = onSetState;
      final currentProject = project;
      if (busy || handler == null || currentProject == null) return;
      unawaited(handler(currentProject, track, state));
    }

    return Semantics(
      container: true,
      label:
          '$kind track $label, ${track.clipCount} clips, ${enabled ? 'enabled' : 'disabled'}, ${track.state.locked ? 'locked' : 'unlocked'}, ${track.state.solo ? 'solo' : 'not solo'}, position ${trackIndex + 1} of $trackCount',
      child: Container(
        height: _timelineLaneHeight,
        padding: const EdgeInsets.only(left: OrSpacing.x2),
        decoration: const BoxDecoration(
          color: OrColors.backgroundRaised,
          border: Border(
            right: BorderSide(color: OrColors.border),
            bottom: BorderSide(color: OrColors.border),
          ),
        ),
        child: Row(
          children: [
            Icon(icon, size: 15, color: OrColors.textSecondary),
            const SizedBox(width: OrSpacing.x1),
            Expanded(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      fontSize: OrType.tinySize,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Text(
                    countLabel,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      fontSize: OrType.tinySize,
                      color: OrColors.textMuted,
                    ),
                  ),
                ],
              ),
            ),
            IconButton(
              key: ValueKey('timeline-track-enabled-${track.trackId}'),
              tooltip: visual
                  ? (track.state.visible ? 'Hide track' : 'Show track')
                  : (track.state.muted ? 'Unmute track' : 'Mute track'),
              onPressed: busy || project == null || onSetState == null
                  ? null
                  : () => submit(
                      visual
                          ? track.state.copyWith(visible: !track.state.visible)
                          : track.state.copyWith(muted: !track.state.muted),
                    ),
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints.tightFor(width: 40, height: 40),
              visualDensity: VisualDensity.standard,
              icon: Icon(
                visual
                    ? (track.state.visible
                          ? Icons.visibility_outlined
                          : Icons.visibility_off_outlined)
                    : (track.state.muted
                          ? Icons.volume_off_outlined
                          : Icons.volume_up_outlined),
                size: 15,
              ),
            ),
            // One control carrying both moves: two more icon buttons overflowed
            // the 272 px compact header, which the compact guards caught.
            // A tooltip on a custom child does not reach the semantics tree, so the
            // control announces which track it moves.
            Semantics(
              // Its own node: the header is a semantics container, so without this
              // the control's label merges into the track's and nothing can address
              // the menu (a screen reader reads one long track node).
              container: true,
              label: 'Move track $label',
              button: true,
              child: PopupMenuButton<int>(
                key: ValueKey('timeline-track-move-${track.trackId}'),
                // The tooltip names the track, so the control is unambiguous to a
                // screen reader and to scripts/or_ui.py without a generated id.
                tooltip: 'Move track $label',
                enabled:
                    !busy &&
                    onMove != null &&
                    project != null &&
                    trackCount > 1,
                itemBuilder: (context) => [
                  PopupMenuItem(
                    value: trackIndex - 1,
                    enabled: trackIndex > 0,
                    child: const Text('Move up'),
                  ),
                  PopupMenuItem(
                    value: trackIndex + 1,
                    enabled: trackIndex + 1 < trackCount,
                    child: const Text('Move down'),
                  ),
                ],
                onSelected: (target) =>
                    unawaited(onMove!(project!, track, target)),
                child: const Padding(
                  padding: EdgeInsets.symmetric(horizontal: 6, vertical: 8),
                  child: Icon(Icons.swap_vert, size: 17),
                ),
              ),
            ),
            IconButton(
              key: ValueKey('timeline-track-lock-${track.trackId}'),
              tooltip: track.state.locked ? 'Unlock track' : 'Lock track',
              onPressed: busy || project == null || onSetState == null
                  ? null
                  : () => submit(
                      track.state.copyWith(locked: !track.state.locked),
                    ),
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints.tightFor(width: 40, height: 40),
              visualDensity: VisualDensity.standard,
              icon: Icon(
                track.state.locked
                    ? Icons.lock_outline
                    : Icons.lock_open_outlined,
                size: 15,
              ),
            ),
            IconButton(
              key: ValueKey('timeline-track-solo-${track.trackId}'),
              tooltip: track.state.solo ? 'Unsolo track' : 'Solo track',
              onPressed: busy || project == null || onSetState == null
                  ? null
                  : () => submit(track.state.copyWith(solo: !track.state.solo)),
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints.tightFor(width: 40, height: 40),
              visualDensity: VisualDensity.standard,
              icon: Icon(
                track.state.solo ? Icons.headphones : Icons.hearing,
                size: 15,
              ),
            ),
            Tooltip(
              message: track.clipCount == 0
                  ? 'Remove Track'
                  : 'Delete clips before removing this track.',
              child: IconButton(
                key: ValueKey('timeline-remove-track-${track.trackId}'),
                tooltip: track.clipCount == 0
                    ? 'Remove Track'
                    : 'Delete clips before removing this track.',
                onPressed: removable && onRemove != null
                    ? () => unawaited(onRemove!(project!, track))
                    : null,
                visualDensity: VisualDensity.standard,
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints.tightFor(
                  width: 40,
                  height: 40,
                ),
                icon: const Icon(Icons.remove_circle_outline, size: 16),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TimelineLoadMoreHeader extends StatelessWidget {
  const _TimelineLoadMoreHeader({
    required this.trackId,
    required this.width,
    required this.loading,
    required this.busy,
    required this.onLoadMore,
  });

  final String trackId;
  final double width;
  final bool loading;
  final bool busy;
  final ValueChanged<String>? onLoadMore;

  @override
  Widget build(BuildContext context) => Container(
    width: width,
    height: 32,
    alignment: Alignment.centerLeft,
    decoration: const BoxDecoration(
      color: OrColors.backgroundRaised,
      border: Border(
        right: BorderSide(color: OrColors.border),
        bottom: BorderSide(color: OrColors.border),
      ),
    ),
    child: TextButton.icon(
      key: ValueKey('timeline-load-more-$trackId'),
      onPressed: loading || busy || onLoadMore == null
          ? null
          : () => onLoadMore!(trackId),
      icon: loading
          ? const SizedBox.square(
              dimension: 13,
              child: CircularProgressIndicator(strokeWidth: 1.5),
            )
          : const Icon(Icons.expand_more, size: 15),
      label: const Text('Load more'),
      style: TextButton.styleFrom(
        minimumSize: const Size(0, 28),
        padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x2),
        textStyle: const TextStyle(fontSize: OrType.tinySize),
      ),
    ),
  );
}

class _TimelineLoadMoreLane extends StatelessWidget {
  const _TimelineLoadMoreLane({required this.width});

  final double width;

  @override
  Widget build(BuildContext context) => Container(
    height: 32,
    width: width,
    decoration: const BoxDecoration(
      color: OrColors.background,
      border: Border(bottom: BorderSide(color: OrColors.border)),
    ),
  );
}

enum _ClipMenuChoice { split, duplicate, reverse, delete, rippleDelete, advanced }

/// One context-menu row: label on the left, shortcut on the right.
class _ClipMenuItem extends StatelessWidget {
  const _ClipMenuItem(this.label, this.shortcut, {this.destructive = false});

  final String label;
  final String shortcut;
  final bool destructive;

  @override
  Widget build(BuildContext context) {
    final color = destructive ? OrColors.danger : OrColors.text;
    return Row(
      children: [
        Expanded(
          child: Text(
            label,
            style: TextStyle(
              color: color,
              fontSize: OrType.metadataCompactSize,
            ),
          ),
        ),
        if (shortcut.isNotEmpty)
          Text(
            shortcut,
            style: const TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.tinySize,
            ),
          ),
      ],
    );
  }
}

class _TimelinePointerSurface extends StatefulWidget {
  const _TimelinePointerSurface({
    super.key,
    required this.child,
    required this.behavior,
    this.onTap,
    this.onTapPosition,
    this.onPanStart,
    this.onPanUpdate,
    this.onPanEnd,
    this.onPanCancel,
  });

  final Widget child;
  final HitTestBehavior behavior;
  final VoidCallback? onTap;

  /// Blade mode splits where the pointer landed, so the tap has to carry its
  /// position instead of only "something was tapped".
  final ValueChanged<Offset>? onTapPosition;
  final ValueChanged<DragStartDetails>? onPanStart;
  final ValueChanged<DragUpdateDetails>? onPanUpdate;
  final ValueChanged<DragEndDetails>? onPanEnd;
  final VoidCallback? onPanCancel;

  @override
  State<_TimelinePointerSurface> createState() =>
      _TimelinePointerSurfaceState();
}

class _TimelinePointerSurfaceState extends State<_TimelinePointerSurface> {
  int? _pointer;
  Offset? _start;
  bool _dragging = false;

  void _handleDown(PointerDownEvent event) {
    if (_pointer != null) return;
    _pointer = event.pointer;
    _start = event.position;
    _dragging = false;
  }

  void _handleMove(PointerMoveEvent event) {
    if (event.pointer != _pointer || _start == null) return;
    if (!_dragging) {
      if ((event.position - _start!).distance <= kTouchSlop) return;
      _dragging = true;
      widget.onPanStart?.call(DragStartDetails(globalPosition: _start!));
    }
    widget.onPanUpdate?.call(
      DragUpdateDetails(globalPosition: event.position, delta: event.delta),
    );
  }

  void _handleUp(PointerUpEvent event) {
    if (event.pointer != _pointer) return;
    if (_dragging) {
      widget.onPanEnd?.call(DragEndDetails(primaryVelocity: 0));
    } else {
      widget.onTapPosition?.call(event.localPosition);
      widget.onTap?.call();
    }
    _reset();
  }

  void _handleCancel(PointerCancelEvent event) {
    if (event.pointer != _pointer) return;
    if (_dragging) widget.onPanCancel?.call();
    _reset();
  }

  void _reset() {
    _pointer = null;
    _start = null;
    _dragging = false;
  }

  @override
  Widget build(BuildContext context) => Listener(
    behavior: widget.behavior,
    onPointerDown: _handleDown,
    onPointerMove: _handleMove,
    onPointerUp: _handleUp,
    onPointerCancel: _handleCancel,
    child: widget.child,
  );
}

class _TimelineTrackLane extends StatelessWidget {
  const _TimelineTrackLane({
    required this.laneKey,
    required this.track,
    required this.clips,
    required this.mediaItems,
    required this.mediaPreviews,
    required this.width,
    required this.pixelsPerSecond,
    required this.tickSeconds,
    required this.playhead,
    required this.gesture,
    required this.snapGuide,
    required this.selectedClipId,
    required this.bladeMode,
    required this.onBladeSplit,
    required this.onDropMedia,
    required this.onOpenClip,
    required this.onOpenClipMenu,
    required this.onMoveStart,
    required this.onMoveUpdate,
    required this.onMoveEnd,
    required this.onMoveCancel,
    required this.onTrimStart,
    required this.onTrimEnd,
    required this.onTrimUpdate,
    required this.onTrimFinish,
    required this.onTrimCancel,
  });

  final GlobalKey laneKey;
  final ProjectTimelineTrack track;
  final List<ProjectTimelineClip> clips;
  final List<ProjectMediaItem> mediaItems;
  final Map<String, ProjectMediaPreview> mediaPreviews;
  final double width;
  final double pixelsPerSecond;
  final double tickSeconds;
  final ProjectRationalTime? playhead;
  final _TimelinePointerGesture? gesture;
  final _TimelineSnapGuide? snapGuide;
  final String? selectedClipId;
  final bool bladeMode;
  final void Function(ProjectTimelineClip clip, ProjectRationalTime time)?
  onBladeSplit;
  final void Function(ProjectMediaItem, Offset)? onDropMedia;
  final ValueChanged<ProjectTimelineClip> onOpenClip;
  final void Function(ProjectTimelineClip, Offset) onOpenClipMenu;
  final void Function(ProjectTimelineClip, DragStartDetails)? onMoveStart;
  final void Function(ProjectTimelineClip, DragUpdateDetails)? onMoveUpdate;
  final void Function(ProjectTimelineClip, DragEndDetails)? onMoveEnd;
  final ValueChanged<ProjectTimelineClip>? onMoveCancel;
  final void Function(ProjectTimelineClip, DragStartDetails)? onTrimStart;
  final void Function(ProjectTimelineClip, DragStartDetails)? onTrimEnd;
  final void Function(ProjectTimelineClip, DragUpdateDetails)? onTrimUpdate;
  final void Function(ProjectTimelineClip, DragEndDetails)? onTrimFinish;
  final ValueChanged<ProjectTimelineClip>? onTrimCancel;

  @override
  Widget build(BuildContext context) {
    final lane = Container(
      key: laneKey,
      height: _timelineLaneHeight,
      width: width,
      decoration: const BoxDecoration(
        color: OrColors.background,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Listener(
        // Right-click is the desktop NLE way to reach clip operations without
        // hijacking a plain click (which now only selects). The lane resolves
        // the clip under the pointer from its time range.
        behavior: HitTestBehavior.translucent,
        onPointerDown: (event) {
          if (event.buttons & kSecondaryMouseButton == 0) return;
          if (track.state.locked) {
            onOpenClipMenu(clips.first, event.position);
            return;
          }
          final box = laneKey.currentContext?.findRenderObject() as RenderBox?;
          if (box == null) return;
          final local = box.globalToLocal(event.position);
          final seconds = local.dx / pixelsPerSecond;
          for (final clip in clips) {
            if (seconds >= clip.timelineStart.secondsForDisplay &&
                seconds < clip.timelineEnd.secondsForDisplay) {
              onOpenClipMenu(clip, event.position);
              return;
            }
          }
        },
        child: Stack(
          clipBehavior: Clip.hardEdge,
          children: [
            Positioned.fill(
              child: CustomPaint(
                painter: _TimelineGridPainter(
                  pixelsPerSecond: pixelsPerSecond,
                  tickSeconds: tickSeconds,
                ),
              ),
            ),
            if (snapGuide?.trackId == track.trackId)
              Positioned(
                key: ValueKey('timeline-snap-guide-${track.trackId}'),
                left: snapGuide!.time.secondsForDisplay * pixelsPerSecond,
                top: 0,
                bottom: 0,
                width: 2,
                child: Container(color: OrColors.selection),
              ),
            for (final clip in clips)
              if (gesture?.clip.clipId != clip.clipId)
                _positionedClip(context, clip),
            if (gesture != null && gesture!.targetTrackId == track.trackId)
              _positionedGhost(gesture!),
            if (playhead != null &&
                playhead!.secondsForDisplay.isFinite &&
                playhead!.secondsForDisplay >= 0 &&
                playhead!.secondsForDisplay * pixelsPerSecond <= width)
              Positioned(
                key: ValueKey('timeline-playhead-${track.trackId}'),
                left: playhead!.secondsForDisplay * pixelsPerSecond,
                top: 0,
                bottom: 0,
                width: 2,
                child: const IgnorePointer(
                  child: ColoredBox(color: OrColors.selection),
                ),
              ),
          ],
        ),
      ),
    );
    final dropHandler = onDropMedia;
    if (dropHandler == null || track.state.locked) return lane;
    return DragTarget<ProjectMediaItem>(
      key: ValueKey('timeline-drop-target-${track.trackId}'),
      onWillAcceptWithDetails: (details) =>
          _mediaSupportsTrack(details.data, track.kind),
      onAcceptWithDetails: (details) =>
          dropHandler(details.data, details.offset),
      builder: (context, candidates, rejected) => Stack(
        fit: StackFit.passthrough,
        children: [
          lane,
          if (candidates.isNotEmpty)
            Positioned.fill(
              child: IgnorePointer(
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: OrColors.selection.withValues(alpha: 0.08),
                    border: Border.all(color: OrColors.selection),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }

  Widget _positionedClip(BuildContext context, ProjectTimelineClip clip) {
    final start = clip.timelineStart.secondsForDisplay;
    final duration = clip.timelineDuration.secondsForDisplay;
    final rawLeft = start * pixelsPerSecond;
    final rawWidth = duration * pixelsPerSecond;
    if (!rawLeft.isFinite || !rawWidth.isFinite || rawWidth <= 0) {
      return const SizedBox.shrink();
    }
    final left = rawLeft.clamp(0.0, width).toDouble();
    final clipWidth = math.min(math.max(2.0, rawWidth), width - left);
    if (clipWidth <= 0) return const SizedBox.shrink();
    final mediaId = clip.mediaId;
    final media = mediaId == null ? null : _findMedia(mediaItems, mediaId);
    final textLabel = (clip.text ?? '').replaceAll('\n', ' ');
    final title = switch (clip.contentKind) {
      ProjectTimelineClipContentKind.media =>
        media == null
            ? 'Media ${mediaId == null ? '' : mediaId.substring(0, math.min(8, mediaId.length))}'
            : _mediaDisplayName(media.sourceUri),
      ProjectTimelineClipContentKind.text => 'Text: $textLabel',
      ProjectTimelineClipContentKind.caption => 'Caption: $textLabel',
    };
    final durationLabel = clip.timelineDuration.canonical;
    final visualTrack = track.kind != ProjectTimelineTrackKind.audio;
    final trackEnabled = visualTrack ? track.state.visible : !track.state.muted;
    final background = visualTrack
        ? OrColors.surface
        : OrColors.backgroundRaised;
    final bladeSplit = onBladeSplit;
    final body = _TimelinePointerSurface(
      behavior: HitTestBehavior.opaque,
      onTap: bladeMode ? null : () => onOpenClip(clip),
      onTapPosition: bladeMode && bladeSplit != null
          ? (local) {
              final seconds = start + (local.dx + math.min(6.0, clipWidth / 2)) /
                  pixelsPerSecond;
              bladeSplit(clip, _timeFromSeconds(seconds));
            }
          : null,
      onPanStart: onMoveStart == null
          ? null
          : (details) => onMoveStart!(clip, details),
      onPanUpdate: onMoveUpdate == null
          ? null
          : (details) => onMoveUpdate!(clip, details),
      onPanEnd: onMoveEnd == null
          ? null
          : (details) => onMoveEnd!(clip, details),
      onPanCancel: onMoveCancel == null ? null : () => onMoveCancel!(clip),
      child: const SizedBox.expand(),
    );
    // A clip shows what it contains, like every desktop NLE: frames for visual
    // media, a waveform for audio. The artifact is the one the media library
    // already requested, tiled along the clip instead of decoded again.
    final previewKind = media == null
        ? null
        : media.videoDetails != null
        ? ProjectMediaArtifactKind.thumbnail
        : media.audioDetails != null
        ? ProjectMediaArtifactKind.waveform
        : null;
    final previewBytes =
        previewKind != null &&
            mediaPreviews[media!.mediaId]?.kind == previewKind
        ? mediaPreviews[media.mediaId]?.bytes
        : null;
    // A clip that plays at another speed says so on the clip itself; the
    // inspector only answers for the selection.
    final rate = clip.playbackRate;
    final speedLabel = rate.numerator == rate.denominator
        ? null
        : '${_rateLabel(rate)}×';
    final visual = Container(
      padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x2),
      decoration: BoxDecoration(
        color: background,
        image: previewBytes == null
            ? null
            : DecorationImage(
                image: MemoryImage(previewBytes),
                fit: BoxFit.fitHeight,
                repeat: ImageRepeat.repeatX,
                alignment: Alignment.centerLeft,
                opacity: 0.55,
                filterQuality: FilterQuality.low,
              ),
        border: Border.all(
          color: clip.clipId == selectedClipId
              ? OrColors.selection
              : OrColors.borderStrong,
          width: clip.clipId == selectedClipId ? 1.5 : 1,
        ),
        borderRadius: BorderRadius.circular(OrRadii.small),
      ),
      alignment: Alignment.centerLeft,
      child: Container(
        padding: previewBytes == null
            ? EdgeInsets.zero
            : const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
        decoration: previewBytes == null
            ? null
            : BoxDecoration(
                color: OrColors.background.withValues(alpha: 0.62),
                borderRadius: BorderRadius.circular(3),
              ),
        child: Text(
          title,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: const TextStyle(
            color: OrColors.text,
            fontSize: OrType.tinySize,
            fontWeight: FontWeight.w500,
          ),
        ),
      ),
    );
    final bodyInset = math.min(6.0, clipWidth / 2);
    final contentDetails = switch (clip.contentKind) {
      ProjectTimelineClipContentKind.media =>
        'Media ID: ${mediaId ?? ''}\nSource range: ${clip.sourceStart?.canonical ?? ''} + ${clip.timelineDuration.canonical}',
      ProjectTimelineClipContentKind.text => 'Title: ${clip.text ?? ''}',
      ProjectTimelineClipContentKind.caption => 'Caption: ${clip.text ?? ''}',
    };
    return Positioned(
      left: left,
      top: 5,
      width: clipWidth,
      height: _timelineLaneHeight - 10,
      child: Opacity(
        opacity: trackEnabled ? 1 : 0.45,
        child: Semantics(
          button: true,
          onTap: () => onOpenClip(clip),
          selected: clip.clipId == selectedClipId,
          // The exact span, not only a human duration: a screen-reader user needs
          // to know where an edit left the clip, and scripts/or_ui.py can then
          // assert a position instead of only that the app accepted a command.
          label:
              '$title, starts ${clip.timelineStart.canonical}, duration $durationLabel, '
              'spans ${clip.timelineStart.canonical} to ${clip.timelineEnd.canonical}'
              '${clip.sourceStart == null ? '' : ', source ${clip.sourceStart!.canonical}'}',
          hint: 'Drag to move. Right-click or use Clip actions for more.',
          child: Tooltip(
            key: ValueKey('timeline-clip-tooltip-${clip.clipId}'),
            message:
                'Clip ID: ${clip.clipId}\n$contentDetails\nTimeline start: ${clip.timelineStart.canonical}\nTimeline duration: ${clip.timelineDuration.canonical}',
            child: Material(
              color: background,
              borderRadius: BorderRadius.circular(OrRadii.small),
              child: Stack(
                clipBehavior: Clip.none,
                children: [
                  Positioned.fill(child: visual),
                  Positioned(
                    left: bodyInset,
                    right: bodyInset,
                    top: 0,
                    bottom: 0,
                    child: KeyedSubtree(
                      key: ValueKey('timeline-clip-${clip.clipId}'),
                      child: body,
                    ),
                  ),
                  if (speedLabel != null)
                    Positioned(
                      right: 3,
                      top: 3,
                      child: IgnorePointer(
                        child: Container(
                          key: ValueKey('timeline-clip-speed-${clip.clipId}'),
                          padding: const EdgeInsets.symmetric(
                            horizontal: 3,
                            vertical: 1,
                          ),
                          decoration: BoxDecoration(
                            color: OrColors.background.withValues(alpha: 0.74),
                            borderRadius: BorderRadius.circular(3),
                          ),
                          child: Text(
                            speedLabel,
                            style: const TextStyle(
                              color: OrColors.selection,
                              fontSize: OrType.tinySize,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ),
                      ),
                    ),
                  if (clip.contentKind ==
                      ProjectTimelineClipContentKind.media) ...[
                    _trimHandle(
                      context,
                      clip,
                      start: true,
                      label: 'Trim start of $title',
                    ),
                    _trimHandle(
                      context,
                      clip,
                      start: false,
                      label: 'Trim end of $title',
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _trimHandle(
    BuildContext context,
    ProjectTimelineClip clip, {
    required bool start,
    required String label,
  }) {
    final onStart = start ? onTrimStart : onTrimEnd;
    final compact = OrBreakpoints.isCompact(MediaQuery.sizeOf(context).width);
    final hitWidth = compact
        ? OrTouchTargets.comfortable
        : OrTouchTargets.minimum;
    const visibleBar = 3.0;
    // A hit surface centred exactly on an outer edge is unreachable, because
    // hit testing treats a box's trailing edge as exclusive. Keeping the
    // centre 1 px in from the edge keeps the target symmetric and reachable
    // while still covering the edge on both sides.
    const centreInset = 1.0;
    final outer = -(hitWidth / 2) + centreInset;
    return Positioned(
      left: start ? outer : null,
      right: start ? null : outer,
      top: -1,
      bottom: -1,
      width: hitWidth,
      child: Semantics(
        button: true,
        label: label,
        onTap: () => onOpenClip(clip),
        hint: 'Drag to adjust the timeline edge. Activate for exact trim actions.',
        child: _TimelinePointerSurface(
          key: ValueKey(
            'timeline-trim-handle-${start ? 'start' : 'end'}-${clip.clipId}',
          ),
          behavior: HitTestBehavior.translucent,
          onPanStart: onStart == null
              ? null
              : (details) => onStart(clip, details),
          onPanUpdate: onTrimUpdate == null
              ? null
              : (details) => onTrimUpdate!(clip, details),
          onPanEnd: onTrimFinish == null
              ? null
              : (details) => onTrimFinish!(clip, details),
          onPanCancel: onTrimCancel == null ? null : () => onTrimCancel!(clip),
          child: Align(
            alignment: Alignment.center,
            child: Container(
              width: visibleBar,
              decoration: BoxDecoration(
                color: OrColors.selection,
                borderRadius: BorderRadius.circular(2),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _positionedGhost(_TimelinePointerGesture gesture) {
    final left = gesture.visualStartSeconds * pixelsPerSecond;
    final rawWidth = gesture.visualDurationSeconds * pixelsPerSecond;
    if (!left.isFinite || !rawWidth.isFinite) return const SizedBox.shrink();
    final invalid = gesture.invalidLane || rawWidth <= 0;
    return Positioned(
      left: left,
      top: 5,
      width: math.max(2, rawWidth.abs()),
      height: _timelineLaneHeight - 10,
      child: IgnorePointer(
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: (invalid ? OrColors.danger : OrColors.selection).withValues(
              alpha: 0.16,
            ),
            border: Border.all(
              color: invalid ? OrColors.danger : OrColors.selection,
              width: 1.5,
            ),
            borderRadius: BorderRadius.circular(OrRadii.small),
          ),
          child: const SizedBox.expand(),
        ),
      ),
    );
  }
}

ProjectMediaItem? _findMedia(List<ProjectMediaItem> items, String mediaId) {
  for (final item in items) {
    if (item.mediaId == mediaId) return item;
  }
  return null;
}

bool _mediaSupportsTrack(
  ProjectMediaItem media,
  ProjectTimelineTrackKind kind,
) => switch (kind) {
  ProjectTimelineTrackKind.video => media.videoDetails != null,
  ProjectTimelineTrackKind.audio => media.audioDetails != null,
  ProjectTimelineTrackKind.text || ProjectTimelineTrackKind.caption => false,
};

Future<void> _showInsertTimelineDialog({
  required BuildContext context,
  required ProjectReadModel project,
  required ProjectMediaItem media,
  required ProjectTimelineTracks tracks,
  required ProjectRationalTime suggestedStart,
  required Future<void> Function(
    ProjectReadModel,
    ProjectMediaItem,
    String,
    ProjectRationalTime,
    ProjectRationalTime,
    ProjectRationalTime,
    String,
  )
  onInsert,
}) async {
  if (project.projectId != tracks.projectId ||
      project.projectInstanceId != tracks.projectInstanceId ||
      project.revision != tracks.projectRevision) {
    return;
  }
  final compatibleKinds = <ProjectTimelineTrackKind>{
    if (media.videoDetails != null) ProjectTimelineTrackKind.video,
    if (media.audioDetails != null) ProjectTimelineTrackKind.audio,
  };
  final compatible = tracks.items
      .where(
        (track) => compatibleKinds.contains(track.kind) && !track.state.locked,
      )
      .toList(growable: false);
  if (compatible.isEmpty) {
    final hasLockedCompatibleTrack = tracks.items.any(
      (track) => compatibleKinds.contains(track.kind) && track.state.locked,
    );
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Add to Timeline'),
        content: Text(
          hasLockedCompatibleTrack
              ? 'Unlock a compatible Video or Audio track first.'
              : 'Add a compatible Video or Audio track first.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Close'),
          ),
        ],
      ),
    );
    return;
  }
  final labels = _timelineTrackLabels(tracks.items);
  var selectedTrack = compatible.first;
  final timelineStartController = TextEditingController(
    text: suggestedStart.canonical,
  );
  final sourceStartController = TextEditingController(text: '0/1');
  final durationController = TextEditingController(
    text: _defaultTimelineDuration(media, selectedTrack.kind)?.canonical ?? '',
  );
  bool fieldsAreValid() {
    final start = ProjectRationalTime.tryParse(timelineStartController.text);
    final source = ProjectRationalTime.tryParse(sourceStartController.text);
    final duration = ProjectRationalTime.tryParse(durationController.text);
    return start != null &&
        start.numerator >= BigInt.zero &&
        source != null &&
        source.numerator >= BigInt.zero &&
        duration != null &&
        duration.isPositive;
  }

  final canInsert = ValueNotifier(fieldsAreValid());
  void updateInsertValidity() => canInsert.value = fieldsAreValid();
  // The mode is presentation state: it travels with the command and is never
  // written into the project.
  var editMode = 'insert';

  timelineStartController.addListener(updateInsertValidity);
  sourceStartController.addListener(updateInsertValidity);
  durationController.addListener(updateInsertValidity);
  try {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (dialogContext, setDialogState) {
          return AlertDialog(
            title: const Text('Insert Clip'),
            content: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  DropdownButtonFormField<String>(
                    isExpanded: true,
                    key: const ValueKey('timeline-insert-track'),
                    initialValue: selectedTrack.trackId,
                    decoration: const InputDecoration(
                      labelText: 'Target Track',
                    ),
                    items: [
                      for (final track in compatible)
                        DropdownMenuItem(
                          value: track.trackId,
                          child: Text(
                            '${labels[track.trackId]} · ${track.kind.name}',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                    ],
                    onChanged: (id) {
                      final next = compatible.firstWhere(
                        (track) => track.trackId == id,
                      );
                      setDialogState(() {
                        selectedTrack = next;
                        durationController.text =
                            _defaultTimelineDuration(
                              media,
                              next.kind,
                            )?.canonical ??
                            '';
                      });
                    },
                  ),
                  DropdownButtonFormField<String>(
                    key: const ValueKey('timeline-insert-mode'),
                    isExpanded: true,
                    initialValue: editMode,
                    decoration: const InputDecoration(
                      labelText: 'Edit Mode',
                      helperText: 'Overwrite replaces the range it lands on; insert refuses to overlap.',
                    ),
                    items: const [
                      DropdownMenuItem(value: 'insert', child: Text('Insert')),
                      DropdownMenuItem(
                        value: 'overwrite',
                        child: Text('Overwrite'),
                      ),
                    ],
                    onChanged: (value) =>
                        setDialogState(() => editMode = value ?? 'insert'),
                  ),
                  _ExactRationalField(
                    fieldKey: const ValueKey('timeline-insert-start'),
                    label: 'Timeline Start',
                    controller: timelineStartController,
                    onChanged: () => setDialogState(() {}),
                  ),
                  _ExactRationalField(
                    fieldKey: const ValueKey('timeline-insert-source-start'),
                    label: 'Source Start',
                    controller: sourceStartController,
                    onChanged: () => setDialogState(() {}),
                  ),
                  _ExactRationalField(
                    fieldKey: const ValueKey('timeline-insert-duration'),
                    label: 'Duration',
                    controller: durationController,
                    onChanged: () => setDialogState(() {}),
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
                valueListenable: canInsert,
                builder: (context, valid, child) {
                  return FilledButton(
                    key: const ValueKey('timeline-confirm-insert'),
                    onPressed: valid
                        ? () {
                            final submittedStart = ProjectRationalTime.tryParse(
                              timelineStartController.text,
                            );
                            final submittedSource =
                                ProjectRationalTime.tryParse(
                                  sourceStartController.text,
                                );
                            final submittedDuration =
                                ProjectRationalTime.tryParse(
                                  durationController.text,
                                );
                            if (submittedStart == null ||
                                submittedStart.numerator < BigInt.zero ||
                                submittedSource == null ||
                                submittedSource.numerator < BigInt.zero ||
                                submittedDuration == null ||
                                !submittedDuration.isPositive) {
                              return;
                            }
                            Navigator.of(dialogContext).pop();
                            unawaited(
                              onInsert(
                                project,
                                media,
                                selectedTrack.trackId,
                                submittedStart,
                                submittedSource,
                                submittedDuration,
                                editMode,
                              ),
                            );
                          }
                        : null,
                    child: const Text('Insert'),
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
    timelineStartController.removeListener(updateInsertValidity);
    sourceStartController.removeListener(updateInsertValidity);
    durationController.removeListener(updateInsertValidity);
    timelineStartController.dispose();
    sourceStartController.dispose();
    durationController.dispose();
    canInsert.dispose();
  }
}

ProjectRationalTime? _defaultTimelineDuration(
  ProjectMediaItem media,
  ProjectTimelineTrackKind kind,
) {
  final isStillImage = media.formatNames.any(
    (format) => const {
      'image2',
      'image2pipe',
      'jpeg_pipe',
      'png_pipe',
      'webp_pipe',
    }.contains(format),
  );
  if (kind == ProjectTimelineTrackKind.video && isStillImage) {
    return ProjectRationalTime(BigInt.from(5), 1);
  }
  return switch (kind) {
    ProjectTimelineTrackKind.video =>
      media.firstVideoDuration ?? media.containerDuration,
    ProjectTimelineTrackKind.audio =>
      media.firstAudioDuration ?? media.containerDuration,
    ProjectTimelineTrackKind.text || ProjectTimelineTrackKind.caption => null,
  };
}

class _ExactRationalField extends StatelessWidget {
  const _ExactRationalField({
    required this.fieldKey,
    required this.label,
    required this.controller,
    required this.onChanged,
  });

  final Key fieldKey;
  final String label;
  final TextEditingController controller;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(top: OrSpacing.x2),
    child: TextField(
      key: fieldKey,
      controller: controller,
      onChanged: (_) => onChanged(),
      keyboardType: TextInputType.text,
      decoration: InputDecoration(
        labelText: label,
        hintText: 'NUM/DEN',
        isDense: true,
      ),
    ),
  );
}

Future<void> _showAddTimelineMarkerDialog({
  required BuildContext context,
  required ProjectReadModel project,
  required Future<void> Function(ProjectReadModel, ProjectRationalTime, String)
  onAdd,
}) async {
  final timeController = TextEditingController(text: '0/1');
  final labelController = TextEditingController();
  bool fieldsAreValid() {
    final time = ProjectRationalTime.tryParse(timeController.text);
    final label = labelController.text.trim();
    return time != null &&
        time.numerator >= BigInt.zero &&
        label.isNotEmpty &&
        label.length <= 256;
  }

  final canAdd = ValueNotifier(fieldsAreValid());
  void updateValidity() => canAdd.value = fieldsAreValid();
  timeController.addListener(updateValidity);
  labelController.addListener(updateValidity);
  try {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Add Marker'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _ExactRationalField(
              fieldKey: const ValueKey('timeline-marker-time'),
              label: 'Timeline Time',
              controller: timeController,
              onChanged: updateValidity,
            ),
            Padding(
              padding: const EdgeInsets.only(top: OrSpacing.x2),
              child: TextField(
                key: const ValueKey('timeline-marker-label'),
                controller: labelController,
                autofocus: true,
                decoration: const InputDecoration(
                  labelText: 'Label',
                  hintText: 'Marker label',
                  isDense: true,
                ),
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          ValueListenableBuilder<bool>(
            valueListenable: canAdd,
            builder: (context, valid, child) => FilledButton(
              key: const ValueKey('timeline-confirm-add-marker'),
              onPressed: valid
                  ? () {
                      final time = ProjectRationalTime.tryParse(
                        timeController.text,
                      );
                      final label = labelController.text.trim();
                      if (time == null ||
                          time.numerator < BigInt.zero ||
                          label.isEmpty ||
                          label.length > 256) {
                        return;
                      }
                      Navigator.of(dialogContext).pop();
                      unawaited(onAdd(project, time, label));
                    }
                  : null,
              child: const Text('Add'),
            ),
          ),
        ],
      ),
    );
  } finally {
    await Future<void>.delayed(const Duration(milliseconds: 250));
    timeController.removeListener(updateValidity);
    labelController.removeListener(updateValidity);
    timeController.dispose();
    labelController.dispose();
    canAdd.dispose();
  }
}

Future<void> _showTimelineMarkerActions({
  required BuildContext context,
  required ProjectReadModel? project,
  required ProjectTimelineMarker marker,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineMarker,
    ProjectRationalTime,
  )?
  onMove,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineMarker,
    String,
  )?
  onRename,
  required Future<void> Function(ProjectReadModel, ProjectTimelineMarker)?
  onDelete,
  required bool busy,
}) async {
  final current = project;
  if (current == null) return;
  await showDialog<void>(
    context: context,
    builder: (dialogContext) => AlertDialog(
      title: Text(marker.label),
      content: Text('Marker time: ${marker.timelineTime.canonical}'),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(dialogContext).pop(),
          child: const Text('Close'),
        ),
        TextButton(
          key: ValueKey('timeline-marker-move-${marker.markerId}'),
          onPressed: busy || onMove == null
              ? null
              : () {
                  Navigator.of(dialogContext).pop();
                  unawaited(
                    _showMoveTimelineMarkerDialog(
                      context: context,
                      project: current,
                      marker: marker,
                      onMove: onMove,
                    ),
                  );
                },
          child: const Text('Move'),
        ),
        TextButton(
          key: ValueKey('timeline-marker-rename-${marker.markerId}'),
          onPressed: busy || onRename == null
              ? null
              : () {
                  Navigator.of(dialogContext).pop();
                  unawaited(
                    _showRenameTimelineMarkerDialog(
                      context: context,
                      project: current,
                      marker: marker,
                      onRename: onRename,
                    ),
                  );
                },
          child: const Text('Rename'),
        ),
        TextButton(
          key: ValueKey('timeline-marker-delete-${marker.markerId}'),
          onPressed: busy || onDelete == null
              ? null
              : () {
                  Navigator.of(dialogContext).pop();
                  unawaited(
                    _confirmDeleteTimelineMarker(
                      context: context,
                      project: current,
                      marker: marker,
                      onDelete: onDelete,
                    ),
                  );
                },
          child: const Text('Delete'),
        ),
      ],
    ),
  );
}

Future<void> _showMoveTimelineMarkerDialog({
  required BuildContext context,
  required ProjectReadModel project,
  required ProjectTimelineMarker marker,
  required Future<void> Function(
    ProjectReadModel,
    ProjectTimelineMarker,
    ProjectRationalTime,
  )
  onMove,
}) async {
  final controller = TextEditingController(text: marker.timelineTime.canonical);
  bool isValid() {
    final time = ProjectRationalTime.tryParse(controller.text);
    return time != null && time.numerator >= BigInt.zero;
  }

  final valid = ValueNotifier(isValid());
  void updateValidity() => valid.value = isValid();
  controller.addListener(updateValidity);
  try {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text('Move ${marker.label}'),
        content: _ExactRationalField(
          fieldKey: const ValueKey('timeline-marker-move-time'),
          label: 'Timeline Time',
          controller: controller,
          onChanged: updateValidity,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          ValueListenableBuilder<bool>(
            valueListenable: valid,
            builder: (context, enabled, child) => FilledButton(
              key: ValueKey('timeline-confirm-move-${marker.markerId}'),
              onPressed: enabled
                  ? () {
                      final time = ProjectRationalTime.tryParse(
                        controller.text,
                      );
                      if (time == null || time.numerator < BigInt.zero) {
                        return;
                      }
                      Navigator.of(dialogContext).pop();
                      unawaited(onMove(project, marker, time));
                    }
                  : null,
              child: const Text('Move'),
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

Future<void> _showRenameTimelineMarkerDialog({
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

Future<void> _confirmDeleteTimelineMarker({
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

Future<void> _showTimelineClipActions({
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
      _compareProjectRational(splitAtPlayhead, clip.timelineStart) > 0 &&
      _compareProjectRational(splitAtPlayhead, clip.timelineEnd) < 0;
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
          'Clip ID: ${clip.clipId}\nMedia ID: ${clip.mediaId ?? ''}\nTimeline start: ${clip.timelineStart.canonical}\nSource range: ${clip.sourceStart?.canonical ?? ''} + ${clip.timelineDuration.canonical}',
        ProjectTimelineClipContentKind.text =>
          'Clip ID: ${clip.clipId}\nTitle: ${clip.text ?? ''}\nTimeline start: ${clip.timelineStart.canonical}\nDuration: ${clip.timelineDuration.canonical}',
        ProjectTimelineClipContentKind.caption =>
          'Clip ID: ${clip.clipId}\nCaption: ${clip.text ?? ''}\nTimeline start: ${clip.timelineStart.canonical}\nDuration: ${clip.timelineDuration.canonical}',
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
                              _showMoveTimelineClipDialog(
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
                                _editTimelineTextClip(
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
                              _showTrimTimelineClipDialog(
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
                              _showSlipTimelineClipDialog(
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
                              _showBoundaryMoveDialog(
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
                              _showBoundaryMoveDialog(
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
                              _showSplitTimelineClipDialog(
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
                              _confirmDeleteTimelineClip(
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
                              _confirmRippleDeleteTimelineClip(
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

class _TimelineTextClipEdit {
  const _TimelineTextClipEdit({required this.content, required this.duration});

  final ProjectTimelineTextContent content;
  final ProjectRationalTime duration;
}

Future<void> _editTimelineTextClip({
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
  final edit = await _showTimelineTextClipDialog(
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

Future<_TimelineTextClipEdit?> _showTimelineTextClipDialog({
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
    return await showDialog<_TimelineTextClipEdit>(
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
                _ExactRationalField(
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
                            font: formatting.font,
                            sizeMilliPoints: (sizePoints * 1000).round(),
                            weight: weight,
                            alignment: alignment,
                            color: colors[colorIndex],
                          ),
                        );
                        Navigator.of(dialogContext).pop(
                          _TimelineTextClipEdit(
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

Future<void> _showTrimTimelineClipDialog({
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
        ? _compareProjectRational(target, clip.timelineEnd) < 0
        : _compareProjectRational(target, clip.timelineStart) > 0;
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
              _ExactRationalField(
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
Future<void> _showBoundaryMoveDialog({
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
              _ExactRationalField(
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

Future<void> _showSlipTimelineClipDialog({
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
              _ExactRationalField(
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

Future<void> _showSplitTimelineClipDialog({
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
        _compareProjectRational(splitAt, clip.timelineStart) > 0 &&
        _compareProjectRational(splitAt, clip.timelineEnd) < 0;
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
            _ExactRationalField(
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

Future<void> _showMoveTimelineClipDialog({
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
  final labels = _timelineTrackLabels(tracks);
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
                _ExactRationalField(
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

Future<void> _confirmDeleteTimelineClip({
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
      content: Text(
        'Delete clip ${clip.clipId}? Later clips and other tracks stay in place. The media remains in the project.',
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

Future<void> _confirmRippleDeleteTimelineClip({
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

class _TimelineRuler extends StatelessWidget {
  const _TimelineRuler({required this.compact});

  final bool compact;

  @override
  Widget build(BuildContext context) {
    final marks = compact
        ? const ['00:00', '00:05', '00:10']
        : const ['00:00', '00:05', '00:10', '00:15', '00:20'];

    return Container(
      height: 26,
      decoration: const BoxDecoration(
        color: OrColors.surface,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Row(
        children: [
          const SizedBox(width: 48),
          for (final mark in marks)
            Expanded(
              child: Container(
                alignment: Alignment.centerLeft,
                padding: const EdgeInsets.only(left: OrSpacing.x2),
                decoration: const BoxDecoration(
                  border: Border(left: BorderSide(color: OrColors.border)),
                ),
                child: Text(
                  mark,
                  style: const TextStyle(
                    color: OrColors.textMuted,
                    fontFamily: 'monospace',
                    fontSize: OrType.tinySize,
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _EmptyTimelineLane extends StatelessWidget {
  const _EmptyTimelineLane();

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: 'Empty timeline lane placeholder',
      child: Row(
        children: [
          const SizedBox(
            width: 48,
            child: Center(
              child: Text('—', style: TextStyle(color: OrColors.textMuted)),
            ),
          ),
          Expanded(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                for (var i = 0; i < 5; i++)
                  Expanded(
                    child: Container(
                      decoration: const BoxDecoration(
                        border: Border(
                          left: BorderSide(color: OrColors.border),
                        ),
                      ),
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

class _MobileToolDock extends StatelessWidget {
  const _MobileToolDock({
    required this.selected,
    required this.hasClipSelection,
    required this.isProjectWorkspace,
    required this.onSelected,
  });

  final String selected;
  final bool hasClipSelection;
  final bool isProjectWorkspace;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    return Container(
      key: const ValueKey('mobile-editor-tool-dock'),
      height: 58,
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(top: BorderSide(color: OrColors.border)),
      ),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            for (final tool in [
              _editorTools.first,
              if (hasClipSelection)
                const _EditorTool('Inspector', Icons.tune_outlined),
              ..._editorTools.skip(1),
            ])
              Tooltip(
                message: tool.label == 'Media' && isProjectWorkspace
                    ? 'Open the project media library'
                    : tool.label == 'Inspector' && hasClipSelection
                    ? 'Edit selected clip settings'
                    : '${tool.label} — unavailable in this Developer Preview',
                // The dock carries the same semantics as the desktop tool
                // rail, so an icon-only control is not silent to a screen
                // reader on one layout and not the other.
                child: Semantics(
                  button: true,
                  selected: selected == tool.label,
                  label: tool.label == 'Media' && isProjectWorkspace
                      ? 'Media library'
                      : tool.label == 'Inspector' && hasClipSelection
                      ? 'Clip settings'
                      : tool.label,
                  child: OrFocusRing(
                    child: InkWell(
                      key: ValueKey(
                        'mobile-editor-tool-${_toolKey(tool.label)}',
                      ),
                      onTap: () => onSelected(tool.label),
                      child: Container(
                        width: 62,
                        height: 56,
                        // Selection is carried by a fill and a border as
                        // well as colour, so it survives a colour-blind or
                        // greyscale reading.
                        decoration: BoxDecoration(
                          color: selected == tool.label
                              ? OrColors.surface
                              : Colors.transparent,
                          border: Border.all(
                            color: selected == tool.label
                                ? OrColors.selection
                                : Colors.transparent,
                            width: 1.5,
                          ),
                          borderRadius: BorderRadius.circular(OrRadii.small),
                        ),
                        child: Column(
                          mainAxisAlignment: MainAxisAlignment.center,
                          children: [
                            Icon(
                              tool.icon,
                              size: 18,
                              color: selected == tool.label
                                  ? OrColors.textSecondary
                                  : OrColors.textMuted,
                            ),
                            const SizedBox(height: 3),
                            Text(
                              _shortToolLabel(tool.label),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: TextStyle(
                                color: selected == tool.label
                                    ? OrColors.textSecondary
                                    : OrColors.textMuted,
                                fontSize: OrType.tinySize,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _UnavailableToolSheet extends StatelessWidget {
  const _UnavailableToolSheet({required this.tool});

  final String tool;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(OrSpacing.x4),
      decoration: BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border.all(color: OrColors.borderStrong),
        borderRadius: const BorderRadius.vertical(
          top: Radius.circular(OrRadii.sheet),
        ),
      ),
      child: SafeArea(
        top: false,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    tool,
                    style: const TextStyle(
                      fontSize: OrType.sectionTitleSize,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                ),
                IconButton(
                  tooltip: 'Close tool panel',
                  onPressed: () => Navigator.of(context).pop(),
                  icon: const Icon(Icons.close_outlined),
                ),
              ],
            ),
            const SizedBox(height: OrSpacing.x2),
            const Text(
              'Unavailable in this Developer Preview',
              style: TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataSize,
              ),
            ),
            const SizedBox(height: OrSpacing.x4),
          ],
        ),
      ),
    );
  }
}

class _PanelHeader extends StatelessWidget {
  const _PanelHeader({required this.title, this.height = 38, this.trailing});

  final String title;
  final double height;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: height,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x3),
        child: Row(
          children: [
            Expanded(
              child: Text(
                title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  fontSize: OrType.metadataCompactSize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            ?trailing,
          ],
        ),
      ),
    );
  }
}

/// The structural separator between the timeline and the rest of the shell.
///
/// Panel resizing is a prototype-reference behaviour, so this renders only a
/// 1 px rule. It deliberately paints no grip: a drag-looking control with no
/// resize handler is a dead affordance.
class _ResizeDivider extends StatelessWidget {
  const _ResizeDivider({required this.horizontal});

  final bool horizontal;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: horizontal ? 1 : null,
      width: horizontal ? null : 1,
      color: OrColors.border,
    );
  }
}

String _shortToolLabel(String label) => switch (label) {
  'Stickers / Shapes' => 'Shapes',
  'Filters / Color' => 'Color',
  _ => label,
};

String _toolKey(String label) =>
    label.toLowerCase().replaceAll(' ', '-').replaceAll('/', '-');
