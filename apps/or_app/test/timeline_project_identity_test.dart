import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/screens/timeline_project_identity.dart';

ProjectReadModel _view({
  String projectId = 'project-1',
  String instanceId = 'instance-1',
  int revision = 1,
}) {
  return ProjectReadModel(
    projectId: projectId,
    projectInstanceId: instanceId,
    revision: BigInt.from(revision),
    name: 'Project',
    dirty: false,
    descriptorPath: '/tmp/project.orproj',
  );
}

void main() {
  test('the same project instance is not an identity change', () {
    expect(timelineProjectIdentityChanged(_view(), _view()), isFalse);
    expect(
      timelineProjectIdentityChanged(_view(revision: 4), _view(revision: 9)),
      isFalse,
      reason: 'a revision bump is the same document',
    );
  });

  test('a different instance or id is an identity change', () {
    expect(
      timelineProjectIdentityChanged(_view(), _view(instanceId: 'instance-2')),
      isTrue,
    );
    expect(
      timelineProjectIdentityChanged(_view(), _view(projectId: 'project-2')),
      isTrue,
    );
  });

  test('a first project counts as an identity change', () {
    expect(timelineProjectIdentityChanged(null, _view()), isTrue);
  });
}
