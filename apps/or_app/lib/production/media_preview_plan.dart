import '../project/project_gateway.dart';

import 'media_preview_kinds.dart';

/// The preview state a media page should show after a refresh.
///
/// One rule set, two callers: the shell applies the result and the widget
/// test asserts it. Keeping it pure means the "keep a preview whose kind
/// still matches, queue a missing one, drop invisible ones" policy can be
/// checked without a session, a gateway or a widget tree.
Map<String, ProjectMediaPreview> planMediaPreviews(
  List<ProjectMediaItem> items,
  Map<String, ProjectMediaPreview> previews,
) {
  final visibleIds = items.map((item) => item.mediaId).toSet();
  final next = Map<String, ProjectMediaPreview>.of(previews)
    ..removeWhere((mediaId, _) => !visibleIds.contains(mediaId));
  for (final item in items) {
    final kind = mediaPreviewKind(item);
    if (kind == null || next[item.mediaId]?.kind == kind) continue;
    next[item.mediaId] = ProjectMediaPreview(
      kind: kind,
      state: ProjectMediaArtifactRequestState.queued,
    );
  }
  return next;
}
