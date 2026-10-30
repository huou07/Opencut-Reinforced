import 'package:flutter/foundation.dart';

import '../project/project_gateway.dart';

/// Stores one media preview and repaints only the surface that shows it.
///
/// The rule this owns is the round-16 perf fix: the timeline paints clip
/// filmstrips from the preview map, so a preview for media that is on the
/// timeline still needs an editor rebuild; every other preview goes through
/// [updates] and repaints just the media pane.
class MediaPreviewPublisher {
  MediaPreviewPublisher({
    required this.isMounted,
    required this.isCurrentSession,
    required this.holds,
    required this.isOnTimeline,
    required this.rebuildEditor,
    required this.updates,
  });

  final bool Function() isMounted;
  final bool Function(Object session) isCurrentSession;

  /// Whether the map already carries an entry for [mediaId]; a preview for a
  /// media that left the visible page is dropped instead of resurrected.
  final bool Function(String mediaId) holds;
  final bool Function(String mediaId) isOnTimeline;
  final void Function() rebuildEditor;
  final ValueNotifier<Map<String, ProjectMediaPreview>> updates;

  /// Returns the stored preview, or null when the update was ignored.
  ProjectMediaPreview? publish(
    Object session,
    String mediaId,
    ProjectMediaPreview preview, {
    required void Function(ProjectMediaPreview preview) store,
  }) {
    if (!isMounted() || !isCurrentSession(session) || !holds(mediaId)) {
      return null;
    }
    store(preview);
    // The timeline reads filmstrips from the preview map, so media that sits on
    // it needs the editor rebuild as well; the pane notifier keeps the library
    // repainting on its own either way.
    if (isOnTimeline(mediaId)) {
      rebuildEditor();
    }
    updates.value = Map<String, ProjectMediaPreview>.unmodifiable({
      ...updates.value,
      mediaId: preview,
    });
    return preview;
  }
}
