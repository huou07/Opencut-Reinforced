import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:flutter_rust_bridge_hooks/flutter_rust_bridge_hooks.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    await FlutterRustBridgeNativeAssetsBuilder(
      cratePath: '../../crates/or_app_bridge',
      extraCargoEnvironmentVariables: {
        ..._macOsCargoEnvironment(),
        ..._targetCargoEnvironment(_rustTargetTriple(input.config.code)),
      },
    ).run(input: input, output: output);
  });
}

String? _rustTargetTriple(CodeConfig code) {
  if (code.targetOS != OS.android) return null;
  return switch (code.targetArchitecture) {
    Architecture.arm64 => 'aarch64-linux-android',
    Architecture.arm => 'armv7-linux-androideabi',
    Architecture.x64 => 'x86_64-linux-android',
    _ => null,
  };
}

Map<String, String> _targetCargoEnvironment(String? targetTriple) {
  const androidTargets = {
    'aarch64-linux-android': ('arm64-v8a', 'aarch64-linux-android'),
    'armv7-linux-androideabi': ('armeabi-v7a', 'armv7a-linux-androideabi'),
    'x86_64-linux-android': ('x86_64', 'x86_64-linux-android'),
  };
  if (targetTriple == null) return const {};
  final target = androidTargets[targetTriple];
  final installRoot = Platform.environment['OR_ANDROID_FFMPEG_INSTALL_ROOT'];
  final androidHome = Platform.environment['ANDROID_HOME'];
  if (target == null || installRoot == null || androidHome == null) {
    return const {};
  }
  final host = Platform.isLinux
      ? 'linux-x86_64'
      : Platform.isMacOS
      ? 'darwin-x86_64'
      : Platform.isWindows
      ? 'windows-x86_64'
      : null;
  if (host == null) return const {};

  final toolchain =
      '$androidHome/ndk/28.2.13676358/toolchains/llvm/prebuilt/$host';
  final cc = '$toolchain/bin/${target.$2}21-clang';
  final targetSuffix = targetTriple.replaceAll('-', '_');
  final prefix = '$installRoot/${target.$1}';
  return {
    'CARGO_TARGET_${targetSuffix.toUpperCase()}_LINKER': cc,
    'CC_$targetSuffix': cc,
    'AR_$targetSuffix': '$toolchain/bin/llvm-ar',
    'BINDGEN_EXTRA_CLANG_ARGS_$targetSuffix':
        '--target=$targetTriple --sysroot=$toolchain/sysroot',
    'FFMPEG_DIR': prefix,
    'PKG_CONFIG_PATH': '$prefix/lib/pkgconfig',
    'PKG_CONFIG_ALLOW_CROSS': '1',
  };
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
