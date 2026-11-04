import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:path/path.dart' as p;
import 'package:or_app/assistant/assistant_settings.dart';
import 'package:or_app/support/host_platform.dart';

void main() {
  test('settings round-trip and reject malformed fields', () {
    const settings = AssistantSettings(
      harnessPath: '/opt/dsh',
      orCliPath: '/repo/target/debug/or',
      workspaceMode: AssistantWorkspaceMode.custom,
      workspacePath: '/tmp/workspace',
      modelValue: '["opencode-go","deepseek-v4.1-flash"]',
      reasoningValue: 'high',
      autoApprove: true,
    );
    final decoded = AssistantSettings.fromJson(settings.toJson());
    expect(decoded.harnessPath, settings.harnessPath);
    expect(decoded.modelValue, settings.modelValue);
    expect(decoded.workspaceMode, AssistantWorkspaceMode.custom);
    expect(decoded.workspacePath, '/tmp/workspace');
    expect(decoded.autoApprove, isTrue);
    expect(decoded.harnessKind, AssistantHarnessKind.auto);

    final malformed = AssistantSettings.fromJson({
      'harnessPath': 7,
      'autoApprove': 'yes',
    });
    expect(malformed.harnessPath, '');
    expect(malformed.autoApprove, isFalse);
  });

  test(
    'older settings keep a chosen folder and otherwise use the agent workspace',
    () {
      final chosen = AssistantSettings.fromJson({'workspacePath': '/tmp/old'});
      expect(chosen.workspaceMode, AssistantWorkspaceMode.custom);
      expect(chosen.workspacePath, '/tmp/old');

      final fresh = AssistantSettings.fromJson({});
      expect(fresh.workspaceMode, AssistantWorkspaceMode.agent);

      final invalid = AssistantSettings.fromJson({
        'workspaceMode': 'elsewhere',
      });
      expect(invalid.workspaceMode, AssistantWorkspaceMode.agent);
    },
  );

  test(
    'the desktop overlay supplies the provider catalog and selected model',
    () async {
      final directory = await Directory.systemTemp.createTemp('or-overlay-');
      addTearDown(() => directory.delete(recursive: true));
      final desktop = Directory('${directory.path}/desktop')
        ..createSync(recursive: true);
      File('${desktop.path}/cordis.patch.yml').writeAsStringSync('''
- id: agent-default-model
  config:
    provider: opencode-go
    model: deepseek-v4.1-flash
- id: llm-pi-ai
  name: "@deepseek-ai/dsh-llm-pi-ai"
  config:
    providers:
      opencode-go:
        apiKeyEnv: OPENCODE_GO_API_KEY
        models:
          - id: deepseek-v4.1-flash
            name: DeepSeek V4.1 Flash
- id: web
  config:
    searchProvider: ddg
''');

      final overlay = await findHarnessOverlay(profilesDirectory: directory);
      expect(overlay, isNotNull);
      expect(overlay!.provider, 'opencode-go');
      expect(overlay.model, 'deepseek-v4.1-flash');
      expect(overlay.patchBlock, contains('apiKeyEnv: OPENCODE_GO_API_KEY'));
      expect(overlay.patchBlock, isNot(contains('searchProvider')));

      final patch = buildHarnessPatch(
        overlay: overlay,
        workspace: '/tmp/project',
        projectName: 'Demo',
      );
      expect(patch, contains('- id: acp'));
      expect(patch, contains('provider: opencode-go'));
      expect(patch, contains('model: deepseek-v4.1-flash'));
      expect(patch, contains('- id: system-prompt'));
      expect(patch, contains('tools/or <verb> --attach project.json --json'));
      expect(patch, contains('tools/or project summary'));
      expect(patch, contains('mcp__or__motion_insert_scene'));
      expect(patch, isNot(contains('Use only the mcp__or__* tools')));
      expect(patch, contains('- id: llm-pi-ai'));
      expect(patch, isNot(contains('searchProvider')));
    },
  );

  test('a Windows harness path resolves with its .exe suffix', () async {
    final root = await Directory.systemTemp.createTemp('or-harness-win-');
    addTearDown(() => root.delete(recursive: true));
    final harness = File('${root.path}/dsh.exe')..writeAsStringSync('');
    const windows = HostPlatform(
      isWindows: true,
      isMacOS: false,
      isLinux: false,
      environment: {'USERPROFILE': r'C:\Users\ana', 'PATH': ''},
      executablePath: r'C:\or\or_app.exe',
    );

    final launch = await resolveHarnessLaunch(
      settings: AssistantSettings(harnessPath: '${root.path}/dsh'),
      patchPath: '${root.path}/agents.patch.yml',
      environment: windows.environment,
      platform: windows,
    );
    expect(launch, isNotNull);
    expect(launch!.executable, harness.path);
    expect(launch.kind, AssistantHarnessKind.dsh);
  });

  test('a harness on PATH is found without a configured path', () async {
    final root = await Directory.systemTemp.createTemp('or-harness-path-');
    addTearDown(() => root.delete(recursive: true));
    final bin = Directory('${root.path}/bin')..createSync();
    final harness = File('${bin.path}/dsh')..writeAsStringSync('');
    final host = HostPlatform(
      isWindows: false,
      isMacOS: false,
      isLinux: true,
      environment: {'HOME': root.path, 'PATH': bin.path},
      executablePath: '${root.path}/or_app',
    );

    final launch = await resolveHarnessLaunch(
      settings: const AssistantSettings(),
      patchPath: '${root.path}/agents.patch.yml',
      environment: host.environment,
      platform: host,
    );
    expect(launch, isNotNull);
    expect(launch!.executable, harness.path);
  });

  test('a stored DeepSeek path does not hijack another selection', () async {
    final root = await Directory.systemTemp.createTemp('or-harness-hijack-');
    addTearDown(() => root.delete(recursive: true));
    final bin = Directory('${root.path}/bin')..createSync();
    final dsh = File('${bin.path}/dsh')..writeAsStringSync('');
    final codex = File('${bin.path}/codex-acp')..writeAsStringSync('');
    final host = HostPlatform(
      isWindows: false,
      isMacOS: false,
      isLinux: true,
      environment: {'HOME': root.path, 'PATH': bin.path},
      executablePath: '${root.path}/or_app',
    );

    final launch = await resolveHarnessLaunch(
      settings: AssistantSettings(
        harnessKind: AssistantHarnessKind.codex,
        harnessPath: dsh.path,
      ),
      patchPath: '${root.path}/agents.patch.yml',
      environment: host.environment,
      platform: host,
    );
    expect(launch, isNotNull);
    expect(launch!.kind, AssistantHarnessKind.codex);
    expect(launch.executable, codex.path);
  });

  test('an explicit selection discovers its own executable', () async {
    final root = await Directory.systemTemp.createTemp('or-harness-pick-');
    addTearDown(() => root.delete(recursive: true));
    final bin = Directory('${root.path}/bin')..createSync();
    File('${bin.path}/dsh').writeAsStringSync('');
    File('${bin.path}/opencode').writeAsStringSync('');
    final host = HostPlatform(
      isWindows: false,
      isMacOS: true,
      isLinux: false,
      environment: {'HOME': root.path, 'PATH': bin.path},
      executablePath: '${root.path}/or_app',
    );

    final launch = await resolveHarnessLaunch(
      settings: const AssistantSettings(
        harnessKind: AssistantHarnessKind.opencode,
      ),
      patchPath: '${root.path}/agents.patch.yml',
      environment: host.environment,
      platform: host,
    );
    expect(launch, isNotNull);
    // The host may already carry its own copy in a system directory; either way
    // the selection must resolve to OpenCode and not to the installed DSH.
    expect(launch!.kind, AssistantHarnessKind.opencode);
    expect(launch.executable, endsWith('opencode'));
    expect(launch.arguments, ['acp']);
  });

  test('no harness anywhere resolves to no launch', () async {
    final root = await Directory.systemTemp.createTemp('or-harness-none-');
    addTearDown(() => root.delete(recursive: true));
    final host = HostPlatform(
      isWindows: false,
      isMacOS: false,
      isLinux: true,
      environment: {'HOME': root.path, 'PATH': ''},
      executablePath: '${root.path}/or_app',
    );
    final launch = await resolveHarnessLaunch(
      settings: const AssistantSettings(),
      patchPath: '${root.path}/agents.patch.yml',
      environment: host.environment,
      platform: host,
    );
    expect(launch, isNull);
  });

  test('a packaged app finds the CLI beside its executable', () async {
    final root = await Directory.systemTemp.createTemp('or-cli-bundle-');
    addTearDown(() => root.delete(recursive: true));
    final macos = Directory(
      '${root.path}/Opencut Reinforced.app/Contents/MacOS',
    )..createSync(recursive: true);
    final executable = '${macos.path}/or_app';
    File(executable).writeAsStringSync('');
    final cli = File('${macos.path}/or')
      ..writeAsStringSync('#!/bin/sh\nexit 0\n');

    final resolved = await resolveOrCliPath(
      settings: const AssistantSettings(),
      environment: const {'PATH': ''},
      currentDirectory: root,
      executablePath: executable,
    );
    expect(
      resolved != null && p.equals(resolved, cli.path),
      isTrue,
      reason: 'resolved $resolved from ${cli.path}',
    );
  });

  test(
    'an explicit CLI path wins and a missing one resolves to null',
    () async {
      final root = await Directory.systemTemp.createTemp('or-cli-explicit-');
      addTearDown(() => root.delete(recursive: true));
      final cli = File('${root.path}/or')..writeAsStringSync('');

      expect(
        await resolveOrCliPath(
          settings: AssistantSettings(orCliPath: cli.path),
          environment: const {'PATH': ''},
          currentDirectory: root,
          executablePath: '${root.path}/or_app',
        ),
        cli.path,
      );
      expect(
        await resolveOrCliPath(
          settings: AssistantSettings(orCliPath: '${root.path}/missing-or'),
          environment: const {'PATH': ''},
          currentDirectory: root,
          executablePath: '${root.path}/or_app',
        ),
        isNull,
      );
    },
  );

  test('no overlay still yields a usable assistant persona patch', () {
    final patch = buildHarnessPatch(
      overlay: null,
      workspace: '/tmp/project',
      projectName: 'Demo',
    );
    expect(patch, contains('- id: system-prompt'));
    expect(patch, isNot(contains('- id: acp')));
  });
  test(
    'the harness kind survives a round-trip and older files default to auto',
    () {
      const settings = AssistantSettings(
        harnessPath: '/opt/opencode',
        harnessKind: AssistantHarnessKind.opencode,
      );
      final decoded = AssistantSettings.fromJson(settings.toJson());
      expect(decoded.harnessKind, AssistantHarnessKind.opencode);
      expect(decoded.harnessPath, '/opt/opencode');

      expect(
        AssistantSettings.fromJson({'version': 2}).harnessKind,
        AssistantHarnessKind.auto,
      );
    },
  );

  test('a custom harness launches without DSH-only arguments', () async {
    final launch = await resolveHarnessLaunch(
      settings: AssistantSettings(
        harnessPath: Platform.resolvedExecutable,
        harnessKind: AssistantHarnessKind.custom,
      ),
      patchPath: '/tmp/patch.yml',
      environment: const {'PATH': '/usr/bin:/bin'},
    );
    expect(launch, isNotNull);
    expect(launch!.executable, Platform.resolvedExecutable);
    expect(launch.arguments, isEmpty);
    expect(launch.kind, AssistantHarnessKind.custom);
  });
}
