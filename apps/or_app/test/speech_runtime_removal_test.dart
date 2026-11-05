import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/updates/github_release.dart';
import 'package:or_app/updates/resource_center.dart';

class _TempService extends ResourceCenterService {
  _TempService(this.directory);

  final Directory directory;

  @override
  Future<Directory> speechRuntimeDirectory() async => directory;
}

void main() {
  late Directory root;

  setUp(() async {
    root = await Directory.systemTemp.createTemp('or-speech-remove-');
  });

  tearDown(() async {
    if (root.existsSync()) await root.delete(recursive: true);
  });

  Future<void> recordInstall(List<String> files) async {
    for (final file in files) {
      final handle = File('${root.path}/$file');
      await handle.parent.create(recursive: true);
      await handle.writeAsString('binary');
    }
    await File('${root.path}/.or-speech-install.json').writeAsString(
      jsonEncode({'installed_at': '2026-11-05T00:00:00Z', 'files': files}),
    );
  }

  test('removal deletes only the recorded files', () async {
    await recordInstall(['whisper-cli', 'libor_whisper_provider.dylib']);
    // A model the user downloaded themselves, or a file they copied by hand.
    final model = File('${root.path}/models/ggml-base.bin');
    await model.parent.create(recursive: true);
    await model.writeAsString('weights');

    final removed = await _TempService(root).removeSpeechRuntime();

    expect(removed, 2);
    expect(File('${root.path}/whisper-cli').existsSync(), isFalse);
    expect(
      File('${root.path}/libor_whisper_provider.dylib').existsSync(),
      isFalse,
    );
    expect(model.existsSync(), isTrue);
    expect(File('${root.path}/.or-speech-install.json').existsSync(), isFalse);
  });

  test(
    'removal refuses a record it did not write and deletes nothing',
    () async {
      final file = File('${root.path}/whisper-cli');
      await file.writeAsString('binary');

      await expectLater(
        _TempService(root).removeSpeechRuntime(),
        throwsA(
          isA<GithubReleaseException>().having(
            (error) => error.message,
            'message',
            contains('No installed copy is recorded'),
          ),
        ),
      );
      expect(file.existsSync(), isTrue);
    },
  );

  test('a record that points outside the folder is ignored', () async {
    final outside = File(
      '${root.parent.path}/or-outside-${root.path.hashCode}',
    );
    await outside.writeAsString('keep me');
    addTearDown(() async {
      if (outside.existsSync()) await outside.delete();
    });
    await File('${root.path}/.or-speech-install.json').writeAsString(
      jsonEncode({
        'installed_at': '2026-11-05T00:00:00Z',
        'files': ['../${outside.uri.pathSegments.last}', '/etc/passwd'],
      }),
    );

    final removed = await _TempService(root).removeSpeechRuntime();

    expect(removed, 0);
    expect(outside.existsSync(), isTrue);
  });

  test('a record that was never written leaves a note, not a crash', () async {
    await File('${root.path}/.or-speech-install.json')
        .writeAsString('not json');

    await expectLater(
      _TempService(root).removeSpeechRuntime(),
      throwsA(
        isA<GithubReleaseException>().having(
          (error) => error.message,
          'message',
          contains('could not be read'),
        ),
      ),
    );
  });
}
