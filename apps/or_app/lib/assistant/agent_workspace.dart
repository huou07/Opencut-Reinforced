import 'dart:convert';
import 'dart:io';

import 'package:flutter/services.dart';

/// The bundles asset written by `tooling/or-agent-workspace/build.mjs`.
const String agentWorkspaceAssetPath = 'assets/agent_workspace/bundle.json';

/// The folder the assistant uses by default, under application support.
const String agentWorkspaceDirectoryName = 'agent-workspace';

/// Records the installed bundle version so an app upgrade can refresh the
/// framework files without touching the agent's own work.
const String agentWorkspaceManifestName = '.or-workspace.json';

/// One versioned set of workspace framework files.
class AgentWorkspaceBundle {
  const AgentWorkspaceBundle({required this.version, required this.files});

  final int version;
  final Map<String, String> files;

  static AgentWorkspaceBundle fromJson(Object? value) {
    if (value is! Map) {
      throw const FormatException(
        'the agent workspace bundle is not an object',
      );
    }
    final version = value['version'];
    final rawFiles = value['files'];
    if (version is! int || version < 1 || rawFiles is! Map) {
      throw const FormatException('the agent workspace bundle is malformed');
    }
    final files = <String, String>{};
    for (final entry in rawFiles.entries) {
      final path = entry.key;
      final content = entry.value;
      if (path is! String || content is! String) {
        throw const FormatException('an agent workspace file is malformed');
      }
      if (!isSafeWorkspacePath(path)) {
        throw FormatException(
          'the agent workspace bundle has an unsafe path: $path',
        );
      }
      files[path] = content;
    }
    if (files.isEmpty) {
      throw const FormatException('the agent workspace bundle is empty');
    }
    return AgentWorkspaceBundle(
      version: version,
      files: Map.unmodifiable(files),
    );
  }
}

/// A relative path inside the workspace: no absolute root, no parent segment and
/// no platform separator, so an asset can never escape the workspace folder.
bool isSafeWorkspacePath(String path) {
  if (path.isEmpty || path.startsWith('/') || path.contains('\\')) return false;
  if (path.contains('\u0000')) return false;
  for (final segment in path.split('/')) {
    if (segment.isEmpty || segment == '.' || segment == '..') return false;
  }
  return true;
}

Future<AgentWorkspaceBundle> loadAgentWorkspaceBundle({
  AssetBundle? assetBundle,
}) async {
  final text = await (assetBundle ?? rootBundle).loadString(
    agentWorkspaceAssetPath,
  );
  return AgentWorkspaceBundle.fromJson(jsonDecode(text));
}

/// The installed agent workspace: its folder, the framework files it owns, and
/// the small runtime files that bind it to the running app.
class AgentWorkspace {
  AgentWorkspace._({required this.directory, required this.bundle});

  final Directory directory;
  final AgentWorkspaceBundle bundle;

  String get path => directory.path;

  /// Installs the bundle into [directory].
  ///
  /// A file is written when it is missing, and every framework file is replaced
  /// when the bundle version changed. Files the agent created (under `work/` or
  /// anywhere else) are never deleted or overwritten because the bundle does not
  /// list them.
  static Future<AgentWorkspace> install({
    required Directory directory,
    required AgentWorkspaceBundle bundle,
  }) async {
    await directory.create(recursive: true);
    final manifest = File(_join(directory.path, agentWorkspaceManifestName));
    var installedVersion = 0;
    try {
      if (await manifest.exists()) {
        final decoded = jsonDecode(await manifest.readAsString());
        if (decoded is Map && decoded['version'] is int) {
          installedVersion = decoded['version'] as int;
        }
      }
    } catch (_) {
      // An unreadable manifest is treated as an absent installation.
    }

    final refresh = installedVersion != bundle.version;
    for (final entry in bundle.files.entries) {
      final file = File(_join(directory.path, entry.key));
      if (!refresh && await file.exists()) continue;
      await file.parent.create(recursive: true);
      await file.writeAsString(entry.value);
    }
    for (final name in const ['work', 'scenes']) {
      await Directory(_join(directory.path, name)).create(recursive: true);
    }
    await manifest.writeAsString(
      jsonEncode({
        'version': bundle.version,
        'file_count': bundle.files.length,
      }),
    );
    return AgentWorkspace._(directory: directory, bundle: bundle);
  }

  /// Records the open project so the agent can name it without calling a tool.
  Future<File> writeProjectMarker({
    required String projectName,
    required String projectPath,
    required String descriptorPath,
  }) => _writeJson('project.json', {
    'version': 1,
    'project_name': projectName,
    'project_path': projectPath,
    'descriptor_path': descriptorPath,
    // The workspace lives directly under application support, so the installed
    // tools can find non-secret app preferences (the narration voice) without
    // guessing a bundle identifier or duplicating a product constant.
    'app_support_path': directory.parent.path,
    // The app-level motion library sits beside the workspace. The CLI resolves
    // it from the pointer it attaches through, so carrying it here is what makes
    // `tools/or motion asset ... --attach project.json` work without --library.
    'motion_library_path':
        '${directory.parent.path}${Platform.pathSeparator}motion-library',
  });

  /// Records the `or` CLI beside the app build for `tools/or`.
  Future<File> writeOrCli(String orCliPath) async {
    await writeOrWrapper(orCliPath);
    return _writeJson('or-cli.json', {'version': 1, 'or_cli_path': orCliPath});
  }

  /// Writes the `tools/or` wrapper. It is regenerated on every connection, so a
  /// checkout move or a rebuilt CLI is picked up without editing the workspace.
  Future<File> writeOrWrapper(String orCliPath) async {
    final file = File(_join(path, 'tools/or'));
    await file.parent.create(recursive: true);
    await file.writeAsString(
      '#!/bin/sh\n'
      '# Generated by Opencut Reinforced; rewritten on every assistant connection.\n'
      "exec ${_shellQuote(orCliPath)} \"\$@\"\n",
    );
    if (!Platform.isWindows) {
      try {
        await Process.run('chmod', ['755', file.path]);
      } catch (_) {
        // The wrapper still works through `sh tools/or` if the mode cannot be set.
      }
    }
    return file;
  }

  Future<File> _writeJson(String name, Map<String, Object?> value) async {
    final file = File(_join(path, name));
    await file.parent.create(recursive: true);
    await file.writeAsString(jsonEncode(value));
    return file;
  }
}

String _join(String root, String relative) =>
    '$root${Platform.pathSeparator}${relative.replaceAll('/', Platform.pathSeparator)}';

String _shellQuote(String value) {
  if (!value.contains(RegExp(r'''[\s'"$`\\!*?()&|;<>]'''))) return value;
  return "'${value.replaceAll("'", "'\\''")}'";
}
