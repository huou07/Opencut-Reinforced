import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:flutter_rust_bridge_hooks/flutter_rust_bridge_hooks.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    final targetTriple = _rustTargetTriple(input.config.code);
    final ffmpegInstallRoot = targetTriple == null
        ? null
        : input.userDefines.path('android_ffmpeg_install_root');
    if (targetTriple != null) {
      if (ffmpegInstallRoot == null ||
          !await Directory.fromUri(ffmpegInstallRoot).exists()) {
        throw StateError(
          'Android bridge builds require the staged FFmpeg install at '
          'apps/or_app/android/.ffmpeg-install.',
        );
      }
      output.dependencies.add(ffmpegInstallRoot);
    }

    await FlutterRustBridgeNativeAssetsBuilder(
      cratePath: '../../crates/or_app_bridge',
      extraCargoEnvironmentVariables: {
        ..._macOsCargoEnvironment(),
        ..._targetCargoEnvironment(
          targetTriple,
          ffmpegInstallRoot?.toFilePath(),
        ),
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

Map<String, String> _targetCargoEnvironment(
  String? targetTriple,
  String? installRoot,
) {
  if (targetTriple == null) return const {};
  if (installRoot == null) {
    throw StateError('Android bridge builds require a staged FFmpeg install.');
  }

  final androidHome = Platform.environment['ANDROID_HOME'];
  if (androidHome == null) {
    throw StateError('Android bridge builds require ANDROID_HOME.');
  }
  final host = Platform.isLinux
      ? 'linux-x86_64'
      : Platform.isMacOS
      ? 'darwin-x86_64'
      : Platform.isWindows
      ? 'windows-x86_64'
      : null;
  if (host == null) {
    throw UnsupportedError('Unsupported host for Android bridge builds.');
  }

  return androidCargoEnvironment(
    targetTriple: targetTriple,
    installRoot: installRoot,
    androidHome: androidHome,
    host: host,
  );
}

Map<String, String> androidCargoEnvironment({
  required String targetTriple,
  required String installRoot,
  required String androidHome,
  required String host,
}) {
  const androidTargets = {
    'aarch64-linux-android': ('arm64-v8a', 'aarch64-linux-android'),
    'armv7-linux-androideabi': ('armeabi-v7a', 'armv7a-linux-androideabi'),
    'x86_64-linux-android': ('x86_64', 'x86_64-linux-android'),
  };
  final target = androidTargets[targetTriple];
  if (target == null) {
    throw ArgumentError.value(targetTriple, 'targetTriple');
  }

  final toolchain =
      '$androidHome/ndk/28.2.13676358/toolchains/llvm/prebuilt/$host';
  // CPAL's Android AAudio backend requires the API 26 NDK stubs.
  final cc = '$toolchain/bin/${target.$2}26-clang';
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
