import 'project_gateway.dart';

/// One clip as the clipboard holds it.
///
/// A copied clip is its content and its placement, not its identity: pasting
/// asks the command path to create a clip, so the copy carries what that command
/// needs and nothing it would have to overwrite.
class TimelineClipClipboard {
  const TimelineClipClipboard({
    required this.contentKind,
    required this.duration,
    this.mediaId,
    this.sourceStart,
    this.text,
    this.formatting,
  });

  final ProjectTimelineClipContentKind contentKind;
  final ProjectRationalTime duration;
  final String? mediaId;
  final ProjectRationalTime? sourceStart;
  final String? text;
  final ProjectTextFormatting? formatting;

  bool get isMedia => contentKind == ProjectTimelineClipContentKind.media;

  /// The clipboard for a clip, or null when the clip cannot be recreated (a
  /// media clip without a source range has nothing to point at).
  static TimelineClipClipboard? fromClip(ProjectTimelineClip clip) {
    if (clip.contentKind == ProjectTimelineClipContentKind.media) {
      final mediaId = clip.mediaId;
      final sourceStart = clip.sourceStart;
      if (mediaId == null || sourceStart == null) return null;
      return TimelineClipClipboard(
        contentKind: clip.contentKind,
        duration: clip.timelineDuration,
        mediaId: mediaId,
        sourceStart: sourceStart,
      );
    }
    return TimelineClipClipboard(
      contentKind: clip.contentKind,
      duration: clip.timelineDuration,
      text: clip.text ?? '',
      formatting: clip.formatting ?? ProjectTextFormatting.defaults,
    );
  }
}
