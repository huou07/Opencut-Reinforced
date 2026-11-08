import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/library/edit_styles.dart';
import 'package:or_app/screens/edit_style_plan_dialog.dart';

void main() {
  Future<EditStyle?> show(WidgetTester tester) async {
    EditStyle? picked;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => TextButton(
              onPressed: () async {
                picked = await showDialog<EditStyle>(
                  context: context,
                  builder: (_) =>
                      const EditStylePickerDialog(styles: bundledEditStyles),
                );
              },
              child: const Text('open'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    return picked;
  }

  testWidgets('offers every bundled style and returns the chosen one', (
    tester,
  ) async {
    await show(tester);
    for (final style in bundledEditStyles) {
      expect(
        find.byKey(ValueKey('edit-style-pick-${style.id}')),
        findsOneWidget,
      );
    }
    await tester.tap(
      find.byKey(const ValueKey('edit-style-pick-fast-paced-educational')),
    );
    await tester.pumpAndSettle();
    expect(find.byType(EditStylePickerDialog), findsNothing);
  });

  testWidgets('says the plan is reviewed before anything is written', (
    tester,
  ) async {
    await show(tester);
    expect(
      find.byKey(const ValueKey('edit-style-picker-copy')),
      findsOneWidget,
    );
    expect(find.textContaining('review the plan'), findsOneWidget);
    expect(find.textContaining('a title clip'), findsOneWidget);
  });

  testWidgets('cancel closes without a style', (tester) async {
    await show(tester);
    await tester.tap(find.byKey(const ValueKey('edit-style-picker-cancel')));
    await tester.pumpAndSettle();
    expect(find.byType(EditStylePickerDialog), findsNothing);
  });
}
