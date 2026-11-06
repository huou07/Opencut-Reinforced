import 'package:flutter_test/flutter_test.dart';

import 'package:or_app/production/media_import_flow.dart';

void main() {
  test(
    'a folder batch walks every source and reports the final count',
    () async {
      final statuses = <String?>[];
      final seen = <String>[];
      final flow = MediaImportFlow(
        onStatus: statuses.add,
        importOne: (source) async {
          seen.add(source);
          return (imported: true, failure: null, aborted: false);
        },
      );

      final outcome = await flow.run([
        '/media/a.wav',
        '/media/b.wav',
        '/media/c.wav',
      ], folder: true);

      expect(seen, ['/media/a.wav', '/media/b.wav', '/media/c.wav']);
      expect(statuses, [
        'Importing 1 of 3 · a.wav',
        'Importing 2 of 3 · b.wav',
        'Importing 3 of 3 · c.wav',
        null,
      ]);
      expect(outcome.imported, 3);
      expect(outcome.failures, isEmpty);
      expect(outcome.aborted, isFalse);
      expect(outcome.summary, 'Imported 3 folder files.');
    },
  );

  test('one failure keeps the run going and names the first problem', () async {
    final flow = MediaImportFlow(
      onStatus: (_) {},
      importOne: (source) async => source.endsWith('b.wav')
          ? (imported: false, failure: 'Unreadable media', aborted: false)
          : (imported: true, failure: null, aborted: false),
    );

    final outcome = await flow.run([
      '/media/a.wav',
      '/media/b.wav',
      '/media/c.wav',
    ], folder: false);

    expect(outcome.imported, 2);
    expect(outcome.failures, ['Unreadable media']);
    expect(
      outcome.summary,
      'Imported 2 of 3 selected files. 1 could not be imported. Unreadable media',
    );
  });

  test('a changed project stops the loop and clears the status', () async {
    final statuses = <String?>[];
    final attempted = <String>[];
    final flow = MediaImportFlow(
      onStatus: statuses.add,
      importOne: (source) async {
        attempted.add(source);
        return attempted.length == 2
            ? (imported: false, failure: null, aborted: true)
            : (imported: true, failure: null, aborted: false);
      },
    );

    final outcome = await flow.run([
      '/media/a.wav',
      '/media/b.wav',
      '/media/c.wav',
    ], folder: false);

    expect(attempted, ['/media/a.wav', '/media/b.wav']);
    expect(outcome.imported, 1);
    expect(outcome.aborted, isTrue);
    expect(statuses.last, isNull);
  });

  test('a single imported file says nothing, and long names are shortened', () {
    expect(
      importSummary(imported: 1, total: 1, failures: const [], folder: false),
      isNull,
    );
    expect(
      importStatusLabel(
        1,
        1,
        '/Users/someone/recordings/a-very-long-recording-name-for-a-file.wav',
      ),
      'Importing a-very-long-recording-name-fo…',
    );
  });

  test(
    'the stop button ends the batch after the file already in flight',
    () async {
      final statuses = <String?>[];
      final attempted = <String>[];
      var stopped = false;
      final flow = MediaImportFlow(
        onStatus: statuses.add,
        cancelled: () => stopped,
        importOne: (source) async {
          attempted.add(source);
          // The user presses stop while the second file is being probed.
          if (attempted.length == 2) stopped = true;
          return (imported: true, failure: null, aborted: false);
        },
      );

      final outcome = await flow.run([
        '/media/a.wav',
        '/media/b.wav',
        '/media/c.wav',
      ], folder: true);

      expect(attempted, ['/media/a.wav', '/media/b.wav']);
      expect(outcome.imported, 2);
      expect(outcome.cancelled, isTrue);
      expect(outcome.aborted, isFalse);
      expect(statuses.last, isNull);
      expect(
        outcome.summary,
        'Imported 2 folder files, then stopped; 1 were not added.',
      );
    },
  );

  test('stopping before the first file says nothing was added', () async {
    final flow = MediaImportFlow(
      onStatus: (_) {},
      cancelled: () => true,
      importOne: (_) async => (imported: true, failure: null, aborted: false),
    );

    final outcome = await flow.run([
      '/media/a.wav',
      '/media/b.wav',
    ], folder: false);

    expect(outcome.imported, 0);
    expect(outcome.cancelled, isTrue);
    expect(
      outcome.summary,
      'Stopped before anything was imported. 2 selected files were not added.',
    );
  });

  test('a cancelled batch still names the files that failed', () async {
    final flow = MediaImportFlow(
      onStatus: (_) {},
      cancelled: () => true,
      importOne: (_) async => (imported: true, failure: null, aborted: false),
    );

    final outcome = await flow.run(['/media/a.wav'], folder: false);

    expect(
      outcome.summary,
      'Stopped before anything was imported. 1 selected files were not added.',
    );
    expect(
      importSummary(
        imported: 1,
        total: 4,
        failures: const ['Unreadable media'],
        folder: false,
        cancelled: true,
      ),
      'Imported 1 selected files, then stopped; 3 were not added. Unreadable media',
    );
  });
}
