import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';

import 'acp_client.dart';
import 'agent_workspace.dart';
import 'assistant_models.dart';
import 'assistant_settings.dart';

/// Owns one assistant conversation: the harness process, its ACP session, the
/// transcript, and the pointer the OpenCut MCP server reads.
///
/// The controller never mutates project state itself. Edits happen because the
/// agent calls OpenCut tools, which attach to the app over local IPC and use
/// the same validated command path as every other client.
class AssistantController extends ChangeNotifier {
  AssistantController({
    AssistantSettingsStore? settingsStore,
    Future<Directory> Function()? supportDirectory,
    AcpClient Function({
      required void Function(Map<String, Object?> update) onUpdate,
      required Future<String?> Function(Map<String, Object?> request)
      onPermissionRequest,
      required void Function(int exitCode, String stderrTail) onExit,
    })?
    clientFactory,
  }) : _settingsStore = settingsStore ?? AssistantSettingsStore(),
       _supportDirectory = supportDirectory ?? getApplicationSupportDirectory,
       _clientFactory = clientFactory ?? _defaultClient;

  static AcpClient _defaultClient({
    required void Function(Map<String, Object?> update) onUpdate,
    required Future<String?> Function(Map<String, Object?> request)
    onPermissionRequest,
    required void Function(int exitCode, String stderrTail) onExit,
  }) => AcpClient(
    onUpdate: onUpdate,
    onPermissionRequest: onPermissionRequest,
    onExit: onExit,
  );

  final AssistantSettingsStore _settingsStore;
  final Future<Directory> Function() _supportDirectory;
  final AcpClient Function({
    required void Function(Map<String, Object?> update) onUpdate,
    required Future<String?> Function(Map<String, Object?> request)
    onPermissionRequest,
    required void Function(int exitCode, String stderrTail) onExit,
  })
  _clientFactory;

  AcpClient? _client;
  AssistantConnectionState _state = AssistantConnectionState.idle;
  AssistantSettings _settings = const AssistantSettings();
  Future<void>? _loadingSettings;
  bool _busy = false;
  String? _errorMessage;
  String? _errorHint;
  String _projectPath = '';
  String _projectName = '';
  String _descriptorPath = '';
  String? _pointerPath;
  AgentWorkspace? _agentWorkspace;
  String _agentWorkspacePath = '';
  String? _harnessPath;
  String _harnessLabel = '';
  String? _orCliPath;
  String? _overlayProfile;
  List<Map<String, Object?>> _configOptions = const [];
  String _selectedModel = '';
  String _selectedReasoning = '';
  int _usageUsed = 0;
  int _usageSize = 0;
  AssistantEditorContext _context = const AssistantEditorContext();
  AssistantPermissionRequest? _pendingPermission;
  Completer<String?>? _permissionCompleter;
  List<Map<String, Object?>> _lastMcpServers = const [];
  String _lastCwd = '';
  final List<AssistantEntry> _entries = [];
  int _entrySequence = 0;
  String? _openAssistantEntryId;
  String? _openThoughtEntryId;
  bool _disposed = false;
  bool _closing = false;
  int _permissionSequence = 0;

  AssistantConnectionState get state => _state;
  AssistantSettings get settings => _settings;
  bool get busy => _busy;
  String? get errorMessage => _errorMessage;
  String? get errorHint => _errorHint;

  /// The directory the harness session starts in.
  ///
  /// The agent workspace is the default because its AGENTS.md routes the agent to
  /// the one guide a task needs. The project folder and a chosen folder remain
  /// selectable in Assistant settings.
  String get workspace => switch (_settings.workspaceMode) {
    AssistantWorkspaceMode.agent =>
      _agentWorkspacePath.isNotEmpty
          ? _agentWorkspacePath
          : _defaultWorkspace(),
    AssistantWorkspaceMode.project =>
      _projectPath.isNotEmpty ? _directoryOf(_projectPath) : _defaultWorkspace(),
    AssistantWorkspaceMode.custom =>
      _settings.workspacePath.isNotEmpty
          ? _settings.workspacePath
          : _defaultWorkspace(),
  };

