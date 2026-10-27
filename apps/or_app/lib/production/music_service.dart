import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:path_provider/path_provider.dart';

/// One selectable background-music character.
class MusicMood {
  const MusicMood({
    required this.id,
    required this.label,
    required this.chords,
    required this.arp,
    required this.drums,
    required this.padGain,
    required this.arpGain,
    required this.brightness,
  });

  final String id;
  final String label;

  /// MIDI note sets, one chord per bar.
  final List<List<int>> chords;

  /// Index pattern into [chords] for the arpeggio.
  final List<int> arp;
  final double drums;
  final double padGain;
  final double arpGain;
  final double brightness;
}

const musicMoods = <MusicMood>[
  MusicMood(
    id: 'calm',
    label: 'Calm',
    chords: [
      [57, 60, 64, 67],
      [53, 57, 60, 64],
      [48, 52, 55, 59],
      [55, 59, 62, 64],
    ],
    arp: [0, 1, 2, 3, 2, 1, 3, 2],
    drums: 0.2,
    padGain: 0.055,
    arpGain: 0.05,
    brightness: 0.5,
  ),
  MusicMood(
    id: 'warm',
    label: 'Warm',
    chords: [
      [48, 52, 55, 59],
      [55, 59, 62, 67],
      [57, 60, 64, 67],
      [53, 57, 60, 64],
    ],
    arp: [0, 2, 1, 3, 1, 2, 3, 2],
    drums: 0.35,
    padGain: 0.06,
    arpGain: 0.055,
    brightness: 0.6,
  ),
  MusicMood(
    id: 'uplifting',
    label: 'Uplifting',
    chords: [
      [48, 52, 55, 60],
      [55, 59, 62, 67],
      [57, 60, 64, 69],
      [53, 57, 60, 65],
    ],
    arp: [0, 1, 2, 3, 2, 1, 2, 3],
    drums: 0.55,
    padGain: 0.05,
    arpGain: 0.065,
    brightness: 0.8,
  ),
  MusicMood(
    id: 'tense',
    label: 'Tense',
    chords: [
      [50, 53, 57, 60],
      [46, 50, 53, 58],
      [43, 46, 50, 55],
      [45, 49, 52, 57],
    ],
    arp: [0, 2, 3, 1, 0, 3, 2, 1],
    drums: 0.4,
    padGain: 0.06,
    arpGain: 0.045,
    brightness: 0.35,
  ),
  MusicMood(
    id: 'minimal',
    label: 'Minimal',
    chords: [
      [57, 60, 64, 69],
      [57, 60, 64, 69],
      [53, 57, 60, 65],
      [53, 57, 60, 65],
    ],
    arp: [0, 2, 1, 3, 2, 0, 1, 3],
    drums: 0.05,
    padGain: 0.07,
    arpGain: 0.035,
    brightness: 0.4,
  ),
];

class MusicBed {
  const MusicBed({
    required this.path,
    required this.durationMs,
    required this.bytes,
    required this.mood,
    required this.tempo,
  });

  final String path;
  final int durationMs;
  final int bytes;
  final String mood;
  final int tempo;
}

class MusicException implements Exception {
  const MusicException(this.message);

  final String message;

  @override
  String toString() => message;
}

/// Generates a quiet, royalty-free background bed offline.
///
/// The synthesis is deterministic for a seed, uses no sample library and makes
/// no network call. The level is deliberately low so the clip sits under speech;
/// the inspector's clip gain remains the final control.
class MusicService {
  static const int sampleRate = 44100;

  Future<Directory> outputDirectory() async {
    final support = await getApplicationSupportDirectory();
    return Directory('${support.path}/music/${DateTime.now().millisecondsSinceEpoch}');
  }

