/// A creative direction: the brief an agent works from, not a script.
///
/// Each field answers one question about how a video should feel and be built.
/// Nothing here names a subject, so the same direction can carry a product
/// launch, a travel film or a lesson. A person can copy a direction, edit any
/// field, and keep their own.
class CreativeDirection {
  const CreativeDirection({
    required this.id,
    required this.name,
    required this.summary,
    required this.bestFor,
    required this.voice,
    required this.script,
    required this.storytelling,
    required this.graphics,
    required this.captions,
    required this.editing,
    required this.audio,
    this.isMine = false,
  });

  final String id;
  final String name;

  /// Two sentences on the overall style.
  final String summary;

  /// Three kinds of video this direction fits.
  final List<String> bestFor;

  /// Who speaks, in what tone, at what pace.
  final String voice;

  /// How the script is shaped: hook, development, turn, ending.
  final String script;

  /// Where the camera stands and how the story is told.
  final String storytelling;

  /// Visual identity: type, colour, layout, motion.
  final String graphics;

  /// Subtitle look: line breaks, emphasis, animation.
  final String captions;

  /// Pacing, cuts, transitions, b-roll, effects.
  final String editing;

  /// Music, effects, ambience, ducking.
  final String audio;

  /// True for a direction the person saved themselves.
  final bool isMine;

  CreativeDirection copyWith({String? id, String? name, bool? isMine}) =>
      CreativeDirection(
        id: id ?? this.id,
        name: name ?? this.name,
        summary: summary,
        bestFor: bestFor,
        voice: voice,
        script: script,
        storytelling: storytelling,
        graphics: graphics,
        captions: captions,
        editing: editing,
        audio: audio,
        isMine: isMine ?? this.isMine,
      );

  /// The brief handed to an agent, in the order a video is decided.
  String toBrief() =>
      '''
Creative direction: $name

$summary

Fits: ${bestFor.join('; ')}

Voice and narration
$voice

Script shape
$script

Storytelling
$storytelling

Graphics
$graphics

Captions
$captions

Editing
$editing

Audio
$audio
''';

  Map<String, Object?> toJson() => {
    'id': id,
    'name': name,
    'summary': summary,
    'bestFor': bestFor,
    'voice': voice,
    'script': script,
    'storytelling': storytelling,
    'graphics': graphics,
    'captions': captions,
    'editing': editing,
    'audio': audio,
  };

  static CreativeDirection fromJson(Map<String, Object?> json) =>
      CreativeDirection(
        id: json['id'] as String,
        name: json['name'] as String,
        summary: json['summary'] as String? ?? '',
        bestFor: [
          for (final item in json['bestFor'] as List<Object?>? ?? const [])
            item as String,
        ],
        voice: json['voice'] as String? ?? '',
        script: json['script'] as String? ?? '',
        storytelling: json['storytelling'] as String? ?? '',
        graphics: json['graphics'] as String? ?? '',
        captions: json['captions'] as String? ?? '',
        editing: json['editing'] as String? ?? '',
        audio: json['audio'] as String? ?? '',
        isMine: true,
      );
}

