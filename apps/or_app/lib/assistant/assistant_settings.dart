import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

/// Which directory the harness session starts in.
enum AssistantWorkspaceMode {
  /// The app-managed agent workspace with its AGENTS.md, guides and tools.
  agent,

  /// The open project's own folder.
  project,

  /// A folder the user chose.
  custom,
}

/// Local preferences for the assistant panel. Nothing here is project state.
class AssistantSettings {
  const AssistantSettings({
    this.harnessPath = '',
    this.orCliPath = '',
    this.workspaceMode = AssistantWorkspaceMode.agent,
    this.workspacePath = '',
    this.modelValue = '',
    this.reasoningValue = '',
    this.autoApprove = false,
  });

  /// Empty means "discover the installed harness".
  final String harnessPath;

  /// Empty means "discover the `or` CLI beside the checkout or on PATH".
  final String orCliPath;

  /// How the session workspace is resolved.
  final AssistantWorkspaceMode workspaceMode;

  /// The chosen folder when [workspaceMode] is [AssistantWorkspaceMode.custom].
  final String workspacePath;

  /// Opaque ACP model option value, e.g. `["opencode-go","deepseek-v4.1-flash"]`.
  final String modelValue;

  /// Opaque ACP reasoning-effort value; empty means the provider default.
  final String reasoningValue;

  /// When true, permission requests for tool calls are allowed automatically.
  final bool autoApprove;

  AssistantSettings copyWith({
    String? harnessPath,
    String? orCliPath,
    AssistantWorkspaceMode? workspaceMode,
    String? workspacePath,
    String? modelValue,
    String? reasoningValue,
    bool? autoApprove,
  }) => AssistantSettings(
    harnessPath: harnessPath ?? this.harnessPath,
    orCliPath: orCliPath ?? this.orCliPath,
    workspaceMode: workspaceMode ?? this.workspaceMode,
    workspacePath: workspacePath ?? this.workspacePath,
    modelValue: modelValue ?? this.modelValue,
    reasoningValue: reasoningValue ?? this.reasoningValue,
    autoApprove: autoApprove ?? this.autoApprove,
  );

  Map<String, Object?> toJson() => {
    'version': 2,
    'harnessPath': harnessPath,
    'orCliPath': orCliPath,
    'workspaceMode': workspaceMode.name,
    'workspacePath': workspacePath,
    'modelValue': modelValue,
    'reasoningValue': reasoningValue,
    'autoApprove': autoApprove,
  };

  static AssistantSettings fromJson(Object? value) {
    if (value is! Map<String, Object?>) return const AssistantSettings();
    String string(String key) {
      final raw = value[key];
      return raw is String && raw.length <= 4096 ? raw : '';
    }

    final workspacePath = string('workspacePath');
    final mode = _workspaceMode(value['workspaceMode'], workspacePath);
    return AssistantSettings(
      harnessPath: string('harnessPath'),
      orCliPath: string('orCliPath'),
      workspaceMode: mode,
      workspacePath: workspacePath,
      modelValue: string('modelValue'),
      reasoningValue: string('reasoningValue'),
      autoApprove: value['autoApprove'] == true,
    );
  }

  /// Reads the stored mode. Settings written before modes existed keep a custom
  /// folder when they had one, and otherwise adopt the agent workspace default.
  static AssistantWorkspaceMode _workspaceMode(
    Object? raw,
    String workspacePath,
  ) {
    for (final mode in AssistantWorkspaceMode.values) {
      if (raw == mode.name) return mode;
    }
    return workspacePath.isEmpty
        ? AssistantWorkspaceMode.agent
        : AssistantWorkspaceMode.custom;
  }
}

/// Persists assistant preferences in the application support directory.
///
/// Writes are serialized so two quick preference changes cannot interleave.
class AssistantSettingsStore {
  AssistantSettingsStore({Future<Directory> Function()? supportDirectory})
    : _supportDirectory = supportDirectory ?? getApplicationSupportDirectory;

