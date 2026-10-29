import 'dart:collection';

import '../project/project_gateway.dart';
import 'media_preview_kinds.dart';

/// Runs at most [maxActive] artifact requests at a time over a media page.
///
/// It owns the three pieces of state that used to sit loose in the shell: the
/// pending queue, the set of ids already queued, and the number of requests in
/// flight. An item leaves the queue when the page no longer shows it, and the
/// queue drains again every time a request finishes.
class MediaPreviewQueue {
  MediaPreviewQueue({
    required this.isCurrentSession,
    required this.visibleItems,
    required this.onStart,
    required this.maxActive,
  });

  final bool Function(Object session) isCurrentSession;

  /// The page currently shown by the library, or null while it is loading.
  final Iterable<ProjectMediaItem>? Function() visibleItems;
  final void Function(
    Object session,
    ProjectMediaItem item,
    ProjectMediaArtifactKind kind,
  )
  onStart;
  final int maxActive;

  final Queue<ProjectMediaItem> _queue = Queue();
  final Set<String> _queued = {};
  int _active = 0;

  int get activeRequests => _active;

  /// Adds every item not already queued; returns the ids that were new.
  Set<String> enqueue(Iterable<ProjectMediaItem> items) {
    final added = <String>{};
    for (final item in items) {
      if (_queued.add(item.mediaId)) {
        _queue.addLast(item);
        added.add(item.mediaId);
      }
    }
    return added;
  }

  /// Drops queued items the page no longer shows.
  void removeInvisible(Set<String> visibleIds) {
    _queue.removeWhere((item) => !visibleIds.contains(item.mediaId));
    _queued.removeWhere((mediaId) => !visibleIds.contains(mediaId));
  }

  /// Accounts for [count] settled requests and drains again.
  void settle(int count, Object session) {
    _active = _active >= count ? _active - count : 0;
    drain(session);
  }

  /// Moves one item to the back of the queue, keeping its single slot.
  void retry(ProjectMediaItem item) {
    _queue.removeWhere((queued) => queued.mediaId == item.mediaId);
    _queue.addLast(item);
    _queued.add(item.mediaId);
  }

  /// Starts requests until the in-flight limit is reached or the queue empties.
  void drain(Object session) {
    if (!isCurrentSession(session)) return;
    final visible = visibleItems()?.map((item) => item.mediaId).toSet();
    if (visible == null) return;
    while (_active < maxActive && _queue.isNotEmpty) {
      final item = _queue.removeFirst();
      _queued.remove(item.mediaId);
      if (!visible.contains(item.mediaId)) continue;
      final kind = mediaPreviewKind(item);
      if (kind == null) continue;
      _active += 1;
      onStart(session, item, kind);
    }
  }

  void finishRequest(Object session) {
    if (!isCurrentSession(session)) return;
    if (_active > 0) _active -= 1;
    drain(session);
  }

  void reset() {
    _queue.clear();
    _queued.clear();
    _active = 0;
  }
}
