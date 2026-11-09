import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/source_path.dart';

/// Why symlinked sources cannot be exercised here, or null when they can.
///
/// Creating a link needs a privilege a plain Windows account does not hold
/// (`OS Error: A required privilege is not held by the client, errno = 1314`
/// unless Developer Mode or admin is on), so the group is skipped there
/// instead of failing. macOS and Linux always run it.
String? _symlinkSkipReason() {
  final probe = Directory.systemTemp.createTempSync('or-source-path-probe-');
  try {
    final target = Directory('${probe.path}/target')..createSync();
    Link('${probe.path}/link').createSync(target.path);
    return null;
  } on FileSystemException catch (error) {
    return 'this host cannot create symbolic links (${error.osError?.message ?? error.message}); '
        'the symlink cases need Developer Mode or an elevated account on Windows';
  } finally {
    probe.deleteSync(recursive: true);
  }
}

void main() {
  final symlinkSkip = _symlinkSkipReason();

  late Directory root;
  late Directory real;
  late String link;

  setUp(() {
    if (symlinkSkip != null) {
      return;
    }
    root = Directory.systemTemp.createTempSync('or-source-path-');
    real = Directory('${root.path}/library')..createSync();
    link = '${root.path}/linked';
    Link(link).createSync(real.path);
  });

  tearDown(() {
    if (symlinkSkip == null) {
      root.deleteSync(recursive: true);
    }
  });

  test(
    'a symlinked directory names the same file as its target',
    skip: symlinkSkip,
    () {
      File('${real.path}/export.mkv').writeAsStringSync('bytes');

      expect(
        sameSourcePath('$link/export.mkv', '${real.path}/export.mkv'),
        isTrue,
      );
      expect(
        canonicalSourcePath('$link/export.mkv'),
        File('${real.path}/export.mkv').resolveSymbolicLinksSync(),
      );
    },
  );

  test('different files stay different', skip: symlinkSkip, () {
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
