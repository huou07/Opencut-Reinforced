import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/assistant/assistant_controller.dart';
import 'package:or_app/assistant/assistant_models.dart';
import 'package:or_app/assistant/assistant_panel.dart';
import 'package:or_app/design/or_theme.dart';
import 'package:or_app/shell/app_shell.dart';
import 'package:or_app/shell/or_shortcuts.dart';
import 'package:flutter/services.dart';

void main() {
  Future<void> mount(
    WidgetTester tester,
    AssistantController controller,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: OrTheme.dark,
        home: Scaffold(
          body: SizedBox(
            width: 320,
            height: 720,
            child: AssistantPanel(controller: controller),
          ),
        ),
      ),
    );
    await tester.pump();
  }

  testWidgets('an empty ready conversation offers concrete examples', (
    tester,
  ) async {
    final controller = AssistantController()
      ..debugSeedConversation(state: AssistantConnectionState.ready);
    addTearDown(controller.dispose);
    await mount(tester, controller);

    expect(find.byKey(const ValueKey('assistant-empty')), findsOneWidget);
    expect(find.text('Ask for an edit'), findsOneWidget);
    expect(find.byKey(const ValueKey('assistant-composer')), findsOneWidget);
    expect(find.byKey(const ValueKey('assistant-send')), findsOneWidget);
    expect(find.byKey(const ValueKey('assistant-model')), findsOneWidget);
  });

  testWidgets('the composer receives typing keys through the shell map', (
    tester,
  ) async {
    final controller = AssistantController()
      ..debugSeedConversation(state: AssistantConnectionState.ready);
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      MaterialApp(
        theme: OrTheme.dark,
        home: Shortcuts(
          shortcuts: orShellShortcuts(textInputFocused: true),
          child: Scaffold(
            body: SizedBox(
              width: 320,
              height: 720,
              child: AssistantPanel(controller: controller),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('assistant-composer')));
    await tester.pump();
    expect(orTextFieldHasFocus(), isTrue);

    await tester.enterText(
      find.byKey(const ValueKey('assistant-composer')),
      'xin chào',
    );
    await tester.pump();
    final field = tester.widget<TextField>(
      find.byKey(const ValueKey('assistant-composer')),
    );
    expect(field.controller?.text, 'xin chào');

    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    await tester.pump();
    expect(field.controller?.text, 'xin chà');
  });

  testWidgets('a failed connection explains the fix instead of an empty rail', (
    tester,
  ) async {
    final controller = AssistantController()
      ..debugSeedConversation(
        state: AssistantConnectionState.error,
        error: 'DeepSeek Harness is not installed where the assistant can find it.',
      );
    addTearDown(controller.dispose);
    await mount(tester, controller);

    expect(find.byKey(const ValueKey('assistant-error')), findsOneWidget);
    expect(find.byKey(const ValueKey('assistant-retry')), findsOneWidget);
    expect(
      find.byKey(const ValueKey('assistant-open-settings')),
      findsOneWidget,
    );
    expect(find.textContaining('not installed'), findsOneWidget);
  });

  testWidgets('tool calls and replies stay readable in the transcript', (
    tester,
  ) async {
    final controller = AssistantController()
      ..debugSeedConversation(
        state: AssistantConnectionState.ready,
        entries: [
          AssistantEntry(
            id: 'user-1',
            kind: AssistantEntryKind.user,
            text: 'Trim this clip.',
          ),
          AssistantEntry(
            id: 'tool-1',
            kind: AssistantEntryKind.tool,
            title: 'Trimming the selected clip',
            toolName: 'mcp__or__trim_clip',
            state: AssistantToolState.done,
            detail: '{"edge":"end","at":"4"}',
          ),
          AssistantEntry(
            id: 'assistant-1',
            kind: AssistantEntryKind.assistant,
            text: 'Trimmed the clip to 4 s. Saved the project.',
          ),
        ],
      );
    addTearDown(controller.dispose);
    await mount(tester, controller);

    expect(find.text('Trim this clip.'), findsOneWidget);
    expect(find.text('Trimming the selected clip'), findsOneWidget);
    expect(find.text('trim_clip'), findsOneWidget);
    expect(find.textContaining('Trimmed the clip to 4 s'), findsOneWidget);
  });
  testWidgets('a tool image renders as a thumbnail with a Finder link', (
    tester,
  ) async {
    final entry = AssistantEntry(
      id: 'tool-image',
      kind: AssistantEntryKind.tool,
      toolName: 'Read',
      state: AssistantToolState.done,
      detail: 'Read image',
      images: [
        AssistantImage(
          bytes: Uint8List.fromList(
            base64Decode(
              'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk'
              '+A8AAQUBAScY42YAAAAASUVORK5CYII=',
            ),
          ),
          mimeType: 'image/png',
        ),
      ],
    );
    final controller = AssistantController()
      ..debugSeedConversation(state: AssistantConnectionState.ready, entries: [entry]);
    addTearDown(controller.dispose);
    await mount(tester, controller);
    await tester.pump();

    expect(
      find.byKey(const ValueKey('assistant-image-tool-image-0')),
      findsOneWidget,
    );
    expect(find.text('Show in Finder'), findsOneWidget);
  });

}
