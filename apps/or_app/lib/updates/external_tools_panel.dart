import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:or_app_bridge/or_app_bridge.dart' as rust;

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../widgets/or_widgets.dart';
import 'companion_setup.dart';
import 'resource_center.dart';

/// One external program as this screen shows it.
///
/// The panel maps the bridge view into this small value type so a widget test
/// can describe a machine without a Rust process, and so a screen never reads
/// a generated class directly.
class CompanionTool {
  const CompanionTool({
    required this.id,
    required this.name,
    required this.category,
    required this.summary,
    required this.benefit,
    required this.license,
    required this.licenseNote,
    required this.homepage,
    required this.installPage,
    required this.installCommand,
    required this.installNote,
    required this.sizeHint,
    required this.runtime,
    required this.outputs,
    required this.projectScoped,
    required this.removeHint,
    required this.updateHint,
    required this.state,
    required this.version,
    required this.detail,
    required this.launchable,
  });

  factory CompanionTool.fromBridge(rust.ExternalToolView view) {
    return CompanionTool(
      id: view.id,
      name: view.name,
      category: view.category,
      summary: view.summary,
      benefit: view.benefit,
      license: view.license,
      licenseNote: view.licenseNote,
      homepage: view.homepage,
      installPage: view.installPage,
      installCommand: view.installCommand,
      installNote: view.installNote,
      sizeHint: view.sizeHint,
      runtime: view.runtime,
      outputs: view.outputs,
      projectScoped: view.projectScoped,
      removeHint: view.removeHint,
      updateHint: view.updateHint,
      state: view.state,
      version: view.version,
      detail: view.detail,
      launchable: view.launchable,
    );
  }

  final String id;
  final String name;
  final String category;
  final String summary;
  final String benefit;
  final String license;
  final String? licenseNote;
  final String homepage;
  final String installPage;
  final String? installCommand;
  final String? installNote;
  final String sizeHint;
  final String runtime;
  final List<String> outputs;
  final bool projectScoped;
  final String removeHint;
  final String updateHint;

  /// `ready`, `missing`, `failed` or `unsupported`.
  final String state;
  final String? version;
  final String detail;
  final bool launchable;

  bool get ready => state == 'ready';
}

/// Loads the catalog and its probe results.
typedef CompanionToolLoader = Future<List<CompanionTool>> Function();

/// Re-probes one tool.
typedef CompanionToolChecker = Future<CompanionTool> Function(String id);

/// Opens a companion window and returns what happened.
typedef CompanionToolLauncher = Future<String> Function(String id);

/// Opens an https page in the system browser.
typedef CompanionPageOpener = Future<void> Function(String url);

Future<List<CompanionTool>> loadCompanionTools() async {
  final tools = await rust.listExternalTools();
  return tools.map(CompanionTool.fromBridge).toList();
}

Future<CompanionTool> checkCompanionTool(String id) async {
  return CompanionTool.fromBridge(await rust.checkExternalTool(id: id));
}

Future<String> launchCompanionTool(String id) async {
  final result = await rust.launchExternalTool(id: id);
  return result.detail;
}

/// Settings surface for programs OR uses but does not own.
///
/// It answers four questions per tool before a person commits to anything:
/// what it is for, what it costs in space and licence, whether this machine
/// can actually run it, and how to remove it. Installing is never automatic:
/// the exact command and the official page are shown, and the person chooses.
class ExternalToolsPanel extends StatefulWidget {
  const ExternalToolsPanel({
    super.key,
    this.loader = loadCompanionTools,
    this.checker = checkCompanionTool,
    this.launcher = launchCompanionTool,
    this.opener,
  });

  final CompanionToolLoader loader;
  final CompanionToolChecker checker;
  final CompanionToolLauncher launcher;
  final CompanionPageOpener? opener;

  @override
  State<ExternalToolsPanel> createState() => _ExternalToolsPanelState();
}

