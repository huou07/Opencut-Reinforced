import 'dart:async';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../support/host_platform.dart';
import 'assistant_controller.dart';
import 'assistant_settings.dart';

Future<void> showAssistantSettingsDialog({
  required BuildContext context,
  required AssistantController controller,
}) async {
  await showDialog<void>(
    context: context,
    builder: (context) => AssistantSettingsDialog(controller: controller),
  );
}

class AssistantSettingsDialog extends StatefulWidget {
  const AssistantSettingsDialog({super.key, required this.controller});

  final AssistantController controller;

  @override
  State<AssistantSettingsDialog> createState() =>
      _AssistantSettingsDialogState();
}

class _AssistantSettingsDialogState extends State<AssistantSettingsDialog> {
  late final TextEditingController _harness = TextEditingController(
    text: widget.controller.settings.harnessPath,
  );
  late final TextEditingController _orCli = TextEditingController(
    text: widget.controller.settings.orCliPath,
  );
  late final TextEditingController _workspace = TextEditingController(
    text:
        widget.controller.settings.workspaceMode ==
            AssistantWorkspaceMode.custom
        ? widget.controller.settings.workspacePath
        : widget.controller.workspace,
  );
  late AssistantHarnessKind _harnessKind =
      widget.controller.settings.harnessKind;
  late AssistantWorkspaceMode _workspaceMode =
      widget.controller.settings.workspaceMode;
  late bool _autoApprove = widget.controller.settings.autoApprove;
  late String _reasoning = widget.controller.settings.reasoningValue;
  String _detection = '';
  bool _detecting = false;

  @override
  void dispose() {
    _harness.dispose();
    _orCli.dispose();
    _workspace.dispose();
    super.dispose();
  }

  Future<void> _downloadHarness() async {
    final url = switch (_harnessKind) {
      AssistantHarnessKind.claudeCode => 'https://claude.com/claude-code',
      AssistantHarnessKind.codex => 'https://github.com/openai/codex',
      AssistantHarnessKind.opencode => 'https://opencode.ai',
      AssistantHarnessKind.dsh ||
      AssistantHarnessKind.auto ||
      AssistantHarnessKind.custom =>
        'https://github.com/deepseek-ai/deepseek-harness',
    };
    try {
      await HostPlatform.current().openExternal(url);
    } catch (_) {
      // No browser or no default handler. Opening a download page is a
      // convenience, so a host that cannot do it must not break the dialog.
    }
  }

  Future<void> _detect() async {
    if (_detecting) return;
    setState(() => _detecting = true);
    final current = widget.controller.settings;
    // Detect for the harness the dropdown is showing, not for the preferred
    // family: choosing Codex and pressing Detect must look for Codex.
    final launch = await resolveHarnessLaunch(
      settings: current.copyWith(harnessKind: _harnessKind, harnessPath: ''),
      patchPath: '',
    );
    final orCli = await resolveOrCliPath(
      settings: current.copyWith(orCliPath: ''),
    );
    if (!mounted) return;
    setState(() {
      _detecting = false;
      if (launch != null) {
        _harness.text = launch.executable;
        if (_harnessKind == AssistantHarnessKind.auto) _harnessKind = launch.kind;
        _detection = 'Found ${launch.kind.label}.';
      } else if (_harnessKind == AssistantHarnessKind.auto) {
        _detection = 'No ACP harness was found. Set the executable below.';
      } else {
        _detection =
            '${_harnessKind.label} was not found on this machine'
            '${_installHint(_harnessKind)}.';
      }
      if (orCli != null) _orCli.text = orCli;
    });
  }

  String _installHint(AssistantHarnessKind kind) => switch (kind) {
    AssistantHarnessKind.codex =>
      '; install the adapter with npm i -g @zed-industries/codex-acp',
    AssistantHarnessKind.claudeCode =>
      '; install the adapter with npm i -g @zed-industries/claude-code-acp',
    AssistantHarnessKind.opencode => '; install it from opencode.ai',
    AssistantHarnessKind.dsh ||
    AssistantHarnessKind.auto ||
    AssistantHarnessKind.custom => '',
  };

