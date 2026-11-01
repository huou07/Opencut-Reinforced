import 'dart:io';

import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../widgets/or_widgets.dart';
import 'github_release.dart';
import 'resource_center.dart';

/// Settings surface for clean-machine setup, optional downloads and updates.
class ResourceCenterPanel extends StatefulWidget {
  const ResourceCenterPanel({
    super.key,
    required this.currentVersion,
    this.onManageSpeechModels,
    this.service = const ResourceCenterService(),
    this.assistantInstallUrl =
        'https://github.com/deepseek-ai/deepseek-harness',
  });

  final String currentVersion;
  final VoidCallback? onManageSpeechModels;
  final ResourceCenterService service;
  final String assistantInstallUrl;

  @override
  State<ResourceCenterPanel> createState() => _ResourceCenterPanelState();
}

class _ResourceCenterPanelState extends State<ResourceCenterPanel> {
  late Future<RuntimeSnapshot> _runtime;
  GithubRelease? _release;
  String? _updateError;
  bool _checking = false;
  bool _downloading = false;
  int _received = 0;
  int _total = 0;
  File? _downloaded;
  String? _actionError;

  @override
  void initState() {
    super.initState();
    _runtime = widget.service.inspectRuntime();
  }

  bool get _hasNewerRelease =>
      _release != null &&
      isNewerRelease(_release!.tagName, widget.currentVersion);

  bool get _isNewerThanPublic =>
      _release != null &&
      compareVersions(widget.currentVersion, _release!.tagName) > 0;

  Future<void> _checkForUpdates() async {
    setState(() {
      _checking = true;
      _updateError = null;
      _actionError = null;
      _downloaded = null;
    });
    try {
      final release = await widget.service.checkForUpdate(
        widget.currentVersion,
      );
      if (!mounted) return;
      setState(() => _release = release);
    } on GithubReleaseException catch (error) {
      if (!mounted) return;
      setState(() => _updateError = error.message);
    } catch (_) {
      if (!mounted) return;
      setState(() => _updateError = 'The update check could not be completed.');
    } finally {
      if (mounted) setState(() => _checking = false);
    }
  }

  Future<void> _downloadUpdate() async {
    final release = _release;
    if (release == null || _downloading) return;
    setState(() {
      _downloading = true;
      _received = 0;
      _total = release.macDmg?.sizeBytes ?? 0;
      _actionError = null;
      _downloaded = null;
    });
    try {
      final directory = await widget.service.downloadDirectory();
      final file = await widget.service.downloadUpdate(
        release,
        directory: directory,
        onProgress: (received, total) {
          if (!mounted) return;
          setState(() {
            _received = received;
            _total = total;
          });
        },
      );
      if (!mounted) return;
      setState(() => _downloaded = file);
    } on GithubReleaseException catch (error) {
      if (!mounted) return;
      setState(() => _actionError = error.message);
    } catch (_) {
      if (!mounted) return;
      setState(() => _actionError = 'The update could not be downloaded.');
    } finally {
      if (mounted) setState(() => _downloading = false);
    }
  }

  Future<void> _openDownloaded() async {
    final file = _downloaded;
    if (file == null) return;
    try {
      await widget.service.openFile(file);
    } on GithubReleaseException catch (error) {
      if (mounted) setState(() => _actionError = error.message);
    }
  }

