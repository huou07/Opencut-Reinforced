import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/library/agent_edit_plan.dart';

String _plan({
  Object? schema = agentEditPlanSchema,
  Object? frameRate = 30,
  Object? title,
  Object? notes,
  Object? shots,
}) {
  return jsonEncode({
    'schema': schema,
    'frameRate': frameRate,
    'title': ?title,
    'notes': ?notes,
    'shots':
        shots ??
        [
          {
            'mediaId': 'media-a',
            'sourceStartFrame': 0,
            'frameCount': 90,
            'why': 'Opens on the subject',
          },
        ],
  });
}

void main() {
  test('parses a valid plan and totals its shots', () {
    final plan = parseAgentEditPlan(
      _plan(
        title: '  A short film  ',
        notes: ['Used three of five clips'],
        shots: [
          {'mediaId': 'a', 'frameCount': 60, 'why': 'hook'},
          {
            'mediaId': 'b',
            'sourceStartFrame': 15,
            'frameCount': 30,
            'why': 'reply',
          },
        ],
      ),
    );
    expect(plan.frameRate, 30);
    expect(plan.title, 'A short film');
    expect(plan.notes, ['Used three of five clips']);
    expect(plan.shots, hasLength(2));
    expect(plan.shots[1].sourceStartFrame, 15);
    expect(plan.totalFrames, 90);
    expect(plan.summary, '2 shots · 3.0 s');
  });

  test('an empty title becomes no title', () {
    expect(parseAgentEditPlan(_plan(title: '   ')).title, isNull);
  });

  test('rejects invalid JSON with an actionable sentence', () {
    expect(
      () => parseAgentEditPlan('{not json'),
      throwsA(
        isA<AgentPlanFormatException>().having(
          (error) => error.message,
          'message',
          contains('not valid JSON'),
        ),
      ),
    );
  });

  test('rejects a different schema', () {
    expect(
      () => parseAgentEditPlan(_plan(schema: 'or.edit-plan/2')),
      throwsA(
        isA<AgentPlanFormatException>().having(
          (error) => error.message,
          'message',
          contains('or.edit-plan/1'),
        ),
      ),
    );
  });

  test('rejects a frame rate outside 1..240', () {
    for (final rate in [0, 241, '30']) {
      expect(
        () => parseAgentEditPlan(_plan(frameRate: rate)),
        throwsA(isA<AgentPlanFormatException>()),
        reason: 'frameRate $rate',
      );
    }
  });

  test('rejects an empty shot list', () {
    expect(
      () => parseAgentEditPlan(_plan(shots: [])),
      throwsA(
        isA<AgentPlanFormatException>().having(
          (error) => error.message,
          'message',
          contains('non-empty "shots"'),
        ),
      ),
    );
  });

  test('rejects a shot without a media id or with a zero length', () {
    expect(
      () => parseAgentEditPlan(
        _plan(
          shots: [
            {'frameCount': 30},
          ],
        ),
      ),
      throwsA(
        isA<AgentPlanFormatException>().having(
          (error) => error.message,
          'message',
          contains('Shot 1 needs a "mediaId" or a "name"'),
        ),
      ),
    );
    expect(
      () => parseAgentEditPlan(
        _plan(
          shots: [
            {'mediaId': 'a', 'frameCount': 0},
          ],
        ),
      ),
      throwsA(
        isA<AgentPlanFormatException>().having(
          (error) => error.message,
          'message',
          contains('positive integer "frameCount"'),
        ),
      ),
    );
  });

  test('a shot may name its media when it has no id', () {
    final plan = parseAgentEditPlan(
      _plan(
        shots: [
          {'name': 'beach.mp4', 'frameCount': 30},
        ],
      ),
    );
    expect(plan.shots.single.mediaId, isEmpty);
    expect(plan.shots.single.name, 'beach.mp4');
  });

  test('rejects a negative source start and malformed notes', () {
    expect(
      () => parseAgentEditPlan(
        _plan(
          shots: [
            {'mediaId': 'a', 'frameCount': 30, 'sourceStartFrame': -1},
          ],
        ),
      ),
      throwsA(isA<AgentPlanFormatException>()),
    );
    expect(
      () => parseAgentEditPlan(_plan(notes: 'not a list')),
      throwsA(
        isA<AgentPlanFormatException>().having(
          (error) => error.message,
          'message',
          contains('notes'),
        ),
      ),
    );
  });

  test('the brief names the schema, the rate and every media id', () {
    final brief = buildAgentPlanBrief(
      media: const [
        AgentPlanMedia(mediaId: 'm-1', label: 'beach.mp4', seconds: 12.5),
        AgentPlanMedia(mediaId: 'm-2', label: 'talk.mp4', seconds: 40),
      ],
      frameRate: 24,
      direction: 'Calm, observational',
      style: 'Long holds',
    );
    expect(brief, contains(agentEditPlanSchema));
    expect(brief, contains('"frameRate": 24'));
    expect(brief, contains('m-1'));
    expect(brief, contains('beach.mp4'));
    expect(brief, contains('40.0 s'));
    expect(brief, contains('Calm, observational'));
    expect(brief, contains('Long holds'));
    expect(brief, contains('Reply with one JSON object'));
  });

  test('the brief says so when the project has no media', () {
    final brief = buildAgentPlanBrief(media: const [], frameRate: 30);
    expect(brief, contains('No media is imported yet'));
  });
}
