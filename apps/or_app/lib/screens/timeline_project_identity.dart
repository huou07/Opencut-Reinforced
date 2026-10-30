import '../project/project_gateway.dart';

/// Whether [previous] and [next] belong to different project instances.
///
/// The timeline panel owns selection state (selected clip ids, a single
/// selection and zoom) that is only meaningful inside one project. A revision
/// bump is the same project; a different id or instance is not.
bool timelineProjectIdentityChanged(
  ProjectReadModel? previous,
  ProjectReadModel? next,
) {
  return previous?.projectId != next?.projectId ||
      previous?.projectInstanceId != next?.projectInstanceId;
}
