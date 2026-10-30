import 'package:flutter/material.dart';

import '../project/project_gateway.dart';
import 'music_dialog.dart';
import 'record_voice_dialog.dart';
import 'voiceover_dialog.dart';
import 'voiceover_service.dart';

/// Opens the three audio workflows the Audio tool owns and reports what landed.
///
/// Every workflow re-reads the live session instead of trusting the session it
/// started with, so a dialog result that arrives after the project changed is
/// discarded rather than applied to a stale project.
class AudioToolFlows {
  const AudioToolFlows({
    required this.gateway,
    required this.activeSession,
    required this.activeProject,
    required this.isBusy,
    required this.refresh,
    required this.notice,
    required this.unavailable,
  });

  final ProjectGateway gateway;
  final ProjectSessionHandle? Function() activeSession;
  final ProjectReadModel? Function() activeProject;
  final bool Function() isBusy;
  final Future<void> Function(ProjectSessionHandle session) refresh;
  final void Function(String message) notice;
  final void Function(String message) unavailable;

  /// Opens the voiceover workflow, then reports the clip and caption count.
  Future<void> generateVoiceover(BuildContext context) async {
    final session = activeSession();
    final project = activeProject();
    if (session == null || project == null || isBusy()) return;
    final applied = await showDialog<VoiceoverApplied>(
      context: context,
      barrierDismissible: false,
      builder: (context) =>
          VoiceoverDialog(gateway: gateway, session: session, project: project),
    );
    if (!context.mounted || !identical(session, activeSession())) return;
    await refresh(session);
    if (applied == null || !context.mounted) return;
    notice(
      'Added ${applied.lines.length} narration '
      '${applied.lines.length == 1 ? 'line' : 'lines'} and '
      '${applied.captionCount} '
      '${applied.captionCount == 1 ? 'caption' : 'captions'}.',
    );
  }

  /// Records one take and places it as an ordinary audio clip.
  Future<void> recordVoice(BuildContext context) async {
    final session = activeSession();
    final project = activeProject();
    if (session == null || project == null || isBusy()) return;
    final recording = await showDialog<VoiceRecording>(
      context: context,
      barrierDismissible: false,
      builder: (context) => const RecordVoiceDialog(),
    );
    if (!context.mounted ||
        !identical(session, activeSession()) ||
        recording == null) {
      return;
    }
    if (recording.durationMs <= 0) {
      unavailable('The recording was empty. Record a longer take.');
      return;
    }
    try {
      await VoiceoverApplier(gateway).apply(
        session: session,
        project: activeProject() ?? project,
        lines: [
          VoiceoverLine(
            index: 1,
            text: 'Recorded voice',
            caption: '',
            path: recording.path,
            startMs: 0,
            endMs: recording.durationMs,
          ),
        ],
        includeCaptions: false,
      );
      if (!context.mounted || !identical(session, activeSession())) return;
      await refresh(session);
      if (!context.mounted) return;
      final seconds = (recording.durationMs / 1000).toStringAsFixed(1);
      notice('Added a recorded voice clip (${seconds}s).');
    } on VoiceoverException catch (error) {
      unavailable(error.message);
    } on ProjectGatewayException catch (error) {
      unavailable(error.message);
    }
  }

  /// Opens the generated-music workflow and reports the bed and any ducking.
  Future<void> generateMusic(BuildContext context) async {
    final session = activeSession();
    final project = activeProject();
    if (session == null || project == null || isBusy()) return;
    final applied = await showDialog<MusicApplied>(
      context: context,
      barrierDismissible: false,
      builder: (context) =>
          MusicDialog(gateway: gateway, session: session, project: project),
    );
    if (!context.mounted || !identical(session, activeSession())) return;
    await refresh(session);
    if (applied == null || !context.mounted) return;
    final duckNote = applied.ducked
        ? ' Ducked -12 dB under the selected voice track; that used '
              '${applied.undoSteps} undo ${applied.undoSteps == 1 ? 'step' : 'steps'}.'
        : '';
    notice('Added a ${applied.bed.mood} music bed.$duckNote');
  }
}
