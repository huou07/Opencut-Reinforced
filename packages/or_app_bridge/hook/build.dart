import 'package:flutter_rust_bridge_hooks/flutter_rust_bridge_hooks.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    if (input.config.linkingEnabled) {
      await const FlutterRustBridgeNativeAssetsBuilder(
        cratePath: '../../crates/or_app_bridge',
      ).run(input: input, output: output);
    }
  });
}
