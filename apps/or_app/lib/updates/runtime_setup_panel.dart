import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../production/local_tts_server_manager.dart';
import '../widgets/or_widgets.dart';
import 'github_release.dart';
import 'resource_center.dart';

/// One surface that installs and repairs everything the app does not bundle.
///
/// The three external pieces — the Whisper engine, a local speech server and
/// the assistant harness — used to need a terminal, and each one a different
/// recipe per operating system. This panel drives the same code on every host:
/// a pinned checksummed download, a folder the user already prepared, or a
/// binary they picked. Nothing is installed silently.
class RuntimeSetupPanel extends StatefulWidget {
  const RuntimeSetupPanel({
    super.key,
    this.service = const ResourceCenterService(),
    this.manager,
    this.assistantInstallUrl =
        'https://github.com/deepseek-ai/deepseek-harness',
  });

  final ResourceCenterService service;
  final LocalTtsServerManager? manager;
  final String assistantInstallUrl;

  @override
  State<RuntimeSetupPanel> createState() => _RuntimeSetupPanelState();
}

class _RuntimeSetupPanelState extends State<RuntimeSetupPanel> {
  late Future<RuntimeSnapshot> _runtime;
  LocalTtsServerManager get _manager =>
      widget.manager ?? LocalTtsServerManager.instance;

  String? _error;
  bool _installingSpeech = false;
  bool _removingSpeech = false;
  bool _canRemoveSpeech = false;
  int _received = 0;
  int _total = 0;
  LocalTtsEngine _engine = localTtsEngines.first;

  @override
  void initState() {
    super.initState();
    _runtime = widget.service.inspectRuntime();
    unawaited(_readRemovable());
  }

  void _refresh() {
    setState(() {
      _runtime = widget.service.inspectRuntime();
    });
    unawaited(_readRemovable());
  }

  Future<void> _readRemovable() async {
    final removable = await widget.service.hasSpeechInstallRecord();
    if (!mounted) return;
    setState(() => _canRemoveSpeech = removable);
  }

