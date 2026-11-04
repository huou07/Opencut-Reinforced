import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/welcome_store.dart';

void main() {
  test('the first-run panel starts shown and the dismissal round-trips', () async {
    final directory = await Directory.systemTemp.createTemp('or-welcome-');
    addTearDown(() => directory.delete(recursive: true));
    final store = WelcomeStore(supportDirectory: () async => directory);

    expect((await store.load()).dismissed, isFalse);
    await store.save(const WelcomeState(dismissed: true));
    expect((await store.load()).dismissed, isTrue);
  });

  test('a corrupt file shows the panel again rather than hiding it', () async {
    final directory = await Directory.systemTemp.createTemp('or-welcome-bad-');
    addTearDown(() => directory.delete(recursive: true));
    await File('${directory.path}${Platform.pathSeparator}welcome.json')
        .writeAsString('{"version": 1, ');
    final store = WelcomeStore(supportDirectory: () async => directory);
    expect((await store.load()).dismissed, isFalse);
  });
}
