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

  @override
  Future<List<VoiceOption>> listVoices() async => voices;

  @override
  Future<File> preview({
    required String voice,
    required String text,
    required Directory directory,
  }) async {
    previewedVoice = voice;
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
}
