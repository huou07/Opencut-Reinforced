import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../project/project_gateway.dart';
import 'voiceover_service.dart';

/// Collects a script and runs offline narration into the open project.
///
/// The dialog owns the whole operation: synthesise every line, import and place
/// it, then import the matching caption cues. It pops with the applied result so
/// the editor can refresh the timeline and report what changed.
class VoiceoverDialog extends StatefulWidget {
  const VoiceoverDialog({
    super.key,
    required this.gateway,
    required this.session,
    required this.project,
  });

  final ProjectGateway gateway;
  final ProjectSessionHandle session;
  final ProjectReadModel project;

  @override
  State<VoiceoverDialog> createState() => _VoiceoverDialogState();
}

class _VoiceoverDialogState extends State<VoiceoverDialog> {
  final TextEditingController _script = TextEditingController();
  final VoiceoverService _voiceover = VoiceoverService();
  late final VoiceoverApplier _applier = VoiceoverApplier(widget.gateway);

  List<VoiceOption> _voices = const [];
  String? _voice;
  double _rate = 180;
  double _gapMs = 250;
  bool _busy = false;
  String _stage = '';
  double? _progress;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadVoices();
  }

  @override
  void dispose() {
    _script.dispose();
    super.dispose();
  }

  Future<void> _loadVoices() async {
    setState(() {
      _error = null;
    });
    try {
      final voices = await _voiceover.listVoices();
      if (!mounted) return;
      VoiceOption? preferred;
      for (final locale in ['vi_VN', 'en_US', 'en_GB']) {
        for (final voice in voices) {
          if (voice.locale == locale) {
            preferred = voice;
            break;
          }
        }
        if (preferred != null) break;
      }
      setState(() {
        _voices = voices;
        _voice = (preferred ?? (voices.isEmpty ? null : voices.first))?.name;
      });
    } on VoiceoverException catch (error) {
      if (mounted) setState(() => _error = error.message);
    }
  }

  Future<void> _generate() async {
    final lines = _script.text
        .split('\n')
        .map((line) => line.trim())
        .where((line) => line.isNotEmpty)
        .toList(growable: false);
    if (lines.isEmpty) {
      setState(() => _error = 'Write at least one narration line.');
      return;
    }
    final voice = _voice;
    if (voice == null) {
      setState(() => _error = 'Choose a voice first.');
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
      _stage = 'Preparing narration…';
      _progress = null;
    });
    try {
      final directory = await _voiceover.outputDirectory();
      final synthesized = await _voiceover.synthesize(
        lines: lines,
        voice: voice,
        rate: _rate.round(),
        gapMs: _gapMs.round(),
        directory: directory,
        onProgress: (completed, total) {
          if (!mounted) return;
          setState(() {
            _stage = 'Speaking line $completed of $total…';
            _progress = total == 0 ? null : completed / total;
          });
        },
      );
      if (!mounted) return;
      setState(() {
        _stage = 'Adding narration and captions to the timeline…';
        _progress = null;
      });
      final applied = await _applier.apply(
        session: widget.session,
        project: widget.project,
        lines: synthesized,
        onProgress: (stage, completed, total) {
          if (!mounted) return;
          setState(() {
            _stage = '$stage $completed of $total…';
            _progress = total == 0 ? null : completed / total;
          });
        },
      );
      if (!mounted) return;
      Navigator.of(context).pop(applied);
    } on VoiceoverException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } on ProjectGatewayException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } catch (error) {
      if (mounted) setState(() => _error = 'Voiceover could not be completed: $error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Generate voiceover'),
      content: SizedBox(
        width: 620,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text(
                'Each non-empty line becomes one narration clip and one caption '
                'at its exact duration. Narration runs offline with the macOS '
                'voice; nothing is uploaded.',
              ),
              const SizedBox(height: 12),
              TextField(
                key: const ValueKey('voiceover-script'),
                controller: _script,
                minLines: 4,
                maxLines: 8,
                enabled: !_busy,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  labelText: 'Script, one line per scene',
                  hintText:
                      'Xin chào, đây là Opencut Reinforced.\nBạn có thể dựng phim ngay trên máy.',
                ),
              ),
              const SizedBox(height: 12),
              if (_voices.isEmpty)
                Row(
                  children: [
                    const Expanded(
                      child: Text('No installed voice was found.'),
                    ),
                    TextButton(
                      onPressed: _busy ? null : _loadVoices,
                      child: const Text('Retry'),
                    ),
                  ],
                )
              else
                DropdownButtonFormField<String>(
                  key: ValueKey('voiceover-voice-${_voice ?? 'none'}'),
                  initialValue: _voice,
                  isExpanded: true,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    labelText: 'Voice',
                  ),
                  items: [
                    for (final voice in _voices)
                      DropdownMenuItem(
                        value: voice.name,
                        child: Text(voice.label, overflow: TextOverflow.ellipsis),
                      ),
                  ],
                  onChanged: _busy
                      ? null
                      : (value) => setState(() => _voice = value),
                ),
              const SizedBox(height: 8),
              Row(
                children: [
                  const SizedBox(width: 96, child: Text('Rate')),
                  Expanded(
                    child: Slider(
                      key: const ValueKey('voiceover-rate'),
                      value: _rate,
                      min: 120,
                      max: 240,
                      divisions: 24,
                      label: '${_rate.round()} wpm',
                      onChanged: _busy
                          ? null
                          : (value) => setState(() => _rate = value),
                    ),
                  ),
                  SizedBox(
                    width: 72,
                    child: Text(
                      '${_rate.round()} wpm',
                      textAlign: TextAlign.end,
                      style: TextStyle(color: OrColors.textSecondary),
                    ),
                  ),
                ],
              ),
              Row(
                children: [
                  const SizedBox(width: 96, child: Text('Gap')),
                  Expanded(
                    child: Slider(
                      key: const ValueKey('voiceover-gap'),
                      value: _gapMs,
                      min: 0,
                      max: 1000,
                      divisions: 20,
                      label: '${_gapMs.round()} ms',
                      onChanged: _busy
                          ? null
                          : (value) => setState(() => _gapMs = value),
                    ),
                  ),
                  SizedBox(
                    width: 72,
                    child: Text(
                      '${_gapMs.round()} ms',
                      textAlign: TextAlign.end,
                      style: TextStyle(color: OrColors.textSecondary),
                    ),
                  ),
                ],
              ),
              if (_busy) ...[
                const SizedBox(height: 8),
                LinearProgressIndicator(value: _progress),
                const SizedBox(height: 6),
                Text(
                  _stage,
                  key: const ValueKey('voiceover-stage'),
                  style: TextStyle(color: OrColors.textSecondary),
                ),
              ],
              if (_error != null) ...[
                const SizedBox(height: 8),
                Text(
                  _error!,
                  key: const ValueKey('voiceover-error'),
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const ValueKey('voiceover-generate'),
          onPressed: _busy ? null : _generate,
          child: const Text('Generate and add'),
        ),
      ],
    );
  }
}
