import 'dart:async';
import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:or_viewer_texture/or_viewer_texture.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../project/project_gateway.dart';
import '../widgets/or_widgets.dart';

class EditorShellPreviewScreen extends StatefulWidget {
  const EditorShellPreviewScreen({
    super.key,
    this.isProjectWorkspace = false,
    this.project,
    this.projectGateway,
    this.projectSession,
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
    this.onLoadMoreMedia,
    this.onRefreshMedia,
    this.onRemoveMedia,
    this.onAddVideoTrack,
    this.onAddAudioTrack,
    this.onAddTextTrack,
    this.onAddCaptionTrack,
    this.onRemoveTimelineTrack,
    this.onSetTimelineTrackState,
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
    this.onRippleDeleteTimelineClip,
    this.onAddTimelineMarker,
    this.onMoveTimelineMarker,
    this.onRenameTimelineMarker,
    this.onDeleteTimelineMarker,
    this.onUpdateTimelineClipVisualSettings,
    this.onUpdateTimelineClipAudioSettings,
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
  final VoidCallback? onLoadMoreMedia;
  final VoidCallback? onRefreshMedia;
  final ValueChanged<ProjectMediaItem>? onRemoveMedia;
  final VoidCallback? onAddVideoTrack;
  final VoidCallback? onAddAudioTrack;
  final VoidCallback? onAddTextTrack;
  final VoidCallback? onAddCaptionTrack;
  final Future<void> Function(ProjectReadModel, ProjectTimelineTrack)?
  onRemoveTimelineTrack;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineTrackState,
  )?
  onSetTimelineTrackState;
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
  )?
  onTrimTimelineClip;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectTimelineClip clip,
    ProjectRationalTime timelineTime,
  )?
  onSplitTimelineClip;
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
  final VoidCallback? onSave;
  final VoidCallback? onRename;
  final VoidCallback? onUndo;
  final VoidCallback? onRedo;
  final VoidCallback? onClose;
  final VoidCallback? onDiscardRecovery;

  @override
  State<EditorShellPreviewScreen> createState() =>
      _EditorShellPreviewScreenState();
}

class _EditorTool {
  const _EditorTool(this.label, this.icon);

  final String label;
  final IconData icon;
}

const _editorTools = [
  _EditorTool('Media', Icons.perm_media_outlined),
  _EditorTool('Templates', Icons.dashboard_outlined),
  _EditorTool('Text', Icons.text_fields_outlined),
  _EditorTool('Captions', Icons.subtitles_outlined),
  _EditorTool('Stickers / Shapes', Icons.category_outlined),
  _EditorTool('Audio', Icons.graphic_eq_outlined),
  _EditorTool('Effects', Icons.auto_fix_high_outlined),
  _EditorTool('Transitions', Icons.compare_arrows_outlined),
  _EditorTool('Filters / Color', Icons.tune_outlined),
  _EditorTool('AI', Icons.auto_awesome_outlined),
  _EditorTool('More', Icons.more_horiz_outlined),
];

class _EditorShellPreviewScreenState extends State<EditorShellPreviewScreen> {
  String _selectedTool = 'Media';
  String? _selectedTrackId;
  String? _selectedClipId;
  ProjectTimelineTrackKind? _selectedTrackKind;
  bool _snapEnabled = true;
  late final ValueNotifier<ProjectPreviewState?> _previewState;

  @override
  void initState() {
    super.initState();
    _previewState = ValueNotifier(null);
  }