class _ExternalToolsPanelState extends State<ExternalToolsPanel> {
  late Future<List<CompanionTool>> _tools;
  final Set<String> _checking = <String>{};
  final Set<String> _settingUp = <String>{};
  final Map<String, List<String>> _setupOutput = <String, List<String>>{};
  String? _expanded;
  String? _error;

  @override
  void initState() {
    super.initState();
    _tools = widget.loader();
  }

  void _reload() {
    setState(() {
      _error = null;
      _tools = widget.loader();
    });
  }

  Future<void> _check(CompanionTool tool) async {
    if (_checking.contains(tool.id)) return;
    setState(() => _checking.add(tool.id));
    try {
      final updated = await widget.checker(tool.id);
      if (!mounted) return;
      setState(() {
        _tools = _replace(updated);
      });
    } catch (error) {
      if (mounted) {
        setState(() => _error = 'That check could not finish: $error');
      }
    } finally {
      if (mounted) setState(() => _checking.remove(tool.id));
    }
  }

  /// Runs the tool's own setup (or removal) command in the app.
  ///
  /// The command is the one the panel shows, so what runs here and what the
  /// instructions say cannot drift apart. Output is streamed into the row and
  /// a successful run re-checks the tool, so the state on screen is the state
  /// the probe found rather than an assumption that it worked.
  Future<void> _runSetup(CompanionTool tool, {required bool remove}) async {
    if (_settingUp.contains(tool.id)) return;
    final command = remove
        ? CompanionSetup.removalFor(tool.installCommand)
        : tool.installCommand;
    if (command == null) return;
    setState(() {
      _settingUp.add(tool.id);
      _setupOutput[tool.id] = <String>[];
    });
    final result = await const CompanionSetup().run(
      command,
      onLine: (line) {
        if (!mounted) return;
        setState(() => (_setupOutput[tool.id] ??= <String>[]).add(line));
      },
    );
    if (!mounted) return;
    setState(() => _settingUp.remove(tool.id));
    _notify(result.summary);
    if (result.succeeded) await _check(tool);
  }

  Future<List<CompanionTool>> _replace(CompanionTool updated) async {
    final current = await _tools;
    return [
      for (final tool in current)
        if (tool.id == updated.id) updated else tool,
    ];
  }

  Future<void> _launch(CompanionTool tool) async {
    try {
      final detail = await widget.launcher(tool.id);
      if (!mounted) return;
      _notify(detail);
    } catch (error) {
      if (mounted) _notify('$error');
    }
  }

  Future<void> _openPage(String url) async {
    try {
      final opener = widget.opener;
      if (opener != null) {
        await opener(url);
      } else {
        await const ResourceCenterService().openUrl(url);
      }
    } catch (error) {
      if (mounted) _notify('The page could not be opened: $error');
    }
  }

  Future<void> _copy(String text, String what) async {
    await Clipboard.setData(ClipboardData(text: text));
    if (mounted) _notify('$what copied.');
  }

