import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/updates/companion_setup.dart';

void main() {
  test('an npm install pairs with its own uninstall', () {
    expect(
      CompanionSetup.removalFor('npm install -g three'),
      'npm uninstall -g three',
    );
    expect(
      CompanionSetup.removalFor('npm i -g puppeteer-lottie'),
      'npm uninstall -g puppeteer-lottie',
    );
    expect(CompanionSetup.removalFor('npm install pupetteer-lottie'), isNull);
    expect(CompanionSetup.removalFor('sudo apt install blender'), isNull);
    expect(CompanionSetup.removalFor(null), isNull);
  });

  test('a command runs the way a terminal would', () {
    const posix = CompanionSetup(isWindows: false);
    expect(posix.invocationFor('npm install puppeteer-lottie'), [
      '/bin/sh',
      '-lc',
      'npm install puppeteer-lottie',
    ]);
    const windows = CompanionSetup(isWindows: true);
    expect(windows.invocationFor('npm install puppeteer-lottie'), [
      'cmd.exe',
      '/c',
      'npm install puppeteer-lottie',
    ]);
  });

  test('output and the exit code come back to the caller', () async {
    if (Platform.isWindows) return;
    final lines = <String>[];
    final result = await const CompanionSetup(isWindows: false)
        .run('echo set-up-line; exit 3', onLine: lines.add);
    expect(result.exitCode, 3);
    expect(result.succeeded, isFalse);
    expect(result.output, contains('set-up-line'));
    expect(lines, contains('set-up-line'));
    expect(result.summary, contains('exit code 3'));
  });
}
