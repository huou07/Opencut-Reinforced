import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'voiceover_service.dart' show wavDurationMsFromBytes;

/// One voice a local speech server offers, including cloned ones.
class LocalTtsVoice {
  const LocalTtsVoice({
    required this.id,
    required this.name,
    this.description = '',
  });

  final String id;
  final String name;
  final String description;

  String get label => description.isEmpty ? name : '$name — $description';
}

class LocalTtsHealth {
  const LocalTtsHealth({
    required this.backend,
    required this.sampleRate,
    required this.maxStreams,
    required this.active,
    required this.waiting,
  });

  final String backend;
  final int sampleRate;
  final int maxStreams;
  final int active;
  final int waiting;

  String get summary =>
      '$backend · ${sampleRate ~/ 1000} kHz · $active/$maxStreams busy'
      '${waiting > 0 ? ' · $waiting waiting' : ''}';
}

class LocalTtsStatus {
  const LocalTtsStatus({required this.health, required this.voices});

  final LocalTtsHealth health;
  final List<LocalTtsVoice> voices;
}

class LocalTtsException implements Exception {
  const LocalTtsException(this.code, this.message);

  final String code;
  final String message;

  @override
  String toString() => message;
}

/// An HTTP request built without sending it, so it can be asserted in a test.
class LocalHttpRequest {
  const LocalHttpRequest({
    required this.uri,
    required this.headers,
    required this.body,
  });

  final Uri uri;
  final Map<String, String> headers;
  final List<int> body;
}

/// Keyless narration through a local server that speaks the OpenAI speech shape.
///
/// The app bundles no engine and no weights: any loopback server that answers
/// `POST /v1/audio/speech` (VieNeu-TTS, Chatterbox, Kokoro, …) works once the
/// user starts it. Voice cloning is `POST /v1/voices` with a 3–8 s reference
/// clip. Nothing is uploaded and no key is needed on loopback.
class LocalTtsService {
  LocalTtsService({HttpClient Function()? httpClientFactory})
    : _httpClientFactory = httpClientFactory ?? HttpClient.new;

  static const defaultBaseUrl = 'http://127.0.0.1:8000';

  final HttpClient Function() _httpClientFactory;

  /// Accepts `127.0.0.1:8000`, `http://localhost:8000/` or an empty value
  /// (the default loopback server) and returns a scheme-qualified origin.
  static String normalizeBaseUrl(String raw) {
    var value = raw.trim();
    if (value.isEmpty) return defaultBaseUrl;
    if (!value.contains('://')) value = 'http://$value';
    while (value.endsWith('/')) {
      value = value.substring(0, value.length - 1);
    }
    return value;
  }

  static Uri endpoint(String baseUrl, String path) =>
      Uri.parse('${normalizeBaseUrl(baseUrl)}$path');

  /// Reads `/health` and `/v1/voices` so one round-trip validates the server.
  Future<LocalTtsStatus> probe(String baseUrl) async {
    final origin = normalizeBaseUrl(baseUrl);
    final health = await _getJson(endpoint(origin, '/health'));
    final voices = await listVoices(origin);
    int number(Object? value) => value is num ? value.toInt() : 0;
    return LocalTtsStatus(
      health: LocalTtsHealth(
        backend: health['backend'] is String
            ? health['backend'] as String
            : 'unknown',
        sampleRate: number(health['sample_rate']),
        maxStreams: number(health['max_streams']),
        active: number(health['active']),
        waiting: number(health['waiting']),
      ),
      voices: voices,
    );
  }

  Future<List<LocalTtsVoice>> listVoices(String baseUrl) async {
    final body = await _getJson(endpoint(baseUrl, '/v1/voices'));
    final data = body['data'];
    if (data is! List) return const [];
    final voices = <LocalTtsVoice>[];
    for (final entry in data) {
      if (entry is! Map) continue;
      final id = entry['id'];
      final name = entry['name'];
      if (id is! String || id.isEmpty) continue;
      final description = entry['description'];
      voices.add(
        LocalTtsVoice(
          id: id,
          name: name is String && name.isNotEmpty ? name : id,
          description: description is String ? description : '',
        ),
      );
    }
    return voices;
  }

