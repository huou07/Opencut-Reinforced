import 'dart:io';

import 'package:flutter_rust_bridge_hooks/flutter_rust_bridge_hooks.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    await FlutterRustBridgeNativeAssetsBuilder(
      cratePath: '../../crates/or_app_bridge',
      extraCargoEnvironmentVariables: _macOsCargoEnvironment(),
    ).run(input: input, output: output);
  });
}

Map<String, String> _macOsCargoEnvironment() {
  if (!Platform.isMacOS) return const {};

  const tools = '/Library/Developer/CommandLineTools';
  const clang = '$tools/usr/bin/clang';
  const sdk = '$tools/SDKs/MacOSX.sdk';
  if (!File(clang).existsSync() || !Directory(sdk).existsSync())
    return const {};

  return {
    'SDKROOT': sdk,
    'CC': clang,
    'AR': '$tools/usr/bin/ar',
    'RANLIB': '$tools/usr/bin/ranlib',
    'CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER': clang,
    'CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER': clang,
  };
}
