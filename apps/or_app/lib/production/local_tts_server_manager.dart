import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';

import '../support/host_platform.dart';
import 'local_tts_service.dart';

/// One local speech server the app can install and run without a terminal.
///
/// An engine is a *recipe*, not a shell script: a repository to clone plus the
/// argument list that installs and starts it. The manager turns those steps
/// into a real process on every host, so a Windows, Linux or macOS user gets the
/// same feature from the same declaration instead of a per-platform command
/// string that only one shell understands.
class LocalTtsEngine {
  const LocalTtsEngine({
    required this.id,
    required this.label,
    required this.description,
    required this.baseUrl,
    this.repository = '',
    this.repositoryRef = '',
    this.installArguments = const ['sync'],
    this.startArguments = const [],
    this.homepage = '',
  });

  final String id;
  final String label;
  final String description;
  final String baseUrl;

  /// Git URL of the engine. Empty means the engine is already installed and
  /// only [startArguments] are needed, which is the custom-server case.
  final String repository;

  /// Optional branch or tag, cloned shallowly.
  final String repositoryRef;

  /// Arguments for `uv` that prepare the checkout, e.g. `sync`.
  final List<String> installArguments;

  /// Arguments for `uv` that start the server, e.g.
  /// `['run', 'python', 'apps/openai_speech.py']`.
  final List<String> startArguments;

  final String homepage;

  /// Whether the app can install this engine. A custom server cannot be
  /// downloaded but can still be started from a command the user provides.
  bool get installable => repository.isNotEmpty;

  /// Whether the engine has a built-in start command.
  bool get startable => startArguments.isNotEmpty;
}

const localTtsEngines = <LocalTtsEngine>[
  LocalTtsEngine(
    id: 'vieneu',
    label: 'VieNeu-TTS (Vietnamese)',
    description:
        'OpenAI-compatible Vietnamese speech with voice cloning. Downloads the '
        'engine on first use; no account or key.',
    baseUrl: LocalTtsService.defaultBaseUrl,
    homepage: 'https://github.com/pnnbao97/VieNeu-TTS',
    repository: 'https://github.com/pnnbao97/VieNeu-TTS.git',
    installArguments: ['sync'],
    startArguments: ['run', 'python', 'apps/openai_speech.py'],
  ),
  LocalTtsEngine(
    id: 'custom',
    label: 'Custom OpenAI-compatible server',
    description:
        'Any loopback server that answers GET /health and POST /v1/audio/speech. '
        'Choose this for Kokoro, Chatterbox, Piper wrappers or your own engine.',
    baseUrl: LocalTtsService.defaultBaseUrl,
  ),
];

/// Starts, stops and installs the local speech servers OR talks to.
///
/// The engine remains a separate process on loopback, exactly like a server the
/// user started by hand. The manager only removes the terminal step: it runs
/// the engine's documented steps, streams their output into the settings
/// surface, and keeps the child process so Stop really stops it.
class LocalTtsServerManager extends ChangeNotifier {
  /// One server per app session: closing the voiceover dialog must not kill a
  /// server the user started, and reopening it must still be able to stop it.
  static final LocalTtsServerManager instance = LocalTtsServerManager();

  LocalTtsServerManager({
    Future<Directory> Function()? supportDirectory,
    HostPlatform? platform,
    this.logLimit = 120,
  }) : _host = platform ?? HostPlatform.current(),
       // ignore: prefer_initializing_formals
       _supportDirectory = supportDirectory;

  final Future<Directory> Function()? _supportDirectory;
  final HostPlatform _host;
  final int logLimit;

  Process? _process;
  StreamSubscription<String>? _stdout;
  StreamSubscription<String>? _stderr;
  final List<String> _log = [];
  bool _running = false;
  bool _busy = false;
  String? _error;
  String _engineId = 'vieneu';

  bool get running => _running;
  bool get busy => _busy;
  String? get error => _error;
  String get engineId => _engineId;
  List<String> get log => List.unmodifiable(_log);
  String get logTail =>
      _log.skip(_log.length > 20 ? _log.length - 20 : 0).join('\n');

  void selectEngine(String id) {
    if (_engineId == id) return;
    _engineId = id;
    _error = null;
    notifyListeners();
  }

