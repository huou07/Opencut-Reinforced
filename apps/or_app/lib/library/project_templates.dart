import '../project/project_gateway.dart';

/// One text clip a template places on the timeline.
///
/// Times are milliseconds from the sequence start. The shell turns them into
/// exact project times, so a template cannot invent a second time base.
class TemplateOverlay {
  const TemplateOverlay({
    required this.label,
    required this.presetId,
    required this.text,
    required this.startMilliseconds,
    required this.durationMilliseconds,
  });

  final String label;
  final String presetId;
  final String text;
  final int startMilliseconds;
  final int durationMilliseconds;

  ProjectRationalTime get start =>
      ProjectRationalTime(BigInt.from(startMilliseconds), 1000);

  ProjectRationalTime get duration =>
      ProjectRationalTime(BigInt.from(durationMilliseconds), 1000);
}

/// A starting point for a new project.
///
/// A template sets the two sequence fields the export depends on — frame rate
/// and canvas — and places a few ordinary text clips. It never carries media, so
/// creating one costs a project file and nothing else, and every clip it creates
/// is editable through the same commands the editor uses.
class ProjectTemplate {
  const ProjectTemplate({
    required this.id,
    required this.name,
    required this.summary,
    required this.suggestedName,
    required this.canvas,
    required this.frameRate,
    required this.overlays,
    required this.nextSteps,
  });

  final String id;
  final String name;
  final String summary;
  final String suggestedName;
  final ProjectSequenceCanvas canvas;
  final ProjectRationalRate frameRate;
  final List<TemplateOverlay> overlays;

  /// What to do next, in order, for someone who has never used the editor.
  final List<String> nextSteps;

  String get canvasLabel => canvas.label;

  /// Whole-number rates read as "30 fps"; anything else keeps both parts, so a
  /// rate is never rounded into a different one.
  String get frameRateLabel {
    final frames = frameRate.numerator % frameRate.denominator == 0
        ? '${frameRate.numerator ~/ frameRate.denominator}'
        : '${frameRate.numerator}/${frameRate.denominator}';
    return '$frames fps';
  }
}

