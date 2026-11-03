import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/local_tts_server_manager.dart';
import 'package:or_app/production/local_tts_service.dart';
import 'package:or_app/support/host_platform.dart';
import 'package:path/path.dart' as p;

void main() {
  test('every bundled engine is declared without a shell recipe', () {
    final vieneu = localTtsEngines.firstWhere(
      (engine) => engine.id == 'vieneu',
    );
    expect(vieneu.installable, isTrue);
    expect(vieneu.startable, isTrue);
    expect(vieneu.baseUrl, LocalTtsService.defaultBaseUrl);
    expect(vieneu.installArguments, isNotEmpty);
    expect(vieneu.startArguments, isNotEmpty);

    final custom = localTtsEngines.firstWhere(
      (engine) => engine.id == 'custom',
    );
    expect(custom.installable, isFalse);
    expect(custom.startable, isFalse);
    expect(localTtsEngines.map((engine) => engine.id).toSet(), hasLength(2));
  });

  test('engine folders live under the host application data folder', () async {
    final support = await Directory.systemTemp.createTemp('or-tts-support-');
    addTearDown(() => support.delete(recursive: true));
    final manager = LocalTtsServerManager(
      supportDirectory: () async => support,
    );
    addTearDown(manager.dispose);

    final directory = await manager.engineDirectory('vieneu');
    expect(directory.path, p.join(support.path, 'tts', 'vieneu'));
  });

  test('a simulated Windows host keeps engine data out of HOME', () async {
    const windows = HostPlatform(
      isWindows: true,
      isMacOS: false,
      isLinux: false,
      environment: {
        'USERPROFILE': r'C:\Users\ana',
        'LOCALAPPDATA': r'C:\Users\ana\AppData\Local',
      },
      executablePath: r'C:\or\or_app.exe',
    );
    final manager = LocalTtsServerManager(platform: windows);
    addTearDown(manager.dispose);

    final directory = await manager.engineDirectory('vieneu');
    expect(directory.path, startsWith(windows.appDataDirectory));
    expect(directory.path, contains('tts'));
  });

  test('selecting an engine reports the new selection', () {
    final manager = LocalTtsServerManager(
      supportDirectory: () async => Directory.systemTemp,
    );
    addTearDown(manager.dispose);
    var notifications = 0;
    manager.addListener(() => notifications++);

    manager.selectEngine('custom');
    expect(manager.engineId, 'custom');
    expect(notifications, 1);
    manager.selectEngine('custom');
    expect(notifications, 1);
  });

  test('a missing command is refused instead of started', () async {
    final manager = LocalTtsServerManager(
      supportDirectory: () async => Directory.systemTemp,
    );
    addTearDown(manager.dispose);
    final custom = localTtsEngines.firstWhere(
      (engine) => engine.id == 'custom',
    );

    await manager.start(custom);
    expect(manager.running, isFalse);
    expect(manager.error, isNotNull);
    expect(manager.error, contains('command'));
  });
}