  /// The installed agent workspace, empty until [initialize] finishes or when
  /// installation failed and the panel fell back to another folder.
  String get agentWorkspacePath => _agentWorkspacePath;
  AssistantWorkspaceMode get workspaceMode => _settings.workspaceMode;
  String get projectName => _projectName;
  String get projectPath => _projectPath;
  String get descriptorPath => _descriptorPath;
  bool get hasProject => _descriptorPath.isNotEmpty;
  String? get harnessPath => _harnessPath;
  String get harnessLabel => _harnessLabel;
  String? get orCliPath => _orCliPath;
  String? get overlayProfile => _overlayProfile;
  int get usageUsed => _usageUsed;
  int get usageSize => _usageSize;
  AssistantEditorContext get context => _context;
  AssistantPermissionRequest? get pendingPermission => _pendingPermission;
  List<AssistantEntry> get entries => List.unmodifiable(_entries);
  bool get canSend => _state == AssistantConnectionState.ready && !_busy;
  String get selectedModel => _selectedModel;
  String get selectedReasoning => _selectedReasoning;

  bool get _isConnected => _state == AssistantConnectionState.ready;

  List<AssistantModelChoice> get modelChoices {
    for (final option in _configOptions) {
      if (option['id'] != 'model') continue;
      return _flattenSelectOptions(option);
    }
    return const [];
  }

  List<AssistantModelChoice> get reasoningChoices {
    for (final option in _configOptions) {
      if (option['id'] != 'reasoning_effort') continue;
      return _flattenSelectOptions(option);
    }
    return const [];
  }

  Future<void> initialize() {
    return _loadingSettings ??= _loadSettings();
  }

  Future<void> _loadSettings() async {
    _settings = await _settingsStore.load();
    await _ensureAgentWorkspace();
    _notify();
  }

  /// Installs the bundled workspace once and keeps its runtime files current.
  ///
  /// A failure is not fatal: the panel falls back to the project folder so the
  /// assistant still works when application support is unavailable (for example
  /// in a widget test).
  Future<void> _ensureAgentWorkspace() async {
    if (_agentWorkspacePath.isNotEmpty) return;
    try {
      final support = await _supportDirectory();
      final bundle = await loadAgentWorkspaceBundle();
      final workspace = await AgentWorkspace.install(
        directory: Directory(
          '${support.path}${Platform.pathSeparator}$agentWorkspaceDirectoryName',
        ),
        bundle: bundle,
      );
      _agentWorkspace = workspace;
      _agentWorkspacePath = workspace.path;
      await _writeWorkspaceRuntimeFiles();
    } catch (_) {
      _agentWorkspace = null;
      _agentWorkspacePath = '';
    }
  }

  /// Keeps `project.json`, `or-cli.json` and `tools/or` in step with the app.
  Future<void> _writeWorkspaceRuntimeFiles() async {
    final workspace = _agentWorkspace;
    if (workspace == null) return;
    try {
      if (_projectPath.isNotEmpty || _descriptorPath.isNotEmpty) {
        await workspace.writeProjectMarker(
          projectName: _projectName,
          projectPath: _projectPath,
          descriptorPath: _descriptorPath,
        );
      }
      final orCli = _orCliPath;
      if (orCli != null && orCli.isNotEmpty) {
        await workspace.writeOrCli(orCli);
      }
    } catch (_) {
      // The workspace stays usable; the agent reports a missing tool if it is
      // asked for before the next connection rewrites it.
    }
  }

  /// Records the open project and writes the MCP pointer. The workspace choice is
  /// independent of the project, so opening another project does not silently
  /// move the agent's files.
  Future<void> updateProject({
    required String descriptorPath,
    required String projectPath,
    required String projectName,
  }) async {
    final changed =
        descriptorPath != _descriptorPath ||
        projectPath != _projectPath ||
        projectName != _projectName;
    if (!changed && _pointerPath != null) return;
    _descriptorPath = descriptorPath;
    _projectPath = projectPath;
    _projectName = projectName;
    await _ensureAgentWorkspace();
    await _writeWorkspaceRuntimeFiles();
    await _writePointer();
    _notify();
  }

