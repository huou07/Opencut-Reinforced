import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/cloud_voice_service.dart' show wrapPcmAsWav;
import 'package:or_app/production/local_tts_service.dart';

void main() {
  test('a bare host:port and trailing slashes become one origin', () {
    expect(
      LocalTtsService.normalizeBaseUrl(''),
      LocalTtsService.defaultBaseUrl,
    );
    expect(
      LocalTtsService.normalizeBaseUrl(' 127.0.0.1:8000/ '),
      'http://127.0.0.1:8000',
    );
    expect(
      LocalTtsService.endpoint('localhost:9000', '/v1/voices').toString(),
      'http://localhost:9000/v1/voices',
    );
  });

  test('speech request uses the OpenAI shape and asks for a WAV', () {
    final request = localSpeechRequest(
      baseUrl: '127.0.0.1:8000',
      text: 'Xin chào',
      voiceId: 'Mai Anh',
      sampleRate: 48000,
    );
    expect(request.uri.toString(), 'http://127.0.0.1:8000/v1/audio/speech');
    expect(request.headers['Content-Type'], 'application/json');
    final body = jsonDecode(utf8.decode(request.body)) as Map<String, Object?>;
    expect(body['input'], 'Xin chào');
    expect(body['voice'], 'Mai Anh');
    expect(body['response_format'], 'wav');
    expect(body['sample_rate'], 48000);
  });

  test('enrolment is a multipart form with the clip and denoise flag', () {
    final request = localVoiceEnrollmentRequest(
      baseUrl: 'http://127.0.0.1:8000',
      name: 'Giọng tôi',
      filename: 'reference.wav',
      clip: [82, 73, 70, 70],
    );
    expect(request.uri.path, '/v1/voices');
    final contentType = request.headers['Content-Type']!;
    final boundary = RegExp(r'boundary=(.+)$')
        .firstMatch(contentType)
        ?.group(1);
    expect(boundary, isNotNull);
    final text = utf8.decode(request.body, allowMalformed: true);
    expect(text, contains('name="name"\r\n\r\nGiọng tôi'));
    expect(text, contains('name="denoise"\r\n\r\ntrue'));
    expect(text, contains('filename="reference.wav"'));
    expect(text, contains('Content-Type: audio/wav'));
    expect(text, endsWith('\r\n--$boundary--\r\n'));
  });

  test('probe, enrol and synthesize against a real loopback server', () async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    addTearDown(() => server.close(force: true));
    final seenPaths = <String>[];
    server.listen((request) async {
      seenPaths.add(request.uri.path);
      final response = request.response;
      switch (request.uri.path) {
        case '/health':
          response.headers.contentType = ContentType.json;
          response.write(
            jsonEncode({
              'status': 'ok',
              'backend': 'onnx',
              'sample_rate': 24000,
              'max_streams': 1,
              'active': 0,
              'waiting': 0,
            }),
          );
        case '/v1/voices':
          response.headers.contentType = ContentType.json;
          response.write(
            request.method == 'POST'
                ? jsonEncode({'id': 'My voice', 'name': 'My voice'})
                : jsonEncode({
                    'object': 'list',
                    'data': [
                      {'id': 'Mai Anh', 'name': 'Mai Anh'},
                    ],
                  }),
          );
        case '/v1/audio/speech':
          response.add(
            wrapPcmAsWav(
              List<int>.filled(4800, 0),
              sampleRate: 24000,
              channels: 1,
            ),
          );
        default:
          response.statusCode = 404;
      }
      await response.close();
    });

    final base = '127.0.0.1:${server.port}';
    final service = LocalTtsService();
    final status = await service.probe(base);
    expect(status.health.backend, 'onnx');
    expect(status.health.sampleRate, 24000);
    expect(status.voices.single.id, 'Mai Anh');

    final directory = await Directory.systemTemp.createTemp('or-local-tts-');
    addTearDown(() => directory.delete(recursive: true));
    final clip = File('${directory.path}/ref.wav')
      ..writeAsBytesSync(List<int>.filled(64, 0));
    final id = await service.enrollVoice(
      baseUrl: base,
      name: 'My voice',
      referencePath: clip.path,
    );
    expect(id, 'My voice');

    final output = '${directory.path}/line.wav';
    final durationMs = await service.synthesize(
      baseUrl: base,
      text: 'Xin chào',
      voiceId: id,
      outputPath: output,
    );
    expect(File(output).existsSync(), isTrue);
    expect(durationMs, 100);
    expect(
      seenPaths,
      containsAll(['/health', '/v1/voices', '/v1/audio/speech']),
    );
  });

  test('preview synthesises one sample and plays the file it wrote', () async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    addTearDown(() => server.close(force: true));
    server.listen((request) async {
      final response = request.response;
      if (request.uri.path == '/v1/audio/speech') {
        response.add(
          wrapPcmAsWav(List<int>.filled(2400, 0), sampleRate: 24000, channels: 1),
        );
      } else {
        response.statusCode = 404;
      }
      await response.close();
    });

    final directory = await Directory.systemTemp.createTemp('or-local-preview-');
    addTearDown(() => directory.delete(recursive: true));
    final played = <String>[];
    final service = LocalTtsService(playAudio: (path) async => played.add(path));
    final output = '${directory.path}/preview.wav';

    final path = await service.preview(
      baseUrl: '127.0.0.1:${server.port}',
      text: 'Xin chào',
      voiceId: 'Mai Anh',
      outputPath: output,
    );

    expect(path, output);
    expect(File(output).lengthSync(), greaterThan(44));
    expect(played, [output]);
  });

  test('a restarted server\'s missing voice is worth one re-enrol retry', () {
    LocalTtsException http(int status, String message) =>
        LocalTtsException('HTTP_$status', message);

    // 404 is the OpenAI-shaped answer for an unknown voice.
    expect(retryAfterReEnroll(http(404, 'Voice not found')), isTrue);
    // Servers that answer 400 with the voice in the text.
    expect(
      retryAfterReEnroll(http(400, 'Unknown voice: Mai Anh')),
      isTrue,
    );
    // A generic 400 (bad text) is not a voice problem but the retry is
    // bounded to one attempt, so erring towards recovery is safe.
    expect(retryAfterReEnroll(http(400, 'input is empty')), isTrue);
    // Transport problems must not be retried blindly.
    expect(
      retryAfterReEnroll(
        const LocalTtsException('OFFLINE', 'No local speech server.'),
      ),
      isFalse,
    );
    expect(
      retryAfterReEnroll(
        const LocalTtsException('TIMEOUT', 'The local server did not answer.'),
      ),
      isFalse,
    );
    expect(
      retryAfterReEnroll(
        const LocalTtsException('HTTP_500', 'internal voice error'),
      ),
      isFalse,
    );
    // The dialog's own guards still win: no clip, no retry.
  });
}
