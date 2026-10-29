import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/cloud_voice_service.dart' show wrapPcmAsWav;
import 'package:or_app/production/local_tts_service.dart';
import 'package:or_app/production/voiceover_dialog.dart'
    show localVoiceHintText;

void main() {
  test('the empty local voice list gets exactly one useful hint', () {
    expect(
      localVoiceHintText(connected: false, hasVoices: false),
      contains('Press Connect'),
    );
    expect(
      localVoiceHintText(connected: true, hasVoices: false),
      contains('Enrol one from a clip'),
    );
    expect(localVoiceHintText(connected: false, hasVoices: true), isNull);
    expect(localVoiceHintText(connected: true, hasVoices: true), isNull);
  });

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
    expect(
      LocalTtsService.normalizeBaseUrl('http://127.0.0.1:8000/v1/'),
      'http://127.0.0.1:8000',
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

  test('an unwritable output path fails with a readable error', () async {
    // The service owns the clip I/O, so a disk error must arrive as
    // LocalTtsException instead of a raw FileSystemException escaping the
    // dialog's error ladder.
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    addTearDown(() => server.close(force: true));
    server.listen((request) async {
      final response = request.response;
      response.headers.contentType = ContentType('audio', 'wav');
      response.add(
        wrapPcmAsWav(
          List<int>.filled(2400, 0),
          sampleRate: 24000,
          channels: 1,
        ),
      );
      await response.close();
    });
    final service = LocalTtsService();
    await expectLater(
      service.synthesize(
        baseUrl: 'http://127.0.0.1:${server.port}',
        text: 'One line.',
        voiceId: 'voice',
        outputPath: '/dev/null/preview.wav',
      ),
      throwsA(
        isA<LocalTtsException>().having(
          (error) => error.code,
          'code',
          'WRITE_FAILED',
        ),
      ),
    );
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

  test('a reference clip declares the format it is really in', () {
    // The dialog accepts MP3, FLAC, OGG and M4A clips too and uploads the file
    // unchanged, so the part must not claim audio/wav for all of them: a server
    // that trusts the header decodes an MP3 as PCM and refuses the enrolment.
    expect(localReferenceContentType('reference.mp3'), 'audio/mpeg');
    expect(localReferenceContentType('reference.flac'), 'audio/flac');
    expect(localReferenceContentType('reference.ogg'), 'audio/ogg');
    expect(localReferenceContentType('reference.m4a'), 'audio/mp4');
    expect(localReferenceContentType('reference.wav'), 'audio/wav');
    expect(localReferenceContentType('reference.MP3'), 'audio/mpeg');
    expect(localReferenceContentType('reference.aiff'), 'audio/wav');

    final request = localVoiceEnrollmentRequest(
      baseUrl: 'http://127.0.0.1:8000',
      name: 'Giọng tôi',
      filename: 'reference.mp3',
      clip: [73, 68, 51],
    );
    final text = utf8.decode(request.body, allowMalformed: true);
    expect(text, contains('filename="reference.mp3"'));
    expect(text, contains('Content-Type: audio/mpeg'));
    expect(text, isNot(contains('Content-Type: audio/wav')));
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

  test('the play hook receives the file just synthesised', () async {
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

    final duration = await service.synthesize(
      baseUrl: '127.0.0.1:${server.port}',
      text: 'Xin chào',
      voiceId: 'Mai Anh',
      outputPath: output,
    );
    await service.play(output);

    expect(duration, 50);
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

  test('a forgotten voice re-enrols against the endpoint that failed', () async {
    final bases = <String>[];
    var attempts = 0;
    var enrolled = 0;
    final duration = await synthesizeWithVoiceRecovery(
      base: 'http://127.0.0.1:9000',
      hasReferenceClip: true,
      reEnroll: (base) async {
        bases.add(base);
        enrolled += 1;
        return 'recovered';
      },
      attempt: () async {
        attempts += 1;
        if (attempts == 1) {
          throw const LocalTtsException('HTTP_404', 'unknown voice');
        }
        return 420;
      },
    );
    expect(duration, 420);
    expect(bases, ['http://127.0.0.1:9000']);
    expect(enrolled, 1);
    expect(attempts, 2);
  });

  test('without a reference clip the failure is not retried', () async {
    expect(
      () => synthesizeWithVoiceRecovery(
        base: 'http://127.0.0.1:9000',
        hasReferenceClip: false,
        reEnroll: (base) async {
          fail('must not re-enrol');
        },
        attempt: () async {
          throw const LocalTtsException('HTTP_404', 'unknown voice');
        },
      ),
      throwsA(isA<LocalTtsException>()),
    );
  });

  test('a silent health check times out instead of waiting', () async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    addTearDown(() => server.close(force: true));
    server.listen((request) async {
      // The process is alive but never answers.
      await Future<void>.delayed(const Duration(milliseconds: 300));
      request.response.statusCode = 200;
      await request.response.close();
    });

    final service = LocalTtsService(
      metadataTimeout: const Duration(milliseconds: 50),
    );
    await expectLater(
      service.probe('http://127.0.0.1:${server.port}'),
      throwsA(
        isA<LocalTtsException>()
            .having((error) => error.code, 'code', 'TIMEOUT')
            .having(
              (error) => error.message,
              'message',
              contains('did not answer in time'),
            ),
      ),
    );
  });

  test('a non-voice failure is rethrown unchanged', () async {
    expect(
      () => synthesizeWithVoiceRecovery(
        base: 'http://127.0.0.1:9000',
        hasReferenceClip: true,
        reEnroll: (base) async => fail('must not re-enrol'),
        attempt: () async {
          throw const LocalTtsException('HTTP_500', 'the server failed');
        },
      ),
      throwsA(isA<LocalTtsException>()),
    );
  });

  test('an automatic re-enrolment needs consent and a reference clip', () {
    expect(
      mayReEnrollClonedVoice(
        acknowledged: true,
        referencePath: '/tmp/ref.wav',
      ),
      isTrue,
    );
    // Consent withdrawn: the clip is still on disk, but re-cloning it is not
    // this run's decision to make.
    expect(
      mayReEnrollClonedVoice(
        acknowledged: false,
        referencePath: '/tmp/ref.wav',
      ),
      isFalse,
    );
    expect(
      mayReEnrollClonedVoice(acknowledged: true, referencePath: '  '),
      isFalse,
    );
  });

  test('a forgotten voice without consent names the missing step', () {
    const unknown = LocalTtsException('HTTP_404', 'Voice not found');
    LocalTtsException explain({
      required bool canReEnrol,
      String referencePath = '/tmp/ref.wav',
      LocalTtsException error = unknown,
    }) {
      return explainLocalFailure(
        error: error,
        canReEnrol: canReEnrol,
        referencePath: referencePath,
      );
    }

    final advice = explain(canReEnrol: false);
    expect(advice.code, 'CLONE_CONSENT');
    expect(advice.message, contains('rights acknowledgement'));
    expect(advice.message, contains('Enrol voice'));

    // With consent the recovery helper heals the line, so the failure is not
    // rewritten into advice the user cannot act on.
    expect(identical(explain(canReEnrol: true), unknown), isTrue);
    // No clip means nothing to enrol: the server's own answer stands.
    expect(
      identical(explain(canReEnrol: false, referencePath: ''), unknown),
      isTrue,
    );
    // A wedged server is not a consent problem.
    const offline = LocalTtsException(
      'OFFLINE',
      'No local speech server answered at 127.0.0.1:8000.',
    );
    expect(
      identical(explain(canReEnrol: false, error: offline), offline),
      isTrue,
    );
  });
}
