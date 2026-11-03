import 'dart:io';

import '../support/host_platform.dart';
import 'github_release.dart';

/// What happened when a downloaded artifact was handed to the system.
class UpdateInstallResult {
  const UpdateInstallResult({
    required this.message,
    this.installed = false,
    this.restartRequired = false,
  });

  /// One sentence the panel can show without interpretation.
  final String message;

  /// The new build is already in place and only needs the app to restart.
  final bool installed;

  /// The app must quit (and, on Windows, let the installer finish) before the
  /// new build runs.
  final bool restartRequired;
}

/// Runs one command and returns its exit code plus output.
typedef ProcessRunner = Future<ProcessResult> Function(
  String program,
  List<String> arguments,
);

/// Starts a program that outlives this one, e.g. a Windows installer.
typedef DetachedStarter = Future<void> Function(
  String program,
  List<String> arguments,
);

Future<ProcessResult> _runSystem(String program, List<String> arguments) =>
    Process.run(program, arguments);

Future<void> _startDetached(String program, List<String> arguments) async {
  await Process.start(program, arguments, mode: ProcessStartMode.detached);
}

/// Installs a downloaded release artifact into the place this host runs apps
/// from, and restarts the app into it.
///
/// One implementation covers every platform: the artifact says what it is, and
/// this class decides how that platform installs it. A future platform adds one
/// branch here instead of a second updater.
class UpdateInstaller {
  UpdateInstaller({
    HostPlatform? platform,
    ProcessRunner runner = _runSystem,
    DetachedStarter startDetached = _startDetached,
    void Function(int code) exitApp = exit,
  }) : _host = platform ?? HostPlatform.current(),
       _run = runner,
       _start = startDetached,
       _exit = exitApp;

  final HostPlatform _host;
  final ProcessRunner _run;
  final DetachedStarter _start;
  final void Function(int code) _exit;

  /// The bundle the update just installed, so quitting can open it again.
  String? _relaunchPath;

  /// Quits this process and, where the platform needs a hand, opens the new
  /// build. Windows is handled by the installer's own restart flag; macOS has
  /// no such flag, so the bundle just copied into place is opened here.
  Future<void> quit() async {
    final relaunch = _relaunchPath;
    if (relaunch != null) {
      try {
        await _start('open', [relaunch]);
      } on Object {
        // The new build is on disk either way; the user can open it.
      }
    }
    _exit(0);
  }

  /// Installs [artifact], returning what the user still has to do.
  Future<UpdateInstallResult> install(File artifact) async {
    final name = artifact.path.toLowerCase();
    if (name.endsWith('.dmg')) return _installOnMac(artifact);
    if (name.endsWith('.exe') || name.endsWith('.msi')) {
      return _installOnWindows(artifact);
    }
    if (name.endsWith('.deb')) {
      return UpdateInstallResult(
        message:
            'The package is ready. Your system asks for permission, so finish '
            'the install from ${artifact.parent.path} with your package '
            'manager; the app then updates itself on next launch.',
      );
    }
    throw const GithubReleaseException(
      'This download is not an installer this build can run.',
    );
  }

  /// Replaces the app bundle with the one inside the disk image.
  Future<UpdateInstallResult> _installOnMac(File artifact) async {
    final mount = await Directory.systemTemp.createTemp('or-update-');
    try {
      final attach = await _run('hdiutil', [
        'attach',
        '-nobrowse',
        '-readonly',
        '-mountpoint',
        mount.path,
        artifact.path,
      ]);
      if (attach.exitCode != 0) {
        throw const GithubReleaseException(
          'The disk image could not be opened. Download it again.',
        );
      }
      final bundle = await _appBundleIn(mount);
      if (bundle == null) {
        throw const GithubReleaseException(
          'The disk image does not contain an application bundle.',
        );
      }
      final destination = Directory(
        '${_applicationsRoot()}/${_basename(bundle.path)}',
      );
      await _removeIfPresent(destination);
      final copy = await _run('ditto', [bundle.path, destination.path]);
      if (copy.exitCode != 0) {
        throw GithubReleaseException(
          'The new version could not be copied to ${destination.parent.path}. '
          'Move the app there yourself, then reopen it.',
        );
      }
      _relaunchPath = destination.path;
      return UpdateInstallResult(
        message: 'Version installed. Restart to finish.',
        installed: true,
        restartRequired: true,
      );
    } finally {
      await _run('hdiutil', ['detach', mount.path, '-quiet']);
      await mount.delete(recursive: true).catchError((_) => mount);
    }
  }

  /// Hands the installer to Windows, which replaces the files after this
  /// process quits and starts the new build itself.
  Future<UpdateInstallResult> _installOnWindows(File artifact) async {
    final lower = artifact.path.toLowerCase();
    final (program, arguments) = lower.endsWith('.msi')
        ? ('msiexec', ['/i', artifact.path, '/passive', '/norestart'])
        : (
            artifact.path,
            const [
              '/SILENT',
              '/CLOSEAPPLICATIONS',
              '/RESTARTAPPLICATIONS',
              '/NORESTART',
            ],
          );
    if (lower.endsWith('.exe')) {
      await _start(program, arguments);
      return const UpdateInstallResult(
        message:
            'The installer is running. This window closes now and the new '
            'version opens when it finishes.',
        installed: true,
        restartRequired: true,
      );
    }
    final result = await _run(program, arguments);
    if (result.exitCode != 0) {
      throw const GithubReleaseException(
        'Windows could not start the installer. Open it from the download '
        'folder instead.',
      );
    }
    return const UpdateInstallResult(
      message: 'Version installed. Restart to finish.',
      installed: true,
      restartRequired: true,
    );
  }

  Future<Directory?> _appBundleIn(Directory mount) async {
    await for (final entry in mount.list()) {
      if (entry is Directory && entry.path.toLowerCase().endsWith('.app')) {
        return entry;
      }
    }
    return null;
  }

  /// `/Applications` when this user may write there, otherwise `~/Applications`.
  String _applicationsRoot() {
    final shared = '/Applications';
    try {
      final directory = Directory(shared);
      if (!directory.existsSync()) directory.createSync();
      final probe = File(
        '$shared/.or-write-${DateTime.now().microsecondsSinceEpoch}',
      );
      probe.writeAsStringSync('');
      probe.deleteSync();
      return shared;
    } on FileSystemException {
      final home = '${_host.homeDirectory}/Applications';
      Directory(home).createSync(recursive: true);
      return home;
    }
  }

  Future<void> _removeIfPresent(Directory directory) async {
    if (await directory.exists()) await directory.delete(recursive: true);
  }

  String _basename(String path) {
    final separator = path.lastIndexOf('/');
    return separator < 0 ? path : path.substring(separator + 1);
  }
}