  Future<void> _openUrl(String url) async {
    try {
      await widget.service.openUrl(url);
    } on GithubReleaseException catch (error) {
      if (mounted) setState(() => _actionError = error.message);
    }
  }

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      key: const ValueKey('settings-resources'),
      title: 'Resources & updates',
      trailing: OrBadge(_hasNewerRelease ? 'Update available' : 'Clean setup'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text(
            'Everything the editor needs to play, caption and export is '
            'bundled with this app. Optional downloads and new public builds '
            'are available below; nothing is installed silently.',
            style: TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.metadataSize,
            ),
          ),
          const SizedBox(height: OrSpacing.x4),
          FutureBuilder<RuntimeSnapshot>(
            future: _runtime,
            builder: (context, snapshot) {
              if (snapshot.connectionState != ConnectionState.done) {
                return const Row(
                  children: [
                    SizedBox.square(
                      dimension: 14,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    ),
                    SizedBox(width: OrSpacing.x3),
                    Text('Checking bundled runtime…'),
                  ],
                );
              }
              final runtime = snapshot.data;
              if (runtime == null) {
                return const _StatusLine(
                  ok: false,
                  label: 'Bundled runtime could not be inspected.',
                );
              }
              return _RuntimeSummary(runtime: runtime);
            },
          ),
          const Divider(height: OrSpacing.x6),
          _ResourceRow(
            icon: Icons.record_voice_over_outlined,
            title: 'Speech models',
            detail: widget.onManageSpeechModels == null
                ? 'Open a project, then use Audio → Speech model to download '
                      'or switch a local Whisper model. Models stay on this Mac.'
                : 'Download, switch or remove a local Whisper model. Models '
                      'stay on this Mac and audio is processed locally.',
            action: widget.onManageSpeechModels == null
                ? null
                : OutlinedButton.icon(
                    key: const ValueKey('resources-manage-speech'),
                    onPressed: widget.onManageSpeechModels,
                    icon: const Icon(Icons.download_outlined, size: 16),
                    label: const Text('Manage models…'),
                  ),
          ),
          _ResourceRow(
            icon: Icons.auto_awesome_outlined,
            title: 'Assistant harness',
            detail:
                'The editor’s AI panel connects to a harness installed '
                'outside the app bundle. DeepSeek Harness is the default.',
            action: OutlinedButton.icon(
              key: const ValueKey('resources-install-assistant'),
              onPressed: () => _openUrl(widget.assistantInstallUrl),
              icon: const Icon(Icons.open_in_new_outlined, size: 16),
              label: const Text('Get harness'),
            ),
          ),
          const Divider(height: OrSpacing.x6),
          _updateBlock(),
          if (_actionError != null) ...[
            const SizedBox(height: OrSpacing.x3),
            Text(
              _actionError!,
              key: const ValueKey('resource-action-error'),
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

  Widget _updateBlock() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            const Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Application update',
                    style: TextStyle(
                      fontSize: OrType.metadataSize,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  SizedBox(height: 2),
                  Text(
                    'Checked from the public GitHub releases of '
                    'huou07/Opencut-Reinforced.',
                    style: TextStyle(
                      color: OrColors.textSecondary,
                      fontSize: OrType.metadataCompactSize,
                    ),
                  ),
                ],
              ),
            ),
            if (_checking)
              const SizedBox.square(
                dimension: 18,
                child: CircularProgressIndicator(strokeWidth: 2),
              )
            else
              OutlinedButton.icon(
                key: const ValueKey('resource-update-check'),
                onPressed: _checkForUpdates,
                icon: const Icon(Icons.refresh_outlined, size: 16),
                label: const Text('Check for updates'),
              ),
          ],
        ),
        if (_updateError != null) ...[
          const SizedBox(height: OrSpacing.x3),
          Text(
            _updateError!,
            key: const ValueKey('resource-update-error'),
            style: const TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.metadataCompactSize,
            ),
          ),
        ],
        if (_release != null) ...[
          const SizedBox(height: OrSpacing.x3),
          Text(
            _hasNewerRelease
                ? 'Version ${_release!.tagName} is available (this build is '
                      '${widget.currentVersion}).'
                : _isNewerThanPublic
                ? 'This build (${widget.currentVersion}) is newer than the '
                      'latest public release (${_release!.tagName}).'
                : 'This build is up to date with the latest public release '
                      '(${_release!.tagName}).',
            key: const ValueKey('resource-update-status'),
            style: const TextStyle(fontSize: OrType.metadataSize),
          ),
          const SizedBox(height: OrSpacing.x2),
          if (_downloading) ...[
            LinearProgressIndicator(
              value: _total > 0 ? _received / _total : null,
            ),
            const SizedBox(height: OrSpacing.x2),
            Text(
              'Downloading ${(_received / (1024 * 1024)).toStringAsFixed(1)} '
              'of ${(_total / (1024 * 1024)).toStringAsFixed(1)} MiB…',
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataCompactSize,
              ),
            ),
          ] else if (_downloaded == null)
            Align(
              alignment: Alignment.centerLeft,
              child: _hasNewerRelease
                  ? FilledButton.icon(
                      key: const ValueKey('resource-update-download'),
                      onPressed: _downloadUpdate,
                      icon: const Icon(Icons.download_outlined, size: 16),
                      label: const Text('Download update'),
                    )
                  : OutlinedButton.icon(
                      key: const ValueKey('resource-update-download'),
                      onPressed: _downloadUpdate,
                      icon: const Icon(Icons.download_outlined, size: 16),
                      label: const Text('Download installer'),
                    ),
            )
          else
            Align(
              alignment: Alignment.centerLeft,
              child: FilledButton.icon(
                key: const ValueKey('resource-update-open'),
                onPressed: _openDownloaded,
                icon: const Icon(Icons.open_in_new_outlined, size: 16),
                label: const Text('Open downloaded image'),
              ),
            ),
          if (_downloaded != null) ...[
            const SizedBox(height: OrSpacing.x2),
            Text(
              'Saved to ${_downloaded!.path}. Open it, then drag '
              'Opencut Reinforced into Applications to replace this build.',
              key: const ValueKey('resource-update-downloaded'),
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataCompactSize,
              ),
            ),
          ],
          const SizedBox(height: OrSpacing.x1),
          Align(
            alignment: Alignment.centerLeft,
            child: TextButton.icon(
              key: const ValueKey('resource-update-page'),
              onPressed: () => _openUrl(_release!.pageUrl),
              icon: const Icon(Icons.description_outlined, size: 16),
              label: const Text('Release notes and checksums'),
            ),
          ),
        ],
      ],
    );
  }
}

