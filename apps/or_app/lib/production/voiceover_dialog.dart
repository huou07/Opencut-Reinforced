import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../project/project_gateway.dart';
import 'cloud_voice_service.dart';
import 'narration_settings.dart';
import 'narration_settings_dialog.dart';
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
  final CloudTtsService _cloud = CloudTtsService();
  final NarrationSettingsStore _settings = NarrationSettingsStore();
  late final VoiceoverApplier _applier = VoiceoverApplier(widget.gateway);

  List<VoiceOption> _voices = const [];
  List<CloudVoice> _cloudVoices = const [];
  NarrationSettings _saved = const NarrationSettings();
  String? _voice;
  String _provider = 'system';
  String _cloudVoiceId = '';
  String _azureRegion = '';
  bool _cloudLoading = false;
  String? _cloudError;
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
      final saved = await _settings.load();
      if (!mounted) return;
      final preferred =
          NarrationSettings.preferredVoiceName(voices, saved) ??
          (voices.isEmpty ? null : voices.first.name);
      setState(() {
        _voices = voices;
        _saved = saved;
        _voice = preferred;
        _provider = saved.provider;
        _cloudVoiceId = saved.cloudVoiceId;
        _azureRegion = saved.azureRegion;
        _rate = saved.rate.toDouble();
        _gapMs = saved.gapMs.toDouble();
      });
      if (_provider != 'system' && mounted) {
        await _loadCloudVoices();
      }
    } on VoiceoverException catch (error) {
      if (mounted) setState(() => _error = error.message);
    }
  }

  CloudVoiceProvider get _cloudProvider => _provider == 'elevenlabs'
      ? CloudVoiceProvider.elevenLabs
      : CloudVoiceProvider.azure;

  Future<void> _setProvider(String provider) async {
    if (provider == _provider) return;
    setState(() {
      _provider = provider;
      _cloudVoices = const [];
      _cloudError = null;
      _cloudVoiceId = provider == _saved.provider ? _saved.cloudVoiceId : '';
    });
    if (provider != 'system') {
      await _loadCloudVoices();
    }
  }

  Future<void> _loadCloudVoices() async {
    if (_provider == 'system') return;
    setState(() {
      _cloudLoading = true;
      _cloudError = null;
    });
    try {
      final voices = await _cloud.listVoices(
        _cloudProvider,
        azureRegion: _azureRegion,
      );
      if (!mounted) return;
      final preferred = voices.any((voice) => voice.id == _cloudVoiceId)
          ? _cloudVoiceId
          : (voices.isEmpty ? '' : voices.first.id);
      setState(() {
        _cloudVoices = voices;
        _cloudVoiceId = preferred;
        _cloudLoading = false;
      });
    } on CloudVoiceException catch (error) {
      if (mounted) {
        setState(() {
          _cloudLoading = false;
          _cloudError = error.message;
        });
      }
    } catch (_) {
      if (mounted) {
        setState(() {
          _cloudLoading = false;
          _cloudError = 'The cloud voice list could not be loaded.';
        });
      }
    }
  }

  Future<List<VoiceoverLine>> _synthesizeCloud(
    List<String> lines,
    Directory directory,
  ) async {
    await directory.create(recursive: true);
    final results = <VoiceoverLine>[];
    var cursor = 0;
    for (var index = 0; index < lines.length; index += 1) {
      final text = lines[index];
      final path = '${directory.path}/line-${index + 1}.wav';
      final duration = await _cloud.synthesize(
        provider: _cloudProvider,
        voiceId: _cloudVoiceId,
        text: text,
        outputPath: path,
        azureRegion: _azureRegion,
      );
      final start = cursor;
      final end = start + duration;
      results.add(
        VoiceoverLine(
          index: index + 1,
          text: text,
          caption: text,
          path: path,
          startMs: start,
          endMs: end,
        ),
      );
      cursor = end + _gapMs.round();
      if (mounted) {
        setState(() {
          _stage = 'Speaking line ${index + 1} of ${lines.length}…';
          _progress = (index + 1) / lines.length;
        });
      }
    }
    return results;
  }

  /// Opens the full voice surface, then adopts and persists its result.
  Future<void> _openVoiceSettings() async {
    final current = NarrationSettings(
      voice: _voice ?? _saved.voice,
      locale: _localeOf(_voice) ?? _saved.locale,
      rate: _rate.round(),
      gapMs: _gapMs.round(),
      provider: _provider,
      cloudVoiceId: _cloudVoiceId,
      azureRegion: _azureRegion,
    );
    final chosen = await showDialog<NarrationSettings>(
      context: context,
      builder: (context) => NarrationSettingsDialog(initial: current),
    );
    if (!mounted || chosen == null) return;
    setState(() {
      _saved = chosen;
      if (chosen.voice.isNotEmpty) _voice = chosen.voice;
      _rate = chosen.rate.toDouble();
      _gapMs = chosen.gapMs.toDouble();
    });
  }

  String? _localeOf(String? voice) {
    for (final option in _voices) {
      if (option.name == voice) return option.locale;
    }
    return null;
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
    if (_provider == 'system' && voice == null) {
      setState(() => _error = 'Choose a voice first.');
      return;
    }
    if (_provider != 'system' && _cloudVoiceId.isEmpty) {
      setState(() => _error = 'Choose a cloud voice first.');
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
      _stage = 'Preparing narration…';
      _progress = null;
    });
    // The inline choices are the next run's defaults; a failed write is not a
    // reason to refuse the narration.
    try {
      await _settings.save(
        NarrationSettings(
          voice: voice ?? _saved.voice,
          locale: _localeOf(voice) ?? _saved.locale,
          rate: _rate.round(),
          gapMs: _gapMs.round(),
          provider: _provider,
          cloudVoiceId: _cloudVoiceId,
          azureRegion: _azureRegion,
        ),
      );
    } catch (_) {
      // Persisting a preference is best-effort.
    }
    try {
      final directory = await _voiceover.outputDirectory();
      final synthesized = _provider == 'system'
          ? await _voiceover.synthesize(
              lines: lines,
              voice: voice!,
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
            )
          : await _synthesizeCloud(lines, directory);
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
    } on CloudVoiceException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } on ProjectGatewayException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } catch (error) {
      if (mounted) {
        setState(() => _error = 'Voiceover could not be completed: $error');
      }
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
                  hintText: 'Xin chào, đây là Opencut Reinforced.\nBạn có thể dựng phim ngay trên máy.',
                ),
              ),
              const SizedBox(height: 12),
              DropdownButtonFormField<String>(
                key: ValueKey('voiceover-provider-$_provider'),
                initialValue: _provider,
                isExpanded: true,
                decoration: const InputDecoration(
                  border: OutlineInputBorder(),
                  labelText: 'Voice source',
                ),
                items: const [
                  DropdownMenuItem(
                    value: 'system',
                    child: Text('Installed voices (offline)'),
                  ),
                  DropdownMenuItem(
                    value: 'elevenlabs',
                    child: Text('ElevenLabs (cloud)'),
                  ),
                  DropdownMenuItem(
                    value: 'azure',
                    child: Text('Microsoft Azure (cloud)'),
                  ),
                ],
                onChanged: _busy
                    ? null
                    : (value) {
                        if (value != null) unawaited(_setProvider(value));
                      },
              ),
              const SizedBox(height: 8),
              if (_provider == 'system') ...[
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
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: DropdownButtonFormField<String>(
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
                                child: Text(
                                  voice.label,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                          ],
                          onChanged: _busy
                              ? null
                              : (value) => setState(() => _voice = value),
                        ),
                      ),
                      const SizedBox(width: 8),
                      TextButton(
                        key: const ValueKey('voiceover-manage-voices'),
                        onPressed: _busy ? null : _openVoiceSettings,
                        child: const Text('Voice settings…'),
                      ),
                    ],
                  ),
              ] else ...[
                if (_cloudLoading) ...[
                  Row(
                    children: [
                      const SizedBox.square(
                        dimension: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                      const SizedBox(width: 8),
                      Text(
                        'Loading ${_provider == 'elevenlabs' ? 'ElevenLabs' : 'Azure'} voices…',
                        style: TextStyle(color: OrColors.textSecondary),
                      ),
                    ],
                  ),
                ] else if (_cloudError != null)
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        _cloudError!,
                        key: const ValueKey('voiceover-cloud-error'),
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.error,
                        ),
                      ),
                      Row(
                        children: [
                          TextButton(
                            key: const ValueKey('voiceover-cloud-retry'),
                            onPressed: _busy ? null : _loadCloudVoices,
                            child: const Text('Retry'),
                          ),
                          TextButton(
                            onPressed: _busy ? null : _openVoiceSettings,
                            child: const Text('Voice settings…'),
                          ),
                        ],
                      ),
                    ],
                  )
                else if (_cloudVoices.isEmpty)
                  Row(
                    children: [
                      const Expanded(
                        child: Text(
                          'No cloud voice is available for this key.',
                        ),
                      ),
                      TextButton(
                        onPressed: _busy ? null : _loadCloudVoices,
                        child: const Text('Retry'),
                      ),
                    ],
                  )
                else
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: DropdownButtonFormField<String>(
                          key: ValueKey('voiceover-cloud-voice-$_cloudVoiceId'),
                          initialValue: _cloudVoiceId.isEmpty
                              ? null
                              : _cloudVoiceId,
                          isExpanded: true,
                          decoration: const InputDecoration(
                            border: OutlineInputBorder(),
                            labelText: 'Cloud voice',
                          ),
                          items: [
                            for (final voice in _cloudVoices)
                              DropdownMenuItem(
                                value: voice.id,
                                child: Text(
                                  voice.label,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                          ],
                          onChanged: _busy
                              ? null
                              : (value) =>
                                    setState(() => _cloudVoiceId = value ?? ''),
                        ),
                      ),
                      const SizedBox(width: 8),
                      TextButton(
                        onPressed: _busy ? null : _openVoiceSettings,
                        child: const Text('Voice settings…'),
                      ),
                    ],
                  ),
              ],
              if (_provider == 'system') ...[
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
              ],
              const SizedBox(height: 8),
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
