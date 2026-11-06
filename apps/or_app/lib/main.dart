import 'dart:io';

import 'package:flutter/material.dart';
import 'package:or_app_bridge/or_app_bridge.dart' show ExternalLibrary, RustLib;

import 'core_gateway.dart';
import 'dev/pacing_recorder.dart';
import 'design/or_theme.dart';
import 'project/project_file_picker.dart';
import 'project/project_gateway.dart';
import 'project/recent_projects.dart';
import 'project/rust_project_gateway.dart';
import 'rust_core_gateway.dart';
import 'production/editor_settings.dart';
import 'project/export_capability.dart';
import 'shell/app_shell.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  // A bridge that refuses to start used to leave a black window: the process was
  // alive, nothing was painted, and the only clue was one stderr line. Paint the
  // reason instead, so a packaged build can be diagnosed from its own window.
  try {
    await RustLib.init(externalLibrary: _bundledMacosRustLibrary());
  } catch (error, stack) {
    stderr.writeln(
      'Opencut Reinforced could not start its Rust bridge: $error',
    );
    stderr.writeln(stack);
    runApp(OrStartupFailure(message: '$error'));
    return;
  }
  // The render device is opened once per process, so the stored backend
  // preference must reach Rust before the first preview. Failure is not fatal:
  // automatic selection still opens a device.
  try {
    final settings = await EditorSettingsStore().load();
    await RustCoreGateway().setGraphicsPreference(settings.graphicsBackend);
  } catch (error) {
    stderr.writeln('Could not apply the graphics backend preference: $error');
  }
  runApp(const OrApp(gateway: RustCoreGateway()));
  PacingRecorder.install();
}

/// Shown when the native bridge cannot load, instead of an empty window.
class OrStartupFailure extends StatelessWidget {
  const OrStartupFailure({super.key, required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Opencut Reinforced',
      debugShowCheckedModeBanner: false,
      theme: OrTheme.dark,
      home: Scaffold(
        body: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Text(
                'Opencut Reinforced could not start',
                key: ValueKey('startup-failure-title'),
                style: TextStyle(fontSize: 20, fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 12),
              const Text(
                'Its native editing engine did not load, so no project can be '
                'opened safely. The reason is below.',
              ),
              const SizedBox(height: 16),
              SelectableText(
                message,
                key: const ValueKey('startup-failure-message'),
                style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

ExternalLibrary? _bundledMacosRustLibrary() {
  if (!Platform.isMacOS) return null;
  final contents = Directory(Platform.resolvedExecutable).parent.parent;
  final path =
      '${contents.path}${Platform.pathSeparator}'
      'Frameworks${Platform.pathSeparator}'
      'or_app_bridge.framework${Platform.pathSeparator}'
      'Versions${Platform.pathSeparator}A${Platform.pathSeparator}'
      'or_app_bridge';
  return File(path).existsSync() ? ExternalLibrary.open(path) : null;
}

class OrApp extends StatelessWidget {
  const OrApp({
    super.key,
    required this.gateway,
    this.projectGateway,
    this.projectFilePicker,
    this.recentProjectStore,
    this.audioCleanupDirectory,
    this.editorSettingsStore,
    this.exportCapabilityCache,
  });

  final CoreGateway gateway;
  final ProjectGateway? projectGateway;
  final ProjectFilePicker? projectFilePicker;
  final RecentProjectStore? recentProjectStore;

  /// Where cleaned audio is written. Injectable so a test does not need a plugin.
  final Future<Directory> Function()? audioCleanupDirectory;

  /// Where editor preferences live. Injectable so a test controls which mode
  /// the export dialog and the editor open in.
  final EditorSettingsStore? editorSettingsStore;

  /// What this build can export. Injectable so a test can describe a package
  /// with no H.264 encoder without a real engine.
  final ExportCapabilityCache? exportCapabilityCache;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Opencut Reinforced',
      debugShowCheckedModeBanner: false,
      theme: OrTheme.dark,
      home: AppShell(
        gateway: gateway,
        projectGateway: projectGateway ?? const RustProjectGateway(),
        projectFilePicker:
            projectFilePicker ?? createDefaultProjectFilePicker(),
        recentProjectStore:
            recentProjectStore ?? PersistentRecentProjectStore(),
        audioCleanupDirectory: audioCleanupDirectory,
        editorSettingsStore: editorSettingsStore,
        exportCapabilityCache: exportCapabilityCache,
      ),
    );
  }
}
