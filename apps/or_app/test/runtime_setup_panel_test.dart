import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/local_tts_server_manager.dart';
import 'package:or_app/updates/resource_center.dart';
import 'package:or_app/updates/runtime_setup_panel.dart';

class _FakeService extends ResourceCenterService {
  _FakeService({this.runtime});

  RuntimeSnapshot? runtime;
  int inspected = 0;
  int installed = 0;
  int added = 0;
  String? savedHarness;
  final openedUrls = <String>[];

  @override
  Future<RuntimeSnapshot> inspectRuntime() async {
    inspected++;
    return runtime ??
        const RuntimeSnapshot(
          packaged: true,
          ffmpeg: true,
          orCli: true,
          assistantPath: null,
          bundlePath: '/Applications/Opencut Reinforced.app',
          speechRuntime: false,
        );
  }

  @override
  Future<Directory> installSpeechRuntime({
    void Function(int received, int total)? onProgress,
  }) async {
    installed++;
    onProgress?.call(1024, 2048);
    runtime = const RuntimeSnapshot(
      packaged: true,
      ffmpeg: true,
      orCli: true,
      assistantPath: null,
      bundlePath: '/Applications/Opencut Reinforced.app',
      speechRuntime: true,
    );
    return Directory.systemTemp;
  }

  @override
  Future<Directory> addSpeechRuntimeFiles(Directory source) async {
    added++;
    return source;
  }

  @override
  Future<String?> chooseRuntimeDirectory() async => '/tmp/runtime';

  @override
  Future<String?> chooseHarnessExecutable() async => '/opt/dsh';

  @override
  Future<void> saveHarnessPath(String path) async {
    savedHarness = path;
  }

  @override
  Future<void> openUrl(String url) async => openedUrls.add(url);
}

void main() {
  testWidgets('a missing speech runtime offers the engine install', (
    tester,
  ) async {
    final service = _FakeService();
    final manager = LocalTtsServerManager(
      supportDirectory: () async => Directory.systemTemp,
    );
    addTearDown(manager.dispose);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: RuntimeSetupPanel(service: service, manager: manager),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('tools-speech-install')), findsOneWidget);
    expect(find.textContaining('Not installed'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('tools-speech-install')));
    await tester.pumpAndSettle();

    expect(service.installed, 1);
    expect(service.inspected, greaterThan(1));
    expect(find.byKey(const ValueKey('tools-speech-install')), findsNothing);
  });

  testWidgets('an installed runtime hides the repair action', (tester) async {
    final service = _FakeService(
      runtime: const RuntimeSnapshot(
        packaged: true,
        ffmpeg: true,
        orCli: true,
        assistantPath: '/opt/dsh',
        bundlePath: '/Applications/Opencut Reinforced.app',
        speechRuntime: true,
      ),
    );
    final manager = LocalTtsServerManager(
      supportDirectory: () async => Directory.systemTemp,
    );
    addTearDown(manager.dispose);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: RuntimeSetupPanel(service: service, manager: manager),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('tools-speech-install')), findsNothing);
    expect(
      find.byKey(const ValueKey('tools-speech-add-files')),
      findsOneWidget,
    );
    expect(find.textContaining('/opt/dsh'), findsOneWidget);
  });

  testWidgets('a chosen harness is saved and detection reruns', (tester) async {
    final service = _FakeService();
    final manager = LocalTtsServerManager(
      supportDirectory: () async => Directory.systemTemp,
    );
    addTearDown(manager.dispose);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: RuntimeSetupPanel(service: service, manager: manager),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final before = service.inspected;
    await tester.tap(find.byKey(const ValueKey('tools-harness-choose')));
    await tester.pumpAndSettle();

    expect(service.savedHarness, '/opt/dsh');
    expect(service.inspected, greaterThan(before));
  });

  testWidgets('a half-installed runtime names the missing half', (
    tester,
  ) async {
    final service = _FakeService(
      runtime: const RuntimeSnapshot(
        packaged: true,
        ffmpeg: true,
        orCli: true,
        assistantPath: null,
        bundlePath: '/Applications/Opencut Reinforced.app',
        whisperEngine: true,
        whisperProvider: false,
      ),
    );
    final manager = LocalTtsServerManager(
      supportDirectory: () async => Directory.systemTemp,
    );
    addTearDown(manager.dispose);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: RuntimeSetupPanel(service: service, manager: manager),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('adapter'), findsOneWidget);
  });
}
