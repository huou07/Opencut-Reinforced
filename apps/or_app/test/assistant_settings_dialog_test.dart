import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/assistant/assistant_controller.dart';
import 'package:or_app/assistant/assistant_settings.dart';
import 'package:or_app/assistant/assistant_settings_dialog.dart';
import 'package:or_app/design/or_theme.dart';

void main() {
  testWidgets('settings offer the agent, project and custom workspaces', (
    tester,
  ) async {
    late Directory support;
    late AssistantController controller;
    // testWidgets runs in a fake-async zone, so real filesystem work belongs in
    // runAsync or its futures never complete.
    await tester.runAsync(() async {
      support = await Directory.systemTemp.createTemp('or-settings-');
      controller = AssistantController(
        supportDirectory: () async => support,
        settingsStore: AssistantSettingsStore(
          supportDirectory: () async => support,
        ),
      );
      await controller.initialize();
    });
    addTearDown(() async {
      controller.dispose();
      await support.delete(recursive: true);
    });

    await tester.pumpWidget(
      MaterialApp(
        theme: OrTheme.dark,
        home: Builder(
          builder: (context) => TextButton(
            key: const ValueKey('open-settings'),
            onPressed: () => showAssistantSettingsDialog(
              context: context,
              controller: controller,
            ),
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('open-settings')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));

    expect(
      find.byKey(const ValueKey('assistant-settings-workspace-mode')),
      findsOneWidget,
    );
    expect(find.text('Agent workspace'), findsOneWidget);
    expect(find.textContaining(controller.agentWorkspacePath), findsOneWidget);

    await tester.tap(
      find.byKey(const ValueKey('assistant-settings-workspace-mode')),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.text('Project folder'), findsWidgets);

    await tester.tap(find.text('Project folder').last);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.textContaining('project folder'), findsOneWidget);

    await tester.tap(find.text('Cancel'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));
    expect(controller.settings.workspaceMode, AssistantWorkspaceMode.agent);
  });
}
