import '../project/project_gateway.dart';
import 'media_preview_kinds.dart';

/// Tracks the artifact requests the library is waiting on.
///
/// The shell used to keep this bookkeeping in three loose fields: a ticket
/// list, a map of events that arrived before their request answered, and a
/// submission counter. Keeping them together makes the one rule explicit — an
/// early event is only worth remembering while a submission is in flight — and
/// gives the matching logic somewhere to be tested.
class MediaArtifactMailbox {
  final List<MediaPreviewTicket> _tickets = [];
  final Map<String, ProjectMediaArtifactEvent> _early = {};
  int _submissionsInFlight = 0;

  /// Tickets still waiting for their job event.
  List<MediaPreviewTicket> get tickets => List.unmodifiable(_tickets);

  bool get hasSubmissionsInFlight => _submissionsInFlight > 0;

  void beginSubmission() => _submissionsInFlight += 1;

  void endSubmission() {
    if (_submissionsInFlight > 0) _submissionsInFlight -= 1;
    if (_submissionsInFlight == 0) _early.clear();
  }

  void reset() {
    _tickets.clear();
    _early.clear();
    _submissionsInFlight = 0;
  }

  void add(MediaPreviewTicket ticket) => _tickets.add(ticket);

  /// Keeps an event that raced ahead of its request response.
  void rememberEarly(String key, ProjectMediaArtifactEvent event) {
    if (_submissionsInFlight > 0) _early[key] = event;
  }

  ProjectMediaArtifactEvent? earlyFor(String key) => _early[key];

  List<MediaPreviewTicket> matching(ProjectMediaArtifactEvent event) =>
      _tickets
          .where(
            (ticket) =>
                ticket.kind == event.kind &&
                ticket.cacheKey == event.cacheKey &&
                ticket.jobId == event.jobId,
          )
          .toList(growable: false);

  /// Removes and returns the tickets an event settles.
  List<MediaPreviewTicket> takeMatching(ProjectMediaArtifactEvent event) {
    final matched = matching(event);
    _tickets.removeWhere(matched.contains);
    return matched;
  }
}