  final Future<Directory> Function() _supportDirectory;
  AssistantSettings _lastLoaded = const AssistantSettings();
  Future<void> _writeChain = Future.value();

  Future<File> _file() async => File(
    '${(await _supportDirectory()).path}${Platform.pathSeparator}'
    'assistant${Platform.pathSeparator}settings.v1.json',
  );

  Future<AssistantSettings> load() async {
    final file = await _file();
    if (!await file.exists() || await file.length() > 64 * 1024) {
      _lastLoaded = const AssistantSettings();
      return _lastLoaded;
    }
    try {
      _lastLoaded = AssistantSettings.fromJson(
        jsonDecode(await file.readAsString()),
      );
    } catch (_) {
      _lastLoaded = const AssistantSettings();
    }
    return _lastLoaded;
  }

  AssistantSettings get cached => _lastLoaded;

  Future<void> save(AssistantSettings settings) {
    _lastLoaded = settings;
    final next = _writeChain.then((_) async {
      final file = await _file();
      await file.parent.create(recursive: true);
      await file.writeAsString(jsonEncode(settings.toJson()));
    });
    _writeChain = next.catchError((Object _) {});
    return next;
  }
}

/// A dsh profile's provider layer, extracted so the assistant can boot the same
/// provider and model the user already runs in DeepSeek Harness.
class HarnessOverlay {
  const HarnessOverlay({
    required this.patchBlock,
    required this.provider,
    required this.model,
    required this.profile,
  });

  /// The verbatim `llm-pi-ai` entry, including its provider model catalog.
  final String patchBlock;
  final String provider;
  final String model;
  final String profile;
}

/// Candidate harness executables, resolved without trusting the GUI PATH.
Future<String?> resolveHarnessPath({
  required AssistantSettings settings,
  Map<String, String>? environment,
}) async {
  final env = environment ?? Platform.environment;
  if (settings.harnessPath.isNotEmpty &&
      await File(settings.harnessPath).exists()) {
    return settings.harnessPath;
  }
  final home = env['HOME'];
  final candidates = <String>[
    '/Applications/DeepSeek Harness.app/Contents/Resources/runtime/cli/bin/dsh',
    if (home != null) '$home/.local/bin/dsh',
  ];
  for (final candidate in candidates) {
    if (await File(candidate).exists()) return candidate;
  }
  return _findOnPath('dsh', env);
}

/// Locates the `or` CLI: the assistant's project surface and MCP bridge.
Future<String?> resolveOrCliPath({
  required AssistantSettings settings,
  Map<String, String>? environment,
  Directory? currentDirectory,
  String? executablePath,
}) async {
  final env = environment ?? Platform.environment;
  if (settings.orCliPath.isNotEmpty) {
    return await File(settings.orCliPath).exists() ? settings.orCliPath : null;
  }
  final resolvedExecutable = executablePath ?? Platform.resolvedExecutable;
  final starts = <Directory>[
    (currentDirectory ?? Directory.current).absolute,
    // A packaged or `open`-launched app has `/` as its working directory, but
    // a development build still sits inside the checkout, so walk from the
    // executable too.
    Directory(resolvedExecutable).parent.absolute,
  ];
  for (final start in starts) {
    var directory = start;
    for (var depth = 0; depth < 12; depth += 1) {
      for (final relative in const ['target/debug/or', 'target/release/or']) {
        final candidate = File(
          '${directory.path}${Platform.pathSeparator}'
          '${relative.replaceAll('/', Platform.pathSeparator)}',
        );
        if (await candidate.exists()) return candidate.path;
      }
      final parent = directory.parent;
      if (parent.path == directory.path) break;
      directory = parent;
    }
  }
  // A packaged OR bundle ships the CLI beside its executable, so the assistant
  // keeps working after the app is moved out of the checkout.
  final bundled = File(
    '${Directory(resolvedExecutable).parent.path}'
    '${Platform.pathSeparator}or',
  );
  if (await bundled.exists()) return bundled.path;
  const applicationCandidate =
      '/Applications/Opencut Reinforced.app/Contents/MacOS/or';
  if (await File(applicationCandidate).exists()) return applicationCandidate;
  return _findOnPath('or', env);
}