  /// Speaks one line into [outputPath] and returns its duration in milliseconds.
  Future<int> synthesize({
    required String baseUrl,
    required String text,
    required String voiceId,
    required String outputPath,
    int sampleRate = 0,
  }) async {
    if (text.trim().isEmpty) {
      throw const LocalTtsException('EMPTY_TEXT', 'There is nothing to speak.');
    }
    final request = localSpeechRequest(
      baseUrl: baseUrl,
      text: text,
      voiceId: voiceId,
      sampleRate: sampleRate,
    );
    final bytes = await _request(
      method: 'POST',
      uri: request.uri,
      headers: request.headers,
      body: request.body,
      timeout: const Duration(minutes: 5),
    );
    if (!_isWav(bytes)) {
      throw const LocalTtsException(
        'INVALID_AUDIO',
        'The local server did not return a WAV file this build can import.',
      );
    }
    final file = File(outputPath);
    await file.parent.create(recursive: true);
    await file.writeAsBytes(bytes, flush: true);
    return wavDurationMsFromBytes(bytes);
  }

  /// Enrols a reference clip, so the server can clone that voice by name.
  ///
  /// Enrolment lives in the server process, not on disk; the caller keeps the
  /// clip path so it can enrol again after the server restarts.
  Future<String> enrollVoice({
    required String baseUrl,
    required String name,
    required String referencePath,
    bool denoise = true,
  }) async {
    final trimmed = name.trim();
    if (trimmed.isEmpty) {
      throw const LocalTtsException(
        'NAME_MISSING',
        'Give the cloned voice a name first.',
      );
    }
    final clip = File(referencePath);
    if (!await clip.exists()) {
      throw const LocalTtsException(
        'CLIP_MISSING',
        'The reference clip no longer exists. Choose it again.',
      );
    }
    final request = localVoiceEnrollmentRequest(
      baseUrl: baseUrl,
      name: trimmed,
      filename: _clipFilename(referencePath),
      clip: await clip.readAsBytes(),
      denoise: denoise,
    );
    final bytes = await _request(
      method: 'POST',
      uri: request.uri,
      headers: request.headers,
      body: request.body,
      timeout: const Duration(minutes: 2),
    );
    final decoded = _tryDecodeJson(bytes);
    final id = decoded?['id'];
    return id is String && id.isNotEmpty ? id : trimmed;
  }

  Future<Map<String, Object?>> _getJson(Uri uri) async {
    final bytes = await _request(
      method: 'GET',
      uri: uri,
      headers: const {'Accept': 'application/json'},
      timeout: const Duration(seconds: 4),
    );
    final decoded = _tryDecodeJson(bytes);
    if (decoded == null) {
      throw const LocalTtsException(
        'INVALID_RESPONSE',
        'The local server answered with something other than JSON.',
      );
    }
    return decoded;
  }

  Future<Uint8List> _request({
    required String method,
    required Uri uri,
    required Map<String, String> headers,
    List<int>? body,
    required Duration timeout,
  }) async {
    final client = _httpClientFactory();
    client.connectionTimeout = const Duration(seconds: 3);
    try {
      final request = await client.openUrl(method, uri).timeout(timeout);
      headers.forEach((name, value) => request.headers.set(name, value));
      if (body != null) request.add(body);
      final response = await request.close().timeout(timeout);
      final bytes = await response
          .fold<BytesBuilder>(BytesBuilder(), (builder, chunk) {
            builder.add(chunk);
            return builder;
          })
          .timeout(timeout);
      final data = bytes.takeBytes();
      if (response.statusCode < 200 || response.statusCode >= 300) {
        throw LocalTtsException(
          'HTTP_${response.statusCode}',
          _serverMessage(uri, data, response.statusCode),
        );
      }
      return data;
    } on LocalTtsException {
      rethrow;
    } on SocketException {
      throw LocalTtsException(
        'OFFLINE',
        'No local speech server answered at ${uri.host}:${uri.port}. '
            'Start one (for example VieNeu-TTS) and try again.',
      );
    } on TimeoutException {
      throw LocalTtsException(
        'TIMEOUT',
        'The local server did not answer in time at ${uri.host}:${uri.port}.',
      );
    } on HttpException catch (error) {
      throw LocalTtsException(
        'NETWORK',
        'The request to ${uri.host}:${uri.port} failed: ${error.message}',
      );
    } finally {
      client.close(force: true);
    }
  }

