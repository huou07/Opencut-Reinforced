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
  });

  final int imported;
  final int total;
  final List<String> failures;
  final bool folder;

  /// True when the project changed under the loop, so the run stopped early.
  final bool aborted;

  /// The line the top bar shows while a file is probed, or null when done.
  String? labelFor(int position, String source) =>
      importStatusLabel(position, total, source);

  /// The closing statement, or null when one imported file is its own proof.
  String? get summary =>
      importSummary(imported: imported, total: total, failures: failures, folder: folder);
}

/// Drives the per-file loop over a picker selection.
///
/// Probing and adding stay in the shell, because each file goes through the
/// same revision-checked project action as an ordinary edit. This class owns
/// only the walk: the progress label, the failure list and the closing line.
class MediaImportFlow {
  const MediaImportFlow({required this.importOne, required this.onStatus});

  /// Runs one source; `aborted` means the project changed under the loop.
  final Future<ImportAttempt> Function(String source) importOne;

  /// Publishes the transient top-bar line; null clears it.
  final void Function(String? status) onStatus;

  Future<MediaImportOutcome> run(
    List<String> sources, {
    required bool folder,
  }) async {
    var imported = 0;
    final failures = <String>[];
    try {
      for (var index = 0; index < sources.length; index += 1) {
        final source = sources[index];
        onStatus(importStatusLabel(index + 1, sources.length, source));
        final attempt = await importOne(source);
        if (attempt.aborted) {
          return MediaImportOutcome(
            imported: imported,
            total: sources.length,
            failures: failures,
            folder: folder,
            aborted: true,
          );
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
    return MediaImportOutcome(
      imported: imported,
      total: sources.length,
      failures: failures,
      folder: folder,
      aborted: false,
    );
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
}) {
  final noun = folder ? 'folder files' : 'selected files';
  if (failures.isNotEmpty) {
    final first = failures.first;
    final failureSummary = first.length > 180
        ? '${first.substring(0, 177)}…'
        : first;
    return 'Imported $imported of $total $noun. '
        '${failures.length} could not be imported. $failureSummary';
  }
  if (total > 1) return 'Imported $imported $noun.';
  return null;
}
