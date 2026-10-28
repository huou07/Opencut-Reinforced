import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/music_prompt.dart';
import 'package:or_app/production/music_service.dart';

void main() {
  group('MusicPromptInterpreter', () {
    test('reads a Vietnamese calm request with no drums', () {
      final plan = MusicPromptInterpreter.interpret(
        'nhạc nền nhẹ nhàng thư giãn 90 giây, không trống',
        fallbackTempo: 120,
        fallbackSeconds: 30,
      );

      expect(plan.moodId, 'calm');
      expect(plan.tempo, 62);
      expect(plan.seconds, 90);
      expect(plan.drumsScale, 0);
      expect(plan.summary, contains('90 s'));
    });

    test('reads explicit bpm, minutes and seconds', () {
      final plan = MusicPromptInterpreter.interpret(
        'upbeat vlog 120 bpm 1 phút 30 giây',
        fallbackTempo: 60,
        fallbackSeconds: 60,
      );

      expect(plan.moodId, 'uplifting');
      expect(plan.tempo, 120);
      expect(plan.seconds, 90);
      expect(plan.drumsScale, greaterThan(0));
    });

    test('a tense request wins over a quiet word in the same sentence', () {
      final plan = MusicPromptInterpreter.interpret(
        'kịch tính, căng thẳng cho trailer, nhẹ nhàng ở đoạn cuối',
        fallbackTempo: 72,
        fallbackSeconds: 60,
      );

      expect(plan.moodId, 'tense');
    });

    test('unrelated text keeps the caller defaults and says so', () {
      final plan = MusicPromptInterpreter.interpret(
        'make it good please',
        fallbackTempo: 84,
        fallbackSeconds: 45,
        fallbackMoodId: 'warm',
      );

      expect(plan.moodId, 'warm');
      expect(plan.tempo, 84);
      expect(plan.seconds, 45);
      expect(plan.notes, isNotEmpty);
    });

    test(
      'the same prompt and variation are reproducible; a new variation is not',
      () {
        final first = MusicPromptInterpreter.interpret(
          'piano 70 bpm 60 giây',
          fallbackTempo: 72,
          fallbackSeconds: 60,
        );
        final again = MusicPromptInterpreter.interpret(
          'piano 70 bpm 60 giây',
          fallbackTempo: 72,
          fallbackSeconds: 60,
        );
        final varied = MusicPromptInterpreter.interpret(
          'piano 70 bpm 60 giây',
          fallbackTempo: 72,
          fallbackSeconds: 60,
          variation: 1,
        );

        expect(first.seed, again.seed);
        expect(first.seed, isNot(varied.seed));
      },
    );

    test('drums can be asked for and remain a mix multiplier', () {
      final plan = MusicPromptInterpreter.interpret(
        'trống mạnh, sôi động 130 bpm',
        fallbackTempo: 72,
        fallbackSeconds: 60,
      );

      expect(plan.moodId, 'uplifting');
      expect(plan.drumsScale, greaterThan(1));
    });
  });

  group('MusicService with a prompt plan', () {
    test('writes a deterministic bed of the requested length', () async {
      final directory = await Directory.systemTemp.createTemp('or-music-test-');
      addTearDown(() => directory.delete(recursive: true));
      final plan = MusicPromptInterpreter.interpret(
        'nhạc nền nhẹ nhàng 4 giây, không trống',
        fallbackTempo: 72,
        fallbackSeconds: 60,
      );
      final service = MusicService();

      final first = await service.generate(
        moodId: 'calm',
        tempo: plan.tempo,
        seconds: plan.seconds,
        directory: directory,
        plan: plan,
      );
      final bytes = File(first.path).readAsBytesSync();
      final second = await service.generate(
        moodId: 'calm',
        tempo: plan.tempo,
        seconds: plan.seconds,
        directory: directory,
        plan: plan,
      );

      expect(first.durationMs, 4000);
      expect(bytes.length, greaterThan(44));
      expect(File(second.path).readAsBytesSync(), bytes);
    });

    test('a drumless plan differs from a drummed one', () async {
      final quietDirectory = await Directory.systemTemp.createTemp(
        'or-music-quiet-',
      );
      final loudDirectory = await Directory.systemTemp.createTemp(
        'or-music-loud-',
      );
      addTearDown(() => quietDirectory.delete(recursive: true));
      addTearDown(() => loudDirectory.delete(recursive: true));
      final service = MusicService();
      final quiet = MusicPromptInterpreter.interpret(
        'nhẹ nhàng 4 giây, không trống',
        fallbackTempo: 72,
        fallbackSeconds: 60,
      );
      final loud = MusicPromptInterpreter.interpret(
        'sôi động 4 giây, trống mạnh',
        fallbackTempo: 72,
        fallbackSeconds: 60,
      );

      final quietBed = await service.generate(
        moodId: quiet.moodId,
        tempo: quiet.tempo,
        seconds: quiet.seconds,
        directory: quietDirectory,
        plan: quiet,
      );
      final loudBed = await service.generate(
        moodId: loud.moodId,
        tempo: loud.tempo,
        seconds: loud.seconds,
        directory: loudDirectory,
        plan: loud,
      );

      expect(
        File(quietBed.path).readAsBytesSync(),
        isNot(File(loudBed.path).readAsBytesSync()),
      );
    });
  });
}