  @override
  void didUpdateWidget(covariant EditorShellPreviewScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
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

  @override
  void dispose() {
    _previewState.dispose();
    super.dispose();
  }

  void _onPreviewStateChanged(ProjectPreviewState state) {
    _previewState.value = state;
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
              if (widget.isProjectWorkspace)
                _ProjectWorkspaceHeader(
                  project: widget.project,
                  busy: widget.busy,
                  onSave: widget.onSave,
                  onRename: widget.onRename,
                  onUndo: widget.onUndo,
                  onRedo: widget.onRedo,
                  onClose: widget.onClose,
                )
              else
                const _EditorPreviewHeader(),
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
              _EditorToolRail(
                selected: _selectedTool,
                isProjectWorkspace: widget.isProjectWorkspace,
                onSelected: (tool) => setState(() => _selectedTool = tool),
              ),
              const VerticalDivider(width: 1),
              SizedBox(
                width: wide ? 224 : 172,
                child: _EditorToolPanel(
                  selectedTool: _selectedTool,
                  isProjectWorkspace: widget.isProjectWorkspace,
                  project: widget.project,
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
                  onLoadMoreMedia: widget.onLoadMoreMedia,
                  onRefreshMedia: widget.onRefreshMedia,
                  onRemoveMedia: widget.busy ? null : widget.onRemoveMedia,
                ),
              ),
              const VerticalDivider(width: 1),
              Expanded(
                child: _ViewerPanel(
                  compact: false,
                  gateway: widget.projectGateway,
                  session: widget.projectSession,
                  project: widget.project,
                  onStateChanged: _onPreviewStateChanged,
                ),
              ),
              const VerticalDivider(width: 1),
              SizedBox(
                width: wide ? 236 : 188,
                child: _InspectorPanel(
                  isProjectWorkspace: widget.isProjectWorkspace,
                  project: widget.project,
                  gateway: widget.projectGateway,
                  session: widget.projectSession,
                  trackId: _selectedTrackId,
                  clipId: _selectedClipId,
                  isVisualTrack:
                      _selectedTrackKind == ProjectTimelineTrackKind.video ||
                      _selectedTrackKind == ProjectTimelineTrackKind.text ||
                      _selectedTrackKind == ProjectTimelineTrackKind.caption,
                  isAudioTrack:
                      _selectedTrackKind == ProjectTimelineTrackKind.audio,
                  trackLocked: _selectedInspectorTrack?.state.locked ?? true,
                  busy: widget.busy,
                  onUpdate: widget.onUpdateTimelineClipVisualSettings,
                  onUpdateAudio: widget.onUpdateTimelineClipAudioSettings,
                ),
              ),
            ],
          ),
        ),
        const _ResizeDivider(horizontal: true),
        _TimelineToolbar(
          compact: false,
          isProjectWorkspace: widget.isProjectWorkspace,
          busy: widget.busy,
          onUndo: widget.onUndo,
          onAddVideoTrack: widget.onAddVideoTrack,
          onAddAudioTrack: widget.onAddAudioTrack,
          onAddTextTrack: widget.onAddTextTrack,
          onAddCaptionTrack: widget.onAddCaptionTrack,
          onAddTitle: () =>
              _showAddTimelineTextClip(ProjectTimelineClipContentKind.text),
          onAddCaption: () =>
              _showAddTimelineTextClip(ProjectTimelineClipContentKind.caption),
          onAddMarker: widget.isProjectWorkspace
              ? _showAddTimelineMarker
              : null,
          snapEnabled: _snapEnabled,
          onSnapChanged: widget.isProjectWorkspace
              ? (value) => setState(() => _snapEnabled = value)
              : null,
        ),
        Expanded(
          flex: 2,
          child: _TimelinePanel(
            compact: false,
            previewState: _previewState,
            isProjectWorkspace: widget.isProjectWorkspace,
            project: widget.project,
            tracks: widget.timelineTracks,
            clipPages: widget.timelineClipPages,
            markerPage: widget.timelineMarkerPage,
            loadingMoreTracks: widget.timelineLoadingMoreTracks,
            loadingMoreMarkers: widget.timelineLoadingMoreMarkers,
            loading: widget.timelineLoading,
            error: widget.timelineLoadError,
            busy: widget.busy,
            onAddVideoTrack: widget.onAddVideoTrack,
            onAddAudioTrack: widget.onAddAudioTrack,
            onAddTextTrack: widget.onAddTextTrack,
            onAddCaptionTrack: widget.onAddCaptionTrack,
            onRemoveTrack: widget.onRemoveTimelineTrack,
            onSetTrackState: widget.onSetTimelineTrackState,
            onLoadMore: widget.onLoadMoreTimelineClips,
            onLoadMoreMarkers: widget.onLoadMoreTimelineMarkers,
            onRefresh: widget.onRefreshTimeline,
            onMoveClip: widget.onMoveTimelineClip,
            onDuplicateClip: widget.onDuplicateTimelineClip,
            onResolveSnap: widget.onResolveTimelineSnap,
            onDeleteClip: widget.onDeleteTimelineClip,
            onTrimClip: widget.onTrimTimelineClip,
            onSplitClip: widget.onSplitTimelineClip,
            onRippleDeleteClip: widget.onRippleDeleteTimelineClip,
            onUpdateTextClip: widget.onUpdateTimelineTextClip,
            onAddMarker: widget.onAddTimelineMarker,
            onMoveMarker: widget.onMoveTimelineMarker,
            onRenameMarker: widget.onRenameTimelineMarker,
            onDeleteMarker: widget.onDeleteTimelineMarker,
            snapEnabled: _snapEnabled,
            onTimelineEditError: _showTimelineEditError,
            mediaItems: widget.mediaPage?.items ?? const [],
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
            compact: true,
            gateway: widget.projectGateway,
            session: widget.projectSession,
            project: widget.project,
            onStateChanged: _onPreviewStateChanged,
          ),
        ),
        _TimelineToolbar(
          compact: true,
          isProjectWorkspace: widget.isProjectWorkspace,
          busy: widget.busy,
          onUndo: widget.onUndo,
          onAddVideoTrack: widget.onAddVideoTrack,
          onAddAudioTrack: widget.onAddAudioTrack,
          onAddTextTrack: widget.onAddTextTrack,
          onAddCaptionTrack: widget.onAddCaptionTrack,
          onAddTitle: () =>
              _showAddTimelineTextClip(ProjectTimelineClipContentKind.text),
          onAddCaption: () =>
              _showAddTimelineTextClip(ProjectTimelineClipContentKind.caption),
          onAddMarker: widget.isProjectWorkspace
              ? _showAddTimelineMarker
              : null,
          snapEnabled: _snapEnabled,
          onSnapChanged: widget.isProjectWorkspace
              ? (value) => setState(() => _snapEnabled = value)
              : null,
        ),
        Expanded(
          flex: 2,
          child: _TimelinePanel(
            compact: true,
            previewState: _previewState,
            isProjectWorkspace: widget.isProjectWorkspace,
            project: widget.project,
            tracks: widget.timelineTracks,
            clipPages: widget.timelineClipPages,
            markerPage: widget.timelineMarkerPage,
            loadingMoreTracks: widget.timelineLoadingMoreTracks,
            loadingMoreMarkers: widget.timelineLoadingMoreMarkers,
            loading: widget.timelineLoading,
            error: widget.timelineLoadError,
            busy: widget.busy,
            onAddVideoTrack: widget.onAddVideoTrack,
            onAddAudioTrack: widget.onAddAudioTrack,
            onAddTextTrack: widget.onAddTextTrack,
            onAddCaptionTrack: widget.onAddCaptionTrack,
            onRemoveTrack: widget.onRemoveTimelineTrack,
            onSetTrackState: widget.onSetTimelineTrackState,
            onLoadMore: widget.onLoadMoreTimelineClips,
            onLoadMoreMarkers: widget.onLoadMoreTimelineMarkers,
            onRefresh: widget.onRefreshTimeline,
            onMoveClip: widget.onMoveTimelineClip,
            onDuplicateClip: widget.onDuplicateTimelineClip,
            onResolveSnap: widget.onResolveTimelineSnap,
            onDeleteClip: widget.onDeleteTimelineClip,
            onTrimClip: widget.onTrimTimelineClip,
            onSplitClip: widget.onSplitTimelineClip,
            onRippleDeleteClip: widget.onRippleDeleteTimelineClip,
            onUpdateTextClip: widget.onUpdateTimelineTextClip,
            onAddMarker: widget.onAddTimelineMarker,
            onMoveMarker: widget.onMoveTimelineMarker,
            onRenameMarker: widget.onRenameTimelineMarker,
            onDeleteMarker: widget.onDeleteTimelineMarker,
            snapEnabled: _snapEnabled,
            onTimelineEditError: _showTimelineEditError,
            mediaItems: widget.mediaPage?.items ?? const [],
            onAddMediaToTimeline: widget.onAddMediaToTimeline,
            onClipSelectionChanged: _onTimelineClipSelectionChanged,
          ),
        ),
        _MobileToolDock(
          selected: _selectedTool,
          onSelected: _showMobileToolSheet,
        ),
      ],
    );
  }

  Future<void> _showMobileToolSheet(String tool) async {
    setState(() => _selectedTool = tool);
    await showModalBottomSheet<void>(
      context: context,
      useSafeArea: true,
      backgroundColor: Colors.transparent,
      builder: (context) => _UnavailableToolSheet(tool: tool),
    );
  }

  void _showTimelineEditError(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(content: Text(message)));
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

class _ProjectWorkspaceHeader extends StatelessWidget {
  const _ProjectWorkspaceHeader({
    required this.project,
    required this.busy,
    required this.onSave,
    required this.onRename,
    required this.onUndo,
    required this.onRedo,
    required this.onClose,
  });

  final ProjectReadModel? project;
  final bool busy;
  final VoidCallback? onSave;
  final VoidCallback? onRename;
  final VoidCallback? onUndo;
  final VoidCallback? onRedo;
  final VoidCallback? onClose;

  @override
  Widget build(BuildContext context) {
    final current = project;
    return Container(
      width: double.infinity,
      height: 52,
      padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x3),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Row(
        children: [
          const Icon(
            Icons.folder_open_outlined,
            size: 17,
            color: OrColors.textSecondary,
          ),
          const SizedBox(width: OrSpacing.x2),
          Expanded(
            child: Text(
              current?.name ?? 'Opening project…',
              key: const ValueKey('workspace-project-name'),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
            ),
          ),
          if (current != null) ...[
            Text(
              'Revision ${current.revision}',
              key: const ValueKey('workspace-project-revision'),
              style: const TextStyle(
                fontSize: 11,
                color: OrColors.textSecondary,
              ),
            ),
            const SizedBox(width: OrSpacing.x2),
            OrBadge(current.dirty ? 'Unsaved changes' : 'Saved'),
          ],
          const SizedBox(width: OrSpacing.x2),
          Expanded(
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                children: [
                  _WorkspaceAction(
                    'Rename',
                    Icons.edit_outlined,
                    onRename,
                    busy,
                  ),
                  _WorkspaceAction(
                    'Save',
                    Icons.save_outlined,
                    onSave,
                    busy || current?.dirty != true,
                  ),
                  _WorkspaceAction('Undo', Icons.undo_outlined, onUndo, busy),
                  _WorkspaceAction('Redo', Icons.redo_outlined, onRedo, busy),
                  _WorkspaceAction(
                    'Close',
                    Icons.close_outlined,
                    onClose,
                    busy,
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

class _WorkspaceAction extends StatelessWidget {
  const _WorkspaceAction(this.label, this.icon, this.onPressed, this.disabled);

  final String label;
  final IconData icon;
  final VoidCallback? onPressed;
  final bool disabled;

  @override
  Widget build(BuildContext context) => TextButton.icon(
    key: ValueKey('workspace-$label'.toLowerCase()),
    onPressed: disabled ? null : onPressed,
    icon: Icon(icon, size: 16),
    label: Text(label),
  );
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
          Expanded(child: Text(message, style: const TextStyle(fontSize: 12))),
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
              style: TextStyle(fontSize: 12, fontWeight: FontWeight.w500),
            ),
          ),
          OrBadge('Non-functional'),
        ],
      ),
    );
  }
}

class _EditorToolRail extends StatelessWidget {
  const _EditorToolRail({
    required this.selected,
    required this.isProjectWorkspace,
    required this.onSelected,
  });

