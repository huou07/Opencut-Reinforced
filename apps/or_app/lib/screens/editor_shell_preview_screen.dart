import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../project/project_gateway.dart';
import '../widgets/or_widgets.dart';

class EditorShellPreviewScreen extends StatefulWidget {
  const EditorShellPreviewScreen({
    super.key,
    this.isProjectWorkspace = false,
    this.project,
    this.notice,
    this.busy = false,
    this.mediaPage,
    this.mediaPreviews = const {},
    this.mediaLoading = false,
    this.mediaLoadingMore = false,
    this.mediaLoadError,
    this.timelineTracks,
    this.timelineClipPages = const {},
    this.timelineLoadingMoreTracks = const {},
    this.timelineLoading = false,
    this.timelineLoadError,
    this.onImportMedia,
    this.onLoadMoreMedia,
    this.onRefreshMedia,
    this.onRemoveMedia,
    this.onAddVideoTrack,
    this.onAddAudioTrack,
    this.onRemoveTimelineTrack,
    this.onLoadMoreTimelineClips,
    this.onRefreshTimeline,
    this.onAddMediaToTimeline,
    this.onMoveTimelineClip,
    this.onDeleteTimelineClip,
    this.onTrimTimelineClip,
    this.onSplitTimelineClip,
    this.onRippleDeleteTimelineClip,
    this.onSave,
    this.onRename,
    this.onUndo,
    this.onRedo,
    this.onClose,
    this.onDiscardRecovery,
  });

