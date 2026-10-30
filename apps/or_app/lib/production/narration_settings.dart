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
    this.provider = 'system',
    this.cloudVoiceId = '',
    this.azureRegion = '',
    this.localBaseUrl = '',
    this.localVoiceId = '',
    this.localVoiceName = '',
    this.localReferencePath = '',
    this.localCloneAcknowledged = false,
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

  /// `system`, `elevenlabs`, `azure` or `local`. Empty/unknown reads as
  /// `system`.
  final String provider;

  /// The cloud voice id the last run used; empty means "ask next time".
  final String cloudVoiceId;

  /// Azure Speech region, stored because the endpoint depends on it. The key
  /// itself lives in the keychain.
  final String azureRegion;

  /// The loopback address of a local, keyless speech server. Empty means the
  /// default `http://127.0.0.1:8000`.
  final String localBaseUrl;

  /// The voice a local server offers, including one cloned from a clip.
  final String localVoiceId;

  /// The name given to a cloned voice when it was enrolled.
  final String localVoiceName;

  /// The reference clip the server cloned; kept so a restarted server can be
  /// re-enrolled without asking for the file again.
  final String localReferencePath;

  /// The user confirmed they have the right to clone the reference clip's
  /// voice. The cloud panel asks the same before an instant clone; the local
  /// path must not be the quiet exception.
  final bool localCloneAcknowledged;

  NarrationSettings copyWith({
    String? voice,
    String? locale,
    int? rate,
    int? gapMs,
    String? provider,
    String? cloudVoiceId,
    String? azureRegion,
    String? localBaseUrl,
    String? localVoiceId,
    String? localVoiceName,
    String? localReferencePath,
    bool? localCloneAcknowledged,
  }) => NarrationSettings(
    voice: voice ?? this.voice,
    locale: locale ?? this.locale,
    rate: rate ?? this.rate,
    gapMs: gapMs ?? this.gapMs,
    provider: provider ?? this.provider,
    cloudVoiceId: cloudVoiceId ?? this.cloudVoiceId,
    azureRegion: azureRegion ?? this.azureRegion,
    localBaseUrl: localBaseUrl ?? this.localBaseUrl,
    localVoiceId: localVoiceId ?? this.localVoiceId,
    localVoiceName: localVoiceName ?? this.localVoiceName,
    localReferencePath: localReferencePath ?? this.localReferencePath,
    localCloneAcknowledged:
        localCloneAcknowledged ?? this.localCloneAcknowledged,
  );

  /// The settings a successful enrolment writes back.
  ///
  /// It records what the server now knows — the base URL, the voice id it
  /// handed out, the name and the clip — and nothing else. Consent is the
  /// user's answer, so [localCloneAcknowledged] is carried forward untouched:
  /// an automatic re-enrolment after a server restart must never manufacture
  /// the agreement a manual enrolment asks for.
  NarrationSettings rememberEnrolment({
    required String baseUrl,
    required String voiceId,
    required String voiceName,
    required String referencePath,
  }) => copyWith(
    localBaseUrl: baseUrl,
    localVoiceId: voiceId,
    localVoiceName: voiceName,
    localReferencePath: referencePath,
  );

  Map<String, Object?> toJson() => {
    'version': 3,
    'voice': voice,
    'locale': locale,
    'rate': rate,
    'gap_ms': gapMs,
    'provider': provider,
    'cloud_voice_id': cloudVoiceId,
    'azure_region': azureRegion,
    'local_base_url': localBaseUrl,
    'local_voice_id': localVoiceId,
    'local_voice_name': localVoiceName,
    'local_reference_path': localReferencePath,
    'local_clone_acknowledged': localCloneAcknowledged,
  };

  static NarrationSettings fromJson(Object? value) {
    if (value is! Map<String, Object?>) return const NarrationSettings();
    String text(String key) {
      final raw = value[key];
      return raw is String && raw.length <= 256 ? raw : '';
    }

    String path(String key) {
      final raw = value[key];
      return raw is String && raw.length <= 1024 ? raw : '';
    }

    int bounded(Object? raw, int fallback, int minimum, int maximum) {
      if (raw is! num || !raw.isFinite) return fallback;
      final rounded = raw.round();
      return rounded < minimum || rounded > maximum ? fallback : rounded;
    }

    final provider = text('provider');
    return NarrationSettings(
      voice: text('voice'),
      locale: text('locale'),
      rate: bounded(value['rate'], 180, 90, 400),
      gapMs: bounded(value['gap_ms'], 250, 0, 10000),
      provider:
          const {'system', 'elevenlabs', 'azure', 'local'}.contains(provider)
          ? provider
          : 'system',
      cloudVoiceId: text('cloud_voice_id'),
      azureRegion: text('azure_region'),
      localBaseUrl: text('local_base_url'),
      localVoiceId: text('local_voice_id'),
      localVoiceName: text('local_voice_name'),
      localReferencePath: path('local_reference_path'),
      localCloneAcknowledged: value['local_clone_acknowledged'] == true,
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
  NarrationSettingsStore({Future<Directory> Function()? supportDirectory})
    : _supportDirectory = supportDirectory ?? getApplicationSupportDirectory;

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
