import 'dart:io';

/// The one platform seam.
///
/// Every operating-system rule the product needs — where a user's home is, how
/// a `PATH` is split, which suffix makes a file executable, which program opens
/// a folder — lives here instead of being re-derived at each call site. A new
/// platform, or a fix for one, therefore lands in one file and every feature
/// inherits it: the assistant, the local speech runtime, the voice servers and
/// the resources panel all ask the same object.
///
/// The fields are injectable so a test can describe a Windows host while it runs
/// on macOS, which is what keeps "works on Windows" from being a claim nobody
/// can check. Filesystem answers still come from the real filesystem; only the
/// *rules* are simulated.
class HostPlatform {
  const HostPlatform({
    required this.isWindows,
    required this.isMacOS,
    required this.isLinux,
    required this.environment,
    required this.executablePath,
  });

  /// The host the app is actually running on.
  factory HostPlatform.current({
    Map<String, String>? environment,
    String? executablePath,
  }) {
    return HostPlatform(
      isWindows: Platform.isWindows,
      isMacOS: Platform.isMacOS,
      isLinux: Platform.isLinux,
      environment: environment ?? Platform.environment,
      executablePath: executablePath ?? Platform.resolvedExecutable,
    );
  }

  final bool isWindows;
  final bool isMacOS;
  final bool isLinux;
  final Map<String, String> environment;

  /// The running application binary, used to find the runtime staged beside it.
  final String executablePath;

  String get name {
    if (isWindows) return 'Windows';
    if (isMacOS) return 'macOS';
    if (isLinux) return 'Linux';
    return Platform.operatingSystem;
  }

  /// The user's home directory. Windows publishes `USERPROFILE`; POSIX systems
  /// publish `HOME`. No caller should have to know that.
  String get homeDirectory {
    final home = isWindows
        ? (environment['USERPROFILE'] ?? environment['HOME'])
        : (environment['HOME'] ?? environment['USERPROFILE']);
    return home ?? '';
  }

  /// `PATH` uses `;` on Windows and `:` everywhere else.
  String get pathListSeparator => isWindows ? ';' : ':';

  String get pathSeparator => isWindows ? '\\' : '/';

  /// Suffix that makes a file executable in this host's convention.
  String get executableSuffix => isWindows ? '.exe' : '';

  /// Suffixes tried for a bare program name, in preference order. Windows also
  /// runs `.cmd` and `.bat` shims, which is how npm-installed CLIs appear.
  List<String> get executableSuffixes =>
      isWindows ? const ['', '.exe', '.cmd', '.bat', '.com'] : const [''];

  /// Directories added to `PATH` by common per-user installers: `uv` and
  /// `pip --user` both use `~/.local/bin`, including on Windows.
  /// The per-user folder the app keeps its own data in.
  ///
  /// One rule for every platform, and the same rule the Rust bridge uses to
  /// find a repaired speech runtime, so a folder the setup surface fills is a
  /// folder the editing core reads without either side guessing.
  String get appDataDirectory {
    final home = homeDirectory;
    if (isWindows) {
      final local = environment['LOCALAPPDATA'];
      if (local != null && local.isNotEmpty) {
        return join(local, 'Opencut Reinforced');
      }
      return home.isEmpty
          ? ''
          : join(home, 'AppData', 'Local', 'Opencut Reinforced');
    }
    if (isMacOS) {
      if (home.isEmpty) return '';
      return join(home, 'Library', 'Application Support', 'Opencut Reinforced');
    }
    final data = environment['XDG_DATA_HOME'];
    final root = data != null && data.isNotEmpty
        ? data
        : (home.isEmpty ? '' : join(home, '.local', 'share'));
    return root.isEmpty ? '' : join(root, 'opencut-reinforced');
  }

  /// Where a speech runtime installed outside the application package lives.
  String get speechRuntimeDirectory {
    final data = appDataDirectory;
    return data.isEmpty ? '' : join(data, 'speech-runtime');
  }

  List<String> get userBinDirectories {
    final home = homeDirectory;
    if (home.isEmpty) return const [];
    return [join(home, '.local', 'bin')];
  }

  /// The directories this host searches for a bare program name, in order:
  /// the inherited `PATH`, then the per-user installer directories.
  List<String> get searchDirectories {
    final seen = <String>{};
    final result = <String>[];
    for (final entry in pathDirectories) {
      if (entry.isEmpty || !seen.add(entry)) continue;
      result.add(entry);
    }
    for (final entry in userBinDirectories) {
      if (!seen.add(entry)) continue;
      result.add(entry);
    }
    return result;
  }

  List<String> get pathDirectories {
    final raw = environment['PATH'] ?? environment['Path'] ?? '';
    return raw
        .split(RegExp('[${RegExp.escape(pathListSeparator)}]'))
        .toList(growable: false);
  }