  void _notify(String message) {
    if (!mounted) return;
    OrSnackBar.show(context, message);
  }

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      key: const ValueKey('settings-companion-tools'),
      title: 'Companion tools',
      trailing: const OrBadge('Optional'),
      child: FutureBuilder<List<CompanionTool>>(
        future: _tools,
        builder: (context, snapshot) {
          if (snapshot.connectionState != ConnectionState.done) {
            return const _Loading();
          }
          if (snapshot.hasError) {
            return _Failure(message: '${snapshot.error}', onRetry: _reload);
          }
          final tools = snapshot.data ?? const <CompanionTool>[];
          if (tools.isEmpty) {
            return const Text(
              'This build has no companion catalog.',
              style: TextStyle(color: OrColors.textMuted),
            );
          }
          final ready = tools.where((tool) => tool.ready).length;
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                'OR runs these as separate programs: it calls them, it does not '
                'copy them in. Nothing is downloaded or installed from this '
                'screen. Each row shows what the tool is for, its licence and '
                'the exact command, so you decide.',
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.metadataSize,
                  height: 1.4,
                ),
              ),
              const SizedBox(height: OrSpacing.x3),
              Row(
                children: [
                  Text(
                    '$ready of ${tools.length} ready on this machine',
                    key: const ValueKey('companion-tools-summary'),
                    style: const TextStyle(
                      color: OrColors.textMuted,
                      fontSize: OrType.metadataCompactSize,
                    ),
                  ),
                  const Spacer(),
                  TextButton.icon(
                    key: const ValueKey('companion-tools-refresh'),
                    onPressed: _reload,
                    icon: const Icon(Icons.refresh, size: 16),
                    label: const Text('Check all'),
                  ),
                ],
              ),
              if (_error != null) ...[
                const SizedBox(height: OrSpacing.x2),
                Text(
                  _error!,
                  key: const ValueKey('companion-tools-error'),
                  style: const TextStyle(
                    color: OrColors.warning,
                    fontSize: OrType.metadataCompactSize,
                  ),
                ),
              ],
              const SizedBox(height: OrSpacing.x2),
              for (final tool in tools) ...[
                _ToolRow(
                  tool: tool,
                  expanded: _expanded == tool.id,
                  checking: _checking.contains(tool.id),
                  settingUp: _settingUp.contains(tool.id),
                  setupOutput: _setupOutput[tool.id] ?? const <String>[],
                  onSetup: tool.installCommand == null
                      ? null
                      : () => _runSetup(tool, remove: false),
                  onRemove: CompanionSetup.removalFor(tool.installCommand) == null
                      ? null
                      : () => _runSetup(tool, remove: true),
                  onToggle: () => setState(
                    () => _expanded = _expanded == tool.id ? null : tool.id,
                  ),
                  onCheck: () => _check(tool),
                  onLaunch: () => _launch(tool),
                  onInstall: () => _openPage(tool.installPage),
                  onHome: () => _openPage(tool.homepage),
                  onCopyInstall: tool.installCommand == null
                      ? null
                      : () => _copy(tool.installCommand!, 'Install command'),
                  onCopyRemove: () => _copy(tool.removeHint, 'Removal note'),
                ),
                if (tool != tools.last)
                  const Divider(height: 1, color: OrColors.border),
              ],
            ],
          );
        },
      ),
    );
  }
}

class _Loading extends StatelessWidget {
  const _Loading();

  @override
  Widget build(BuildContext context) {
    return const Row(
      children: [
        SizedBox.square(
          dimension: 16,
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
        SizedBox(width: OrSpacing.x3),
        Text('Checking what this machine can run…'),
      ],
    );
  }
}

class _Failure extends StatelessWidget {
  const _Failure({required this.message, required this.onRetry});

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'The companion catalog could not be read.',
          style: const TextStyle(color: OrColors.text),
        ),
        const SizedBox(height: OrSpacing.x1),
        Text(
          message,
          style: const TextStyle(
            color: OrColors.textMuted,
            fontSize: OrType.metadataCompactSize,
          ),
        ),
        const SizedBox(height: OrSpacing.x2),
        OutlinedButton(onPressed: onRetry, child: const Text('Try again')),
      ],
    );
  }
}

class _ToolRow extends StatelessWidget {
  const _ToolRow({
    required this.tool,
    required this.expanded,
    required this.checking,
    required this.settingUp,
    required this.setupOutput,
    required this.onSetup,
    required this.onRemove,
    required this.onToggle,
    required this.onCheck,
    required this.onLaunch,
    required this.onInstall,
    required this.onHome,
    required this.onCopyInstall,
    required this.onCopyRemove,
  });

  final CompanionTool tool;
  final bool expanded;
  final bool checking;
  final bool settingUp;
  final List<String> setupOutput;
  final VoidCallback? onSetup;
  final VoidCallback? onRemove;
  final VoidCallback onToggle;
  final VoidCallback onCheck;
  final VoidCallback onLaunch;
  final VoidCallback onInstall;
  final VoidCallback onHome;
  final VoidCallback? onCopyInstall;
  final VoidCallback onCopyRemove;

