import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import 'narration_settings.dart';
import 'voiceover_service.dart';

/// Chooses the installed narration voice and the defaults a narration run uses.
///
/// The list comes from the machine, never from a product constant, so a host
/// without the previously hardcoded Vietnamese voice can still narrate. The
/// result pops as [NarrationSettings] and is persisted by the caller through
/// [NarrationSettingsStore].
/// What the narration settings dialog edits, named for the active source.
///
/// The dialog only manages the installed voices. When narration currently uses
/// a cloud provider or a local server, saying so keeps a user from believing
/// this screen changes the voice they actually hear.
String narrationSettingsIntroText(String provider) {
  final source = switch (provider) {
    'local' => 'the local server',
    'elevenlabs' => 'ElevenLabs',
    'azure' => 'Microsoft Azure',
    _ => '',
  };
  if (source.isEmpty) {
    return 'Voices installed on this Mac. The assistant and the Audio tool use '
        'the saved default; nothing is uploaded.';
  }
  return 'Narration currently uses $source, chosen in Generate voiceover. This '
      'screen sets the installed-voice fallback; nothing is uploaded.';
}

class NarrationSettingsDialog extends StatefulWidget {
  const NarrationSettingsDialog({
    super.key,
    required this.initial,
    this.voiceover,
    this.store,
    this.supportDirectory,
    this.temporaryDirectory,
  });

  final NarrationSettings initial;
  final VoiceoverService? voiceover;
  final NarrationSettingsStore? store;
  final Future<Directory> Function()? supportDirectory;
  final Future<Directory> Function()? temporaryDirectory;

  @override
  State<NarrationSettingsDialog> createState() =>
      _NarrationSettingsDialogState();
}

class _NarrationSettingsDialogState extends State<NarrationSettingsDialog> {
  final TextEditingController _search = TextEditingController();
  final VoiceoverService _voiceover = VoiceoverService();
  late final NarrationSettingsStore _store =
      widget.store ??
      NarrationSettingsStore(supportDirectory: widget.supportDirectory);

