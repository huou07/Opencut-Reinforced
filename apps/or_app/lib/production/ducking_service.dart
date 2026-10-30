import '../project/project_gateway.dart';

class DuckingResult {
  const DuckingResult({required this.changed, required this.undoSteps});

  final bool changed;
  final int undoSteps;
}

class DuckingException implements Exception {
  const DuckingException(this.message);

  final String message;

  @override
  String toString() => message;
}

/// Lowers a music clip wherever one or more voice clips overlap it.
///
/// The split and gain edit happens in Rust as `timeline.clip.duck`, so the whole
/// duck is one revision and one undo entry even when it produces many segments.
class DuckingService {
  const DuckingService(this.gateway);

  final ProjectGateway gateway;

  Future<DuckingResult> duck({
    required ProjectSessionHandle session,
    required ProjectReadModel project,
    required String musicTrackId,
    required String musicClipId,
    required String voiceTrackId,
    int duckDecibels = -12,
  }) async {
    if (musicTrackId == voiceTrackId) {
      throw const DuckingException(
        'Choose a music track and a separate voice track.',
      );
    }
    if (duckDecibels > -1 || duckDecibels < -40) {
      throw const DuckingException('The duck must be between -40 and -1 dB.');
    }
    final result = await gateway.duckTimelineClip(
      session,
      project,
      trackId: musicTrackId,
      clipId: musicClipId,
      voiceTrackId: voiceTrackId,
      duckDecibels: duckDecibels,
    );
    if (!result.succeeded) {
      throw DuckingException(
        result.message.isEmpty
            ? 'The music could not be ducked under that voice track.'
            : result.message,
      );
    }
    return const DuckingResult(changed: true, undoSteps: 1);
  }
}