Future<String?> _findOnPath(String name, Map<String, String> env) async {
  final path = env['PATH'];
  if (path == null) return null;
  for (final directory in path.split(':')) {
    if (directory.isEmpty) continue;
    final candidate = File('$directory/$name');
    if (await candidate.exists()) return candidate.path;
  }
  return null;
}

/// Finds the user's active dsh profile overlay (desktop first) and copies the
/// provider/model configuration out of it.
Future<HarnessOverlay?> findHarnessOverlay({
  Map<String, String>? environment,
  Directory? profilesDirectory,
}) async {
  final env = environment ?? Platform.environment;
  final home = env['HOME'];
  final root =
      profilesDirectory ??
      (home == null ? null : Directory('$home/.dsh/profiles'));
  if (root == null || !await root.exists()) return null;
  final candidates = await root
      .list()
      .where((entity) => entity is Directory)
      .cast<Directory>()
      .toList();
  candidates.sort((a, b) {
    final aDesktop = _basename(a.path) == 'desktop' ? 0 : 1;
    final bDesktop = _basename(b.path) == 'desktop' ? 0 : 1;
    if (aDesktop != bDesktop) return aDesktop - bDesktop;
    return a.path.compareTo(b.path);
  });
  for (final directory in candidates) {
    final patch = File(
      '${directory.path}${Platform.pathSeparator}cordis.patch.yml',
    );
    if (!await patch.exists()) continue;
    final text = await patch.readAsString();
    final block = _yamlEntry(text, 'llm-pi-ai');
    if (block == null) continue;
    final defaults = _yamlEntry(text, 'agent-default-model');
    final provider = _yamlValue(defaults, 'provider') ?? '';
    final model = _yamlValue(defaults, 'model') ?? '';
    if (provider.isEmpty || model.isEmpty) continue;
    return HarnessOverlay(
      patchBlock: block.trimRight(),
      provider: provider,
      model: model,
      profile: _basename(directory.path),
    );
  }
  return null;
}

String _basename(String path) =>
    path.split(RegExp(r'[\\/]')).where((part) => part.isNotEmpty).last;

/// Returns the top-level patch entry with this id, verbatim.
String? _yamlEntry(String text, String id) {
  final lines = const LineSplitter().convert(text);
  final start = lines.indexWhere(
    (line) => RegExp('^- id:\\s*$id\\s*\$').hasMatch(line),
  );
  if (start < 0) return null;
  var end = start + 1;
  while (end < lines.length && !RegExp(r'^- id:').hasMatch(lines[end])) {
    end += 1;
  }
  return lines.sublist(start, end).join('\n');
}

String? _yamlValue(String? entry, String key) {
  if (entry == null) return null;
  for (final line in const LineSplitter().convert(entry)) {
    final match = RegExp('^\\s*$key:\\s*(.+)\$').firstMatch(line);
    if (match != null) {
      return match.group(1)!.trim().replaceAll(RegExp(r'''^["']|["']$'''), '');
    }
  }
  return null;
}

/// Builds the `--patch` overlay the assistant boots the harness with.
///
/// The overlay keeps the user's provider catalog, selects their model, and
/// replaces the persona with OR's assistant contract. It contains no secrets;
/// the harness resolves credentials from its own credential service.
String buildHarnessPatch({
  required HarnessOverlay? overlay,
  required String workspace,
  required String projectName,
}) {
  final buffer = StringBuffer();
  if (overlay != null) {
    buffer.writeln('- id: acp');
    buffer.writeln('  config:');
    buffer.writeln('    provider: ${_yamlScalar(overlay.provider)}');
    buffer.writeln('    model: ${_yamlScalar(overlay.model)}');
  }
  buffer.writeln('- id: system-prompt');
  buffer.writeln('  config:');
  buffer.writeln('    personaPrefix: |-');
  for (final line in _assistantPersona(
    workspace: workspace,
    projectName: projectName,
  ).split('\n')) {
    buffer.writeln('      $line');
  }
  buffer.writeln('    personaSuffix: |-');
  buffer.writeln(
    '      Read and write files only inside the session workspace. The running app '
    'owns the project and the or CLI reaches it; import an external file by '
    'absolute path only when the user names it.',
  );
  if (overlay != null) {
    buffer.writeln(overlay.patchBlock);
  }
  return buffer.toString();
}

