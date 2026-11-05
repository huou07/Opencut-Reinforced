import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/editor_settings.dart';
import 'package:or_app/screens/editor_guided_bar.dart';

void main() {
  group('guided next step', () {
    test('an empty project asks for media first', () {
      final step = guidedNextStep(
        hasMedia: false,
        clipCount: 0,
        hasText: false,
        hasAudio: false,
        hasExported: false,
      );
      expect(step?.kind, GuidedStepKind.importMedia);
      expect(step?.actionLabel, 'Import media');
    });

    test('imported but unused media asks for a clip', () {
      expect(
        guidedNextStep(
          hasMedia: true,
          clipCount: 0,
          hasText: false,
          hasAudio: false,
          hasExported: false,
        )?.kind,
        GuidedStepKind.addFirstClip,
      );
    });

    test('a clip without a title asks for one', () {
      expect(
        guidedNextStep(
          hasMedia: true,
          clipCount: 1,
          hasText: false,
          hasAudio: false,
          hasExported: false,
        )?.kind,
        GuidedStepKind.addTitle,
      );
    });

    test('a title without sound asks for sound', () {
      expect(
        guidedNextStep(
          hasMedia: true,
          clipCount: 1,
          hasText: true,
          hasAudio: false,
          hasExported: false,
        )?.kind,
        GuidedStepKind.addAudio,
      );
    });

    test('a titled edit with sound asks for the export', () {
      expect(
        guidedNextStep(
          hasMedia: true,
          clipCount: 1,
          hasText: true,
          hasAudio: true,
          hasExported: false,
        )?.kind,
        GuidedStepKind.exportVideo,
      );
    });

    test('a finished export stops the row', () {
      expect(
        guidedNextStep(
          hasMedia: true,
          clipCount: 1,
          hasText: true,
          hasAudio: true,
          hasExported: true,
        ),
        isNull,
      );
    });
  });

  group('guided preferences', () {
    test('guided mode and guides are on by default, guides off', () async {
      final directory = await Directory.systemTemp.createTemp('or-guided-');
      addTearDown(() => directory.delete(recursive: true));
      final store = EditorSettingsStore(
        supportDirectory: () async => directory,
      );
      final settings = await store.load();
      expect(settings.simpleMode, isTrue);
      expect(settings.showGuides, isFalse);
      expect(settings.tipsDismissed, isFalse);
    });

    test('a version 1 file upgrades into guided mode', () async {
      final directory = await Directory.systemTemp.createTemp('or-guided-v1-');
      addTearDown(() => directory.delete(recursive: true));
      await File(
        '${directory.path}${Platform.pathSeparator}editor-settings.json',
      ).writeAsString('{"version":1,"show_inspector":true}');
      final store = EditorSettingsStore(
        supportDirectory: () async => directory,
      );
      final settings = await store.load();
      expect(settings.showInspector, isTrue);
      expect(settings.simpleMode, isTrue);
      expect(settings.showGuides, isFalse);
    });

    test('every guided preference round-trips', () async {
      final directory = await Directory.systemTemp.createTemp('or-guided-rt-');
      addTearDown(() => directory.delete(recursive: true));
      final store = EditorSettingsStore(
        supportDirectory: () async => directory,
      );
      await store.save(
        const EditorSettings(
          simpleMode: false,
          showGuides: true,
          tipsDismissed: true,
        ),
      );
      final settings = await store.load();
      expect(settings.simpleMode, isFalse);
      expect(settings.showGuides, isTrue);
      expect(settings.tipsDismissed, isTrue);
    });
  });
}
