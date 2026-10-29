import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/media_preview_kinds.dart';
import 'package:or_app/project/project_gateway.dart';

ProjectMediaItem _item({
  String mediaId = 'media-1',
  String? videoDetails,
  String? audioDetails,
}) => ProjectMediaItem(
  mediaId: mediaId,
  sourceUri: Uri.file('/tmp/source.mp4').toString(),
  formatNames: const ['mp4'],
  duration: '10 s',
  videoDetails: videoDetails,
  audioDetails: audioDetails,
);

void main() {
  test('a video item previews with a thumbnail', () {
    expect(
      mediaPreviewKind(_item(videoDetails: '1920×1080 h264')),
      ProjectMediaArtifactKind.thumbnail,
    );
  });

  test('an audio-only item previews with a waveform', () {
    expect(
      mediaPreviewKind(_item(audioDetails: 'stereo 48 kHz')),
      ProjectMediaArtifactKind.waveform,
    );
  });

  test('an item with no probed stream previews with nothing', () {
    expect(mediaPreviewKind(_item()), isNull);
  });

  test('artifact keys separate jobs and kinds', () {
    final first = mediaArtifactEventKey(
      ProjectMediaArtifactKind.thumbnail,
      'cache-a',
      'job-1',
    );
    expect(
      first,
      mediaArtifactEventKey(
        ProjectMediaArtifactKind.thumbnail,
        'cache-a',
        'job-1',
      ),
    );
    expect(
      first,
      isNot(
        mediaArtifactEventKey(
          ProjectMediaArtifactKind.waveform,
          'cache-a',
          'job-1',
        ),
      ),
    );
    expect(
      first,
      isNot(
        mediaArtifactEventKey(
          ProjectMediaArtifactKind.thumbnail,
          'cache-a',
          'job-2',
        ),
      ),
    );
  });

  test('tickets carry the request identity', () {
    const ticket = MediaPreviewTicket(
      mediaId: 'media-1',
      kind: ProjectMediaArtifactKind.waveform,
      cacheKey: 'cache-a',
      jobId: 'job-1',
    );
    expect(ticket.mediaId, 'media-1');
    expect(ticket.kind, ProjectMediaArtifactKind.waveform);
  });
}
