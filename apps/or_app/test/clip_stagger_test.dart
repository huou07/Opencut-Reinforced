import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/screens/clip_motion_presets.dart';
import 'package:or_app/screens/clip_stagger.dart';

ProjectReadModel _view(int revision) => ProjectReadModel(
  projectId: 'project-1',
  projectInstanceId: 'instance-1',
  revision: BigInt.from(revision),
  name: 'Project',
  dirty: false,
  descriptorPath: '/tmp/project.orproj',
);

ProjectTimelineClip _clip(String id, double seconds) => ProjectTimelineClip(
  clipId: id,
  timelineStart: ProjectRationalTime(BigInt.zero, 1),
  timelineDuration: ProjectRationalTime(
    BigInt.from((seconds * 1000).round()),
    1000,
  ),
);

class _Session implements ProjectSessionHandle {}

/// Answers only the two animation calls; anything else is a test bug.
class _FakeAnimationGateway implements ProjectGateway {
  _FakeAnimationGateway({this.failOnClipId, this.failWithResult = false});

  final String? failOnClipId;
  final bool failWithResult;
  final List<String> updates = [];
  int _revision = 1;

  @override
  Future<ProjectClipAnimation> getTimelineClipAnimation(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
  }) async => ProjectClipAnimation.empty;

  @override
  Future<ProjectActionResult> updateTimelineClipAnimation(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectClipAnimation animation,
  }) async {
    if (clipId == failOnClipId) {
      if (failWithResult) {
        return const ProjectActionResult(
          succeeded: false,
          errorCode: 'timeline_clip_not_found',
          message: 'That clip is gone.',
        );
      }
      throw const ProjectGatewayException('bridge', 'The host refused it.');
    }
    expect(
      current.revision,
      BigInt.from(_revision),
      reason: 'each call must chain the revision the previous one returned',
    );
    _revision += 1;
    updates.add(clipId);
    return ProjectActionResult(succeeded: true, view: _view(_revision));
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  test('a stagger spreads clips apart and clamps each entrance', () {
    final plan = planStagger(
      orderedClipIds: const ['a', 'b', 'c', 'd'],
      durationsSeconds: const [2.0, 2.0, 2.0, 2.0],
      delaySeconds: 0.2,
    );
    expect(
      [for (final step in plan.steps) step.delaySeconds],
      [0, closeTo(0.2, 1e-9), closeTo(0.4, 1e-9), closeTo(0.6, 1e-9)],
    );
    expect(
      [for (final step in plan.steps) step.spanSeconds],
      [0.7, closeTo(0.7, 1e-9), closeTo(0.7, 1e-9), closeTo(0.7, 1e-9)],
    );
    expect(plan.skippedClipIds, isEmpty);
    expect(plan.clipCount, 4);
  });

  test('a clip too short for its delay is skipped, not truncated', () {
    final plan = planStagger(
      orderedClipIds: const ['long', 'short'],
      durationsSeconds: const [2.0, 0.1],
      delaySeconds: 0.2,
    );
    expect([for (final step in plan.steps) step.clipId], ['long']);
    expect(plan.skippedClipIds, ['short']);
  });

  test('a negative delay still cascades from the first clip', () {
    final plan = planStagger(
      orderedClipIds: const ['a', 'b'],
      durationsSeconds: const [2.0, 2.0],
      delaySeconds: -1,
    );
    expect([for (final step in plan.steps) step.delaySeconds], [0, 0]);
  });

  test('a preset writes its keyframes after the delay', () {
    final animation = motionPresets
        .firstWhere((preset) => preset.wire == 'fade-in')
        .build(ProjectClipAnimation.empty, 0.4, 0.2);
    expect(animation.opacityBasisPoints, hasLength(2));
    expect(animation.opacityBasisPoints.first.at.secondsForDisplay, 0.2);
    expect(animation.opacityBasisPoints.first.value, 0);
    expect(animation.opacityBasisPoints.last.at.secondsForDisplay, 0.6);
    expect(animation.opacityBasisPoints.last.value, 10000);
  });

  test('the span never exceeds the preset ceiling or the floor', () {
    expect(motionSpanFor(5), 0.7);
    expect(motionSpanFor(0.01), 0.05);
    expect(motionSpanFor(0.3), 0.3);
  });

  test('a stagger writes each clip once and chains revisions', () async {
    final fake = _FakeAnimationGateway();
    final outcome = await applyStagger(
      gateway: fake,
      session: _Session(),
      project: _view(1),
      clips: [_clip('a', 2), _clip('b', 2), _clip('c', 2)],
      trackIdByClipId: {'a': 't1', 'b': 't1', 'c': 't1'},
      preset: motionPresets.first,
      delaySeconds: 0.2,
    );
    expect(outcome.appliedClipIds, ['a', 'b', 'c']);
    expect(outcome.skippedClipIds, isEmpty);
    expect(outcome.failure, isNull);
    expect(fake.updates, ['a', 'b', 'c']);
  });

  test('a failed update stops the cascade and names the failure', () async {
    final fake = _FakeAnimationGateway(failOnClipId: 'b');
    final outcome = await applyStagger(
      gateway: fake,
      session: _Session(),
      project: _view(1),
      clips: [_clip('a', 2), _clip('b', 2), _clip('c', 2)],
      trackIdByClipId: {'a': 't1', 'b': 't1', 'c': 't1'},
      preset: motionPresets.first,
      delaySeconds: 0.2,
    );
    expect(outcome.appliedClipIds, ['a']);
    expect(outcome.failure, 'The host refused it.');
    expect(fake.updates, ['a']);
  });

  test('a refused result carries its own message', () async {
    final fake = _FakeAnimationGateway(failOnClipId: 'a', failWithResult: true);
    final outcome = await applyStagger(
      gateway: fake,
      session: _Session(),
      project: _view(1),
      clips: [_clip('a', 2)],
      trackIdByClipId: {'a': 't1'},
      preset: motionPresets.first,
      delaySeconds: 0,
    );
    expect(outcome.appliedClipIds, isEmpty);
    expect(outcome.failure, 'That clip is gone.');
  });
}
