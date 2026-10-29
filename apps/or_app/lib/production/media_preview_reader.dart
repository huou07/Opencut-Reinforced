import '../project/project_gateway.dart';

/// Turns one artifact read into the preview update it deserves.
///
/// The rules this owns are easy to get subtly wrong inline: the read is only
/// useful while the preview still names the same cache key, a PNG is the only
/// artifact the library can paint, and a missing or unreadable artifact must
/// surface as a failure instead of a pending tile.
class MediaPreviewReader {
  MediaPreviewReader({
    required this.isCurrentSession,
    required this.readArtifact,
    required this.current,
  });

  final bool Function(Object session) isCurrentSession;
  final Future<ProjectMediaArtifact?> Function(
    Object session,
    ProjectMediaArtifactKind kind,
    String cacheKey,
  )
  readArtifact;

  /// The preview currently stored for a media id, if any.
  final ProjectMediaPreview? Function(String mediaId) current;

  Future<ProjectMediaPreview?> read(
    Object session, {
    required String mediaId,
    required ProjectMediaArtifactKind kind,
    required String cacheKey,
  }) async {
    if (!_stillWanted(session, mediaId, cacheKey)) return null;
    try {
      final artifact = await readArtifact(session, kind, cacheKey);
      if (!_stillWanted(session, mediaId, cacheKey)) return null;
      final isPng = artifact?.mimeType == 'image/png';
      return ProjectMediaPreview(
        kind: kind,
        state: isPng
            ? ProjectMediaArtifactRequestState.ready
            : ProjectMediaArtifactRequestState.failed,
        cacheKey: cacheKey,
        bytes: isPng ? artifact?.bytes : null,
        errorCode: artifact == null ? 'ARTIFACT_NOT_FOUND' : null,
      );
    } catch (_) {
      if (!_stillWanted(session, mediaId, cacheKey)) return null;
      return ProjectMediaPreview(
        kind: kind,
        state: ProjectMediaArtifactRequestState.failed,
        cacheKey: cacheKey,
        errorCode: 'ARTIFACT_READ_FAILED',
      );
    }
  }

  bool _stillWanted(Object session, String mediaId, String cacheKey) {
    if (!isCurrentSession(session)) return false;
    final preview = current(mediaId);
    return preview != null && preview.cacheKey == cacheKey;
  }
}
