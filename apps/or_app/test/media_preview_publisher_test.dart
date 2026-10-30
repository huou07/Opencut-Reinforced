import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/production/media_preview_publisher.dart';

const _session = Object();

const _preview = ProjectMediaPreview(
  kind: ProjectMediaArtifactKind.thumbnail,
  state: ProjectMediaArtifactRequestState.ready,
  cacheKey: 'cache-1',
);

void main() {
  late List<ProjectMediaPreview> stored;
  late int rebuilds;
  late ValueNotifier<Map<String, ProjectMediaPreview>> updates;

  MediaPreviewPublisher build({
    bool mounted = true,
    bool current = true,
    bool holds = true,
    bool onTimeline = false,
  }) => MediaPreviewPublisher(
    isMounted: () => mounted,
    isCurrentSession: (_) => current,
    holds: (_) => holds,
    isOnTimeline: (_) => onTimeline,
    rebuildEditor: () => rebuilds += 1,
    updates: updates,
  );

  setUp(() {
    stored = [];
    rebuilds = 0;
    updates = ValueNotifier(const {});
  });

  test('an off-timeline preview repaints only through the pane notifier', () {
    final result = build().publish(
      _session,
      'media-1',
      _preview,
      store: stored.add,
    );
    expect(result, _preview);
    expect(stored, hasLength(1));
    expect(rebuilds, 0);
    expect(updates.value['media-1'], _preview);
  });

  test('an on-timeline preview also rebuilds the editor', () {
    build(onTimeline: true)
        .publish(_session, 'media-1', _preview, store: stored.add);
    expect(rebuilds, 1);
    expect(updates.value['media-1'], _preview);
  });

  test('a stale session, a hidden media or an unmounted shell is ignored', () {
    for (final publisher in [
      build(current: false),
      build(holds: false),
      build(mounted: false),
    ]) {
      expect(
        publisher.publish(_session, 'media-1', _preview, store: stored.add),
        isNull,
      );
    }
    expect(stored, isEmpty);
    expect(rebuilds, 0);
    expect(updates.value, isEmpty);
  });
}
