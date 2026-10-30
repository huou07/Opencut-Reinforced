import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';

import 'local_tts_service.dart';

/// One local speech server the app can install and run without a terminal.
class LocalTtsEngine {
  const LocalTtsEngine({
    required this.id,
    required this.label,
    required this.description,
    required this.baseUrl,
    required this.installCommand,
    required this.startCommand,
    this.homepage = '',
  });

  final String id;
  final String label;
  final String description;
  final String baseUrl;

  /// A shell command run in the engine directory. Empty means the engine is
  /// already installed and only [startCommand] is needed.
  final String installCommand;
  final String startCommand;
  final String homepage;

  bool get installable => installCommand.isNotEmpty;
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
    installCommand:
        'set -e; command -v uv >/dev/null 2>&1 || curl -LsSf https://astral.sh/uv/install.sh | sh; '
        'export PATH="\$HOME/.local/bin:\$PATH"; '
        'if [ ! -d .git ]; then git clone --depth 1 https://github.com/pnnbao97/VieNeu-TTS.git .; fi; '
        'uv sync',
    startCommand: 'export PATH="\$HOME/.local/bin:\$PATH"; exec uv run python apps/openai_speech.py',
  ),
  LocalTtsEngine(
    id: 'custom',
    label: 'Custom OpenAI-compatible server',
    description:
        'Any loopback server that answers GET /health and POST /v1/audio/speech. '
        'Choose this for Kokoro, Chatterbox, Piper wrappers or your own engine.',
    baseUrl: LocalTtsService.defaultBaseUrl,
    installCommand: '',
    startCommand: '',
  ),
];

/// Starts, stops and installs the local speech servers OR talks to.
///
/// The engine remains a separate process on loopback, exactly like a server the
/// user started by hand. The manager only removes the terminal step: it runs
/// the engine's documented commands, streams their output into the settings
/// surface, and keeps the child process so Stop really stops it.
class LocalTtsServerManager extends ChangeNotifier {
  /// One server per app session: closing the voiceover dialog must not kill a
  /// server the user started, and reopening it must still be able to stop it.
  static final LocalTtsServerManager instance = LocalTtsServerManager();

  LocalTtsServerManager({
    Future<Directory> Function()? supportDirectory,
    this.logLimit = 120,
  }) : _supportDirectory = supportDirectory ?? _defaultSupportDirectory;

  static Future<Directory> _defaultSupportDirectory() async {
    final home = Platform.environment['HOME'];
    final base = home == null || home.isEmpty
        ? Directory.systemTemp
        : Directory('$home/Library/Application Support/io.github.huou07.orApp');
    return Directory('${base.path}${Platform.pathSeparator}tts');
  }

  final Future<Directory> Function() _supportDirectory;
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

  Future<Directory> engineDirectory(String id) async {
    final support = await _supportDirectory();
    return Directory('${support.path}${Platform.pathSeparator}$id');
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

  Future<void> install(LocalTtsEngine engine) async {
    if (_busy || engine.installCommand.isEmpty) return;
    await _run(
      engine,
      engine.installCommand,
      markRunning: false,
      timeout: const Duration(minutes: 30),
    );
  }

  Future<void> start(LocalTtsEngine engine, {String command = ''}) async {
    if (_busy || _running) return;
    final effective = command.trim().isEmpty
        ? engine.startCommand
        : command.trim();
    if (effective.isEmpty) {
      _error = 'Enter the command that starts this server.';
      notifyListeners();
      return;
    }
    _engineId = engine.id;
    await _run(engine, effective, markRunning: true);
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

  Future<void> _run(
    LocalTtsEngine engine,
    String command, {
    required bool markRunning,
    Duration timeout = const Duration(minutes: 2),
  }) async {
    _busy = true;
    _error = null;
    _log.clear();
    _append('\$ $command');
    notifyListeners();
    try {
      final directory = await _ensureEngineDirectory(engine.id);
      final process = await Process.start(
        'bash',
        ['-lc', command],
        workingDirectory: directory.path,
        environment: {...Platform.environment, 'PYTHONUNBUFFERED': '1'},
        runInShell: false,
      );
      if (markRunning) {
        _process = process;
        _running = true;
      }
      _stdout = process.stdout
          .transform(utf8.decoder)
          .transform(const LineSplitter())
          .listen(_append);
      _stderr = process.stderr
          .transform(utf8.decoder)
          .transform(const LineSplitter())
          .listen(_append);
      if (markRunning) {
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
        return;
      }
      final code = await process.exitCode.timeout(timeout);
      if (code != 0) _error = 'The install command exited with code $code.';
    } on TimeoutException {
      _error = 'The command did not finish in time.';
    } on ProcessException catch (error) {
      _error = error.message;
    } on Object catch (error) {
      _error = '$error';
    } finally {
      _busy = false;
      notifyListeners();
    }
  }

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
