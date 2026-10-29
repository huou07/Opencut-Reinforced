import 'dart:async';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../project/project_gateway.dart';
import 'cloud_voice_service.dart';
import 'local_tts_service.dart';
import 'narration_settings.dart';
import 'narration_settings_dialog.dart';
import 'record_voice_dialog.dart';
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
  final LocalTtsService _local = LocalTtsService();
  final NarrationSettingsStore _settings = NarrationSettingsStore();
  final TextEditingController _localUrl = TextEditingController();
  final TextEditingController _localVoiceName = TextEditingController();
  late final VoiceoverApplier _applier = VoiceoverApplier(widget.gateway);

  List<VoiceOption> _voices = const [];
  List<CloudVoice> _cloudVoices = const [];
  NarrationSettings _saved = const NarrationSettings();
  String? _voice;
  String _provider = 'system';
  String _cloudVoiceId = '';

  /// The voice chosen per cloud provider in this dialog, so switching the
  /// source does not throw away a deliberate pick before Generate.
  final Map<String, String> _cloudSelections = {};
  String _azureRegion = '';
  String _localBaseUrl = LocalTtsService.defaultBaseUrl;
  String _localVoiceId = '';

  /// The local voice chosen in this dialog, including one enrolled here. It
  /// survives a switch to another source and back.
  String _localSelection = '';
  String _localReferencePath = '';
  List<LocalTtsVoice> _localVoices = const [];
  LocalTtsHealth? _localHealth;
  bool _localLoading = false;
  bool _localEnrolling = false;
  String? _connectError;
  String? _cloneError;
  String? _cloneNameError;
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
    _localUrl.dispose();
    _localVoiceName.dispose();
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
        _localBaseUrl = saved.localBaseUrl.isEmpty
            ? LocalTtsService.defaultBaseUrl
            : saved.localBaseUrl;
        _localUrl.text = _localBaseUrl;
        _localVoiceId = saved.localVoiceId;
        _localVoiceName.text = saved.localVoiceName;
        _localReferencePath = saved.localReferencePath;
      });
      if (_provider == 'local' && mounted) {
        await _loadLocalVoices();
      } else if (_provider != 'system' && mounted) {
        await _loadCloudVoices();
      }
    } on VoiceoverException catch (error) {
      if (mounted) setState(() => _error = error.message);
    }
  }

  CloudVoiceProvider get _cloudProvider => _provider == 'elevenlabs'
      ? CloudVoiceProvider.elevenLabs
      : CloudVoiceProvider.azure;

  /// Every source the dialog can speak through, in review order: the
  /// installed voice first because it needs no setup at all.
  static const List<String> _voiceSources = [
    'system',
    'local',
    'elevenlabs',
    'azure',
  ];

  static String _voiceSourceLabel(String source) => switch (source) {
    'system' => 'Installed voices (offline)',
    'local' => 'Local server (clones a voice)',
    'elevenlabs' => 'ElevenLabs (cloud)',
    'azure' => 'Azure (cloud)',
    _ => source,
  };

  Future<void> _setProvider(String provider) async {
    if (provider == _provider) return;
    if (_provider == 'local' && _localVoiceId.isNotEmpty) {
      _localSelection = _localVoiceId;
    }
    if (_provider != 'system' &&
        _provider != 'local' &&
        _cloudVoiceId.isNotEmpty) {
      _cloudSelections[_cloudProvider.wireName] = _cloudVoiceId;
    }
    final nextCloud = provider == 'azure'
        ? CloudVoiceProvider.azure
        : CloudVoiceProvider.elevenLabs;
    final savedCloudId = provider == _saved.provider ? _saved.cloudVoiceId : '';
    final savedLocalId = provider == _saved.provider ? _saved.localVoiceId : '';
    setState(() {
      _provider = provider;
      _cloudVoices = const [];
      _cloudError = null;
      _cloudVoiceId = provider == 'system'
          ? ''
          : (_cloudSelections[nextCloud.wireName] ?? savedCloudId);
      _localVoices = const [];
      _connectError = null;
      _cloneError = null;
      _cloneNameError = null;
      _localHealth = null;
      _localVoiceId = provider == 'local'
          ? (_localSelection.isNotEmpty ? _localSelection : savedLocalId)
          : '';
    });
    if (provider == 'local') {
      await _loadLocalVoices();
    } else if (provider != 'system') {
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

  Future<void> _loadLocalVoices() async {
    final base = LocalTtsService.normalizeBaseUrl(_localUrl.text);
    setState(() {
      _localLoading = true;
      _connectError = null;
    });
    try {
      final status = await _local.probe(base);
      if (!mounted) return;
      final savedId = _localVoiceId.isNotEmpty
          ? _localVoiceId
          : (_localSelection.isNotEmpty
                ? _localSelection
                : _saved.localVoiceId);
      final preferred = status.voices.any((voice) => voice.id == savedId)
          ? savedId
          : (status.voices.isEmpty ? '' : status.voices.first.id);
      setState(() {
        _localBaseUrl = base;
        _localUrl.text = base;
        _localHealth = status.health;
        _localVoices = status.voices;
        _localVoiceId = preferred;
        _localLoading = false;
      });
    } on LocalTtsException catch (error) {
      if (!mounted) return;
      setState(() {
        _localLoading = false;
        _connectError = error.message;
        // Voices on the screen belong to the server that last answered; after
        // a failed probe they no longer describe the field's URL.
        _localVoices = const [];
        _localHealth = null;
        _localVoiceId = '';
      });
    }
  }

  Future<void> _enrollLocal() async {
    final name = _localVoiceName.text.trim();
    if (name.isEmpty) {
      setState(() => _cloneNameError = 'Give the cloned voice a name first.');
      return;
    }
    if (_localReferencePath.isEmpty) {
      setState(
        () => _cloneError = 'Choose or record a 3–8 s reference clip first.',
      );
      return;
    }
    setState(() {
      _localEnrolling = true;
      _cloneError = null;
      _cloneNameError = null;
    });
    try {
      final id = await _local.enrollVoice(
        baseUrl: LocalTtsService.normalizeBaseUrl(_localUrl.text),
        name: name,
        referencePath: _localReferencePath,
      );
      if (!mounted) return;
      setState(() {
        _localVoiceId = id;
        _localSelection = id;
        _localEnrolling = false;
      });
      await _loadLocalVoices();
    } on LocalTtsException catch (error) {
      if (!mounted) return;
      setState(() {
        _localEnrolling = false;
        _cloneError = error.message;
      });
    }
  }

  Future<void> _chooseReference() async {
    const type = XTypeGroup(
      label: 'Reference audio (3–8 s)',
      extensions: ['wav', 'mp3', 'flac', 'ogg', 'm4a'],
    );
    final file = await openFile(acceptedTypeGroups: const [type]);
    if (file == null || !mounted) return;
    setState(() => _localReferencePath = file.path);
  }

  Future<void> _recordReference() async {
    final recording = await showDialog<VoiceRecording>(
      context: context,
      builder: (context) => const RecordVoiceDialog(),
    );
    if (recording == null || !mounted) return;
    setState(() => _localReferencePath = recording.path);
  }

  Future<void> _reEnrollLocal() async {
    final name = _localVoiceName.text.trim().isEmpty
        ? (_saved.localVoiceName.isEmpty ? 'cloned' : _saved.localVoiceName)
        : _localVoiceName.text.trim();
    final id = await _local.enrollVoice(
      baseUrl: _localBaseUrl,
      name: name,
      referencePath: _localReferencePath,
    );
    _localVoiceId = id;
    // The server only remembers enrolments for the life of its process, so the
    // id it just handed back is the one the next launch must ask for.
    final saved = _saved.copyWith(
      localBaseUrl: _localBaseUrl,
      localVoiceId: id,
      localVoiceName: name,
      localReferencePath: _localReferencePath,
    );
    _saved = saved;
    await _settings.save(saved);
  }

  Future<List<VoiceoverLine>> _synthesizeLocal(
    List<String> lines,
    Directory directory,
  ) async {
    // The server field is the source of truth. A URL the owner typed but has
    // not connected yet must not silently fall back to the last good server.
    final base = LocalTtsService.normalizeBaseUrl(_localUrl.text);
    if (base != _localBaseUrl) {
      await _loadLocalVoices();
      if (base != _localBaseUrl) {
        throw LocalTtsException(
          'server-unreachable',
          _connectError ?? 'The local server did not answer at $base.',
        );
      }
    }
    await directory.create(recursive: true);
    final results = <VoiceoverLine>[];
    var cursor = 0;
    for (var index = 0; index < lines.length; index += 1) {
      final text = lines[index];
      final path = '${directory.path}/local-line-${index + 1}.wav';
      int duration;
      try {
        duration = await _local.synthesize(
          baseUrl: base,
          text: text,
          voiceId: _localVoiceId,
          outputPath: path,
          sampleRate: _localHealth?.sampleRate ?? 0,
        );
      } on LocalTtsException catch (error) {
        if (_localReferencePath.isEmpty ||
            !error.message.toLowerCase().contains('unknown voice')) {
          rethrow;
        }
        // The server keeps enrolled voices in process memory only; after a
        // restart it answers "unknown voice". Enrol the saved clip and retry.
        await _reEnrollLocal();
        duration = await _local.synthesize(
          baseUrl: base,
          text: text,
          voiceId: _localVoiceId,
          outputPath: path,
          sampleRate: _localHealth?.sampleRate ?? 0,
        );
      }
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
      localBaseUrl: _localBaseUrl,
      localVoiceId: _localVoiceId,
      localVoiceName: _localVoiceName.text.trim(),
      localReferencePath: _localReferencePath,
    );
    final chosen = await showDialog<NarrationSettings>(
      context: context,
      builder: (context) => NarrationSettingsDialog(initial: current),
    );
    if (!mounted || chosen == null) return;
    final providerChanged = chosen.provider != _provider;
    setState(() {
      _saved = chosen;
      if (chosen.voice.isNotEmpty) _voice = chosen.voice;
      _rate = chosen.rate.toDouble();
      _gapMs = chosen.gapMs.toDouble();
      // The management dialog is the authority for the next run: adopt its
      // source and voice, otherwise the next Generate would persist the stale
      // inline values over the choice just made there.
      _provider = chosen.provider;
      _cloudVoiceId = chosen.cloudVoiceId;
      _azureRegion = chosen.azureRegion;
      _localBaseUrl = chosen.localBaseUrl.isEmpty
          ? LocalTtsService.defaultBaseUrl
          : chosen.localBaseUrl;
      _localUrl.text = _localBaseUrl;
      _localVoiceId = chosen.localVoiceId;
      _localSelection = chosen.localVoiceId;
      _localVoiceName.text = chosen.localVoiceName;
      _localReferencePath = chosen.localReferencePath;
    });
    if (!providerChanged) return;
    if (_provider == 'local') {
      await _loadLocalVoices();
    } else if (_provider != 'system') {
      await _loadCloudVoices();
    }
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
    if (_provider == 'local' && _localVoiceId.isEmpty) {
      // Probe first: "choose a voice" is the wrong complaint when the server
      // the owner typed does not answer.
      await _loadLocalVoices();
      if (_localVoiceId.isEmpty) {
        if (mounted) {
          setState(
            () => _error =
                _connectError ?? 'Choose or clone a local voice first.',
          );
        }
        return;
      }
    }
    if (_provider != 'system' &&
        _provider != 'local' &&
        _cloudVoiceId.isEmpty) {
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
          localBaseUrl: _localBaseUrl,
          localVoiceId: _localVoiceId,
          localVoiceName: _localVoiceName.text.trim(),
          localReferencePath: _localReferencePath,
        ),
      );
    } catch (_) {
      // Persisting a preference is best-effort.
    }
    try {
      final directory = await _voiceover.outputDirectory();
      final List<VoiceoverLine> synthesized;
      if (_provider == 'local') {
        synthesized = await _synthesizeLocal(lines, directory);
      } else if (_provider != 'system') {
        synthesized = await _synthesizeCloud(lines, directory);
      } else {
        synthesized = await _voiceover.synthesize(
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
        );
      }
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
    } on LocalTtsException catch (error) {
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
    return PopScope(
      canPop: !_busy,
      child: AlertDialog(
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
                  'at its exact duration. Narration runs offline: an installed '
                  'macOS voice, or a local speech server with voice cloning. '
                  'Nothing is uploaded.',
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
                Text(
                  'Voice source',
                  style: TextStyle(color: OrColors.textSecondary, fontSize: 12),
                ),
                const SizedBox(height: 6),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    for (final source in _voiceSources)
                      ChoiceChip(
                        key: ValueKey('voiceover-provider-$source'),
                        label: Text(_voiceSourceLabel(source)),
                        selected: _provider == source,
                        onSelected: _busy
                            ? null
                            : (selected) {
                                if (selected) unawaited(_setProvider(source));
                              },
                      ),
                  ],
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
                            key: ValueKey(
                              'voiceover-voice-${_voice ?? 'none'}',
                            ),
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
                ] else if (_provider == 'local') ...[
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          key: const ValueKey('voiceover-local-url'),
                          controller: _localUrl,
                          enabled: !_busy,
                          keyboardType: TextInputType.url,
                          autocorrect: false,
                          textInputAction: TextInputAction.done,
                          onSubmitted: (_) {
                            if (!_busy && !_localLoading) _loadLocalVoices();
                          },
                          decoration: const InputDecoration(
                            border: OutlineInputBorder(),
                            labelText: 'Local server',
                            hintText: 'http://127.0.0.1:8000',
                            isDense: true,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      TextButton(
                        key: const ValueKey('voiceover-local-connect'),
                        onPressed: _busy || _localLoading
                            ? null
                            : _loadLocalVoices,
                        child: Text(_localLoading ? 'Checking…' : 'Connect'),
                      ),
                    ],
                  ),
                  if (_connectError != null) ...[
                    const SizedBox(height: 6),
                    Semantics(
                      liveRegion: true,
                      child: Text(
                        _connectError!,
                        key: const ValueKey('voiceover-local-error'),
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.error,
                        ),
                      ),
                    ),
                  ],
                  if (_localHealth != null) ...[
                    const SizedBox(height: 4),
                    Semantics(
                      liveRegion: true,
                      child: Text(
                        'Connected · ${_localHealth!.summary}',
                        key: const ValueKey('voiceover-local-health'),
                        style: TextStyle(color: OrColors.textSecondary),
                      ),
                    ),
                  ] else if (_localVoices.isEmpty) ...[
                    const SizedBox(height: 4),
                    Text(
                      _localHealth == null
                          ? 'Press Connect to list the voices on this server.'
                          : 'No voice on the server yet. Enrol one from a clip '
                                'below.',
                      style: TextStyle(color: OrColors.textSecondary),
                    ),
                  ],
                  const SizedBox(height: 8),
                  DropdownButtonFormField<String>(
                    key: ValueKey('voiceover-local-voice-$_localVoiceId'),
                    initialValue: _localVoiceId.isEmpty ? null : _localVoiceId,
                    isExpanded: true,
                    decoration: const InputDecoration(
                      border: OutlineInputBorder(),
                      labelText: 'Local voice',
                    ),
                    items: [
                      for (final voice in _localVoices)
                        DropdownMenuItem(
                          value: voice.id,
                          child: Tooltip(
                            message: voice.label,
                            child: Text(
                              voice.label,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ),
                    ],
                    onChanged: _busy
                        ? null
                        : (value) =>
                              setState(() => _localVoiceId = value ?? ''),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Voice cloning: choose or record a 3–8 s clip, name it, then '
                    'enrol. Only clone a voice you have the right to use.',
                    style: TextStyle(
                      color: OrColors.textSecondary,
                      fontSize: 12,
                    ),
                  ),
                  const SizedBox(height: 6),
                  Row(
                    children: [
                      Expanded(
                        child: Tooltip(
                          message: _localReferencePath.isEmpty
                              ? 'No reference clip yet.'
                              : _localReferencePath,
                          child: Text(
                            _localReferencePath.isEmpty
                                ? 'No reference clip yet.'
                                : _localReferencePath,
                            key: const ValueKey('voiceover-local-reference'),
                            overflow: TextOverflow.ellipsis,
                            style: TextStyle(
                              color: OrColors.textSecondary,
                              fontSize: 12,
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      OutlinedButton(
                        key: const ValueKey('voiceover-local-choose-clip'),
                        onPressed: _busy || _localEnrolling
                            ? null
                            : _chooseReference,
                        child: const Text('Choose clip…'),
                      ),
                      TextButton(
                        key: const ValueKey('voiceover-local-record-clip'),
                        onPressed: _busy || _localEnrolling
                            ? null
                            : _recordReference,
                        child: const Text('Record…'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          key: const ValueKey('voiceover-local-clone-name'),
                          controller: _localVoiceName,
                          enabled: !_busy && !_localEnrolling,
                          onChanged: (_) {
                            if (_cloneNameError != null) {
                              setState(() => _cloneNameError = null);
                            }
                          },
                          decoration: InputDecoration(
                            border: const OutlineInputBorder(),
                            labelText: 'Clone name',
                            isDense: true,
                            errorText: _cloneNameError,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      FilledButton.tonal(
                        key: const ValueKey('voiceover-local-enroll'),
                        onPressed: _busy || _localEnrolling
                            ? null
                            : _enrollLocal,
                        child: Text(
                          _localEnrolling ? 'Enrolling…' : 'Enrol voice',
                        ),
                      ),
                    ],
                  ),
                  if (_cloneError != null) ...[
                    const SizedBox(height: 6),
                    Semantics(
                      liveRegion: true,
                      child: Text(
                        _cloneError!,
                        key: const ValueKey('voiceover-local-clone-error'),
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.error,
                        ),
                      ),
                    ),
                  ],
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
                            key: ValueKey(
                              'voiceover-cloud-voice-$_cloudVoiceId',
                            ),
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
                                : (value) => setState(
                                    () => _cloudVoiceId = value ?? '',
                                  ),
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
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
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
      ),
    );
  }
}