  Future<MusicBed> generate({
    required String moodId,
    required int tempo,
    required int seconds,
    required Directory directory,
    int seed = 1,
    double levelDb = -9,
  }) async {
    MusicMood? mood;
    for (final candidate in musicMoods) {
      if (candidate.id == moodId) {
        mood = candidate;
        break;
      }
    }
    if (mood == null) {
      throw const MusicException('Choose a known music mood.');
    }
    if (tempo < 40 || tempo > 180) {
      throw const MusicException('The tempo must be 40-180 bpm.');
    }
    if (seconds < 4 || seconds > 600) {
      throw const MusicException('The bed must be 4-600 seconds.');
    }
    if (levelDb < -30 || levelDb > 0) {
      throw const MusicException('The level must be -30-0 dBFS.');
    }
    final frames = (seconds * sampleRate);
    final left = Float32List(frames);
    final right = Float32List(frames);
    final random = Random(seed);
    final barSeconds = (60 / tempo) * 4;
    final barFrames = (barSeconds * sampleRate).round();
    final bars = (frames / barFrames).ceil();
    final delayFrames = (sampleRate * 0.375).round();

    for (var bar = 0; bar < bars; bar += 1) {
      final barStart = bar * barFrames;
      final chord = mood.chords[bar % mood.chords.length];
      for (final note in chord) {
        for (final octave in [0, 12]) {
          _addSine(
            left,
            right,
            barStart,
            barFrames,
            _frequency(note + octave) * (1 + (random.nextDouble() - 0.5) * 0.0012),
            mood.padGain / chord.length,
            0.5 + (random.nextDouble() - 0.5) * 0.2,
            (time) => _padEnvelope(time, barSeconds),
          );
        }
      }
      _addSine(
        left,
        right,
        barStart,
        barFrames,
        _frequency(chord.first - 12),
        0.11,
        0.5,
        (time) => _bassEnvelope(time, barSeconds),
      );
      final stepFrames = barFrames ~/ mood.arp.length;
      for (var step = 0; step < mood.arp.length; step += 1) {
        final note = chord[mood.arp[step]] + 12;
        final start = barStart + step * stepFrames;
        final pan = 0.35 + (step % 2) * 0.3 + (random.nextDouble() - 0.5) * 0.1;
        final amplitude = mood.arpGain * (0.8 + random.nextDouble() * 0.4);
        final noteFrames = min((sampleRate * 0.5).round(), barFrames);
        _addPluck(left, right, start, noteFrames, _frequency(note), amplitude, pan);
        _addPluck(
          left,
          right,
          start + delayFrames,
          noteFrames,
          _frequency(note),
          amplitude * 0.28,
          1 - pan,
        );
      }
      _addSine(
        left,
        right,
        barStart,
        barFrames,
        _frequency(chord.last + 24),
        0.012 * mood.brightness,
        0.5,
        (time) =>
            _padEnvelope(time, barSeconds) *
            (0.6 + 0.4 * sin(2 * pi * 0.15 * time)),
      );
      if (mood.drums > 0) {
        _kick(left, right, barStart, 0.2 * mood.drums);
        _kick(left, right, barStart + barFrames ~/ 2, 0.12 * mood.drums);
        for (final fraction in [0.25, 0.75]) {
          _hat(
            left,
            right,
            barStart + (barFrames * fraction).round(),
            0.05 * mood.drums,
            random,
          );
        }
      }
    }

    var peak = 0.0;
    for (var index = 0; index < frames; index += 1) {
      peak = max(peak, max(left[index].abs(), right[index].abs()));
    }
    final target = pow(10, levelDb / 20).toDouble();
    final scale = peak > 0 ? target / peak : 1.0;
    final fadeFrames = min((sampleRate * 1.2).round(), frames ~/ 4);
    for (var index = 0; index < frames; index += 1) {
      var envelope = 1.0;
      if (index < fadeFrames) envelope = index / fadeFrames;
      final tail = frames - index;
      if (tail < fadeFrames) envelope = min(envelope, tail / fadeFrames);
      left[index] = (left[index] * scale * envelope).clamp(-1.0, 1.0).toDouble();
      right[index] = (right[index] * scale * envelope).clamp(-1.0, 1.0).toDouble();
    }

    await directory.create(recursive: true);
    final file = File('${directory.path}/bed-${mood.id}-$tempo.wav');
    final bytes = _wavBytes(left, right);
    await file.writeAsBytes(bytes, flush: true);
    return MusicBed(
      path: file.path,
      durationMs: (frames * 1000) ~/ sampleRate,
      bytes: bytes.length,
      mood: mood.id,
      tempo: tempo,
    );
  }

  static double _frequency(int midi) => 440 * pow(2, (midi - 69) / 12).toDouble();

