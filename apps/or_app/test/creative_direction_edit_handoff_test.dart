import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/library/creative_directions.dart';
import 'package:or_app/screens/creative_direction_editor_dialog.dart';
import 'package:or_app/screens/edit_style_plan_dialog.dart';

CreativeDirection _mine() => const CreativeDirection(
  id: 'warm-personal-narrative-mine',
  name: 'Warm personal narrative (mine)',
  summary: 'Close and specific.',
  bestFor: ['a personal essay'],
  voice: 'First person, low and unhurried.',
  script: 'One scene, one turn, no montage.',
  storytelling: 'Let the silence sit.',
  graphics: 'Soft serif, warm paper.',
  captions: 'One line, sentence case.',
  editing: 'Long takes, cuts on breath.',
  audio: 'Room tone, one piano note.',
  isMine: true,
);

void main() {
  testWidgets('tapping Edit opens the editor after the brief closes', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1400, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => Scaffold(
            body: Center(
              child: ElevatedButton(
                onPressed: () => showDialog<void>(
                  context: context,
                  builder: (dialogContext) => CreativeDirectionDialog(
                    direction: _mine(),
                    onCopy: () {},
                    onAddPinned: null,
                    onEdit: () => showDialog<void>(
                      context: context,
                      builder: (_) =>
                          CreativeDirectionEditorDialog(direction: _mine()),
                    ),
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

    expect(find.byKey(const ValueKey('creative-direction-brief')), findsOne);
    await tester.tap(find.byKey(const ValueKey('creative-direction-edit')));
    await tester.pumpAndSettle();

    // The editor must survive the brief's own route being popped.
    expect(find.text('Edit your creative direction'), findsOneWidget);
    expect(find.byKey(const ValueKey('direction-edit-voice')), findsOneWidget);
    expect(find.byKey(const ValueKey('creative-direction-brief')), findsNothing);
  });

  testWidgets('a bundled direction offers no Edit action', (tester) async {
    tester.view.physicalSize = const Size(1400, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(
      MaterialApp(
        home: CreativeDirectionDialog(
          direction: _mine().copyWith(isMine: false),
          onCopy: () {},
          onAddPinned: () {},
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('creative-direction-edit')), findsNothing);
    expect(find.byKey(const ValueKey('creative-direction-pin')), findsOneWidget);
  });
}
