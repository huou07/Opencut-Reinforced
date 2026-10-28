import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/editor_settings.dart';

void main() {
  test('the inspector starts hidden and the preference round-trips', () async {
    final directory = await Directory.systemTemp.createTemp(
      'or-editor-settings-',
    );
    addTearDown(() => directory.delete(recursive: true));
    final store = EditorSettingsStore(supportDirectory: () async => directory);

    expect((await store.load()).showInspector, isFalse);
    await store.save(const EditorSettings(showInspector: true));
    expect((await store.load()).showInspector, isTrue);
    await store.save(const EditorSettings());
    expect((await store.load()).showInspector, isFalse);
  });

  test('a corrupt settings file falls back to the default', () async {
    final directory = await Directory.systemTemp.createTemp('or-editor-bad-');
    addTearDown(() => directory.delete(recursive: true));
    await File('${directory.path}${Platform.pathSeparator}editor-settings.json')
        .writeAsString('not json');
    final store = EditorSettingsStore(supportDirectory: () async => directory);
    expect((await store.load()).showInspector, isFalse);
  });
}
