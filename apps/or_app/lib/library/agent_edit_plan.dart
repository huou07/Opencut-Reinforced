import 'dart:convert';

/// The plan format an assistant writes when it proposes a cut.
///
/// A plan is data, not a script: OR parses it, checks every reference against
/// the project it will be applied to, shows the result for review, and only
/// then writes ordinary clips. The schema is versioned because a plan written
/// today may be opened by a later build.
const agentEditPlanSchema = 'or.edit-plan/1';

/// One shot in an assistant plan, in frames at the plan's rate.
class AgentPlanShot {
  const AgentPlanShot({
    required this.mediaId,
    required this.name,
    required this.sourceStartFrame,
    required this.frameCount,
    required this.why,
  });

  /// The id the brief listed, when the assistant used it.
  final String mediaId;

  /// The file name, for a plan written by hand or by an assistant that only
  /// saw the media list. OR resolves it to exactly one media item.
  final String? name;
  final int sourceStartFrame;
  final int frameCount;

  /// The reason this shot is here, in the assistant's own words.
  final String why;
}

/// A parsed, validated plan. Construction is private so every instance has
/// passed [parseAgentEditPlan].
class AgentEditPlan {
  const AgentEditPlan._({
    required this.frameRate,
    required this.title,
    required this.notes,
    required this.shots,
  });

  final int frameRate;
  final String? title;
  final List<String> notes;
  final List<AgentPlanShot> shots;

  int get totalFrames => shots.fold(0, (sum, shot) => sum + shot.frameCount);

  String get summary =>
      '${shots.length} ${shots.length == 1 ? 'shot' : 'shots'} · '
      '${(totalFrames / frameRate).toStringAsFixed(1)} s';
}

/// A plan that cannot be applied, with a sentence a person can act on.
class AgentPlanFormatException implements Exception {
  const AgentPlanFormatException(this.message);

  final String message;

  @override
  String toString() => message;
}

/// Reads a plan and rejects anything the timeline could not honour.
///
/// Validation is strict on purpose: a plan that half-applies is worse than a
/// plan that is refused with a reason. Unknown extra keys are ignored so a
/// newer assistant can add commentary without breaking an older OR.
AgentEditPlan parseAgentEditPlan(String source) {
  final Object? decoded;
  try {
    decoded = jsonDecode(source);
  } on FormatException catch (error) {
    throw AgentPlanFormatException(
      'This is not valid JSON (${error.message}). Ask the assistant for the '
      'plan again, or paste only the JSON object.',
    );
  }
  if (decoded is! Map<String, Object?>) {
    throw const AgentPlanFormatException(
      'The plan must be one JSON object, not a list or a bare value.',
    );
  }
  final schema = decoded['schema'];
  if (schema != agentEditPlanSchema) {
    throw AgentPlanFormatException(
      'Expected "schema": "$agentEditPlanSchema", found '
      '${schema == null ? 'no schema' : '"$schema"'}.',
    );
  }
  final frameRate = decoded['frameRate'];
  if (frameRate is! int || frameRate < 1 || frameRate > 240) {
    throw const AgentPlanFormatException(
      'The plan needs an integer "frameRate" between 1 and 240.',
    );
  }
  final rawShots = decoded['shots'];
  if (rawShots is! List<Object?> || rawShots.isEmpty) {
    throw const AgentPlanFormatException(
      'The plan needs a non-empty "shots" list.',
    );
  }
  if (rawShots.length > 500) {
    throw const AgentPlanFormatException(
      'The plan has more than 500 shots; split it into smaller plans.',
    );
  }
  final shots = <AgentPlanShot>[];
  for (final (index, raw) in rawShots.indexed) {
    final number = index + 1;
    if (raw is! Map<String, Object?>) {
      throw AgentPlanFormatException('Shot $number is not a JSON object.');
    }
    final rawMediaId = raw['mediaId'];
    final rawName = raw['name'];
    final mediaId = rawMediaId is String ? rawMediaId.trim() : '';
    final name = rawName is String ? rawName.trim() : '';
    if (mediaId.isEmpty && name.isEmpty) {
      throw AgentPlanFormatException(
        'Shot $number needs a "mediaId" or a "name".',
      );
    }
    if (rawMediaId != null && rawMediaId is! String) {
      throw AgentPlanFormatException(
        'Shot $number has a "mediaId" that is not text.',
      );
    }
    if (rawName != null && rawName is! String) {
      throw AgentPlanFormatException(
        'Shot $number has a "name" that is not text.',
      );
    }
    final frameCount = raw['frameCount'];
    if (frameCount is! int || frameCount < 1) {
      throw AgentPlanFormatException(
        'Shot $number needs a positive integer "frameCount".',
      );
    }
    if (frameCount > frameRate * 600) {
      throw AgentPlanFormatException(
        'Shot $number is longer than ten minutes at $frameRate fps.',
      );
    }
    final sourceStartFrame = raw['sourceStartFrame'] ?? 0;
    if (sourceStartFrame is! int || sourceStartFrame < 0) {
      throw AgentPlanFormatException(
        'Shot $number needs "sourceStartFrame" as zero or a positive integer.',
      );
    }
    final why = raw['why'];
    if (why != null && why is! String) {
      throw AgentPlanFormatException(
        'Shot $number has a "why" that is not text.',
      );
    }
    shots.add(
      AgentPlanShot(
        mediaId: mediaId,
        name: name.isEmpty ? null : name,
        sourceStartFrame: sourceStartFrame,
        frameCount: frameCount,
        why: (why as String?)?.trim() ?? '',
      ),
    );
  }
  final rawTitle = decoded['title'];
  if (rawTitle != null && (rawTitle is! String || rawTitle.length > 200)) {
    throw const AgentPlanFormatException(
      'A plan title must be text of 200 characters or fewer.',
    );
  }
  final rawNotes = decoded['notes'];
  if (rawNotes != null &&
      (rawNotes is! List<Object?> ||
          rawNotes.length > 20 ||
          rawNotes.any((note) => note is! String))) {
    throw const AgentPlanFormatException(
      'Plan notes must be a list of at most twenty strings.',
    );
  }
  final title = (rawTitle as String?)?.trim();
  return AgentEditPlan._(
    frameRate: frameRate,
    title: title == null || title.isEmpty ? null : title,
    notes: rawNotes == null
        ? const []
        : List<String>.unmodifiable((rawNotes as List<Object?>).cast<String>()),
    shots: List<AgentPlanShot>.unmodifiable(shots),
  );
}

