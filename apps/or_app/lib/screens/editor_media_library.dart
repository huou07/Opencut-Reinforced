import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../project/project_gateway.dart';
import 'editor_media_preview.dart';
import 'editor_panel_header.dart';

/// Number of `EditorMediaLibraryPanel` builds, for rebuild-cost tests.
int mediaLibraryPanelBuilds = 0;

class EditorMediaLibraryPanel extends StatelessWidget {
  const EditorMediaLibraryPanel({
    super.key,
    required this.page,
    required this.previews,
    this.previewUpdates,
    required this.loading,
    required this.loadingMore,
    required this.error,
    required this.onImport,
    required this.onImportFolder,
    required this.onLoadMore,
    required this.onRefresh,
    this.onRetryPreview,
    required this.onRemove,
    required this.onRelink,
    required this.onGenerateCaptions,
    this.onManageSpeechModel,
    required this.onCleanUpAudio,
    this.onInsertMedia,
    this.onClose,
    this.showTitle = true,
    this.searchController,
  });

  final ProjectMediaPage? page;
  final Map<String, ProjectMediaPreview> previews;

  /// Live previews painted without waiting for a shell rebuild.
  final ValueListenable<Map<String, ProjectMediaPreview>>? previewUpdates;
  final bool loading;
  final bool loadingMore;
  final String? error;
  final VoidCallback? onImport;
  final VoidCallback? onImportFolder;
  final VoidCallback? onLoadMore;
  final VoidCallback? onRefresh;

  /// Re-requests one failed preview instead of waiting for a full refresh.
  final ValueChanged<ProjectMediaItem>? onRetryPreview;
  final ValueChanged<ProjectMediaItem>? onRemove;
  final ValueChanged<ProjectMediaItem>? onRelink;
  final Future<void> Function(ProjectMediaItem media)? onGenerateCaptions;
  final VoidCallback? onManageSpeechModel;
  final Future<void> Function(ProjectMediaItem media)? onCleanUpAudio;
  final Future<void> Function(ProjectMediaItem media)? onInsertMedia;
  final VoidCallback? onClose;

  /// Hidden when the tab strip already names the category.
  final bool showTitle;

  /// A library of a hundred clips is unusable without a filter. The field owns
  /// a controller so a keystroke re-filters without rebuilding the editor.
  final TextEditingController? searchController;

  @override
  Widget build(BuildContext context) {
    final controller = searchController;
    final updates = previewUpdates;
    if (controller == null) {
      if (updates == null) return _panel(context, '');
      return ValueListenableBuilder<Map<String, ProjectMediaPreview>>(
        valueListenable: updates,
        builder: (context, live, _) => _panel(context, '', liveUpdates: live),
      );
    }
    void clearSearch() => controller
      ..text = ''
      ..selection = const TextSelection.collapsed(offset: 0);
    if (updates == null) {
      return ValueListenableBuilder<TextEditingValue>(
        valueListenable: controller,
        builder: (context, value, _) =>
            _panel(context, value.text, onClearSearch: clearSearch),
      );
    }
    return ValueListenableBuilder<TextEditingValue>(
      valueListenable: controller,
      builder: (context, value, _) =>
          ValueListenableBuilder<Map<String, ProjectMediaPreview>>(
            valueListenable: updates,
            builder: (context, live, _) =>
                _panel(context, value.text, liveUpdates: live, onClearSearch: clearSearch),
          ),
    );
  }

