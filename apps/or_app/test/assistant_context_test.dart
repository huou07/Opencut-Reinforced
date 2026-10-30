import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/assistant/assistant_controller.dart';
import 'package:or_app/assistant/assistant_models.dart';

const _clip = AssistantEditorContext(
  clipId: 'clip-1',
  clipLabel: 'video clip',
  trackKind: 'video',
  playheadSeconds: 1.5,
);

void main() {
  test('a context compares by what it says', () {
    expect(
      const AssistantEditorContext(
        clipId: 'clip-1',
        clipLabel: 'video clip',
        trackKind: 'video',
        playheadSeconds: 1.5,
      ),
      _clip,
    );
    expect(_clip == const AssistantEditorContext(clipId: 'clip-1'), isFalse);
    expect(
      const AssistantEditorContext(playheadSeconds: 1.5),
      isNot(const AssistantEditorContext(playheadSeconds: 1.6)),
    );
  });

  test('the same context twice is not published twice', () {
    final controller = AssistantController();
    addTearDown(controller.dispose);
    var notified = 0;
    controller.addListener(() => notified++);

    controller.setContext(_clip);
    controller.setContext(_clip);
    controller.setContext(
      const AssistantEditorContext(
        clipId: 'clip-1',
        clipLabel: 'video clip',
        trackKind: 'video',
        playheadSeconds: 1.5,
      ),
    );

    expect(notified, 1);
    expect(controller.context, _clip);
  });

  test('playback publishes a quarter second of playhead, not every frame', () {
    final tick = const Duration(microseconds: 16667);
    var elapsed = Duration.zero;
    AssistantEditorContext? current;
    var publishes = 0;
    for (var frame = 0; frame < 240; frame++) {
      final next = AssistantEditorContext(
        playheadSeconds: frame * 16.667 / 1000,
      );
      final publish = assistantContextToPublish(
        current: current,
        next: next,
        streaming: true,
        sinceLastPublish: elapsed,
      );
      if (publish != null) {
        current = publish;
        elapsed = Duration.zero;
        publishes++;
      } else {
        elapsed += tick;
      }
    }
    expect(publishes, lessThanOrEqualTo(17));
    expect(publishes, greaterThan(10));
  });

  test('a selection change is never delayed by the playhead budget', () {
    final published = assistantContextToPublish(
      current: const AssistantEditorContext(playheadSeconds: 1.0),
      next: const AssistantEditorContext(
        clipId: 'clip-2',
        playheadSeconds: 1.01,
      ),
      streaming: true,
      sinceLastPublish: const Duration(milliseconds: 5),
    );
    expect(published?.clipId, 'clip-2');

    expect(
      assistantContextToPublish(
        current: null,
        next: const AssistantEditorContext(playheadSeconds: 3),
        streaming: true,
        sinceLastPublish: Duration.zero,
      )?.playheadSeconds,
      3,
    );
    expect(
      assistantContextToPublish(
        current: const AssistantEditorContext(playheadSeconds: 1.0),
        next: const AssistantEditorContext(playheadSeconds: 1.004),
        streaming: true,
        sinceLastPublish: const Duration(milliseconds: 240),
      ),
      isNull,
    );
    expect(
      assistantContextToPublish(
        current: const AssistantEditorContext(playheadSeconds: 1.0),
        next: const AssistantEditorContext(playheadSeconds: 1.26),
        streaming: true,
        sinceLastPublish: const Duration(milliseconds: 250),
      )?.playheadSeconds,
      1.26,
    );
  });

  test('a paused seek reaches the panel at once', () {
    expect(
      assistantContextToPublish(
        current: const AssistantEditorContext(playheadSeconds: 1.0),
        next: const AssistantEditorContext(playheadSeconds: 1.004),
        streaming: false,
        sinceLastPublish: Duration.zero,
      )?.playheadSeconds,
      1.004,
    );
  });
}