/// The template catalogue.
///
/// These are the shapes the editor actually exports: 16:9 for landscape
/// viewing, 9:16 for a phone, 1:1 for a feed square. Each one names its frame
/// rate, because a project without a rate cannot play or step frames.
const List<ProjectTemplate> projectTemplates = [
  ProjectTemplate(
    id: 'landscape-1080p',
    name: 'Landscape 1080p',
    summary: '16:9 at 30 fps. The default for a video you will watch on a '
        'laptop or television.',
    suggestedName: 'Landscape project',
    canvas: ProjectSequenceCanvas(1920, 1080),
    frameRate: ProjectRationalRate(30, 1),
    overlays: [
      TemplateOverlay(
        label: 'Title',
        presetId: 'title',
        text: 'Untitled',
        startMilliseconds: 0,
        durationMilliseconds: 3000,
      ),
      TemplateOverlay(
        label: 'Subtitle',
        presetId: 'subtitle',
        text: 'Add your second line here',
        startMilliseconds: 500,
        durationMilliseconds: 2500,
      ),
    ],
    nextSteps: [
      'Import a clip or image from the Media tab.',
      'Drag it onto the timeline, then trim the ends.',
      'Press Space to play, then export when the cut is right.',
    ],
  ),
  ProjectTemplate(
    id: 'vertical-short',
    name: 'Vertical short',
    summary: '9:16 at 30 fps for a phone screen. Titles and captions are '
        'already placed where they stay readable.',
    suggestedName: 'Vertical short',
    canvas: ProjectSequenceCanvas(1080, 1920),
    frameRate: ProjectRationalRate(30, 1),
    overlays: [
      TemplateOverlay(
        label: 'Hook title',
        presetId: 'title',
        text: 'Start with the reason to watch',
        startMilliseconds: 0,
        durationMilliseconds: 2500,
      ),
      TemplateOverlay(
        label: 'Caption',
        presetId: 'caption',
        text: 'One line of speech, one line of text',
        startMilliseconds: 0,
        durationMilliseconds: 6000,
      ),
    ],
    nextSteps: [
      'Keep the hook under two seconds.',
      'Import the take from the Media tab and cut on the beat.',
      'Captions are text clips, so you can retype them instead of re-importing.',
    ],
  ),
  ProjectTemplate(
    id: 'square-social',
    name: 'Square',
    summary: '1:1 at 30 fps for a feed post. One centred composition, no '
        'safe-area guesswork.',
    suggestedName: 'Square post',
    canvas: ProjectSequenceCanvas(1080, 1080),
    frameRate: ProjectRationalRate(30, 1),
    overlays: [
      TemplateOverlay(
        label: 'Headline',
        presetId: 'title',
        text: 'One idea, one square',
        startMilliseconds: 0,
        durationMilliseconds: 3000,
      ),
    ],
    nextSteps: [
      'Choose the strongest single frame as the first image.',
      'Keep text inside the middle 80 % of the square.',
      'Export as MP4 for the widest playback support.',
    ],
  ),
  ProjectTemplate(
    id: 'tutorial',
    name: 'Tutorial',
    summary: '16:9 with a chapter label and a lower third, ready for a screen '
        'recording.',
    suggestedName: 'Tutorial',
    canvas: ProjectSequenceCanvas(1920, 1080),
    frameRate: ProjectRationalRate(30, 1),
    overlays: [
      TemplateOverlay(
        label: 'Chapter label',
        presetId: 'chapter',
        text: '01 · Introduction',
        startMilliseconds: 0,
        durationMilliseconds: 5000,
      ),
      TemplateOverlay(
        label: 'Presenter',
        presetId: 'lower-third',
        text: 'Your name · Your role',
        startMilliseconds: 1000,
        durationMilliseconds: 4000,
      ),
    ],
    nextSteps: [
      'Record the screen first, then edit the take.',
      'Duplicate the chapter label and retype it for each section.',
      'Record a voiceover from Audio → Record voice… when the cut is locked.',
    ],
  ),
  ProjectTemplate(
    id: 'interview',
    name: 'Interview',
    summary: '16:9 with a lower third and a caption line, for a seated '
        'conversation.',
    suggestedName: 'Interview',
    canvas: ProjectSequenceCanvas(1920, 1080),
    frameRate: ProjectRationalRate(30, 1),
    overlays: [
      TemplateOverlay(
        label: 'Guest name',
        presetId: 'lower-third',
        text: 'Guest name · Role',
        startMilliseconds: 1500,
        durationMilliseconds: 4500,
      ),
      TemplateOverlay(
        label: 'Caption',
        presetId: 'caption',
        text: 'The line the viewer must not miss',
        startMilliseconds: 0,
        durationMilliseconds: 5000,
      ),
    ],
    nextSteps: [
      'Import both camera files and align the audio waveforms.',
      'Cut on the change of speaker, not on every breath.',
      'Use Audio → Transcript to caption the whole conversation at once.',
    ],
  ),
  ProjectTemplate(
    id: 'title-card',
    name: 'Title card',
    summary: 'Three seconds of nothing but a title and a subtitle. Import it '
        'into a longer edit as an opening.',
    suggestedName: 'Title card',
    canvas: ProjectSequenceCanvas(1920, 1080),
    frameRate: ProjectRationalRate(30, 1),
    overlays: [
      TemplateOverlay(
        label: 'Title',
        presetId: 'title',
        text: 'Title',
        startMilliseconds: 0,
        durationMilliseconds: 3000,
      ),
      TemplateOverlay(
        label: 'Subtitle',
        presetId: 'subtitle',
        text: 'A line under the title',
        startMilliseconds: 0,
        durationMilliseconds: 3000,
      ),
    ],
    nextSteps: [
      'Set the canvas to match the sequence it will open.',
      'Export as MP4 and import the file into the longer project.',
    ],
  ),
  ProjectTemplate(
    id: 'end-card',
    name: 'End card',
    summary: 'Two seconds of closing text with a follow-up line.',
    suggestedName: 'End card',
    canvas: ProjectSequenceCanvas(1920, 1080),
    frameRate: ProjectRationalRate(30, 1),
    overlays: [
      TemplateOverlay(
        label: 'Closing line',
        presetId: 'end-card',
        text: 'Thanks for watching',
        startMilliseconds: 0,
        durationMilliseconds: 2000,
      ),
      TemplateOverlay(
        label: 'Follow-up',
        presetId: 'subtitle',
        text: 'Subscribe for the next one',
        startMilliseconds: 0,
        durationMilliseconds: 2000,
      ),
    ],
    nextSteps: [
      'Place it at the end of the timeline, after the last clip.',
      'Keep it shorter than the audience expects.',
    ],
  ),
  ProjectTemplate(
    id: 'podcast-clip',
    name: 'Podcast clip',
    summary: 'Vertical 9:16 with a caption block, for cutting one answer out of '
        'a longer conversation.',
    suggestedName: 'Podcast clip',
    canvas: ProjectSequenceCanvas(1080, 1920),
    frameRate: ProjectRationalRate(30, 1),
    overlays: [
      TemplateOverlay(
        label: 'Caption',
        presetId: 'caption',
        text: 'The strongest sentence of the answer',
        startMilliseconds: 0,
        durationMilliseconds: 8000,
      ),
      TemplateOverlay(
        label: 'Speaker',
        presetId: 'lower-third',
        text: 'Name · Podcast',
        startMilliseconds: 500,
        durationMilliseconds: 4000,
      ),
    ],
    nextSteps: [
      'Import the audio, then trim to the answer you want to keep.',
      'Type the caption rather than importing subtitles for one short clip.',
      'Export as MP4 so the clip plays everywhere it is posted.',
    ],
  ),
];

/// The template with [id], or null when the catalogue no longer has it.
ProjectTemplate? projectTemplateById(String id) {
  for (final template in projectTemplates) {
    if (template.id == id) return template;
  }
  return null;
}
