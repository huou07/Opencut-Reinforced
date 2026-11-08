import 'dart:math' as math;

/// An editing style: how a cut is paced and told, not what it is about.
///
/// A style is a recipe for the timeline — shot length, order, whether captions
/// and a title belong, and where the audio sits. It is deliberately independent
/// of the footage: the same style cuts a travel film and a product demo.
class EditStyle {
  const EditStyle({
    required this.id,
    required this.name,
    required this.summary,
    required this.description,
    required this.whenToUse,
    required this.pacing,
    required this.shotSeconds,
    required this.hookSeconds,
    required this.maxShots,
    required this.captions,
    required this.title,
    required this.titleText,
    required this.order,
    required this.audioNote,
  });

  final String id;
  final String name;

  /// One line, for the card.
  final String summary;

  /// Two or three sentences on how this style cuts and tells.
  final String description;

  /// Three situations the style suits.
  final List<String> whenToUse;

  /// How the cut moves, in words a person can check against the result.
  final String pacing;

  /// Target length of a middle shot.
  final double shotSeconds;

  /// Length of the first shot; the hook is usually longer than the rest.
  final double hookSeconds;

  /// How many shots the style is willing to cut.
  final int maxShots;

  final bool captions;
  final bool title;

  /// The title this style puts on the opening shot.
  final String titleText;

  /// How the style orders the material it was given.
  final EditStyleOrder order;

  /// Where sound sits under the cut.
  final String audioNote;

  String get pacingLabel => '${shotSeconds.toStringAsFixed(1)} s shots';
}

/// How a style picks and orders the footage it is handed.
enum EditStyleOrder {
  /// The longest material opens the film: a documentary lets a subject speak.
  longestFirst,

  /// The shortest usable material opens: a social cut spends its first second.
  shortestFirst,

  /// The library order the person imported; the cut follows their intention.
  importedOrder,
}

/// The bundled styles. Each one describes a way of cutting, not a subject.
const List<EditStyle> bundledEditStyles = [
  EditStyle(
    id: 'cinematic-documentary',
    name: 'Cinematic documentary',
    summary: 'Long holds, few cuts, the subject carries the film',
    description:
        'Scenes are allowed to finish. The camera stays on a face or a place '
        'until the moment is complete, and the cut moves when the thought does '
        'rather than on a beat. Sound is the room plus one low bed under it.',
    whenToUse: [
      'A person telling their own story',
      'A place that needs atmosphere before information',
      'Footage with long takes and natural sound',
    ],
    pacing: 'Few cuts, long holds, no rhythm fixed by music',
    shotSeconds: 6,
    hookSeconds: 8,
    maxShots: 6,
    captions: false,
    title: true,
    titleText: 'A short film',
    order: EditStyleOrder.longestFirst,
    audioNote:
        'Room tone under everything; one low bed, never louder than '
        'the voice',
  ),
  EditStyle(
    id: 'fast-paced-educational',
    name: 'Fast-paced educational',
    summary: 'Short shots that land on a sentence',
    description:
        'Every cut carries one idea. Shots are short enough to keep attention '
        'and long enough to be read, captions repeat the key word, and the '
        'opening states the promise of the video before any detail arrives.',
    whenToUse: [
      'Explaining a tool or a process',
      'A screen recording with a voiceover',
      'A video where the viewer came with a question',
    ],
    pacing: 'A cut every couple of seconds, on the sentence, never mid-word',
    shotSeconds: 2.5,
    hookSeconds: 1.5,
    maxShots: 10,
    captions: true,
    title: true,
    titleText: 'Here is how it works',
    order: EditStyleOrder.shortestFirst,
    audioNote: 'Voice first; music ducks 12 dB under every line',
  ),
  EditStyle(
    id: 'personal-vlog',
    name: 'Personal vlog',
    summary: 'Warm, unhurried, spoken like a message to a friend',
    description:
        'The voice leads and the picture follows it. Shots are as long as the '
        'sentence they belong to, captions are on because people watch without '
        'sound, and the cut keeps the small imperfections that make it a person '
        'rather than a presenter.',
    whenToUse: [
      'A day, a trip or a process you lived through',
      'Talking to camera with b-roll from the same day',
      'A channel where the person is the format',
    ],
    pacing: 'Unhurried, cuts follow the sentence, not a beat',
    shotSeconds: 4,
    hookSeconds: 3,
    maxShots: 8,
    captions: true,
    title: true,
    titleText: 'Today',
    order: EditStyleOrder.importedOrder,
    audioNote: 'Voice up front, quiet music under, room sound left in',
  ),
  EditStyle(
    id: 'product-showcase',
    name: 'Product showcase',
    summary: 'Reveal, then detail, then the one moment that sells it',
    description:
        'The opening shot shows the whole object, the next shots move closer to '
        'the detail that matters, and the cut ends on the product doing its job. '
        'Nothing is on screen for longer than it takes to understand.',
    whenToUse: [
      'A launch or a store listing',
      'A physical object with a clear feature to show',
      'Footage shot in controlled light',
    ],
    pacing: 'Reveal, detail, use — three moves, repeated',
    shotSeconds: 3,
    hookSeconds: 2,
    maxShots: 8,
    captions: false,
    title: true,
    titleText: 'Introducing',
    order: EditStyleOrder.longestFirst,
    audioNote: 'Clean bed with one accent on each cut; no voice unless written',
  ),
  EditStyle(
    id: 'interview-storytelling',
    name: 'Interview storytelling',
    summary: 'Long answers, cut on the pause, questions removed',
    description:
        'The interview is the spine and the pictures illustrate what is said. '
        'Answers are trimmed to their strongest sentence, the question is never '
        'heard, and the lower third appears once so the viewer knows who is '
        'speaking.',
    whenToUse: [
      'A recorded conversation with one or two people',
      'A case study or a testimonial',
      'When the words matter more than the pictures',
    ],
    pacing: 'Long answers, cut on the pause, b-roll over the dry passages',
    shotSeconds: 8,
    hookSeconds: 6,
    maxShots: 5,
    captions: true,
    title: true,
    titleText: 'In their words',
    order: EditStyleOrder.longestFirst,
    audioNote:
        'Voice clean and forward; b-roll sections keep the interview '
        'audio running underneath',
  ),
];