  /// Publishes the editor focus, skipping a context the panel already has.
  void setContext(AssistantEditorContext context) {
    if (_context == context) return;
    _context = context;
    _notify();
  }

  /// Changes the session workspace and restarts the conversation so the
  /// agent's working directory and file tools match the choice.
  Future<void> changeWorkspace(String path) async {
    if (path.isEmpty || path == workspace) return;
    _settings = _settings.copyWith(
      workspaceMode: AssistantWorkspaceMode.custom,
      workspacePath: path,
    );
    await _settingsStore.save(_settings);
    _notify();
    if (_isConnected) {
      await newConversation();
    }
  }

  Future<void> connect() async {
    if (_state == AssistantConnectionState.connecting || _isConnected) return;
    _state = AssistantConnectionState.connecting;
    _errorMessage = null;
    _errorHint = null;
    _notify();
    await initialize();
    try {
      final directory = await _assistantDirectory();
      final patchFile = File(
        '${directory.path}${Platform.pathSeparator}harness.patch.yml',
      );
      final launch = await resolveHarnessLaunch(
        settings: _settings,
        patchPath: patchFile.path,
      );
      if (launch == null) {
        throw const AssistantHarnessFailure(
          'No ACP harness was found where the assistant can start it.',
          hint: 'Choose a harness in Assistant settings, install DeepSeek Harness, '
              'Claude Code ACP, Codex ACP or OpenCode, or point at another ACP command.',
        );
      }
      final orCli = await resolveOrCliPath(settings: _settings);
      if (orCli == null) {
        throw const AssistantHarnessFailure(
          'The or CLI was not found, so the assistant cannot edit the project.',
          hint: 'Use a packaged build that ships the CLI, build it once with `cargo build -p or_cli`, or set its path in Assistant settings.',
        );
      }
      _harnessPath = launch.executable;
      _harnessLabel = launch.label;
      _orCliPath = orCli;
      await _ensureAgentWorkspace();
      await _writeWorkspaceRuntimeFiles();
      final overlay = await findHarnessOverlay();
      _overlayProfile = overlay?.profile;
      await patchFile.writeAsString(
        buildHarnessPatch(
          overlay: overlay,
          workspace: workspace,
          projectName: _projectName.isEmpty ? 'Untitled project' : _projectName,
        ),
      );
      await _writePointer();
      final pointerPath = _pointerPath;
      if (pointerPath == null) {
        throw const AssistantHarnessFailure(
          'The assistant could not write its session pointer.',
        );
      }
      final cwd = Directory(workspace).existsSync()
          ? workspace
          : Directory.current.path;
      _lastCwd = cwd;
      _lastMcpServers = [
        {
          'name': 'or',
          'command': orCli,
          'args': ['mcp', '--pointer', pointerPath],
          'env': <Object?>[],
        },
      ];
      final client = _clientFactory(
        onUpdate: _onUpdate,
        onPermissionRequest: _onPermissionRequest,
        onExit: _onExit,
      );
      _client = client;
      await client.start(
        executable: launch.executable,
        arguments: launch.arguments,
        workingDirectory: cwd,
        cwd: cwd,
        mcpServers: _lastMcpServers,
      );
      _configOptions = client.configOptions;
      _state = AssistantConnectionState.ready;
      _errorMessage = null;
      _errorHint = null;
      await _applyPreferredOptions();
      _notify();
    } catch (error) {
      _closing = true;
      await _client?.dispose();
      _closing = false;
      _client = null;
      _state = AssistantConnectionState.error;
      _errorMessage = error is AssistantHarnessFailure
          ? error.message
          : error is AcpException
          ? error.message
          : 'The assistant could not start: $error';
      _errorHint = error is AssistantHarnessFailure ? error.hint : null;
      _notify();
    }
  }

