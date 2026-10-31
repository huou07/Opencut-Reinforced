import 'dart:async';

import 'package:flutter/foundation.dart';

import '../project/export_profile.dart';
import '../project/project_file_picker.dart';
import '../project/project_gateway.dart';

/// Options the owner picked in the export dialog, as the picker and gateway
/// need them.
typedef ExportOptions = ({
  ExportProfile profile,
  ExportQuality quality,
  int? shortSide,
});

/// One export: options, the job, its polling timer, the chosen destination and
/// publishing the finished file.
///
/// The shell keeps only presentation state. Everything that outlives a frame
/// (the job, its timer, the pending destination) lives here, so a rebuild
/// never re-creates or leaks it, and the flow is testable with a fake gateway.
class ExportFlow extends ChangeNotifier {
  ExportFlow({
    required this.gateway,
    required this.picker,
    required this.chooseOptions,
    required this.onProjectSummary,
    required this.onExported,
  });

  final ProjectGateway gateway;
  final ProjectFilePicker picker;
  final Future<ExportOptions?> Function() chooseOptions;
  final void Function(ProjectReadModel current) onProjectSummary;
  final void Function(String destination) onExported;

  Timer? _pollTimer;
  bool _pollInFlight = false;
  int _statusFailures = 0;
  static const _maxStatusFailures = 5;
  ProjectExportJob? _job;
  String? _destinationPath;
  String? _status;
  bool _failed = false;
  bool _publishing = false;
  ProjectSessionHandle? _session;

  ProjectExportJob? get job => _job;
  String? get status => _status;
  bool get failed => _failed;
  bool get publishing => _publishing;

  /// True while a job runs or a finished file is being published; the shell
  /// disables export and close in that window.
  bool get isActive => ((_job?.isActive ?? false) && !_failed) || _publishing;

  /// Whether this host can start an export at all.
  bool get enabled => picker.supportsExport;

  /// Starts an export for [session]. A no-op unless the session is still the
  /// one the shell shows after each await.
  Future<void> start(ProjectSessionHandle session) async {
    if (!picker.supportsExport || isActive) return;
    _session = session;
    // Each export gets a fresh polling budget: the counter belongs to a
    // running job, not to this flow object.
    _statusFailures = 0;
    String? destination;
    try {
      final current = await gateway.summary(session);
      if (!_isCurrent(session)) return;
      final choice = await chooseOptions();
      if (choice == null || !_isCurrent(session)) return;
      destination = await picker.saveExportPath(
        suggestedName: choice.profile.suggestedName(current.name),
        profile: choice.profile,
      );
      if (destination == null) return;
      if (!_isCurrent(session)) {
        await _discard(destination);
        return;
      }
      final result = await gateway.startExport(
        session,
        current,
        destination,
        quality: choice.profile.hasQualityChoice
            ? choice.quality.wireName
            : null,
        shortSide: choice.shortSide,
      );
      if (!_isCurrent(session)) {
        await _discard(destination);
        return;
      }
      onProjectSummary(current);
      _projectView = current;
      _job = result;
      _destinationPath = destination;
      _failed = !result.succeeded;
      _status = exportJobLabel(result);
      notifyListeners();
      if (result.succeeded && result.isActive) {
        _pollTimer?.cancel();
        _pollTimer = Timer.periodic(
          const Duration(milliseconds: 400),
          (_) => unawaited(_poll()),
        );
      } else if (!result.succeeded) {
        await _discard(destination);
      }
    } on ProjectGatewayException catch (error) {
      if (destination != null) await _discard(destination);
      if (_isCurrent(session)) {
        _status = error.message;
        _failed = true;
        notifyListeners();
      }
    } catch (_) {
      if (destination != null) await _discard(destination);
      if (_isCurrent(session)) {
        _status = 'Export could not start';
        _failed = true;
        notifyListeners();
      }
    }
  }

