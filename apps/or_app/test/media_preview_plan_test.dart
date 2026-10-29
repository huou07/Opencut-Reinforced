import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/production/media_preview_plan.dart';

ProjectMediaItem _video(String id) => ProjectMediaItem(
  mediaId: id,
  sourceUri: Uri.file('/tmp/$id.mp4').toString(),
  formatNames: const ['mp4'],
  duration: '10 s',
  videoDetails: '1920×1080 h264',
  audioDetails: 'AAC',
);

ProjectMediaItem _audio(String id) => ProjectMediaItem(
  mediaId: id,
  sourceUri: Uri.file('/tmp/$id.wav').toString(),
  formatNames: const ['wav'],
  duration: '10 s',
  videoDetails: null,
  audioDetails: 'PCM',
);

void main() {
  test('a page with no previews queues the kind each item previews with', () {
    final plan = planMediaPreviews([_video('video'), _audio('audio')], const {});

    expect(plan['video']!.kind, ProjectMediaArtifactKind.thumbnail);
    expect(plan['audio']!.kind, ProjectMediaArtifactKind.waveform);
    expect(
      plan.values.every((p) => p.state == ProjectMediaArtifactRequestState.queued),
      isTrue,
    );
  });

  test('a preview of the right kind is kept instead of requeued', () {
    const ready = ProjectMediaPreview(
      kind: ProjectMediaArtifactKind.thumbnail,
      state: ProjectMediaArtifactRequestState.ready,
    );

    final plan = planMediaPreviews([_video('video')], {'video': ready});

    expect(plan['video'], same(ready));
  });

  test('previews for media outside the page are dropped', () {
    const ready = ProjectMediaPreview(
      kind: ProjectMediaArtifactKind.thumbnail,
      state: ProjectMediaArtifactRequestState.ready,
    );

    final plan = planMediaPreviews([_video('video')], {
      'video': ready,
      'gone': ready,
    });

    expect(plan.keys, ['video']);
  });

  test('only a failed preview of the supported kind is retryable', () {
    final video = _video('m1');
    const failed = ProjectMediaPreview(
      kind: ProjectMediaArtifactKind.thumbnail,
      state: ProjectMediaArtifactRequestState.failed,
    );
    final queued = retryablePreview(video, failed);
    expect(queued, isNotNull);
    expect(queued!.kind, ProjectMediaArtifactKind.thumbnail);
    expect(queued.state, ProjectMediaArtifactRequestState.queued);

    expect(retryablePreview(video, null), isNull);
    expect(
      retryablePreview(
        video,
        const ProjectMediaPreview(
          kind: ProjectMediaArtifactKind.waveform,
          state: ProjectMediaArtifactRequestState.failed,
        ),
      ),
      isNull,
    );
    expect(
      retryablePreview(
        video,
        const ProjectMediaPreview(
          kind: ProjectMediaArtifactKind.thumbnail,
          state: ProjectMediaArtifactRequestState.ready,
        ),
      ),
      isNull,
    );
    final audio = _audio('a1');
    expect(
      retryablePreview(
        audio,
        const ProjectMediaPreview(
          kind: ProjectMediaArtifactKind.thumbnail,
          state: ProjectMediaArtifactRequestState.failed,
        ),
      ),
      isNull,
    );
  });
}
