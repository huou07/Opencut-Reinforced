import 'dart:async';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
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
    text: widget.controller.settings.workspacePath.isEmpty
        ? widget.controller.workspace
        : widget.controller.settings.workspacePath,
  );
  late bool _autoApprove = widget.controller.settings.autoApprove;
  late String _reasoning = widget.controller.settings.reasoningValue;

  @override
  void dispose() {
    _harness.dispose();
    _orCli.dispose();
    _workspace.dispose();
    super.dispose();
  }

  Future<void> _detect() async {
    final current = widget.controller.settings;
    final harness = await resolveHarnessPath(
      settings: current.copyWith(harnessPath: ''),
    );
    final orCli = await resolveOrCliPath(
      settings: current.copyWith(orCliPath: ''),
    );
    if (!mounted) return;
    setState(() {
      if (harness != null) _harness.text = harness;
      if (orCli != null) _orCli.text = orCli;
    });
  }

  Future<void> _pickWorkspace() async {
    final path = await getDirectoryPath(
      confirmButtonText: 'Use workspace',
      initialDirectory: _workspace.text,
    );
    if (path == null || path.isEmpty) return;
    setState(() => _workspace.text = path);
  }

  Future<void> _save() async {
    final current = widget.controller.settings;
    final workspace = _workspace.text.trim();
    // Keep following the open project unless the user actually chose a
    // different folder; saving the prefill would silently pin this project's
    // directory forever.
    final workspacePath = workspace == widget.controller.workspace
        ? current.workspacePath
        : workspace;
    await widget.controller.updateSettings(
      current.copyWith(
        harnessPath: _harness.text.trim(),
        orCliPath: _orCli.text.trim(),
        workspacePath: workspacePath,
        reasoningValue: _reasoning,
        autoApprove: _autoApprove,
      ),
    );
    if (mounted) Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final reasoning = widget.controller.reasoningChoices;
    return AlertDialog(
      key: const ValueKey('assistant-settings-dialog'),
      backgroundColor: OrColors.surface,
      title: const Text('Assistant'),
      content: SizedBox(
        width: 460,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text(
                'The assistant runs DeepSeek Harness locally and edits the '
                'open project through its command path. It never receives '
                'provider credentials from this app.',
                style: TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                  height: 1.4,
                ),
              ),
              const SizedBox(height: OrSpacing.x4),
              _field(
                keyName: 'assistant-settings-harness',
                label: 'Harness executable',
                controller: _harness,
                hint: 'Auto-detected when empty',
              ),
              const SizedBox(height: OrSpacing.x3),
              _field(
                keyName: 'assistant-settings-or-cli',
                label: 'or CLI',
                controller: _orCli,
                hint: 'Auto-detected from the checkout or PATH',
              ),
              const SizedBox(height: OrSpacing.x3),
              Row(
                crossAxisAlignment: CrossAxisAlignment.end,
                children: [
                  Expanded(
                    child: _field(
                      keyName: 'assistant-settings-workspace',
                      label: 'Workspace',
                      controller: _workspace,
                      hint: 'Defaults to the project folder',
                    ),
                  ),
                  const SizedBox(width: OrSpacing.x2),
                  OutlinedButton(
                    onPressed: _pickWorkspace,
                    child: const Text('Choose'),
                  ),
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
                    onPressed: _detect,
                    child: const Text('Detect paths'),
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
