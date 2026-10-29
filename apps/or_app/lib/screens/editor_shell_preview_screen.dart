import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

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
import 'editor_panel_header.dart';
import 'editor_timeline_panel.dart';
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
    this.mediaPreviewUpdates,
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
  final ValueListenable<Map<String, ProjectMediaPreview>>? mediaPreviewUpdates;
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
  final GlobalKey<EditorTimelinePanelState> _timelineKey = GlobalKey(
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
                  mediaPreviewUpdates: widget.mediaPreviewUpdates,
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
          child: EditorTimelinePanel(
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
          child: EditorTimelinePanel(
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
                mediaPreviewUpdates: widget.mediaPreviewUpdates,
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
    await showAddTimelineMarkerDialog(
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
    final edit = await showTimelineTextClipDialog(
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
    this.mediaPreviewUpdates,
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
  final ValueListenable<Map<String, ProjectMediaPreview>>? mediaPreviewUpdates;
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
        previewUpdates: mediaPreviewUpdates,
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
            ? (media) => showInsertTimelineDialog(
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
