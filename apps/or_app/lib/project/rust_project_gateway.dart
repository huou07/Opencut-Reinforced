import 'package:or_app_bridge/or_app_bridge.dart' as rust;

import 'project_gateway.dart';

class RustProjectGateway implements ProjectGateway {
  const RustProjectGateway();

  @override
  Future<ProjectSessionHandle> createProject(String path, String name) async {
    try {
      return _RustProjectSession(
        await rust.createProject(path: path, name: name),
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectSessionHandle> openProject(String path) async {
    try {
      return _RustProjectSession(await rust.openProject(path: path));
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectReadModel> summary(ProjectSessionHandle session) async {
    try {
      return _readModel(await _host(session).summary());
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectActionResult> rename(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String name,
  ) async => _action(
    await _host(session).rename(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      name: name,
    ),
  );

  @override
  Future<ProjectActionResult> undo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  ) async => _action(
    await _host(session).undo(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
    ),
  );

  @override
  Future<ProjectActionResult> redo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  ) async => _action(
    await _host(session).redo(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
    ),
  );

  @override
  Future<ProjectActionResult> save(ProjectSessionHandle session) async =>
      _action(await _host(session).save());

  @override
  Future<void> close(
    ProjectSessionHandle session, {
    required bool discardUnsaved,
  }) async {
    final result = await _host(session).close(discardUnsaved: discardUnsaved);
    if (!result.succeeded) {
      throw ProjectGatewayException(result.errorCode, result.message);
    }
  }

  @override
  Stream<ProjectHostEvent> watch(ProjectSessionHandle session) =>
      _host(session).subscribeEvents().map(
        (event) => ProjectHostEvent(
          sequence: event.sequence,
          kind: event.kind,
          projectId: event.projectId,
          projectInstanceId: event.projectInstanceId,
          revision: event.revision,
          dirty: event.dirty,
        ),
      );

  @override
  Future<ProjectRecoveryInspection> inspectRecovery(String path) async {
    final inspection = await rust.inspectRecovery(path: path);
    return ProjectRecoveryInspection(
      kind: switch (inspection.status) {
        'candidate' => ProjectRecoveryKind.candidate,
        'stale' => ProjectRecoveryKind.stale,
        'conflict' => ProjectRecoveryKind.conflict,
        'invalid' => ProjectRecoveryKind.invalid,
        _ => ProjectRecoveryKind.none,
      },
      projectId: inspection.projectId,
      baseRevision: inspection.baseRevision,
      recoveryRevision: inspection.recoveryRevision,
      recoveryName: inspection.recoveryName,
      conflictReason: inspection.conflictReason,
      message: inspection.message,
    );
  }

  @override
  Future<ProjectRecoveryActionResult> applyRecovery(String path) async =>
      _recoveryAction(await rust.applyRecovery(path: path));

  @override
  Future<ProjectRecoveryActionResult> discardRecovery(String path) async =>
      _recoveryAction(await rust.discardRecovery(path: path));

  static rust.ProjectHostHandle _host(ProjectSessionHandle session) {
    if (session is _RustProjectSession) return session.host;
    throw ArgumentError.value(session, 'session', 'belongs to another gateway');
  }

  static ProjectReadModel _readModel(rust.ProjectView view) => ProjectReadModel(
    projectId: view.projectId,
    projectInstanceId: view.projectInstanceId,
    revision: view.revision,
    name: view.name,
    dirty: view.dirty,
    descriptorPath: view.descriptorPath,
  );

  static ProjectActionResult _action(rust.ProjectActionResult result) =>
      ProjectActionResult(
        succeeded: result.succeeded,
        errorCode: result.errorCode,
        message: result.message,
        view: result.view == null ? null : _readModel(result.view!),
      );

  static ProjectRecoveryActionResult _recoveryAction(
    rust.RecoveryActionResult result,
  ) => ProjectRecoveryActionResult(
    succeeded: result.succeeded,
    changed: result.changed,
    message: result.message,
  );
}

class _RustProjectSession implements ProjectSessionHandle {
  const _RustProjectSession(this.host);

  final rust.ProjectHostHandle host;
}
