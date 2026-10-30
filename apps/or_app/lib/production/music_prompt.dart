/// Turns a free-form request into the parameters the local music synthesiser
/// understands.
///
/// The synthesiser is deliberately small (a pad, an arpeggio, a bass and light
/// drums over a fixed chord set), so the prompt cannot invent a new instrument.
/// What it can do is read intent in Vietnamese or English: character, tempo,
/// length, and whether drums belong at all. The readback string is shown in the
/// dialog, so the user always sees how the sentence was understood before the
/// bed is generated.
library;

/// One interpreted music request.
class MusicPlan {
  const MusicPlan({
    required this.moodId,
    required this.tempo,
    required this.seconds,
    required this.drumsScale,
    required this.padScale,
    required this.arpScale,
    required this.brightnessScale,
    required this.octaveShift,
    required this.seed,
    required this.summary,
    required this.notes,
  });

  final String moodId;
  final int tempo;
  final int seconds;

  /// Multipliers over the mood's own mix. Zero drums means a drumless bed.
  final double drumsScale;
  final double padScale;
  final double arpScale;
  final double brightnessScale;
  final int octaveShift;

  /// Stable for the same prompt and variation, so one request is reproducible.
  final int seed;

  /// One short line for the dialog: mood, tempo, length, drums.
  final String summary;

  /// What the interpreter could not use, if anything.
  final List<String> notes;
}

/// Reads music requests. Unknown words are ignored rather than refused: a prompt
/// is a preference, not a command language.
class MusicPromptInterpreter {
  const MusicPromptInterpreter._();

  /// Interprets [prompt]. Empty or unrecognised input keeps the caller's
  /// defaults, so the prompt field can never make the dialog unusable.
  static MusicPlan interpret(
    String prompt, {
    required int fallbackTempo,
    required int fallbackSeconds,
    int variation = 0,
    String fallbackMoodId = 'calm',
  }) {
    final folded = ' ${_fold(prompt).replaceAll(RegExp(r'\s+'), ' ').trim()} ';
    final mood = _mood(folded, fallbackMoodId);
    final tempo = _tempo(folded, fallbackTempo);
    final seconds = _seconds(folded, fallbackSeconds);
    var drums = 1.0;
    var pad = 1.0;
    var arp = 1.0;
    var brightness = 1.0;
    var octaveShift = 0;
    final notes = <String>[];

    final noDrums =
        folded.contains('khong trong') ||
        folded.contains('khong can trong') ||
        folded.contains('chi piano') ||
        folded.contains('chi dan ') ||
        folded.contains('piano only') ||
        folded.contains('no drums') ||
        folded.contains('drumless') ||
        folded.contains('without drums');
    final moreDrums =
        folded.contains('trong manh') ||
        folded.contains('co trong') ||
        folded.contains('punchy drums') ||
        folded.contains('with drums') ||
        folded.contains('strong drums') ||
        folded.contains('beat manh');
    if (noDrums) {
      drums = 0;
    } else if (moreDrums) {
      drums = 1.2;
    }

    if (folded.contains('piano') ||
        folded.contains('dan ') ||
        folded.contains('acoustic') ||
        folded.contains('moc')) {
      // The synthesiser has no sampled piano; a stronger plucked arpeggio and a
      // quieter pad is the closest honest reading.
      arp = 1.25;
      pad = 0.85;
      if (noDrums) drums = 0;
    }
    if (folded.contains('chuong') ||
        folded.contains('bell') ||
        folded.contains('bright') ||
        folded.contains('tuoi sang')) {
      brightness = 1.2;
    }
    if (folded.contains('toi ') ||
        folded.contains('dark') ||
        folded.contains('u am') ||
        folded.contains('tram ')) {
      brightness = 0.75;
    }
    if (folded.contains('quang cao') ||
        folded.contains('high pitch') ||
        folded.contains('octave up')) {
      octaveShift = 12;
    }
    if (folded.contains('quang tram') ||
        folded.contains('octave down') ||
        folded.contains('bass sau')) {
      octaveShift = -12;
      pad = 1.1;
    }

    if (prompt.trim().isNotEmpty && notes.isEmpty) {
      final understood = <String>[
        if (_hasAny(folded, _calmWords) ||
            _hasAny(folded, _warmWords) ||
            _hasAny(folded, _upliftingWords) ||
            _hasAny(folded, _tenseWords) ||
            _hasAny(folded, _minimalWords))
          'mood',
        if (RegExp(r'\d+\s*(bpm|nhip)').hasMatch(folded) ||
            _hasAny(folded, _slowWords) ||
            _hasAny(folded, _fastWords))
          'tempo',
        if (RegExp(r'\d+\s*(giay|phut|s |sec|min)').hasMatch(folded)) 'length',
        if (noDrums || moreDrums) 'drums',
      ];
      if (understood.isEmpty) {
        notes.add(
          'No music words were recognised, so the controls below were kept.',
        );
      }
    }

    final seed = _seed(folded, variation);
    final moodLabel = _moodLabel(mood);
    final drumsLabel = drums == 0
        ? 'no drums'
        : drums > 1
        ? 'strong drums'
        : 'light drums';
    return MusicPlan(
      moodId: mood,
      tempo: tempo,
      seconds: seconds,
      drumsScale: drums,
      padScale: pad,
      arpScale: arp,
      brightnessScale: brightness,
      octaveShift: octaveShift,
      seed: seed,
      summary: '$moodLabel · $tempo bpm · $seconds s · $drumsLabel',
      notes: List.unmodifiable(notes),
    );
  }