  Future<void> _pickWorkspace() async {
    final path = await getDirectoryPath(
      confirmButtonText: 'Use workspace',
      initialDirectory: _workspace.text,
    );
    if (path == null || path.isEmpty) return;
    setState(() {
      _workspaceMode = AssistantWorkspaceMode.custom;
      _workspace.text = path;
    });
  }

  Future<void> _save() async {
    final current = widget.controller.settings;
    final customPath = _workspace.text.trim();
    final mode =
        _workspaceMode == AssistantWorkspaceMode.custom && customPath.isEmpty
        ? AssistantWorkspaceMode.agent
        : _workspaceMode;
    await widget.controller.updateSettings(
      current.copyWith(
        harnessKind: _harnessKind,
        harnessPath: _harness.text.trim(),
        orCliPath: _orCli.text.trim(),
        workspaceMode: mode,
        workspacePath: mode == AssistantWorkspaceMode.custom ? customPath : '',
        reasoningValue: _reasoning,
        autoApprove: _autoApprove,
      ),
    );
    if (mounted) Navigator.of(context).pop();
  }

  String _workspaceDescription() {
    switch (_workspaceMode) {
      case AssistantWorkspaceMode.agent:
        final path = widget.controller.agentWorkspacePath;
        return path.isEmpty
            ? 'The app installs its AGENTS.md, guides, skills and tools here on connect.'
            : 'AGENTS.md, guides, skills and tools installed by the app.\n$path';
      case AssistantWorkspaceMode.project:
        final path = widget.controller.projectPath;
        return path.isEmpty
            ? 'The open project folder. No project is open right now.'
            : 'The open project folder.\n$path';
      case AssistantWorkspaceMode.custom:
        return _workspace.text.trim().isEmpty
            ? 'Choose a folder for this session.'
            : _workspace.text.trim();
    }
  }

