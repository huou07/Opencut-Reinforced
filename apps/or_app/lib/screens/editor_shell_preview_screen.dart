import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../assistant/assistant_controller.dart';
import '../assistant/assistant_models.dart';
import '../assistant/assistant_panel.dart';
import '../design/or_colors.dart';
import '../design/or_typography.dart';
import '../design/or_spacing.dart';
import '../motion/motion_insert_log.dart';
import 'editor_media_library.dart';
import 'editor_viewer_panel.dart';
import 'editor_inspector_panel.dart';
import 'editor_labels.dart';
import 'editor_panel_header.dart';
import 'editor_rational_time.dart';
import 'editor_tool_actions.dart';
import '../motion/motion_library_controller.dart';
import '../production/editor_settings.dart';
import '../motion/motion_library_models.dart';
import '../motion/motion_library_panel.dart';
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
    this.assistant,
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
    this.motionOutputDirectory,
    this.motionInsertLog,
    this.onInsertMotionClip,
    this.pickMotionImage,
    this.pickMotionScene,
    this.onImportCaptions,
    this.onExportCaptions,
    this.onGenerateCaptions,
    this.onManageSpeechModel,
    this.onRecordVoice,
    this.onAdjustClipVolume,
    this.onGenerateVoiceover,
    this.onGenerateMusic,
    this.onCleanUpAudio,
    this.onRemoveTimelineTrack,
    this.onSetTimelineTrackState,
    this.onMoveTimelineTrack,
    this.onLoadMoreTimelineClips,
    this.onLoadMoreTimelineMarkers,
    this.onRefreshTimeline,
    this.onAddMediaToTimeline,
    this.onEnsureTrack,
    this.onInsertTimelineTextClip,
    this.onUpdateTimelineTextClip,
    this.onMoveTimelineClip,
    this.onMoveTimelineClips,
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
    this.onSetTimelineClipFrozen,
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
  final AssistantController? assistant;
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

  /// A project-adjacent folder where materialised motion clips are written.
  final String? motionOutputDirectory;

  /// The last motion clip the Motion panel put on the timeline.
  final MotionInsertLog? motionInsertLog;
  final Future<bool> Function(String path)? onInsertMotionClip;
  final Future<String?> Function()? pickMotionImage;

  /// Returns one `.ormotion.json` path, or null when the picker is cancelled.
  final Future<String?> Function()? pickMotionScene;
  final VoidCallback? onImportCaptions;
  final VoidCallback? onExportCaptions;
  final Future<void> Function(ProjectMediaItem media)? onGenerateCaptions;
  final VoidCallback? onManageSpeechModel;
  final VoidCallback? onRecordVoice;
  final Future<void> Function(ProjectReadModel, String, String)?
  onAdjustClipVolume;
  final VoidCallback? onGenerateVoiceover;
  final VoidCallback? onGenerateMusic;
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
  final Future<(ProjectReadModel, ProjectTimelineTracks)?> Function(
    ProjectTimelineTrackKind kind,
  )?
  onEnsureTrack;
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

  /// Moves every selected clip on one track by one delta, as one undo step.
  final Future<void> Function(
    ProjectReadModel expected,
    String trackId,
    List<String> clipIds,
    ProjectRationalTime delta,
  )?
  onMoveTimelineClips;
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
  final Future<ProjectReadModel?> Function(
    ProjectReadModel expected,
    String trackId,
    String clipId,
    bool frozen,
  )?
  onSetTimelineClipFrozen;
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
  final GlobalKey<EditorViewerPanelState> _viewerKey = GlobalKey(
    debugLabel: 'editor viewer',
  );
  final GlobalKey<_TimelinePanelState> _timelineKey = GlobalKey(
    debugLabel: 'editor timeline',
  );

  String _selectedTool = 'Media';
  _RightPanelTab _rightPanelTab = _RightPanelTab.inspector;
  final EditorSettingsStore _editorSettings = EditorSettingsStore();
  bool _showInspectorPreference = false;
  bool _inspectorRevealed = false;
  bool _assistantReady = false;
  bool _userChoseInspector = false;
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
  late final MotionLibraryController _motionLibrary;

  @override
  void initState() {
    super.initState();
    _motionLibrary = MotionLibraryController(insertLog: widget.motionInsertLog);
    _previewState = ValueNotifier(null);
    _assistantReady = widget.assistant?.state == AssistantConnectionState.ready;
    widget.assistant?.addListener(_onAssistantChanged);
    _loadEditorSettings();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _syncAssistantContext();
    });
  }

  Future<void> _loadEditorSettings() async {
    final settings = await _editorSettings.load();
    if (!mounted) return;
    setState(() => _showInspectorPreference = settings.showInspector);
  }

  /// The inspector is the pre-connection fallback: while the harness starts,
  /// there is nothing else useful in the rail. Once the assistant is ready the
  /// advanced tab hides unless the user asked for it.
  void _onAssistantChanged() {
    final ready = widget.assistant?.state == AssistantConnectionState.ready;
    if (ready == _assistantReady || !mounted) return;
    setState(() {
      _assistantReady = ready;
      if (ready &&
          !_showInspectorPreference &&
          !_userChoseInspector &&
          !_inspectorRevealed) {
        _rightPanelTab = _RightPanelTab.assistant;
      }
    });
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
    if (projectIdentityChanged || oldWidget.assistant != widget.assistant) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _syncAssistantContext();
      });
    }
  }

  /// Publishes the editor focus (selected clip, playhead) to the assistant so
  /// "this clip" has a concrete meaning without the agent guessing.
  void _syncAssistantContext() {
    final assistant = widget.assistant;
    if (assistant == null) return;
    final position = _previewState.value?.position;
    final seconds = position?.secondsForDisplay;
    ProjectTimelineClip? selected;
    final clipId = _selectedClipId;
    if (clipId != null) {
      for (final page in widget.timelineClipPages.values) {
        for (final clip in page.items) {
          if (clip.clipId == clipId) selected = clip;
        }
      }
    }
    assistant.setContext(
      AssistantEditorContext(
        clipId: clipId,
        clipLabel: selected == null
            ? null
            : '${selected.contentKind.name} clip',
        trackKind: _selectedTrackKind?.name,
        playheadSeconds: seconds,
      ),
    );
  }

  /// Switches the right rail between clip settings and the assistant.
  void showAssistantPanel() {
    if (widget.assistant == null) return;
    _userChoseInspector = false;
    if (_rightPanelTab == _RightPanelTab.assistant) return;
    setState(() => _rightPanelTab = _RightPanelTab.assistant);
    _syncAssistantContext();
  }

  void showInspectorPanel() {
    if (_rightPanelTab == _RightPanelTab.inspector) return;
    // The palette can always reach the advanced surface; opening it once reveals
    // its tab for this session without changing the stored preference.
    setState(() {
      _inspectorRevealed = true;
      _userChoseInspector = true;
      _rightPanelTab = _RightPanelTab.inspector;
    });
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
    widget.assistant?.removeListener(_onAssistantChanged);
    _mediaSearchController.dispose();
    _previewState.dispose();
    _motionLibrary.dispose();
    _mobileSheetRevision.dispose();
    super.dispose();
  }

  /// Materialises a dragged library asset and inserts it on the timeline.
  Future<void> _insertMotionAsset(MotionAssetDragData data) async {
    final directory = widget.motionOutputDirectory;
    if (directory == null || directory.isEmpty) {
      OrSnackBar.show(context, 'Open a project before adding a motion clip.');
      return;
    }
    final safe = data.name.replaceAll(RegExp(r'[^A-Za-z0-9_-]'), '-');
    final output =
        '$directory${Platform.pathSeparator}motion-$safe-'
        '${DateTime.now().millisecondsSinceEpoch}.mkv';
    final view = await _motionLibrary.materializeDrag(data, output);
    if (!mounted) return;
    if (view == null || !view.succeeded) {
      final message = view?.message ?? '';
      OrSnackBar.show(
        context,
        message.isNotEmpty
            ? message
            : (_motionLibrary.error ??
                  'The motion clip could not be rendered.'),
      );
      return;
    }
    final added =
        await widget.onInsertMotionClip?.call(view.outputPath) ?? false;
    if (!mounted) return;
    if (added) {
      _motionLibrary.recordInsert(view, MotionInsertSource.asset);
    }
    OrSnackBar.show(
      context,
      added
          ? 'Motion clip added to the timeline.'
          : 'The rendered clip could not be inserted.',
    );
  }

  void _onPreviewStateChanged(ProjectPreviewState state) {
    final previous = _previewState.value;
    _previewState.value = state;
    _syncAssistantContext();
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
    _syncAssistantContext();
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
  Future<void> _commitSelectedTransform(ProjectTimelineVisualSettings next) =>
      _queueSelectedTransform(next);

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

  Future<void> _sendSelectedTransform(
    ProjectTimelineVisualSettings next,
  ) async {
    final project = widget.project;
    final trackId = _selectedTrackId;
    final clipId = _selectedClipId;
    final update = widget.onUpdateTimelineClipVisualSettings;
    if (project == null ||
        trackId == null ||
        clipId == null ||
        update == null) {
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

  /// The timeline hands every scrub frame to the viewer transport.
  void scrubPreviewTo(ProjectRationalTime position) =>
      _viewerKey.currentState?.scrubTo(position);

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

  /// Holds the selected clip's first frame, or releases it.
  ///
  /// A freeze is a clip property like reverse, so it keeps the clip's place on the
  /// timeline and its trim; unfreezing restores the footage untouched.
  void toggleSelectedClipFrozen() {
    final clipId = _selectedClipId;
    final onSet = widget.onSetTimelineClipFrozen;
    final project = widget.project;
    if (clipId == null || onSet == null || project == null) return;
    for (final entry in widget.timelineClipPages.entries) {
      for (final clip in entry.value.items) {
        if (clip.clipId != clipId) continue;
        if (clip.contentKind != ProjectTimelineClipContentKind.media) return;
        unawaited(onSet(project, entry.key, clip.clipId, !clip.frozen));
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
              // (docs/archive/legacy/EDITOR_WORKSPACE_DESIGN.md §3).
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
                  onEnsureTrack: widget.isProjectWorkspace && !widget.busy
                      ? widget.onEnsureTrack
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
                  onManageSpeechModel: widget.busy
                      ? null
                      : widget.onManageSpeechModel,
                  onRecordVoice: widget.busy ? null : widget.onRecordVoice,
                  onGenerateVoiceover: widget.busy
                      ? null
                      : widget.onGenerateVoiceover,
                  onGenerateMusic: widget.busy ? null : widget.onGenerateMusic,
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
                  motionLibrary: _motionLibrary,
                  motionOutputDirectory: widget.motionOutputDirectory,
                  motionInsertLog: widget.motionInsertLog,
                  onInsertMotionClip: widget.onInsertMotionClip,
                  pickMotionImage: widget.pickMotionImage,
                  pickMotionScene: widget.pickMotionScene,
                  onNotice: (message) => OrSnackBar.show(context, message),
                ),
              ),
              const VerticalDivider(width: 1),
              Expanded(
                child: EditorViewerPanel(
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
                width: wide ? 320 : 280,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    if (widget.assistant != null) ...[
                      _RightPanelTabs(
                        selected: _rightPanelTab,
                        showInspector:
                            _showInspectorPreference ||
                            _inspectorRevealed ||
                            !_assistantReady,
                        assistantBusy: widget.assistant!.busy,
                        onSelected: (tab) {
                          setState(() => _rightPanelTab = tab);
                          _syncAssistantContext();
                        },
                      ),
                      const Divider(height: 1),
                    ],
                    Expanded(
                      child:
                          _rightPanelTab == _RightPanelTab.assistant &&
                              widget.assistant != null
                          ? AssistantPanel(
                              controller: widget.assistant!,
                              compact: !wide,
                            )
                          : EditorInspectorPanel(
                              showHeader: widget.assistant == null,
                              isProjectWorkspace: widget.isProjectWorkspace,
                              project: widget.project,
                              gateway: widget.projectGateway,
                              session: widget.projectSession,
                              trackId: _selectedTrackId,
                              clipId: _selectedClipId,
                              clip: _selectedInspectorClip,
                              isVisualTrack:
                                  _selectedTrackKind ==
                                      ProjectTimelineTrackKind.video ||
                                  _selectedTrackKind ==
                                      ProjectTimelineTrackKind.text ||
                                  _selectedTrackKind ==
                                      ProjectTimelineTrackKind.caption,
                              isAudioTrack:
                                  _selectedTrackKind ==
                                  ProjectTimelineTrackKind.audio,
                              trackLocked:
                                  _selectedInspectorTrack?.state.locked ?? true,
                              busy: widget.busy,
                              sequenceFrameRate:
                                  _previewState.value?.sequenceFrameRate,
                              contentEnd: _previewState.value?.contentEnd,
                              onSetSequenceFrameRate:
                                  widget.onSetSequenceFrameRate,
                              trackCount:
                                  widget.timelineTracks?.items.length ?? 0,
                              clipCount: widget.timelineClipPages.values
                                  .fold<int>(
                                    0,
                                    (total, page) => total + page.totalCount,
                                  ),
                              onUpdate:
                                  widget.onUpdateTimelineClipVisualSettings,
                              onUpdateAudio:
                                  widget.onUpdateTimelineClipAudioSettings,
                              onUpdatePlaybackRate:
                                  widget.onUpdateTimelineClipPlaybackRate,
                              onSetReversed: widget.onSetTimelineClipReversed,
                              onSetFrozen: widget.onSetTimelineClipFrozen,
                            ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
        const _ResizeDivider(horizontal: true),
        Expanded(
          flex: 2,
          child: _TimelinePanel(
            onMotionAssetDropped: _insertMotionAsset,
            key: _timelineKey,
            compact: false,
            previewState: _previewState,
            onScrubTo: widget.projectSession == null ? null : scrubPreviewTo,
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
            onMoveClips: widget.onMoveTimelineClips,
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
            onSetClipFrozen: widget.onSetTimelineClipFrozen,
            onUpdatePlaybackRate: widget.onUpdateTimelineClipPlaybackRate,
            onAdjustClipVolume: widget.onAdjustClipVolume,
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
          child: EditorViewerPanel(
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
            onMotionAssetDropped: _insertMotionAsset,
            key: _timelineKey,
            compact: true,
            previewState: _previewState,
            onScrubTo: widget.projectSession == null ? null : scrubPreviewTo,
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
            onMoveClips: widget.onMoveTimelineClips,
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
            onSetClipFrozen: widget.onSetTimelineClipFrozen,
            onUpdatePlaybackRate: widget.onUpdateTimelineClipPlaybackRate,
            onAdjustClipVolume: widget.onAdjustClipVolume,
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
              child: EditorInspectorPanel(
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
          if (tool == 'Assistant' && widget.assistant != null) {
            return SizedBox(
              height: height,
              child: AssistantPanel(
                controller: widget.assistant!,
                compact: true,
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
                motionLibrary: _motionLibrary,
                motionOutputDirectory: widget.motionOutputDirectory,
                motionInsertLog: widget.motionInsertLog,
                onInsertMotionClip: widget.onInsertMotionClip,
                pickMotionImage: widget.pickMotionImage,
                pickMotionScene: widget.pickMotionScene,
                onNotice: (message) => OrSnackBar.show(context, message),
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
                onManageSpeechModel: widget.busy
                    ? null
                    : widget.onManageSpeechModel,
                onRecordVoice: widget.busy ? null : widget.onRecordVoice,
                onGenerateVoiceover: widget.busy
                    ? null
                    : widget.onGenerateVoiceover,
                onGenerateMusic: widget.busy ? null : widget.onGenerateMusic,
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
      formatting: contentKind == ProjectTimelineClipContentKind.caption
          ? ProjectTextFormatting.captionDefaults
          : ProjectTextFormatting.defaults,
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
/// column and cost 68 px of viewer width (docs/archive/legacy/EDITOR_WORKSPACE_DESIGN.md §3).
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
    this.onManageSpeechModel,
    this.onRecordVoice,
    this.onGenerateVoiceover,
    this.onGenerateMusic,
    required this.onCleanUpAudio,
    required this.onAddMediaToTimeline,
    this.onEnsureTrack,
    this.onAddTextTrack,
    this.onAddCaptionTrack,
    this.onAddAudioTrack,
    this.onImportCaptions,
    this.onExportCaptions,
    this.onOpenMotion,
    required this.motionLibrary,
    this.motionOutputDirectory,
    this.motionInsertLog,
    this.onInsertMotionClip,
    this.pickMotionImage,
    this.pickMotionScene,
    required this.onNotice,
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
  final VoidCallback? onManageSpeechModel;
  final VoidCallback? onRecordVoice;
  final VoidCallback? onGenerateVoiceover;
  final VoidCallback? onGenerateMusic;
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
  final Future<(ProjectReadModel, ProjectTimelineTracks)?> Function(
    ProjectTimelineTrackKind kind,
  )?
  onEnsureTrack;
  final VoidCallback? onAddTextTrack;
  final VoidCallback? onAddCaptionTrack;
  final VoidCallback? onAddAudioTrack;
  final VoidCallback? onImportCaptions;
  final VoidCallback? onExportCaptions;
  final VoidCallback? onOpenMotion;
  final MotionLibraryController motionLibrary;
  final String? motionOutputDirectory;

  /// The last motion clip the Motion panel put on the timeline.
  final MotionInsertLog? motionInsertLog;
  final Future<bool> Function(String path)? onInsertMotionClip;
  final Future<String?> Function()? pickMotionImage;
  final Future<String?> Function()? pickMotionScene;
  final ValueChanged<String> onNotice;
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
      final page = mediaPage;
      final currentProject = project;
      final currentPreviewState = previewState;
      final currentTracks = timelineTracks;
      final insert = onAddMediaToTimeline;
      // The library only offers an insert while its own page, the project and
      // the timeline snapshot it was built from still describe the same state.
      final snapshotMatches =
          page != null &&
          currentProject != null &&
          currentTracks != null &&
          page.projectId == currentProject.projectId &&
          page.projectInstanceId == currentProject.projectInstanceId &&
          page.projectRevision == currentProject.revision &&
          currentTracks.projectId == currentProject.projectId &&
          currentTracks.projectInstanceId == currentProject.projectInstanceId &&
          currentTracks.projectRevision == currentProject.revision;
      return EditorMediaLibraryPanel(
        showTitle: false,
        searchController: mediaSearchController,
        page: page,
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
        onManageSpeechModel: onManageSpeechModel,
        onCleanUpAudio: onCleanUpAudio,
        onInsertMedia:
            insert != null && snapshotMatches && currentPreviewState != null
            ? (media) => _showInsertTimelineDialog(
                context: context,
                project: currentProject,
                media: media,
                tracks: currentTracks,
                suggestedStart:
                    currentPreviewState.contentEnd ??
                    ProjectRationalTime(BigInt.zero, 1),
                onInsert: insert,
                onEnsureTrack: onEnsureTrack,
              )
            : null,
        onClose: onClose,
      );
    }

    if (selectedTool == 'Text') {
      return _ToolActionsPanel(
        title: 'Text',
        showTitle: false,
        subtitle: textToolSubtitle,
        actions: textToolActions(
          onAddTextTrack: onAddTextTrack,
          onAddCaptionTrack: onAddCaptionTrack,
          onImportCaptions: onImportCaptions,
          onExportCaptions: onExportCaptions,
        ),
      );
    }
    if (selectedTool == 'Audio') {
      return _ToolActionsPanel(
        title: 'Audio',
        showTitle: false,
        subtitle: audioToolSubtitle,
        actions: audioToolActions(
          onAddAudioTrack: onAddAudioTrack,
          onImportMedia: onImportMedia,
          onRecordVoice: onRecordVoice,
          onGenerateVoiceover: onGenerateVoiceover,
          onGenerateMusic: onGenerateMusic,
          onManageSpeechModel: onManageSpeechModel,
        ),
      );
    }
    if (selectedTool == 'Motion') {
      return MotionLibraryPanel(
        controller: motionLibrary,
        outputDirectory: motionOutputDirectory,
        onInsertClip: onInsertMotionClip ?? (_) async => false,
        onNotice: onNotice,
        onOpenCodeMotion: onOpenMotion ?? () {},
        pickImage: pickMotionImage,
        pickScene: pickMotionScene,
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
          EditorPanelHeader(title: selectedTool),
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
  final List<EditorToolAction> actions;

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
            EditorPanelHeader(title: title),
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
    this.onMotionAssetDropped,
    required this.previewState,
    this.onScrubTo,
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
    this.onMoveClips,
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
    this.onSetClipFrozen,
    this.onUpdatePlaybackRate,
    this.onAdjustClipVolume,
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

  /// Accepts a dragged motion asset anywhere on the timeline and appends the
  /// materialised clip. Placement is deliberately the track end, not the drop
  /// pixel: a drop cannot silently rewrite existing clip boundaries.
  final Future<void> Function(MotionAssetDragData)? onMotionAssetDropped;

  final ValueListenable<ProjectPreviewState?> previewState;

  /// Moves the playhead from a ruler scrub gesture.
  ///
  /// Null when no project session owns a preview, which is how the ruler knows
  /// to leave the pointer alone instead of showing a drag affordance that
  /// cannot move anything. A scrub is presentation only: it never opens a
  /// project command, never changes the revision and never enters undo.
  final void Function(ProjectRationalTime position)? onScrubTo;
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

  /// Moves the whole selection on [String] track by one delta in one command.
  final Future<void> Function(
    ProjectReadModel,
    String,
    List<String>,
    ProjectRationalTime,
  )?
  onMoveClips;
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
    ProjectReadModel project,
    String trackId,
    String clipId,
    bool frozen,
  )?
  onSetClipFrozen;
  final Future<ProjectReadModel?> Function(
    ProjectReadModel project,
    String trackId,
    String clipId,
    ProjectRationalRate rate,
  )?
  onUpdatePlaybackRate;
  final Future<void> Function(ProjectReadModel, String, String)?
  onAdjustClipVolume;
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

  /// A rubber-band selection in progress: which track it started on and the
  /// lane-local rectangle it currently covers.
  String? _marqueeTrackId;
  Offset? _marqueeAnchor;
  Offset? _marqueeCurrent;
  String? _selectedClipId;

  /// The full selection. `_selectedClipId` stays the primary clip (the one the
  /// inspector edits); this set is what bulk verbs act on.
  final Set<String> _selectedClipIds = <String>{};
  double _timelineZoom = 1;
  double _pixelsPerSecond = 64;
  bool _fitTimelineToView = false;
  int _gestureToken = 0;

  /// The time the pointer is scrubbing to, while it is dragging.
  ///
  /// The preview round-tripped every earlier target, so the playhead follows
  /// the pointer directly and only falls back to the committed preview position
  /// once that position has caught up (see [_onPreviewStateChanged]).
  ProjectRationalTime? _scrubPosition;
  bool _scrubbing = false;

  @override
  void initState() {
    super.initState();
    widget.previewState.addListener(_onPreviewStateChanged);
  }

  void _onPreviewStateChanged() {
    final scrub = _scrubPosition;
    if (scrub == null || _scrubbing) return;
    final state = widget.previewState.value;
    final position = state?.position;
    // Playback moving on is a newer intent than the finished scrub, so the
    // preview wins just as soon as it is at the scrubbed time.
    if (state?.playing == true ||
        (position != null && _compareProjectRational(position, scrub) == 0)) {
      if (mounted) setState(() => _scrubPosition = null);
    }
  }

  @override
  void didUpdateWidget(covariant _TimelinePanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.previewState != widget.previewState) {
      oldWidget.previewState.removeListener(_onPreviewStateChanged);
      widget.previewState.addListener(_onPreviewStateChanged);
    }
    final previousProject = oldWidget.project;
    final nextProject = widget.project;
    if (previousProject?.projectId != nextProject?.projectId ||
        previousProject?.projectInstanceId != nextProject?.projectInstanceId ||
        previousProject?.revision != nextProject?.revision) {
      _cancelGesture();
      _scrubPosition = null;
      _scrubbing = false;
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
    widget.previewState.removeListener(_onPreviewStateChanged);
    _gestureToken++;
    _timelineFocusNode.dispose();
    _horizontalController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final timeline = ValueListenableBuilder<ProjectPreviewState?>(
      valueListenable: widget.previewState,
      builder: (context, preview, _) => _buildTimeline(context, preview),
    );
    final drop = widget.onMotionAssetDropped;
    if (drop == null) return timeline;
    return DragTarget<MotionAssetDragData>(
      key: const ValueKey('timeline-motion-drop'),
      onAcceptWithDetails: (details) => unawaited(drop(details.data)),
      builder: (context, candidates, rejected) => Stack(
        children: [
          timeline,
          if (candidates.isNotEmpty)
            Positioned.fill(
              child: IgnorePointer(
                child: Container(
                  color: OrColors.focus.withValues(alpha: 0.08),
                  alignment: Alignment.center,
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: OrSpacing.x3,
                      vertical: OrSpacing.x2,
                    ),
                    decoration: BoxDecoration(
                      color: OrColors.surfaceSelected,
                      borderRadius: BorderRadius.circular(OrRadii.control),
                      border: Border.all(color: OrColors.focus),
                    ),
                    child: const Text(
                      'Drop to add the motion clip',
                      style: TextStyle(
                        color: OrColors.text,
                        fontSize: OrType.tinySize,
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
                  const EditorPanelHeader(
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

  /// Moves the playhead to [position] from a ruler or lane gesture.
  ///
  /// The pointer emits far more targets than a frame can be rendered for, so a
  /// move smaller than half a pixel changes nothing the user can see and is
  /// dropped before it reaches the preview.
  void _scrubAt(ProjectRationalTime position) {
    final onScrubTo = widget.onScrubTo;
    if (onScrubTo == null) return;
    final end = widget.previewState.value?.contentEnd;
    var target = position;
    if (target.numerator < BigInt.zero) {
      target = ProjectRationalTime(BigInt.zero, 1);
    } else if (end != null && _compareProjectRational(target, end) > 0) {
      // Seeking past the last frame leaves the preview with nothing to show,
      // so every scrub surface stops at the sequence end.
      target = end;
    }
    final previous = _scrubPosition;
    if (_scrubbing &&
        previous != null &&
        (target.secondsForDisplay - previous.secondsForDisplay).abs() <
            0.5 / math.max(1.0, _pixelsPerSecond)) {
      return;
    }
    setState(() {
      _scrubbing = true;
      _scrubPosition = target;
    });
    onScrubTo(target);
  }

  void _endScrub() {
    if (!_scrubbing) return;
    setState(() => _scrubbing = false);
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
          // A scrub owns the playhead until the preview reaches that time; the
          // committed position is the fallback for every other frame.
          final playhead = _scrubPosition ?? preview?.position;
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
                                        playhead: playhead,
                                        contentEnd: preview?.contentEnd,
                                        pixelsPerSecond: scale,
                                        tickSeconds: tickSeconds,
                                        width: canvasWidth,
                                        onScrubTo: widget.onScrubTo == null
                                            ? null
                                            : _scrubAt,
                                        onScrubEnd: _endScrub,
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
                                        playhead: playhead,
                                        gesture: _gesture,
                                        snapGuide: _snapGuide,
                                        selectedClipId: _selectedClipId,
                                        selectedClipIds: _selectedClipIds,
                                        bladeMode: widget.bladeMode,
                                        marqueeRect:
                                            _marqueeTrackId == track.trackId &&
                                                _marqueeAnchor != null &&
                                                _marqueeCurrent != null
                                            ? Rect.fromPoints(
                                                _marqueeAnchor!,
                                                _marqueeCurrent!,
                                              )
                                            : null,
                                        onMarqueeStart: (local) =>
                                            _beginMarquee(track.trackId, local),
                                        onMarqueeUpdate: (local) =>
                                            _updateMarquee(
                                              track.trackId,
                                              local,
                                            ),
                                        onMarqueeEnd: () => _endMarquee(track),
                                        // Pressing empty lane space moves the
                                        // playhead, and the playhead line is
                                        // itself a drag surface in every lane.
                                        onScrubTo: widget.onScrubTo == null
                                            ? null
                                            : _scrubAt,
                                        onScrubEnd: _endScrub,
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
              if (_selectedClipIds.length > 1)
                Padding(
                  padding: const EdgeInsets.only(left: OrSpacing.x1),
                  child: Text(
                    '${_selectedClipIds.length} clips',
                    key: const ValueKey('timeline-selection-count'),
                    style: const TextStyle(
                      fontSize: OrType.tinySize,
                      fontWeight: FontWeight.w600,
                      color: OrColors.selection,
                    ),
                  ),
                ),
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
                message: 'Split at playhead (${shortcutLabel('B')})',
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
                tooltip: 'Duplicate clip (${shortcutLabel('D')})',
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
              if (selectedTrack != null &&
                  selectedTrack.kind == ProjectTimelineTrackKind.audio &&
                  selectedClip.contentKind ==
                      ProjectTimelineClipContentKind.media &&
                  widget.project != null &&
                  widget.onAdjustClipVolume != null)
                TextButton.icon(
                  key: const ValueKey('timeline-selection-volume'),
                  onPressed: widget.busy
                      ? null
                      : () => unawaited(
                          widget.onAdjustClipVolume!(
                            widget.project!,
                            selectedTrack.trackId,
                            selectedClip.clipId,
                          ),
                        ),
                  icon: const Icon(Icons.volume_up_outlined, size: 15),
                  label: const Text('Volume'),
                ),
              if (selectedClip.contentKind ==
                      ProjectTimelineClipContentKind.media &&
                  selectedTrack != null &&
                  widget.project != null &&
                  widget.onUpdatePlaybackRate != null)
                TextButton.icon(
                  key: const ValueKey('timeline-selection-speed'),
                  onPressed: widget.busy
                      ? null
                      : () async {
                          final rate = await _showPlaybackSpeedDialog(
                            context,
                            selectedClip.playbackRate,
                          );
                          if (rate == null || !context.mounted) return;
                          await widget.onUpdatePlaybackRate!(
                            widget.project!,
                            selectedTrack.trackId,
                            selectedClip.clipId,
                            rate,
                          );
                        },
                  icon: const Icon(Icons.speed, size: 15),
                  label: Text(_playbackSpeedLabel(selectedClip.playbackRate)),
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

  ProjectTimelineClip? _clipById(String clipId) {
    for (final page in widget.clipPages.values) {
      for (final clip in page.items) {
        if (clip.clipId == clipId) return clip;
      }
    }
    return null;
  }

  ProjectTimelineTrack? _trackForClipId(String clipId) {
    for (final track
        in widget.tracks?.items ?? const <ProjectTimelineTrack>[]) {
      final page = widget.clipPages[track.trackId];
      if (page == null) continue;
      if (page.items.any((clip) => clip.clipId == clipId)) return track;
    }
    return null;
  }

  /// Deletes every selected clip, in order, without a dialog per clip: the owner
  /// already said what they meant by selecting the set.
  Future<void> _deleteSelectedClips() async {
    final project = widget.project;
    final onDelete = widget.onDeleteClip;
    if (project == null || onDelete == null) return;
    for (final id in _selectedClipIds.toList(growable: false)) {
      final clip = _clipById(id);
      final track = _trackForClipId(id);
      if (clip == null || track == null || track.state.locked) continue;
      await onDelete(project, clip);
    }
    if (!mounted) return;
    setState(() {
      _selectedClipIds.clear();
      _selectedClipId = null;
    });
    widget.onClipSelectionChanged?.call(null, null, null);
  }

  void confirmDeleteSelectedClip() {
    if (_selectedClipIds.length > 1) {
      unawaited(_deleteSelectedClips());
      return;
    }
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
          child: _ClipMenuItem('Split at playhead', shortcutLabel('B')),
        ),
        PopupMenuItem<_ClipMenuChoice>(
          value: _ClipMenuChoice.duplicate,
          enabled: widget.onDuplicateClip != null && !widget.busy,
          child: _ClipMenuItem('Duplicate', shortcutLabel('D')),
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
          value: _ClipMenuChoice.freeze,
          enabled:
              widget.onSetClipFrozen != null &&
              !widget.busy &&
              clip.contentKind == ProjectTimelineClipContentKind.media,
          child: _ClipMenuItem(
            clip.frozen ? 'Release this frame' : 'Hold this frame',
            '',
          ),
        ),
        PopupMenuItem<_ClipMenuChoice>(
          value: _ClipMenuChoice.delete,
          enabled: widget.onDeleteClip != null && !widget.busy,
          child: _ClipMenuItem(
            'Delete',
            deleteShortcutLabel(),
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
      case _ClipMenuChoice.freeze:
        await widget.onSetClipFrozen!(
          project,
          track.trackId,
          clip.clipId,
          !clip.frozen,
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
            name = mediaDisplayName(item.sourceUri);
            break;
          }
        }
      }
    }
    final start = clockLabel(rationalSeconds(clip.timelineStart));
    final end = clockLabel(rationalSeconds(clip.timelineEnd));
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
    final extend = HardwareKeyboard.instance.isShiftPressed;
    setState(() {
      if (extend) {
        if (!_selectedClipIds.remove(clip.clipId)) {
          _selectedClipIds.add(clip.clipId);
        }
        _selectedClipId = _selectedClipIds.contains(clip.clipId)
            ? clip.clipId
            : (_selectedClipIds.isEmpty ? null : _selectedClipIds.last);
      } else {
        _selectedClipIds
          ..clear()
          ..add(clip.clipId);
        _selectedClipId = clip.clipId;
      }
    });
    widget.onClipSelectionChanged?.call(
      track.trackId,
      _selectedClipId,
      _selectedClipId == null ? null : track.kind,
    );
  }

  void _beginMarquee(String trackId, Offset local) {
    if (trackId != _marqueeTrackId) {
      setState(() {
        _marqueeTrackId = trackId;
        _marqueeAnchor = local;
        _marqueeCurrent = local;
      });
      return;
    }
    setState(() {
      _marqueeAnchor = local;
      _marqueeCurrent = local;
    });
  }

  void _updateMarquee(String trackId, Offset local) {
    if (_marqueeTrackId != trackId) return;
    setState(() => _marqueeCurrent = local);
  }

  /// Selects every clip on the track whose span the band touched. A band that
  /// never moved is a click on empty space, which clears the selection.
  void _endMarquee(ProjectTimelineTrack track) {
    final local = _marqueeCurrent ?? _marqueeAnchor ?? Offset.zero;
    final anchor = _marqueeAnchor;
    final marqueeTrackId = _marqueeTrackId;
    setState(() {
      _marqueeTrackId = null;
      _marqueeAnchor = null;
      _marqueeCurrent = null;
    });
    if (anchor == null || marqueeTrackId != track.trackId) return;
    final left = math.min(anchor.dx, local.dx);
    final right = math.max(anchor.dx, local.dx);
    if (right - left < 4) {
      _clearSelection();
      return;
    }
    final startSeconds = left / _pixelsPerSecond;
    final endSeconds = right / _pixelsPerSecond;
    final clips = widget.clipPages[track.trackId]?.items ?? const [];
    final hit = [
      for (final clip in clips)
        if (clip.timelineEnd.secondsForDisplay > startSeconds &&
            clip.timelineStart.secondsForDisplay < endSeconds)
          clip.clipId,
    ];
    setState(() {
      _selectedClipIds
        ..clear()
        ..addAll(hit);
      _selectedClipId = hit.isEmpty ? null : hit.last;
    });
    widget.onClipSelectionChanged?.call(
      track.trackId,
      _selectedClipId,
      _selectedClipId == null ? null : track.kind,
    );
  }

  void _clearSelection() {
    setState(() {
      _selectedClipId = null;
      _selectedClipIds.clear();
    });
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
        final onMoveClips = widget.onMoveClips;
        final selection = _selectedClipIds.toList(growable: false);
        // A drag that started on one of several selected clips moves the whole
        // set in one command, so one gesture still costs one undo.
        if (onMoveClips != null &&
            selection.length > 1 &&
            selection.contains(gesture.clip.clipId) &&
            gesture.targetTrackId == gesture.sourceTrack.trackId) {
          final delta = resolvedTargetTime.add(
            ProjectRationalTime(
              -gesture.clip.timelineStart.numerator,
              gesture.clip.timelineStart.denominator,
            ),
          );
          if (delta.numerator != BigInt.zero) {
            await onMoveClips(
              gesture.project,
              gesture.sourceTrack.trackId,
              selection,
              delta,
            );
            return;
          }
        }
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

/// How wide the playhead's grab band is in the lanes. Wide enough for a
/// pointer, narrow enough that a clip press beside the line still selects.
const double _playheadGrabWidth = 12;

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
    this.playheadX,
  });

  final double pixelsPerSecond;
  final double tickSeconds;

  /// Where the playhead sits, so a tick label that would sit under its cap
  /// yields instead of showing a half covered word.
  final double? playheadX;

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
      final labelLeft = x + 4;
      final playhead = playheadX;
      final hidden =
          playhead != null &&
          labelLeft + painter.width > playhead - 7 &&
          labelLeft < playhead + 7;
      if (!hidden) painter.paint(canvas, Offset(labelLeft, 2));
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
      oldDelegate.tickSeconds != tickSeconds ||
      oldDelegate.playheadX != playheadX;
}

/// The grab notch over the ruler's playhead.
///
/// A two pixel line states a position; the notch states that the line has a
/// draggable end, which is what the pointer cursor alone cannot say.
class _PlayheadHandlePainter extends CustomPainter {
  const _PlayheadHandlePainter({required this.color, required this.apex});

  final Color color;

  /// Horizontal position of the cap's point in painter coordinates.
  final double apex;

  @override
  void paint(Canvas canvas, Size size) {
    final path = Path()
      ..moveTo(0, 0)
      ..lineTo(size.width, 0)
      ..lineTo(apex, size.height)
      ..close();
    canvas.drawPath(path, Paint()..color = color);
  }

  @override
  bool shouldRepaint(_PlayheadHandlePainter oldDelegate) =>
      oldDelegate.color != color || oldDelegate.apex != apex;
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
    this.contentEnd,
    this.onScrubTo,
    this.onScrubEnd,
  });

  final List<ProjectTimelineMarker> markers;
  final ProjectRationalTime? playhead;

  /// The last scrubbable time. The ruler stops there, like the viewer slider.
  final ProjectRationalTime? contentEnd;
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

  /// Presses and drags anywhere on the ruler move the playhead, the gesture
  /// CapCut uses. Null when no preview can seek, so the ruler stays inert.
  final ValueChanged<ProjectRationalTime>? onScrubTo;
  final VoidCallback? onScrubEnd;

  @override
  State<_TimelineMarkerRuler> createState() => _TimelineMarkerRulerState();
}

class _TimelineMarkerRulerState extends State<_TimelineMarkerRuler> {
  ProjectTimelineMarker? _draggingMarker;
  Offset? _dragStart;
  double _dragDeltaPixels = 0;
  bool _scrubbing = false;

  @override
  Widget build(BuildContext context) {
    final markerWidgets = <Widget>[
      Positioned.fill(
        child: CustomPaint(
          painter: _TimelineRulerPainter(
            pixelsPerSecond: widget.pixelsPerSecond,
            tickSeconds: widget.tickSeconds,
            playheadX: widget.playhead == null ? null : _xFor(widget.playhead!),
          ),
        ),
      ),
      // Pressing or dragging the ruler moves the playhead, the gesture CapCut
      // uses. The layer sits under the markers and the "Markers" button so a
      // marker keeps its own press; the timeline body still scrolls from the
      // wheel and the scrollbar.
      if (widget.onScrubTo != null)
        Positioned.fill(
          child: MouseRegion(
            cursor: SystemMouseCursors.resizeLeftRight,
            child: Semantics(
              label: 'Timeline ruler',
              hint: 'Press or drag to move the playhead',
              child: GestureDetector(
                key: const ValueKey('timeline-ruler-scrub'),
                behavior: HitTestBehavior.translucent,
                onTapDown: (details) => _scrubTo(details.localPosition.dx),
                onTapUp: (_) => _endScrub(),
                onTapCancel: _endScrub,
                onHorizontalDragStart: (details) =>
                    _scrubTo(details.localPosition.dx),
                onHorizontalDragUpdate: (details) =>
                    _scrubTo(details.localPosition.dx),
                onHorizontalDragEnd: (_) => _endScrub(),
                onHorizontalDragCancel: _endScrub,
              ),
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
      if (widget.playhead != null)
        // The notch stays inside the canvas at either end and keeps its point
        // on the playhead line, so time zero reads as a full cap instead of a
        // half clipped one.
        _playheadHandle(_xFor(widget.playhead!)),
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

  /// The ruler's playhead cap.
  ///
  /// The cap is held inside the canvas and its point follows the playhead
  /// line, so time zero and the far end show a whole cap instead of a clipped
  /// half.
  Widget _playheadHandle(double x) {
    const capWidth = 12.0;
    final left = (x - capWidth / 2)
        .clamp(0.0, math.max(0.0, widget.width - capWidth))
        .toDouble();
    return Positioned(
      key: const ValueKey('timeline-playhead-handle'),
      left: left,
      top: 1,
      width: capWidth,
      height: 9,
      child: IgnorePointer(
        child: CustomPaint(
          painter: _PlayheadHandlePainter(
            color: OrColors.selection,
            apex: (x - left).clamp(0.0, capWidth).toDouble(),
          ),
        ),
      ),
    );
  }

  /// Turns a ruler-local pointer position into a playhead time.
  ///
  /// Pressing past the end of the sequence lands on the end rather than on a
  /// time the preview has no frame for, which keeps the drag reversible: the
  /// playhead stays where the last rendered frame is.
  void _scrubTo(double dx) {
    final onScrubTo = widget.onScrubTo;
    if (onScrubTo == null) return;
    final pixels = dx.clamp(0.0, math.max(0.0, widget.width)).toDouble();
    var time = rationalTimeFromSeconds(pixels / widget.pixelsPerSecond);
    final end = widget.contentEnd;
    if (end != null && _compareProjectRational(time, end) > 0) {
      time = end;
    }
    _scrubbing = true;
    onScrubTo(time);
  }

  void _endScrub() {
    if (!_scrubbing) return;
    setState(() => _scrubbing = false);
    widget.onScrubEnd?.call();
  }

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

enum _ClipMenuChoice {
  split,
  duplicate,
  reverse,
  freeze,
  delete,
  rippleDelete,
  advanced,
}

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
    required this.selectedClipIds,
    required this.bladeMode,
    required this.onBladeSplit,
    required this.marqueeRect,
    required this.onMarqueeStart,
    required this.onMarqueeUpdate,
    required this.onMarqueeEnd,
    this.onScrubTo,
    this.onScrubEnd,
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

  /// Every clip the owner has selected. Shift-click extends the set so a bulk
  /// delete is one gesture instead of a dialog per clip.
  final Set<String> selectedClipIds;
  final bool bladeMode;
  final void Function(ProjectTimelineClip clip, ProjectRationalTime time)?
  onBladeSplit;

  /// The rubber band being drawn on this lane, in lane-local coordinates.
  final Rect? marqueeRect;
  final void Function(Offset local)? onMarqueeStart;
  final void Function(Offset local)? onMarqueeUpdate;
  final VoidCallback? onMarqueeEnd;

  /// Moves the playhead from a lane press, or from dragging its line, which is
  /// the gesture CapCut binds to the whole timeline. Null when nothing can
  /// seek, so the lane keeps its select-only behaviour.
  final ValueChanged<ProjectRationalTime>? onScrubTo;
  final VoidCallback? onScrubEnd;
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
    final playheadSeconds = playhead?.secondsForDisplay;
    final playheadVisible =
        playheadSeconds != null &&
        playheadSeconds.isFinite &&
        playheadSeconds >= 0 &&
        playheadSeconds * pixelsPerSecond <= width;
    final playheadX = playheadVisible ? playheadSeconds * pixelsPerSecond : 0.0;
    // The grab band is held inside the canvas, so the line stays grabbable at
    // either end of the sequence.
    final grabLeft = (playheadX - _playheadGrabWidth / 2)
        .clamp(0.0, math.max(0.0, width - _playheadGrabWidth))
        .toDouble();
    final canScrub = onScrubTo != null && !bladeMode && !track.state.locked;
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
            if (onMarqueeStart != null)
              Positioned.fill(
                // A gesture detector, not a raw listener: the band has to win
                // the horizontal-drag arena, otherwise the timeline scroller
                // takes the pointer and leaves its content un-hit-testable
                // (IgnorePointer) for the rest of the gesture.
                child: GestureDetector(
                  key: ValueKey('timeline-marquee-${track.trackId}'),
                  behavior: HitTestBehavior.translucent,
                  onHorizontalDragStart: (details) {
                    if (bladeMode || track.state.locked) return;
                    final local = details.localPosition;
                    final seconds = local.dx / pixelsPerSecond;
                    final overClip = clips.any(
                      (clip) =>
                          seconds >= clip.timelineStart.secondsForDisplay &&
                          seconds < clip.timelineEnd.secondsForDisplay,
                    );
                    if (overClip) return;
                    onMarqueeStart!(local);
                  },
                  onHorizontalDragUpdate: (details) =>
                      onMarqueeUpdate?.call(details.localPosition),
                  onHorizontalDragEnd: (_) => onMarqueeEnd?.call(),
                  onHorizontalDragCancel: () => onMarqueeEnd?.call(),
                  // A plain click on empty lane space moves the playhead. Clip
                  // presses still land on the clip above this layer, so
                  // selecting a clip never moves the playhead.
                  onTapUp: onScrubTo == null || bladeMode || track.state.locked
                      ? null
                      : (details) {
                          onScrubTo!(
                            rationalTimeFromSeconds(
                              details.localPosition.dx / pixelsPerSecond,
                            ),
                          );
                          onScrubEnd?.call();
                        },
                  onTapCancel: onScrubTo == null ? null : onScrubEnd,
                ),
              ),
            if (marqueeRect != null)
              Positioned.fromRect(
                rect: marqueeRect!,
                child: IgnorePointer(
                  child: DecoratedBox(
                    key: const ValueKey('timeline-marquee-band'),
                    decoration: BoxDecoration(
                      color: OrColors.selection.withValues(alpha: 0.12),
                      border: Border.all(color: OrColors.selection, width: 1),
                    ),
                  ),
                ),
              ),
            // The playhead line is grabbable in the lane, the CapCut gesture.
            // The band sits under the clips, so a press on a clip — however
            // narrow — still selects, moves or trims it, and the line is
            // grabbable everywhere else on the track.
            if (canScrub && playheadVisible)
              Positioned(
                key: ValueKey('timeline-playhead-grab-${track.trackId}'),
                left: grabLeft,
                top: 0,
                bottom: 0,
                width: _playheadGrabWidth,
                child: MouseRegion(
                  cursor: SystemMouseCursors.resizeLeftRight,
                  child: GestureDetector(
                    behavior: HitTestBehavior.translucent,
                    onTapDown: (details) =>
                        _scrubFromGrab(grabLeft, details.localPosition.dx),
                    onTapUp: (_) => onScrubEnd?.call(),
                    onTapCancel: onScrubEnd,
                    onHorizontalDragStart: (details) =>
                        _scrubFromGrab(grabLeft, details.localPosition.dx),
                    onHorizontalDragUpdate: (details) =>
                        _scrubFromGrab(grabLeft, details.localPosition.dx),
                    onHorizontalDragEnd: (_) => onScrubEnd?.call(),
                    onHorizontalDragCancel: () => onScrubEnd?.call(),
                  ),
                ),
              ),
            for (final clip in clips)
              if (gesture?.clip.clipId != clip.clipId)
                _positionedClip(context, clip),
            if (gesture != null && gesture!.targetTrackId == track.trackId)
              _positionedGhost(gesture!),
            if (playheadVisible)
              Positioned(
                key: ValueKey('timeline-playhead-${track.trackId}'),
                left: playheadX,
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

  /// Turns a press on the playhead's grab band into a playhead time.
  ///
  /// The band reports positions relative to itself, so the lane offset comes
  /// back from [grabLeft]; the panel clamps the result to the sequence end.
  void _scrubFromGrab(double grabLeft, double dx) {
    final onScrubTo = this.onScrubTo;
    if (onScrubTo == null) return;
    final x = (grabLeft + dx).clamp(0.0, math.max(0.0, width)).toDouble();
    onScrubTo(rationalTimeFromSeconds(x / pixelsPerSecond));
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
            : mediaDisplayName(media.sourceUri),
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
              final seconds =
                  start +
                  (local.dx + math.min(6.0, clipWidth / 2)) / pixelsPerSecond;
              bladeSplit(clip, rationalTimeFromSeconds(seconds));
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
        : '${rateLabel(rate)}×';
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
          color: selectedClipIds.contains(clip.clipId)
              ? OrColors.selection
              : OrColors.borderStrong,
          width: clip.clipId == selectedClipId
              ? 1.5
              : (selectedClipIds.contains(clip.clipId) ? 1.2 : 1),
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
                      clipWidth: clipWidth,
                      start: true,
                      label: 'Trim start of $title',
                    ),
                    _trimHandle(
                      context,
                      clip,
                      clipWidth: clipWidth,
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
    required double clipWidth,
    required bool start,
    required String label,
  }) {
    final onStart = start ? onTrimStart : onTrimEnd;
    final compact = OrBreakpoints.isCompact(MediaQuery.sizeOf(context).width);
    final comfortable = compact
        ? OrTouchTargets.comfortable
        : OrTouchTargets.minimum;
    // A handle that covers the whole clip makes the clip impossible to move:
    // two inflated targets meet in the middle. Capping each at a third of the
    // clip keeps a draggable centre at every zoom level (a quarter-second clip
    // at 100 % is 16 px wide, which the 24 px targets used to swallow whole).
    final hitWidth = math.min(comfortable, clipWidth / 3);
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
  Future<(ProjectReadModel, ProjectTimelineTracks)?> Function(
    ProjectTimelineTrackKind kind,
  )?
  onEnsureTrack,
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
    // CapCut-class behaviour: adding media creates the track it needs. One command
    // sequence in the shell answers the requirement, so the owner never reads a
    // dead end like "Add a compatible Video or Audio track first."
    if (!hasLockedCompatibleTrack && onEnsureTrack != null) {
      final created = await onEnsureTrack(
        media.videoDetails != null
            ? ProjectTimelineTrackKind.video
            : ProjectTimelineTrackKind.audio,
      );
      if (created != null && context.mounted) {
        final (refreshedProject, refreshedTracks) = created;
        return _showInsertTimelineDialog(
          context: context,
          project: refreshedProject,
          media: media,
          tracks: refreshedTracks,
          suggestedStart: suggestedStart,
          onInsert: onInsert,
          onEnsureTrack: onEnsureTrack,
        );
      }
    }
    if (!context.mounted) return;
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

int _greatestCommonDivisor(int left, int right) {
  var a = left.abs();
  var b = right.abs();
  while (b != 0) {
    final remainder = a % b;
    a = b;
    b = remainder;
  }
  return a == 0 ? 1 : a;
}

String _playbackSpeedLabel(ProjectRationalRate rate) {
  final value = rate.numerator / rate.denominator;
  if (rate.numerator == rate.denominator) return '1×';
  return '${value.toStringAsFixed(value == value.roundToDouble() ? 0 : 2)}×';
}

Future<ProjectRationalRate?> _showPlaybackSpeedDialog(
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
                      label: Text(_playbackSpeedLabel(rate)),
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
              final divisor = _greatestCommonDivisor(hundredths, 100);
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

Widget _textStyleSlider({
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
                _textStyleSlider(
                  keyName: 'timeline-text-offset-x',
                  label: 'Horizontal',
                  value: offsetX,
                  min: -100,
                  max: 100,
                  display: '${offsetX.round()}%',
                  onChanged: (value) => setDialogState(() => offsetX = value),
                ),
                _textStyleSlider(
                  keyName: 'timeline-text-offset-y',
                  label: 'Vertical',
                  value: offsetY,
                  min: -100,
                  max: 100,
                  display: '${offsetY.round()}%',
                  onChanged: (value) => setDialogState(() => offsetY = value),
                ),
                _textStyleSlider(
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
                  _textStyleSlider(
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
              if (isProjectWorkspace)
                const _EditorTool('Assistant', Icons.chat_bubble_outline),
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

enum _RightPanelTab { inspector, assistant }

/// The right rail's two surfaces. The Assistant tab is a tab, not a new
/// permanent region: it reuses the inspector slot and keeps the shell stable.
class _RightPanelTabs extends StatelessWidget {
  const _RightPanelTabs({
    required this.selected,
    required this.showInspector,
    required this.assistantBusy,
    required this.onSelected,
  });

  final _RightPanelTab selected;
  final bool showInspector;
  final bool assistantBusy;
  final ValueChanged<_RightPanelTab> onSelected;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 38,
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: OrSpacing.x2,
          vertical: OrSpacing.x1,
        ),
        child: Row(
          children: [
            Expanded(
              child: _RightPanelTabButton(
                keyName: 'editor-tab-inspector',
                label: 'Inspector',
                selected: selected == _RightPanelTab.inspector,
                onTap: () => onSelected(_RightPanelTab.inspector),
              ),
            ),
            const SizedBox(width: OrSpacing.x1),
            Expanded(
              child: _RightPanelTabButton(
                keyName: 'editor-tab-assistant',
                label: 'Assistant',
                selected: selected == _RightPanelTab.assistant,
                busy: assistantBusy,
                onTap: () => onSelected(_RightPanelTab.assistant),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _RightPanelTabButton extends StatelessWidget {
  const _RightPanelTabButton({
    required this.keyName,
    required this.label,
    required this.selected,
    required this.onTap,
    this.busy = false,
  });

  final String keyName;
  final String label;
  final bool selected;
  final VoidCallback onTap;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      button: true,
      selected: selected,
      label: '$label panel',
      child: OrFocusRing(
        radius: OrRadii.small,
        child: InkWell(
          key: ValueKey(keyName),
          onTap: onTap,
          borderRadius: BorderRadius.circular(OrRadii.small),
          child: Container(
            height: 28,
            decoration: BoxDecoration(
              color: selected ? OrColors.surfaceSelected : Colors.transparent,
              border: Border.all(
                color: selected ? OrColors.borderStrong : Colors.transparent,
              ),
              borderRadius: BorderRadius.circular(OrRadii.small),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                if (busy) ...[
                  const SizedBox(
                    width: 8,
                    height: 8,
                    child: CircularProgressIndicator(
                      strokeWidth: 1.4,
                      color: OrColors.ai,
                    ),
                  ),
                  const SizedBox(width: OrSpacing.x1),
                ],
                Text(
                  label,
                  style: TextStyle(
                    color: selected ? OrColors.text : OrColors.textSecondary,
                    fontSize: OrType.tinySize,
                    fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                  ),
                ),
              ],
            ),
          ),
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
