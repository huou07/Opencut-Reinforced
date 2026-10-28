import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import 'voice_recorder.dart';

class VoiceRecording {
  const VoiceRecording({required this.path, required this.durationMs});

  final String path;
  final int durationMs;
}

/// Records one microphone take, previews it, and returns the WAV to import.
class RecordVoiceDialog extends StatefulWidget {
  const RecordVoiceDialog({
    super.key,
    this.recorder,
    this.supportDirectory,
    this.temporaryDirectory,
  });

  final VoiceRecorder? recorder;
  final Future<Directory> Function()? supportDirectory;
  final Future<Directory> Function()? temporaryDirectory;

  @override
  State<RecordVoiceDialog> createState() => _RecordVoiceDialogState();
}

class _RecordVoiceDialogState extends State<RecordVoiceDialog>
    with WidgetsBindingObserver {
  late final VoiceRecorder _recorder = widget.recorder ?? const VoiceRecorder();
  Timer? _ticker;
  Stopwatch? _watch;
  String? _permission;
  File? _file;
  int? _durationMs;
  bool _recording = false;
  bool _busy = false;
  String? _error;
  String? _notice;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _refreshPermission();
  }

  @override
  void dispose() {
    _ticker?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  /// Reads the state without prompting; the prompt belongs to a take.
  Future<void> _refreshPermission() async {
    try {
      final permission = await _recorder.permission();
      if (mounted) setState(() => _permission = permission);
    } on VoiceRecorderException catch (error) {
      if (mounted) setState(() => _error = error.message);
    }
  }

  /// The user may grant access in System Settings and come straight back, so the
  /// dialog re-reads the state instead of keeping a stale denial.
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) _refreshPermission();
  }

  Future<void> _start() async {
    setState(() {
      _busy = true;
      _error = null;
      _notice = null;
    });
    try {
      if (_permission != 'granted') {
        // First take asks macOS itself: the request is what creates the TCC
        // entry, so the app appears under Privacy & Security → Microphone.
        final permission = await _recorder.requestPermission();
        if (mounted) setState(() => _permission = permission);
        if (permission != 'granted') {
          if (mounted) {
            setState(() {
              _error = permission == 'undetermined'
                  ? 'macOS did not answer the microphone request. Try again.'
                  : 'Microphone access is not granted. Allow Opencut Reinforced '
                        'in System Settings → Privacy & Security → Microphone, '
                        'then start again.';
            });
          }
          return;
        }
      }
      final base = await (widget.temporaryDirectory ?? getTemporaryDirectory)();
      final directory = Directory(
        '${base.path}${Platform.pathSeparator}voice-recordings',
      );
      final file = File(
        '${directory.path}${Platform.pathSeparator}${DateTime.now().millisecondsSinceEpoch}.wav',
      );
      await _recorder.start(file);
      _watch = Stopwatch()..start();
      _ticker?.cancel();
      _ticker = Timer.periodic(const Duration(milliseconds: 200), (_) {
        if (mounted) setState(() {});
      });
      if (mounted) {
        setState(() {
          _file = file;
          _durationMs = null;
          _recording = true;
        });
      }
    } on VoiceRecorderException catch (error) {
      if (mounted) {
        setState(() {
          _error = error.message;
          // The native side is the authority: a revoked grant is a denial even
          // if the dialog believed access was granted.
          if (error.code == 'PERMISSION_DENIED') _permission = 'denied';
        });
      }
    } catch (_) {
      if (mounted) setState(() => _error = 'Recording could not start.');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _stop() async {
    setState(() {
      _busy = true;
      _error = null;
      _notice = null;
    });
    _ticker?.cancel();
    _watch?.stop();
    try {
      final measured = await _recorder.stop();
      if (mounted) {
        setState(() {
          _recording = false;
          _durationMs = measured > 0
              ? measured
              : (_watch?.elapsedMilliseconds ?? 0);
        });
      }
    } on VoiceRecorderException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _play() async {
    final file = _file;
    if (file == null || _recording) return;
    try {
      await Process.run('afplay', [file.path]);
    } catch (_) {
      if (mounted) setState(() => _error = 'The preview could not play.');
    }
  }

  /// Opens the microphone pane so a denial can be reversed without hunting.
  Future<void> _openSettings() async {
    try {
      await _recorder.openSystemSettings();
    } on VoiceRecorderException catch (error) {
      if (mounted) setState(() => _error = error.message);
    }
  }

  Future<void> _saveSample() async {
    final file = _file;
    if (file == null) return;
    setState(() {
      _busy = true;
      _error = null;
      _notice = null;
    });
    try {
      final support =
          await (widget.supportDirectory ?? getApplicationSupportDirectory)();
      final directory = Directory(
        '${support.path}${Platform.pathSeparator}voice-samples',
      );
      await directory.create(recursive: true);
      final destination = File(
        '${directory.path}${Platform.pathSeparator}${DateTime.now().millisecondsSinceEpoch}.wav',
      );
      await file.copy(destination.path);
      if (mounted) {
        setState(
          () => _notice =
              'Saved as a voice sample. It can be used to create a personal '
              'cloud voice from Settings → Narration.',
        );
      }
    } catch (_) {
      if (mounted) setState(() => _error = 'The sample could not be saved.');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  String get _elapsed {
    final ms = _watch?.elapsedMilliseconds ?? _durationMs ?? 0;
    final seconds = ms ~/ 1000;
    return '${(seconds ~/ 60).toString().padLeft(2, '0')}:'
        '${(seconds % 60).toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    final file = _file;
    final canUse = file != null && !_recording && (_durationMs ?? 0) > 0;
    return AlertDialog(
      title: const Text('Record voice'),
      content: SizedBox(
        width: 520,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'Records one 48 kHz mono WAV from the microphone. The take is '
              'ordinary media: import it, or keep it as a sample for a personal voice.',
              style: TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataSize,
              ),
            ),
            const SizedBox(height: OrSpacing.x3),
            Text(
              _recording ? 'Recording  $_elapsed' : 'Duration  $_elapsed',
              key: const ValueKey('record-voice-time'),
              textAlign: TextAlign.center,
              style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: OrSpacing.x3),
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                FilledButton.icon(
                  key: const ValueKey('record-voice-toggle'),
                  onPressed: _busy
                      ? null
                      : _recording
                      ? _stop
                      : _start,
                  icon: Icon(_recording ? Icons.stop : Icons.mic, size: 18),
                  label: Text(_recording ? 'Stop' : 'Start recording'),
                ),
                if (canUse) ...[
                  const SizedBox(width: OrSpacing.x2),
                  OutlinedButton.icon(
                    key: const ValueKey('record-voice-play'),
                    onPressed: _busy ? null : _play,
                    icon: const Icon(Icons.play_arrow, size: 18),
                    label: const Text('Play'),
                  ),
                ],
              ],
            ),
            if (_permission != null && _permission != 'granted') ...[
              const SizedBox(height: OrSpacing.x2),
              Text(
                'Microphone permission: $_permission. Allow it in System '
                'Settings → Privacy & Security → Microphone.',
                key: const ValueKey('record-voice-permission'),
                style: TextStyle(
                  color: OrColors.warning,
                  fontSize: OrType.tinySize,
                ),
              ),
              if (_permission == 'denied' || _permission == 'restricted')
                TextButton.icon(
                  key: const ValueKey('record-voice-open-settings'),
                  onPressed: _busy ? null : _openSettings,
                  icon: const Icon(Icons.open_in_new, size: 16),
                  label: const Text('Open microphone settings'),
                ),
            ],
            if (_notice != null) ...[
              const SizedBox(height: OrSpacing.x2),
              Text(
                _notice!,
                key: const ValueKey('record-voice-notice'),
                style: TextStyle(
                  color: OrColors.success,
                  fontSize: OrType.tinySize,
                ),
              ),
            ],
            if (_error != null) ...[
              const SizedBox(height: OrSpacing.x2),
              Text(
                _error!,
                key: const ValueKey('record-voice-error'),
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        TextButton(
          key: const ValueKey('record-voice-save-sample'),
          onPressed: canUse && !_busy ? _saveSample : null,
          child: const Text('Save as sample'),
        ),
        FilledButton(
          key: const ValueKey('record-voice-use'),
          onPressed: canUse && !_busy
              ? () => Navigator.of(context).pop(
                  VoiceRecording(path: file.path, durationMs: _durationMs ?? 0),
                )
              : null,
          child: const Text('Add to timeline'),
        ),
      ],
    );
  }
}