String _assistantPersona({
  required String workspace,
  required String projectName,
}) =>
    '''
You are the built-in editing assistant inside Opencut Reinforced, a local-first video editor.
The open project is "$projectName". The session workspace is $workspace.

The workspace AGENTS.md is the routing index for this session. Read the one guide that matches the current task; do not preload every guide.

Read and change the project with the or CLI in this workspace, not with the MCP tools. The CLI is the default project surface: `tools/or <verb> --attach project.json --json`. project.json is the app-written session pointer, and the CLI follows it to the running app on every call, so the app and the CLI always share one project and one history. Run every command from the workspace root. The running application owns the project; never edit a .orproj file or the project directory directly.

Use an mcp__or__* tool only where the CLI cannot act:
- tools/or motion publish --attach project.json --scene PATH (or --source PATH): validates the scene, saves it to the motion library, renders it, imports the video and inserts the clip in one call. `tools/or motion materialize` only writes a file; use it when a file is the goal. `mcp__or__motion_insert_scene` is the fallback when publish is unavailable.
- tools/or motion asset list|insert --attach project.json: read and use the app's motion library. project.json records the library path. The mcp__or__motion_asset_* tools are the fallback, not the default.
- A CLI call failed for a reason its error does not explain: report it, then use the matching MCP tool once instead of guessing.

Work in this order:
1. Read state first, adding `--attach project.json --json` to each call: `tools/or project summary`, `tools/or timeline tracks|settings|markers`, and `tools/or timeline clips --track TRACK_ID` or `tools/or media list` for ids, durations and exact times. Use only the ids and times those calls return.
2. Make the smallest edit the user asked for with `tools/or media add`, `tools/or timeline add-track|insert-clip|insert-text|move-clip|trim-clip|split-clip|delete-clip|ripple-delete-clip|set-frame-rate|add-marker`, or `tools/or history undo|redo`. `tools/or commands` and `tools/or queries` list the exact verbs and their flags.
3. Times accept exact NUM/DEN seconds or decimal seconds ("2.5"). insert-clip requires --source-start and --duration; take the duration from media list.
4. Verify the result with a CLI read, then save once with `tools/or project save --attach project.json --json` unless the user asked to review first. An attached edit is live but unsaved until that call.
5. For an illustration or motion graphic, write a scene in the workspace, validate it with `tools/or motion validate --file ... --json`, read its composition with `tools/or motion layout --file ... --at 1/2 --json` (node boxes at an exact time — do not ask for a screenshot), then publish it with `tools/or motion publish --attach project.json --scene PATH --json` (absolute path). MCP insertion is a fallback only.
6. For a script, narration, captions or background music, read docs/script-and-voice.md and use tools/voiceover.mjs, tools/music.mjs, tools/or captions transcribe or tools/or timeline import-captions. Narration and generated music are offline; transcription reuses the model installed from the app's Generate Captions action. The narration voice is the default the user saved in Settings → Narration, then the first installed Vietnamese voice: omit `--voice` unless the user names a voice, and never assume one voice name exists on this machine.

Never claim a change you did not make. If a tool fails, report the error and the next concrete step. Keep replies short: say what changed, on which track, at what time.''';

String _yamlScalar(String value) {
  if (RegExp(r'^[A-Za-z0-9_.\-/]+$').hasMatch(value)) return value;
  return "'${value.replaceAll("'", "''")}'";
}
