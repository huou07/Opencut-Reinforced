import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/project_file_picker.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('test.or_app/saf_storage');

  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  test(
    'SAF working copy sync carries its URI only as runtime metadata',
    () async {
      final calls = <MethodCall>[];
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            calls.add(call);
            return switch (call.method) {
              'createProject' => {
                'workingPath':
                    '/data/user/0/or_app/files/or-projects/demo.orproj',
                'documentUri': 'content://com.example.documents/document/demo',
              },
              'synchronizeProject' => {'verified': true},
              _ => null,
            };
          });
      final picker = AndroidSafProjectPicker(channel: channel);

      final path = await picker.saveProjectPath(suggestedName: 'demo.orproj');
      final result = await picker.synchronizeProjectPath(path!);

      expect(path, '/data/user/0/or_app/files/or-projects/demo.orproj');
      expect(result?.verified, isTrue);
      expect(calls.map((call) => call.method), [
        'createProject',
        'synchronizeProject',
      ]);
      expect(calls.last.arguments, {
        'workingPath': path,
        'documentUri': 'content://com.example.documents/document/demo',
      });
    },
  );

  test(
    'provider conflicts are surfaced without exposing provider details',
    () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            if (call.method == 'openProject') {
              return {
                'workingPath':
                    '/data/user/0/or_app/files/or-projects/demo.orproj',
                'documentUri': 'content://com.example.documents/document/demo',
              };
            }
            throw PlatformException(
              code: 'EXTERNAL_PROJECT_CHANGED',
              message: 'private provider detail',
            );
          });
      final picker = AndroidSafProjectPicker(channel: channel);
      final path = await picker.openProjectPath();

      await expectLater(
        picker.synchronizeProjectPath(path!),
        throwsA(
          isA<ProjectSafStorageException>().having(
            (error) => error.message,
            'message',
            contains('changed outside OR'),
          ),
        ),
      );
    },
  );

  test('non-empty create targets get a clear safe error', () async {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          throw PlatformException(
            code: 'PROJECT_NOT_EMPTY',
            message: 'private provider detail',
          );
        });
    final picker = AndroidSafProjectPicker(channel: channel);

    await expectLater(
      picker.saveProjectPath(suggestedName: 'demo.orproj'),
      throwsA(
        isA<ProjectSafStorageException>().having(
          (error) => error.message,
          'message',
          'The selected document is not empty. Choose an empty document to create a project.',
        ),
      ),
    );
  });
}
