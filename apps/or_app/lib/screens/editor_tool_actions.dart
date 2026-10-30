import 'package:flutter/material.dart';

/// One action inside a tool panel: an honest button that performs a real
/// command, with the widget key the UI harness and tests address it by.
class EditorToolAction {
  const EditorToolAction({
    required this.keyName,
    required this.label,
    required this.icon,
    required this.onPressed,
  });

  final String keyName;
  final String label;
  final IconData icon;
  final VoidCallback? onPressed;
}

/// Copy for the Text tool panel.
const String textToolSubtitle =
    'Titles and captions live on their own tracks. Select a clip to edit its '
    'content and formatting in the inspector.';

/// Copy for the Audio tool panel.
const String audioToolSubtitle =
    'Audio clips carry gain, pan and fade times. The inspector shows them in '
    'dB, % and seconds for the selected clip.';

/// Actions the Text tool offers. Kept beside the copy so a tool's panel can be
/// reviewed, and later moved, without touching the editor shell.
List<EditorToolAction> textToolActions({
  VoidCallback? onAddTextTrack,
  VoidCallback? onAddCaptionTrack,
  VoidCallback? onImportCaptions,
  VoidCallback? onExportCaptions,
}) => [
  EditorToolAction(
    keyName: 'tool-add-text-track',
    label: 'Add text track',
    icon: Icons.text_fields_outlined,
    onPressed: onAddTextTrack,
  ),
  EditorToolAction(
    keyName: 'tool-add-caption-track',
    label: 'Add caption track',
    icon: Icons.subtitles_outlined,
    onPressed: onAddCaptionTrack,
  ),
  EditorToolAction(
    keyName: 'tool-import-captions',
    label: 'Import captions…',
    icon: Icons.file_open_outlined,
    onPressed: onImportCaptions,
  ),
  EditorToolAction(
    keyName: 'tool-export-captions',
    label: 'Export captions…',
    icon: Icons.file_download_outlined,
    onPressed: onExportCaptions,
  ),
];

/// Actions the Audio tool offers: tracks, import, recording, generated
/// narration, generated music and the local speech model.
List<EditorToolAction> audioToolActions({
  VoidCallback? onAddAudioTrack,
  VoidCallback? onImportMedia,
  VoidCallback? onRecordVoice,
  VoidCallback? onGenerateVoiceover,
  VoidCallback? onGenerateMusic,
  VoidCallback? onManageSpeechModel,
}) => [
  EditorToolAction(
    keyName: 'tool-add-audio-track',
    label: 'Add audio track',
    icon: Icons.graphic_eq_outlined,
    onPressed: onAddAudioTrack,
  ),
  EditorToolAction(
    keyName: 'tool-import-audio',
    label: 'Import audio or video…',
    icon: Icons.add,
    onPressed: onImportMedia,
  ),
  EditorToolAction(
    keyName: 'tool-record-voice',
    label: 'Record voice…',
    icon: Icons.mic_none_outlined,
    onPressed: onRecordVoice,
  ),
  EditorToolAction(
    keyName: 'tool-generate-voiceover',
    label: 'Generate voiceover…',
    icon: Icons.record_voice_over_outlined,
    onPressed: onGenerateVoiceover,
  ),
  EditorToolAction(
    keyName: 'tool-generate-music',
    label: 'Generate music…',
    icon: Icons.music_note_outlined,
    onPressed: onGenerateMusic,
  ),
  EditorToolAction(
    keyName: 'tool-speech-model',
    label: 'Speech model…',
    icon: Icons.mic_none_outlined,
    onPressed: onManageSpeechModel,
  ),
];