  Color get _stateColor => switch (tool.state) {
    'ready' => OrColors.success,
    'failed' => OrColors.warning,
    _ => OrColors.textMuted,
  };

  String get _stateLabel => switch (tool.state) {
    'ready' => tool.version == null ? 'Ready' : 'Ready · ${tool.version}',
    'failed' => 'Needs attention',
    'unsupported' => 'Not on this platform',
    'project_scoped' => 'Installed per project',
    _ => 'Not installed',
  };

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        InkWell(
          key: ValueKey('companion-tool-${tool.id}'),
          onTap: onToggle,
          borderRadius: BorderRadius.circular(OrRadii.control),
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 52),
            child: Padding(
              padding: const EdgeInsets.symmetric(
                horizontal: OrSpacing.x2,
                vertical: OrSpacing.x2,
              ),
              child: Row(
                children: [
                  Container(
                    width: 8,
                    height: 8,
                    decoration: BoxDecoration(
                      color: _stateColor,
                      shape: BoxShape.circle,
                    ),
                  ),
                  const SizedBox(width: OrSpacing.x3),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Flexible(
                              child: Text(
                                tool.name,
                                style: OrType.control,
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                            const SizedBox(width: OrSpacing.x2),
                            Text(
                              _stateLabel,
                              key: ValueKey('companion-tool-${tool.id}-state'),
                              style: TextStyle(
                                color: _stateColor,
                                fontSize: OrType.tinySize,
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 2),
                        Text(
                          tool.summary,
                          maxLines: expanded ? 3 : 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(
                            color: OrColors.textSecondary,
                            fontSize: OrType.metadataCompactSize,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: OrSpacing.x2),
                  if (checking)
                    const SizedBox.square(
                      dimension: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  else
                    IconButton(
                      key: ValueKey('companion-tool-${tool.id}-check'),
                      tooltip: 'Check again',
                      onPressed: onCheck,
                      icon: const Icon(Icons.refresh, size: 16),
                    ),
                  if (tool.launchable)
                    TextButton(
                      key: ValueKey('companion-tool-${tool.id}-open'),
                      onPressed: onLaunch,
                      child: const Text('Open'),
                    )
                  else if (!tool.ready)
                    TextButton(
                      key: ValueKey('companion-tool-${tool.id}-install'),
                      onPressed: onInstall,
                      child: const Text('Get it'),
                    ),
                  Icon(
                    expanded ? Icons.expand_less : Icons.expand_more,
                    size: 18,
                    color: OrColors.textMuted,
                  ),
                ],
              ),
            ),
          ),
        ),
        if (expanded) _details(context),
      ],
    );
  }

