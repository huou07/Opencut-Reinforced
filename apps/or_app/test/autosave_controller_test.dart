import 'dart:async';

import 'package:fake_async/fake_async.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/shell/autosave_controller.dart';

void main() {
  test('a tick reports progress then the outcome', () async {
    final statuses = <AutosaveStatus>[];
    final controller = AutosaveController(
      attempt: () async => true,
      onChanged: statuses.add,
    );

    await controller.tick();

    expect(
      statuses.map((status) => status.label),
      ['Autosaving recovery', 'Recovery saved'],
    );
    expect(controller.status.failed, isFalse);
  });

  test('a failed checkpoint marks the status as failed', () async {
    final controller = AutosaveController(
      attempt: () async => false,
      onChanged: (_) {},
    );

    await controller.tick();

    expect(controller.status.label, 'Autosave needs attention');
    expect(controller.status.failed, isTrue);
  });

  test('a status from a swapped session is ignored', () async {
    final statuses = <AutosaveStatus>[];
    final controller = AutosaveController(
      attempt: () async => null,
      onChanged: statuses.add,
    );

    await controller.tick();

    expect(statuses.map((status) => status.label), ['Autosaving recovery']);
    expect(controller.status.label, 'Autosaving recovery');
    expect(controller.status.failed, isFalse);
  });

  test('an exception reports attention instead of escaping', () async {
    final controller = AutosaveController(
      attempt: () async => throw StateError('gateway gone'),
      onChanged: (_) {},
    );

    await controller.tick();

    expect(controller.status.failed, isTrue);
    expect(controller.status.label, 'Autosave needs attention');
  });

  test('ticks do not overlap while an attempt is in flight', () {
    fakeAsync((async) {
      var calls = 0;
      final pending = Completer<bool>();
      final controller = AutosaveController(
        interval: const Duration(seconds: 1),
        attempt: () {
          calls++;
          return pending.future;
        },
        onChanged: (_) {},
      );

      controller.start();
      async.elapse(const Duration(seconds: 3));
      expect(calls, 1, reason: 'the first attempt never resolved');

      pending.complete(true);
      async.flushMicrotasks();
      async.elapse(const Duration(seconds: 1));
      expect(calls, 2);
      controller.stop();
    });
  });

  test('clear forgets the last status', () async {
    final controller = AutosaveController(
      attempt: () async => false,
      onChanged: (_) {},
    );
    await controller.tick();
    expect(controller.status.failed, isTrue);

    controller.clear();

    expect(controller.status.label, isNull);
    expect(controller.status.failed, isFalse);
  });
}
