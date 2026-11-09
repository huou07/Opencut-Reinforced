import 'dart:async';
import 'dart:convert';
import 'dart:io';

/// Runs a companion tool's setup or removal command inside the app.
///
/// The catalog states the exact command per platform and the panel shows it so
/// a user can copy it. Running that same string here means "Set up here" can
/// never drift from the written instructions, and the output stays visible in
/// the panel instead of disappearing into a terminal the user never opened.
///
/// Nothing is installed behind the user's back: the caller only reaches this
/// object from a button that names the command it is about to run.
class CompanionSetup {
  const CompanionSetup({this.isWindows});

  /// Simulated for tests; the real host answers through [Platform.isWindows].
  final bool? isWindows;

  bool get _windows => isWindows ?? Platform.isWindows;

  /// The process that runs [command] the way a terminal would.
  ///
  /// A login shell is used on POSIX so a tool installed by Homebrew or npm is
  /// on `PATH` even though the app was started from Finder, and `cmd /c` on
  /// Windows because `npm` is a `.cmd` shim there.
  List<String> invocationFor(String command) => _windows
      ? <String>['cmd.exe', '/c', command]
      : <String>['/bin/sh', '-lc', command];

  /// The command that undoes [installCommand], when this app can know it.
  ///
  /// `npm install -g x` becomes `npm uninstall -g x`. Anything else — a system
  /// package manager, a vendor installer — is left to the tool's own removal
  /// note, because a guessed uninstall is worse than an honest hint.
  static String? removalFor(String? installCommand) {
    final command = installCommand?.trim();
    if (command == null || command.isEmpty) return null;
    final match = RegExp(
      r'^npm\s+(?:install|i|add)\s+-g\s+(.+)$',
      caseSensitive: false,
    ).firstMatch(command);
    if (match == null) return null;
    return 'npm uninstall -g ${match.group(1)!.trim()}';
  }

  /// Runs [command], reporting every output line as it arrives.
  ///
  /// The exit code is the answer: a non-zero code means the machine did not
  /// change, so the caller can re-check the tool and show what it said. The
  /// output is capped so one runaway command cannot grow the panel without
  /// bound, and the tail is what stays visible.
  Future<CompanionSetupResult> run(
    String command, {
    void Function(String line)? onLine,
    Duration timeout = const Duration(minutes: 10),
  }) async {
    final invocation = invocationFor(command);
    final result = <String>[];
    final started = DateTime.now();
    final Process process;
    try {
      process = await Process.start(
        invocation.first,
        invocation.sublist(1),
        runInShell: false,
      );
    } on ProcessException catch (error) {
      final line = '${invocation.first}: ${error.message}';
      onLine?.call(line);
      return CompanionSetupResult(
        command: command,
        exitCode: -1,
        output: <String>[line],
        elapsed: DateTime.now().difference(started),
      );
    }
    void record(String line) {
      if (line.isEmpty) return;
      if (result.length >= _maxLines) result.removeAt(0);
      result.add(line);
      onLine?.call(line);
    }

    final done = Completer<int>();
    final subscriptions = <StreamSubscription<String>>[
      process.stdout
          .transform(utf8.decoder)
          .transform(const LineSplitter())
          .listen(record),
      process.stderr
          .transform(utf8.decoder)
          .transform(const LineSplitter())
          .listen(record),
    ];
    process.exitCode.then((code) {
      if (!done.isCompleted) done.complete(code);
    });
    var exitCode = -1;
    try {
      exitCode = await done.future.timeout(timeout);
    } on TimeoutException {
      process.kill();
      record('The command did not finish within ${timeout.inMinutes} minutes.');
    } finally {
      for (final subscription in subscriptions) {
        await subscription.cancel();
      }
    }
    return CompanionSetupResult(
      command: command,
      exitCode: exitCode,
      output: List<String>.unmodifiable(result),
      elapsed: DateTime.now().difference(started),
    );
  }

  static const int _maxLines = 200;
}

/// What running a setup command produced.
class CompanionSetupResult {
  const CompanionSetupResult({
    required this.command,
    required this.exitCode,
    required this.output,
    required this.elapsed,
  });

  final String command;
  final int exitCode;
  final List<String> output;
  final Duration elapsed;

  bool get succeeded => exitCode == 0;

  /// One sentence the panel can show without the user reading the raw output.
  String get summary {
    if (succeeded) {
      return 'Setup finished in ${elapsed.inSeconds}s.';
    }
    final hint = output.isEmpty ? 'it printed nothing' : output.last;
    return 'Setup failed with exit code $exitCode: $hint';
  }
}
