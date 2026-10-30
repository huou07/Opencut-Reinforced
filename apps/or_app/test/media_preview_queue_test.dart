import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/production/media_preview_queue.dart';

ProjectMediaItem _item(String id) => ProjectMediaItem(
  mediaId: id,
  sourceUri: 'file:///tmp/$id.mp4',
  formatNames: const ['mp4'],
  duration: '1',
  videoDetails: 'h264 1920x1080',
  audioDetails: null,
);

void main() {
  late List<String> started;
  late List<ProjectMediaItem> visible;

  MediaPreviewQueue build({int maxActive = 2, bool current = true}) =>
      MediaPreviewQueue(
        isCurrentSession: (_) => current,
        visibleItems: () => visible,
        onStart: (_, item, _) => started.add(item.mediaId),
        maxActive: maxActive,
      );

  setUp(() {
    started = [];
    visible = [];
  });

  test('drains only the visible items up to the limit', () {
    visible = [_item('a'), _item('b'), _item('c')];
    final queue = build();
    queue.enqueue(visible);
    queue.drain(Object());
    expect(started, ['a', 'b']);
  });

  test('an item that left the page starts nothing', () {
    visible = [_item('a')];
    final queue = build();
    queue.enqueue([_item('a'), _item('gone')]);
    queue.drain(Object());
    expect(started, ['a']);
  });

  test('enqueue is idempotent while the item is still queued', () {
    visible = [_item('a'), _item('b')];
    final queue = build(maxActive: 1);
    final first = queue.enqueue(visible);
    final second = queue.enqueue(visible);
    expect(first, {'a', 'b'});
    expect(second, isEmpty);
    queue.drain(Object());
    expect(started, ['a']);
  });

  test('finishing a request drains the next item', () {
    visible = [_item('a'), _item('b'), _item('c')];
    final queue = build(maxActive: 1);
    queue.enqueue(visible);
    queue.drain(Object());
    expect(started, ['a']);
    queue.finishRequest(Object());
    expect(started, ['a', 'b']);
  });

  test('settling several tickets frees the same number of slots', () {
    visible = [_item('a'), _item('b'), _item('c')];
    final queue = build();
    queue.enqueue(visible);
    queue.drain(Object());
    expect(queue.activeRequests, 2);
    queue.settle(2, Object());
    expect(started, ['a', 'b', 'c']);
  });

  test('a stale session starts nothing and retry keeps one slot', () {
    visible = [_item('a'), _item('b')];
    final queue = build(current: false);
    queue.enqueue(visible);
    queue.drain(Object());
    expect(started, isEmpty);
    queue.reset();
    expect(queue.enqueue(visible), {'a', 'b'});
  });
}
