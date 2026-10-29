import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/narration_settings.dart';
import 'package:or_app/production/narration_settings_dialog.dart';
import 'package:or_app/production/voiceover_service.dart';

class _MemoryStore extends NarrationSettingsStore {
  _MemoryStore();

  NarrationSettings saved = const NarrationSettings();

  @override
  Future<NarrationSettings> load() async => saved;

  @override
  Future<void> save(NarrationSettings settings) async {
    saved = settings;
  }
}

class _FakeVoiceover extends VoiceoverService {
  _FakeVoiceover(this.voices);

  final List<VoiceOption> voices;
  String? previewedVoice;
  String? previewedText;
  int? previewedRate;

  @override
  Future<List<VoiceOption>> listVoices() async => voices;

  @override
  Future<File> preview({
    required String voice,
    required String text,
    required Directory directory,
    int rate = 180,
  }) async {
    previewedVoice = voice;
    previewedText = text;
    previewedRate = rate;
    directory.createSync(recursive: true);
    return File('${directory.path}/preview.wav');
  }
}

const _voices = [
  VoiceOption(name: 'Linh', locale: 'vi_VN'),
  VoiceOption(name: 'Daniel', locale: 'en_GB'),
  VoiceOption(name: 'Samantha', locale: 'en_US'),
];

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('the saved voice wins, then an installed Vietnamese voice', () {
    expect(
      NarrationSettings.preferredVoiceName(
        _voices,
        const NarrationSettings(voice: 'Daniel'),
      ),
      'Daniel',
    );
    expect(
      NarrationSettings.preferredVoiceName(_voices, const NarrationSettings()),
      'Linh',
    );
    expect(
      NarrationSettings.preferredVoiceName(const [
        VoiceOption(name: 'Daniel', locale: 'en_GB'),
      ], const NarrationSettings(voice: 'missing')),
      'Daniel',
    );
  });

  test('stored settings clamp unusable values instead of failing', () {
    final settings = NarrationSettings.fromJson({
      'voice': 'Linh',
      'rate': 2000,
      'gap_ms': 4,
    });
    expect(settings.voice, 'Linh');
    expect(settings.rate, 180);
    expect(settings.gapMs, 4);

    final corrupt = NarrationSettings.fromJson('not a map');
    expect(corrupt.voice, isEmpty);
    expect(corrupt.rate, 180);
  });

  test('the store round-trips its file under application support', () async {
    final directory = await Directory.systemTemp.createTemp('or-narration-');
    addTearDown(() => directory.delete(recursive: true));
    final store = NarrationSettingsStore(
      supportDirectory: () async => directory,
    );
    expect((await store.load()).voice, isEmpty);
    await store.save(
      const NarrationSettings(
        voice: 'Mai',
        locale: 'vi_VN',
        rate: 195,
        gapMs: 400,
        provider: 'local',
        localBaseUrl: 'http://127.0.0.1:9000',
        localVoiceId: 'cloned',
        localVoiceName: 'My voice',
        localReferencePath: '/tmp/ref.wav',
        localCloneAcknowledged: true,
      ),
    );
    final loaded = await store.load();
    expect(loaded.voice, 'Mai');
    expect(loaded.locale, 'vi_VN');
    expect(loaded.rate, 195);
    expect(loaded.gapMs, 400);
    expect(loaded.provider, 'local');
    expect(loaded.localBaseUrl, 'http://127.0.0.1:9000');
    expect(loaded.localVoiceId, 'cloned');
    expect(loaded.localVoiceName, 'My voice');
    expect(loaded.localReferencePath, '/tmp/ref.wav');
    expect(loaded.localCloneAcknowledged, isTrue);
  });

  test('copyWith keeps every field it is not asked to change', () {
    const base = NarrationSettings(
      voice: 'Mai',
      locale: 'vi_VN',
      rate: 190,
      gapMs: 300,
      provider: 'local',
      cloudVoiceId: 'cloud-1',
      azureRegion: 'southeastasia',
      localBaseUrl: 'http://127.0.0.1:9001',
      localVoiceId: 'cloned',
      localVoiceName: 'My voice',
      localReferencePath: '/tmp/ref.wav',
      localCloneAcknowledged: true,
    );
    final changed = base.copyWith(voice: 'Other', rate: 180, gapMs: 250);
    expect(changed.voice, 'Other');
    expect(changed.rate, 180);
    expect(changed.gapMs, 250);
    // A field added to the settings must also reach copyWith, or a screen that
    // edits four options silently resets the rest.
    expect(changed.locale, 'vi_VN');
    expect(changed.provider, 'local');
    expect(changed.cloudVoiceId, 'cloud-1');
    expect(changed.azureRegion, 'southeastasia');
    expect(changed.localBaseUrl, 'http://127.0.0.1:9001');
    expect(changed.localVoiceId, 'cloned');
    expect(changed.localVoiceName, 'My voice');
    expect(changed.localReferencePath, '/tmp/ref.wav');
    expect(changed.localCloneAcknowledged, isTrue);
  });

  test('a re-enrolment records the voice but never invents consent', () {
    const base = NarrationSettings(
      provider: 'local',
      localBaseUrl: 'http://127.0.0.1:8000',
      localVoiceId: 'old',
      localVoiceName: 'old name',
      localReferencePath: '/tmp/old.wav',
    );
    final recorded = base.rememberEnrolment(
      baseUrl: 'http://127.0.0.1:9000',
      voiceId: 'Mai Anh',
      voiceName: 'Mai Anh',
      referencePath: '/tmp/ref.wav',
    );
    // What the server just learned is written back...
    expect(recorded.localBaseUrl, 'http://127.0.0.1:9000');
    expect(recorded.localVoiceId, 'Mai Anh');
    expect(recorded.localVoiceName, 'Mai Anh');
    expect(recorded.localReferencePath, '/tmp/ref.wav');
    // ...and consent is the user's answer, not something a recovery run can
    // grant. A server restart used to launder a withdrawn acknowledgement here.
    expect(recorded.localCloneAcknowledged, isFalse);
    expect(recorded.provider, 'local');
    // An acknowledgement that is on record survives the same write.
    final kept = base
        .copyWith(localCloneAcknowledged: true)
        .rememberEnrolment(
          baseUrl: 'http://127.0.0.1:9000',
          voiceId: 'Mai Anh',
          voiceName: 'Mai Anh',
          referencePath: '/tmp/ref.wav',
        );
    expect(kept.localCloneAcknowledged, isTrue);
  });

  test('the local provider is accepted and an unknown one falls back', () {
    final local = NarrationSettings.fromJson({
      'provider': 'local',
      'local_base_url': '127.0.0.1:9000',
      'local_voice_id': 'cloned',
      'local_reference_path': '/tmp/ref.wav',
    });
    expect(local.provider, 'local');
    expect(local.localVoiceId, 'cloned');
    expect(
      NarrationSettings.fromJson({'provider': 'bogus'}).provider,
      'system',
    );
  });

  test('the settings intro names the source that is actually in use', () {
    expect(
      narrationSettingsIntroText('system'),
      'Voices installed on this Mac. The assistant and the Audio tool use '
      'the saved default; nothing is uploaded.',
    );
    final local = narrationSettingsIntroText('local');
    expect(local, contains('the local server'));
    expect(local, contains('installed-voice fallback'));
    expect(narrationSettingsIntroText('elevenlabs'), contains('ElevenLabs'));
    expect(narrationSettingsIntroText('azure'), contains('Microsoft Azure'));
  });

  testWidgets('voice settings save the chosen voice, rate and gap', (
    tester,
  ) async {
    final voiceover = _FakeVoiceover(_voices);
    final store = _MemoryStore();
    NarrationSettings? result;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => Center(
              child: ElevatedButton(
                onPressed: () async {
                  result = await showDialog<NarrationSettings>(
                    context: context,
                    builder: (context) => NarrationSettingsDialog(
                      initial: const NarrationSettings(),
                      voiceover: voiceover,
                      store: store,
                      temporaryDirectory: () async => Directory.systemTemp,
                    ),
                  );
                },
                child: const Text('Open'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Open'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    await tester.pump();

    expect(find.text('Narration voice'), findsOneWidget);
    await tester.ensureVisible(
      find.byKey(const ValueKey('narration-voice-Daniel')),
    );
    await tester.tap(find.byKey(const ValueKey('narration-voice-Daniel')));
    await tester.pump();
    await tester.ensureVisible(
      find.byKey(const ValueKey('narration-voice-preview')),
    );
    await tester.tap(find.byKey(const ValueKey('narration-voice-preview')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));
    expect(voiceover.previewedVoice, 'Daniel');
    expect(voiceover.previewedText, contains('voice preview'));
    expect(voiceover.previewedRate, 180);
    await tester.ensureVisible(
      find.byKey(const ValueKey('narration-voice-save')),
    );
    await tester.tap(find.byKey(const ValueKey('narration-voice-save')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));

    expect(result, isNotNull);
    expect(result!.voice, 'Daniel');
    expect(result!.locale, 'en_GB');
    expect(store.saved.voice, 'Daniel');
    expect(store.saved.rate, 180);
  });

  testWidgets('preview speaks at the configured rate in the voice language', (
    tester,
  ) async {
    final voiceover = _FakeVoiceover(_voices);
    final store = _MemoryStore();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => TextButton(
              onPressed: () {
                showDialog<NarrationSettings>(
                  context: context,
                  builder: (context) => NarrationSettingsDialog(
                    initial: const NarrationSettings(rate: 240),
                    voiceover: voiceover,
                    store: store,
                    temporaryDirectory: () async => Directory.systemTemp,
                  ),
                );
              },
              child: const Text('Open'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Open'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    await tester.pump();

    await tester.ensureVisible(
      find.byKey(const ValueKey('narration-voice-Linh')),
    );
    await tester.tap(find.byKey(const ValueKey('narration-voice-Linh')));
    await tester.pump();
    await tester.ensureVisible(
      find.byKey(const ValueKey('narration-voice-preview')),
    );
    await tester.tap(find.byKey(const ValueKey('narration-voice-preview')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));

    expect(voiceover.previewedVoice, 'Linh');
    expect(voiceover.previewedRate, 240);
    expect(voiceover.previewedText, contains('Xin chào'));
  });
}
