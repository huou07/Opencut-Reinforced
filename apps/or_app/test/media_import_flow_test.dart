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
}
