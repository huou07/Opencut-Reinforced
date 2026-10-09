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

  test('Android media picker returns multiple validated SAF URIs', () async {
    var pickerCalled = false;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          expect(call.method, 'openMedia');
          pickerCalled = true;
          return {
            'sourceUris': [
              'content://com.example.documents/document/video%3A42',
              'content://com.example.documents/document/video%3A43',
            ],
          };
        });
    final picker = AndroidSafProjectPicker(channel: channel);

    expect(picker.supportsMediaImport, isTrue);
    expect(await picker.openMediaSources(), [
      'content://com.example.documents/document/video%3A42',
      'content://com.example.documents/document/video%3A43',
    ]);
    expect(pickerCalled, isTrue);
  });

  test(
    'Android caption picker stages and cleans a private working file',
    () async {
      final calls = <MethodCall>[];
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            calls.add(call);
            return switch (call.method) {
              'openCaptionFile' => {
                'workingPath': '/data/user/0/or_app/cache/or-captions/cue.srt',
              },
              _ => null,
            };
          });
      final picker = AndroidSafProjectPicker(channel: channel);

      final path = await picker.openCaptionFile();
      await picker.cleanupCaptionFile(path!);

      expect(path, '/data/user/0/or_app/cache/or-captions/cue.srt');
      expect(calls.map((call) => call.method), [
        'openCaptionFile',
        'deleteCaptionFile',
      ]);
      expect(calls.last.arguments, {'workingPath': path});
    },
  );

  test(
    'Android caption export stages locally and publishes through SAF',
    () async {
      final calls = <MethodCall>[];
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            calls.add(call);
            return switch (call.method) {
              'createCaptionExport' => {
                'workingPath':
                    '/data/user/0/or_app/cache/or-caption-exports/captions.vtt',
                'documentUri':
                    'content://com.example.documents/document/captions',
              },
              _ => null,
            };
          });
      final picker = AndroidSafProjectPicker(channel: channel);

      final path = await picker.saveCaptionPath(suggestedName: 'captions.vtt');
      await picker.publishCaptionPath(path!);

      expect(path, '/data/user/0/or_app/cache/or-caption-exports/captions.vtt');
      expect(calls.map((call) => call.method), [
        'createCaptionExport',
        'publishCaptionExport',
      ]);
      expect(calls.first.arguments, {
        'suggestedName': 'captions.vtt',
        'extension': 'vtt',
        'mimeType': 'text/vtt',
      });
      expect(calls.last.arguments, {'workingPath': path});
    },
  );

  test('Android media picker rejects non-content selections', () async {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
          channel,
          (call) async => {
            'sourceUris': ['/data/user/0/or_app/cache/media.mkv'],
          },
        );
    final picker = AndroidSafProjectPicker(channel: channel);

    await expectLater(
      picker.openMediaSources(),
      throwsA(isA<ProjectSafStorageException>()),
    );
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

  test(
    'Android export stages locally and publishes through the selected SAF URI',
    () async {
      final calls = <MethodCall>[];
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            calls.add(call);
            return switch (call.method) {
              'createExport' => {
                'workingPath':
                    '/data/user/0/or_app/cache/or-exports/export.mkv',
                'documentUri':
                    'content://com.example.documents/document/export',
              },
              _ => null,
            };
          });
      final picker = AndroidSafProjectPicker(channel: channel);

      expect(picker.supportsExport, isTrue);
      final path = await picker.saveExportPath(suggestedName: 'demo.mkv');
      await picker.publishExportPath(path!);

      expect(path, '/data/user/0/or_app/cache/or-exports/export.mkv');
      expect(calls.map((call) => call.method), [
        'createExport',
        'publishExport',
      ]);
      expect(calls.last.arguments, {
        'workingPath': path,
        'documentUri': 'content://com.example.documents/document/export',
      });
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