  Future<void> disconnect() async {
    final client = _client;
    _client = null;
    _state = AssistantConnectionState.idle;
    _busy = false;
    _closing = true;
    await client?.dispose();
    _closing = false;
    _notify();
  }

  Future<void> newConversation() async {
    final client = _client;
    if (client == null || !_isConnected) {
      _entries.clear();
      _pendingPermission = null;
      _permissionCompleter?.complete(null);
      _permissionCompleter = null;
      await connect();
      return;
    }
    _busy = false;
    _pendingPermission = null;
    _permissionCompleter?.complete(null);
    _permissionCompleter = null;
    try {
      await client.newSession(cwd: _lastCwd, mcpServers: _lastMcpServers);
      _configOptions = client.configOptions;
      _entries.clear();
      _openAssistantEntryId = null;
      _openThoughtEntryId = null;
      _state = AssistantConnectionState.ready;
      _errorMessage = null;
      _errorHint = null;
      await _applyPreferredOptions();
    } catch (error) {
      _state = AssistantConnectionState.error;
      _errorMessage =
          'The assistant could not start a new conversation: $error';
    }
    _notify();
  }

  Future<void> send(String text) async {
    final trimmed = text.trim();
    if (trimmed.isEmpty || _busy) return;
    if (!_isConnected) {
      await connect();
      if (!_isConnected) return;
    }
    final client = _client;
    if (client == null) return;
    _entries.add(
      AssistantEntry(
        id: _nextEntryId(),
        kind: AssistantEntryKind.user,
        text: trimmed,
      ),
    );
    _openAssistantEntryId = null;
    _openThoughtEntryId = null;
    _busy = true;
    _notify();
    try {
      await client.prompt(trimmed + _context.toPromptAppendix());
    } catch (error) {
      _entries.add(
        AssistantEntry(
          id: _nextEntryId(),
          kind: AssistantEntryKind.error,
          text: error is AcpException
              ? '${error.message}${error.stderrTail.isEmpty ? '' : '\n${error.stderrTail}'}'
              : '$error',
        ),
      );
      if (error is AcpException && error.code == 'ACP_EXITED') {
        _state = AssistantConnectionState.stopped;
      }
    } finally {
      _busy = false;
      _openAssistantEntryId = null;
      _openThoughtEntryId = null;
      _notify();
    }
  }

  void cancel() {
    if (!_busy) return;
    _client?.cancel();
  }

  Future<void> setModel(String value) async {
    if (value.isEmpty || value == _selectedModel) return;
    _selectedModel = value;
    _settings = _settings.copyWith(modelValue: value);
    unawaited(_settingsStore.save(_settings));
    _notify();
    if (_isConnected) {
      try {
        await _client?.setConfigOption('model', value);
      } catch (_) {
        // The next turn reports a real failure; the selection stays visible.
      }
    }
  }

  Future<void> setReasoning(String value) async {
    if (value == _selectedReasoning) return;
    _selectedReasoning = value;
    _settings = _settings.copyWith(reasoningValue: value);
    unawaited(_settingsStore.save(_settings));
    _notify();
    if (_isConnected) {
      try {
        await _client?.setConfigOption('reasoning_effort', value);
      } catch (_) {
        // Reasoning effort is optional; a provider may not expose it.
      }
    }
  }

  Future<void> updateSettings(AssistantSettings settings) async {
    final previousWorkspace = workspace;
    final workspaceChoiceChanged =
        settings.workspaceMode != _settings.workspaceMode ||
        settings.workspacePath != _settings.workspacePath;
    _settings = settings;
    await _settingsStore.save(settings);
    await _ensureAgentWorkspace();
    await _writeWorkspaceRuntimeFiles();
    _notify();
    if (workspaceChoiceChanged &&
        workspace != previousWorkspace &&
        _isConnected) {
      await newConversation();
    }
  }

