import 'dart:convert';

import 'package:flutter/material.dart';
import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/cloud_voice_service.dart';
import 'package:or_app/production/cloud_voice_settings_panel.dart';
import 'package:or_app/production/narration_settings.dart';

class _FakeCloud extends CloudTtsService {
  _FakeCloud() : super(keychain: const NarrationKeychain());

  @override
  Future<List<CloudVoice>> listVoices(
    CloudVoiceProvider provider, {
    String azureRegion = '',
  }) async => const [
    CloudVoice(
      provider: CloudVoiceProvider.elevenLabs,
      id: 'voice-1',
      name: 'Mai',
      locale: 'vi',
    ),
  ];

  @override
  Future<void> testKey(
    CloudVoiceProvider provider, {
    String azureRegion = '',
  }) async {}
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('ElevenLabs asks for v3 PCM and sends the line as JSON', () {
    final request = elevenLabsSpeechRequest(voiceId: 'abc', text: 'Xin chào');
    expect(request.uri.host, 'api.elevenlabs.io');
    expect(request.uri.path, '/v1/text-to-speech/abc');
    expect(request.uri.queryParameters['output_format'], 'pcm_44100');
    final body = jsonDecode(utf8.decode(request.body)) as Map<String, Object?>;
    expect(body['model_id'], 'eleven_v3');
    expect(body['text'], 'Xin chào');
    expect(request.headers['Content-Type'], 'application/json');
  });

  test('Azure builds an escaped SSML body for the chosen region and voice', () {
    final request = azureSpeechRequest(
      region: 'southeastasia',
      voiceId: 'vi-VN-HoaiMyNeural',
      text: 'A & B <c>',
    );
    expect(
      request.uri.toString(),
      'https://southeastasia.tts.speech.microsoft.com/cognitiveservices/v1',
    );
    expect(
      request.headers['X-Microsoft-OutputFormat'],
      'riff-24khz-16bit-mono-pcm',
    );
    final ssml = utf8.decode(request.body);
    expect(ssml, contains('name="vi-VN-HoaiMyNeural"'));
    expect(ssml, contains('A &amp; B &lt;c&gt;'));
  });

  test('raw PCM is wrapped in a canonical WAV with the exact duration', () {
    final pcm = Uint8List(44100 * 2); // one second of 44.1 kHz mono s16
    final wav = wrapPcmAsWav(pcm, sampleRate: 44100, channels: 1);
    expect(String.fromCharCodes(wav.sublist(0, 4)), 'RIFF');
    expect(String.fromCharCodes(wav.sublist(8, 12)), 'WAVE');
    expect(wav[22], 1);
    expect(wav[24], 44100 & 0xFF);
    expect(wavDurationMs(wav), 1000);
  });

  test('a provider payload that is not a WAV is refused', () {
    expect(
      () => wavDurationMs(Uint8List.fromList([1, 2, 3, 4])),
      throwsA(isA<CloudVoiceException>()),
    );
  });

  test('the keychain channel is scoped to the provider account', () async {
    final calls = <MethodCall>[];
    const channel = MethodChannel('or_narration');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          switch (call.method) {
            case 'keychainGet':
              return 'sk-test';
            case 'keychainSet':
            case 'keychainDelete':
              return true;
          }
          return null;
        });
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null),
    );

    const keychain = NarrationKeychain();
    expect(await keychain.read('elevenlabs'), 'sk-test');
    await keychain.write('azure', 'azure-key');
    await keychain.delete('elevenlabs');
    expect(calls.map((call) => call.method), [
      'keychainGet',
      'keychainSet',
      'keychainDelete',
    ]);
    expect((calls[0].arguments as Map)['account'], 'elevenlabs');
    expect((calls[1].arguments as Map)['account'], 'azure');
    expect((calls[1].arguments as Map)['value'], 'azure-key');
  });

  test('a keychain query that never answers fails with a named error', () async {
    const channel = MethodChannel('or_narration');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          // macOS holds the reply while it asks for keychain access.
          return Completer<String?>().future;
        });
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null),
    );

    const keychain = NarrationKeychain(
      timeout: Duration(milliseconds: 30),
    );
    await expectLater(
      keychain.read('elevenlabs'),
      throwsA(
        isA<CloudVoiceException>()
            .having((error) => error.code, 'code', 'KEYCHAIN_TIMEOUT')
            .having(
              (error) => error.message,
              'message',
              'macOS did not answer the keychain check. Unlock the login '
                  'keychain and try again.',
            ),
      ),
    );
  });

  testWidgets('cloud settings saves a key, tests it, and removes it', (
    tester,
  ) async {
    const channel = MethodChannel('or_narration');
    var stored = <String, String>{};
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          final arguments = call.arguments as Map;
          final account = arguments['account'] as String;
          switch (call.method) {
            case 'keychainGet':
              return stored[account];
            case 'keychainSet':
              stored[account] = arguments['value'] as String;
              return true;
            case 'keychainDelete':
              stored.remove(account);
              return true;
          }
          return null;
        });
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: CloudVoiceSettingsPanel(
              settings: const NarrationSettings(),
              onChanged: (_) {},
              service: _FakeCloud(),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const ValueKey('cloud-voice-key-elevenlabs')),
      'sk-test',
    );
    await tester.tap(find.byKey(const ValueKey('cloud-voice-save-elevenlabs')));
    await tester.pumpAndSettle();
    expect(stored['elevenlabs'], 'sk-test');
    expect(find.text('Key saved to the macOS keychain.'), findsOneWidget);
    expect(
      tester
          .widget<Text>(
            find.byKey(const ValueKey('cloud-voice-status-elevenlabs')),
          )
          .data,
      'Configured',
    );

    await tester.tap(find.byKey(const ValueKey('cloud-voice-test-elevenlabs')));
    await tester.pumpAndSettle();
    expect(find.text('The key works: 1 voices available.'), findsOneWidget);

    await tester.tap(
      find.byKey(const ValueKey('cloud-voice-remove-elevenlabs')),
    );
    await tester.pumpAndSettle();
    expect(stored.containsKey('elevenlabs'), isFalse);
    expect(
      tester
          .widget<Text>(
            find.byKey(const ValueKey('cloud-voice-status-elevenlabs')),
          )
          .data,
      'No key',
    );
  });
}