  final bool isProjectWorkspace;
  final ProjectReadModel? project;
  final String? notice;
  final bool busy;
  final ProjectMediaPage? mediaPage;
  final Map<String, ProjectMediaPreview> mediaPreviews;
  final bool mediaLoading;
  final bool mediaLoadingMore;
  final String? mediaLoadError;
  final ProjectTimelineTracks? timelineTracks;
  final Map<String, ProjectTimelineClipPage> timelineClipPages;
  final Set<String> timelineLoadingMoreTracks;
  final bool timelineLoading;
  final String? timelineLoadError;
  final VoidCallback? onImportMedia;
  final VoidCallback? onLoadMoreMedia;
  final VoidCallback? onRefreshMedia;
  final ValueChanged<ProjectMediaItem>? onRemoveMedia;
  final VoidCallback? onAddVideoTrack;
  final VoidCallback? onAddAudioTrack;
  final Future<void> Function(ProjectReadModel, ProjectTimelineTrack)?
  onRemoveTimelineTrack;
  final ValueChanged<String>? onLoadMoreTimelineClips;
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
    ProjectReadModel project,
    ProjectTimelineClip clip,
    String trackId,
    ProjectRationalTime timelineStart,
  )?
  onMoveTimelineClip;
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
              const Expanded(child: _ViewerPanel(compact: false)),
              const VerticalDivider(width: 1),
              SizedBox(width: wide ? 236 : 188, child: const _InspectorPanel()),
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
        ),
        Expanded(
          flex: 2,
          child: _TimelinePanel(
            compact: false,
            isProjectWorkspace: widget.isProjectWorkspace,
            project: widget.project,
            tracks: widget.timelineTracks,
            clipPages: widget.timelineClipPages,
            loadingMoreTracks: widget.timelineLoadingMoreTracks,
            loading: widget.timelineLoading,
            error: widget.timelineLoadError,
            busy: widget.busy,
            onAddVideoTrack: widget.onAddVideoTrack,
            onAddAudioTrack: widget.onAddAudioTrack,
            onRemoveTrack: widget.onRemoveTimelineTrack,
            onLoadMore: widget.onLoadMoreTimelineClips,
            onRefresh: widget.onRefreshTimeline,
            onMoveClip: widget.onMoveTimelineClip,
            onDeleteClip: widget.onDeleteTimelineClip,
            onTrimClip: widget.onTrimTimelineClip,
            onSplitClip: widget.onSplitTimelineClip,
            onRippleDeleteClip: widget.onRippleDeleteTimelineClip,
            mediaItems: widget.mediaPage?.items ?? const [],
          ),
        ),
      ],
    );
  }

  Widget _compactLayout() {
    return Column(
      children: [
        const Expanded(flex: 4, child: _ViewerPanel(compact: true)),
        _TimelineToolbar(
          compact: true,
          isProjectWorkspace: widget.isProjectWorkspace,
          busy: widget.busy,
          onUndo: widget.onUndo,
          onAddVideoTrack: widget.onAddVideoTrack,
          onAddAudioTrack: widget.onAddAudioTrack,
        ),
        Expanded(
          flex: 2,
          child: _TimelinePanel(
            compact: true,
            isProjectWorkspace: widget.isProjectWorkspace,
            project: widget.project,
            tracks: widget.timelineTracks,
            clipPages: widget.timelineClipPages,
            loadingMoreTracks: widget.timelineLoadingMoreTracks,
            loading: widget.timelineLoading,
            error: widget.timelineLoadError,
            busy: widget.busy,
            onAddVideoTrack: widget.onAddVideoTrack,
            onAddAudioTrack: widget.onAddAudioTrack,
            onRemoveTrack: widget.onRemoveTimelineTrack,
            onLoadMore: widget.onLoadMoreTimelineClips,
            onRefresh: widget.onRefreshTimeline,
            onMoveClip: widget.onMoveTimelineClip,
            onDeleteClip: widget.onDeleteTimelineClip,
            onTrimClip: widget.onTrimTimelineClip,
            onSplitClip: widget.onSplitTimelineClip,
            onRippleDeleteClip: widget.onRippleDeleteTimelineClip,
            mediaItems: widget.mediaPage?.items ?? const [],
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
    final details = <String>[
      if (item.formatNames.isNotEmpty) item.formatNames.join(', '),
      if (item.duration != null) item.duration!,
      if (item.videoDetails != null) item.videoDetails!,
      if (item.audioDetails != null) 'Audio · ${item.audioDetails}',
    ];
    return Tooltip(
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

class _ViewerPanel extends StatelessWidget {
  const _ViewerPanel({required this.compact});

  final bool compact;

  @override
  Widget build(BuildContext context) {
    return ColoredBox(
      color: OrColors.background,
      child: Column(
        children: [
          _PanelHeader(
            title: 'Viewer',
            height: compact ? 34 : 38,
            trailing: const OrBadge('Preview surface'),
          ),
          Expanded(
            child: Container(
              key: const ValueKey('editor-viewer-surface'),
              alignment: Alignment.center,
              color: OrColors.background,
              child: ConstrainedBox(
                constraints: BoxConstraints(
                  maxWidth: compact ? 520 : 660,
                  maxHeight: compact ? 360 : 420,
                ),
                child: AspectRatio(
                  aspectRatio: 16 / 9,
                  child: Container(
                    decoration: BoxDecoration(
                      color: const Color(0xFF0C0C0D),
                      border: Border.all(color: OrColors.borderStrong),
                      borderRadius: BorderRadius.circular(OrRadii.small),
                    ),
                    alignment: Alignment.center,
                    child: const Column(
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
                  ),
                ),
              ),
            ),
          ),
          _PreviewTransport(compact: compact),
        ],
      ),
    );
  }
}

class _PreviewTransport extends StatelessWidget {
  const _PreviewTransport({required this.compact});

  final bool compact;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: compact ? 42 : 44,
      decoration: const BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border(top: BorderSide(color: OrColors.border)),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Tooltip(
            message: 'Playback is unavailable in this Developer Preview',
            child: IconButton(
              key: const ValueKey('preview-previous-frame'),
              onPressed: null,
              icon: const Icon(Icons.skip_previous_outlined, size: 18),
            ),
          ),
          Tooltip(
            message: 'Playback is unavailable in this Developer Preview',
            child: IconButton(
              key: const ValueKey('preview-play'),
              onPressed: null,
              icon: const Icon(Icons.play_arrow_outlined, size: 20),
            ),
          ),
          Tooltip(
            message: 'Playback is unavailable in this Developer Preview',
            child: IconButton(
              key: const ValueKey('preview-next-frame'),
              onPressed: null,
              icon: const Icon(Icons.skip_next_outlined, size: 18),
            ),
          ),
          if (!compact) ...[
            const SizedBox(width: OrSpacing.x2),
            const Text(
              'Playback unavailable',
              style: TextStyle(color: OrColors.textMuted, fontSize: 11),
            ),
          ],
        ],
      ),
    );
  }
}

class _InspectorPanel extends StatelessWidget {
  const _InspectorPanel();

  @override
  Widget build(BuildContext context) {
    return const ColoredBox(
      color: OrColors.backgroundRaised,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _PanelHeader(title: 'Inspector'),
          Divider(height: 1),
          Expanded(
            child: OrEmptyState(
              title: 'No selection',
              message: 'Inspector controls are unavailable in this preview.',
              icon: Icons.tune_outlined,
            ),
          ),
        ],
      ),
    );
  }
}

class _TimelineToolbar extends StatelessWidget {
  const _TimelineToolbar({
    required this.compact,
    required this.isProjectWorkspace,
    required this.busy,
    required this.onUndo,
    required this.onAddVideoTrack,
    required this.onAddAudioTrack,
  });

  final bool compact;
  final bool isProjectWorkspace;
  final bool busy;
  final VoidCallback? onUndo;
  final VoidCallback? onAddVideoTrack;
  final VoidCallback? onAddAudioTrack;

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
              IconButton(
                key: const ValueKey('timeline-undo'),
                tooltip: 'Undo',
                onPressed: busy ? null : onUndo,
                icon: const Icon(Icons.undo_outlined, size: 17),
                visualDensity: VisualDensity.compact,
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

class _TimelinePanel extends StatelessWidget {
  const _TimelinePanel({
    required this.compact,
    required this.isProjectWorkspace,
    required this.project,
    required this.tracks,
    required this.clipPages,
    required this.loadingMoreTracks,
    required this.loading,
    required this.error,
    required this.busy,
    required this.onAddVideoTrack,
    required this.onAddAudioTrack,
    required this.onRemoveTrack,
    required this.onLoadMore,
    required this.onRefresh,
    required this.onMoveClip,
    required this.onDeleteClip,
    required this.onTrimClip,
    required this.onSplitClip,
    required this.onRippleDeleteClip,
    required this.mediaItems,
  });

  final bool compact;
  final bool isProjectWorkspace;
  final ProjectReadModel? project;
  final ProjectTimelineTracks? tracks;
  final Map<String, ProjectTimelineClipPage> clipPages;
  final Set<String> loadingMoreTracks;
  final bool loading;
  final String? error;
  final bool busy;
  final VoidCallback? onAddVideoTrack;
  final VoidCallback? onAddAudioTrack;
  final Future<void> Function(ProjectReadModel, ProjectTimelineTrack)?
  onRemoveTrack;
  final ValueChanged<String>? onLoadMore;
  final VoidCallback? onRefresh;
  final Future<void> Function(
    ProjectReadModel,
    ProjectTimelineClip,
    String,
    ProjectRationalTime,
  )?
  onMoveClip;
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
  final List<ProjectMediaItem> mediaItems;

  @override
  Widget build(BuildContext context) {
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
            height: compact ? 34 : 38,
            trailing: isProjectWorkspace
                ? const OrBadge('Project timeline')
                : const OrBadge('Developer Preview'),
          ),
          if (!isProjectWorkspace)
            Expanded(
              child: Column(
                children: [
                  _TimelineRuler(compact: compact),
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
                        fontSize: compact ? 10 : 11,
                      ),
                    ),
                  ),
                ],
              ),
            )
          else
            Expanded(child: _buildProjectTimeline(context)),
        ],
      ),
    );
  }

  Widget _buildProjectTimeline(BuildContext context) {
    final snapshot = tracks;
    if (loading && snapshot == null) {
      return const Center(child: CircularProgressIndicator(strokeWidth: 2));
    }
    if (snapshot == null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              error ?? 'The timeline is unavailable.',
              textAlign: TextAlign.center,
              style: const TextStyle(color: OrColors.textSecondary),
            ),
            TextButton.icon(
              onPressed: onRefresh,
              icon: const Icon(Icons.refresh, size: 16),
              label: const Text('Refresh timeline'),
            ),
          ],
        ),
      );
    }
    if (snapshot.items.isEmpty) {
      return Center(
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
                  onPressed: busy ? null : onAddVideoTrack,
                  icon: const Icon(Icons.movie_outlined, size: 16),
                  label: const Text('Add Video Track'),
                ),
                OutlinedButton.icon(
                  key: const ValueKey('timeline-empty-add-audio'),
                  onPressed: busy ? null : onAddAudioTrack,
                  icon: const Icon(Icons.graphic_eq_outlined, size: 16),
                  label: const Text('Add Audio Track'),
                ),
              ],
            ),
          ],
        ),
      );
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        final labels = _timelineTrackLabels(snapshot.items);
        var endSeconds = 0.0;
        for (final page in clipPages.values) {
          for (final clip in page.items) {
            final start = clip.timelineStart.secondsForDisplay;
            final duration = clip.sourceDuration.secondsForDisplay;
            final end = start + duration;
            if (start.isFinite && duration.isFinite && end.isFinite) {
              endSeconds = math.max(endSeconds, end);
            }
          }
        }
        final displayDuration = math.max(20.0, endSeconds);
        const maximumWidth = 100000.0;
        const preferredScale = 64.0;
        final scale = math.min(preferredScale, maximumWidth / displayDuration);
        final canvasWidth = math.max(
          constraints.maxWidth - _timelineTrackHeaderWidth,
          math.min(maximumWidth, displayDuration * scale),
        );
        final tickSeconds = _timelineTickSeconds(displayDuration, canvasWidth);
        final trackRows = <Widget>[];
        for (final track in snapshot.items) {
          final page = clipPages[track.trackId];
          final loadedCount = page?.items.length ?? 0;
          final countLabel = loadedCount < track.clipCount
              ? '$loadedCount / ${track.clipCount}'
              : '${track.clipCount}';
          trackRows.add(
            _TimelineTrackHeader(
              track: track,
              label: labels[track.trackId]!,
              countLabel: countLabel,
              busy: busy,
              project: project,
              onRemove: onRemoveTrack,
            ),
          );
          if (page?.nextOffset != null) {
            trackRows.add(
              _TimelineLoadMoreHeader(
                trackId: track.trackId,
                loading: loadingMoreTracks.contains(track.trackId),
                busy: busy,
                onLoadMore: onLoadMore,
              ),
            );
          }
        }

        return Scrollbar(
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
                  child: SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: SizedBox(
                      width: canvasWidth,
                      child: Column(
                        children: [
                          SizedBox(
                            height: _timelineRulerHeight,
                            child: CustomPaint(
                              painter: _TimelineRulerPainter(
                                pixelsPerSecond: scale,
                                tickSeconds: tickSeconds,
                              ),
                              child: const SizedBox.expand(),
                            ),
                          ),
                          for (final track in snapshot.items) ...[
                            _TimelineTrackLane(
                              track: track,
                              clips:
                                  clipPages[track.trackId]?.items ?? const [],
                              mediaItems: mediaItems,
                              width: canvasWidth,
                              pixelsPerSecond: scale,
                              tickSeconds: tickSeconds,
                              onOpenClip: (clip) => unawaited(
                                _showTimelineClipActions(
                                  context: context,
                                  project: project,
                                  tracks: snapshot.items,
                                  track: track,
                                  clip: clip,
                                  onMove: onMoveClip,
                                  onDelete: onDeleteClip,
                                  onTrim: onTrimClip,
                                  onSplit: onSplitClip,
                                  onRippleDelete: onRippleDeleteClip,
                                  busy: busy,
                                ),
                              ),
                            ),
                            if (clipPages[track.trackId]?.nextOffset != null)
                              _TimelineLoadMoreLane(width: canvasWidth),
                          ],
                        ],
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

const double _timelineTrackHeaderWidth = 132;
const double _timelineRulerHeight = 27;
const double _timelineLaneHeight = 58;

Map<String, String> _timelineTrackLabels(List<ProjectTimelineTrack> tracks) {
  var video = 0;
  var audio = 0;
  return {
    for (final track in tracks)
      track.trackId: switch (track.kind) {
        ProjectTimelineTrackKind.video => 'V${++video}',
        ProjectTimelineTrackKind.audio => 'A${++audio}',
      },
  };
}

double _timelineTickSeconds(double duration, double width) {
  final marks = (width / 100).ceil().clamp(1, 1000);
  final needed = duration / marks;
  return math.max(5, (needed / 5).ceil() * 5).toDouble();
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
  });

  final ProjectTimelineTrack track;
  final String label;
  final String countLabel;
  final bool busy;
  final ProjectReadModel? project;
  final Future<void> Function(ProjectReadModel, ProjectTimelineTrack)? onRemove;

  @override
  Widget build(BuildContext context) {
    final kind = track.kind == ProjectTimelineTrackKind.video
        ? 'Video'
        : 'Audio';
    final icon = track.kind == ProjectTimelineTrackKind.video
        ? Icons.movie_outlined
        : Icons.graphic_eq_outlined;
    final removable = track.clipCount == 0 && !busy && project != null;
    return Semantics(
      container: true,
      label: '$kind track $label, ${track.clipCount} clips',
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
                visualDensity: VisualDensity.compact,
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

class _TimelineTrackLane extends StatelessWidget {
  const _TimelineTrackLane({
    required this.track,
    required this.clips,
    required this.mediaItems,
    required this.width,
    required this.pixelsPerSecond,
    required this.tickSeconds,
    required this.onOpenClip,
  });

  final ProjectTimelineTrack track;
  final List<ProjectTimelineClip> clips;
  final List<ProjectMediaItem> mediaItems;
  final double width;
  final double pixelsPerSecond;
  final double tickSeconds;
  final ValueChanged<ProjectTimelineClip> onOpenClip;

  @override
  Widget build(BuildContext context) => Container(
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
        for (final clip in clips) _positionedClip(context, clip),
      ],
    ),
  );

  Widget _positionedClip(BuildContext context, ProjectTimelineClip clip) {
    final start = clip.timelineStart.secondsForDisplay;
    final duration = clip.sourceDuration.secondsForDisplay;
    final rawLeft = start * pixelsPerSecond;
    final rawWidth = duration * pixelsPerSecond;
    if (!rawLeft.isFinite || !rawWidth.isFinite || rawWidth <= 0) {
      return const SizedBox.shrink();
    }
    final left = rawLeft.clamp(0.0, width).toDouble();
    final clipWidth = math.min(math.max(2.0, rawWidth), width - left);
    if (clipWidth <= 0) return const SizedBox.shrink();
    final media = _findMedia(mediaItems, clip.mediaId);
    final title = media == null
        ? 'Media ${clip.mediaId.substring(0, math.min(8, clip.mediaId.length))}'
        : _mediaDisplayName(media.sourceUri);
    final durationLabel = clip.sourceDuration.canonical;
    final trackKind = track.kind == ProjectTimelineTrackKind.video
        ? ProjectTimelineTrackKind.video
        : ProjectTimelineTrackKind.audio;
    final background = trackKind == ProjectTimelineTrackKind.video
        ? OrColors.surface
        : OrColors.backgroundRaised;
    return Positioned(
      left: left,
      top: 5,
      width: clipWidth,
      height: _timelineLaneHeight - 10,
      child: Semantics(
        button: true,
        label:
            '$title, starts ${clip.timelineStart.canonical}, duration $durationLabel',
        child: Tooltip(
          key: ValueKey('timeline-clip-tooltip-${clip.clipId}'),
          message:
              'Clip ID: ${clip.clipId}\nMedia ID: ${clip.mediaId}\nTimeline start: ${clip.timelineStart.canonical}\nSource range: ${clip.sourceStart.canonical} + ${clip.sourceDuration.canonical}',
          child: Material(
            color: background,
            borderRadius: BorderRadius.circular(OrRadii.small),
            child: InkWell(
              key: ValueKey('timeline-clip-${clip.clipId}'),
              onTap: () => onOpenClip(clip),
              borderRadius: BorderRadius.circular(OrRadii.small),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x2),
                decoration: BoxDecoration(
                  border: Border.all(color: OrColors.borderStrong),
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
              ),
            ),
          ),
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
      .where((track) => compatibleKinds.contains(track.kind))
      .toList(growable: false);
  if (compatible.isEmpty) {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Add to Timeline'),
        content: const Text('Add a compatible Video or Audio track first.'),
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
) =>
    (kind == ProjectTimelineTrackKind.video
        ? media.firstVideoDuration
        : media.firstAudioDuration) ??
    media.containerDuration;

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
  required bool busy,
}) async {
  final media = clip.mediaId;
  final canMove =
      project != null &&
      onMove != null &&
      tracks.any((candidate) => candidate.kind == track.kind);
  final canTrim = project != null && onTrim != null;
  final canSplit = project != null && onSplit != null;
  final canRippleDelete = project != null && onRippleDelete != null;
  await showDialog<void>(
    context: context,
    builder: (dialogContext) => AlertDialog(
      title: const Text('Clip'),
      content: Text(
        'Clip ID: ${clip.clipId}\nMedia ID: $media\nTimeline start: ${clip.timelineStart.canonical}\nSource range: ${clip.sourceStart.canonical} + ${clip.sourceDuration.canonical}',
      ),
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
          onPressed: busy || project == null || onDelete == null
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
      .where((candidate) => candidate.kind == sourceTrack.kind)
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