  Future<void> answerPermission(String optionId) async {
    final completer = _permissionCompleter;
    _permissionCompleter = null;
    _pendingPermission = null;
    if (completer != null && !completer.isCompleted) {
      completer.complete(optionId);
    }
    _notify();
  }

  Future<String?> _onPermissionRequest(Map<String, Object?> params) async {
    final toolCall = _asMap(params['toolCall']);
    final title = (toolCall['title'] as String?)?.trim();
    final options = <AssistantPermissionOption>[];
    final rawOptions = params['options'];
    if (rawOptions is List) {
      for (final raw in rawOptions) {
        if (raw is! Map) continue;
        final option = _asMap(raw);
        final optionId = option['optionId'];
        if (optionId is! String || optionId.isEmpty) continue;
        options.add(
          AssistantPermissionOption(
            optionId: optionId,
            name: (option['name'] as String?) ?? optionId,
            kind: (option['kind'] as String?) ?? 'other',
          ),
        );
      }
    }
    if (_settings.autoApprove) {
      for (final option in options) {
        if (option.isAllow) return option.optionId;
      }
    }
    if (options.isEmpty) return null;
    final completer = Completer<String?>();
    _permissionCompleter = completer;
    _pendingPermission = AssistantPermissionRequest(
      id: _permissionSequence++,
      title: title == null || title.isEmpty ? 'Allow this action?' : title,
      detail: _permissionDetail(toolCall),
      options: options,
    );
    _notify();
    return completer.future;
  }

  String _permissionDetail(Map<String, Object?> toolCall) {
    final raw = toolCall['rawInput'];
    if (raw is Map || raw is List) {
      try {
        return const JsonEncoder.withIndent('  ').convert(raw);
      } catch (_) {
        return '';
      }
    }
    if (raw is String) return raw;
    return '';
  }

  void _onUpdate(Map<String, Object?> update) {
    final kind = update['sessionUpdate'] as String?;
    switch (kind) {
      case 'agent_message_chunk':
        final text = _contentText(update['content']);
        if (text.isEmpty) return;
        final messageId = update['messageId'] as String?;
        if (_openAssistantEntryId == null ||
            (messageId != null && _openAssistantEntryId != messageId)) {
          final entry = AssistantEntry(
            id: _nextEntryId(),
            kind: AssistantEntryKind.assistant,
          );
          _entries.add(entry);
          _openAssistantEntryId = entry.id;
        }
        _entryById(_openAssistantEntryId!)?.text += text;
        _notify();
        break;
      case 'agent_thought_chunk':
        final text = _contentText(update['content']);
        if (text.isEmpty) return;
        final messageId = update['messageId'] as String?;
        if (_openThoughtEntryId == null ||
            (messageId != null && _openThoughtEntryId != messageId)) {
          final entry = AssistantEntry(
            id: _nextEntryId(),
            kind: AssistantEntryKind.thought,
            title: 'Thinking',
          );
          _entries.add(entry);
          _openThoughtEntryId = entry.id;
        }
        _entryById(_openThoughtEntryId!)?.text += text;
        _notify();
        break;
      case 'tool_call':
        _openAssistantEntryId = null;
        _openThoughtEntryId = null;
        final toolCallId = update['toolCallId'] as String?;
        if (toolCallId == null) return;
        _entries.add(
          AssistantEntry(
            id: toolCallId,
            kind: AssistantEntryKind.tool,
            title: (update['title'] as String?) ?? 'Tool call',
            toolName: (update['name'] as String?) ?? '',
            toolKind: (update['kind'] as String?) ?? 'other',
            state: _toolState(update['status']),
            detail: _toolDetail(update),
            images: _toolImages(update),
          ),
        );
        _notify();
        break;
      case 'tool_call_update':
        final toolCallId = update['toolCallId'] as String?;
        if (toolCallId == null) return;
        final entry = _entryById(toolCallId);
        if (entry == null) return;
        final title = update['title'];
        if (title is String && title.isNotEmpty) entry.title = title;
        final name = update['name'];
        if (name is String && name.isNotEmpty) entry.toolName = name;
        final toolKind = update['kind'];
        if (toolKind is String && toolKind.isNotEmpty) {
          entry.toolKind = toolKind;
        }
        if (update.containsKey('status')) {
          entry.state = _toolState(update['status']);
        }
        final detail = _toolDetail(update);
        if (detail.isNotEmpty) entry.detail = detail;
        final images = _toolImages(update);
        if (images.isNotEmpty) entry.images = images;
        _notify();
        break;
      case 'usage_update':
        _usageUsed = (_integer(update['used'])).clamp(0, 1 << 62);
        _usageSize = (_integer(update['size'])).clamp(0, 1 << 62);
        _notify();
        break;
      default:
        break;
    }
  }