  Widget _panel(
    BuildContext context,
    String searchQuery, {
    Map<String, ProjectMediaPreview> liveUpdates = const {},
    VoidCallback? onClearSearch,
  }) {
    assert(() {
      mediaLibraryPanelBuilds += 1;
      return true;
    }());
    final library = page?.items ?? const <ProjectMediaItem>[];
    final needle = searchQuery.trim().toLowerCase();
    final items = needle.isEmpty
        ? library
        : library
              .where(
                (item) =>
                    mediaDisplayName(item.sourceUri)
                        .toLowerCase()
                        .contains(needle),
              )
              .toList(growable: false);
    final filtering = needle.isNotEmpty;
    return ColoredBox(
      key: const ValueKey('project-media-panel'),
      color: OrColors.backgroundRaised,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (showTitle) ...[
            EditorPanelHeader(
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
                // "12 of 50 items" would read as "the library has 50" while
                // only the loaded page is counted; say so when a page remains.
                switch ((filtering, page?.nextOffset != null)) {
                  (true, true) => '${items.length} of ${library.length} '
                        'matches · more available',
                  (true, false) => '${items.length} of ${library.length} items',
                  (false, true) =>
                    '${library.length} '
                        '${library.length == 1 ? 'item' : 'items'} · more available',
                  (false, false) =>
                    '${library.length} '
                        '${library.length == 1 ? 'item' : 'items'}',
                },
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
            Expanded(
              child: Align(
                alignment: Alignment.topCenter,
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(
                    OrSpacing.x3,
                    OrSpacing.x4,
                    OrSpacing.x3,
                    0,
                  ),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      const Text(
                        'No media matches this search.',
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          color: OrColors.textMuted,
                          fontSize: OrType.metadataCompactSize,
                        ),
                      ),
                      if (onClearSearch != null) ...[
                        const SizedBox(height: OrSpacing.x2),
                        TextButton(
                          key: const ValueKey('media-search-clear'),
                          onPressed: onClearSearch,
                          child: const Text('Clear search'),
                        ),
                      ],
                    ],
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
                      child: EditorMediaLibraryItem(
                        item: item,
                        preview:
                            liveUpdates[item.mediaId] ?? previews[item.mediaId],
                        onRetry: onRetryPreview == null
                            ? null
                            : () => onRetryPreview!(item),
                        onAddToTimeline: onInsertMedia == null
                            ? null
                            : () => unawaited(onInsertMedia!(item)),
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
                        onManageSpeechModel: onManageSpeechModel,
                        onCleanUpAudio:
                            onCleanUpAudio == null || item.audioDetails == null
                            ? null
                            : () => unawaited(onCleanUpAudio!(item)),
                      ),
                    ),
                  if (error != null) ...[
                    Padding(
                      padding: const EdgeInsets.all(OrSpacing.x2),
                      child: Semantics(
                        liveRegion: true,
                        child: Text(
                          error!,
                          key: const ValueKey('media-library-error'),
                          style: const TextStyle(
                            color: OrColors.textSecondary,
                            fontSize: OrType.tinySize,
                          ),
                        ),
                      ),
                    ),
                    TextButton(
                      key: const ValueKey('media-library-retry'),
                      // A retry while another page request is in flight would
                      // queue the same load twice and reset the appended page.
                      onPressed: loading || loadingMore ? null : onRefresh,
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

class EditorMediaLibraryItem extends StatelessWidget {
  const EditorMediaLibraryItem({
    super.key,
    required this.item,
    required this.preview,
    this.onRetry,
    required this.onAddToTimeline,
    required this.onRelink,
    required this.onRemove,
    required this.onGenerateCaptions,
    this.onManageSpeechModel,
    required this.onCleanUpAudio,
  });

  final ProjectMediaItem item;
  final ProjectMediaPreview? preview;

  /// Retries the preview after it failed.
  final VoidCallback? onRetry;
  final VoidCallback? onAddToTimeline;
  final VoidCallback? onRelink;
  final VoidCallback? onRemove;
  final VoidCallback? onGenerateCaptions;
  final VoidCallback? onManageSpeechModel;
  final VoidCallback? onCleanUpAudio;

  @override
  Widget build(BuildContext context) {
    final name = mediaDisplayName(item.sourceUri);
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
              'Add ${mediaDisplayName(item.sourceUri)} to Timeline',
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
                onManageSpeechModel == null &&
                onCleanUpAudio == null
            ? null
            : (action) {
                if (action == 'relink') onRelink?.call();
                if (action == 'remove') onRemove?.call();
                if (action == 'transcribe') onGenerateCaptions?.call();
                if (action == 'speech-model') onManageSpeechModel?.call();
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
          if (onManageSpeechModel != null)
            PopupMenuItem(
              key: ValueKey('media-speech-model-${item.mediaId}'),
              value: 'speech-model',
              child: const Text('Speech Model…'),
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
              EditorMediaItemPreview(
                item: item,
                preview: preview,
                onRetry: onRetry,
              ),
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
