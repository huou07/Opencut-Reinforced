import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/assistant/assistant_settings.dart';

void main() {
  test('settings round-trip and reject malformed fields', () {
    const settings = AssistantSettings(
      harnessPath: '/opt/dsh',
      orCliPath: '/repo/target/debug/or',
      workspacePath: '/tmp/workspace',
      modelValue: '["opencode-go","deepseek-v4.1-flash"]',
      reasoningValue: 'high',
      autoApprove: true,
    );
    final decoded = AssistantSettings.fromJson(settings.toJson());
    expect(decoded.harnessPath, settings.harnessPath);
    expect(decoded.modelValue, settings.modelValue);
    expect(decoded.autoApprove, isTrue);

    final malformed = AssistantSettings.fromJson({
      'harnessPath': 7,
      'autoApprove': 'yes',
    });
    expect(malformed.harnessPath, '');
    expect(malformed.autoApprove, isFalse);
  });

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
      expect(patch, contains('mcp__or__project_status'));
      expect(patch, contains('- id: llm-pi-ai'));
      expect(patch, isNot(contains('searchProvider')));
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
}
