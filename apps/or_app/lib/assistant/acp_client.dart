import 'dart:async';
import 'dart:convert';
import 'dart:io';

/// One Agent Client Protocol (ACP) session over a child process' stdio.
///
/// The harness is an external program: OR starts it, speaks newline-delimited
/// JSON-RPC to it, and streams its semantic updates into the assistant panel.
/// This client owns no project state; every edit the agent makes goes through
/// the OpenCut MCP server and the shared command path.
class AcpClient {
  AcpClient({this.onUpdate, this.onPermissionRequest, this.onExit});

  /// Called for every `session/update` notification.
  final void Function(Map<String, Object?> update)? onUpdate;

  /// Called for `session/request_permission`; resolves to an option id, or
  /// null when the user dismissed the request.
  final Future<String?> Function(Map<String, Object?> request)?
  onPermissionRequest;

  /// Called once when the child process ends, with its exit code (or -1).
  final void Function(int exitCode, String stderrTail)? onExit;

  Process? _process;
  StreamSubscription<String>? _stdoutSubscription;
  StreamSubscription<String>? _stderrSubscription;
  final _pending = <int, Completer<Map<String, Object?>>>{};
  final _stderr = StringBuffer();
  int _nextId = 0;
  bool _closed = false;
  bool _exited = false;

  String? sessionId;
  List<Map<String, Object?>> configOptions = const [];
  Map<String, Object?>? agentInfo;

  bool get isRunning => _process != null && !_exited;

  /// Starts the harness and creates a session.
  ///
  /// [workingDirectory] is the process working directory; [cwd] is the ACP
  /// session workspace (absolute). [mcpServers] is passed to `session/new`.
  Future<void> start({
    required String executable,
    required List<String> arguments,
    required String workingDirectory,
    required String cwd,
    required List<Map<String, Object?>> mcpServers,
  }) async {
    if (_process != null) {
      throw StateError('the ACP client is already started');
    }
    final environment = Map<String, String>.from(Platform.environment)
      ..removeWhere((key, _) => key.startsWith('DSH_'));
    final process = await Process.start(
      executable,
      arguments,
      workingDirectory: workingDirectory,
      environment: environment,
      includeParentEnvironment: false,
    );
    _process = process;
    _stdoutSubscription = process.stdout
        .transform(utf8.decoder)
        .transform(const LineSplitter())
        .listen(_handleLine, onError: _handleTransportError);
    _stderrSubscription = process.stderr.transform(utf8.decoder).listen((
      chunk,
    ) {
      // Keep a bounded tail: the process can be chatty and the panel only
      // needs the reason it failed.
      _stderr.write(chunk);
      if (_stderr.length > 4000) {
        final text = _stderr.toString();
        _stderr
          ..clear()
          ..write(text.substring(text.length - 4000));
      }
    });
    unawaited(
      process.exitCode.then((code) {
        _handleExit(code);
      }),
    );
    final initialized = await _request('initialize', {
      'protocolVersion': 1,
      'clientCapabilities': {
        'fs': {'readTextFile': false, 'writeTextFile': false},
      },
      'clientInfo': {'name': 'opencut-reinforced', 'version': '0.1.0'},
    }, timeout: const Duration(seconds: 30));
    agentInfo = _asMap(initialized['agentInfo']);
    await newSession(cwd: cwd, mcpServers: mcpServers);
  }

  /// Closes the current session (if any) and creates a fresh one.
  Future<void> newSession({
    required String cwd,
    required List<Map<String, Object?>> mcpServers,
  }) async {
    await closeSession();
    final created = await _request('session/new', {
      'cwd': cwd,
      'mcpServers': mcpServers,
    }, timeout: const Duration(seconds: 60));
    sessionId = created['sessionId'] as String?;
    configOptions = _asList(created['configOptions']);
    if (sessionId == null || sessionId!.isEmpty) {
      throw const AcpException(
        'The harness created no session.',
        'ACP_SESSION_MISSING',
      );
    }
  }

  /// Ends the current session but keeps the process alive.
  Future<void> closeSession() async {
    final id = sessionId;
    if (id == null) return;
    sessionId = null;
    try {
      await _request('session/close', {
        'sessionId': id,
      }, timeout: const Duration(seconds: 10));
    } on AcpException {
      // A closing session may already be gone; the next session is what matters.
    }
  }

  /// Sets one advertised configuration option (model or reasoning effort).
  Future<void> setConfigOption(String configId, String value) async {
    final id = sessionId;
    if (id == null) {
      throw const AcpException('No session is open.', 'ACP_NO_SESSION');
    }
    final result = await _request('session/set_config_option', {
      'sessionId': id,
      'configId': configId,
      'value': value,
    }, timeout: const Duration(seconds: 20));
    configOptions = _asList(result['configOptions']);
  }

  /// Sends one prompt turn and resolves when the agent finishes it.
  Future<void> prompt(String text) async {
    final id = sessionId;
    if (id == null) {
      throw const AcpException('No session is open.', 'ACP_NO_SESSION');
    }
    await _request('session/prompt', {
      'sessionId': id,
      'prompt': [
        {'type': 'text', 'text': text},
      ],
    });
  }

  /// Asks the agent to cancel the turn in flight.
  void cancel() {
    final id = sessionId;
    if (id == null) return;
    _notify('session/cancel', {'sessionId': id});
  }

  /// Stops the child process. Safe to call more than once.
  Future<void> dispose() async {
    if (_closed) return;
    _closed = true;
    final process = _process;
    if (process == null) return;
    await closeSession();
    await _stdoutSubscription?.cancel();
    await _stderrSubscription?.cancel();
    try {
      await process.stdin.flush();
      await process.stdin.close();
    } catch (_) {
      // The child may already be gone.
    }
    if (!_exited) {
      process.kill(ProcessSignal.sigterm);
      final ended = await process.exitCode.timeout(
        const Duration(seconds: 5),
        onTimeout: () {
          process.kill(ProcessSignal.sigkill);
          return -1;
        },
      );
      _handleExit(ended);
    }
  }

