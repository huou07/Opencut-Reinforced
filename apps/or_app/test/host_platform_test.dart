import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:path/path.dart' as p;
import 'package:or_app/support/host_platform.dart';

void main() {
  test('the current host reports exactly one platform family', () {
    final host = HostPlatform.current();
    expect(
      [host.isWindows, host.isMacOS, host.isLinux].where((value) => value),
      hasLength(1),
    );
    expect(host.name, isNotEmpty);
  });

  test('a Windows host reads USERPROFILE and uses the Windows rules', () {
    const host = HostPlatform(
      isWindows: true,
      isMacOS: false,
      isLinux: false,
      environment: {
        'USERPROFILE': r'C:\Users\ana',
        'LOCALAPPDATA': r'C:\Users\ana\AppData\Local',
        'APPDATA': r'C:\Users\ana\AppData\Roaming',
        'PATH': r'C:\Windows;C:\Tools',
      },
      executablePath: r'C:\Program Files\Opencut Reinforced\or_app.exe',
    );
    expect(host.homeDirectory, r'C:\Users\ana');
    expect(host.pathListSeparator, ';');
    expect(host.executableSuffix, '.exe');
    expect(host.pathDirectories, [r'C:\Windows', r'C:\Tools']);
    expect(
      host.userBinDirectories.first,
      host.join(r'C:\Users\ana', '.local', 'bin'),
    );
    expect(
      host.appDataDirectory,
      host.join(r'C:\Users\ana\AppData\Local', 'Opencut Reinforced'),
    );
    expect(
      host.speechRuntimeDirectory,
      host.join(
        r'C:\Users\ana\AppData\Local',
        'Opencut Reinforced',
        'speech-runtime',
      ),
    );
  });

  test('a POSIX host reads HOME and keeps the colon separator', () {
    const mac = HostPlatform(
      isWindows: false,
      isMacOS: true,
      isLinux: false,
      environment: {'HOME': '/Users/ana', 'PATH': '/usr/bin:/bin'},
      executablePath:
          '/Applications/Opencut Reinforced.app/Contents/MacOS/or_app',
    );
    expect(mac.homeDirectory, '/Users/ana');
    expect(mac.pathListSeparator, ':');
    expect(mac.executableSuffix, '');
    expect(mac.pathDirectories, ['/usr/bin', '/bin']);
    expect(
      mac.appDataDirectory,
      '/Users/ana/Library/Application Support/Opencut Reinforced',
    );

    const linux = HostPlatform(
      isWindows: false,
      isMacOS: false,
      isLinux: true,
      environment: {'HOME': '/home/ana', 'XDG_DATA_HOME': '/home/ana/.data'},
      executablePath: '/usr/bin/or_app',
    );
    expect(
      linux.speechRuntimeDirectory,
      '/home/ana/.data/opencut-reinforced/speech-runtime',
    );
  });

  test('a bare name resolves through the host PATH', () async {
    final root = await Directory.systemTemp.createTemp('or-host-path-');
    addTearDown(() => root.delete(recursive: true));
    final first = Directory('${root.path}/first')..createSync();
    final second = Directory('${root.path}/second')..createSync();
    // The real rules of the running host: a simulated POSIX host on Windows
    // splits a `C:\...` PATH entry at its drive colon.
    final isWindows = Platform.isWindows;
    final executable = File(p.join(second.path, isWindows ? 'dsh.exe' : 'dsh'))
      ..writeAsStringSync('');
    final host = HostPlatform(
      isWindows: isWindows,
      isMacOS: Platform.isMacOS,
      isLinux: Platform.isLinux,
      environment: {
        if (isWindows) 'USERPROFILE': root.path else 'HOME': root.path,
        'PATH': [first.path, second.path].join(isWindows ? ';' : ':'),
      },
      executablePath: p.join(root.path, isWindows ? 'or_app.exe' : 'or_app'),
    );
    expect(host.findExecutable('dsh'), executable.path);
  });

  test('a simulated Windows host parses a semicolon-separated PATH', () {
    const host = HostPlatform(
      isWindows: true,
      isMacOS: false,
      isLinux: false,
      environment: {
        'USERPROFILE': r'C:\Users\ana',
        'PATH': r'C:\Windows;C:\Tools',
      },
      executablePath: r'C:\or\or_app.exe',
    );
    expect(host.pathDirectories, [r'C:\Windows', r'C:\Tools']);
    expect(host.executableSuffixes, contains('.exe'));
  });

  test('an explicit path is accepted with or without its suffix', () async {
    final root = await Directory.systemTemp.createTemp('or-host-explicit-');
    addTearDown(() => root.delete(recursive: true));
    final file = File(p.join(root.path, 'or.exe'))..writeAsStringSync('');
    final host = HostPlatform(
      isWindows: true,
      isMacOS: false,
      isLinux: false,
      environment: {'USERPROFILE': r'C:\Users\ana'},
      executablePath: r'C:\or\or_app.exe',
    );
    expect(host.resolveExecutable(p.join(root.path, 'or')), file.path);
    expect(host.resolveExecutable(file.path), file.path);
    expect(host.resolveExecutable(p.join(root.path, 'missing')), isNull);
  });
}
