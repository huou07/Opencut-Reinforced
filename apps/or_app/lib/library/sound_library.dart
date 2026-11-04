import 'dart:math' as math;
import 'dart:typed_data';

/// The shape a generated sound follows.
///
/// The catalogue ships no audio files: every effect is rendered from these
/// shapes on demand, so the app stays asset-free, the download stays small, and
/// the same sound is produced on macOS, Linux and Windows bit for bit.
enum SoundShape { tick, click, pop, whoosh, riser, impact, chime }

/// One generated sound effect.
class SoundEffect {
  const SoundEffect({
    required this.id,
    required this.name,
    required this.description,
    required this.shape,
    required this.durationMilliseconds,
  });

  final String id;
  final String name;
  final String description;
  final SoundShape shape;
  final int durationMilliseconds;

  /// The file name used when the sound is written for import.
  String get fileName => 'or-sound-$id.wav';
}

/// Sound effects the library can write into a project.
///
/// Durations are short on purpose: an effect is a punctuation mark, and a long
/// one is music, which belongs in the music generator instead.
const List<SoundEffect> soundEffects = [
  SoundEffect(
    id: 'tick',
    name: 'Tick',
    description: 'A very short high blip for a cut or a countdown.',
    shape: SoundShape.tick,
    durationMilliseconds: 60,
  ),
  SoundEffect(
    id: 'click',
    name: 'Click',
    description: 'A dry transient for a button, a snap or a marker.',
    shape: SoundShape.click,
    durationMilliseconds: 40,
  ),
  SoundEffect(
    id: 'pop',
    name: 'Pop',
    description: 'A small pitch drop that lands a word or an on-screen item.',
    shape: SoundShape.pop,
    durationMilliseconds: 140,
  ),
  SoundEffect(
    id: 'whoosh',
    name: 'Whoosh',
    description: 'Noise sweeping past for a transition or a wipe.',
    shape: SoundShape.whoosh,
    durationMilliseconds: 700,
  ),
  SoundEffect(
    id: 'riser',
    name: 'Riser',
    description: 'A rising tone that builds into a reveal.',
    shape: SoundShape.riser,
    durationMilliseconds: 1400,
  ),
  SoundEffect(
    id: 'impact',
    name: 'Impact',
    description: 'Low thump under a hard cut or a logo reveal.',
    shape: SoundShape.impact,
    durationMilliseconds: 500,
  ),
  SoundEffect(
    id: 'chime',
    name: 'Chime',
    description: 'Two soft tones for a confirmation or an end card.',
    shape: SoundShape.chime,
    durationMilliseconds: 1000,
  ),
];

/// The effect with [id], or null when the catalogue no longer has it.
SoundEffect? soundEffectById(String id) {
  for (final effect in soundEffects) {
    if (effect.id == id) return effect;
  }
  return null;
}

const int _defaultSampleRate = 48000;

/// The loudest sample any effect may reach: -1 dBFS, leaving room for the sum
/// of an effect and a music bed on the same export.
const double _ceiling = 0.891;

/// Renders [effect] as a complete 16-bit mono PCM WAV file.
///
/// The bytes are a real WAV container: any importer that reads PCM reads it, and
/// the media pipeline probes it like any other source. Generation is
/// deterministic, so two machines produce identical files and a test can assert
/// on the header instead of on a golden binary.
Uint8List renderSoundWav(
  SoundEffect effect, {
  int sampleRate = _defaultSampleRate,
}) {
  final samples = _renderSamples(effect, sampleRate);
  const bytesPerSample = 2;
  final dataLength = samples.length * bytesPerSample;
  final out = ByteData(44 + dataLength);

  void writeAscii(int offset, String value) {
    for (var index = 0; index < value.length; index++) {
      out.setUint8(offset + index, value.codeUnitAt(index));
    }
  }

  writeAscii(0, 'RIFF');
  out.setUint32(4, 36 + dataLength, Endian.little);
  writeAscii(8, 'WAVE');
  writeAscii(12, 'fmt ');
  out.setUint32(16, 16, Endian.little); // PCM header size
  out.setUint16(20, 1, Endian.little); // format: PCM
  out.setUint16(22, 1, Endian.little); // channels: mono
  out.setUint32(24, sampleRate, Endian.little);
  out.setUint32(28, sampleRate * bytesPerSample, Endian.little);
  out.setUint16(32, bytesPerSample, Endian.little);
  out.setUint16(34, 16, Endian.little); // bits per sample
  writeAscii(36, 'data');
  out.setUint32(40, dataLength, Endian.little);

  for (var index = 0; index < samples.length; index++) {
    final clamped = samples[index].clamp(-1.0, 1.0);
    out.setInt16(44 + index * 2, (clamped * 32767).round(), Endian.little);
  }
  return out.buffer.asUint8List();
}

