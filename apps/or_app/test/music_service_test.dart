import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/music_service.dart';

void main() {
  test('generates a deterministic stereo PCM bed with a quiet peak', () async {
    final directory = await Directory.systemTemp.createTemp('or-music-');
    try {
      final service = MusicService();
      final bed = await service.generate(
        moodId: 'warm',
        tempo: 88,
        seconds: 4,
        directory: directory,
      );
      expect(bed.durationMs, 4000);
      expect(bed.bytes, 44 + 4 * 44100 * 4);
      final file = File(bed.path);
      expect(file.existsSync(), isTrue);
      final bytes = await file.readAsBytes();
      expect(String.fromCharCodes(bytes.sublist(0, 4)), 'RIFF');
      expect(String.fromCharCodes(bytes.sublist(8, 12)), 'WAVE');
      final header = ByteData.sublistView(bytes);
      expect(header.getUint16(22, Endian.little), 2);
      expect(header.getUint32(24, Endian.little), 44100);
      expect(header.getUint16(34, Endian.little), 16);
      var peak = 0;
      var nonzero = 0;
      for (var offset = 44; offset < 44 + 44100; offset += 2) {
        final sample = header.getInt16(offset, Endian.little).abs();
        if (sample != 0) nonzero += 1;
        if (sample > peak) peak = sample;
      }
      expect(nonzero, greaterThan(1000));
      expect(peak, lessThanOrEqualTo(32767));
    } finally {
      await directory.delete(recursive: true);
    }
  });

  test('refuses an unknown mood without writing a file', () async {
    final directory = await Directory.systemTemp.createTemp('or-music-bad-');
    try {
      await expectLater(
        MusicService().generate(
          moodId: 'nonsense',
          tempo: 72,
          seconds: 4,
          directory: directory,
        ),
        throwsA(isA<MusicException>()),
      );
      expect(directory.listSync(), isEmpty);
    } finally {
      await directory.delete(recursive: true);
    }
  });
}