  /// Cancels the running job, or aborts publishing a finished one.
  Future<void> cancel() async {
    final session = _session;
    final project = _projectView;
    final job = _job;
    final destination = _destinationPath;
    if (_publishing && destination != null) {
      try {
        await picker.cancelExportPublish(destination);
      } catch (_) {
        _status = 'Export cancellation failed';
        _failed = true;
        notifyListeners();
      }
      return;
    }
    if (session == null || project == null || job == null || !job.isActive) {
      return;
    }
    try {
      final result = await gateway.cancelExport(session, project, job.jobId);
      if (!_isCurrent(session)) return;
      _job = result;
      _failed = !result.succeeded;
      _status = exportJobLabel(result);
      notifyListeners();
      if (!result.succeeded || !result.isActive) {
        _pollTimer?.cancel();
        if (result.state == 'cancelled') {
          _destinationPath = null;
          if (destination != null) await _discard(destination);
        }
      }
    } catch (_) {
      if (!_isCurrent(session)) return;
      _status = 'Export cancellation failed';
      _failed = true;
      notifyListeners();
    }
  }

  /// Forgets the job when the project goes away; the file, if any, has already
  /// been published or discarded by then.
  void detach() {
    _session = null;
    _statusFailures = 0;
    _pollTimer?.cancel();
    _pollTimer = null;
    _job = null;
    _destinationPath = null;
    _status = null;
    _failed = false;
    _publishing = false;
    _projectView = null;
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    super.dispose();
  }

  ProjectReadModel? _projectView;

  bool _isCurrent(ProjectSessionHandle session) => identical(session, _session);

  Future<void> _poll() async {
    final session = _session;
    final job = _job;
    if (session == null || job == null || _pollInFlight) return;
    _pollInFlight = true;
    try {
      final status = await gateway.exportStatus(
        session,
        _projectView!,
        job.jobId,
      );
      if (!_isCurrent(session)) return;
      _statusFailures = 0;
      if (status.succeeded && status.state == 'succeeded') {
        final destination = _destinationPath;
        if (destination != null) {
          _job = status;
          _publishing = true;
          _failed = false;
          _status = 'Saving export to the selected location…';
          notifyListeners();
          try {
            await picker.publishExportPath(destination);
            if (!_isCurrent(session)) return;
            _status = 'Export complete';
            notifyListeners();
            // The status line is two words; the file itself is what the owner
            // needs, so the receipt carries its name, size and a copy action.
            onExported(destination);
          } on ProjectSafStorageException catch (error) {
            if (!_isCurrent(session)) return;
            _status = error.message;
            _failed = true;
            notifyListeners();
            await _discard(destination);
          } catch (_) {
            if (!_isCurrent(session)) return;
            _status = 'The export could not be saved to the selected location.';
            _failed = true;
            notifyListeners();
            await _discard(destination);
          } finally {
            _destinationPath = null;
            _publishing = false;
            _pollTimer?.cancel();
          }
          return;
        }
      }
      _job = status;
      _failed = !status.succeeded || status.state == 'failed';
      _status = exportJobLabel(status);
      notifyListeners();
      if (!status.succeeded || !status.isActive) {
        _pollTimer?.cancel();
        if (!status.succeeded || status.state == 'cancelled') {
          final destination = _destinationPath;
          _destinationPath = null;
          if (destination != null) await _discard(destination);
        }
      }
    } catch (_) {
      if (!_isCurrent(session)) return;
      _statusFailures += 1;
      // A single status read can fail while the renderer is busy. Abandoning
      // the timer there would leave a running job with no watcher, and the
      // shell would stop treating the export as active. Retry a bounded number
      // of times, then report it.
      if (_statusFailures < _maxStatusFailures) {
        _status = 'Export status unavailable, retrying…';
        notifyListeners();
        return;
      }
      _status = 'Export status unavailable';
      _failed = true;
      notifyListeners();
      _pollTimer?.cancel();
    } finally {
      _pollInFlight = false;
    }
  }

  Future<void> _discard(String path) async {
    try {
      await picker.discardExportPath(path);
    } catch (_) {
      // Preserve the primary export result if cleanup fails.
    }
  }
}

/// The one-line label the workspace shows for [job].
String exportJobLabel(ProjectExportJob job) {
  if (!job.succeeded) {
    return job.message.isEmpty ? 'Export failed' : job.message;
  }
  return switch (job.state) {
    'queued' => 'Export queued',
    'running' =>
      job.progressCompleted != null && job.progressTotal != null
          ? 'Export ${job.progressCompleted}/${job.progressTotal}'
          : 'Exporting',
    'succeeded' => 'Export complete',
    'failed' => job.failureMessage ?? 'Export failed',
    'cancelled' => 'Export cancelled',
    _ => job.message,
  };
}
