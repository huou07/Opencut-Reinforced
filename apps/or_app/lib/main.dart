import 'dart:io';

import 'package:flutter/material.dart';
import 'package:or_app_bridge/or_app_bridge.dart' show RustLib;

import 'core_gateway.dart';
import 'design/or_theme.dart';
import 'project/project_file_picker.dart';
import 'project/project_gateway.dart';
import 'project/recent_projects.dart';
import 'project/rust_project_gateway.dart';
import 'rust_core_gateway.dart';
import 'shell/app_shell.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const OrApp(gateway: RustCoreGateway()));
}

class OrApp extends StatelessWidget {
  const OrApp({
    super.key,
    required this.gateway,
    this.projectGateway,
    this.projectFilePicker,
    this.recentProjectStore,
    this.audioCleanupDirectory,
  });

  final CoreGateway gateway;
  final ProjectGateway? projectGateway;
  final ProjectFilePicker? projectFilePicker;
  final RecentProjectStore? recentProjectStore;

  /// Where cleaned audio is written. Injectable so a test does not need a plugin.
  final Future<Directory> Function()? audioCleanupDirectory;

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
      ),
    );
  }
}