Float64List _renderSamples(SoundEffect effect, int sampleRate) {
  final total = (effect.durationMilliseconds * sampleRate / 1000).round();
  final samples = Float64List(total);
  // A fixed seed keeps the noise identical between runs and platforms.
  final noise = _Noise(0x5eed1234);
  var lowPass = 0.0;
  final peak = _peakFor(effect.shape);

  for (var index = 0; index < total; index++) {
    final t = index / sampleRate;
    final progress = total <= 1 ? 1.0 : index / (total - 1);
    final value = switch (effect.shape) {
      SoundShape.tick =>
        math.sin(2 * math.pi * 3200 * t) *
            _decay(progress, curve: 26) *
            _fadeIn(t, 0.002),
      SoundShape.click =>
        (noise.next() * 0.7 + math.sin(2 * math.pi * 1800 * t) * 0.3) *
            _decay(progress, curve: 14) *
            _fadeIn(t, 0.001),
      SoundShape.pop =>
        math.sin(2 * math.pi * (420 - 260 * progress) * t) *
            _decay(progress, curve: 8) *
            _fadeIn(t, 0.004),
      SoundShape.whoosh => () {
        final cutoff = 300 + 5200 * math.sin(math.pi * progress);
        final alpha = (cutoff / (cutoff + sampleRate / (2 * math.pi))).clamp(
          0.0,
          1.0,
        );
        lowPass += alpha * (noise.next() - lowPass);
        final envelope = math.sin(math.pi * progress);
        return lowPass * envelope * envelope * 1.15;
      }(),
      SoundShape.riser => () {
        final frequency = 180 + 1500 * math.pow(progress, 1.6);
        final tone = math.sin(2 * math.pi * frequency * t) * 0.55;
        final air = noise.next() * 0.25 * progress;
        return (tone + air) * math.pow(progress, 0.7) * 0.9;
      }(),
      SoundShape.impact =>
        (math.sin(2 * math.pi * (72 - 30 * progress) * t) * 0.8 +
                noise.next() * 0.35 * _decay(progress, curve: 22)) *
            _decay(progress, curve: 5) *
            _fadeIn(t, 0.003),
      SoundShape.chime =>
        (math.sin(2 * math.pi * 880 * t) * 0.6 +
                math.sin(2 * math.pi * 1320 * t) * 0.35) *
            _decay(progress, curve: 4) *
            _fadeIn(t, 0.006),
    };
    // A guard rail below full scale: a shape that overshoots its own headroom
    // would otherwise clip, and a clipped effect is audibly broken.
    samples[index] = (value * peak).clamp(-_ceiling, _ceiling);
  }

  // A short fade at the tail removes the click a cut sample would leave.
  final fadeSamples = math.min(total, (0.004 * sampleRate).round());
  for (var index = 0; index < fadeSamples; index++) {
    final position = total - 1 - index;
    samples[position] *= index / fadeSamples;
  }
  return samples;
}

/// Headroom per shape, so one effect is not four times louder than another.
double _peakFor(SoundShape shape) => switch (shape) {
  SoundShape.tick => 0.5,
  SoundShape.click => 0.55,
  SoundShape.pop => 0.6,
  SoundShape.whoosh => 0.7,
  SoundShape.riser => 0.75,
  SoundShape.impact => 0.85,
  SoundShape.chime => 0.6,
};

double _decay(double progress, {required double curve}) {
  final remaining = (1 - progress).clamp(0.0, 1.0);
  return math.pow(remaining, curve / 2).toDouble();
}

/// Linear attack over [seconds] from the start of the sound.
double _fadeIn(double t, double seconds) =>
    seconds <= 0 ? 1.0 : math.min(1.0, t / seconds);

class _Noise {
  _Noise(this._state);

  int _state;

  /// xorshift32: small, deterministic and good enough for a noise burst.
  double next() {
    _state ^= (_state << 13) & 0xFFFFFFFF;
    _state ^= _state >> 17;
    _state ^= (_state << 5) & 0xFFFFFFFF;
    _state &= 0xFFFFFFFF;
    return (_state / 0x7FFFFFFF) - 1.0;
  }
}