  Future<Map<String, Object?>> _request(
    String method,
    Map<String, Object?> params, {
    Duration? timeout,
  }) {
    final process = _process;
    if (process == null || _exited) {
      throw AcpException(
        'The harness is not running.',
        'ACP_NOT_RUNNING',
        stderrTail: _stderr.toString(),
      );
    }
    final id = ++_nextId;
    final completer = Completer<Map<String, Object?>>();
    _pending[id] = completer;
    try {
      process.stdin.writeln(
        jsonEncode({
          'jsonrpc': '2.0',
          'id': id,
          'method': method,
          'params': params,
        }),
      );
    } catch (error) {
      _pending.remove(id);
      throw AcpException(
        'The harness could not receive a request: $error',
        'ACP_WRITE_FAILED',
      );
    } finally {
      if (_pending.containsKey(id)) {
        unawaited(process.stdin.flush());
      }
    }
    Future<Map<String, Object?>> pending = completer.future;
    if (timeout != null) {
      pending = pending.timeout(
        timeout,
        onTimeout: () {
          _pending.remove(id);
          throw AcpException(
            'The harness did not answer "$method" in time.',
            'ACP_TIMEOUT',
          );
        },
      );
    }
    return pending;
  }

  void _notify(String method, Map<String, Object?> params) {
    final process = _process;
    if (process == null || _exited) return;
    try {
      process.stdin.writeln(
        jsonEncode({'jsonrpc': '2.0', 'method': method, 'params': params}),
      );
      unawaited(process.stdin.flush());
    } catch (_) {
      // A transport failure is reported by the exit handler.
    }
  }

  void _respond(Object id, Map<String, Object?> result) {
    final process = _process;
    if (process == null || _exited) return;
    try {
      process.stdin.writeln(
        jsonEncode({'jsonrpc': '2.0', 'id': id, 'result': result}),
      );
      unawaited(process.stdin.flush());
    } catch (_) {
      // The exit handler owns the failure path.
    }
  }

  void _respondError(Object id, int code, String message) {
    final process = _process;
    if (process == null || _exited) return;
    try {
      process.stdin.writeln(
        jsonEncode({
          'jsonrpc': '2.0',
          'id': id,
          'error': {'code': code, 'message': message},
        }),
      );
      unawaited(process.stdin.flush());
    } catch (_) {
      // The exit handler owns the failure path.
    }
  }

  void _handleLine(String line) {
    if (line.trim().isEmpty) return;
    Map<String, Object?> message;
    try {
      final decoded = jsonDecode(line);
      if (decoded is! Map<String, Object?>) return;
      message = decoded;
    } catch (_) {
      return;
    }
    final id = message['id'];
    final method = message['method'] as String?;
    if (method != null) {
      // A request from the agent. Permission is the only client-side request
      // this client advertises; anything else is refused rather than hanging.
      if (method == 'session/update') {
        final params = _asMap(message['params']);
        onUpdate?.call(_asMap(params['update']));
        return;
      }
      if (id is int || id is String) {
        if (method == 'session/request_permission') {
          final params = _asMap(message['params']);
          unawaited(_answerPermission(id as Object, params));
        } else {
          _respondError(
            id as Object,
            -32601,
            'this client does not support $method',
          );
        }
      }
      return;
    }
    if (id is! int) return;
    final completer = _pending.remove(id);
    if (completer == null || completer.isCompleted) return;
    final error = message['error'];
    if (error is Map) {
      final errorMap = _asMap(error);
      completer.completeError(
        AcpException(
          (errorMap['message'] as String?) ??
              'The harness rejected the request.',
          (errorMap['code']?.toString()) ?? 'ACP_ERROR',
        ),
      );
      return;
    }
    final result = message['result'];
    completer.complete(result is Map<String, Object?> ? result : const {});
  }

  Future<void> _answerPermission(Object id, Map<String, Object?> params) async {
    String? optionId;
    final handler = onPermissionRequest;
    if (handler != null) {
      try {
        optionId = await handler(params);
      } catch (_) {
        optionId = null;
      }
    }
    if (optionId == null) {
      _respond(id, {
        'outcome': {'outcome': 'cancelled'},
      });
    } else {
      _respond(id, {
        'outcome': {'outcome': 'selected', 'optionId': optionId},
      });
    }
  }

  void _handleTransportError(Object error) {
    _handleExit(-1, tail: 'stream error: $error');
  }

  void _handleExit(int code, {String? tail}) {
    if (_exited) return;
    _exited = true;
    final message = tail ?? _stderr.toString();
    for (final completer in _pending.values) {
      if (!completer.isCompleted) {
        completer.completeError(
          AcpException(
            code == 0
                ? 'The harness stopped.'
                : 'The harness stopped with exit code $code.',
            'ACP_EXITED',
            stderrTail: message,
          ),
        );
      }
    }
    _pending.clear();
    onExit?.call(code, message);
  }
}

class AcpException implements Exception {
  const AcpException(this.message, this.code, {this.stderrTail = ''});

  final String message;
  final String code;
  final String stderrTail;

  @override
  String toString() => message;
}

Map<String, Object?> _asMap(Object? value) {
  if (value is Map<String, Object?>) return value;
  if (value is Map) {
    return value.map((key, value) => MapEntry(key.toString(), value));
  }
  return const {};
}

List<Map<String, Object?>> _asList(Object? value) {
  if (value is! List) return const [];
  return [
    for (final item in value)
      if (item is Map) _asMap(item),
  ];
}
