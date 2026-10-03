import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_viewer_texture/or_viewer_texture.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('or_viewer_texture');
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  tearDown(() => messenger.setMockMethodCallHandler(channel, null));

  test('media binding carries generation and project ownership and checks rejection', () async {
    messenger.setMockMethodCallHandler(channel, (call) async {
      expect(call.method, 'setMediaSources');
      expect(call.arguments, {
        'sources': ['content://provider/document/clip'],
        'generation': 23,
        'owner': 'instance-1',
      });
      return false;
    });
    expect(
      await OrViewerTexture.setMediaSources(
        ['content://provider/document/clip'],
        generation: BigInt.from(23),
        owner: 'instance-1',
      ),
      isFalse,
    );
  });

  test('native binding errors remain actionable rather than successful empty registration', () async {
    messenger.setMockMethodCallHandler(
      channel,
      (_) async => throw PlatformException(
        code: 'MEDIA_SOURCE_NOT_SEEKABLE',
        message: 'Choose a seekable source.',
      ),
    );
    await expectLater(
      OrViewerTexture.setMediaSources(['content://provider/document/pipe']),
      throwsA(
        isA<PlatformException>().having(
          (error) => error.code,
          'code',
          'MEDIA_SOURCE_NOT_SEEKABLE',
        ),
      ),
    );
  });

  test('descriptor release cannot silently succeed when the native operation fails', () async {
    messenger.setMockMethodCallHandler(channel, (_) async => false);
    await expectLater(
      OrViewerTexture.clearMediaSources(),
      throwsA(
        isA<PlatformException>().having(
          (error) => error.code,
          'code',
          'MEDIA_SOURCE_UNAVAILABLE',
        ),
      ),
    );
  });
}