/// The bundled directions. Five ways of making a video, no fixed stories.
const List<CreativeDirection> bundledCreativeDirections = [
  CreativeDirection(
    id: 'cinematic-documentary',
    name: 'Cinematic documentary',
    summary:
        'Patient, observational and quiet. The film trusts the subject to be '
        'interesting without help from the edit.',
    bestFor: [
      'Portraits of a person or a craft',
      'A place shot over several days',
      'Stories where the viewer should feel present',
    ],
    voice:
        'A single narrator, low and unhurried, speaking in complete sentences. '
        'Pauses are kept. No exclamation, no upspeak, roughly two sentences per '
        'ten seconds. Sometimes no narration at all, only the subject talking.',
    script:
        'Open on a detail with no explanation. Establish who and where in the '
        'first thirty seconds. Develop through one day or one process. Turn on '
        'the moment the subject reveals what this costs them. End on the image '
        'from the opening, changed by what was learned.',
    storytelling:
        'Observe rather than explain. Let the camera stay on a face a beat too '
        'long. Prefer a whole action shown once to three fragments. Cut on '
        'movement or on a sound, never on a beat.',
    graphics:
        'Serif or humanist sans, small, low contrast, placed with generous '
        'space. One accent colour taken from the footage. Titles fade up very '
        'slowly and never move once placed. No lower thirds except the name, '
        'once.',
    captions:
        'Only for spoken words the viewer would otherwise miss. Two lines at '
        'most, centred low, no background box, no per-word emphasis, no '
        'bouncing animation.',
    editing:
        'Six to ten second shots. Hard cuts, one dissolve on a passage of time. '
        'B-roll illustrates what is being said, never decorates it. No zoom '
        'punch-ins, no transitions for their own sake.',
    audio:
        'Room tone under everything. A single low sustained bed, twelve decibels '
        'below the voice, ducked further under speech. Real sounds kept: steps, '
        'tools, weather.',
  ),
  CreativeDirection(
    id: 'modern-tech-explainer',
    name: 'Modern tech explainer',
    summary:
        'Clear, confident and quick to the point. Every second earns the next '
        'one, and nothing is on screen that is not being explained.',
    bestFor: [
      'Explaining a product or a process',
      'A developer tool walkthrough',
      'A concept video for a launch page',
    ],
    voice:
        'One narrator, warm but efficient, medium pace, short sentences. '
        'Second person: "you", "your". Confident, never salesy. A small pause '
        'before each important noun.',
    script:
        'Hook in the first sentence with the problem in the viewer\'s words. '
        'Promise what they will be able to do by the end. Three movements, one '
        'idea each, in the order they would try it. Close by repeating the '
        'result, not the feature list.',
    storytelling:
        'Show the screen the viewer will see. Move from whole picture to the one '
        'control that matters, then back out. Every claim is followed by the '
        'thing that proves it, on screen.',
    graphics:
        'Geometric sans, tight leading, one accent over near-black or near-white. '
        'Rectangular panels with eight-point spacing. Motion is a fast fade and a '
        'four-pixel rise, one hundred and sixty milliseconds, eased out. '
        'Diagrams build one element at a time.',
    captions:
        'Burned-in or track, short lines of three to five words, keyword in the '
        'accent colour. Positioned clear of the interface being shown. No '
        'animation beyond a fade.',
    editing:
        'Two to three second shots, hard cuts on the sentence. Screen recordings '
        'sped up between actions, normal speed on the action. Callouts keyed to '
        'the narration. No transitions except a wipe between two chapters.',
    audio:
        'A clean percussive bed at low level, one accent on each cut, ducked '
        'twelve decibels under speech. Interface sounds only when the action '
        'matters.',
  ),
  CreativeDirection(
    id: 'energetic-social',
    name: 'Energetic social video',
    summary:
        'Fast, direct and made for a thumb. The first frame is already the '
        'story, and the cut never lets attention drift.',
    bestFor: [
      'A vertical short for a feed',
      'An announcement that has to land in ten seconds',
      'A montage of a day or an event',
    ],
    voice:
        'Either a fast, bright narrator who sounds like a friend, or no voice at '
        'all and captions carrying the words. First word inside the first half '
        'second. Short clauses, present tense.',
    script:
        'Hook as text on the first frame: the claim, the number or the question. '
        'Deliver one idea per three seconds. Turn at the midpoint with a '
        'surprise. End with a single call to action that fits in four words.',
    storytelling:
        'Start in the middle of the action. Vertical framing, subject centred, '
        'faces large. Movement in every shot: a pan, a hand, a walk. Cut before '
        'the viewer is ready.',
    graphics:
        'Heavy geometric sans, high contrast, one or two saturated colours on a '
        'plain field. Words appear whole, not typed on. A single sticker or '
        'outline may accent one word per shot. Nothing else moves.',
    captions:
        'Always on, large, centred in the upper third, two to three words per '
        'line, key word in the accent colour, easing in over eighty '
        'milliseconds on the beat.',
    editing:
        'One to two second shots. Hard cuts on the beat, jump cuts inside a '
        'sentence, one whip or zoom per section at most. B-roll is the texture '
        'between the spoken parts, not the subject.',
    audio:
        'Music is the spine: clear beat, ten decibels under any voice, cut points '
        'on the beat. One whoosh per transition, no more. Voice compressed and '
        'forward.',
  ),
  CreativeDirection(
    id: 'minimalist-storytelling',
    name: 'Minimalist storytelling',
    summary:
        'Almost nothing on screen, and everything that is there is deliberate. '
        'Space and silence do the work.',
    bestFor: [
      'A quiet product film',
      'A reflection or an essay',
      'A brand piece with no narration',
    ],
    voice:
        'Few words. Either a calm narrator reading short lines with long pauses, '
        'or typed text alone. Never two sentences where one will do. No '
        'filler, no greeting, no summary at the end.',
    script:
        'One sentence that states the thing. One image that shows it. One '
        'example. One sentence that turns it. Stop. If a line can be cut and the '
        'video still makes sense, it is cut.',
    storytelling:
        'Still frames held longer than comfortable. Symmetry, negative space, a '
        'single subject. Change one thing per shot. The camera does not move '
        'unless the movement is the point.',
    graphics:
        'One typeface, one weight, small on a large field. Monochrome with a '
        'single tint. Type placed on a grid, never centred by accident. Motion '
        'is a fade of two hundred milliseconds and nothing else.',
    captions:
        'Off unless words are spoken. When on: one line, lowercase, small, '
        'bottom left, no emphasis, no animation.',
    editing:
        'Four to eight second shots. Cuts only when the thought changes. No '
        'transitions. No effects. Black frames used as punctuation between '
        'sections.',
    audio:
        'Silence is allowed. A single sustained tone or a field recording, no '
        'drums. Fade in over two seconds, out over three.',
  ),
  CreativeDirection(
    id: 'warm-personal-narrative',
    name: 'Warm personal narrative',
    summary:
        'A person telling you something true. Close, imperfect and specific, '
        'with the small details left in.',
    bestFor: [
      'A personal essay or a vlog',
      'A founder or team story',
      'A letter to an audience',
    ],
    voice:
        'First person, spoken as if to one friend. Warm, slightly uneven, with '
        'real breaths. Medium pace, sentences that trail and restart. No '
        'broadcast tone, no reading voice.',
    script:
        'Start with the small concrete thing: the object, the room, the day. '
        'Widen to why it mattered. Admit the part that did not go well. Close on '
        'what changed, in one sentence, without a moral.',
    storytelling:
        'Camera close, at eye height, handheld or locked but never fussy. Keep '
        'the look to the side, the laugh, the pause. B-roll from the same day, '
        'not stock.',
    graphics:
        'Rounded humanist sans or a light serif, warm off-white background, '
        'muted earth accent. Titles look handwritten in placement, not in font. '
        'Nothing snaps into place.',
    captions:
        'On by default, two lines, sentence case, soft shadow instead of a box, '
        'key word underlined once per section.',
    editing:
        'Three to five second shots. Cuts follow the sentence and sometimes '
        'leave the pause in. One slow push per scene at most. No effects except '
        'a gentle fade at the ends.',
    audio:
        'Voice unprocessed enough to sound like a room. Quiet acoustic bed, '
        'fifteen decibels under, ducked hard under speech. Keep the real sounds '
        'of the day.',
  ),
];