/// The material a brief can name, so the assistant plans against real ids.
class AgentPlanMedia {
  const AgentPlanMedia({
    required this.mediaId,
    required this.label,
    required this.seconds,
  });

  final String mediaId;
  final String label;
  final double seconds;
}

/// A prompt an assistant can answer with a plan.
///
/// The brief is the whole contract: the schema, the media that exists, and the
/// limits OR enforces. It is text on purpose — it can be pasted into the
/// Assistant panel or any other agent, and the answer can be reviewed the same
/// way whichever produced it.
String buildAgentPlanBrief({
  required List<AgentPlanMedia> media,
  required int frameRate,
  String? direction,
  String? style,
}) {
  final buffer = StringBuffer()
    ..writeln('Plan a cut for the project open in Opencut Reinforced.')
    ..writeln()
    ..writeln('Reply with one JSON object and nothing else. Schema:')
    ..writeln()
    ..writeln('```json')
    ..writeln('{')
    ..writeln('  "schema": "$agentEditPlanSchema",')
    ..writeln('  "frameRate": $frameRate,')
    ..writeln('  "title": "optional opening title",')
    ..writeln('  "notes": ["what you compromised on, if anything"],')
    ..writeln('  "shots": [')
    ..writeln('    {"mediaId": "<one of the ids below>",')
    ..writeln('     "name": "optional file name, when you only know that",')
    ..writeln('     "sourceStartFrame": 0,')
    ..writeln('     "frameCount": ${frameRate * 3},')
    ..writeln('     "why": "one sentence on why this shot is here"}')
    ..writeln('  ]')
    ..writeln('}')
    ..writeln('```')
    ..writeln()
    ..writeln('Rules:')
    ..writeln(
      '- Use only the media ids below; an unknown id rejects the plan.',
    );
  if (media.isEmpty) {
    buffer.writeln('- No media is imported yet, so first say what to import.');
  }
  buffer
    ..writeln('- Shots play in list order, back to back, on one video track.')
    ..writeln(
      '- "frameCount" and "sourceStartFrame" are whole frames at '
      '$frameRate fps.',
    )
    ..writeln('- A shot may not run past the end of its source.')
    ..writeln('- At most 500 shots; keep the total under ten minutes.');
  if (style != null && style.trim().isNotEmpty) {
    buffer
      ..writeln()
      ..writeln('Edit style: ${style.trim()}');
  }
  if (direction != null && direction.trim().isNotEmpty) {
    buffer
      ..writeln()
      ..writeln('Creative direction: ${direction.trim()}');
  }
  if (media.isNotEmpty) {
    buffer
      ..writeln()
      ..writeln('Media in the project:');
    for (final item in media) {
      buffer.writeln(
        '- ${item.mediaId} — ${item.label} '
        '(${item.seconds.toStringAsFixed(1)} s)',
      );
    }
  }
  return buffer.toString();
}