  String join(String first, [String? second, String? third, String? fourth]) {
    final parts = <String>[first, ?second, ?third, ?fourth];
    final buffer = StringBuffer(parts.first);
    for (final part in parts.skip(1)) {
      final current = buffer.toString();
      if (current.isEmpty) {
        buffer.write(part);
      } else if (current.endsWith('/') || current.endsWith('\\')) {
        buffer.write(part);
      } else {
        buffer.write(pathSeparator);
        buffer.write(part);
      }
    }
    return buffer.toString();
  }

  /// Splits a `/`-separated relative path into host separators.
  String fromPosix(String relative) => relative.replaceAll('/', pathSeparator);

  /// Resolves a configured executable path.
  ///
  /// A settings field may name a file the user picked (absolute, with its
  /// suffix) or a bare program (typed by hand). Both must work without the
  /// caller knowing which platform it is on.
  String? resolveExecutable(String configured) {
    final value = configured.trim();
    if (value.isEmpty) return null;
    final direct = File(value);
    if (direct.existsSync()) return direct.path;
    if (isWindows && !_hasSuffix(value)) {
      for (final suffix in executableSuffixes) {
        if (suffix.isEmpty) continue;
        final candidate = File('$value$suffix');
        if (candidate.existsSync()) return candidate.path;
      }
    }
    if (!value.contains('/') && !value.contains('\\')) {
      return findExecutable(value);
    }
    return null;
  }

  /// Finds [name] as a program the host can actually execute.
  ///
  /// [extraDirectories] are searched before `PATH`, so a packaged runtime that
  /// sits beside the application wins over an unrelated copy on `PATH`.
  String? findExecutable(
    String name, {
    Iterable<String> extraDirectories = const [],
  }) {
    final directories = <String>[...extraDirectories, ...searchDirectories];
    for (final directory in directories) {
      if (directory.isEmpty) continue;
      for (final suffix in executableSuffixes) {
        final candidate = File(join(directory, '$name$suffix'));
        if (candidate.existsSync()) return candidate.path;
      }
    }
    return null;
  }

  /// The directory holding the running application binary.
  Directory get executableDirectory => File(executablePath).parent;

  /// The program and arguments that hand [target] to the host.
  ///
  /// Public so a test can read the decision on any OS. Windows has no
  /// `xdg-open`, and the usual stand-in — `cmd /c start "" <target>` — makes
  /// the command interpreter parse the path, which is how an update ends in
  /// the system's "how do you want to open this file?" dialog instead of
  /// running: `start` reads the first quoted token as a window title, and a
  /// program that goes through the shell loses its own quoting. Executables are
  /// therefore started directly, and everything else goes to the shell's
  /// protocol handler, which takes the path as an argument and parses nothing.
  ({String program, List<String> arguments}) openCommandFor(String target) {
    if (isMacOS) return (program: 'open', arguments: <String>[target]);
    if (isLinux) return (program: 'xdg-open', arguments: <String>[target]);
    final lower = target.toLowerCase();
    const programSuffixes = <String>['.exe', '.msi', '.com', '.bat', '.cmd'];
    final runsItself = programSuffixes.any(lower.endsWith);
    if (runsItself) return (program: target, arguments: const <String>[]);
    return (
      program: 'rundll32',
      arguments: <String>['url.dll,FileProtocolHandler', target],
    );
  }

  /// Opens a file, folder or HTTPS link with the host's default handler.
  Future<void> openExternal(String target) async {
    final (:program, :arguments) = openCommandFor(target);
    final result = await Process.run(program, arguments);
    if (result.exitCode != 0) {
      throw ProcessException(
        program,
        arguments,
        'the host could not open the item',
        result.exitCode,
      );
    }
  }

  /// How this host names the action that reveals a file in its file manager.
  String get revealLabel {
    if (isMacOS) return 'Show in Finder';
    if (isWindows) return 'Show in Explorer';
    return 'Show in Files';
  }

  /// Reveals [path] in the host's file manager.
  ///
  /// Every desktop has this; only the program differs. Windows selects the file
  /// itself, macOS selects it in Finder, and the freedesktop hosts can only open
  /// the containing folder, which is why the caller keeps a clipboard fallback.
  Future<void> revealFile(String path) async {
    if (isWindows) {
      // explorer.exe reports 1 even when it opened the window.
      await Process.run('explorer.exe', ['/select,$path']);
      return;
    }
    if (isMacOS) {
      final result = await Process.run('open', ['-R', path]);
      if (result.exitCode != 0) {
        throw ProcessException(
          'open',
          ['-R', path],
          'the host could not reveal the file',
          result.exitCode,
        );
      }
      return;
    }
    final parent = File(path).parent.path;
    final result = await Process.run('xdg-open', [parent]);
    if (result.exitCode != 0) {
      throw ProcessException(
        'xdg-open',
        [parent],
        'the host could not open the containing folder',
        result.exitCode,
      );
    }
  }

  bool _hasSuffix(String path) {
    final slash = path.lastIndexOf(RegExp(r'[\\/]'));
    final name = slash == -1 ? path : path.substring(slash + 1);
    final dot = name.lastIndexOf('.');
    return dot > 0;
  }
}