  List<VoiceOption> _voices = const [];
  String _voice = '';
  double _rate = 180;
  double _gapMs = 250;
  bool _loading = true;
  bool _previewing = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _rate = widget.initial.rate.toDouble();
    _gapMs = widget.initial.gapMs.toDouble();
    _load();
  }

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final voices = await _effectiveVoiceover.listVoices();
      if (!mounted) return;
      final preferred =
          NarrationSettings.preferredVoiceName(voices, widget.initial) ??
          (voices.isEmpty ? null : voices.first.name);
      final selected = _voiceByName(voices, preferred);
      setState(() {
        _voices = voices;
        _voice = selected?.name ?? '';
        _loading = false;
      });
      if (voices.isEmpty) {
        setState(() => _error = 'No installed voice was found.');
      }
    } on VoiceoverException catch (error) {
      if (mounted) {
        setState(() {
          _loading = false;
          _error = error.message;
        });
      }
    } on Object catch (error) {
      // An unexpected failure must not leave the dialog loading forever.
      if (mounted) {
        setState(() {
          _loading = false;
          _error = 'The voice list could not be loaded: $error';
        });
      }
    }
  }

  VoiceoverService get _effectiveVoiceover => widget.voiceover ?? _voiceover;

  List<VoiceOption> get _filtered {
    final query = _search.text.trim().toLowerCase();
    if (query.isEmpty) return _voices;
    return _voices
        .where(
          (voice) =>
              voice.name.toLowerCase().contains(query) ||
              voice.locale.toLowerCase().contains(query),
        )
        .toList(growable: false);
  }

  Future<void> _preview() async {
    if (_voice.isEmpty || _previewing) return;
    setState(() {
      _previewing = true;
      _error = null;
    });
    try {
      final directory =
          await (widget.temporaryDirectory ?? getTemporaryDirectory)();
      VoiceOption? selected;
      for (final option in _voices) {
        if (option.name == _voice) {
          selected = option;
          break;
        }
      }
      // Speak a sample in the voice's own language, so an English voice does
      // not read a Vietnamese sentence and vice versa.
      final languages = (selected?.locale ?? 'vi').toLowerCase();
      await _effectiveVoiceover.preview(
        voice: _voice,
        text: languages.startsWith('vi')
            ? 'Xin chào, đây là giọng đọc của Opencut Reinforced.'
            : 'Hello, this is a voice preview for Opencut Reinforced.',
        directory: Directory('${directory.path}/voice-previews'),
        rate: _rate.round(),
      );
    } on VoiceoverException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } catch (_) {
      if (mounted) setState(() => _error = 'The voice preview could not play.');
    } finally {
      if (mounted) setState(() => _previewing = false);
    }
  }

  VoiceOption? _voiceByName(List<VoiceOption> voices, String? name) {
    if (name == null || name.isEmpty) return null;
    for (final voice in voices) {
      if (voice.name == name) return voice;
    }
    return null;
  }

  Future<void> _save() async {
    final selected = _voiceByName(_voices, _voice);
    // The screen edits four fields; everything else it does not own is carried
    // forward by copyWith, so a field added later cannot be dropped here by
    // omission.
    final settings = widget.initial.copyWith(
      voice: selected?.name ?? '',
      locale: selected?.locale ?? '',
      rate: _rate.round(),
      gapMs: _gapMs.round(),
    );
    try {
      await _store.save(settings);
    } catch (_) {
      if (mounted) {
        setState(() => _error = 'The narration settings could not be saved.');
        return;
      }
    }
    if (mounted) Navigator.of(context).pop(settings);
  }

  @override
  Widget build(BuildContext context) {
    final filtered = _filtered;
    return AlertDialog(
      title: const Text('Narration voice'),
      content: SizedBox(
        width: 680,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                narrationSettingsIntroText(widget.initial.provider),
                style: TextStyle(color: OrColors.textSecondary, fontSize: 12),
              ),
              const SizedBox(height: OrSpacing.x2),
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      key: const ValueKey('narration-voice-search'),
                      controller: _search,
                      enabled: !_loading,
                      onChanged: (_) => setState(() {}),
                      decoration: const InputDecoration(
                        border: OutlineInputBorder(),
                        isDense: true,
                        labelText: 'Search voices or languages',
                        hintText: 'Vietnamese, vi_VN, Linh…',
                      ),
                    ),
                  ),
                  const SizedBox(width: OrSpacing.x2),
                  OutlinedButton.icon(
                    key: const ValueKey('narration-voice-preview'),
                    onPressed: _voice.isEmpty || _loading || _previewing
                        ? null
                        : _preview,
                    icon: const Icon(Icons.play_arrow, size: 18),
                    label: Text(_previewing ? 'Playing…' : 'Preview'),
                  ),
                ],
              ),
              const SizedBox(height: OrSpacing.x2),
              Container(
                height: 240,
                decoration: BoxDecoration(
                  border: Border.all(color: OrColors.border),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: _loading
                    ? const Center(
                        child: SizedBox.square(
                          dimension: 20,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        ),
                      )
                    : filtered.isEmpty
                    ? Center(
                        child: Text(
                          'No voice matches this search.',
                          style: TextStyle(color: OrColors.textMuted),
                        ),
                      )
                    : ListView.builder(
                        itemCount: filtered.length,
                        itemBuilder: (context, index) {
                          final voice = filtered[index];
                          final selected = voice.name == _voice;
                          return ListTile(
                            key: ValueKey('narration-voice-${voice.name}'),
                            dense: true,
                            selected: selected,
                            selectedTileColor: OrColors.surfaceSelected,
                            leading: Icon(
                              selected
                                  ? Icons.radio_button_checked
                                  : Icons.radio_button_unchecked,
                              size: 18,
                              color: selected
                                  ? OrColors.text
                                  : OrColors.textMuted,
                            ),
                            title: Text(voice.displayName),
                            subtitle: Text(
                              voice.locale,
                              style: TextStyle(color: OrColors.textMuted),
                            ),
                            onTap: () => setState(() => _voice = voice.name),
                          );
                        },
                      ),
              ),
              const SizedBox(height: OrSpacing.x2),
              Row(
                children: [
                  const SizedBox(width: 96, child: Text('Rate')),
                  Expanded(
                    child: Slider(
                      key: const ValueKey('narration-rate'),
                      value: _rate,
                      min: 120,
                      max: 240,
                      divisions: 24,
                      label: '${_rate.round()} wpm',
                      onChanged: (value) => setState(() => _rate = value),
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
                      key: const ValueKey('narration-gap'),
                      value: _gapMs,
                      min: 0,
                      max: 1000,
                      divisions: 20,
                      label: '${_gapMs.round()} ms',
                      onChanged: (value) => setState(() => _gapMs = value),
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
              if (_error != null) ...[
                const SizedBox(height: OrSpacing.x1),
                Text(
                  _error!,
                  key: const ValueKey('narration-voice-error'),
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const ValueKey('narration-voice-save'),
          onPressed: _loading ? null : _save,
          child: const Text('Save default'),
        ),
      ],
    );
  }
}
