/// One instruction in a [GuideSection].
///
/// [detail] is the sentence a first-time user needs; nothing here repeats what
/// the control itself says, and no step claims a behaviour the app does not have.
class GuideStep {
  const GuideStep(this.title, this.detail);

  final String title;
  final String detail;
}

/// One keyboard shortcut, with the modifier written for the current host by the
/// screen that renders it.
class GuideShortcut {
  const GuideShortcut(this.keys, this.action, {this.command = false});

  final String keys;
  final String action;

  /// Whether [keys] use the platform command modifier (Cmd on macOS, Ctrl on
  /// Windows and Linux). The guide renders "Cmd" or "Ctrl" accordingly.
  final bool command;
}

class GuideSection {
  const GuideSection({
    required this.id,
    required this.title,
    required this.summary,
    required this.steps,
    this.shortcuts = const [],
  });

  final String id;
  final String title;
  final String summary;
  final List<GuideStep> steps;
  final List<GuideShortcut> shortcuts;
}

/// The in-app guide.
///
/// It is the short path for someone opening the app for the first time: create a
/// project, import, cut, add text, add sound, export. Depth stays in `docs/`; the
/// guide never copies a specification, it names the next action.
const List<GuideSection> usageGuide = [
  GuideSection(
    id: 'start',
    title: 'Your first project',
    summary: 'Five steps from an empty window to a finished file.',
    steps: [
      GuideStep(
        'Create a project',
        'Home → New Project. Pick a name, a frame rate and where the '
            '.orproj file lives. The frame rate matters: without one the '
            'preview cannot play or step frames.',
      ),
      GuideStep(
        'Or start from a template',
        'Library → Templates sets the canvas and frame rate for you and '
            'places the first text clips. You can delete or retype every one.',
      ),
      GuideStep(
        'Import media',
        'In the editor, Media → Import media (or press the import shortcut). '
            'Video, images, audio and text files all land in the media list.',
      ),
      GuideStep(
        'Place it on the timeline',
        'Drag a media item onto a track, or use its add action. Drag an edge '
            'to trim it, and drag the clip body to move it.',
      ),
      GuideStep(
        'Export',
        'Export writes one file next to the location you choose. The project '
            'stays as it is, so you can export again at a different quality.',
      ),
    ],
  ),
  GuideSection(
    id: 'import',
    title: 'Importing media',
    summary: 'What the app reads, and what it does not move.',
    steps: [
      GuideStep(
        'Files stay where they are',
        'Import references the original file; nothing is copied into the '
            'project. Moving or renaming a source later needs a relink, and the '
            'app says so instead of showing a black frame.',
      ),
      GuideStep(
        'Folders',
        'Import folder adds every supported file inside one folder at once, '
            'bounded so a huge tree cannot stall the editor.',
      ),
      GuideStep(
        'What is supported',
        'Common containers and codecs through the packaged FFmpeg runtime: '
            'MP4/MOV/WebM/MKV video, MP3/WAV/FLAC/AAC audio, PNG/JPEG/WebP '
            'images, and text/caption files.',
      ),
      GuideStep(
        'If import fails',
        'Settings → Resources & updates states whether the FFmpeg runtime is '
            'present. A build without it can edit but cannot decode or export.',
      ),
    ],
  ),
  GuideSection(
    id: 'edit',
    title: 'Cutting on the timeline',
    summary: 'The verbs you need for a first cut, and where they live.',
    steps: [
      GuideStep(
        'Playhead and selection',
        'Arrow keys move the playhead by one second; Shift+Arrow steps one '
            'frame. Click a clip to select it, Escape to clear the selection.',
      ),
      GuideStep(
        'Trim, split, delete',
        'Drag a clip edge to trim. Split cuts at the playhead. Delete removes '
            'the clip and leaves a gap; ripple delete closes the gap.',
      ),
      GuideStep(
        'Blade mode',
        'C arms the blade so one click splits at the pointer; V returns to '
            'the normal pointer.',
      ),
      GuideStep(
        'Snapping and zoom',
        'Snapping aligns clip edges, the playhead and markers. Zoom with the '
            'timeline controls or the bracket keys, and Fit shows the whole '
            'sequence.',
      ),
      GuideStep(
        'Undo',
        'Every edit is a command with history. Undo and redo are in the top '
            'bar and in the command palette.',
      ),
    ],
  ),
  GuideSection(
    id: 'text',
    title: 'Text and captions',
    summary: 'Titles are clips, not overlays burned at export time.',
    steps: [
      GuideStep(
        'Add a text clip',
        'Text → add a title or caption. It becomes a clip on a text track, so '
            'it can be moved, trimmed and retyped like any other clip.',
      ),
      GuideStep(
        'Reuse a look',
        'Library → Text lists ready looks with their exact font, weight and '
            'size. Adding one creates an ordinary text clip you can still edit.',
      ),
      GuideStep(
        'Captions',
        'Import an SRT or WebVTT file to get caption clips on a caption '
            'track. Captions use the same text path as titles.',
      ),
      GuideStep(
        'Fonts',
        'Two bundled fonts cover Latin and Vietnamese: Inter and Be Vietnam '
            'Pro. Both ship with the app, so a project looks the same on every '
            'machine.',
      ),
    ],
  ),
  GuideSection(
    id: 'audio',
    title: 'Voice, music and sound',
    summary: 'Audio tools that run on the machine, not on a server.',
    steps: [
      GuideStep(
        'Record a voice track',
        'Audio → Record voice… writes a take through the normal import path. '
            'The app asks for microphone access the first time and says so if '
            'it is denied.',
      ),
      GuideStep(
        'Generate a voiceover',
        'Audio → Generate voiceover… speaks a script with an installed local '
            'voice. Settings → Narration chooses the voice and rate.',
      ),
      GuideStep(
        'Generate music',
        'Audio → Generate music… builds a bed from a one-sentence '
            'description, then imports it. Ducking can lower it under speech.',
      ),
      GuideStep(
        'Sound effects',
        'Library → Sound writes a short generated effect so it can be added '
            'to the media list. Nothing is downloaded.',
      ),
    ],
  ),
  GuideSection(
    id: 'export',
    title: 'Export',
    summary: 'One dialog, three profiles, one output file.',
    steps: [
      GuideStep(
        'Choose a profile',
        'MP4 (H.264 + AAC) plays almost everywhere. WebM (VP9 + Opus) is '
            'smaller for sharing. Matroska (FFV1 + PCM) is lossless and much '
            'larger.',
      ),
      GuideStep(
        'Quality and size',
        'Quality trades file size against detail and applies to the two lossy '
            'profiles. Resolution sets the output frame; the sequence stays as '
            'it was.',
      ),
      GuideStep(
        'Speed',
        'The exporter renders and encodes at the same time and uses the '
            'machine\'s hardware encoder when one is available, falling back to '
            'the software encoder otherwise. The choice is automatic and the '
            'output is the same.',
      ),
      GuideStep(
        'Cancel and failure',
        'A cancelled export deletes the partial file. A failed export says '
            'why and leaves the project untouched.',
      ),
    ],
  ),
  GuideSection(
    id: 'shortcuts',
    title: 'Keyboard',
    summary: 'Every verb is also a menu entry and a palette command.',
    steps: [
      GuideStep(
        'Command palette',
        'One search field for every action in the app. If you do not know '
            'where a control lives, start here.',
      ),
    ],
    shortcuts: [
      GuideShortcut('K', 'Command palette', command: true),
      GuideShortcut('S', 'Save project', command: true),
      GuideShortcut('E', 'Export', command: true),
      GuideShortcut('Z', 'Undo', command: true),
      GuideShortcut('Shift+Z', 'Redo', command: true),
      GuideShortcut('I', 'Import media', command: true),
      GuideShortcut('Shift+I', 'Import folder', command: true),
      GuideShortcut('Shift+A', 'Assistant panel', command: true),
      GuideShortcut('D', 'Duplicate clip', command: true),
      GuideShortcut('B', 'Split at playhead', command: true),
      GuideShortcut('Space', 'Play or pause'),
      GuideShortcut('Left / Right', 'Seek one second'),
      GuideShortcut('Shift+Left / Right', 'Step one frame'),
      GuideShortcut('Comma / Period', 'Step one frame'),
      GuideShortcut('[ / ]', 'Seek five seconds'),
      GuideShortcut('= / -', 'Zoom timeline'),
      GuideShortcut('F5', 'Fit timeline'),
      GuideShortcut('S', 'Split at playhead'),
      GuideShortcut('C / V', 'Blade mode on / off'),
      GuideShortcut('M', 'Add marker'),
      GuideShortcut('Delete', 'Delete selected clip'),
      GuideShortcut('Shift+Delete', 'Ripple delete'),
      GuideShortcut('1 2 3 4', 'Media / Text / Audio / Motion panel'),
      GuideShortcut('Alt+Arrow', 'Nudge the selected clip in the frame'),
    ],
  ),
  GuideSection(
    id: 'files',
    title: 'Projects and files',
    summary: 'What the app writes, and where.',
    steps: [
      GuideStep(
        'One project file',
        'A project is a single .orproj file that holds the edit, not the '
            'media. Copying it to another machine needs the source files too.',
      ),
      GuideStep(
        'Saving',
        'The top bar shows whether the project has unsaved changes. Save '
            'writes the whole project atomically, so an interrupted write '
            'cannot corrupt the previous version.',
      ),
      GuideStep(
        'Recovery',
        'If the app closed with unsaved work, opening the project offers the '
            'recovery copy before it is replaced.',
      ),
      GuideStep(
        'Recent projects',
        'Home lists recent projects. Removing one from that list deletes the '
            'entry, never the file.',
      ),
    ],
  ),
  GuideSection(
    id: 'trouble',
    title: 'When something is missing',
    summary: 'The app states the cause instead of failing silently.',
    steps: [
      GuideStep(
        'Export or decode fails',
        'Settings → Resources & updates reports the packaged FFmpeg runtime, '
            'the speech runtime and the or CLI. A missing runtime disables the '
            'affected actions with the reason shown.',
      ),
      GuideStep(
        'Microphone is denied',
        'The record dialog says access was denied and links to the system '
            'setting. It never records silently.',
      ),
      GuideStep(
        'A clip shows a missing source',
        'Use relink on that media item to point the project at the moved '
            'file. Everything else in the project keeps working.',
      ),
    ],
  ),
];

/// The section with [id], or null when the guide no longer has it.
GuideSection? guideSectionById(String id) {
  for (final section in usageGuide) {
    if (section.id == id) return section;
  }
  return null;
}