  /// The app-data folder for one engine.
  ///
  /// The default follows the same per-platform application data rule the rest
  /// of the app uses, so Windows and Linux no longer inherit a macOS path.
  Future<Directory> engineDirectory(String id) async {
    final custom = _supportDirectory;
    final support = custom != null
        ? await custom()
        : Directory(_host.appDataDirectory);
    return Directory(_host.join(support.path, 'tts', id));
  }

  Future<Directory> _ensureEngineDirectory(String id) async {
    final directory = await engineDirectory(id);
    await directory.create(recursive: true);
    return directory;
  }

  Future<bool> isInstalled(LocalTtsEngine engine) async {
    if (!engine.installable) return true;
    final directory = await engineDirectory(engine.id);
    return Directory('${directory.path}${Platform.pathSeparator}.git').exists();
  }

  /// Clones and prepares the engine, streaming every step into the log.
  Future<void> install(LocalTtsEngine engine) async {
    if (_busy || !engine.installable) return;
    _begin();
    try {
      final directory = await _ensureEngineDirectory(engine.id);
      final git = _host.findExecutable('git');
      if (git == null) {
        throw LocalTtsException(
          'git-missing',
          'Git is required to install ${engine.label}. Install Git for '
              '${_host.name}, then try again.',
        );
      }
      if (!Directory('${directory.path}${Platform.pathSeparator}.git')
          .existsSync()) {
        await _runStep(
          _step(git, [
            'clone',
            '--depth',
            '1',
            if (engine.repositoryRef.isNotEmpty) ...[
              '--branch',
              engine.repositoryRef,
            ],
            engine.repository,
            '.',
          ]),
          workingDirectory: directory,
          timeout: const Duration(minutes: 20),
        );
      }
      final uv = await _ensureUv();
      if (engine.installArguments.isNotEmpty) {
        await _runStep(
          _step(uv, engine.installArguments),
          workingDirectory: directory,
          timeout: const Duration(minutes: 30),
        );
      }
    } on LocalTtsException catch (error) {
      _error = error.message;
    } on ProcessException catch (error) {
      _error = error.message;
    } on TimeoutException {
      _error = 'The install did not finish in time.';
    } on Object catch (error) {
      _error = '$error';
    } finally {
      _finish();
    }
  }

  /// Starts the engine, or a shell command the user typed for a custom server.
  Future<void> start(LocalTtsEngine engine, {String command = ''}) async {
    if (_busy || _running) return;
    final custom = command.trim();
    _engineId = engine.id;
    _begin();
    try {
      final directory = await _ensureEngineDirectory(engine.id);
      final Process process;
      if (custom.isNotEmpty) {
        // A user-supplied command is theirs to write, so it keeps the host's
        // own shell. Everything the app generates never needs a shell.
        _append(r'$ $custom');
        final (shell, arguments) = _host.isWindows
            ? ('cmd.exe', ['/c', custom])
            : ('/bin/sh', ['-lc', custom]);
        process = await Process.start(
          shell,
          arguments,
          workingDirectory: directory.path,
          environment: _childEnvironment,
        );
      } else {
        if (engine.startArguments.isEmpty) {
          throw const LocalTtsException(
            'start-command',
            'Enter the command that starts this server.',
          );
        }
        final uv = await _ensureUv();
        _append('\$ ${_format(_step(uv, engine.startArguments))}');
        process = await Process.start(
          uv,
          engine.startArguments,
          workingDirectory: directory.path,
          environment: _childEnvironment,
        );
      }
      _process = process;
      _running = true;
      _stdout = process.stdout
          .transform(utf8.decoder)
          .transform(const LineSplitter())
          .listen(_append);
      _stderr = process.stderr
          .transform(utf8.decoder)
          .transform(const LineSplitter())
          .listen(_append);
      _busy = false;
      notifyListeners();
      unawaited(
        process.exitCode.then((code) {
          if (_process == process) {
            _process = null;
            _running = false;
            _append('[server exited with code $code]');
            notifyListeners();
          }
        }),
      );
    } on LocalTtsException catch (error) {
      _error = error.message;
      _finish();
    } on ProcessException catch (error) {
      _error = error.message;
      _finish();
    } on Object catch (error) {
      _error = '$error';
      _finish();
    }
  }