  static String _mood(String folded, String fallback) {
    // Order matters: a "buồn" request inside a thriller sentence is still tense.
    if (_hasAny(folded, _tenseWords)) return 'tense';
    if (_hasAny(folded, _upliftingWords)) return 'uplifting';
    if (_hasAny(folded, _minimalWords)) return 'minimal';
    if (_hasAny(folded, _warmWords)) return 'warm';
    if (_hasAny(folded, _calmWords)) return 'calm';
    return fallback;
  }

  static int _tempo(String folded, int fallback) {
    final explicit = RegExp(r'(\d{2,3})\s*(bpm|nhip)').firstMatch(folded);
    if (explicit != null) {
      final value = int.tryParse(explicit.group(1)!);
      if (value != null) return value.clamp(40, 180);
    }
    if (_hasAny(folded, _slowWords)) return 62;
    if (_hasAny(folded, _fastWords)) return 138;
    return fallback;
  }

  static int _seconds(String folded, int fallback) {
    final minutes = RegExp(r'(\d+)\s*(phut|min)').firstMatch(folded);
    final seconds = RegExp(r'(\d+)\s*(giay|sec|s )').firstMatch(folded);
    var total = 0;
    if (minutes != null) {
      total += (int.tryParse(minutes.group(1)!) ?? 0) * 60;
    }
    if (seconds != null) {
      total += int.tryParse(seconds.group(1)!) ?? 0;
    }
    if (total == 0) return fallback;
    return total.clamp(4, 600);
  }

  static bool _hasAny(String text, List<String> words) =>
      words.any((word) => text.contains(word));

  static String _moodLabel(String moodId) => switch (moodId) {
    'calm' => 'calm',
    'warm' => 'warm',
    'uplifting' => 'uplifting',
    'tense' => 'tense',
    'minimal' => 'minimal',
    _ => moodId,
  };

  /// FNV-1a over the folded prompt, so the same sentence and variation always
  /// produce the same bed.
  static int _seed(String folded, int variation) {
    var hash = 0x811c9dc5;
    for (final unit in folded.codeUnits) {
      hash ^= unit;
      hash = (hash * 0x01000193) & 0x7fffffff;
    }
    return (hash + variation * 7919) & 0x7fffffff;
  }

  /// Removes Vietnamese diacritics so one keyword list serves both spellings.
  static String _fold(String value) {
    var result = value.toLowerCase();
    for (final entry in _diacriticGroups.entries) {
      for (final rune in entry.value.runes) {
        result = result.replaceAll(String.fromCharCode(rune), entry.key);
      }
    }
    return result;
  }

  static const Map<String, String> _diacriticGroups = {
    'a': 'àáạảãâầấậẩẫăằắặẳẵ',
    'e': 'èéẹẻẽêềếệểễ',
    'i': 'ìíịỉĩ',
    'o': 'òóọỏõôồốộổỗơờớợởỡ',
    'u': 'ùúụủũưừứựửữ',
    'y': 'ỳýỵỷỹ',
    'd': 'đ',
  };

  static const List<String> _calmWords = [
    'calm',
    'chill',
    'relax',
    'soft',
    'gentle',
    'quiet',
    'ambient',
    'lofi',
    'lo-fi',
    'thu gian',
    'nhe nhang',
    'em diu',
    'diu ',
    'buon',
    'sad',
    'mua ',
    'rain',
    'chieu ',
    'evening',
    'ngu ',
    'sleep',
    'thien',
    'meditation',
    'slow',
    'cham',
  ];

  static const List<String> _warmWords = [
    'warm',
    'retro',
    'vintage',
    'soul',
    'jazz',
    'am ',
    'hoai niem',
    'xua',
    'moc',
  ];

  static const List<String> _upliftingWords = [
    'uplifting',
    'happy',
    'joyful',
    'bright',
    'upbeat',
    'energetic',
    'vlog',
    'travel',
    'vui',
    'tuoi',
    'soi dong',
    'nang luong',
    'nhiet huyet',
    'du lich',
    'hanh trinh',
    'fast',
    'nhanh',
    'gap',
  ];

  static const List<String> _tenseWords = [
    'tension',
    'tense',
    'thriller',
    'dramatic',
    'suspense',
    'dark',
    'cinematic',
    'horror',
    'cang',
    'kich tinh',
    'hoi hop',
    'toi ',
    'den ',
    'trailer',
    'hanh dong',
    'action',
  ];

  static const List<String> _minimalWords = [
    'minimal',
    'sparse',
    'simple',
    'toi gian',
    'don gian',
    'tinh te',
    'thanh lich',
    'elegant',
  ];

  static const List<String> _slowWords = [
    'slow',
    'cham',
    'nhe nhang',
    'em diu',
    'ballad',
    'lang man',
    'buon',
    'sad',
    'thu gian',
    'relax',
  ];

  static const List<String> _fastWords = [
    'fast',
    'nhanh',
    'soi dong',
    'upbeat',
    'energetic',
    'gap',
    'manh',
  ];
}

/// A convenience for the dialog: the seed for a "surprise me" variation.
int nextMusicVariation(int current) => (current + 1) % 1000;
