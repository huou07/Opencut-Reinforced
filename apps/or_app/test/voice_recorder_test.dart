import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/record_voice_dialog.dart';
import 'package:or_app/production/voice_recorder.dart';

class _FakeRecorder extends VoiceRecorder {
  _FakeRecorder();

  bool started = false;
  bool stopped = false;
  String? path;

  @override
  Future<String> permission() async => 'granted';

  @override
  Future<void> start(File output) async {
    started = true;
    path = output.path;
  }

  @override
  Future<int> stop() async {
    stopped = true;
    return 1500;
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('the recorder bridge starts and stops with a writeable path', () async {
    final calls = <MethodCall>[];
    const channel = MethodChannel('or_narration');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          switch (call.method) {
            case 'recordPermission':
              return 'granted';
            case 'recordStart':
              return true;
            case 'recordStop':
              return {'duration_ms': 1234};
          }
          return null;
        });
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null),
    );

    const recorder = VoiceRecorder();
    expect(await recorder.permission(), 'granted');
    final directory = await Directory.systemTemp.createTemp('or-recorder-');
    addTearDown(() => directory.delete(recursive: true));
    final file = File('${directory.path}/take.wav');
    await recorder.start(file);
    expect(await recorder.stop(), 1234);
    expect((calls[1].arguments as Map)['path'], file.path);
  });

  testWidgets('the record dialog returns the take with its measured duration', (
    tester,
  ) async {
    final recorder = _FakeRecorder();
    VoiceRecording? result;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => Center(
              child: ElevatedButton(
                onPressed: () async {
                  result = await showDialog<VoiceRecording>(
                    context: context,
                    builder: (context) => RecordVoiceDialog(
                      recorder: recorder,
                      temporaryDirectory: () async => Directory.systemTemp,
                      supportDirectory: () async => Directory.systemTemp,
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
    await tester.pump(const Duration(milliseconds: 300));
    await tester.tap(find.byKey(const ValueKey('record-voice-toggle')));
    await tester.pump();
    expect(recorder.started, isTrue);
    await tester.tap(find.byKey(const ValueKey('record-voice-toggle')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    expect(recorder.stopped, isTrue);
    await tester.tap(find.byKey(const ValueKey('record-voice-use')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    expect(result, isNotNull);
    expect(result!.durationMs, 1500);
    expect(result!.path, recorder.path);
  });
}
