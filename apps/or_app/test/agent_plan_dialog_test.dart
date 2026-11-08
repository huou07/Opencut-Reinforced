import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/design/or_theme.dart';
import 'package:or_app/library/agent_edit_plan.dart';
import 'package:or_app/screens/agent_plan_dialog.dart';

const _media = [
  AgentPlanMedia(mediaId: 'm-1', label: 'beach.mp4', seconds: 12),
  AgentPlanMedia(mediaId: 'm-2', label: 'talk.mp4', seconds: 40),
];

Widget _host(Widget child) => MaterialApp(
  theme: OrTheme.dark,
  home: Scaffold(body: Builder(builder: (context) => child)),
);

AgentEditPlan _plan() => parseAgentEditPlan(
  jsonEncode({
    'schema': agentEditPlanSchema,
    'frameRate': 30,
    'title': 'A short film',
    'notes': ['Only two clips were long enough'],
    'shots': [
      {'mediaId': 'm-1', 'frameCount': 90, 'why': 'Sets the place'},
      {
        'mediaId': 'm-2',
        'sourceStartFrame': 30,
        'frameCount': 60,
        'why': 'The subject speaks',
      },
    ],
  }),
);

void main() {
  testWidgets('pasting a valid plan closes with the parsed plan', (
    tester,
  ) async {
    AgentEditPlan? result;
    await tester.pumpWidget(
      _host(
        Builder(
          builder: (context) => TextButton(
            onPressed: () async {
              result = await showDialog<AgentEditPlan>(
                context: context,
                builder: (context) => AgentPlanImportDialog(
                  media: _media,
                  frameRate: 30,
                  brief: 'the brief',
                  parse: parseAgentEditPlan,
                ),
              );
            },
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('agent-plan-json')),
      jsonEncode({
        'schema': agentEditPlanSchema,
        'frameRate': 30,
        'shots': [
          {'mediaId': 'm-1', 'frameCount': 30, 'why': 'hook'},
        ],
      }),
    );
    await tester.tap(find.byKey(const ValueKey('agent-plan-import-review')));
    await tester.pumpAndSettle();
    expect(result, isNotNull);
    expect(result!.shots, hasLength(1));
    expect(result!.shots.single.why, 'hook');
    expect(find.byKey(const ValueKey('agent-plan-json')), findsNothing);
  });

  testWidgets('a malformed plan stays in the box with the reason', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(
        AgentPlanImportDialog(
          media: _media,
          frameRate: 30,
          parse: parseAgentEditPlan,
        ),
      ),
    );
    await tester.enterText(
      find.byKey(const ValueKey('agent-plan-json')),
      '{"schema": "wrong"}',
    );
    await tester.tap(find.byKey(const ValueKey('agent-plan-import-review')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('agent-plan-import-error')),
      findsOneWidget,
    );
    final error = tester.widget<Text>(
      find.byKey(const ValueKey('agent-plan-import-error')),
    );
    expect(error.data, contains('or.edit-plan/1'));
  });

  testWidgets('an empty box asks for the plan instead of parsing', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(AgentPlanImportDialog(media: _media, frameRate: 30)),
    );
    await tester.tap(find.byKey(const ValueKey('agent-plan-import-review')));
    await tester.pumpAndSettle();
    expect(find.textContaining('Paste the plan JSON'), findsOneWidget);
  });

  testWidgets('copying the brief puts it on the clipboard', (tester) async {
    String? copied;
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'Clipboard.setData') {
          copied = (call.arguments as Map)['text'] as String?;
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    await tester.pumpWidget(
      _host(
        AgentPlanImportDialog(
          media: _media,
          frameRate: 30,
          brief: 'plan the cut',
        ),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('agent-plan-copy-brief')));
    await tester.pumpAndSettle();
    expect(copied, 'plan the cut');
    expect(find.textContaining('Brief copied'), findsOneWidget);
  });

  testWidgets('the review lists every shot, its reason and the title', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(
        AgentPlanReviewDialog(
          plan: _plan(),
          labels: const {'m-1': 'beach.mp4', 'm-2': 'talk.mp4'},
          appendsAfter: true,
        ),
      ),
    );
    expect(find.textContaining('2 shots · 5.0 s'), findsOneWidget);
    expect(find.textContaining('beach.mp4 · 3.0 s'), findsOneWidget);
    expect(find.textContaining('talk.mp4 · 2.0 s'), findsOneWidget);
    expect(find.text('Sets the place'), findsOneWidget);
    expect(find.text('The subject speaks'), findsOneWidget);
    expect(find.textContaining('Only two clips'), findsOneWidget);
    expect(find.textContaining('A short film'), findsOneWidget);
    expect(find.textContaining('after the clips already'), findsOneWidget);
    expect(find.byKey(const ValueKey('agent-plan-apply')), findsOneWidget);
  });

  testWidgets('the review applies only when the button is pressed', (
    tester,
  ) async {
    bool? approved;
    await tester.pumpWidget(
      _host(
        Builder(
          builder: (context) => TextButton(
            onPressed: () async {
              approved = await showDialog<bool>(
                context: context,
                builder: (context) => AgentPlanReviewDialog(
                  plan: _plan(),
                  labels: const {'m-1': 'beach.mp4', 'm-2': 'talk.mp4'},
                  appendsAfter: false,
                ),
              );
            },
            child: const Text('open'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('agent-plan-apply')));
    await tester.pumpAndSettle();
    expect(approved, isTrue);
  });
}
