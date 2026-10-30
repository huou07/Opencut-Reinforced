import 'dart:async';
import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import 'assistant_controller.dart';
import 'assistant_models.dart';
import 'assistant_settings.dart';
import 'assistant_settings_dialog.dart';

/// The right-hand assistant surface: connection state, workspace and model
/// choices, the transcript, permission prompts, and the composer.
class AssistantPanel extends StatefulWidget {
  const AssistantPanel({
    super.key,
    required this.controller,
    this.compact = false,
  });

  final AssistantController controller;
  final bool compact;

  @override
  State<AssistantPanel> createState() => _AssistantPanelState();
}

class _AssistantPanelState extends State<AssistantPanel> {
  final _input = TextEditingController();
  final _scroll = ScrollController();
  final _focus = FocusNode();
  bool _autoStarted = false;
  bool _followLatest = true;

  AssistantController get controller => widget.controller;

  @override
  void initState() {
    super.initState();
    controller.addListener(_onChanged);
    _input.addListener(_onInputChanged);
    _scroll.addListener(_onScroll);
    unawaited(controller.initialize());
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      if (_autoStarted) return;
      _autoStarted = true;
      if (controller.state == AssistantConnectionState.idle) {
        unawaited(controller.connect());
      }
    });
  }

  @override
  void dispose() {
    controller.removeListener(_onChanged);
    _input.dispose();
    _scroll.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _onChanged() {
    if (!mounted) return;
    final follow = _followLatest;
    setState(() {});
    if (follow) _scrollToEnd();
  }

  void _onScroll() {
    if (!_scroll.hasClients) return;
    final atBottom =
        _scroll.position.maxScrollExtent - _scroll.position.pixels < 48;
    if (atBottom != _followLatest) {
      setState(() => _followLatest = atBottom);
    }
  }

  void _onInputChanged() {
    if (mounted) setState(() {});
  }

  void _scrollToEnd() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !_scroll.hasClients || !_followLatest) return;
      final target = _scroll.position.maxScrollExtent;
      if ((target - _scroll.position.pixels).abs() < 1) return;
      _scroll.animateTo(
        target,
        duration: const Duration(milliseconds: 120),
        curve: Curves.easeOut,
      );
    });
  }

  Future<void> _send() async {
    final text = _input.text;
    if (text.trim().isEmpty || controller.busy) return;
    _input.clear();
    await controller.send(text);
  }

  Future<void> _pickWorkspace() async {
    final path = await getDirectoryPath(
      confirmButtonText: 'Use workspace',
      initialDirectory: controller.workspace,
    );
    if (path == null || path.isEmpty) return;
    await controller.changeWorkspace(path);
  }

  String _workspaceLabel() => switch (controller.workspaceMode) {
    AssistantWorkspaceMode.agent => 'Agent workspace',
    AssistantWorkspaceMode.project => 'Project folder',
    AssistantWorkspaceMode.custom => _basename(controller.workspace),
  };

  Future<void> _openSettings() async {
    await showAssistantSettingsDialog(context: context, controller: controller);
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      key: const ValueKey('assistant-panel'),
      color: OrColors.background,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _header(context),
          _contextRow(context),
          const Divider(height: 1),
          if (controller.pendingPermission != null)
            _PermissionCard(
              request: controller.pendingPermission!,
              onAnswer: (optionId) =>
                  unawaited(controller.answerPermission(optionId)),
            ),
          Expanded(
            child:
                controller.state == AssistantConnectionState.idle ||
                    controller.state == AssistantConnectionState.connecting
                ? _WaitingState(controller: controller)
                : controller.state == AssistantConnectionState.error &&
                      controller.entries.isEmpty
                ? _ErrorState(
                    controller: controller,
                    onRetry: () => unawaited(controller.connect()),
                    onSettings: _openSettings,
                  )
                : controller.entries.isEmpty
                ? _EmptyState(
                    onExample: (prompt) {
                      _input.text = prompt;
                      _focus.requestFocus();
                      setState(() {});
                    },
                  )
                : _transcript(context),
          ),
          _statusRow(context),
          _composer(context),
        ],
      ),
    );
  }

  Widget _header(BuildContext context) {
    final state = controller.state;
    final label = switch (state) {
      AssistantConnectionState.ready => 'Assistant',
      AssistantConnectionState.connecting => 'Connecting',
      AssistantConnectionState.error => 'Assistant unavailable',
      AssistantConnectionState.stopped => 'Assistant stopped',
      AssistantConnectionState.idle => 'Assistant',
    };
    return SizedBox(
      height: 38,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x2),
        child: Row(
          children: [
            Container(
              width: 6,
              height: 6,
              decoration: BoxDecoration(
                color: assistantStateColor(state),
                shape: BoxShape.circle,
              ),
            ),
            const SizedBox(width: OrSpacing.x2),
            Expanded(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: OrColors.text,
                  fontSize: OrType.metadataSize,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
            _HeaderAction(
              keyName: 'assistant-new-conversation',
              icon: Icons.add_comment_outlined,
              tooltip: 'New conversation',
              onPressed: () => unawaited(controller.newConversation()),
            ),
            _HeaderAction(
              keyName: 'assistant-reconnect',
              icon: Icons.refresh,
              tooltip: 'Reconnect',
              onPressed: state == AssistantConnectionState.connecting
                  ? null
                  : () => unawaited(controller.connect()),
            ),
            _HeaderAction(
              keyName: 'assistant-settings',
              icon: Icons.tune,
              tooltip: 'Assistant settings',
              onPressed: _openSettings,
            ),
          ],
        ),
      ),
    );
  }

  Widget _contextRow(BuildContext context) {
    final models = controller.modelChoices;
    final reasoning = controller.reasoningChoices;
    final selectedModel =
        models.any((choice) => choice.value == controller.selectedModel)
        ? controller.selectedModel
        : null;
    return Padding(
      padding: const EdgeInsets.fromLTRB(
        OrSpacing.x2,
        0,
        OrSpacing.x2,
        OrSpacing.x2,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _ContextButton(
            keyName: 'assistant-workspace',
            icon: Icons.folder_outlined,
            label: _workspaceLabel(),
            tooltip: '${controller.workspace}\nChange in Assistant settings',
            onPressed: _pickWorkspace,
          ),
          const SizedBox(height: OrSpacing.x1),
          Row(
            children: [
              Expanded(
                child: _Dropdown(
                  keyName: 'assistant-model',
                  icon: Icons.memory,
                  hint: 'Model',
                  value: selectedModel,
                  items: [
                    for (final choice in models)
                      DropdownMenuItem(
                        value: choice.value,
                        child: Text(
                          choice.name,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(fontSize: OrType.tinySize),
                        ),
                      ),
                  ],
                  onChanged: (value) {
                    if (value != null) unawaited(controller.setModel(value));
                  },
                ),
              ),
              if (reasoning.length > 1) ...[
                const SizedBox(width: OrSpacing.x1),
                Expanded(
                  child: _Dropdown(
                    keyName: 'assistant-reasoning',
                    icon: Icons.psychology_outlined,
                    hint: 'Reasoning',
                    value:
                        reasoning.any(
                          (choice) =>
                              choice.value == controller.selectedReasoning,
                        )
                        ? controller.selectedReasoning
                        : null,
                    items: [
                      for (final choice in reasoning)
                        DropdownMenuItem(
                          value: choice.value,
                          child: Text(
                            choice.name,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(fontSize: OrType.tinySize),
                          ),
                        ),
                    ],
                    onChanged: (value) {
                      if (value != null) {
                        unawaited(controller.setReasoning(value));
                      }
                    },
                  ),
                ),
              ],
            ],
          ),
        ],
      ),
    );
  }

  Widget _transcript(BuildContext context) {
    return ListView.builder(
      key: const ValueKey('assistant-transcript'),
      controller: _scroll,
      padding: const EdgeInsets.fromLTRB(
        OrSpacing.x3,
        OrSpacing.x2,
        OrSpacing.x3,
        OrSpacing.x2,
      ),
      itemCount: controller.entries.length,
      itemBuilder: (context, index) =>
          _EntryView(entry: controller.entries[index]),
    );
  }

  Widget _statusRow(BuildContext context) {
    final contextInfo = controller.context;
    final usage = controller.usageSize > 0
        ? '${_compactTokens(controller.usageUsed)} / '
              '${_compactTokens(controller.usageSize)} tokens'
        : '';
    final contextLabel = contextInfo.clipId == null
        ? 'Playhead ${contextInfo.playheadSeconds?.toStringAsFixed(2) ?? '0.00'} s'
        : 'Selected clip · ${contextInfo.trackKind ?? 'timeline'}';
    final error = controller.errorMessage;
    return Padding(
      padding: const EdgeInsets.fromLTRB(
        OrSpacing.x3,
        0,
        OrSpacing.x2,
        OrSpacing.x1,
      ),
      child: Row(
        children: [
          if (error != null &&
              controller.entries.isNotEmpty &&
              controller.state == AssistantConnectionState.error)
            Expanded(
              child: Text(
                error,
                key: const ValueKey('assistant-inline-error'),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: OrColors.danger,
                  fontSize: OrType.tinySize,
                ),
              ),
            )
          else
            Expanded(
              child: Row(
                children: [
                  Flexible(
                    child: Icon(
                      contextInfo.clipId == null
                          ? Icons.schedule
                          : Icons.movie_outlined,
                      size: 12,
                      color: OrColors.textMuted,
                    ),
                  ),
                  const SizedBox(width: OrSpacing.x1),
                  Expanded(
                    child: Text(
                      contextLabel,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        color: OrColors.textMuted,
                        fontSize: OrType.tinySize,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          if (contextInfo.clipId != null)
            _IconButton(
              keyName: 'assistant-clear-context',
              icon: Icons.close,
              tooltip: 'Clear selection context',
              onPressed: () => controller.setContext(
                AssistantEditorContext(
                  playheadSeconds: contextInfo.playheadSeconds,
                ),
              ),
            ),
          if (usage.isNotEmpty)
            Padding(
              padding: const EdgeInsets.only(left: OrSpacing.x2),
              child: Text(
                usage,
                key: const ValueKey('assistant-usage'),
                style: const TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                ),
              ),
            ),
        ],
      ),
    );
  }

  Widget _composer(BuildContext context) {
    return Container(
      decoration: const BoxDecoration(
        border: Border(top: BorderSide(color: OrColors.border)),
      ),
      padding: const EdgeInsets.fromLTRB(
        OrSpacing.x2,
        OrSpacing.x2,
        OrSpacing.x2,
        OrSpacing.x2,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Container(
            decoration: BoxDecoration(
              color: OrColors.surface,
              borderRadius: BorderRadius.circular(OrRadii.control),
              border: Border.all(color: OrColors.border),
            ),
            padding: const EdgeInsets.fromLTRB(
              OrSpacing.x2,
              OrSpacing.x1,
              OrSpacing.x1,
              OrSpacing.x1,
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Focus(
                  canRequestFocus: false,
                  skipTraversal: true,
                  onKeyEvent: (node, event) {
                    if (event is! KeyDownEvent) return KeyEventResult.ignored;
                    // An IME uses Enter to commit a composition (Vietnamese
                    // Telex/VNI input does this); sending here would split the
                    // word and dispatch a half-written prompt.
                    if (_input.value.composing.isValid) {
                      return KeyEventResult.ignored;
                    }
                    final isEnter =
                        event.logicalKey == LogicalKeyboardKey.enter;
                    final shift = HardwareKeyboard.instance.isShiftPressed;
                    if (isEnter && !shift) {
                      unawaited(_send());
                      return KeyEventResult.handled;
                    }
                    return KeyEventResult.ignored;
                  },
                  child: TextField(
                    key: const ValueKey('assistant-composer'),
                    controller: _input,
                    focusNode: _focus,
                    minLines: 1,
                    maxLines: widget.compact ? 4 : 6,
                    maxLength: 16000,
                    style: const TextStyle(
                      color: OrColors.text,
                      fontSize: OrType.metadataSize,
                      height: 1.4,
                    ),
                    decoration: const InputDecoration(
                      isDense: true,
                      border: InputBorder.none,
                      counterText: '',
                      hintText: 'Ask to edit this project…',
                      hintStyle: TextStyle(
                        color: OrColors.textMuted,
                        fontSize: OrType.metadataSize,
                      ),
                    ),
                  ),
                ),
                Row(
                  children: [
                    const Expanded(
                      child: Text(
                        'Edits go through the project’s command path',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          color: OrColors.textMuted,
                          fontSize: OrType.tinySize,
                        ),
                      ),
                    ),
                    if (controller.busy)
                      _IconButton(
                        keyName: 'assistant-stop',
                        icon: Icons.stop_circle_outlined,
                        tooltip: 'Stop',
                        onPressed: controller.cancel,
                      )
                    else
                      _SendButton(
                        enabled: _input.text.trim().isNotEmpty,
                        onPressed: _send,
                      ),
                  ],
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  String _basename(String path) {
    final parts = path
        .split(RegExp(r'[\\/]'))
        .where((part) => part.isNotEmpty)
        .toList();
    return parts.isEmpty ? path : parts.last;
  }
}

String _compactTokens(int value) {
  if (value >= 1000000) {
    return '${(value / 1000000).toStringAsFixed(1)}M';
  }
  if (value >= 1000) return '${(value / 1000).toStringAsFixed(1)}k';
  return '$value';
}

class _HeaderAction extends StatelessWidget {
  const _HeaderAction({
    required this.keyName,
    required this.icon,
    required this.tooltip,
    required this.onPressed,
  });

  final String keyName;
  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return IconButton(
      key: ValueKey(keyName),
      icon: Icon(icon, size: 16),
      color: OrColors.textSecondary,
      disabledColor: OrColors.textMuted,
      tooltip: tooltip,
      visualDensity: VisualDensity.compact,
      constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
      onPressed: onPressed,
    );
  }
}

class _IconButton extends StatelessWidget {
  const _IconButton({
    required this.keyName,
    required this.icon,
    required this.tooltip,
    required this.onPressed,
  });

  final String keyName;
  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return IconButton(
      key: ValueKey(keyName),
      icon: Icon(icon, size: 15),
      color: OrColors.textSecondary,
      tooltip: tooltip,
      visualDensity: VisualDensity.compact,
      constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
      padding: EdgeInsets.zero,
      onPressed: onPressed,
    );
  }
}

class _SendButton extends StatelessWidget {
  const _SendButton({required this.enabled, required this.onPressed});

  final bool enabled;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return IconButton(
      key: const ValueKey('assistant-send'),
      icon: const Icon(Icons.arrow_upward, size: 16),
      style: IconButton.styleFrom(
        backgroundColor: enabled ? OrColors.primary : OrColors.surfaceHover,
        foregroundColor: enabled ? OrColors.primaryText : OrColors.textMuted,
        minimumSize: const Size(28, 28),
        padding: EdgeInsets.zero,
      ),
      tooltip: 'Send',
      onPressed: enabled ? onPressed : null,
    );
  }
}

class _ContextButton extends StatelessWidget {
  const _ContextButton({
    required this.keyName,
    required this.icon,
    required this.label,
    required this.tooltip,
    required this.onPressed,
  });

  final String keyName;
  final IconData icon;
  final String label;
  final String tooltip;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: InkWell(
        key: ValueKey(keyName),
        onTap: onPressed,
        borderRadius: BorderRadius.circular(OrRadii.small),
        child: Container(
          height: 30,
          padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x2),
          decoration: BoxDecoration(
            border: Border.all(color: OrColors.border),
            borderRadius: BorderRadius.circular(OrRadii.small),
          ),
          child: Row(
            children: [
              Icon(icon, size: 14, color: OrColors.textSecondary),
              const SizedBox(width: OrSpacing.x2),
              Expanded(
                child: Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: OrType.tinySize,
                  ),
                ),
              ),
              const Icon(
                Icons.unfold_more,
                size: 13,
                color: OrColors.textMuted,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Dropdown extends StatelessWidget {
  const _Dropdown({
    required this.keyName,
    required this.icon,
    required this.hint,
    required this.value,
    required this.items,
    required this.onChanged,
  });

  final String keyName;
  final IconData icon;
  final String hint;
  final String? value;
  final List<DropdownMenuItem<String>> items;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 30,
      padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x2),
      decoration: BoxDecoration(
        border: Border.all(color: OrColors.border),
        borderRadius: BorderRadius.circular(OrRadii.small),
      ),
      child: Row(
        children: [
          Icon(icon, size: 13, color: OrColors.textMuted),
          const SizedBox(width: OrSpacing.x1),
          Expanded(
            child: DropdownButtonHideUnderline(
              child: DropdownButton<String>(
                key: ValueKey(keyName),
                value: value,
                isDense: true,
                isExpanded: true,
                hint: Text(
                  hint,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: OrColors.textMuted,
                    fontSize: OrType.tinySize,
                  ),
                ),
                icon: const Icon(
                  Icons.expand_more,
                  size: 14,
                  color: OrColors.textMuted,
                ),
                dropdownColor: OrColors.surface,
                borderRadius: BorderRadius.circular(OrRadii.small),
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.tinySize,
                ),
                items: items,
                onChanged: items.isEmpty ? null : onChanged,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _WaitingState extends StatelessWidget {
  const _WaitingState({required this.controller});

  final AssistantController controller;

  @override
  Widget build(BuildContext context) {
    return Center(
      key: const ValueKey('assistant-waiting'),
      child: Padding(
        padding: const EdgeInsets.all(OrSpacing.x6),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // A static mark, not a spinner: this state can last as long as the
            // harness takes to start, and a repeating animation makes the whole
            // editor unable to settle (and is visual noise in a rail).
            const Icon(Icons.auto_awesome, size: 16, color: OrColors.ai),
            const SizedBox(height: OrSpacing.x3),
            Text(
              controller.state == AssistantConnectionState.connecting
                  ? 'Starting DeepSeek Harness…'
                  : 'Preparing the assistant…',
              textAlign: TextAlign.center,
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataSize,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ErrorState extends StatelessWidget {
  const _ErrorState({
    required this.controller,
    required this.onRetry,
    required this.onSettings,
  });

  final AssistantController controller;
  final VoidCallback onRetry;
  final VoidCallback onSettings;

  @override
  Widget build(BuildContext context) {
    return ListView(
      key: const ValueKey('assistant-error'),
      padding: const EdgeInsets.all(OrSpacing.x4),
      children: [
        const Icon(Icons.error_outline, size: 22, color: OrColors.danger),
        const SizedBox(height: OrSpacing.x3),
        Text(
          controller.errorMessage ?? 'The assistant could not start.',
          style: const TextStyle(
            color: OrColors.text,
            fontSize: OrType.metadataSize,
            height: 1.4,
          ),
        ),
        if (controller.errorHint != null) ...[
          const SizedBox(height: OrSpacing.x2),
          Text(
            controller.errorHint!,
            style: const TextStyle(
              color: OrColors.textMuted,
              fontSize: OrType.tinySize,
              height: 1.4,
            ),
          ),
        ],
        const SizedBox(height: OrSpacing.x4),
        Wrap(
          spacing: OrSpacing.x2,
          runSpacing: OrSpacing.x1,
          children: [
            FilledButton.tonal(
              key: const ValueKey('assistant-retry'),
              onPressed: onRetry,
              child: const Text('Try again'),
            ),
            TextButton(
              key: const ValueKey('assistant-open-settings'),
              onPressed: onSettings,
              child: const Text('Settings'),
            ),
          ],
        ),
      ],
    );
  }
}

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.onExample});

  final ValueChanged<String> onExample;

  static const _examples = [
    'What is on the timeline right now?',
    'Add a 4-second title at 2 s that says “First cut”.',
    'Create an illustration from src/main.rs and append it to the timeline.',
  ];

  @override
  Widget build(BuildContext context) {
    return ListView(
      key: const ValueKey('assistant-empty'),
      padding: const EdgeInsets.all(OrSpacing.x4),
      children: [
        const Text(
          'Ask for an edit',
          style: TextStyle(
            color: OrColors.text,
            fontSize: OrType.metadataSize,
            fontWeight: FontWeight.w600,
          ),
        ),
        const SizedBox(height: OrSpacing.x2),
        const Text(
          'The assistant reads this project and edits it through the same '
          'commands as the editor. “This clip” and the playhead are shared '
          'with it.',
          style: TextStyle(
            color: OrColors.textSecondary,
            fontSize: OrType.tinySize,
            height: 1.45,
          ),
        ),
        const SizedBox(height: OrSpacing.x4),
        for (final example in _examples)
          Padding(
            padding: const EdgeInsets.only(bottom: OrSpacing.x2),
            child: InkWell(
              key: ValueKey('assistant-example-${_examples.indexOf(example)}'),
              onTap: () => onExample(example),
              borderRadius: BorderRadius.circular(OrRadii.small),
              child: Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: OrSpacing.x3,
                  vertical: OrSpacing.x2,
                ),
                decoration: BoxDecoration(
                  border: Border.all(color: OrColors.border),
                  borderRadius: BorderRadius.circular(OrRadii.small),
                ),
                child: Text(
                  example,
                  style: const TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: OrType.tinySize,
                    height: 1.4,
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class _PermissionCard extends StatelessWidget {
  const _PermissionCard({required this.request, required this.onAnswer});

  final AssistantPermissionRequest request;
  final ValueChanged<String> onAnswer;

  @override
  Widget build(BuildContext context) {
    return Container(
      key: const ValueKey('assistant-permission'),
      margin: const EdgeInsets.fromLTRB(
        OrSpacing.x3,
        OrSpacing.x1,
        OrSpacing.x3,
        OrSpacing.x2,
      ),
      padding: const EdgeInsets.all(OrSpacing.x3),
      decoration: BoxDecoration(
        color: OrColors.surface,
        border: Border.all(color: OrColors.borderStrong),
        borderRadius: BorderRadius.circular(OrRadii.control),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              const Icon(
                Icons.verified_user_outlined,
                size: 14,
                color: OrColors.warning,
              ),
              const SizedBox(width: OrSpacing.x2),
              Expanded(
                child: Text(
                  request.title,
                  style: const TextStyle(
                    color: OrColors.text,
                    fontSize: OrType.metadataSize,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ),
            ],
          ),
          if (request.detail.isNotEmpty) ...[
            const SizedBox(height: OrSpacing.x2),
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 96),
              child: SingleChildScrollView(
                child: Text(
                  request.detail,
                  style: const TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: OrType.tinySize,
                    fontFamily: 'monospace',
                    height: 1.35,
                  ),
                ),
              ),
            ),
          ],
          const SizedBox(height: OrSpacing.x3),
          Wrap(
            spacing: OrSpacing.x2,
            runSpacing: OrSpacing.x1,
            children: [
              for (final option in request.options)
                option.isAllow
                    ? FilledButton.tonal(
                        onPressed: () => onAnswer(option.optionId),
                        child: Text(option.name),
                      )
                    : OutlinedButton(
                        onPressed: () => onAnswer(option.optionId),
                        child: Text(option.name),
                      ),
            ],
          ),
        ],
      ),
    );
  }
}

class _EntryView extends StatelessWidget {
  const _EntryView({required this.entry});

  final AssistantEntry entry;

  @override
  Widget build(BuildContext context) {
    return switch (entry.kind) {
      AssistantEntryKind.user => _user(context),
      AssistantEntryKind.assistant => _assistant(context),
      AssistantEntryKind.thought => _ThoughtEntry(entry: entry),
      AssistantEntryKind.tool => _ToolEntry(entry: entry),
      AssistantEntryKind.error => _error(context),
      AssistantEntryKind.note => _note(context),
    };
  }

  Widget _user(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x3),
      child: Align(
        alignment: Alignment.centerRight,
        child: Container(
          constraints: BoxConstraints(
            maxWidth: MediaQuery.sizeOf(context).width * 0.72,
          ),
          padding: const EdgeInsets.symmetric(
            horizontal: OrSpacing.x3,
            vertical: OrSpacing.x2,
          ),
          decoration: BoxDecoration(
            color: OrColors.surface,
            border: Border.all(color: OrColors.border),
            borderRadius: BorderRadius.circular(OrRadii.control),
          ),
          child: SelectableText(
            entry.text,
            style: const TextStyle(
              color: OrColors.text,
              fontSize: OrType.metadataSize,
              height: 1.4,
            ),
          ),
        ),
      ),
    );
  }

  Widget _assistant(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x3),
      child: _AssistantText(text: entry.text),
    );
  }

  Widget _error(BuildContext context) {
    return Container(
      margin: const EdgeInsets.only(bottom: OrSpacing.x3),
      padding: const EdgeInsets.all(OrSpacing.x3),
      decoration: BoxDecoration(
        border: Border.all(color: OrColors.danger.withValues(alpha: 0.6)),
        borderRadius: BorderRadius.circular(OrRadii.control),
      ),
      child: SelectableText(
        entry.text,
        style: const TextStyle(
          color: OrColors.danger,
          fontSize: OrType.tinySize,
          height: 1.4,
        ),
      ),
    );
  }

  Widget _note(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x3),
      child: Text(
        entry.text,
        style: const TextStyle(
          color: OrColors.textMuted,
          fontSize: OrType.tinySize,
          height: 1.4,
        ),
      ),
    );
  }
}

class _AssistantText extends StatelessWidget {
  const _AssistantText({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final parts = _splitFences(text);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final part in parts)
          if (part.isCode)
            Container(
              margin: const EdgeInsets.only(bottom: OrSpacing.x2),
              padding: const EdgeInsets.all(OrSpacing.x2),
              decoration: BoxDecoration(
                color: OrColors.backgroundRaised,
                border: Border.all(color: OrColors.border),
                borderRadius: BorderRadius.circular(OrRadii.small),
              ),
              child: SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: SelectableText(
                  part.text,
                  style: const TextStyle(
                    color: OrColors.textSecondary,
                    fontSize: OrType.tinySize,
                    fontFamily: 'monospace',
                    height: 1.4,
                  ),
                ),
              ),
            )
          else if (part.text.trim().isNotEmpty)
            Padding(
              padding: const EdgeInsets.only(bottom: OrSpacing.x2),
              child: SelectableText(
                part.text.trim(),
                style: const TextStyle(
                  color: OrColors.text,
                  fontSize: OrType.metadataSize,
                  height: 1.45,
                ),
              ),
            ),
      ],
    );
  }
}

class _TextPart {
  const _TextPart(this.text, {this.isCode = false});

  final String text;
  final bool isCode;
}

List<_TextPart> _splitFences(String text) {
  final parts = <_TextPart>[];
  final pattern = RegExp(r'```[^\n]*\n([\s\S]*?)```');
  var start = 0;
  for (final match in pattern.allMatches(text)) {
    if (match.start > start) {
      parts.add(_TextPart(text.substring(start, match.start)));
    }
    parts.add(_TextPart(match.group(1) ?? '', isCode: true));
    start = match.end;
  }
  if (start < text.length) parts.add(_TextPart(text.substring(start)));
  return parts;
}

class _ThoughtEntry extends StatefulWidget {
  const _ThoughtEntry({required this.entry});

  final AssistantEntry entry;

  @override
  State<_ThoughtEntry> createState() => _ThoughtEntryState();
}

class _ThoughtEntryState extends State<_ThoughtEntry> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x2),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          InkWell(
            onTap: () => setState(() => _expanded = !_expanded),
            child: Row(
              children: [
                Icon(
                  _expanded ? Icons.expand_less : Icons.expand_more,
                  size: 14,
                  color: OrColors.textMuted,
                ),
                const SizedBox(width: OrSpacing.x1),
                Text(
                  widget.entry.title,
                  style: const TextStyle(
                    color: OrColors.textMuted,
                    fontSize: OrType.tinySize,
                  ),
                ),
              ],
            ),
          ),
          if (_expanded)
            Padding(
              padding: const EdgeInsets.only(top: OrSpacing.x1),
              child: SelectableText(
                widget.entry.text,
                style: const TextStyle(
                  color: OrColors.textMuted,
                  fontSize: OrType.tinySize,
                  height: 1.4,
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _ToolEntry extends StatefulWidget {
  const _ToolEntry({required this.entry});

  final AssistantEntry entry;

  @override
  State<_ToolEntry> createState() => _ToolEntryState();
}

class _ToolEntryState extends State<_ToolEntry> {
  bool _expanded = false;

  Future<File?> _writeImage(AssistantImage image, int index) async {
    try {
      final file = File(
        '${Directory.systemTemp.path}/or-assistant-${widget.entry.id}-$index-'
        '${DateTime.now().microsecondsSinceEpoch}.${image.extension}',
      );
      await file.writeAsBytes(image.bytes, flush: true);
      return file;
    } on FileSystemException {
      return null;
    }
  }

  Future<void> _openImage(AssistantImage image, int index) async {
    final file = await _writeImage(image, index);
    if (file == null) return;
    await Process.run('open', [file.path]);
  }

  Future<void> _revealImage(AssistantImage image, int index) async {
    final file = await _writeImage(image, index);
    if (file == null) return;
    await Process.run('open', ['-R', file.path]);
  }

  Widget _imageTile(AssistantImage image, int index) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Tooltip(
          message: 'Open image',
          child: InkWell(
            key: ValueKey('assistant-image-${widget.entry.id}-$index'),
            onTap: () => unawaited(_openImage(image, index)),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(OrRadii.control),
              child: Container(
                width: 132,
                height: 84,
                color: OrColors.backgroundRaised,
                child: Image.memory(
                  image.bytes,
                  fit: BoxFit.cover,
                  gaplessPlayback: true,
                ),
              ),
            ),
          ),
        ),
        InkWell(
          onTap: () => unawaited(_revealImage(image, index)),
          child: const Padding(
            padding: EdgeInsets.only(top: 2),
            child: Text(
              'Show in Finder',
              style: TextStyle(
                color: OrColors.textMuted,
                fontSize: OrType.tinySize,
              ),
            ),
          ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final entry = widget.entry;
    final (icon, color, statusLabel) = switch (entry.state) {
      AssistantToolState.pending => (
        Icons.radio_button_unchecked,
        OrColors.textMuted,
        'Queued',
      ),
      AssistantToolState.running => (Icons.autorenew, OrColors.ai, 'Running'),
      AssistantToolState.done => (
        Icons.check_circle_outline,
        OrColors.success,
        'Done',
      ),
      AssistantToolState.failed => (
        Icons.error_outline,
        OrColors.danger,
        'Failed',
      ),
    };
    final toolLabel = entry.toolName.replaceFirst('mcp__or__', '');
    return Container(
      margin: const EdgeInsets.only(bottom: OrSpacing.x2),
      decoration: BoxDecoration(
        border: Border.all(color: OrColors.border),
        borderRadius: BorderRadius.circular(OrRadii.control),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          InkWell(
            key: ValueKey('assistant-tool-${entry.id}'),
            onTap: () => setState(() => _expanded = !_expanded),
            child: Padding(
              padding: const EdgeInsets.symmetric(
                horizontal: OrSpacing.x2,
                vertical: OrSpacing.x2,
              ),
              child: Row(
                children: [
                  Icon(icon, size: 14, color: color),
                  const SizedBox(width: OrSpacing.x2),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          entry.title,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(
                            color: OrColors.text,
                            fontSize: OrType.tinySize,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                        if (toolLabel.isNotEmpty)
                          Text(
                            toolLabel,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(
                              color: OrColors.textMuted,
                              fontSize: OrType.tinySize,
                              fontFamily: 'monospace',
                            ),
                          ),
                      ],
                    ),
                  ),
                  Text(
                    statusLabel,
                    style: TextStyle(color: color, fontSize: OrType.tinySize),
                  ),
                ],
              ),
            ),
          ),
          if (entry.images.isNotEmpty)
            Padding(
              padding: const EdgeInsets.fromLTRB(
                OrSpacing.x2,
                0,
                OrSpacing.x2,
                OrSpacing.x2,
              ),
              child: Wrap(
                spacing: OrSpacing.x2,
                runSpacing: OrSpacing.x2,
                children: [
                  for (var index = 0; index < entry.images.length; index++)
                    _imageTile(entry.images[index], index),
                ],
              ),
            ),
          if (_expanded && entry.detail.isNotEmpty)
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(OrSpacing.x2),
              decoration: const BoxDecoration(
                color: OrColors.backgroundRaised,
                border: Border(top: BorderSide(color: OrColors.border)),
              ),
              child: SelectableText(
                entry.detail,
                style: const TextStyle(
                  color: OrColors.textSecondary,
                  fontSize: OrType.tinySize,
                  fontFamily: 'monospace',
                  height: 1.35,
                ),
              ),
            ),
        ],
      ),
    );
  }
}
