import 'dart:typed_data';

abstract interface class ProjectSessionHandle {}

class ProjectReadModel {
  const ProjectReadModel({
    required this.projectId,
    required this.projectInstanceId,
    required this.revision,
    required this.name,
    required this.dirty,
    required this.descriptorPath,
  });

  final String projectId;
  final String projectInstanceId;
  final BigInt revision;
  final String name;
  final bool dirty;
  final String descriptorPath;
}

class ProjectActionResult {
  const ProjectActionResult({
    required this.succeeded,
    this.errorCode = '',
    this.message = '',
    this.view,
  });

  final bool succeeded;
  final String errorCode;
  final String message;
  final ProjectReadModel? view;
}

/// Read-only media data returned by the bounded project query.
class ProjectMediaItem {
  const ProjectMediaItem({
    required this.mediaId,
    required this.sourceUri,
    required this.formatNames,
    required this.duration,
    required this.videoDetails,
    required this.audioDetails,
  });

  final String mediaId;
  final String sourceUri;
  final List<String> formatNames;
  final String? duration;
  final String? videoDetails;
  final String? audioDetails;
}

/// One bounded page from the project's insertion-ordered media library.
class ProjectMediaPage {
  const ProjectMediaPage({
    required this.projectId,
    required this.projectInstanceId,
    required this.projectRevision,
    required this.items,
    required this.totalCount,
    required this.offset,
    required this.limit,
    required this.nextOffset,
  });

  final String projectId;
  final String projectInstanceId;
  final BigInt projectRevision;
  final List<ProjectMediaItem> items;
  final int totalCount;
  final int offset;
  final int limit;
  final int? nextOffset;
}

enum ProjectMediaArtifactKind { thumbnail, waveform }

enum ProjectMediaArtifactRequestState {
  ready,
  queued,
  running,
  notApplicable,
  failed,
}

enum ProjectMediaArtifactEventState { succeeded, failed, cancelled }

class ProjectMediaArtifactRequest {
  const ProjectMediaArtifactRequest({
    required this.mediaId,
    required this.kind,
    required this.state,
    this.cacheKey,
    this.jobId,
    this.errorCode,
    this.message,
  });

  final String mediaId;
  final ProjectMediaArtifactKind kind;
  final ProjectMediaArtifactRequestState state;
  final String? cacheKey;
  final String? jobId;
  final String? errorCode;
  final String? message;
}

class ProjectMediaArtifact {
  const ProjectMediaArtifact({required this.bytes, required this.mimeType});

  final Uint8List bytes;
  final String mimeType;
}

class ProjectMediaArtifactEvent {
  const ProjectMediaArtifactEvent({
    required this.sequence,
    required this.mediaId,
    required this.kind,
    required this.cacheKey,
    required this.jobId,
    required this.state,
    this.errorCode,
  });

  final BigInt sequence;
  final String mediaId;
  final ProjectMediaArtifactKind kind;
  final String cacheKey;
  final String jobId;
  final ProjectMediaArtifactEventState state;
  final String? errorCode;
}

class ProjectMediaPreview {
  const ProjectMediaPreview({
    required this.kind,
    required this.state,
    this.cacheKey,
    this.jobId,
    this.bytes,
    this.errorCode,
  });

  final ProjectMediaArtifactKind kind;
  final ProjectMediaArtifactRequestState state;
  final String? cacheKey;
  final String? jobId;
  final Uint8List? bytes;
  final String? errorCode;
}

class ProjectHostEvent {
  const ProjectHostEvent({
    required this.sequence,
    required this.kind,
    required this.projectId,
    required this.projectInstanceId,
    required this.revision,
    required this.dirty,
  });

  final BigInt sequence;
  final String kind;
  final String projectId;
  final String projectInstanceId;
  final BigInt revision;
  final bool dirty;
}

enum ProjectRecoveryKind { none, candidate, stale, conflict, invalid }

class ProjectRecoveryInspection {
  const ProjectRecoveryInspection({
    required this.kind,
    required this.projectId,
    required this.baseRevision,
    required this.recoveryRevision,
    required this.recoveryName,
    required this.conflictReason,
    required this.message,
  });

  final ProjectRecoveryKind kind;
  final String projectId;
  final BigInt baseRevision;
  final BigInt recoveryRevision;
  final String recoveryName;
  final String conflictReason;
  final String message;
}

class ProjectRecoveryActionResult {
  const ProjectRecoveryActionResult({
    required this.succeeded,
    required this.changed,
    required this.message,
  });

  final bool succeeded;
  final bool changed;
  final String message;
}

class ProjectGatewayException implements Exception {
  const ProjectGatewayException(this.code, this.message);

  final String code;
  final String message;

  @override
  String toString() => '$code: $message';
}

abstract interface class ProjectGateway {
  Future<ProjectSessionHandle> createProject(String path, String name);
  Future<ProjectSessionHandle> openProject(String path);
  Future<ProjectReadModel> summary(ProjectSessionHandle session);
  Future<ProjectActionResult> rename(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String name,
  );
  Future<ProjectActionResult> undo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  );
  Future<ProjectActionResult> redo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  );
  Future<ProjectMediaPage> listMediaPage(
    ProjectSessionHandle session, {
    required int offset,
    required int limit,
  });
  Future<ProjectMediaArtifactRequest> requestMediaThumbnail(
    ProjectSessionHandle session,
    String mediaId,
  );
  Future<ProjectMediaArtifactRequest> requestMediaWaveform(
    ProjectSessionHandle session,
    String mediaId,
  );
  Future<ProjectMediaArtifact?> readMediaArtifact(
    ProjectSessionHandle session, {
    required ProjectMediaArtifactKind kind,
    required String cacheKey,
  });
  Stream<ProjectMediaArtifactEvent> watchMediaArtifacts(
    ProjectSessionHandle session,
  );
  Future<ProjectActionResult> importMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String path,
  );
  Future<ProjectActionResult> removeMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String mediaId,
  );
  Future<ProjectActionResult> save(ProjectSessionHandle session);
  Future<void> close(
    ProjectSessionHandle session, {
    required bool discardUnsaved,
  });
  Stream<ProjectHostEvent> watch(ProjectSessionHandle session);
  Future<ProjectRecoveryInspection> inspectRecovery(String path);
  Future<ProjectRecoveryActionResult> applyRecovery(String path);
  Future<ProjectRecoveryActionResult> discardRecovery(String path);
}
