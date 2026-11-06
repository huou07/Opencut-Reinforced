/// One file's import result, as the shell's revision-checked command reports it.
typedef ImportAttempt = ({bool imported, String? failure, bool aborted});

/// The outcome of one picker round: every selected source was attempted once.
class MediaImportOutcome {
  const MediaImportOutcome({
    required this.imported,
    required this.total,
    required this.failures,
    required this.folder,
    required this.aborted,
    this.cancelled = false,
  });

  final int imported;
  final int total;
  final List<String> failures;
  final bool folder;

  /// True when the project changed under the loop, so the run stopped early.
  final bool aborted;

  /// True when the user stopped the batch before every source was attempted.
  final bool cancelled;

  /// The line the top bar shows while a file is probed, or null when done.
  String? labelFor(int position, String source) =>
      importStatusLabel(position, total, source);

  /// The closing statement, or null when one imported file is its own proof.
  String? get summary => importSummary(
    imported: imported,
    total: total,
    failures: failures,
    folder: folder,
    cancelled: cancelled,
  );
}

/// Drives the per-file loop over a picker selection.
///
/// Probing and adding stay in the shell, because each file goes through the
/// same revision-checked project action as an ordinary edit. This class owns
/// only the walk: the progress label, the failure list and the closing line.
class MediaImportFlow {
  const MediaImportFlow({
    required this.importOne,
    required this.onStatus,
    this.cancelled,
  });

  /// Runs one source; `aborted` means the project changed under the loop.
  final Future<ImportAttempt> Function(String source) importOne;

  /// Publishes the transient top-bar line; null clears it.
  final void Function(String? status) onStatus;

  /// Asked before every file. A folder of two hundred recordings had no way
  /// out once it started: the only control was "wait", so this is the check the
  /// top bar's stop button answers. Null means the batch always runs to the end.
  final bool Function()? cancelled;

  Future<MediaImportOutcome> run(
    List<String> sources, {
    required bool folder,
  }) async {
    var imported = 0;
    final failures = <String>[];
    MediaImportOutcome outcome({
      required bool aborted,
      bool cancelled = false,
    }) => MediaImportOutcome(
      imported: imported,
      total: sources.length,
      failures: failures,
      folder: folder,
      aborted: aborted,
      cancelled: cancelled,
    );
    try {
      for (var index = 0; index < sources.length; index += 1) {
        if (cancelled?.call() ?? false) {
          return outcome(aborted: false, cancelled: true);
        }
        final source = sources[index];
        onStatus(importStatusLabel(index + 1, sources.length, source));
        final attempt = await importOne(source);
        if (attempt.aborted) {
          return outcome(aborted: true);
        }
        if (attempt.imported) {
          imported += 1;
        } else if (attempt.failure != null) {
          failures.add(attempt.failure!);
        }
      }
    } finally {
      onStatus(null);
    }
    // A stop pressed during the last file still means the batch was stopped,
    // even though nothing was left to skip.
    return outcome(aborted: false, cancelled: (cancelled?.call() ?? false));
  }
}

/// One line for the top bar's transient slot while files are probed and added
/// one at a time.
String importStatusLabel(int position, int total, String source) {
  final segments = source.replaceAll('\\', '/').split('/');
  final name = segments.isEmpty || segments.last.isEmpty
      ? source
      : segments.last;
  // The trailing ellipsis below belongs to the in-progress line; adding one
  // here as well used to print two dots on long file names.
  final shown = name.length > 32 ? name.substring(0, 29) : name;
  return total <= 1
      ? 'Importing $shown…'
      : 'Importing $position of $total · $shown';
}

/// The closing statement for a batch, or null when a single file imported.
///
/// One file is its own confirmation (it appears in the library); a batch of ten
/// otherwise finished with no statement that anything happened.
String? importSummary({
  required int imported,
  required int total,
  required List<String> failures,
  required bool folder,
  bool cancelled = false,
}) {
  final noun = folder ? 'folder files' : 'selected files';
  if (cancelled) {
    final skipped = total - imported;
    final head = imported == 0
        ? 'Stopped before anything was imported. $total $noun were not added.'
        : 'Imported $imported $noun, then stopped; $skipped were not added.';
    return failures.isEmpty ? head : '$head ${_failureTail(failures)}';
  }
  if (failures.isNotEmpty) {
    return 'Imported $imported of $total $noun. '
        '${failures.length} could not be imported. ${_failureTail(failures)}';
  }
  if (total > 1) return 'Imported $imported $noun.';
  return null;
}

String _failureTail(List<String> failures) {
  final first = failures.first;
  return first.length > 180 ? '${first.substring(0, 177)}…' : first;
}
