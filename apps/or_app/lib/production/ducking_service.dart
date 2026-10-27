import '../project/project_gateway.dart';

class DuckingResult {
  const DuckingResult({
    required this.splitCount,
    required this.changedCount,
    required this.voiceIntervals,
  });

  final int splitCount;
  final int changedCount;
  final int voiceIntervals;

  int get undoSteps => splitCount + changedCount;
}

class DuckingException implements Exception {
  const DuckingException(this.message);

  final String message;

  @override
  String toString() => message;
}

/// Lowers a music clip wherever one or more voice clips overlap it.
///
/// The project model has static clip gain and no volume automation yet, so the
/// duck is expressed with ordinary validated commands: split the music clip at
/// every voice boundary, then set the gain of the overlapping segments. The
/// result is editable and exportable like any other clip; it costs one undo step
/// per split and per changed segment.
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
      throw const DuckingException('Choose a music track and a separate voice track.');
    }
    if (duckDecibels > -1 || duckDecibels < -40) {
      throw const DuckingException('The duck must be between -40 and -1 dB.');
    }
    final musicClips = await _allClips(session, musicTrackId);
    ProjectTimelineClip? music;
    for (final clip in musicClips) {
      if (clip.clipId == musicClipId) {
        music = clip;
        break;
      }
    }
    if (music == null) {
      throw const DuckingException('The music clip is no longer on its track.');
    }
    final musicStart = _ms(music.timelineStart);
    final musicEnd = musicStart + _ms(music.timelineDuration);
    final voiceClips = await _allClips(session, voiceTrackId);
    final intervals = <DuckingInterval>[];
    for (final clip in voiceClips) {
      final start = _ms(clip.timelineStart);
      final end = start + _ms(clip.timelineDuration);
      if (end <= musicStart || start >= musicEnd) continue;
      intervals.add(
        DuckingInterval(
          start < musicStart ? musicStart : start,
          end > musicEnd ? musicEnd : end,
        ),
      );
    }
    if (intervals.isEmpty) {
      throw const DuckingException(
        'No voice clip overlaps the music clip on that track.',
      );
    }
    final merged = mergeDuckingIntervals(intervals);
    final targetGain = duckDecibels * 1000;
    var current = project;
    var splits = 0;
    var changed = 0;

    final boundaries = <int>{};
    for (final interval in merged) {
      if (interval.start > musicStart && interval.start < musicEnd) {
        boundaries.add(interval.start);
      }
      if (interval.end > musicStart && interval.end < musicEnd) {
        boundaries.add(interval.end);
      }
    }
    final ordered = boundaries.toList()..sort((left, right) => right.compareTo(left));
    for (final boundary in ordered) {
      final result = await gateway.splitTimelineClip(
        session,
        current,
        clipId: musicClipId,
        timelineTime: ProjectRationalTime(BigInt.from(boundary), 1000),
      );
      if (!result.succeeded) {
        throw DuckingException(
          result.message.isEmpty ? 'The music clip could not be split.' : result.message,
        );
      }
      current = result.view ?? await gateway.summary(session);
      splits += 1;
    }

    final segments = await _allClips(session, musicTrackId);
    for (final segment in segments) {
      final start = _ms(segment.timelineStart);
      final end = start + _ms(segment.timelineDuration);
      if (start < musicStart || end > musicEnd) continue;
      final overlaps = merged.any(
        (interval) => start < interval.end && end > interval.start,
      );
      if (!overlaps) continue;
      final settings = await gateway.getTimelineClipAudioSettings(
        session,
        current,
        trackId: musicTrackId,
        clipId: segment.clipId,
      );
      if (settings.gainMilliDecibels == targetGain) continue;
      final updated = await gateway.updateTimelineClipAudioSettings(
        session,
        current,
        trackId: musicTrackId,
        clipId: segment.clipId,
        settings: ProjectTimelineAudioSettings(
          gainMilliDecibels: targetGain,
          panBasisPoints: settings.panBasisPoints,
          fadeIn: settings.fadeIn,
          fadeOut: settings.fadeOut,
        ),
      );
      if (!updated.succeeded) {
        throw DuckingException(
          updated.message.isEmpty ? 'A music segment could not be lowered.' : updated.message,
        );
      }
      current = updated.view ?? await gateway.summary(session);
      changed += 1;
    }
    return DuckingResult(
      splitCount: splits,
      changedCount: changed,
      voiceIntervals: merged.length,
    );
  }

  Future<List<ProjectTimelineClip>> _allClips(
    ProjectSessionHandle session,
    String trackId,
  ) async {
    final clips = <ProjectTimelineClip>[];
    var offset = 0;
    while (true) {
      final page = await gateway.listTimelineClips(
        session,
        trackId: trackId,
        offset: offset,
        limit: 100,
      );
      clips.addAll(page.items);
      final next = page.nextOffset;
      if (next == null || next <= offset) break;
      offset = next;
    }
    return clips;
  }

  int _ms(ProjectRationalTime time) =>
      ((time.numerator * BigInt.from(1000)) ~/ BigInt.from(time.denominator)).toInt();

}

/// Merges overlapping or touching millisecond intervals in ascending order.
List<DuckingInterval> mergeDuckingIntervals(List<DuckingInterval> intervals) {
  final sorted = [...intervals]..sort((left, right) => left.start.compareTo(right.start));
  final merged = <DuckingInterval>[];
  for (final interval in sorted) {
    if (merged.isEmpty || interval.start > merged.last.end) {
      merged.add(interval);
      continue;
    }
    final last = merged.removeLast();
    merged.add(
      DuckingInterval(last.start, interval.end > last.end ? interval.end : last.end),
    );
  }
  return merged;
}

class DuckingInterval {
  const DuckingInterval(this.start, this.end);

  final int start;
  final int end;
}