  @override
  Widget build(BuildContext context) {
    final reasoning = widget.controller.reasoningChoices;
    return AlertDialog(
      key: const ValueKey('assistant-settings-dialog'),
      backgroundColor: OrColors.surface,
      title: const Text('Assistant'),
      content: SizedBox(
        width: OrDialogSizes.width(context, 460),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text(
                'The assistant runs an ACP harness locally — DSH, Claude Code, '
                'Codex, OpenCode or your own command — and edits the open '
                'project through its command path. It never receives provider '
                'credentials from this app.',
                style: TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                  height: 1.4,
                ),
              ),
              const SizedBox(height: OrSpacing.x4),
              DropdownButtonFormField<AssistantHarnessKind>(
                key: const ValueKey('assistant-settings-harness-kind'),
                initialValue: _harnessKind,
                isExpanded: true,
                dropdownColor: OrColors.surface,
                decoration: const InputDecoration(
                  labelText: 'Harness',
                  isDense: true,
                ),
                items: [
                  for (final kind in AssistantHarnessKind.values)
                    DropdownMenuItem(value: kind, child: Text(kind.label)),
                ],
                onChanged: (value) {
                  if (value == null) return;
                  setState(() {
                    _harnessKind = value;
                    // A path belongs to the harness it was detected for; a new
                    // selection starts from discovery again.
                    _harness.clear();
                    _detection = '';
                  });
                  _detect();
                },
              ),
              const SizedBox(height: OrSpacing.x2),
              _field(
                keyName: 'assistant-settings-harness',
                label: 'Harness executable',
                controller: _harness,
                hint: 'Found automatically when empty',
              ),
              if (_detection.isNotEmpty) ...[
                const SizedBox(height: OrSpacing.x1),
                Text(
                  _detection,
                  key: const ValueKey('assistant-settings-detection'),
                  style: const TextStyle(
                    color: OrColors.textMuted,
                    fontSize: OrType.tinySize,
                  ),
                ),
              ],
              const SizedBox(height: OrSpacing.x3),
              _field(
                keyName: 'assistant-settings-or-cli',
                label: 'or CLI',
                controller: _orCli,
                hint: 'Auto-detected from the checkout or PATH',
              ),
              const SizedBox(height: OrSpacing.x3),
              DropdownButtonFormField<AssistantWorkspaceMode>(
                key: const ValueKey('assistant-settings-workspace-mode'),
                initialValue: _workspaceMode,
                isExpanded: true,
                dropdownColor: OrColors.surface,
                decoration: const InputDecoration(
                  labelText: 'Workspace',
                  isDense: true,
                ),
                items: const [
                  DropdownMenuItem(
                    value: AssistantWorkspaceMode.agent,
                    child: Text('Agent workspace'),
                  ),
                  DropdownMenuItem(
                    value: AssistantWorkspaceMode.project,
                    child: Text('Project folder'),
                  ),
                  DropdownMenuItem(
                    value: AssistantWorkspaceMode.custom,
                    child: Text('Choose a folder'),
                  ),
                ],
                onChanged: (value) =>
                    setState(() => _workspaceMode = value ?? _workspaceMode),
              ),
              const SizedBox(height: OrSpacing.x2),
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: Text(
                      _workspaceDescription(),
                      style: const TextStyle(
                        color: OrColors.textMuted,
                        fontSize: OrType.tinySize,
                        height: 1.4,
                      ),
                    ),
                  ),
                  if (_workspaceMode == AssistantWorkspaceMode.custom) ...[
                    const SizedBox(width: OrSpacing.x2),
                    OutlinedButton(
                      key: const ValueKey(
                        'assistant-settings-workspace-choose',
                      ),
                      onPressed: _pickWorkspace,
                      child: const Text('Choose'),
                    ),
                  ],
                ],
              ),
              const SizedBox(height: OrSpacing.x3),
              if (reasoning.length > 1)
                DropdownButtonFormField<String>(
                  key: const ValueKey('assistant-settings-reasoning'),
                  initialValue:
                      reasoning.any((choice) => choice.value == _reasoning)
                      ? _reasoning
                      : null,
                  isExpanded: true,
                  dropdownColor: OrColors.surface,
                  decoration: const InputDecoration(
                    labelText: 'Reasoning effort',
                    isDense: true,
                  ),
                  items: [
                    for (final choice in reasoning)
                      DropdownMenuItem(
                        value: choice.value,
                        child: Text(
                          choice.name,
                          style: const TextStyle(fontSize: OrType.tinySize),
                        ),
                      ),
                  ],
                  onChanged: (value) =>
                      setState(() => _reasoning = value ?? ''),
                ),
              if (reasoning.length > 1) const SizedBox(height: OrSpacing.x2),
              SwitchListTile(
                key: const ValueKey('assistant-settings-auto-approve'),
                contentPadding: EdgeInsets.zero,
                value: _autoApprove,
                onChanged: (value) => setState(() => _autoApprove = value),
                title: const Text(
                  'Allow tool calls without asking',
                  style: TextStyle(
                    color: OrColors.text,
                    fontSize: OrType.metadataSize,
                  ),
                ),
                subtitle: const Text(
                  'File edits and commands run without a permission prompt. '
                  'Prefer off unless you trust the request.',
                  style: TextStyle(
                    color: OrColors.textMuted,
                    fontSize: OrType.tinySize,
                    height: 1.35,
                  ),
                ),
              ),
              const SizedBox(height: OrSpacing.x2),
              Row(
                children: [
                  const Expanded(
                    child: Text(
                      'Changes to the workspace start a new conversation.',
                      style: TextStyle(
                        color: OrColors.textMuted,
                        fontSize: OrType.tinySize,
                      ),
                    ),
                  ),
                  TextButton(
                    onPressed: _downloadHarness,
                    child: const Text('Get a harness'),
                  ),
                  TextButton(
                    key: const ValueKey('assistant-settings-detect'),
                    onPressed: _detecting ? null : _detect,
                    child: Text(_detecting ? 'Detecting…' : 'Detect paths'),
                  ),
                ],
              ),
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
          key: const ValueKey('assistant-settings-save'),
          onPressed: _save,
          child: const Text('Save'),
        ),
      ],
    );
  }

  Widget _field({
    required String keyName,
    required String label,
    required TextEditingController controller,
    required String hint,
  }) {
    return TextField(
      key: ValueKey(keyName),
      controller: controller,
      style: const TextStyle(
        color: OrColors.text,
        fontSize: OrType.metadataSize,
      ),
      decoration: InputDecoration(
        labelText: label,
        hintText: hint,
        isDense: true,
      ),
    );
  }
}