  Future<void> stop() async {
    final process = _process;
    _process = null;
    _running = false;
    _busy = true;
    notifyListeners();
    try {
      if (process != null) {
        process.kill(ProcessSignal.sigterm);
        await process.exitCode.timeout(
          const Duration(seconds: 4),
          onTimeout: () {
            process.kill(ProcessSignal.sigkill);
            return process.exitCode;
          },
        );
      }
    } finally {
      await _cancelStreams();
      _busy = false;
      notifyListeners();
    }
  }

  void _begin() {
    _busy = true;
    _error = null;
    _log.clear();
    notifyListeners();
  }

  void _finish() {
    _busy = false;
    notifyListeners();
  }

  Map<String, String> get _childEnvironment => {
    ..._host.environment,
    'PYTHONUNBUFFERED': '1',
  };

  /// Resolves `uv`, bootstrapping it through its own installer when the user's
  /// machine has never run it. Both installers place the binary in
  /// `~/.local/bin`, which [HostPlatform] already searches.
  Future<String> _ensureUv() async {
    final existing = _host.findExecutable('uv');
    if (existing != null) return existing;
    _append('uv was not found; installing it for ${_host.name}.');
    if (_host.isWindows) {
      final powershell =
          _host.findExecutable('powershell') ?? _host.findExecutable('pwsh');
      if (powershell == null) {
        throw const LocalTtsException(
          'uv-bootstrap',
          'PowerShell is required to install the speech engine automatically. '
              'Install uv from https://docs.astral.sh/uv and try again.',
        );
      }
      await _runStep(
        _step(powershell, [
          '-NoProfile',
          '-ExecutionPolicy',
          'Bypass',
          '-Command',
          'irm https://astral.sh/uv/install.ps1 | iex',
        ]),
        workingDirectory: await _ensureEngineDirectory(_engineId),
        timeout: const Duration(minutes: 10),
      );
    } else {
      final shell = File('/bin/sh');
      if (!shell.existsSync()) {
        throw const LocalTtsException(
          'uv-bootstrap',
          'Installing the speech engine needs a POSIX shell at /bin/sh.',
        );
      }
      await _runStep(
        _step('/bin/sh', [
          '-c',
          'curl -LsSf https://astral.sh/uv/install.sh | sh',
        ]),
        workingDirectory: await _ensureEngineDirectory(_engineId),
        timeout: const Duration(minutes: 10),
      );
    }
    final installed = _host.findExecutable('uv');
    if (installed == null) {
      throw const LocalTtsException(
        'uv-missing',
        'uv was installed but could not be found. Restart the app and try '
            'again, or install uv manually from https://docs.astral.sh/uv.',
      );
    }
    return installed;
  }

  Future<void> _runStep(
    _Step step, {
    required Directory workingDirectory,
    required Duration timeout,
  }) async {
    await _cancelStreams();
    _append('\$ ${_format(step)}');
    final process = await Process.start(
      step.program,
      step.arguments,
      workingDirectory: workingDirectory.path,
      environment: _childEnvironment,
    );
    _stdout = process.stdout
        .transform(utf8.decoder)
        .transform(const LineSplitter())
        .listen(_append);
    _stderr = process.stderr
        .transform(utf8.decoder)
        .transform(const LineSplitter())
        .listen(_append);
    final int code;
    try {
      code = await process.exitCode.timeout(timeout);
    } on TimeoutException {
      process.kill(ProcessSignal.sigkill);
      rethrow;
    }
    if (code != 0) {
      throw LocalTtsException(
        'install-failed',
        'The install step exited with code $code.',
      );
    }
  }

  _Step _step(String program, List<String> arguments) =>
      _Step(program, arguments);

  String _format(_Step step) => step.arguments.isEmpty
      ? step.program
      : '${step.program} ${step.arguments.join(' ')}';

  void _append(String line) {
    if (line.isEmpty) return;
    _log.add(line);
    while (_log.length > logLimit) {
      _log.removeAt(0);
    }
    notifyListeners();
  }

  Future<void> _cancelStreams() async {
    await _stdout?.cancel();
    await _stderr?.cancel();
    _stdout = null;
    _stderr = null;
  }

  @override
  void dispose() {
    unawaited(_cancelStreams());
    _process?.kill(ProcessSignal.sigterm);
    super.dispose();
  }
}

class _Step {
  const _Step(this.program, this.arguments);

  final String program;
  final List<String> arguments;
}
