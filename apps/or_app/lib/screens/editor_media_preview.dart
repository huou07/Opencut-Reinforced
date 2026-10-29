import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../project/project_gateway.dart';

/// The small square that shows what an item is: a thumbnail for visual media, a
/// waveform for audio, the retry control after a failed request, and a plain
/// kind glyph while nothing is known yet.
///
/// It lives beside the library panel rather than inside it so the panel keeps
/// owning layout while this owns one preview's three states.
class EditorMediaItemPreview extends StatelessWidget {
  const EditorMediaItemPreview({
    super.key,
    required this.item,
    required this.preview,
    this.onRetry,
  });

  final ProjectMediaItem item;
  final ProjectMediaPreview? preview;
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final hasVideo = item.videoDetails != null;
    final kind = hasVideo
        ? ProjectMediaArtifactKind.thumbnail
        : item.audioDetails != null
        ? ProjectMediaArtifactKind.waveform
        : null;
    final failed = preview?.state == ProjectMediaArtifactRequestState.failed;
    final generating = preview?.kind == kind &&
        (preview?.state == ProjectMediaArtifactRequestState.running ||
            preview?.state == ProjectMediaArtifactRequestState.queued);
    final bytes = preview?.kind == kind && !failed ? preview?.bytes : null;
    final label = kind == ProjectMediaArtifactKind.thumbnail
        ? 'Thumbnail preview for ${mediaDisplayName(item.sourceUri)}'
        : 'Waveform preview for ${mediaDisplayName(item.sourceUri)}';
    return SizedBox(
      key: ValueKey('media-preview-${item.mediaId}'),
      width: 52,
      height: 36,
      child: failed
          ? Tooltip(
              message: 'Preview unavailable. Retry.',
              child: Semantics(
                button: true,
                label: 'Preview unavailable. Retry.',
                child: MouseRegion(
                  cursor: onRetry == null
                      ? MouseCursor.defer
                      : SystemMouseCursors.click,
                  child: GestureDetector(
                    key: ValueKey('media-preview-retry-${item.mediaId}'),
                    behavior: HitTestBehavior.opaque,
                    onTap: onRetry,
                    child: const Center(
                      child: Icon(
                        Icons.refresh,
                        size: 16,
                        color: OrColors.textMuted,
                      ),
                    ),
                  ),
                ),
              ),
            )
          : bytes == null
          ? generating
                ? Center(
                    key: ValueKey('media-preview-progress-${item.mediaId}'),
                    child: Semantics(
                      label: 'Generating preview for '
                          '${mediaDisplayName(item.sourceUri)}',
                      child: const Icon(
                        Icons.hourglass_top,
                        size: 14,
                        color: OrColors.textMuted,
                      ),
                    ),
                  )
                : Center(
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
                // The tile is 52 wide; decoding a 4K thumbnail at full size
                // costs tens of megabytes per library page.
                cacheWidth: (52 * MediaQuery.devicePixelRatioOf(context)).round(),
                cacheHeight: (36 * MediaQuery.devicePixelRatioOf(context)).round(),
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

String mediaDisplayName(String sourceUri) {
  final uri = Uri.tryParse(sourceUri);
  if (uri != null && uri.pathSegments.isNotEmpty) {
    return uri.pathSegments.last;
  }
  final normalized = sourceUri.replaceAll('\\', '/');
  return normalized.split('/').last;
}