  void _onExit(int exitCode, String stderrTail) {
    if (_disposed || _closing) return;
    _busy = false;
    _state = AssistantConnectionState.stopped;
    if (stderrTail.trim().isNotEmpty) {
      _errorMessage = stderrTail.trim().split('\n').last;
    }
    _entries.add(
      AssistantEntry(
        id: _nextEntryId(),
        kind: AssistantEntryKind.error,
        text: exitCode == 0
            ? 'The harness stopped.'
            : 'The harness stopped with exit code $exitCode.',
      ),
    );
    _notify();
  }

  Future<void> _applyPreferredOptions() async {
    _selectedModel = '';
    for (final option in _configOptions) {
      if (option['id'] == 'model') {
        _selectedModel = (option['currentValue'] as String?) ?? '';
      }
      if (option['id'] == 'reasoning_effort') {
        _selectedReasoning = (option['currentValue'] as String?) ?? '';
      }
    }
    final models = modelChoices;
    final preferred = _settings.modelValue;
    if (preferred.isNotEmpty &&
        models.any((choice) => choice.value == preferred) &&
        preferred != _selectedModel) {
      await setModel(preferred);
    } else if (preferred.isNotEmpty && preferred == _selectedModel) {
      _notify();
    }
    final reasoning = _settings.reasoningValue;
    if (reasoning.isNotEmpty &&
        reasoningChoices.any((choice) => choice.value == reasoning) &&
        reasoning != _selectedReasoning) {
      await setReasoning(reasoning);
    }
  }

  /// Seeds a transcript for widget tests. It never starts the harness.
  @visibleForTesting
  void debugSeedConversation({
    AssistantConnectionState? state,
    List<AssistantEntry> entries = const [],
    String? error,
  }) {
    if (state != null) _state = state;
    _entries
      ..clear()
      ..addAll(entries);
    _errorMessage = error;
    _notify();
  }

  Future<Directory> _assistantDirectory() async {
    final support = await _supportDirectory();
    final directory = Directory(
      '${support.path}${Platform.pathSeparator}assistant',
    );
    await directory.create(recursive: true);
    return directory;
  }

  Future<void> _writePointer() async {
    try {
      final directory = await _assistantDirectory();
      final file = File(
        '${directory.path}${Platform.pathSeparator}session-pointer.json',
      );
      _pointerPath = file.path;
      final support = await _supportDirectory();
      await file.writeAsString(
        jsonEncode({
          'version': 1,
          'descriptor_path': _descriptorPath,
          'project_path': _projectPath,
          'project_name': _projectName,
          'workspace_path': workspace,
          'motion_library_path':
              '${support.path}${Platform.pathSeparator}motion-library',
        }),
      );
    } catch (_) {
      // The pointer is rewritten on the next project update; the panel shows
      // the tool failure if it stays missing.
    }
  }

  String _nextEntryId() => 'entry-${_entrySequence++}';

  AssistantEntry? _entryById(String id) {
    for (final entry in _entries) {
      if (entry.id == id) return entry;
    }
    return null;
  }

  String _defaultWorkspace() {
    if (_projectPath.isNotEmpty) return _directoryOf(_projectPath);
    return Directory.current.path;
  }

  String _directoryOf(String path) {
    final index = path.lastIndexOf(RegExp(r'[\\/]'));
    if (index <= 0) return path;
    return path.substring(0, index);
  }

