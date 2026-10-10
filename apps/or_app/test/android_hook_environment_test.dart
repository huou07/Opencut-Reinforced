import 'package:flutter_test/flutter_test.dart';

import '../../../packages/or_app_bridge/hook/build.dart'
    show androidCargoEnvironment;

void main() {
  test('Android bridge hook targets the supported API for every ABI', () {
    const installRoot = '/runner/android-ffmpeg/install';
    const androidHome = '/runner/android-sdk';
    const targets = {
      'aarch64-linux-android': ('arm64-v8a', 'aarch64-linux-android'),
      'armv7-linux-androideabi': ('armeabi-v7a', 'armv7a-linux-androideabi'),
      'x86_64-linux-android': ('x86_64', 'x86_64-linux-android'),
    };

    for (final entry in targets.entries) {
      final environment = androidCargoEnvironment(
        targetTriple: entry.key,
        installRoot: installRoot,
        androidHome: androidHome,
        host: 'linux-x86_64',
      );
      final prefix = '$installRoot/${entry.value.$1}';
      final cc =
          '$androidHome/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/${entry.value.$2}26-clang';

      expect(
        environment['CARGO_TARGET_${entry.key.toUpperCase().replaceAll('-', '_')}_LINKER'],
        cc,
      );
      expect(environment['CC_${entry.key.replaceAll('-', '_')}'], cc);
      expect(environment['FFMPEG_DIR'], prefix);
      expect(environment['PKG_CONFIG_PATH'], '$prefix/lib/pkgconfig');
      expect(environment['PKG_CONFIG_ALLOW_CROSS'], '1');
    }
  });
}
