import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:flutter_rust_bridge_hooks/flutter_rust_bridge_hooks.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    final targetTriple = _rustTargetTriple(input.config.code);
    final ffmpegInstallRoot = targetTriple == null
        ? null
        : input.userDefines.path('android_ffmpeg_install_root');
    final macOsFfmpegInstallUri =
        Platform.isMacOS && input.config.code.targetOS == OS.macOS
        ? input.userDefines.path('macos_ffmpeg_install_root')
        : null;
    String? macOsFfmpegInstallRoot;
    if (macOsFfmpegInstallUri != null) {
      final ffmpegDirectory = Directory.fromUri(macOsFfmpegInstallUri);
      if (await ffmpegDirectory.exists()) {
        macOsFfmpegInstallRoot = await ffmpegDirectory.resolveSymbolicLinks();
        output.dependencies.add(macOsFfmpegInstallUri);
        output.dependencies.add(Directory(macOsFfmpegInstallRoot).uri);
      }
    }
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
      // The environment alone is not enough on a host whose shared Cargo config
      // force-pins FFMPEG_DIR, PKG_CONFIG_PATH and a native search path to a
      // *different* FFmpeg build: `force = true` beats the environment, and the
      // hook keeps the host's HOME. Measured 2026-10-18 on the Debian
      // workstation: the locally built libor_app_bridge.so asked for
      // libavcodec.so.62 (the Linux soname) while the staged Android prefix
      // provides an unversioned SONAME, so the APK could not load on any guest.
      // Command-line `--config` outranks the config file, so the Android pin is
      // stated there too; on CI, which has no such host config, these are the
      // same values the environment already carries.
      extraCargoBuildArgs: [
        ..._androidCargoArguments(
          targetTriple,
          ffmpegInstallRoot?.toFilePath(),
        ),
        ..._macOsCargoArguments(macOsFfmpegInstallRoot),
      ],
      extraCargoEnvironmentVariables: {
        ..._macOsCargoEnvironment(macOsFfmpegInstallRoot),
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
    // `RUSTFLAGS` is the only effective way to drop the host config's native
    // search path for this target: cargo *concatenates* the `build.rustflags`
    // arrays from every config source, so a laptop whose shared config points
    // `-L native=` at another FFmpeg build keeps that path ahead of the staged
    // Android one and the linker records the wrong soname. The environment
    // replaces the config value instead. Measured 2026-10-18: with this set the
    // bridge required the unversioned Android sonames and the build log had no
    // reference to the desktop prefix; without it, every object file was linked
    // with the desktop path first.
    'RUSTFLAGS': '-L native=$prefix/lib',
    'CARGO_TARGET_${targetSuffix.toUpperCase()}_LINKER': cc,
    'CC_$targetSuffix': cc,
    'AR_$targetSuffix': '$toolchain/bin/llvm-ar',
    // `-I` on the staged prefix's own headers is stated here as well as through
    // FFMPEG_DIR: measured 2026-10-18, a host whose Cargo config forced a
    // different FFmpeg made bindgen resolve the wrapper without an include path
    // and fail with "'libavutil/adler32.h' file not found". bindgen takes this
    // variable directly, so the header path does not depend on how the binding
    // crate happens to pass its own includes.
    'BINDGEN_EXTRA_CLANG_ARGS_$targetSuffix':
        '--target=$targetTriple --sysroot=$toolchain/sysroot -I$prefix/include',
    'FFMPEG_DIR': prefix,
    'PKG_CONFIG_PATH': '$prefix/lib/pkgconfig',
    'PKG_CONFIG_ALLOW_CROSS': '1',
  };
}

/// Cargo `--config` overrides that outrank a host's forced FFmpeg pin.
///
/// Returns an empty list for non-Android targets, so host and desktop builds are
/// unchanged.
List<String> _androidCargoArguments(String? targetTriple, String? installRoot) {
  if (targetTriple == null || installRoot == null) return const <String>[];
  final abi = switch (targetTriple) {
    'aarch64-linux-android' => 'arm64-v8a',
    'armv7-linux-androideabi' => 'armeabi-v7a',
    'x86_64-linux-android' => 'x86_64',
    _ => null,
  };
  if (abi == null) return const <String>[];
  final prefix = '$installRoot/$abi';
  // `cargo --config KEY=VALUE` cannot set an inline table, so `[env]` entries
  // are stated as their two dotted keys. Measured 2026-10-18: the inline-table
  // form is rejected with "sets a value to an inline table, which is not
  // accepted".
  return <String>[
    '--config',
    'env.FFMPEG_DIR.value="$prefix"',
    '--config',
    'env.FFMPEG_DIR.force=true',
    '--config',
    'env.PKG_CONFIG_PATH.value="$prefix/lib/pkgconfig"',
    '--config',
    'env.PKG_CONFIG_PATH.force=true',
    '--config',
    'env.PKG_CONFIG_ALLOW_CROSS.value="1"',
    '--config',
    'env.PKG_CONFIG_ALLOW_CROSS.force=true',
  ];
}

Map<String, String> _macOsCargoEnvironment(String? ffmpegRoot) {
  if (!Platform.isMacOS) return const {};

  const tools = '/Library/Developer/CommandLineTools';
  const clang = '$tools/usr/bin/clang';
  const sdk = '$tools/SDKs/MacOSX.sdk';
  if (!File(clang).existsSync() || !Directory(sdk).existsSync())
    return const {};

  final pkgConfigPath = ffmpegRoot == null ? null : '$ffmpegRoot/lib/pkgconfig';
  return {
    'SDKROOT': sdk,
    'CC': clang,
    'AR': '$tools/usr/bin/ar',
    'RANLIB': '$tools/usr/bin/ranlib',
    'CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER': clang,
    'CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER': clang,
    if (ffmpegRoot != null) 'FFMPEG_DIR': ffmpegRoot,
    if (pkgConfigPath != null) 'PKG_CONFIG_PATH': pkgConfigPath,
    if (pkgConfigPath != null) 'PKG_CONFIG_LIBDIR': pkgConfigPath,
    if (ffmpegRoot != null)
      'CARGO_ENCODED_RUSTFLAGS': [
        '-C',
        'link-arg=-Wl,-rpath,$ffmpegRoot/lib',
      ].join('\x1f'),
  };
}

List<String> _macOsCargoArguments(String? ffmpegRoot) {
  if (ffmpegRoot == null) return const [];

  final pkgConfigPath = '$ffmpegRoot/lib/pkgconfig';
  return [
    '--config',
    'env.FFMPEG_DIR.value=${_tomlString(ffmpegRoot)}',
    '--config',
    'env.FFMPEG_DIR.force=true',
    '--config',
    'env.PKG_CONFIG_PATH.value=${_tomlString(pkgConfigPath)}',
    '--config',
    'env.PKG_CONFIG_PATH.force=true',
    '--config',
    'env.PKG_CONFIG_LIBDIR.value=${_tomlString(pkgConfigPath)}',
    '--config',
    'env.PKG_CONFIG_LIBDIR.force=true',
  ];
}

String _tomlString(String value) =>
    '"${value.replaceAll(r'\', r'\\').replaceAll('"', r'\"')}"';