  String _contentText(Object? content) {
    final map = _asMap(content);
    if (map['type'] == 'text') {
      return (map['text'] as String?) ?? '';
    }
    return '';
  }

  List<AssistantImage> _toolImages(Map<String, Object?> update) {
    final images = <AssistantImage>[];
    final content = update['content'];
    if (content is! List) return images;
    for (final item in content) {
      final map = _asMap(item);
      if (map['type'] != 'image') continue;
      final data = map['data'];
      if (data is! String || data.isEmpty) continue;
      try {
        images.add(
          AssistantImage(
            bytes: base64Decode(data),
            mimeType: (map['mimeType'] as String?) ?? 'image/png',
          ),
        );
      } on FormatException {
        // Ignore a malformed image payload; the text transcript still shows.
      }
    }
    return images;
  }

  String _toolDetail(Map<String, Object?> update) {
    final buffer = StringBuffer();
    final content = update['content'];
    if (content is List) {
      for (final item in content) {
        final map = _asMap(item);
        switch (map['type']) {
          case 'content':
            final inner = _asMap(map['content']);
            final text = inner['text'];
            if (text is String && text.isNotEmpty) buffer.writeln(text);
            break;
          case 'diff':
            final path = map['path'];
            if (path is String) buffer.writeln('---- $path');
            final oldText = map['oldText'];
            final newText = map['newText'];
            if (oldText is String && oldText.isNotEmpty) {
              buffer.writeln('- $oldText');
            }
            if (newText is String && newText.isNotEmpty) {
              buffer.writeln('+ $newText');
            }
            break;
          case 'terminal':
            final terminalId = map['terminalId'];
            if (terminalId is String) {
              buffer.writeln('terminal $terminalId');
            }
            break;
        }
      }
    }
    if (buffer.isEmpty) {
      final raw = update['rawInput'];
      if (raw is Map || raw is List) {
        try {
          buffer.write(const JsonEncoder.withIndent('  ').convert(raw));
        } catch (_) {
          // Ignore an unserializable payload.
        }
      }
    }
    return buffer.toString().trim();
  }

  AssistantToolState _toolState(Object? status) => switch (status) {
    'pending' => AssistantToolState.pending,
    'in_progress' => AssistantToolState.running,
    'failed' => AssistantToolState.failed,
    _ => AssistantToolState.done,
  };

  List<AssistantModelChoice> _flattenSelectOptions(
    Map<String, Object?> option,
  ) {
    final raw = option['options'];
    final choices = <AssistantModelChoice>[];
    if (raw is! List) return choices;
    for (final item in raw) {
      if (item is! Map) continue;
      final map = _asMap(item);
      if (map['options'] is List) {
        final group =
            (map['name'] as String?) ?? (map['group'] as String?) ?? '';
        for (final nested in map['options'] as List) {
          final optionMap = _asMap(nested);
          final value = optionMap['value'];
          if (value is! String) continue;
          choices.add(
            AssistantModelChoice(
              value: value,
              name: (optionMap['name'] as String?) ?? value,
              group: group,
              description: (optionMap['description'] as String?) ?? '',
            ),
          );
        }
      } else {
        final value = map['value'];
        if (value is! String) continue;
        choices.add(
          AssistantModelChoice(
            value: value,
            name: (map['name'] as String?) ?? value,
            description: (map['description'] as String?) ?? '',
          ),
        );
      }
    }
    return choices;
  }

  int _integer(Object? value) {
    if (value is int) return value;
    if (value is num) return value.toInt();
    return 0;
  }

  Map<String, Object?> _asMap(Object? value) {
    if (value is Map<String, Object?>) return value;
    if (value is Map) {
      return value.map((key, value) => MapEntry(key.toString(), value));
    }
    return const {};
  }

  void _notify() {
    if (!_disposed) notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    _permissionCompleter?.complete(null);
    unawaited(_client?.dispose());
    super.dispose();
  }
}