  Widget _details(BuildContext context) {
    return Container(
      margin: const EdgeInsets.only(left: OrSpacing.x5, bottom: OrSpacing.x3),
      padding: const EdgeInsets.all(OrSpacing.x3),
      decoration: BoxDecoration(
        color: OrColors.surface,
        borderRadius: BorderRadius.circular(OrRadii.card),
        border: Border.all(color: OrColors.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            tool.benefit,
            style: const TextStyle(
              color: OrColors.textSecondary,
              fontSize: OrType.metadataSize,
              height: 1.4,
            ),
          ),
          const SizedBox(height: OrSpacing.x3),
          Wrap(
            spacing: OrSpacing.x2,
            runSpacing: OrSpacing.x2,
            children: [
              OrBadge(tool.category),
              OrBadge('Licence: ${tool.license}'),
              OrBadge(tool.sizeHint),
              OrBadge(tool.runtime),
              if (tool.outputs.isNotEmpty)
                OrBadge('Gives back: ${tool.outputs.join(', ')}'),
            ],
          ),
          if (tool.licenseNote != null) ...[
            const SizedBox(height: OrSpacing.x2),
            Text(
              tool.licenseNote!,
              style: const TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.metadataCompactSize,
              ),
            ),
          ],
          if (tool.detail.isNotEmpty) ...[
            const SizedBox(height: OrSpacing.x3),
            Text(
              tool.detail,
              key: ValueKey('companion-tool-${tool.id}-detail'),
              style: const TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.metadataCompactSize,
              ),
            ),
          ],
          if (tool.installCommand != null) ...[
            const SizedBox(height: OrSpacing.x3),
            const Text('Install', style: OrType.control),
            const SizedBox(height: OrSpacing.x1),
            Row(
              children: [
                Expanded(
                  child: SelectableText(
                    tool.installCommand!,
                    style: const TextStyle(
                      fontFamily: 'monospace',
                      fontSize: OrType.metadataCompactSize,
                      color: OrColors.textSecondary,
                    ),
                  ),
                ),
                if (onCopyInstall != null)
                  IconButton(
                    tooltip: 'Copy install command',
                    onPressed: onCopyInstall,
                    icon: const Icon(Icons.copy, size: 16),
                  ),
              ],
            ),
            if (tool.installNote != null)
              Text(
                tool.installNote!,
                style: const TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                ),
              ),
          ],
          if (tool.projectScoped) ...[
            const SizedBox(height: OrSpacing.x2),
            const Text(
              'Installed per project, not for the whole machine.',
              style: TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.tinySize,
              ),
            ),
          ],
          const SizedBox(height: OrSpacing.x3),
          Row(
            children: [
              if (onSetup != null)
                FilledButton(
                  key: ValueKey('companion-tool-${tool.id}-setup'),
                  onPressed: settingUp ? null : onSetup,
                  child: Text(
                    tool.ready ? 'Run setup again' : 'Set up here',
                  ),
                ),
              if (onSetup != null) const SizedBox(width: OrSpacing.x2),
              if (settingUp)
                const SizedBox.square(
                  dimension: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              else if (onRemove != null)
                TextButton(
                  key: ValueKey('companion-tool-${tool.id}-remove'),
                  onPressed: onRemove,
                  child: const Text('Uninstall'),
                ),
            ],
          ),
          if (settingUp || setupOutput.isNotEmpty) ...[
            const SizedBox(height: OrSpacing.x2),
            Container(
              key: ValueKey('companion-tool-${tool.id}-setup-output'),
              width: double.infinity,
              padding: const EdgeInsets.all(OrSpacing.x2),
              constraints: const BoxConstraints(maxHeight: 120),
              decoration: BoxDecoration(
                color: OrColors.surface,
                borderRadius: BorderRadius.circular(OrRadii.control),
                border: Border.all(color: OrColors.border),
              ),
              child: SingleChildScrollView(
                child: SelectableText(
                  setupOutput.isEmpty
                      ? 'Running ${tool.installCommand ?? ''}…'
                      : setupOutput.join('\n'),
                  style: const TextStyle(
                    fontFamily: 'monospace',
                    fontSize: OrType.tinySize,
                    color: OrColors.textSecondary,
                  ),
                ),
              ),
            ),
          ],
          const SizedBox(height: OrSpacing.x2),
          Wrap(
            spacing: OrSpacing.x2,
            runSpacing: OrSpacing.x2,
            children: [
              OutlinedButton(
                key: ValueKey('companion-tool-${tool.id}-page'),
                onPressed: onInstall,
                child: const Text('Official download page'),
              ),
              OutlinedButton(
                key: ValueKey('companion-tool-${tool.id}-home'),
                onPressed: onHome,
                child: const Text('Project home'),
              ),
              OutlinedButton(
                onPressed: onCopyRemove,
                child: const Text('Copy removal steps'),
              ),
            ],
          ),
          const SizedBox(height: OrSpacing.x2),
          Text(
            'Update: ${tool.updateHint}',
            style: const TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.tinySize,
            ),
          ),
          Text(
            'Remove: ${tool.removeHint}',
            style: const TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.tinySize,
            ),
          ),
        ],
      ),
    );
  }
}
