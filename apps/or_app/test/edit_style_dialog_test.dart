import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:or_app/library/creative_directions.dart';
import 'package:or_app/library/edit_styles.dart';
import 'package:or_app/screens/edit_style_plan_dialog.dart';

EditStyle _style(String id) =>
    bundledEditStyles.firstWhere((style) => style.id == id);

Future<bool?> _showPlan(WidgetTester tester, EditStylePlan plan,
    {String? title, bool appendsAfter = false}) async {
  bool? answer;
  await tester.pumpWidget(
    MaterialApp(
      home: Builder(
        builder: (context) => Scaffold(
          body: TextButton(
            onPressed: () async {
              answer = await showDialog<bool>(
                context: context,
                builder: (context) => EditStylePlanDialog(
                  plan: plan,
                  title: title,
                  labels: {
                    for (final shot in plan.shots) shot.mediaId: '${shot.mediaId}.mp4',
                  },
                  appendsAfter: appendsAfter,
                ),
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
  return answer;
}

void main() {
  testWidgets('a one-shot plan reads as one shot, not one shots', (
    tester,
  ) async {
    final plan = planEditStyleCut(
      style: _style('cinematic-documentary'),
      sources: const [
        EditStyleSource(
          mediaId: 'a',
          frames: 360,
          hasVideo: true,
          label: 'a.mp4',
        ),
      ],
      frameRate: 30,
    );

    await _showPlan(tester, plan, title: 'Repro');

    expect(find.text('Build a Cinematic documentary cut'), findsOneWidget);
    expect(find.text('1 shot · 8.0 s'), findsOneWidget);
    expect(find.byKey(const ValueKey('edit-style-shot-why-0')), findsOneWidget);
    expect(
      find.byKey(const ValueKey('edit-style-effect')),
      findsOneWidget,
    );
  });

  testWidgets('a plan with a cap explains what it left out', (tester) async {
    final plan = planEditStyleCut(
      style: _style('product-showcase'),
      sources: [
        for (var index = 0; index < 12; index++)
          EditStyleSource(
            mediaId: 'clip$index',
            frames: 300,
            hasVideo: true,
            label: 'clip$index.mp4',
          ),
      ],
      frameRate: 30,
    );

    await _showPlan(tester, plan);

    expect(find.textContaining('Used 8 of 12 clips'), findsOneWidget);
    expect(find.byKey(const ValueKey('edit-style-note-0')), findsOneWidget);
  });

  testWidgets('Build the cut returns true and Cancel returns false', (
    tester,
  ) async {
    final plan = planEditStyleCut(
      style: _style('personal-vlog'),
      sources: const [
        EditStyleSource(
          mediaId: 'a',
          frames: 300,
          hasVideo: true,
          label: 'a.mp4',
        ),
      ],
      frameRate: 30,
    );

    await _showPlan(tester, plan);
    await tester.tap(find.byKey(const ValueKey('edit-style-apply')));
    await tester.pumpAndSettle();
    expect(find.text('Build a Personal vlog cut'), findsNothing);

    await _showPlan(tester, plan);
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(find.text('Build a Personal vlog cut'), findsNothing);
  });

  testWidgets('a creative direction shows all seven brief sections', (
    tester,
  ) async {
    var copied = false;
    final contact = bundledCreativeDirections.first;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => TextButton(
              onPressed: () => showDialog<void>(
                context: context,
                builder: (context) => CreativeDirectionDialog(
                  direction: contact,
                  onCopy: () => copied = true,
                  onAddPinned: null,
                ),
              ),
              child: const Text('open'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    for (final heading in [
      'VOICE AND NARRATION',
      'SCRIPT SHAPE',
      'STORYTELLING',
      'GRAPHICS',
      'CAPTIONS',
      'EDITING',
      'AUDIO',
    ]) {
      expect(find.text(heading), findsOneWidget, reason: heading);
    }
    expect(find.byKey(const ValueKey('creative-direction-pin')), findsNothing);

    await tester.tap(find.byKey(const ValueKey('creative-direction-copy')));
    await tester.pumpAndSettle();
    expect(copied, isTrue);
  });
}
