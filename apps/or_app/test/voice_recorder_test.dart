import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/record_voice_dialog.dart';
import 'package:or_app/production/voice_recorder.dart';

class _FakeRecorder extends VoiceRecorder {
  _FakeRecorder({this.state = 'granted', this.requestedState = 'granted'});

  String state;
  final String requestedState;

  int permissionReads = 0;
  int requests = 0;
  bool started = false;
  bool stopped = false;
  bool openedSettings = false;
  String? path;

  @override
  Future<String> permission() async {
    permissionReads += 1;
    return state;
  }

  @override
  Future<String> requestPermission() async {
    requests += 1;
    state = requestedState;
    return requestedState;
  }

  @override
  Future<bool> openSystemSettings() async {
    openedSettings = true;
    return true;
  }

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

  test('the recorder bridge asks macOS before the first take', () async {
    final calls = <MethodCall>[];
    var granted = false;
    const channel = MethodChannel('or_narration');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          switch (call.method) {
            case 'recordPermission':
              return granted ? 'granted' : 'undetermined';
            case 'recordPermissionRequest':
              granted = true;
              return 'granted';
            case 'recordPermissionSettings':
              return true;
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
    expect(await recorder.permission(), 'undetermined');
    expect(await recorder.requestPermission(), 'granted');
    final directory = await Directory.systemTemp.createTemp('or-recorder-');
    addTearDown(() => directory.delete(recursive: true));
    final file = File('${directory.path}/take.wav');
    await recorder.start(file);
    expect(await recorder.stop(), 1234);
    expect(await recorder.openSystemSettings(), isTrue);

    expect(
      calls.map((call) => call.method).toList(),
      <String>[
        'recordPermission',
        'recordPermissionRequest',
        'recordPermission',
        'recordStart',
        'recordStop',
        'recordPermissionSettings',
      ],
    );
    expect((calls[3].arguments as Map)['path'], file.path);
    // No call carries a keychain account: the microphone path has no credential.
    expect(calls.every((call) => call.arguments is Map == false ||
        !(call.arguments as Map).containsKey('account')), isTrue);
  });

  test('a native denial surfaces as a recorder error', () async {
    const channel = MethodChannel('or_narration');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          switch (call.method) {
            case 'recordPermission':
              return 'granted';
            case 'recordStart':
              throw PlatformException(
                code: 'PERMISSION_DENIED',
                message: 'Microphone access is not granted.',
              );
          }
          return null;
        });
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null),
    );

    final directory = await Directory.systemTemp.createTemp('or-recorder-');
    addTearDown(() => directory.delete(recursive: true));
    await expectLater(
      const VoiceRecorder().start(File('${directory.path}/take.wav')),
      throwsA(
        isA<VoiceRecorderException>().having(
          (error) => error.code,
          'code',
          'PERMISSION_DENIED',
        ),
      ),
    );
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
    expect(recorder.requests, 0, reason: 'a granted recorder is not asked again');
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

  testWidgets('the first take sends the macOS permission request', (
    tester,
  ) async {
    final recorder = _FakeRecorder(
      state: 'undetermined',
      requestedState: 'granted',
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RecordVoiceDialog(
            recorder: recorder,
            temporaryDirectory: () async => Directory.systemTemp,
            supportDirectory: () async => Directory.systemTemp,
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    await tester.tap(find.byKey(const ValueKey('record-voice-toggle')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));

    expect(recorder.requests, 1, reason: 'the system prompt must be requested');
    expect(recorder.started, isTrue);

    await tester.tap(find.byKey(const ValueKey('record-voice-toggle')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
  });

  testWidgets('a denial offers the microphone settings pane', (tester) async {
    final recorder = _FakeRecorder(
      state: 'denied',
      requestedState: 'denied',
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RecordVoiceDialog(
            recorder: recorder,
            temporaryDirectory: () async => Directory.systemTemp,
            supportDirectory: () async => Directory.systemTemp,
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    expect(find.byKey(const ValueKey('record-voice-open-settings')), findsOne);

    await tester.tap(find.byKey(const ValueKey('record-voice-toggle')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 100));
    expect(recorder.requests, 1);
    expect(recorder.started, isFalse);
    expect(find.byKey(const ValueKey('record-voice-error')), findsOne);

    await tester.tap(find.byKey(const ValueKey('record-voice-open-settings')));
    await tester.pump();
    expect(recorder.openedSettings, isTrue);
  });
}
