import '../project/project_gateway.dart';

/// The artifact a library item previews with, or null when it has none.
///
/// The kind is derived from the probed media details: a video item previews
/// with a thumbnail, an audio-only item with a waveform, and anything else
/// (a still with no audio stream, an unprobed item) with nothing at all.
ProjectMediaArtifactKind? mediaPreviewKind(ProjectMediaItem item) {
  if (item.videoDetails != null) return ProjectMediaArtifactKind.thumbnail;
  if (item.audioDetails != null) return ProjectMediaArtifactKind.waveform;
  return null;
}

/// The key that pairs an artifact event with the request that started it.
///
/// The event stream names the artifact by kind and cache key, and the request
/// also carries its job id, so the composite keeps two concurrent requests for
/// the same media apart.
String mediaArtifactEventKey(
  ProjectMediaArtifactKind kind,
  String cacheKey,
  String jobId,
) => '${kind.name}\u0000$cacheKey\u0000$jobId';

/// One outstanding artifact request the library is waiting on.
///
/// A ticket is only recorded once the request returns a cache key and job id;
/// it exists so an event that arrives before the request response can be
/// matched back to the media it belongs to.
class MediaPreviewTicket {
  const MediaPreviewTicket({
    required this.mediaId,
    required this.kind,
    required this.cacheKey,
    required this.jobId,
  });

  final String mediaId;
  final ProjectMediaArtifactKind kind;
  final String cacheKey;
  final String jobId;
}
