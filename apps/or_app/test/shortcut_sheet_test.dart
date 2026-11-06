import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/shell/shortcut_sheet.dart';

/// Reads the key names bound in `lib/shell/app_shell.dart`.
Set<String> _boundKeys() {
  final source = File('lib/shell/app_shell.dart').readAsStringSync();
  final keys = <String>{};
  for (final marker in const [
    '_alwaysShortcuts = {',
    '_editingShortcuts = {',
  ]) {
    final start = source.indexOf(marker);
    expect(
      start,
      greaterThan(0),
      reason: '$marker is gone from app_shell.dart',
    );
    final end = source.indexOf('\n};', start);
    expect(end, greaterThan(start), reason: '$marker is not closed');
    final block = source.substring(start, end);
    for (final match in RegExp(
      r'LogicalKeyboardKey\.(\w+)',
    ).allMatches(block)) {
      keys.add(match.group(1)!);
    }
  }
  return keys;
}

/// How a bound key is printed in the sheet.
const Map<String, String> _printed = {
  'keyK': 'K',
  'keyS': 'S',
  'keyE': 'E',
  'keyA': 'A',
  'keyI': 'I',
  'keyZ': 'Z',
  'keyD': 'D',
  'keyB': 'B',
  'keyC': 'C',
  'keyV': 'V',
  'keyM': 'M',
  'keyG': 'G',
  'space': 'Space',
  'arrowLeft': '←',
  'arrowRight': '→',
  'arrowUp': '↑',
  'arrowDown': '↓',
  'bracketLeft': '[',
  'bracketRight': ']',
  'comma': ',',
  'period': '.',
  'equal': '+',
  'minus': '−',
  'numpadAdd': '+',
  'numpadSubtract': '−',
  'f5': 'F5',
  'delete': 'Delete',
  'backspace': 'Delete',
  'escape': 'Esc',
  'digit1': '1',
  'digit2': '2',
  'digit3': '3',
  'digit4': '4',
  'slash': '/',
};

void main() {
  test('every key the app binds is printed in the sheet', () {
    final printed = orShortcutGroups
        .expand((group) => group.rows)
        .map((row) => row.keys)
        .join(' ');
    final missing = <String>[];
    for (final key in _boundKeys()) {
      final label = _printed[key];
      expect(
        label,
        isNotNull,
        reason:
            'LogicalKeyboardKey.$key is bound but the sheet has no label '
            'for it yet; add one to _printed and to orShortcutGroups',
      );
      if (!printed.contains(label!)) missing.add('$key → $label');
    }
    expect(missing, isEmpty, reason: 'bound but not listed: $missing');
  });

  testWidgets('the sheet lists the groups and closes on its button', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => Scaffold(
            body: TextButton(
              onPressed: () => showOrShortcutSheet(context),
              child: const Text('shortcuts'),
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text('shortcuts'));
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('shortcut-sheet')), findsOneWidget);
    expect(find.text('Keyboard shortcuts'), findsOneWidget);
    // The sheet scrolls, so each group is built only when it comes into view.
    for (final group in orShortcutGroups) {
      await tester.scrollUntilVisible(
        find.text(group.title),
        120,
        scrollable: find.byType(Scrollable),
      );
      expect(
        find.text(group.title),
        findsOneWidget,
        reason: 'the ${group.title} group is missing',
      );
    }
    await tester.scrollUntilVisible(
      find.byKey(const ValueKey('shortcut-sheet-row-Split at the playhead')),
      -120,
      scrollable: find.byType(Scrollable),
    );
    expect(
      find.byKey(const ValueKey('shortcut-sheet-row-Split at the playhead')),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const ValueKey('shortcut-sheet-close')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('shortcut-sheet')), findsNothing);
  });
}
