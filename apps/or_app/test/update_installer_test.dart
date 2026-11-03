import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/support/host_platform.dart';
import 'package:or_app/updates/github_release.dart';
import 'package:or_app/updates/update_installer.dart';

HostPlatform _host({
  bool windows = false,
  bool macos = false,
  bool linux = false,
  String home = '/Users/test',
}) => HostPlatform(
  isWindows: windows,
  isMacOS: macos,
  isLinux: linux,
  environment: {'HOME': home},
  executablePath: '/tmp/or_app',
);

void main() {
  test(
    'a Debian package explains the permission step instead of faking it',
    () async {
      final installer = UpdateInstaller(
        platform: _host(linux: true),
        exitApp: (_) {},
      );
      final result = await installer.install(File('/tmp/pkg.deb'));
      expect(result.installed, isFalse);
      expect(result.restartRequired, isFalse);
      expect(result.message, contains('package manager'));
    },
  );

  test(
    'a Windows setup program runs silently and asks for a restart',
    () async {
      final calls = <String>[];
      final installer = UpdateInstaller(
        platform: _host(windows: true),
        runner: (program, arguments) async {
          calls.add('$program ${arguments.join(' ')}');
          return ProcessResult(0, 0, '', '');
        },
        startDetached: (program, arguments) async =>
            calls.add('spawn $program ${arguments.join(' ')}'),
        exitApp: (_) {},
      );
      final result = await installer.install(File(r'C:\Downloads\setup.exe'));
      expect(result.installed, isTrue);
      expect(result.restartRequired, isTrue);
      expect(calls.single, contains('spawn'));
      expect(calls.single, contains('/SILENT'));
      expect(calls.single, contains('/CLOSEAPPLICATIONS'));
    },
  );

  test('an image without an application bundle is refused', () async {
    final calls = <String>[];
    final installer = UpdateInstaller(
      platform: _host(macos: true),
      runner: (program, arguments) async {
        calls.add('$program ${arguments.first}');
        return ProcessResult(0, 0, '', '');
      },
      startDetached: (_, _) async {},
      exitApp: (_) {},
    );
    await expectLater(
      installer.install(File('/tmp/Opencut.dmg')),
      throwsA(
        isA<GithubReleaseException>().having(
          (error) => error.message,
          'message',
          contains('application bundle'),
        ),
      ),
    );
    expect(calls.first, startsWith('hdiutil attach'));
    expect(calls.last, startsWith('hdiutil detach'));
  });

  test('an unknown artifact is not run', () async {
    final installer = UpdateInstaller(
      platform: _host(macos: true),
      exitApp: (_) {},
    );
    await expectLater(
      installer.install(File('/tmp/notes.txt')),
      throwsA(isA<GithubReleaseException>()),
    );
  });

  test('quitting after a disk image install opens the new bundle', () async {
    final started = <String>[];
    var exited = -1;
    final home = await Directory.systemTemp.createTemp('or-home-');
    addTearDown(() => home.delete(recursive: true));
    final installer = UpdateInstaller(
      platform: _host(macos: true, home: home.path),
      runner: (program, arguments) async {
        if (program == 'hdiutil' && arguments.first == 'attach') {
          final mount = arguments[arguments.indexOf('-mountpoint') + 1];
          Directory('$mount/Opencut Reinforced.app').createSync();
        }
        return ProcessResult(0, 0, '', '');
      },
      startDetached: (program, arguments) async {
        started.add('$program ${arguments.join(' ')}');
      },
      exitApp: (code) => exited = code,
    );
    final result = await installer.install(
      File('${Directory.systemTemp.path}/Opencut.dmg'),
    );
    expect(result.installed, isTrue);
    expect(result.restartRequired, isTrue);
    await installer.quit();
    expect(exited, 0);
    expect(started, hasLength(1));
    expect(started.single, startsWith('open '));
    expect(started.single, endsWith('Opencut Reinforced.app'));
  });
}