  static double _padEnvelope(double time, double duration) {
    final attack = min(0.9, duration * 0.35);
    final release = min(1.1, duration * 0.35);
    if (time < attack) return time / attack;
    if (time > duration - release) {
      return max(0, (duration - time) / release);
    }
    return 1;
  }

  static double _bassEnvelope(double time, double duration) {
    final attack = min(0.08, duration * 0.1);
    final release = min(0.35, duration * 0.25);
    if (time < attack) return time / attack;
    if (time > duration - release) {
      return max(0, (duration - time) / release);
    }
    return 0.8;
  }

  static void _addSine(
    Float32List left,
    Float32List right,
    int start,
    int length,
    double frequency,
    double amplitude,
    double pan,
    double Function(double time) envelope,
  ) {
    for (var index = 0; index < length; index += 1) {
      final position = start + index;
      if (position >= left.length) break;
      final time = index / sampleRate;
      final gain = envelope(time) * amplitude;
      if (gain <= 0) continue;
      final sample = sin(2 * pi * frequency * time) * gain;
      left[position] += sample * (1 - pan);
      right[position] += sample * pan;
    }
  }

  static void _addPluck(
    Float32List left,
    Float32List right,
    int start,
    int length,
    double frequency,
    double amplitude,
    double pan,
  ) {
    for (var index = 0; index < length; index += 1) {
      final position = start + index;
      if (position >= left.length) break;
      final time = index / sampleRate;
      final decay = exp(-time * 5.5);
      final shape =
          sin(2 * pi * frequency * time) * 0.7 +
          sin(2 * pi * frequency * 2 * time) * 0.22 +
          sin(2 * pi * frequency * 3 * time) * 0.08;
      final sample = shape * amplitude * decay;
      left[position] += sample * (1 - pan);
      right[position] += sample * pan;
    }
  }

  static void _kick(Float32List left, Float32List right, int start, double amplitude) {
    final length = (sampleRate * 0.22).round();
    for (var index = 0; index < length; index += 1) {
      final position = start + index;
      if (position >= left.length) break;
      final time = index / sampleRate;
      final frequency = 120 * exp(-time * 38) + 42;
      final envelope = exp(-time * 16);
      final sample = sin(2 * pi * frequency * time) * envelope * amplitude;
      left[position] += sample;
      right[position] += sample;
    }
  }

  static void _hat(
    Float32List left,
    Float32List right,
    int start,
    double amplitude,
    Random random,
  ) {
    final length = (sampleRate * 0.045).round();
    var previous = 0.0;
    for (var index = 0; index < length; index += 1) {
      final position = start + index;
      if (position >= left.length) break;
      final time = index / sampleRate;
      final noise = random.nextDouble() * 2 - 1;
      final filtered = noise - previous * 0.85;
      previous = noise;
      final sample = filtered * exp(-time * 70) * amplitude;
      left[position] += sample;
      right[position] += sample;
    }
  }

  static Uint8List _wavBytes(Float32List left, Float32List right) {
    final frames = left.length;
    final dataBytes = frames * 4;
    final bytes = Uint8List(44 + dataBytes);
    final header = ByteData.sublistView(bytes);
    void ascii(int offset, String value) {
      for (var index = 0; index < value.length; index += 1) {
        bytes[offset + index] = value.codeUnitAt(index);
      }
    }

    ascii(0, 'RIFF');
    header.setUint32(4, 36 + dataBytes, Endian.little);
    ascii(8, 'WAVE');
    ascii(12, 'fmt ');
    header.setUint32(16, 16, Endian.little);
    header.setUint16(20, 1, Endian.little);
    header.setUint16(22, 2, Endian.little);
    header.setUint32(24, sampleRate, Endian.little);
    header.setUint32(28, sampleRate * 4, Endian.little);
    header.setUint16(32, 4, Endian.little);
    header.setUint16(34, 16, Endian.little);
    ascii(36, 'data');
    header.setUint32(40, dataBytes, Endian.little);
    var offset = 44;
    for (var index = 0; index < frames; index += 1) {
      header.setInt16(offset, (left[index] * 32767).round(), Endian.little);
      header.setInt16(offset + 2, (right[index] * 32767).round(), Endian.little);
      offset += 4;
    }
    return bytes;
  }
}
