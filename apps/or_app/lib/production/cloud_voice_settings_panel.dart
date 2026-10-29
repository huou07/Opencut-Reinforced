import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import 'cloud_voice_service.dart';
import 'narration_settings.dart';

/// Provider cards for the keys OR needs to call a cloud voice.
///
/// The key field is write-only: after saving, the card says **Configured** and
/// never shows the secret again. The keychain owns the value; this widget only
/// knows whether the account exists.
class CloudVoiceSettingsPanel extends StatefulWidget {
  const CloudVoiceSettingsPanel({
    super.key,
    required this.settings,
    required this.onChanged,
    this.keychain,
    this.service,
  });

  final NarrationSettings settings;
  final ValueChanged<NarrationSettings> onChanged;
  final NarrationKeychain? keychain;
  final CloudTtsService? service;

  @override
  State<CloudVoiceSettingsPanel> createState() =>
      _CloudVoiceSettingsPanelState();
}

class _CloudVoiceSettingsPanelState extends State<CloudVoiceSettingsPanel> {
  late final NarrationKeychain _keychain =
      widget.keychain ?? const NarrationKeychain();
  late final CloudTtsService _service =
      widget.service ?? CloudTtsService(keychain: _keychain);

  final Map<CloudVoiceProvider, TextEditingController> _keys = {
    for (final provider in CloudVoiceProvider.values)
      provider: TextEditingController(),
  };
  final Map<CloudVoiceProvider, bool> _configured = {
    for (final provider in CloudVoiceProvider.values) provider: false,
  };
  final Map<CloudVoiceProvider, bool> _busy = {
    for (final provider in CloudVoiceProvider.values) provider: false,
  };
  final Map<CloudVoiceProvider, String?> _status = {
    for (final provider in CloudVoiceProvider.values) provider: null,
  };
  final TextEditingController _region = TextEditingController();
  final TextEditingController _samplePath = TextEditingController();
  final TextEditingController _cloneName = TextEditingController();
  bool _loaded = false;
  bool _keyCheckFailed = false;
  bool _consent = false;
  bool _cloning = false;
  String? _cloneStatus;

  @override
  void initState() {
    super.initState();
    _region.text = widget.settings.azureRegion;
    _loadConfigured();
  }

