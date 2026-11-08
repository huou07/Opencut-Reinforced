import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/library/creative_directions.dart';
import 'package:or_app/screens/creative_direction_editor_dialog.dart';

CreativeDirection _direction() => const CreativeDirection(
  id: 'cinematic-documentary-mine',
  name: 'Cinematic documentary (mine)',
  summary: 'Slow, observational, patient.',
  bestFor: ['a long story', 'a place', 'a person at work'],
  voice: 'Unhurried narration, second person, no adjectives for their own sake.',
  script: 'Hook on one concrete image, develop the cost, turn on the decision.',
  storytelling: 'Camera stays put and lets the scene finish.',
  graphics: 'Serif titles, one accent, no lower thirds.',
  captions: 'Two lines at most, keywords in the accent colour.',
  editing: 'Twelve-second shots, cuts on movement, no transitions.',
  audio: 'Room tone under everything, music only in the turn.',
  isMine: true,
);

Future<CreativeDirection?> _open(WidgetTester tester) async {
  CreativeDirection? result;
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: Center(
            child: ElevatedButton(
              onPressed: () async {
                result = await showDialog<CreativeDirection>(
                  context: context,
                  builder: (context) =>
                      CreativeDirectionEditorDialog(direction: _direction()),
                );
              },
              child: const Text('open'),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('open'));
  await tester.pumpAndSettle();
  return result;
}

void main() {
  testWidgets('the editor starts from the direction it was given', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1400, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await _open(tester);

    expect(find.text('Edit your creative direction'), findsOneWidget);
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('direction-edit-name')))
          .controller!
          .text,
      'Cinematic documentary (mine)',
    );
    expect(
      tester
          .widget<TextField>(find.byKey(const ValueKey('direction-edit-voice')))
          .controller!
          .text,
      contains('Unhurried narration'),
    );
    // A kept copy is complete, so Save is available straight away.
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const ValueKey('direction-edit-save')),
          )
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('an emptied section blocks saving until it is filled again', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1400, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await _open(tester);

    await tester.enterText(
      find.byKey(const ValueKey('direction-edit-audio')),
      '   ',
    );
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const ValueKey('direction-edit-save')),
          )
          .onPressed,
      isNull,
    );

    await tester.enterText(
      find.byKey(const ValueKey('direction-edit-audio')),
      'One music cue.',
    );
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const ValueKey('direction-edit-save')),
          )
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('saving returns the edited brief, not the bundled one', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1400, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    CreativeDirection? result;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => Scaffold(
            body: Center(
              child: ElevatedButton(
                onPressed: () async {
                  result = await showDialog<CreativeDirection>(
                    context: context,
                    builder: (context) => CreativeDirectionEditorDialog(
                      direction: _direction(),
                    ),
                  );
                },
                child: const Text('open'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const ValueKey('direction-edit-name')),
      'My documentary',
    );
    await tester.enterText(
      find.byKey(const ValueKey('direction-edit-best-for')),
      'a portrait; a process',
    );
    await tester.enterText(
      find.byKey(const ValueKey('direction-edit-graphics')),
      'Sans-serif titles, warm paper.',
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('direction-edit-save')));
    await tester.pumpAndSettle();

    expect(result, isNotNull);
    expect(result!.name, 'My documentary');
    expect(result!.bestFor, ['a portrait', 'a process']);
    expect(result!.graphics, 'Sans-serif titles, warm paper.');
    expect(result!.isMine, isTrue);
    // Untouched sections keep what the direction already said.
    expect(result!.editing, contains('Twelve-second shots'));
    expect(result!.toBrief(), contains('Creative direction: My documentary'));
  });

  testWidgets('Cancel returns nothing', (tester) async {
    tester.view.physicalSize = const Size(1400, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);

    CreativeDirection? result;
    var closed = false;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => Scaffold(
            body: Center(
              child: ElevatedButton(
                onPressed: () async {
                  result = await showDialog<CreativeDirection>(
                    context: context,
                    builder: (context) => CreativeDirectionEditorDialog(
                      direction: _direction(),
                    ),
                  );
                  closed = true;
                },
                child: const Text('open'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('direction-edit-cancel')));
    await tester.pumpAndSettle();

    expect(closed, isTrue);
    expect(result, isNull);
  });
}