  final String selected;
  final bool isProjectWorkspace;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 52,
      child: ColoredBox(
        color: OrColors.backgroundRaised,
        child: SingleChildScrollView(
          child: Column(
            children: [
              for (final tool in _editorTools)
                Tooltip(
                  message: isProjectWorkspace && tool.label == 'Media'
                      ? 'Media library'
                      : '${tool.label} — unavailable in this Developer Preview',
                  child: Semantics(
                    button: true,
                    label: isProjectWorkspace && tool.label == 'Media'
                        ? 'Media library'
                        : '${tool.label}, unavailable in this Developer Preview',
                    child: InkWell(
                      key: ValueKey('editor-tool-${_toolKey(tool.label)}'),
                      onTap: () => onSelected(tool.label),
                      child: Container(
                        width: 48,
                        height: 48,
                        margin: const EdgeInsets.symmetric(vertical: 1),
                        decoration: BoxDecoration(
                          color: selected == tool.label
                              ? OrColors.surface
                              : Colors.transparent,
                          borderRadius: BorderRadius.circular(OrRadii.small),
                        ),
                        child: Column(
                          mainAxisAlignment: MainAxisAlignment.center,
                          children: [
                            Icon(
                              tool.icon,
                              size: 17,
                              color: OrColors.textMuted,
                            ),
                            const SizedBox(height: 3),
                            Text(
                              _shortToolLabel(tool.label),
                              maxLines: 1,
                              overflow: TextOverflow.clip,
                              style: const TextStyle(
                                color: OrColors.textMuted,
                                fontSize: 8,
                              ),
                            ),
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
    );
  }
}

class _EditorToolPanel extends StatelessWidget {
  const _EditorToolPanel({
    required this.selectedTool,
    required this.isProjectWorkspace,
    required this.project,
    required this.mediaPage,
    required this.timelineTracks,
    required this.mediaPreviews,
    required this.mediaLoading,
    required this.mediaLoadingMore,
    required this.mediaLoadError,
    required this.onImportMedia,
    required this.onLoadMoreMedia,
    required this.onRefreshMedia,
    required this.onRemoveMedia,
    required this.onAddMediaToTimeline,
  });

  final String selectedTool;
  final bool isProjectWorkspace;
  final ProjectReadModel? project;
  final ProjectMediaPage? mediaPage;
  final ProjectTimelineTracks? timelineTracks;
  final Map<String, ProjectMediaPreview> mediaPreviews;
  final bool mediaLoading;
  final bool mediaLoadingMore;
  final String? mediaLoadError;
  final VoidCallback? onImportMedia;
  final VoidCallback? onLoadMoreMedia;
  final VoidCallback? onRefreshMedia;
  final ValueChanged<ProjectMediaItem>? onRemoveMedia;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectMediaItem media,
    String trackId,
    ProjectRationalTime timelineStart,
    ProjectRationalTime sourceStart,
    ProjectRationalTime duration,
  )?
  onAddMediaToTimeline;

  @override
  Widget build(BuildContext context) {
    if (selectedTool == 'Media' && isProjectWorkspace) {
      return _MediaLibraryPanel(
        page: mediaPage,
        project: project,
        tracks: timelineTracks,
        previews: mediaPreviews,
        loading: mediaLoading,
        loadingMore: mediaLoadingMore,
        error: mediaLoadError,
        onImport: onImportMedia,
        onLoadMore: onLoadMoreMedia,
        onRefresh: onRefreshMedia,
        onRemove: onRemoveMedia,
        onAddMediaToTimeline: onAddMediaToTimeline,
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
                      fontSize: 13,
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

class _MediaLibraryPanel extends StatelessWidget {
  const _MediaLibraryPanel({
    required this.page,
    required this.project,
    required this.tracks,
    required this.previews,
    required this.loading,
    required this.loadingMore,
    required this.error,
    required this.onImport,
    required this.onLoadMore,
    required this.onRefresh,
    required this.onRemove,
    required this.onAddMediaToTimeline,
  });

  final ProjectMediaPage? page;
  final ProjectReadModel? project;
  final ProjectTimelineTracks? tracks;
  final Map<String, ProjectMediaPreview> previews;
  final bool loading;
  final bool loadingMore;
  final String? error;
  final VoidCallback? onImport;
  final VoidCallback? onLoadMore;
  final VoidCallback? onRefresh;
  final ValueChanged<ProjectMediaItem>? onRemove;
  final Future<void> Function(
    ProjectReadModel project,
    ProjectMediaItem media,
    String trackId,
    ProjectRationalTime timelineStart,
    ProjectRationalTime sourceStart,
    ProjectRationalTime duration,
  )?
  onAddMediaToTimeline;

  @override
  Widget build(BuildContext context) {
    final items = page?.items ?? const <ProjectMediaItem>[];
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
          const _PanelHeader(title: 'Media'),
          const Divider(height: 1),
          Padding(
            padding: const EdgeInsets.fromLTRB(
              OrSpacing.x2,
              OrSpacing.x2,
              OrSpacing.x2,
              0,
            ),
            child: OutlinedButton.icon(
              key: const ValueKey('media-import'),
              onPressed: onImport,
              icon: const Icon(Icons.add, size: 16),
              label: const Text('Import Media'),
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
                '${page!.totalCount} ${page!.totalCount == 1 ? 'item' : 'items'}',
                style: const TextStyle(color: OrColors.textMuted, fontSize: 10),
              ),
            ),
          if (loading && items.isEmpty)
            const Expanded(
              child: Center(child: CircularProgressIndicator(strokeWidth: 2)),
            )
          else if (items.isEmpty && error == null)
            const Expanded(
              child: Center(
                child: Padding(
                  padding: EdgeInsets.all(OrSpacing.x3),
                  child: Text(
                    'No media imported',
                    textAlign: TextAlign.center,
                    style: TextStyle(
                      color: OrColors.textSecondary,
                      fontSize: 12,
                    ),
                  ),
                ),
              ),
            )
          else
            Expanded(
              child: ListView(
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
                                !timelineSnapshotMatches
                            ? null
                            : () => unawaited(
                                _showInsertTimelineDialog(
                                  context: context,
                                  project: project!,
                                  media: item,
                                  tracks: tracks!,
                                  onInsert: onAddMediaToTimeline!,
                                ),
                              ),
                        onRemove: onRemove == null
                            ? null
                            : () => onRemove!(item),
                      ),
                    ),
                  if (error != null) ...[
                    Padding(
                      padding: const EdgeInsets.all(OrSpacing.x2),
                      child: Text(
                        error!,
                        style: const TextStyle(
                          color: OrColors.textSecondary,
                          fontSize: 11,
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
    required this.onRemove,
  });

  final ProjectMediaItem item;
  final ProjectMediaPreview? preview;
  final VoidCallback? onAddToTimeline;
  final VoidCallback? onRemove;

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
        child: Row(
          children: [
            _MediaItemPreview(item: item, preview: preview),
            const SizedBox(width: OrSpacing.x2),
            Expanded(
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
                        fontSize: 11,
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
                          fontSize: 9,
                          height: 1.35,
                        ),
                      ),
                  ],
                ),
              ),
            ),
            if (supportsTimelineDrag)
              Draggable<ProjectMediaItem>(
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
                          fontSize: 11,
                        ),
                      ),
                    ),
                  ),
                ),
                childWhenDragging: Opacity(opacity: 0.45, child: dragHandle()),
                child: dragHandle(),
              ),
            if (item.videoDetails != null || item.audioDetails != null)
              IconButton(
                key: ValueKey('media-add-timeline-${item.mediaId}'),
                tooltip: 'Add to Timeline',
                onPressed: onAddToTimeline,
                visualDensity: VisualDensity.compact,
                icon: const Icon(Icons.playlist_add_outlined, size: 18),
                color: OrColors.textMuted,
              ),
            IconButton(
              key: ValueKey('media-remove-${item.mediaId}'),
              tooltip: 'Remove from Project',
              onPressed: onRemove,
              visualDensity: VisualDensity.compact,
              icon: const Icon(Icons.remove_circle_outline, size: 17),
              color: OrColors.textMuted,
            ),
          ],
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
    required this.compact,
    required this.gateway,
    required this.session,
    required this.project,
    required this.onStateChanged,
  });

  final bool compact;
  final ProjectGateway? gateway;
  final ProjectSessionHandle? session;
  final ProjectReadModel? project;
  final ValueChanged<ProjectPreviewState> onStateChanged;

  @override
  State<_ViewerPanel> createState() => _ViewerPanelState();
}

class _ViewerPanelState extends State<_ViewerPanel> {
  static const _rateChoices = [
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

  ProjectPreviewState? _preview;
  ProjectRationalTime? _scrubPosition;
  String? _error;
  Timer? _timer;
  int? _textureId;
  int _epoch = 0;
  bool _busy = false;
  bool _tickInFlight = false;

  bool get _connected => widget.gateway != null && widget.session != null;

  @override
  void initState() {
    super.initState();
    unawaited(_initialize());
  }

  @override
  void didUpdateWidget(covariant _ViewerPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    final oldProject = oldWidget.project;
    final project = widget.project;
    if (oldWidget.gateway != widget.gateway ||
        oldWidget.session != widget.session ||
        oldProject?.projectId != project?.projectId ||
        oldProject?.projectInstanceId != project?.projectInstanceId ||
        oldProject?.revision != project?.revision) {
      unawaited(_initialize());
    }
  }

  @override
  void dispose() {
    _epoch++;
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
      final rendered = await gateway.previewSeek(session, state.position);
      _acceptState(rendered, epoch);
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
    if (_timer != null) return;
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
    }
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
      if (_isCurrent(epoch)) setState(() => _busy = false);
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
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: widget.compact ? 520 : 660,
          maxHeight: widget.compact ? 360 : 420,
        ),
        child: AspectRatio(
          aspectRatio: aspectRatio,
          child: Container(
            decoration: BoxDecoration(
              color: const Color(0xFF0C0C0D),
              border: Border.all(color: OrColors.borderStrong),
              borderRadius: BorderRadius.circular(OrRadii.small),
            ),
            clipBehavior: Clip.antiAlias,
            child: Stack(
              fit: StackFit.expand,
              children: [
                if (preview?.contentEnd == null)
                  const Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          Icons.movie_outlined,
                          size: 22,
                          color: OrColors.textMuted,
                        ),
                        SizedBox(height: OrSpacing.x2),
                        Text(
                          'No media loaded',
                          style: TextStyle(
                            color: OrColors.textSecondary,
                            fontSize: 13,
                          ),
                        ),
                      ],
                    ),
                  )
                else if (textureId != null)
                  Texture(
                    key: ValueKey('or-viewer-texture-$textureId'),
                    textureId: textureId,
                    filterQuality: FilterQuality.medium,
                  )
                else
                  Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          hasSession
                              ? Icons.error_outline
                              : Icons.movie_outlined,
                          size: 22,
                          color: OrColors.textMuted,
                        ),
                        const SizedBox(height: OrSpacing.x2),
                        Text(
                          hasSession
                              ? 'Viewer texture unavailable'
                              : 'Open a project to preview',
                          style: const TextStyle(
                            color: OrColors.textSecondary,
                            fontSize: 13,
                          ),
                        ),
                      ],
                    ),
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
                            fontSize: 10,
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
      child: Column(
        children: [
          _PanelHeader(
            title: 'Viewer',
            height: widget.compact ? 34 : 38,
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
                                    (gateway, session) => gateway.previewSeek(
                                      session,
                                      _timeFromSeconds(seconds),
                                    ),
                                  ),
                                );
                              },
                      ),
                    ),
                  ),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(
                        _clockLabel(0),
                        style: const TextStyle(
                          color: OrColors.textMuted,
                          fontFamily: 'monospace',
                          fontSize: 9,
                        ),
                      ),
                      Text(
                        _clockLabel(endSeconds),
                        style: const TextStyle(
                          color: OrColors.textMuted,
                          fontFamily: 'monospace',
                          fontSize: 9,
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
                      icon: const Icon(Icons.skip_previous_outlined, size: 18),
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
                      child: Text(
                        shownPosition == null
                            ? '0/1'
                            : _timeFeedback(
                                shownPosition,
                                preview?.presentedTime,
                              ),
                        key: const ValueKey('preview-time-readout'),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        textAlign: TextAlign.center,
                        style: const TextStyle(
                          color: OrColors.textSecondary,
                          fontFamily: 'monospace',
                          fontSize: 10,
                        ),
                      ),
                    ),
                    PopupMenuButton<_FrameRateChoice>(
                      key: const ValueKey('preview-frame-rate'),
                      tooltip: 'Set sequence frame rate',
                      enabled: _connected && widget.project != null && !_busy,
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
                        padding: const EdgeInsets.symmetric(horizontal: 6),
                        child: Text(
                          frameRate == null
                              ? 'Set rate'
                              : '${frameRate.canonical} fps',
                          key: const ValueKey('preview-rate-label'),
                          style: const TextStyle(
                            color: OrColors.textSecondary,
                            fontSize: 10,
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
                        fontSize: 10,
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

class _FrameRateChoice {
  const _FrameRateChoice({required this.rate, required this.label});
  const _FrameRateChoice.unset() : rate = null, label = 'Clear rate';

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

String _clockLabel(double seconds) {
  final milliseconds = (seconds * 1000).round();
  final minutes = milliseconds ~/ 60000;
  final wholeSeconds = milliseconds ~/ 1000 % 60;
  final fraction = milliseconds % 1000;
  return '${minutes.toString().padLeft(2, '0')}:${wholeSeconds.toString().padLeft(2, '0')}.${fraction.toString().padLeft(3, '0')}';
}

String _timeFeedback(
  ProjectRationalTime position,
  ProjectRationalTime? presentedTime,
) {
  final time = position.secondsForDisplay;
  final display = _clockLabel(time);
  final frame = presentedTime == null ? '' : ' · ${presentedTime.canonical}';
  return '$display · ${position.canonical}$frame';
}

class _InspectorPanel extends StatefulWidget {
  const _InspectorPanel({
    required this.isProjectWorkspace,
    required this.project,
    required this.gateway,
    required this.session,
    required this.trackId,
    required this.clipId,
    required this.isVisualTrack,
    required this.isAudioTrack,
    required this.trackLocked,
    required this.busy,
    required this.onUpdate,
    required this.onUpdateAudio,
  });

  final bool isProjectWorkspace;
  final ProjectReadModel? project;
  final ProjectGateway? gateway;
  final ProjectSessionHandle? session;
  final String? trackId;
  final String? clipId;
  final bool isVisualTrack;
  final bool isAudioTrack;
  final bool trackLocked;
  final bool busy;
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
    _fields['x']!.text = '${settings.xMilliCanvas}';
    _fields['y']!.text = '${settings.yMilliCanvas}';
    _fields['scale_x']!.text = '${settings.scaleXMilli}';
    _fields['scale_y']!.text = '${settings.scaleYMilli}';
    _fields['rotation']!.text = '${settings.rotationMilliDegrees}';
    _fields['anchor_x']!.text = '${settings.anchorXBasisPoints}';
    _fields['anchor_y']!.text = '${settings.anchorYBasisPoints}';
    _fields['crop_left']!.text = '${settings.cropLeftBasisPoints}';
    _fields['crop_top']!.text = '${settings.cropTopBasisPoints}';
    _fields['crop_right']!.text = '${settings.cropRightBasisPoints}';
    _fields['crop_bottom']!.text = '${settings.cropBottomBasisPoints}';
    _fields['opacity']!.text = '${settings.opacityBasisPoints}';
    _fields['brightness']!.text = '${settings.brightnessAmountMilli}';
    _fields['contrast']!.text = '${settings.contrastAmountMilli}';
    _fields['saturation']!.text = '${settings.saturationAmountMilli}';
    _fields['blur']!.text = '${settings.gaussianBlurRadiusMilli}';
    _fields['transition_in_kind']!.text = '${settings.transitionIn.index}';
    _fields['transition_in_duration']!.text =
        settings.transitionInDuration?.canonical ?? '0/1';
    _fields['transition_out_kind']!.text = '${settings.transitionOut.index}';
    _fields['transition_out_duration']!.text =
        settings.transitionOutDuration?.canonical ?? '0/1';
    _modifiedEffects.clear();
    _updateTransitionIn = false;
    _updateTransitionOut = false;
  }

  void _writeAudioSettings(ProjectTimelineAudioSettings settings) {
    _audioFields['gain_db']!.text = (settings.gainMilliDecibels / 1000)
        .toStringAsFixed(3);
    _audioFields['pan_percent']!.text = (settings.panBasisPoints / 100)
        .toStringAsFixed(2);
    _audioFields['fade_in']!.text = settings.fadeIn.canonical;
    _audioFields['fade_out']!.text = settings.fadeOut.canonical;
  }

  ProjectTimelineVisualSettings? _readSettings() {
    final values = <String, int>{};
    for (final key in _visualFieldKeys) {
      final value = int.tryParse(_fields[key]!.text);
      if (value == null) return null;
      values[key] = value;
    }
    if (values['x']! < -100000 ||
        values['x']! > 100000 ||
        values['y']! < -100000 ||
        values['y']! > 100000 ||
        values['scale_x']! < 1 ||
        values['scale_x']! > 100000 ||
        values['scale_y']! < 1 ||
        values['scale_y']! > 100000 ||
        values['rotation']! < -360000 ||
        values['rotation']! > 360000 ||
        values['anchor_x']! > 10000 ||
        values['anchor_y']! > 10000 ||
        values['crop_left']! > 10000 ||
        values['crop_top']! > 10000 ||
        values['crop_right']! > 10000 ||
        values['crop_bottom']! > 10000 ||
        values['opacity']! > 10000 ||
        values['crop_left']! + values['crop_right']! >= 10000 ||
        values['crop_top']! + values['crop_bottom']! >= 10000 ||
        values['brightness']! < -1000 ||
        values['brightness']! > 1000 ||
        values['contrast']! < 0 ||
        values['contrast']! > 4000 ||
        values['saturation']! < 0 ||
        values['saturation']! > 4000 ||
        values['blur']! < 0 ||
        values['blur']! > 128000 ||
        values['transition_in_kind']! < 0 ||
        values['transition_in_kind']! > 3 ||
        values['transition_out_kind']! < 0 ||
        values['transition_out_kind']! > 3) {
      return null;
    }
    final transitionInDuration = ProjectRationalTime.tryParse(
      _fields['transition_in_duration']!.text,
    );
    final transitionOutDuration = ProjectRationalTime.tryParse(
      _fields['transition_out_duration']!.text,
    );
    if (transitionInDuration == null ||
        transitionOutDuration == null ||
        (values['transition_in_kind'] != 0 &&
            !transitionInDuration.isPositive) ||
        (values['transition_out_kind'] != 0 &&
            !transitionOutDuration.isPositive)) {
      return null;
    }
    return ProjectTimelineVisualSettings(
      xMilliCanvas: values['x']!,
      yMilliCanvas: values['y']!,
      scaleXMilli: values['scale_x']!,
      scaleYMilli: values['scale_y']!,
      rotationMilliDegrees: values['rotation']!,
      anchorXBasisPoints: values['anchor_x']!,
      anchorYBasisPoints: values['anchor_y']!,
      cropLeftBasisPoints: values['crop_left']!,
      cropTopBasisPoints: values['crop_top']!,
      cropRightBasisPoints: values['crop_right']!,
      cropBottomBasisPoints: values['crop_bottom']!,
      opacityBasisPoints: values['opacity']!,
      brightnessAmountMilli: values['brightness']!,
      contrastAmountMilli: values['contrast']!,
      saturationAmountMilli: values['saturation']!,
      gaussianBlurRadiusMilli: values['blur']!,
      transitionIn:
          ProjectTimelineTransition.values[values['transition_in_kind']!],
      transitionInDuration: transitionInDuration,
      transitionOut:
          ProjectTimelineTransition.values[values['transition_out_kind']!],
      transitionOutDuration: transitionOutDuration,
      modifiedEffects: Set.unmodifiable(_modifiedEffects),
      updateTransitionIn: _updateTransitionIn,
      updateTransitionOut: _updateTransitionOut,
    );
  }

  ProjectTimelineAudioSettings? _readAudioSettings() {
    final gain = double.tryParse(_audioFields['gain_db']!.text);
    final pan = double.tryParse(_audioFields['pan_percent']!.text);
    final fadeIn = ProjectRationalTime.tryParse(_audioFields['fade_in']!.text);
    final fadeOut = ProjectRationalTime.tryParse(
      _audioFields['fade_out']!.text,
    );
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

  Widget _numberField(String key, String label, {required bool signed}) {
    final expression = RegExp(signed ? r'^-?\d{0,7}$' : r'^\d{0,7}$');
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
      keyboardType: TextInputType.numberWithOptions(signed: signed),
      inputFormatters: [
        TextInputFormatter.withFunction((oldValue, newValue) {
          if (!expression.hasMatch(newValue.text)) return oldValue;
          if (newValue.text.isEmpty || newValue.text == '-') return newValue;
          final value = int.tryParse(newValue.text);
          if (value == null) return newValue;
          final minimum = switch (key) {
            'x' || 'y' || 'rotation' => -limit,
            'brightness' => -limit,
            'scale_x' || 'scale_y' => 1,
            _ => 0,
          };
          return value >= minimum && value <= limit ? newValue : oldValue;
        }),
      ],
      decoration: InputDecoration(
        labelText: label,
        isDense: true,
        counterText: '',
        contentPadding: const EdgeInsets.symmetric(
          horizontal: OrSpacing.x1,
          vertical: OrSpacing.x1,
        ),
      ),
      style: const TextStyle(fontSize: 11),
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
      helperText: 'Exact seconds, for example 1/2',
      contentPadding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x1,
        vertical: OrSpacing.x1,
      ),
    ),
    style: const TextStyle(fontSize: 11),
  );

  Widget _transitionField(String key, String label) =>
      DropdownButtonFormField<int>(
        key: ValueKey('inspector-visual-$key'),
        initialValue: int.tryParse(_fields[key]!.text) ?? 0,
        isExpanded: true,
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
        horizontal: OrSpacing.x1,
        vertical: OrSpacing.x1,
      ),
    ),
    style: const TextStyle(fontSize: 11),
  );

  Widget _audioTimeField(String key, String label) => TextField(
    key: ValueKey('inspector-audio-$key'),
    controller: _audioFields[key],
    enabled: !widget.busy && !_saving && !_loading && !widget.trackLocked,
    decoration: InputDecoration(
      labelText: label,
      isDense: true,
      counterText: '',
      helperText: 'Exact seconds, for example 1/2',
      contentPadding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x1,
        vertical: OrSpacing.x1,
      ),
    ),
    style: const TextStyle(fontSize: 11),
  );

  Widget _audioEditor() => ListView(
    key: const ValueKey('inspector-scroll'),
    padding: const EdgeInsets.all(OrSpacing.x2),
    children: [
      Text(
        'AUDIO',
        style: TextStyle(
          color: OrColors.textMuted,
          fontSize: 10,
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
      _audioTimeField('fade_in', 'Fade in'),
      const SizedBox(height: OrSpacing.x1),
      _audioTimeField('fade_out', 'Fade out'),
      if (_error != null) ...[
        const SizedBox(height: OrSpacing.x2),
        Text(
          _error!,
          style: TextStyle(
            color: Theme.of(context).colorScheme.error,
            fontSize: 11,
          ),
        ),
      ],
      const SizedBox(height: OrSpacing.x2),
      Row(
        children: [
          OutlinedButton(
            key: const ValueKey('inspector-audio-reset'),
            onPressed: widget.busy || _saving || widget.trackLocked
                ? null
                : () => _applyAudio(ProjectTimelineAudioSettings.identity),
            child: const Text('Reset'),
          ),
          const Spacer(),
          FilledButton(
            key: const ValueKey('inspector-audio-apply'),
            onPressed: widget.busy || _saving || widget.trackLocked || _loading
                ? null
                : () {
                    final settings = _readAudioSettings();
                    if (settings == null) {
                      setState(
                        () => _error = 'Enter valid gain, pan, and nonnegative rational fade times.',
                      );
                      return;
                    }
                    _applyAudio(settings);
                  },
            child: const Text('Apply'),
          ),
        ],
      ),
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
          const _PanelHeader(title: 'Inspector'),
          const Divider(height: 1),
          if (!canEdit)
            Expanded(
              child: OrEmptyState(
                title: hasSelection ? 'Clip settings' : 'No selection',
                message: !widget.isProjectWorkspace
                    ? 'Inspector controls are unavailable in this preview.'
                    : widget.trackLocked
                    ? 'Unlock the selected track to edit clip settings.'
                    : hasSelection
                    ? 'Select a video or audio clip to edit its settings.'
                    : 'Select a video or audio clip to edit its settings.',
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
                  Text(
                    'TRANSFORM',
                    style: TextStyle(
                      color: OrColors.textMuted,
                      fontSize: 10,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _pair(
                    'x',
                    'X (milli-canvas)',
                    true,
                    'y',
                    'Y (milli-canvas)',
                    true,
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _pair(
                    'scale_x',
                    'Scale X (milli)',
                    false,
                    'scale_y',
                    'Scale Y (milli)',
                    false,
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _numberField(
                    'rotation',
                    'Rotation (milli-deg)',
                    signed: true,
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _pair(
                    'anchor_x',
                    'Anchor X (bp)',
                    false,
                    'anchor_y',
                    'Anchor Y (bp)',
                    false,
                  ),
                  const SizedBox(height: OrSpacing.x3),
                  Text(
                    'CROP',
                    style: TextStyle(
                      color: OrColors.textMuted,
                      fontSize: 10,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _pair(
                    'crop_left',
                    'Left (bp)',
                    false,
                    'crop_right',
                    'Right (bp)',
                    false,
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _pair(
                    'crop_top',
                    'Top (bp)',
                    false,
                    'crop_bottom',
                    'Bottom (bp)',
                    false,
                  ),
                  const SizedBox(height: OrSpacing.x3),
                  Text(
                    'OPACITY',
                    style: TextStyle(
                      color: OrColors.textMuted,
                      fontSize: 10,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _numberField(
                    'opacity',
                    'Opacity (basis points)',
                    signed: false,
                  ),
                  const SizedBox(height: OrSpacing.x3),
                  Text(
                    'EFFECTS',
                    style: TextStyle(
                      color: OrColors.textMuted,
                      fontSize: 10,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _numberField(
                    'brightness',
                    'Brightness (milli)',
                    signed: true,
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _pair(
                    'contrast',
                    'Contrast (milli)',
                    false,
                    'saturation',
                    'Saturation (milli)',
                    false,
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _numberField(
                    'blur',
                    'Blur radius (milli-pixels)',
                    signed: false,
                  ),
                  const SizedBox(height: OrSpacing.x3),
                  Text(
                    'TRANSITIONS',
                    style: TextStyle(
                      color: OrColors.textMuted,
                      fontSize: 10,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _transitionField('transition_in_kind', 'Transition in'),
                  const SizedBox(height: OrSpacing.x1),
                  _visualTimeField(
                    'transition_in_duration',
                    'Transition in duration',
                  ),
                  const SizedBox(height: OrSpacing.x1),
                  _transitionField('transition_out_kind', 'Transition out'),
                  const SizedBox(height: OrSpacing.x1),
                  _visualTimeField(
                    'transition_out_duration',
                    'Transition out duration',
                  ),
                  if (_error != null) ...[
                    const SizedBox(height: OrSpacing.x2),
                    Text(
                      _error!,
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.error,
                        fontSize: 11,
                      ),
                    ),
                  ],
                  const SizedBox(height: OrSpacing.x2),
                  Row(
                    children: [
                      OutlinedButton(
                        key: const ValueKey('inspector-visual-reset'),
                        onPressed: widget.busy || _saving || widget.trackLocked
                            ? null
                            : () => _apply(
                                ProjectTimelineVisualSettings.identity,
                              ),
                        child: const Text('Reset'),
                      ),
                      const Spacer(),
                      FilledButton(
                        key: const ValueKey('inspector-visual-apply'),
                        onPressed:
                            widget.busy ||
                                _saving ||
                                widget.trackLocked ||
                                _loading
                            ? null
                            : () {
                                final settings = _readSettings();
                                if (settings == null) {
                                  setState(
                                    () => _error = 'Enter valid whole numbers within the supported ranges.',
                                  );
                                  return;
                                }
                                _apply(settings);
                              },
                        child: const Text('Apply'),
                      ),
                    ],
                  ),
                ],
              ),
            ),
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

class _TimelineToolbar extends StatelessWidget {
  const _TimelineToolbar({
    required this.compact,
    required this.isProjectWorkspace,
    required this.busy,
    required this.onUndo,
    required this.onAddVideoTrack,
    required this.onAddAudioTrack,
    required this.onAddTextTrack,
    required this.onAddCaptionTrack,
    required this.onAddTitle,
    required this.onAddCaption,
    required this.onAddMarker,
    required this.snapEnabled,
    required this.onSnapChanged,
  });

  final bool compact;
  final bool isProjectWorkspace;
  final bool busy;
  final VoidCallback? onUndo;
  final VoidCallback? onAddVideoTrack;
  final VoidCallback? onAddAudioTrack;
  final VoidCallback? onAddTextTrack;
  final VoidCallback? onAddCaptionTrack;
  final VoidCallback? onAddTitle;
  final VoidCallback? onAddCaption;
  final VoidCallback? onAddMarker;
  final bool snapEnabled;
  final ValueChanged<bool>? onSnapChanged;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      height: 40,
      padding: EdgeInsets.symmetric(
        horizontal: compact ? OrSpacing.x2 : OrSpacing.x3,
      ),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border.symmetric(
          horizontal: BorderSide(color: OrColors.border),
        ),
      ),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            Text(
              isProjectWorkspace
                  ? 'Timeline'
                  : compact
                  ? 'Timeline preview'
                  : 'Timeline tools unavailable',
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: 12,
                fontWeight: FontWeight.w500,
              ),
            ),
            const SizedBox(width: OrSpacing.x2),
            if (isProjectWorkspace) ...[
              TextButton.icon(
                key: const ValueKey('timeline-add-video-track'),
                onPressed: busy ? null : onAddVideoTrack,
                icon: const Icon(Icons.movie_outlined, size: 16),
                label: Text(compact ? 'Add Video' : 'Add Video Track'),
              ),
              TextButton.icon(
                key: const ValueKey('timeline-add-audio-track'),
                onPressed: busy ? null : onAddAudioTrack,
                icon: const Icon(Icons.graphic_eq_outlined, size: 16),
                label: Text(compact ? 'Add Audio' : 'Add Audio Track'),
              ),
              TextButton.icon(
                key: const ValueKey('timeline-add-marker'),
                onPressed: busy ? null : onAddMarker,
                icon: const Icon(Icons.bookmark_add_outlined, size: 16),
                label: const Text('Add Marker'),
              ),
              IconButton(
                key: const ValueKey('timeline-undo'),
                tooltip: 'Undo',
                onPressed: busy ? null : onUndo,
                icon: const Icon(Icons.undo_outlined, size: 17),
                visualDensity: VisualDensity.compact,
              ),
              Semantics(
                label: 'Snap to clip boundaries',
                toggled: snapEnabled,
                child: FilterChip(
                  key: const ValueKey('timeline-snap-toggle'),
                  label: const Text('Snap'),
                  selected: snapEnabled,
                  onSelected: busy || onSnapChanged == null
                      ? null
                      : onSnapChanged,
                  showCheckmark: false,
                  visualDensity: VisualDensity.compact,
                  selectedColor: OrColors.selection,
                  labelStyle: TextStyle(
                    color: snapEnabled
                        ? OrColors.primaryText
                        : OrColors.textSecondary,
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                  ),
                  side: BorderSide(
                    color: snapEnabled
                        ? OrColors.selection
                        : OrColors.borderStrong,
                  ),
                ),
              ),
              TextButton.icon(
                key: const ValueKey('timeline-add-text-track'),
                onPressed: busy ? null : onAddTextTrack,
                icon: const Icon(Icons.text_fields_outlined, size: 16),
                label: Text(compact ? 'Add Text' : 'Add Text Track'),
              ),
              TextButton.icon(
                key: const ValueKey('timeline-add-caption-track'),
                onPressed: busy ? null : onAddCaptionTrack,
                icon: const Icon(Icons.subtitles_outlined, size: 16),
                label: Text(compact ? 'Add Captions' : 'Add Caption Track'),
              ),
              TextButton.icon(
                key: const ValueKey('timeline-add-title'),
                onPressed: busy ? null : onAddTitle,
                icon: const Icon(Icons.title, size: 16),
                label: const Text('Add Title'),
              ),
              TextButton.icon(
                key: const ValueKey('timeline-add-manual-caption'),
                onPressed: busy ? null : onAddCaption,
                icon: const Icon(Icons.subtitles_outlined, size: 16),
                label: const Text('Add Caption'),
              ),
            ] else ...[
              _UnavailableTimelineAction(
                tooltip: 'Undo is unavailable in this Developer Preview',
                icon: Icons.undo_outlined,
              ),
            ],
            _UnavailableTimelineAction(
              tooltip: 'Timeline zoom is unavailable in this Developer Preview',
              icon: Icons.zoom_in_outlined,
            ),
          ],
        ),
      ),
    );
  }
}

class _UnavailableTimelineAction extends StatelessWidget {
  const _UnavailableTimelineAction({required this.tooltip, required this.icon});

  final String tooltip;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: IconButton(
        onPressed: null,
        icon: Icon(icon, size: 17),
        visualDensity: VisualDensity.compact,
      ),
    );
  }
}

class _TimelinePanel extends StatefulWidget {
  const _TimelinePanel({
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
    required this.onAddVideoTrack,
    required this.onAddAudioTrack,
    required this.onAddTextTrack,
    required this.onAddCaptionTrack,
    required this.onRemoveTrack,
    required this.onSetTrackState,
    required this.onLoadMore,
    required this.onLoadMoreMarkers,
    required this.onRefresh,
    required this.onMoveClip,
    required this.onDuplicateClip,
    required this.onResolveSnap,
    required this.onDeleteClip,
    required this.onTrimClip,
    required this.onSplitClip,
    required this.onRippleDeleteClip,
    required this.onUpdateTextClip,
    required this.onAddMarker,
    required this.onMoveMarker,
    required this.onRenameMarker,
    required this.onDeleteMarker,
    required this.snapEnabled,
    required this.onTimelineEditError,
    required this.mediaItems,
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
  )?
  onTrimClip;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    ProjectRationalTime,
  )?
  onSplitClip;
  final Future<void> Function(ProjectReadModel, ProjectTimelineClip)?
  onRippleDeleteClip;
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
  final Future<void> Function(
    ProjectReadModel,
    ProjectMediaItem,
    String,
    ProjectRationalTime,
    ProjectRationalTime,
    ProjectRationalTime,
  )?
  onAddMediaToTimeline;
  final void Function(
    String? trackId,
    String? clipId,
    ProjectTimelineTrackKind? kind,
  )?
  onClipSelectionChanged;

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
        color: Color(0xFF101011),
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Column(
        children: [
          _PanelHeader(
            title: 'Timeline',
            height: widget.compact ? 34 : 38,
            trailing: widget.isProjectWorkspace
                ? const OrBadge('Project timeline')
                : const OrBadge('Developer Preview'),
          ),
          if (!widget.isProjectWorkspace)
            Expanded(
              child: Column(
                children: [
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
                        fontSize: widget.compact ? 10 : 11,
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
      return SingleChildScrollView(
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Text(
                'No timeline tracks',
                style: TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: 13,
                  fontWeight: FontWeight.w500,
                ),
              ),
              const SizedBox(height: OrSpacing.x2),
              Wrap(
                spacing: OrSpacing.x2,
                runSpacing: OrSpacing.x2,
                alignment: WrapAlignment.center,
                children: [
                  OutlinedButton.icon(
                    key: const ValueKey('timeline-empty-add-video'),
                    onPressed: widget.busy ? null : widget.onAddVideoTrack,
                    icon: const Icon(Icons.movie_outlined, size: 16),
                    label: const Text('Add Video Track'),
                  ),
                  OutlinedButton.icon(
                    key: const ValueKey('timeline-empty-add-audio'),
                    onPressed: widget.busy ? null : widget.onAddAudioTrack,
                    icon: const Icon(Icons.graphic_eq_outlined, size: 16),
                    label: const Text('Add Audio Track'),
                  ),
                  OutlinedButton.icon(
                    key: const ValueKey('timeline-empty-add-text'),
                    onPressed: widget.busy ? null : widget.onAddTextTrack,
                    icon: const Icon(Icons.text_fields_outlined, size: 16),
                    label: const Text('Add Text Track'),
                  ),
                  OutlinedButton.icon(
                    key: const ValueKey('timeline-empty-add-caption'),
                    onPressed: widget.busy ? null : widget.onAddCaptionTrack,
                    icon: const Icon(Icons.subtitles_outlined, size: 16),
                    label: const Text('Add Caption Track'),
                  ),
                ],
              ),
            ],
          ),
        ),
      );
    }

    return Focus(
      focusNode: _timelineFocusNode,
      autofocus: true,
      onKeyEvent: _handleTimelineKey,
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
          final viewportWidth = math.max(
            1.0,
            constraints.maxWidth - _timelineTrackHeaderWidth,
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
          for (final track in snapshot.items) {
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
                busy: widget.busy,
                project: widget.project,
                onRemove: widget.onRemoveTrack,
                onSetState: widget.onSetTrackState,
              ),
            );
            if (page?.nextOffset != null) {
              trackRows.add(
                _TimelineLoadMoreHeader(
                  trackId: track.trackId,
                  loading: widget.loadingMoreTracks.contains(track.trackId),
                  busy: widget.busy,
                  onLoadMore: widget.onLoadMore,
                ),
              );
            }
          }

          return Column(
            children: [
              _timelineViewControls(),
              Expanded(
                child: Scrollbar(
                  child: SingleChildScrollView(
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        SizedBox(
                          width: _timelineTrackHeaderWidth,
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
                                        width: canvasWidth,
                                        pixelsPerSecond: scale,
                                        tickSeconds: tickSeconds,
                                        playhead: preview?.position,
                                        gesture: _gesture,
                                        snapGuide: _snapGuide,
                                        selectedClipId: _selectedClipId,
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
                                        onOpenClip: (clip) {
                                          _selectClip(track, clip);
                                          unawaited(() async {
                                            await _showTimelineClipActions(
                                              context: context,
                                              project: widget.project,
                                              tracks: snapshot.items,
                                              track: track,
                                              clip: clip,
                                              onMove: widget.onMoveClip,
                                              onDuplicate:
                                                  widget.onDuplicateClip,
                                              onDelete: widget.onDeleteClip,
                                              onTrim: widget.onTrimClip,
                                              onSplit: widget.onSplitClip,
                                              onRippleDelete:
                                                  widget.onRippleDeleteClip,
                                              onEditTextClip:
                                                  widget.onUpdateTextClip,
                                              busy: widget.busy,
                                            );
                                            if (mounted) {
                                              _timelineFocusNode.requestFocus();
                                            }
                                          }());
                                        },
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
    return Container(
      height: 34,
      padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x1),
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(bottom: BorderSide(color: OrColors.border)),
      ),
      child: Row(
        children: [
          if (selectedClip != null) ...[
            Text(
              'Selected clip ${selectedClip.clipId.substring(0, math.min(8, selectedClip.clipId.length))}',
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: 10,
              ),
            ),
            IconButton(
              key: const ValueKey('timeline-selection-duplicate'),
              tooltip: 'Duplicate clip (Ctrl+D)',
              visualDensity: VisualDensity.compact,
              onPressed: canDuplicate ? _duplicateSelectedClip : null,
              icon: const Icon(Icons.content_copy_outlined, size: 15),
            ),
            IconButton(
              key: const ValueKey('timeline-selection-clear'),
              tooltip: 'Clear selection',
              visualDensity: VisualDensity.compact,
              onPressed: _clearSelection,
              icon: const Icon(Icons.close, size: 15),
            ),
          ],
          const Spacer(),
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
              _fitTimelineToView ? 'Fit' : '${(_timelineZoom * 100).round()}%',
              textAlign: TextAlign.center,
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: 10,
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
        ],
      ),
    );
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

  KeyEventResult _handleTimelineKey(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent) return KeyEventResult.ignored;
    if (event.logicalKey == LogicalKeyboardKey.escape &&
        _selectedClipId != null) {
      _clearSelection();
      return KeyEventResult.handled;
    }
    final clip = _selectedClip;
    final track = _selectedTrack;
    if (clip == null || track == null || widget.busy) {
      return KeyEventResult.ignored;
    }
    final hasShortcutModifier =
        HardwareKeyboard.instance.isControlPressed ||
        HardwareKeyboard.instance.isMetaPressed;
    if (hasShortcutModifier &&
        event.logicalKey == LogicalKeyboardKey.keyD &&
        !track.state.locked &&
        widget.onDuplicateClip != null) {
      _duplicateSelectedClip();
      return KeyEventResult.handled;
    }
    if ((event.logicalKey == LogicalKeyboardKey.delete ||
            event.logicalKey == LogicalKeyboardKey.backspace) &&
        !track.state.locked &&
        widget.onDeleteClip != null &&
        widget.project != null &&
        node.context != null) {
      unawaited(
        _confirmDeleteTimelineClip(
          context: node.context!,
          project: widget.project!,
          clip: clip,
          onDelete: widget.onDeleteClip!,
        ),
      );
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
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

const double _timelineTrackHeaderWidth = 272;
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
      fontSize: 10,
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
                  fontSize: 9,
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
              textStyle: const TextStyle(fontSize: 9),
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
                  fontSize: 9,
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
    required this.busy,
    required this.project,
    required this.onRemove,
    required this.onSetState,
  });

  final ProjectTimelineTrack track;
  final String label;
  final String countLabel;
  final bool busy;
  final ProjectReadModel? project;
  final Future<void> Function(ProjectReadModel, ProjectTimelineTrack)? onRemove;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineTrack,
    ProjectTimelineTrackState,
  )?
  onSetState;

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
          '$kind track $label, ${track.clipCount} clips, ${enabled ? 'enabled' : 'disabled'}, ${track.state.locked ? 'locked' : 'unlocked'}, ${track.state.solo ? 'solo' : 'not solo'}',
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
                    style: const TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Text(
                    countLabel,
                    style: const TextStyle(
                      fontSize: 9,
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
    required this.loading,
    required this.busy,
    required this.onLoadMore,
  });

  final String trackId;
  final bool loading;
  final bool busy;
  final ValueChanged<String>? onLoadMore;

  @override
  Widget build(BuildContext context) => Container(
    width: _timelineTrackHeaderWidth,
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
        textStyle: const TextStyle(fontSize: 10),
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

class _TimelinePointerSurface extends StatefulWidget {
  const _TimelinePointerSurface({
    super.key,
    required this.child,
    required this.behavior,
    this.onTap,
    this.onPanStart,
    this.onPanUpdate,
    this.onPanEnd,
    this.onPanCancel,
  });

  final Widget child;
  final HitTestBehavior behavior;
  final VoidCallback? onTap;
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
    required this.width,
    required this.pixelsPerSecond,
    required this.tickSeconds,
    required this.playhead,
    required this.gesture,
    required this.snapGuide,
    required this.selectedClipId,
    required this.onDropMedia,
    required this.onOpenClip,
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
  final double width;
  final double pixelsPerSecond;
  final double tickSeconds;
  final ProjectRationalTime? playhead;
  final _TimelinePointerGesture? gesture;
  final _TimelineSnapGuide? snapGuide;
  final String? selectedClipId;
  final void Function(ProjectMediaItem, Offset)? onDropMedia;
  final ValueChanged<ProjectTimelineClip> onOpenClip;
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
    final body = _TimelinePointerSurface(
      behavior: HitTestBehavior.opaque,
      onTap: () => onOpenClip(clip),
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
    final visual = Container(
      padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x2),
      decoration: BoxDecoration(
        border: Border.all(
          color: clip.clipId == selectedClipId
              ? OrColors.selection
              : OrColors.borderStrong,
          width: clip.clipId == selectedClipId ? 1.5 : 1,
        ),
        borderRadius: BorderRadius.circular(OrRadii.small),
      ),
      alignment: Alignment.centerLeft,
      child: Text(
        title,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: const TextStyle(
          color: OrColors.text,
          fontSize: 10,
          fontWeight: FontWeight.w500,
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
          selected: clip.clipId == selectedClipId,
          label:
              '$title, starts ${clip.timelineStart.canonical}, duration $durationLabel',
          hint: 'Drag to move. Activate for clip actions.',
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
                  if (clip.contentKind ==
                      ProjectTimelineClipContentKind.media) ...[
                    _trimHandle(
                      clip,
                      start: true,
                      label: 'Trim start of $title',
                    ),
                    _trimHandle(
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
    ProjectTimelineClip clip, {
    required bool start,
    required String label,
  }) {
    final onStart = start ? onTrimStart : onTrimEnd;
    return Positioned(
      left: start ? -5 : null,
      right: start ? null : -5,
      top: -1,
      bottom: -1,
      width: 11,
      child: Semantics(
        button: true,
        label: label,
        hint: 'Drag to adjust the timeline edge.',
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
            alignment: start ? Alignment.centerLeft : Alignment.centerRight,
            child: Container(
              width: 3,
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
  required Future<void> Function(
    ProjectReadModel,
    ProjectMediaItem,
    String,
    ProjectRationalTime,
    ProjectRationalTime,
    ProjectRationalTime,
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
  final timelineStartController = TextEditingController(text: '0/1');
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
) => switch (kind) {
  ProjectTimelineTrackKind.video =>
    media.firstVideoDuration ?? media.containerDuration,
  ProjectTimelineTrackKind.audio =>
    media.firstAudioDuration ?? media.containerDuration,
  ProjectTimelineTrackKind.text || ProjectTimelineTrackKind.caption => null,
};

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
  )?
  onTrim,
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
        TextButton(
          key: ValueKey('timeline-delete-${clip.clipId}'),
          onPressed:
              busy || track.state.locked || project == null || onDelete == null
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
                  style: TextStyle(color: OrColors.textSecondary, fontSize: 11),
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
  )?
  onTrim,
}) async {
  if (project == null || onTrim == null) return;
  var selectedEdge = ProjectTimelineTrimEdge.start;
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
                        unawaited(onTrim(project, clip, selectedEdge, target));
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
                    fontSize: 10,
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
  const _MobileToolDock({required this.selected, required this.onSelected});

  final String selected;
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
            for (final tool in _editorTools)
              Tooltip(
                message:
                    '${tool.label} — unavailable in this Developer Preview',
                child: InkWell(
                  key: ValueKey('mobile-editor-tool-${_toolKey(tool.label)}'),
                  onTap: () => onSelected(tool.label),
                  child: SizedBox(
                    width: 62,
                    height: 56,
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
                          style: const TextStyle(
                            color: OrColors.textMuted,
                            fontSize: 9,
                          ),
                        ),
                      ],
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
                      fontSize: 16,
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
              style: TextStyle(color: OrColors.textSecondary, fontSize: 13),
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
                  fontSize: 12,
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

class _ResizeDivider extends StatelessWidget {
  const _ResizeDivider({required this.horizontal});

  final bool horizontal;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: horizontal ? 7 : double.infinity,
      alignment: Alignment.center,
      color: OrColors.background,
      child: Container(
        width: horizontal ? 36 : 1,
        height: horizontal ? 2 : double.infinity,
        decoration: BoxDecoration(
          color: OrColors.borderStrong,
          borderRadius: BorderRadius.circular(2),
        ),
      ),
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
