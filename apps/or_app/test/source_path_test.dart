import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/source_path.dart';

void main() {
  late Directory root;
  late Directory real;
  late String link;

  setUp(() {
    root = Directory.systemTemp.createTempSync('or-source-path-');
    real = Directory('${root.path}/library')..createSync();
    link = '${root.path}/linked';
    Link(link).createSync(real.path);
  });

  tearDown(() => root.deleteSync(recursive: true));

  test('a symlinked directory names the same file as its target', () {
    File('${real.path}/export.mkv').writeAsStringSync('bytes');

    expect(sameSourcePath('$link/export.mkv', '${real.path}/export.mkv'), isTrue);
    expect(
      canonicalSourcePath('$link/export.mkv'),
      File('${real.path}/export.mkv').resolveSymbolicLinksSync(),
    );
  });

  test('different files stay different', () {
    File('${real.path}/first.mkv').writeAsStringSync('a');
    File('${real.path}/second.mkv').writeAsStringSync('b');

    expect(
      sameSourcePath('$link/first.mkv', '${real.path}/second.mkv'),
      isFalse,
    );
  });

  test('a missing file falls back to its absolute path', () {
    final missing = '${real.path}/gone.mkv';

    // A deleted source cannot be resolved, so it keeps the name the caller
    // gave: still a stable identity, never an exception.
    expect(canonicalSourcePath(missing), File(missing).absolute.path);
    expect(sameSourcePath(missing, missing), isTrue);
  });
}
