import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/library/creative_directions.dart';
import 'package:or_app/screens/edit_style_plan_dialog.dart';

CreativeDirection _direction() => const CreativeDirection(
  id: 'warm',
  name: 'Warm personal narrative',
  summary: 'Close and specific.',
  bestFor: ['a personal essay'],
  voice: 'First person, low and unhurried.',
  script: 'One scene, one turn, no montage.',
  storytelling: 'Let the silence sit.',
  graphics: 'Soft serif, warm paper.',
  captions: 'One line, sentence case.',
  editing: 'Long takes, cuts on breath.',
  audio: 'Room tone, one piano note.',
);

Future<void> _open(WidgetTester tester, {required bool withProject}) async {
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: Center(
            child: ElevatedButton(
              onPressed: () => showDialog<void>(
                context: context,
                builder: (_) => CreativeDirectionDialog(
                  direction: _direction(),
                  onCopy: () {},
                  onAddPinned: null,
                  onPlanWithAssistant: withProject ? () {} : null,
                ),
              ),
              child: const Text('open brief'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open brief'));
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('an open project offers a plan from the brief', (tester) async {
    await _open(tester, withProject: true);
    expect(find.byKey(const ValueKey('creative-direction-plan')), findsOne);
    expect(find.text('Plan a video…'), findsOne);
  });

  testWidgets('without a project the brief offers no plan', (tester) async {
    await _open(tester, withProject: false);
    expect(find.byKey(const ValueKey('creative-direction-plan')), findsNothing);
  });

  testWidgets('planning closes the brief before it starts', (tester) async {
    var planned = false;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => Scaffold(
            body: Center(
              child: ElevatedButton(
                onPressed: () => showDialog<void>(
                  context: context,
                  builder: (_) => CreativeDirectionDialog(
                    direction: _direction(),
                    onCopy: () {},
                    onAddPinned: null,
                    onPlanWithAssistant: () => planned = true,
                  ),
                ),
                child: const Text('open brief'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open brief'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('creative-direction-plan')));
    await tester.pumpAndSettle();
    expect(planned, isTrue);
    expect(
      find.byKey(const ValueKey('creative-direction-brief')),
      findsNothing,
    );
  });
}
