import 'dart:async';

/// What the top bar can say about the recovery checkpoint.
class AutosaveStatus {
  const AutosaveStatus({this.label, this.failed = false});

  final String? label;
  final bool failed;

  static const none = AutosaveStatus();
}

/// Runs a recovery checkpoint on a fixed interval.
///
/// The controller owns the "one attempt at a time" rule and the status text;
/// the owner decides what an attempt means (session identity, gateway call)
/// and rebuilds on [onChanged].
class AutosaveController {
  AutosaveController({
    required this.attempt,
    required this.onChanged,
    this.interval = const Duration(seconds: 30),
  });

  /// Returns whether the checkpoint succeeded, or null when the attempt no
  /// longer belongs to the active session and its result should be ignored.
  final Future<bool?> Function() attempt;
  final void Function(AutosaveStatus status) onChanged;
  final Duration interval;

  Timer? _timer;
  bool _inFlight = false;
  AutosaveStatus _status = AutosaveStatus.none;

  AutosaveStatus get status => _status;

  void start() {
    _timer ??= Timer.periodic(interval, (_) => unawaited(tick()));
  }

  void stop() {
    _timer?.cancel();
    _timer = null;
  }

  /// Forgets the last status, e.g. when the active project is cleared.
  void clear() => _set(AutosaveStatus.none);

  Future<void> tick() async {
    if (_inFlight) return;
    _inFlight = true;
    _set(const AutosaveStatus(label: 'Autosaving recovery'));
    try {
      final succeeded = await attempt();
      if (succeeded == null) return;
      _set(
        AutosaveStatus(
          label: succeeded ? 'Recovery saved' : 'Autosave needs attention',
          failed: !succeeded,
        ),
      );
    } catch (_) {
      _set(
        const AutosaveStatus(label: 'Autosave needs attention', failed: true),
      );
    } finally {
      _inFlight = false;
    }
  }

  void _set(AutosaveStatus value) {
    _status = value;
    onChanged(value);
  }
}
