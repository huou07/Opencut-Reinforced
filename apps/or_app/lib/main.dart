import 'package:flutter/material.dart';
import 'package:or_app_bridge/or_app_bridge.dart' show RustLib;

import 'core_gateway.dart';
import 'design/or_theme.dart';
import 'rust_core_gateway.dart';
import 'shell/app_shell.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const OrApp(gateway: RustCoreGateway()));
}

class OrApp extends StatelessWidget {
  const OrApp({super.key, required this.gateway});

  final CoreGateway gateway;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Opencut Reinforced',
      debugShowCheckedModeBanner: false,
      theme: OrTheme.dark,
      home: AppShell(gateway: gateway),
    );
  }
}
