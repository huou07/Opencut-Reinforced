import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/voiceover_service.dart';

Uint8List pcmWav({
  required int sampleRate,
  required int channels,
  required int milliseconds,
}) {
  final frames = (sampleRate * milliseconds) ~/ 1000;
  final dataBytes = frames * channels * 2;
  final bytes = BytesBuilder();
  void ascii(String value) => bytes.add(value.codeUnits);
  void u32(int value) => bytes.add([
    value & 0xFF,
    (value >> 8) & 0xFF,
    (value >> 16) & 0xFF,
    (value >> 24) & 0xFF,
  ]);
  void u16(int value) => bytes.add([value & 0xFF, (value >> 8) & 0xFF]);

  ascii('RIFF');
  u32(36 + dataBytes);
  ascii('WAVE');
  ascii('fmt ');
  u32(16);
  u16(1);
  u16(channels);
  u32(sampleRate);
  u32(sampleRate * channels * 2);
  u16(channels * 2);
  u16(16);
  ascii('data');
  u32(dataBytes);
  bytes.add(Uint8List(dataBytes));
  return bytes.takeBytes();
}

void main() {
  test('the preview args keep the chosen rate and cap the sample length', () {
    final args = voiceoverPreviewArguments(
      voice: 'Linh',
      rate: 240,
      text: 'a' * 400,
      outputPath: '/tmp/preview.wav',
    );
    expect(args.sublist(0, 2), ['-v', 'Linh']);
    expect(args[args.indexOf('-r') + 1], '240');
    expect(args.last.length, 240);

    final defaultVoice = voiceoverPreviewArguments(
      voice: 'default',
      rate: 900,
      text: 'Xin chào',
      outputPath: '/tmp/preview.wav',
    );
    expect(defaultVoice.contains('-v'), isFalse);
    expect(defaultVoice[defaultVoice.indexOf('-r') + 1], '400');
  });

  test('reads the exact PCM frame count and ignores odd chunk padding', () {
    final wav = pcmWav(sampleRate: 44100, channels: 1, milliseconds: 1500);
    expect(wavDurationMsFromBytes(wav), 1500);
  });

  test('refuses a file that is not a PCM WAV', () {
    expect(
      () => wavDurationMsFromBytes(Uint8List.fromList([1, 2, 3, 4])),
      throwsA(isA<VoiceoverException>()),
    );
  });

  test('an unreadable voice table fails instead of listing nothing', () async {
    final directory = await Directory.systemTemp.createTemp('or_say_empty');
    addTearDown(() => directory.delete(recursive: true));
    final script = File('${directory.path}/empty_say.sh');
    await script.writeAsString('#!/bin/sh\necho "not a voice row"\n');
    await Process.run('chmod', ['+x', script.path]);

    await expectLater(
      VoiceoverService(sayExecutable: script.path).listVoices(),
      throwsA(
        isA<VoiceoverException>().having(
          (error) => error.message,
          'message',
          contains('say printed'),
        ),
      ),
    );
  });

  test('a system voice that never answers fails instead of hanging', () async {
    final directory = await Directory.systemTemp.createTemp('or_say_timeout');
    addTearDown(() => directory.delete(recursive: true));
    final script = File('${directory.path}/slow_say.sh');
    await script.writeAsString('#!/bin/sh\nsleep 5\n');
    await Process.run('chmod', ['+x', script.path]);

    final service = VoiceoverService(
      sayExecutable: script.path,
      processTimeout: const Duration(milliseconds: 80),
    );
    await expectLater(
      service.preview(
        voice: 'Daniel',
        rate: 180,
        text: 'Hello.',
        directory: directory,
      ),
      throwsA(
        isA<VoiceoverException>().having(
          (error) => error.message,
          'message',
          contains('did not speak the preview in time'),
        ),
      ),
    );
  });

  test('lists the Vietnamese voice with a short display label', () async {
    final voices = await VoiceoverService().listVoices();
    final vietnamese = voices.where((voice) => voice.locale == 'vi_VN');
    expect(vietnamese, isNotEmpty);
    expect(vietnamese.first.displayName, 'Linh');
    expect(vietnamese.first.label, 'Linh (vi_VN)');
  }, skip: !Platform.isMacOS);

  test(
    'the system voice produces a real WAV whose duration is measured',
    () async {
      final directory = await Directory.systemTemp.createTemp('or-voiceover-');
      try {
        final service = VoiceoverService();
        final lines = await service.synthesize(
          lines: const ['Hello from OR.'],
          voice: 'default',
          rate: 180,
          gapMs: 250,
          directory: directory,
        );
        expect(lines, hasLength(1));
        expect(lines.single.startMs, 0);
        expect(lines.single.durationMs, greaterThan(400));
        expect(File(lines.single.path).existsSync(), isTrue);
        expect(lines.single.endMs, lines.single.durationMs);
      } finally {
        await directory.delete(recursive: true);
      }
    },
    skip: !Platform.isMacOS,
  );
}