  @override
  void didUpdateWidget(covariant CloudVoiceSettingsPanel oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.settings.azureRegion != widget.settings.azureRegion &&
        _region.text != widget.settings.azureRegion) {
      _region.text = widget.settings.azureRegion;
    }
  }

  @override
  void dispose() {
    for (final controller in _keys.values) {
      controller.dispose();
    }
    _region.dispose();
    _samplePath.dispose();
    _cloneName.dispose();
    super.dispose();
  }

  Future<void> _loadConfigured() async {
    // Both checks run together: each one may wait out the keychain budget, and
    // a sequential loop would double the worst case.
    final results = await Future.wait(
      CloudVoiceProvider.values.map((provider) async {
        try {
          return (provider, await _keychain.has(provider.wireName));
        } on CloudVoiceException {
          return (provider, null);
        }
      }),
    );
    if (!mounted) return;
    setState(() {
      _loaded = true;
      for (final (provider, configured) in results) {
        if (configured != null) _configured[provider] = configured;
      }
      // A failed check must not read as "no key configured".
      _keyCheckFailed = results.any((entry) => entry.$2 == null);
    });
  }

  Future<void> _saveKey(CloudVoiceProvider provider) async {
    final value = _keys[provider]!.text.trim();
    if (value.isEmpty) {
      setState(() => _status[provider] = 'Enter a key first.');
      return;
    }
    setState(() {
      _busy[provider] = true;
      _status[provider] = null;
    });
    try {
      await _keychain.write(provider.wireName, value);
      _keys[provider]!.clear();
      if (mounted) {
        setState(() {
          _configured[provider] = true;
          _status[provider] = 'Key saved to the macOS keychain.';
        });
      }
    } on CloudVoiceException catch (error) {
      if (mounted) setState(() => _status[provider] = error.message);
    } finally {
      if (mounted) setState(() => _busy[provider] = false);
    }
  }

  Future<void> _removeKey(CloudVoiceProvider provider) async {
    setState(() {
      _busy[provider] = true;
      _status[provider] = null;
    });
    try {
      await _keychain.delete(provider.wireName);
      if (mounted) {
        setState(() {
          _configured[provider] = false;
          _status[provider] = 'Key removed.';
        });
      }
    } on CloudVoiceException catch (error) {
      if (mounted) setState(() => _status[provider] = error.message);
    } finally {
      if (mounted) setState(() => _busy[provider] = false);
    }
  }

  Future<void> _testKey(CloudVoiceProvider provider) async {
    final typed = _keys[provider]!.text.trim();
    setState(() {
      _busy[provider] = true;
      _status[provider] = null;
    });
    try {
      if (typed.isNotEmpty) {
        await _keychain.write(provider.wireName, typed);
        _keys[provider]!.clear();
        if (mounted) setState(() => _configured[provider] = true);
      }
      if (provider == CloudVoiceProvider.azure) {
        final region = _region.text.trim();
        if (region.isNotEmpty && region != widget.settings.azureRegion) {
          widget.onChanged(widget.settings.copyWith(azureRegion: region));
        }
        await _service.testKey(provider, azureRegion: region);
        if (mounted) setState(() => _status[provider] = 'The key works.');
      } else {
        final voices = await _service.listVoices(provider);
        if (mounted) {
          setState(
            () => _status[provider] =
                'The key works: ${voices.length} voices available.',
          );
        }
      }
    } on CloudVoiceException catch (error) {
      if (mounted) setState(() => _status[provider] = error.message);
    } catch (_) {
      if (mounted) setState(() => _status[provider] = 'The test failed.');
    } finally {
      if (mounted) setState(() => _busy[provider] = false);
    }
  }

  void _saveRegion() {
    final text = _region.text.trim();
    if (text == widget.settings.azureRegion) return;
    widget.onChanged(widget.settings.copyWith(azureRegion: text));
  }

  Future<void> _browseSample() async {
    const wav = XTypeGroup(label: 'WAV recording', extensions: ['wav']);
    final file = await openFile(acceptedTypeGroups: const [wav]);
    if (file != null) _samplePath.text = file.path;
  }

  Future<void> _cloneVoice() async {
    if (!_consent) {
      setState(
        () => _cloneStatus =
            'Confirm that you own this voice and consent to cloning it.',
      );
      return;
    }
    if (_samplePath.text.trim().isEmpty) {
      setState(() => _cloneStatus = 'Choose a WAV sample first.');
      return;
    }
    if (_cloneName.text.trim().isEmpty) {
      setState(() => _cloneStatus = 'Name the personal voice.');
      return;
    }
    setState(() {
      _cloning = true;
      _cloneStatus = null;
    });
    try {
      final voice = await _service.cloneVoice(
        name: _cloneName.text,
        samplePath: _samplePath.text.trim(),
      );
      if (mounted) {
        setState(() {
          _cloneStatus =
              'Created "${voice.name}". It appears in the cloud voice list.';
        });
      }
    } on CloudVoiceException catch (error) {
      if (mounted) setState(() => _cloneStatus = error.message);
    } catch (_) {
      if (mounted) {
        setState(() => _cloneStatus = 'The voice could not be created.');
      }
    } finally {
      if (mounted) setState(() => _cloning = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (_keyCheckFailed) ...[
          Text(
            'The macOS keychain did not answer, so saved keys could not be '
            'checked. Reopen Settings to try again.',
            key: const ValueKey('cloud-voice-check-failed'),
            style: TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.tinySize,
            ),
          ),
          const SizedBox(height: OrSpacing.x2),
        ],
        for (final provider in CloudVoiceProvider.values) ...[
          _providerCard(provider),
          const SizedBox(height: OrSpacing.x3),
        ],
        Text(
          'Keys are stored in the macOS keychain and never in a project or a '
          'preferences file.',
          style: TextStyle(
            color: OrColors.textMuted,
            fontSize: OrType.tinySize,
          ),
        ),
      ],
    );
  }

  Widget _providerCard(CloudVoiceProvider provider) {
    final configured = _configured[provider] ?? false;
    final busy = _busy[provider] ?? false;
    final status = _status[provider];
    return Container(
      key: ValueKey('cloud-voice-${provider.wireName}'),
      padding: const EdgeInsets.all(OrSpacing.x3),
      decoration: BoxDecoration(
        color: OrColors.surface,
        border: Border.all(color: OrColors.border),
        borderRadius: BorderRadius.circular(OrRadii.card),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  provider.label,
                  style: const TextStyle(
                    fontSize: OrType.metadataSize,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              Text(
                !_loaded
                    ? 'Checking…'
                    : configured
                    ? 'Configured'
                    : 'No key',
                key: ValueKey('cloud-voice-status-${provider.wireName}'),
                style: TextStyle(
                  color: configured ? OrColors.success : OrColors.textMuted,
                  fontSize: OrType.tinySize,
                ),
              ),
            ],
          ),
          const SizedBox(height: OrSpacing.x1),
          Text(
            provider.keyHint,
            style: TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.tinySize,
            ),
          ),
          if (provider == CloudVoiceProvider.azure) ...[
            const SizedBox(height: OrSpacing.x2),
            TextField(
              key: const ValueKey('cloud-voice-azure-region'),
              controller: _region,
              enabled: !busy,
              onSubmitted: (_) => _saveRegion(),
              decoration: const InputDecoration(
                labelText: 'Azure Speech region',
                hintText: 'southeastasia',
                isDense: true,
                border: OutlineInputBorder(),
              ),
            ),
            Align(
              alignment: Alignment.centerRight,
              child: TextButton(
                key: const ValueKey('cloud-voice-azure-save-region'),
                onPressed: busy ? null : _saveRegion,
                child: const Text('Save region'),
              ),
            ),
          ],
          const SizedBox(height: OrSpacing.x2),
          Row(
            children: [
              Expanded(
                child: TextField(
                  key: ValueKey('cloud-voice-key-${provider.wireName}'),
                  controller: _keys[provider],
                  enabled: !busy,
                  obscureText: true,
                  autocorrect: false,
                  enableSuggestions: false,
                  decoration: InputDecoration(
                    labelText: configured
                        ? 'Replace key'
                        : 'Paste ${provider.label} key',
                    isDense: true,
                    border: const OutlineInputBorder(),
                  ),
                ),
              ),
              const SizedBox(width: OrSpacing.x2),
              FilledButton(
                key: ValueKey('cloud-voice-save-${provider.wireName}'),
                onPressed: busy ? null : () => _saveKey(provider),
                child: const Text('Save'),
              ),
              const SizedBox(width: OrSpacing.x1),
              OutlinedButton(
                key: ValueKey('cloud-voice-test-${provider.wireName}'),
                onPressed: busy ? null : () => _testKey(provider),
                child: Text(busy ? 'Testing…' : 'Test'),
              ),
              if (configured) ...[
                const SizedBox(width: OrSpacing.x1),
                TextButton(
                  key: ValueKey('cloud-voice-remove-${provider.wireName}'),
                  onPressed: busy ? null : () => _removeKey(provider),
                  child: const Text('Remove'),
                ),
              ],
            ],
          ),
          if (status != null) ...[
            const SizedBox(height: OrSpacing.x2),
            Text(
              status,
              key: ValueKey('cloud-voice-message-${provider.wireName}'),
              style: TextStyle(
                color: status.contains('works') || status.contains('saved')
                    ? OrColors.success
                    : OrColors.textSecondary,
                fontSize: OrType.tinySize,
              ),
            ),
          ],
          if (provider == CloudVoiceProvider.elevenLabs) ...[
            const Divider(height: OrSpacing.x6),
            const Text(
              'A personal voice from a recording',
              style: TextStyle(
                fontSize: OrType.metadataSize,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: OrSpacing.x1),
            Text(
              'Record one take with Record voice → Save as sample, then choose it '
              'here. ElevenLabs requires the account owner to consent to cloning.',
              style: TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.tinySize,
              ),
            ),
            const SizedBox(height: OrSpacing.x2),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    key: const ValueKey('cloud-voice-sample-path'),
                    controller: _samplePath,
                    enabled: !_cloning,
                    decoration: const InputDecoration(
                      labelText: 'WAV sample path',
                      isDense: true,
                      border: OutlineInputBorder(),
                    ),
                  ),
                ),
                const SizedBox(width: OrSpacing.x2),
                OutlinedButton(
                  key: const ValueKey('cloud-voice-sample-browse'),
                  onPressed: _cloning ? null : _browseSample,
                  child: const Text('Choose…'),
                ),
              ],
            ),
            const SizedBox(height: OrSpacing.x2),
            TextField(
              key: const ValueKey('cloud-voice-clone-name'),
              controller: _cloneName,
              enabled: !_cloning,
              decoration: const InputDecoration(
                labelText: 'Voice name',
                isDense: true,
                border: OutlineInputBorder(),
              ),
            ),
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Checkbox(
                  key: const ValueKey('cloud-voice-clone-consent'),
                  value: _consent,
                  onChanged: _cloning
                      ? null
                      : (value) => setState(() => _consent = value ?? false),
                ),
                const Expanded(
                  child: Padding(
                    padding: EdgeInsets.only(top: 12),
                    child: Text(
                      'I own this voice and consent to creating a cloned voice.',
                      style: TextStyle(fontSize: OrType.tinySize),
                    ),
                  ),
                ),
              ],
            ),
            Row(
              children: [
                FilledButton(
                  key: const ValueKey('cloud-voice-clone-create'),
                  onPressed: _cloning ? null : _cloneVoice,
                  child: Text(_cloning ? 'Creating…' : 'Create personal voice'),
                ),
                if (_cloneStatus != null) ...[
                  const SizedBox(width: OrSpacing.x2),
                  Expanded(
                    child: Text(
                      _cloneStatus!,
                      key: const ValueKey('cloud-voice-clone-status'),
                      style: TextStyle(
                        color: _cloneStatus!.startsWith('Created')
                            ? OrColors.success
                            : OrColors.textSecondary,
                        fontSize: OrType.tinySize,
                      ),
                    ),
                  ),
                ],
              ],
            ),
          ],
        ],
      ),
    );
  }
}
