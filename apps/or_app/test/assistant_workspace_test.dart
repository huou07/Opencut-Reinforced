import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/assistant/assistant_controller.dart';
import 'package:or_app/assistant/assistant_settings.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<(AssistantController, Directory)> controllerWithSupport() async {
    final support = await Directory.systemTemp.createTemp('or-assistant-');
    addTearDown(() => support.delete(recursive: true));
    final controller = AssistantController(
      supportDirectory: () async => support,
      settingsStore: AssistantSettingsStore(
        supportDirectory: () async => support,
      ),
    );
    addTearDown(controller.dispose);
    return (controller, support);
  }

  test('the agent workspace is the default session folder', () async {
    final (controller, support) = await controllerWithSupport();
    await controller.initialize();

    expect(controller.workspaceMode, AssistantWorkspaceMode.agent);
    expect(
      controller.workspace,
      '${support.path}${Platform.pathSeparator}agent-workspace',
    );
    expect(
      File('${controller.workspace}${Platform.pathSeparator}AGENTS.md')
          .existsSync(),
      isTrue,
    );
    expect(
      File(
        '${controller.workspace}${Platform.pathSeparator}docs'
        '${Platform.pathSeparator}timeline-editing.md',
      ).existsSync(),
      isTrue,
    );
  });

  test(
    'opening a project records it without moving the agent workspace',
    () async {
      final (controller, _) = await controllerWithSupport();
      await controller.initialize();
      final workspace = controller.workspace;

      await controller.updateProject(
        descriptorPath: '/tmp/demo.or-session',
        projectPath: '/tmp/demo.orproj',
        projectName: 'Demo',
      );

      expect(controller.workspace, workspace);
      final marker = jsonDecode(
        File('$workspace${Platform.pathSeparator}project.json')
            .readAsStringSync(),
      );
      expect(marker['project_name'], 'Demo');
      expect(marker['project_path'], '/tmp/demo.orproj');
    },
  );

  test(
    'a stored custom workspace is used instead of the agent workspace',
    () async {
      final (controller, support) = await controllerWithSupport();
      final custom = Directory('${support.path}/chosen')
        ..createSync(recursive: true);
      final store = AssistantSettingsStore(
        supportDirectory: () async => support,
      );
      await store.save(
        AssistantSettings(
          workspaceMode: AssistantWorkspaceMode.custom,
          workspacePath: custom.path,
        ),
      );
      await controller.initialize();

      expect(controller.workspaceMode, AssistantWorkspaceMode.custom);
      expect(controller.workspace, custom.path);
    },
  );

  test('the project folder mode follows the open project', () async {
    final (controller, support) = await controllerWithSupport();
    final store = AssistantSettingsStore(supportDirectory: () async => support);
    await store.save(
      const AssistantSettings(workspaceMode: AssistantWorkspaceMode.project),
    );
    await controller.initialize();
    await controller.updateProject(
      descriptorPath: '/tmp/demo.or-session',
      projectPath: '/tmp/demo-folder/demo.orproj',
      projectName: 'Demo',
    );

    expect(controller.workspace, '/tmp/demo-folder');
  });

  test('a settings change persists the workspace mode', () async {
    final (controller, support) = await controllerWithSupport();
    await controller.initialize();

    await controller.updateSettings(
      controller.settings.copyWith(
        workspaceMode: AssistantWorkspaceMode.project,
      ),
    );

    expect(controller.workspaceMode, AssistantWorkspaceMode.project);
    final store = AssistantSettingsStore(supportDirectory: () async => support);
    final stored = await store.load();
    expect(stored.workspaceMode, AssistantWorkspaceMode.project);
    expect(stored.workspacePath, '');
  });
}