class _RuntimeSummary extends StatelessWidget {
  const _RuntimeSummary({required this.runtime});

  final RuntimeSnapshot runtime;

  @override
  Widget build(BuildContext context) {
    if (!runtime.packaged) {
      return const _StatusLine(
        ok: true,
        label:
            'Development build: the runtime is read from the build tree. '
            'Packaged releases carry FFmpeg, the speech runtime and the or CLI.',
      );
    }
    return Column(
      key: const ValueKey('resource-runtime-status'),
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _StatusLine(ok: runtime.ffmpeg, label: 'FFmpeg runtime and codecs'),
        _StatusLine(ok: runtime.speechRuntime, label: 'Local speech runtime'),
        _StatusLine(ok: runtime.orCli, label: 'Local or CLI'),
        _StatusLine(
          ok: runtime.assistantPath != null,
          label: runtime.assistantPath == null
              ? 'Assistant harness not detected'
              : 'Assistant harness detected',
        ),
        const SizedBox(height: OrSpacing.x2),
        Text(
          runtime.complete
              ? 'All required components are bundled.'
              : 'A required component is missing. Download and reinstall the '
                    'latest app image below to repair this installation.',
          style: const TextStyle(
            color: OrColors.textSecondary,
            fontSize: OrType.metadataCompactSize,
          ),
        ),
      ],
    );
  }
}

class _StatusLine extends StatelessWidget {
  const _StatusLine({required this.ok, required this.label});

  final bool ok;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const EdgeInsets.only(top: 2),
            child: Icon(
              ok ? Icons.check_circle_outline : Icons.error_outline,
              size: 14,
              color: ok ? OrColors.textSecondary : OrColors.text,
            ),
          ),
          const SizedBox(width: OrSpacing.x2),
          Expanded(
            child: Text(
              label,
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataCompactSize,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _ResourceRow extends StatelessWidget {
  const _ResourceRow({
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