  static String _serverMessage(Uri uri, List<int> bytes, int status) {
    final decoded = _tryDecodeJson(bytes);
    final error = decoded?['error'];
    final detail = error is Map
        ? (error['message'] is String ? error['message'] as String : null)
        : decoded?['detail'] is String
        ? decoded!['detail'] as String
        : null;
    if (detail != null && detail.trim().isNotEmpty) {
      final oneLine = detail.replaceAll(RegExp(r'\s+'), ' ').trim();
      return '${uri.host}: ${oneLine.length > 240 ? '${oneLine.substring(0, 240)}…' : oneLine}';
    }
    return '${uri.host} refused the request ($status).';
  }

  static Map<String, Object?>? _tryDecodeJson(List<int> bytes) {
    try {
      final decoded = jsonDecode(utf8.decode(bytes, allowMalformed: true));
      return decoded is Map<String, Object?> ? decoded : null;
    } on FormatException {
      return null;
    }
  }

  static bool _isWav(List<int> bytes) =>
      bytes.length >= 12 &&
      String.fromCharCodes(bytes.sublist(0, 4)) == 'RIFF' &&
      String.fromCharCodes(bytes.sublist(8, 12)) == 'WAVE';

  static String _clipFilename(String path) {
    final slash = path.lastIndexOf(RegExp(r'[/\\]'));
    final name = slash < 0 ? path : path.substring(slash + 1);
    final dot = name.lastIndexOf('.');
    final extension = dot < 0 ? 'wav' : name.substring(dot + 1).toLowerCase();
    return 'reference.${const {'wav', 'mp3', 'flac', 'ogg', 'm4a'}.contains(extension) ? extension : 'wav'}';
  }
}

/// `POST /v1/audio/speech`, the OpenAI shape every local server implements.
LocalHttpRequest localSpeechRequest({
  required String baseUrl,
  required String text,
  required String voiceId,
  int sampleRate = 0,
}) {
  final payload = <String, Object?>{
    'input': text,
    'voice': voiceId,
    'response_format': 'wav',
  };
  if (sampleRate > 0) payload['sample_rate'] = sampleRate;
  return LocalHttpRequest(
    uri: LocalTtsService.endpoint(baseUrl, '/v1/audio/speech'),
    headers: const {'Content-Type': 'application/json', 'Accept': 'audio/wav'},
    body: utf8.encode(jsonEncode(payload)),
  );
}

/// `POST /v1/voices`, multipart form: name, the reference clip and `denoise`.
LocalHttpRequest localVoiceEnrollmentRequest({
  required String baseUrl,
  required String name,
  required String filename,
  required List<int> clip,
  bool denoise = true,
}) {
  const boundary = 'orapp-local-tts-boundary';
  final body = BytesBuilder(copy: false);
  void field(String key, String value) {
    body.add(
      utf8.encode(
        '--$boundary\r\n'
        'Content-Disposition: form-data; name="$key"\r\n\r\n'
        '$value\r\n',
      ),
    );
  }

  field('name', name);
  field('denoise', denoise ? 'true' : 'false');
  body.add(
    utf8.encode(
      '--$boundary\r\n'
      'Content-Disposition: form-data; name="file"; filename="$filename"\r\n'
      'Content-Type: audio/wav\r\n\r\n',
    ),
  );
  body.add(clip);
  body.add(utf8.encode('\r\n--$boundary--\r\n'));
  return LocalHttpRequest(
    uri: LocalTtsService.endpoint(baseUrl, '/v1/voices'),
    headers: {'Content-Type': 'multipart/form-data; boundary=$boundary'},
    body: body.takeBytes(),
  );
}