  Future<void> _installSpeechRuntime() async {
    if (_installingSpeech) return;
    setState(() {
      _installingSpeech = true;
      _error = null;
      _received = 0;
      _total = 0;
    });
    try {
      await widget.service.installSpeechRuntime(
        onProgress: (received, total) {
          if (!mounted) return;
          setState(() {
            _received = received;
            _total = total;
          });
        },
      );
      if (!mounted) return;
      _refresh();
    } on GithubReleaseException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } catch (_) {
      if (mounted) {
        setState(() => _error = 'The speech engine could not be installed.');
      }
    } finally {
      if (mounted) setState(() => _installingSpeech = false);
    }
  }

  Future<void> _addRuntimeFiles() async {
    try {
      final folder = await widget.service.chooseRuntimeDirectory();
      if (folder == null || folder.isEmpty) return;
      await widget.service.addSpeechRuntimeFiles(Directory(folder));
      if (!mounted) return;
      _refresh();
    } on GithubReleaseException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } catch (_) {
      if (mounted) {
        setState(() => _error = 'Those runtime files could not be copied.');
      }
    }
  }

  Future<void> _chooseHarness() async {
    try {
      final path = await widget.service.chooseHarnessExecutable();
      if (path == null || path.isEmpty) return;
      await widget.service.saveHarnessPath(path);
      if (!mounted) return;
      _refresh();
    } on GithubReleaseException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } catch (_) {
      if (mounted) setState(() => _error = 'That harness could not be saved.');
    }
  }

  Future<void> _openUrl(String url) async {
    try {
      await widget.service.openUrl(url);
    } on GithubReleaseException catch (error) {
      if (mounted) setState(() => _error = error.message);
    }
  }

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      key: const ValueKey('settings-external-tools'),
      title: 'External tools',
      trailing: const OrBadge('Optional'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text(
            'The editor, captions and exports work without these. Install one '
            'here instead of in a terminal; each step is shown before it runs.',
            style: TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.metadataSize,
            ),
          ),
          const SizedBox(height: OrSpacing.x3),
          FutureBuilder<RuntimeSnapshot>(
            future: _runtime,
            builder: (context, snapshot) =>
                _speechRuntimeSection(snapshot.data),
          ),
          const Divider(height: OrSpacing.x5),
          _harnessSection(),
          const Divider(height: OrSpacing.x5),
          _voiceServerSection(),
          if (_error != null) ...[
            const SizedBox(height: OrSpacing.x3),
            Text(
              _error!,
              key: const ValueKey('tools-error'),
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataCompactSize,
              ),
            ),
          ],
        ],
      ),
    );
  }

  Future<void> _removeSpeechRuntime() async {
    if (_removingSpeech) return;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Remove the speech engine?'),
        content: const Text(
          'Only the files this setup installed are removed. Captions and '
          'speech-to-text stop working until you install them again, and any '
          'voiceover recordings you already made stay in their projects.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const ValueKey('tools-speech-remove-confirm'),
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('Remove'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    setState(() {
      _removingSpeech = true;
      _error = null;
    });
    try {
      final removed = await widget.service.removeSpeechRuntime();
      if (!mounted) return;
      _refresh();
      OrSnackBar.show(
        context,
        removed == 1
            ? 'Removed 1 file the setup installed.'
            : 'Removed $removed files the setup installed.',
      );
    } on GithubReleaseException catch (error) {
      if (mounted) setState(() => _error = error.message);
    } catch (_) {
      if (mounted) {
        setState(
          () => _error =
              'The installed files could not be removed. '
              'Nothing else in the folder was touched.',
        );
      }
    } finally {
      if (mounted) setState(() => _removingSpeech = false);
    }
  }

  Widget _speechRuntimeSection(RuntimeSnapshot? runtime) {
    final ready = runtime?.speechRuntime ?? false;
    final detail = ready
        ? 'The Whisper engine and its adapter are installed.'
        : runtime != null && runtime.whisperEngine && !runtime.whisperProvider
        ? 'The Whisper engine is installed; the adapter that feeds captions '
              'is missing.'
        : runtime != null && !runtime.whisperEngine && runtime.whisperProvider
        ? 'The adapter is installed; the Whisper engine itself is missing.'
        : 'Not installed. Captions and speech-to-text need both files.';
    return _SetupRow(
      icon: Icons.record_voice_over_outlined,
      title: 'Speech runtime',
      detail: detail,
      action: Column(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          if (_installingSpeech) ...[
            SizedBox(
              width: 160,
              child: LinearProgressIndicator(
                value: _total > 0 ? _received / _total : null,
              ),
            ),
            const SizedBox(height: OrSpacing.x1),
            Text(
              _total > 0
                  ? '${(_received / (1024 * 1024)).toStringAsFixed(1)} of '
                        '${(_total / (1024 * 1024)).toStringAsFixed(1)} MiB'
                  : 'Starting…',
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataCompactSize,
              ),
            ),
          ] else
            Wrap(
              spacing: OrSpacing.x2,
              runSpacing: OrSpacing.x2,
              alignment: WrapAlignment.end,
              children: [
                if (!ready)
                  FilledButton.icon(
                    key: const ValueKey('tools-speech-install'),
                    onPressed: _installSpeechRuntime,
                    icon: const Icon(Icons.download_outlined, size: 16),
                    label: const Text('Install engine'),
                  ),
                if (ready && _canRemoveSpeech)
                  TextButton.icon(
                    key: const ValueKey('tools-speech-remove'),
                    onPressed: _removingSpeech ? null : _removeSpeechRuntime,
                    icon: _removingSpeech
                        ? const SizedBox(
                            width: 14,
                            height: 14,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.delete_outline, size: 16),
                    label: const Text('Remove copy'),
                  ),
                OutlinedButton.icon(
                  key: const ValueKey('tools-speech-add-files'),
                  onPressed: _addRuntimeFiles,
                  icon: const Icon(Icons.folder_open_outlined, size: 16),
                  label: const Text('Use files…'),
                ),
              ],
            ),
        ],
      ),
    );
  }

  Widget _harnessSection() {
    return FutureBuilder<RuntimeSnapshot>(
      future: _runtime,
      builder: (context, snapshot) {
        final path = snapshot.data?.assistantPath;
        return _SetupRow(
          icon: Icons.auto_awesome_outlined,
          title: 'Assistant harness',
          detail: path == null
              ? 'Not detected on this machine. The AI panel needs a harness '
                    'running over ACP.'
              : 'Detected at $path',
          action: Wrap(
            spacing: OrSpacing.x2,
            runSpacing: OrSpacing.x2,
            alignment: WrapAlignment.end,
            children: [
              OutlinedButton.icon(
                key: const ValueKey('tools-harness-detect'),
                onPressed: _refresh,
                icon: const Icon(Icons.refresh_outlined, size: 16),
                label: const Text('Detect again'),
              ),
              OutlinedButton.icon(
                key: const ValueKey('tools-harness-choose'),
                onPressed: _chooseHarness,
                icon: const Icon(Icons.terminal_outlined, size: 16),
                label: const Text('Choose…'),
              ),
              OutlinedButton.icon(
                key: const ValueKey('tools-harness-get'),
                onPressed: () => _openUrl(widget.assistantInstallUrl),
                icon: const Icon(Icons.open_in_new_outlined, size: 16),
                label: const Text('Get harness'),
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _voiceServerSection() {
    return AnimatedBuilder(
      animation: _manager,
      builder: (context, _) {
        final manager = _manager;
        final engine = _engine;
        final running = manager.running;
        final busy = manager.busy;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Icon(
                  Icons.graphic_eq_outlined,
                  size: 18,
                  color: OrColors.textSecondary,
                ),
                const SizedBox(width: OrSpacing.x3),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text(
                        'Local speech server',
                        style: TextStyle(
                          fontSize: OrType.metadataSize,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        engine.description,
                        style: const TextStyle(
                          color: OrColors.textSecondary,
                          fontSize: OrType.metadataCompactSize,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: OrSpacing.x2),
            DropdownButtonFormField<String>(
              key: const ValueKey('tools-tts-engine'),
              initialValue: engine.id,
              decoration: const InputDecoration(
                labelText: 'Engine',
                isDense: true,
              ),
              items: [
                for (final option in localTtsEngines)
                  DropdownMenuItem(value: option.id, child: Text(option.label)),
              ],
              onChanged: busy
                  ? null
                  : (value) {
                      if (value == null) return;
                      setState(() {
                        _engine = localTtsEngines.firstWhere(
                          (option) => option.id == value,
                        );
                      });
                      manager.selectEngine(value);
                    },
            ),
            const SizedBox(height: OrSpacing.x2),
            Wrap(
              spacing: OrSpacing.x2,
              runSpacing: OrSpacing.x2,
              children: [
                if (engine.installable)
                  OutlinedButton.icon(
                    key: const ValueKey('tools-tts-install'),
                    onPressed: busy || running
                        ? null
                        : () => manager.install(engine),
                    icon: const Icon(Icons.download_outlined, size: 16),
                    label: const Text('Install engine'),
                  ),
                FilledButton.icon(
                  key: const ValueKey('tools-tts-start'),
                  onPressed: busy || running
                      ? null
                      : () => manager.start(engine),
                  icon: const Icon(Icons.play_arrow_outlined, size: 16),
                  label: const Text('Start server'),
                ),
                OutlinedButton.icon(
                  key: const ValueKey('tools-tts-stop'),
                  onPressed: running ? manager.stop : null,
                  icon: const Icon(Icons.stop_outlined, size: 16),
                  label: const Text('Stop'),
                ),
              ],
            ),
            if (manager.error != null) ...[
              const SizedBox(height: OrSpacing.x2),
              Text(
                manager.error!,
                key: const ValueKey('tools-tts-error'),
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataCompactSize,
                ),
              ),
            ],
            if (manager.log.isNotEmpty) ...[
              const SizedBox(height: OrSpacing.x2),
              Container(
                key: const ValueKey('tools-tts-log'),
                constraints: const BoxConstraints(maxHeight: 160),
                padding: const EdgeInsets.all(OrSpacing.x2),
                decoration: BoxDecoration(
                  border: Border.all(color: OrColors.border),
                  borderRadius: BorderRadius.circular(OrRadii.small),
                ),
                child: SingleChildScrollView(
                  child: SelectableText(
                    manager.logTail,
                    style: const TextStyle(
                      fontFamily: 'monospace',
                      fontSize: OrType.metadataCompactSize,
                      color: OrColors.textSecondary,
                    ),
                  ),
                ),
              ),
            ],
          ],
        );
      },
    );
  }
}

class _SetupRow extends StatelessWidget {
  const _SetupRow({
    required this.icon,
    required this.title,
    required this.detail,
    this.action,
  });

  final IconData icon;
  final String title;
  final String detail;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: OrSpacing.x2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(icon, size: 18, color: OrColors.textSecondary),
          const SizedBox(width: OrSpacing.x3),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: const TextStyle(
                    fontSize: OrType.metadataSize,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  detail,
                  style: const TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: OrType.metadataCompactSize,
                  ),
                ),
              ],
            ),
          ),
          if (action != null) ...[const SizedBox(width: OrSpacing.x3), action!],
        ],
      ),
    );
  }
}