/// One shot the planner wants on the timeline.
class EditPlanShot {
  const EditPlanShot({
    required this.mediaId,
    required this.startFrame,
    required this.sourceStartFrame,
    required this.frameCount,
    required this.why,
  });

  final String mediaId;
  final int startFrame;
  final int sourceStartFrame;
  final int frameCount;

  /// Why this shot is here, in the words the review shows.
  final String why;

  String durationLabel(int frameRate) =>
      '${(frameCount / (frameRate <= 0 ? 30 : frameRate)).toStringAsFixed(1)} s';
}

/// A cut the planner can build, with the reasons it can be checked against.
class EditStylePlan {
  const EditStylePlan({
    required this.style,
    required this.shots,
    required this.frameRate,
    required this.notes,
  });

  final EditStyle style;
  final List<EditPlanShot> shots;
  final int frameRate;

  /// What the planner had to compromise on, in the order it noticed.
  final List<String> notes;

  int get totalFrames => shots.fold(0, (sum, shot) => sum + shot.frameCount);

  bool get isEmpty => shots.isEmpty;

  String get summary =>
      '${shots.length} ${shots.length == 1 ? 'shot' : 'shots'} · '
      '${(totalFrames / frameRate).toStringAsFixed(1)} s';
}

/// The material a plan can be built from, in the order it was imported.
class EditStyleSource {
  const EditStyleSource({
    required this.mediaId,
    required this.frames,
    required this.hasVideo,
    required this.label,
  });

  final String mediaId;
  final int frames;
  final bool hasVideo;
  final String label;
}

/// Turns a style plus the imported material into a concrete cut.
///
/// Pure on purpose: the pacing rules are the product behaviour, so they are
/// tested without a project, and the review a person sees is the same object
/// that is applied to the timeline.
EditStylePlan planEditStyleCut({
  required EditStyle style,
  required List<EditStyleSource> sources,
  required int frameRate,
}) {
  final notes = <String>[];
  final rate = frameRate <= 0 ? 30 : frameRate;
  final usable = sources
      .where((source) => source.hasVideo && source.frames > rate ~/ 4)
      .toList();
  if (usable.isEmpty) {
    return EditStylePlan(
      style: style,
      shots: const [],
      frameRate: rate,
      notes: const ['No video long enough to cut; import a clip first.'],
    );
  }
  final ordered = switch (style.order) {
    EditStyleOrder.longestFirst => [
      ...usable,
    ]..sort((a, b) => b.frames.compareTo(a.frames)),
    EditStyleOrder.shortestFirst => [
      ...usable,
    ]..sort((a, b) => a.frames.compareTo(b.frames)),
    EditStyleOrder.importedOrder => usable,
  };
  final maxShots = math.min(style.maxShots, ordered.length);
  if (ordered.length > maxShots) {
    notes.add(
      'Used $maxShots of ${ordered.length} clips; ${style.name} cuts '
      '${style.maxShots} shots at most.',
    );
  }
  final shots = <EditPlanShot>[];
  var start = 0;
  for (var index = 0; index < maxShots; index++) {
    final source = ordered[index];
    final wanted = index == 0 ? style.hookSeconds : style.shotSeconds;
    final frames = math.min((wanted * rate).round(), source.frames);
    if (frames <= 0) continue;
    shots.add(
      EditPlanShot(
        mediaId: source.mediaId,
        startFrame: start,
        sourceStartFrame: 0,
        frameCount: frames,
        why: index == 0
            ? 'Opens on ${source.label}: ${style.hookSeconds.toStringAsFixed(1)} s '
                  'to set the promise'
            : '${source.label}: ${(frames / rate).toStringAsFixed(1)} s, '
                  '${style.pacingLabel}',
      ),
    );
    start += frames;
  }
  if (shots.isEmpty) {
    notes.add('Nothing could be cut; every clip was shorter than a frame.');
  } else if (shots.length < maxShots) {
    notes.add(
      'The cut is shorter than the style wants: only ${shots.length} clips '
      'were usable.',
    );
  }
  return EditStylePlan(
    style: style,
    shots: shots,
    frameRate: rate,
    notes: notes,
  );
}

/// The title clip a style wants under or over the opening shot.
String editStyleTitle(EditStyle style, {String? projectName}) {
  final name = projectName?.trim();
  if (name != null && name.isNotEmpty) return name;
  return style.titleText;
}
