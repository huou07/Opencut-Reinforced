import 'package:flutter_test/flutter_test.dart';

import '../../../packages/or_app_bridge/hook/build.dart'
    show androidCargoEnvironment;

void main() {
  test('Android bridge hook configures FFmpeg for every ABI', () {
    const installRoot = '/runner/android-ffmpeg/install';
    const androidHome = '/runner/android-sdk';
    const targets = {
      'aarch64-linux-android': 'arm64-v8a',
      'armv7-linux-androideabi': 'armeabi-v7a',
      'x86_64-linux-android': 'x86_64',
    };

    for (final entry in targets.entries) {
      final environment = androidCargoEnvironment(
        targetTriple: entry.key,
        installRoot: installRoot,
        androidHome: androidHome,
        host: 'linux-x86_64',
      );
      final prefix = '$installRoot/${entry.value}';

      expect(environment['FFMPEG_DIR'], prefix);
      expect(environment['PKG_CONFIG_PATH'], '$prefix/lib/pkgconfig');
      expect(environment['PKG_CONFIG_ALLOW_CROSS'], '1');
    }
  });
}
