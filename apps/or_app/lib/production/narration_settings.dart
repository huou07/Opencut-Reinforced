import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

import 'voiceover_service.dart';

/// The narration preferences shared by the app and the installed agent tools.
///
/// This file never holds a credential. A cloud provider is named here, while its
/// API key belongs in the platform keychain and is read only by the code that
/// performs the request.
class NarrationSettings {
  const NarrationSettings({
    this.voice = '',
    this.locale = '',
    this.rate = 180,
    this.gapMs = 250,
  });

  /// The installed macOS voice name, for example `Linh`. Empty means "let the
  /// app choose the best installed Vietnamese voice on this machine".
  final String voice;

  /// The BCP-47-ish locale `say` reports, for example `vi_VN`.
  final String locale;

  /// Words per minute passed to the synthesizer.
  final int rate;

  /// Silence between narration lines, in milliseconds.
  final int gapMs;

  NarrationSettings copyWith({
    String? voice,
    String? locale,
    int? rate,
    int? gapMs,
  }) => NarrationSettings(
    voice: voice ?? this.voice,
    locale: locale ?? this.locale,
    rate: rate ?? this.rate,
    gapMs: gapMs ?? this.gapMs,
  );

  Map<String, Object?> toJson() => {
    'version': 1,
    'voice': voice,
    'locale': locale,
    'rate': rate,
    'gap_ms': gapMs,
  };

  static NarrationSettings fromJson(Object? value) {
    if (value is! Map<String, Object?>) return const NarrationSettings();
    String text(String key) {
      final raw = value[key];
      return raw is String && raw.length <= 256 ? raw : '';
    }

    int bounded(Object? raw, int fallback, int minimum, int maximum) {
      if (raw is! num || !raw.isFinite) return fallback;
      final rounded = raw.round();
      return rounded < minimum || rounded > maximum ? fallback : rounded;
    }

    return NarrationSettings(
      voice: text('voice'),
      locale: text('locale'),
      rate: bounded(value['rate'], 180, 90, 400),
      gapMs: bounded(value['gap_ms'], 250, 0, 10000),
    );
  }

  /// The deterministic default order: a saved Vietnamese voice, then any
  /// installed Vietnamese voice, then US English, then the first installed one.
  ///
  /// There is deliberately no product-level voice name in this list: a machine
  /// that has no `Linh` voice must still narrate.
  static String? preferredVoiceName(
    List<VoiceOption> voices,
    NarrationSettings settings,
  ) {
    if (voices.isEmpty) return null;
    if (settings.voice.isNotEmpty &&
        voices.any((voice) => voice.name == settings.voice)) {
      return settings.voice;
    }
    for (final locale in const ['vi_VN', 'vi', 'en_US', 'en_GB']) {
      for (final voice in voices) {
        if (voice.locale == locale) return voice.name;
      }
    }
    for (final voice in voices) {
      if (voice.locale.startsWith('vi')) return voice.name;
    }
    return voices.first.name;
  }
}

/// Reads and writes `<application support>/narration-settings.json`.
///
/// Writes are serialized so two quick changes cannot interleave, and a corrupt
/// file falls back to the defaults rather than breaking the Audio tool.
class NarrationSettingsStore {
  NarrationSettingsStore({
    Future<Directory> Function()? supportDirectory,
  }) : _supportDirectory = supportDirectory ?? getApplicationSupportDirectory;

  final Future<Directory> Function() _supportDirectory;
  Future<void> _pending = Future<void>.value();

  Future<File> settingsFile() async {
    final support = await _supportDirectory();
    return File(
      '${support.path}${Platform.pathSeparator}narration-settings.json',
    );
  }

  Future<NarrationSettings> load() async {
    try {
      final file = await settingsFile();
      if (!await file.exists()) return const NarrationSettings();
      return NarrationSettings.fromJson(jsonDecode(await file.readAsString()));
    } catch (_) {
      // No platform support (a widget test) or a corrupt file: the caller gets
      // the product defaults instead of a broken Audio tool.
      return const NarrationSettings();
    }
  }

  Future<void> save(NarrationSettings settings) {
    final write = _pending.then((_) async {
      final file = await settingsFile();
      await file.parent.create(recursive: true);
      final temporary = File('${file.path}.tmp');
      await temporary.writeAsString(jsonEncode(settings.toJson()));
      await temporary.rename(file.path);
    });
    // A failed write must not poison the next one; the caller still sees it.
    _pending = write.catchError((_) {});
    return write;
  }
}
