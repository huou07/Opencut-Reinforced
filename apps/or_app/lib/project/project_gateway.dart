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
