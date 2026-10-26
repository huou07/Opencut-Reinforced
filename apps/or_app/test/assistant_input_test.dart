import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/shell/app_shell.dart';
import 'package:or_app/shell/or_shortcuts.dart';

void main() {
  test('typing keys are not installed while a text field owns the keyboard', () {
    final textMap = orShellShortcuts(textInputFocused: true);
    final editorMap = orShellShortcuts(textInputFocused: false);

    // Text-editing and typing keys must not be captured by the shell.
    for (final activator in const [
      SingleActivator(LogicalKeyboardKey.backspace),
      SingleActivator(LogicalKeyboardKey.space),
      SingleActivator(LogicalKeyboardKey.keyS),
      SingleActivator(LogicalKeyboardKey.keyC),
      SingleActivator(LogicalKeyboardKey.keyV),
      SingleActivator(LogicalKeyboardKey.keyM),
      SingleActivator(LogicalKeyboardKey.digit1),
      SingleActivator(LogicalKeyboardKey.period),
      OrCommandActivator(LogicalKeyboardKey.keyZ),
      OrCommandActivator(LogicalKeyboardKey.keyC),
      OrCommandActivator(LogicalKeyboardKey.keyV),
    ]) {
      expect(textMap.containsKey(activator), isFalse, reason: '$activator');
      expect(editorMap.containsKey(activator), isTrue, reason: '$activator');
    }

    // Commands that do not collide with text editing stay available.
    expect(
      textMap.containsKey(const OrCommandActivator(LogicalKeyboardKey.keyS)),
      isTrue,
    );
    expect(
      textMap.containsKey(const OrCommandActivator(LogicalKeyboardKey.keyK)),
      isTrue,
    );
  });

  testWidgets('Backspace deletes text and never fires an editor verb', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'xin chào');
    addTearDown(controller.dispose);
    var deletedClips = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Shortcuts(
          shortcuts: orShellShortcuts(textInputFocused: true),
          child: Actions(
            actions: {
              DeleteTimelineClipIntent:
                  CallbackAction<DeleteTimelineClipIntent>(
                    onInvoke: (_) {
                      deletedClips += 1;
                      return null;
                    },
                  ),
            },
            child: Scaffold(
              body: TextField(
                key: const ValueKey('field'),
                controller: controller,
                autofocus: true,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    expect(orTextFieldHasFocus(), isTrue);

    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    await tester.pump();

    expect(controller.text, 'xin chà');
    expect(deletedClips, 0);
  });

  testWidgets('an editor shortcut still fires when no text field is focused', (
    tester,
  ) async {
    var deletedClips = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Shortcuts(
          shortcuts: orShellShortcuts(textInputFocused: false),
          child: Actions(
            actions: {
              DeleteTimelineClipIntent:
                  CallbackAction<DeleteTimelineClipIntent>(
                    onInvoke: (_) {
                      deletedClips += 1;
                      return null;
                    },
                  ),
            },
            child: const Focus(
              autofocus: true,
              child: Scaffold(body: SizedBox()),
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    await tester.pump();

    expect(deletedClips, 1);
  });

  testWidgets('composed Vietnamese input reaches the controller intact', (
    tester,
  ) async {
    final controller = TextEditingController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: TextField(
            key: const ValueKey('field'),
            controller: controller,
            autofocus: true,
          ),
        ),
      ),
    );
    await tester.pump();

    tester.testTextInput.updateEditingValue(
      const TextEditingValue(
        text: 'xin chào',
        composing: TextRange(start: 4, end: 8),
      ),
    );
    await tester.pump();
    expect(controller.text, 'xin chào');

    tester.testTextInput.updateEditingValue(
      const TextEditingValue(text: 'xin chào'),
    );
    await tester.pump();
    expect(controller.text, 'xin chào');
  });
}
