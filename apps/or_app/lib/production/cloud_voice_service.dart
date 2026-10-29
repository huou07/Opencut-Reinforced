import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/services.dart';

/// Cloud narration providers OR can use once the user adds a key.
///
/// A provider is a catalog entry, not a default: nothing is selected until the
/// user chooses it, and no voice name is baked into the product.
enum CloudVoiceProvider { elevenLabs, azure }

extension CloudVoiceProviderDescription on CloudVoiceProvider {
  String get wireName => switch (this) {
    CloudVoiceProvider.elevenLabs => 'elevenlabs',
    CloudVoiceProvider.azure => 'azure',
  };

  String get label => switch (this) {
    CloudVoiceProvider.elevenLabs => 'ElevenLabs',
    CloudVoiceProvider.azure => 'Microsoft Azure',
  };

  /// Where the user gets a key, shown in Settings.
  String get keyHint => switch (this) {
    CloudVoiceProvider.elevenLabs => 'xi-api-key from elevenlabs.io → Profile',
    CloudVoiceProvider.azure => 'Key from the Azure Speech resource',
  };
}

class CloudVoice {
  const CloudVoice({
    required this.provider,
    required this.id,
    required this.name,
    this.locale = '',
  });

  final CloudVoiceProvider provider;
  final String id;
  final String name;
  final String locale;

  String get label => locale.isEmpty ? name : '$name ($locale)';
}

class CloudVoiceException implements Exception {
  const CloudVoiceException(this.code, this.message);

  final String code;
  final String message;

  @override
  String toString() => message;
}

/// The macOS keychain surface owned by the app's Runner.
///
/// Only the account name crosses this boundary. The secret goes in and out as a
/// short-lived string and is never written to a project or a preference file.
class NarrationKeychain {
  const NarrationKeychain({
    MethodChannel? channel,
    this.timeout = const Duration(seconds: 10),
  }) : _channel = channel ?? const MethodChannel('or_narration');

  final MethodChannel _channel;

  /// macOS can hold a keychain query while it asks for access. A request that
  /// never answers must read as a failure with a next step, not as a spinner
  /// that never resolves.
  final Duration timeout;

  Future<String?> read(String account) async {
    try {
      return await _channel
          .invokeMethod<String>('keychainGet', {'account': account})
          .timeout(
            timeout,
            onTimeout: () => throw const CloudVoiceException(
              'KEYCHAIN_TIMEOUT',
              'macOS did not answer the keychain check. Unlock the login '
                  'keychain and try again.',
            ),
          );
    } on PlatformException catch (error) {
      throw CloudVoiceException(
        error.code,
        error.message ?? 'The keychain is unavailable.',
      );
    } on MissingPluginException {
      throw const CloudVoiceException(
        'KEYCHAIN_UNAVAILABLE',
        'The macOS keychain bridge is not available in this build.',
      );
    }
  }

  Future<void> write(String account, String value) async {
    try {
      await _channel.invokeMethod<bool>('keychainSet', {
        'account': account,
        'value': value,
      });
    } on PlatformException catch (error) {
      throw CloudVoiceException(
        error.code,
        error.message ?? 'The key could not be saved.',
      );
    } on MissingPluginException {
      throw const CloudVoiceException(
        'KEYCHAIN_UNAVAILABLE',
        'The macOS keychain bridge is not available in this build.',
      );
    }
  }

  Future<void> delete(String account) async {
    try {
      await _channel.invokeMethod<bool>('keychainDelete', {'account': account});
    } on PlatformException catch (error) {
      throw CloudVoiceException(
        error.code,
        error.message ?? 'The key could not be removed.',
      );
    } on MissingPluginException {
      throw const CloudVoiceException(
        'KEYCHAIN_UNAVAILABLE',
        'The macOS keychain bridge is not available in this build.',
      );
    }
  }

  Future<bool> has(String account) async =>
      (await read(account))?.isNotEmpty == true;
}

/// The two cloud providers implemented so far.
///
/// Requests are built as pure functions so their shape can be checked without a
/// network, and a failed request never repeats, retries or leaks the key in its
/// message.
class CloudTtsService {
  CloudTtsService({
    NarrationKeychain? keychain,
    HttpClient Function()? httpClientFactory,
  }) : _keychain = keychain ?? const NarrationKeychain(),
       _httpClientFactory = httpClientFactory ?? HttpClient.new;

  final NarrationKeychain _keychain;
  final HttpClient Function() _httpClientFactory;

  static const _elevenLabsBase = 'https://api.elevenlabs.io';

  /// The voice list a provider reports for this key.
  Future<List<CloudVoice>> listVoices(
    CloudVoiceProvider provider, {
    String azureRegion = '',
  }) async {
    switch (provider) {
      case CloudVoiceProvider.elevenLabs:
        final key = await _keychain.read(provider.wireName);
        if (key == null || key.isEmpty) {
          throw const CloudVoiceException(
            'KEY_MISSING',
            'Add an ElevenLabs key in Settings → Narration first.',
          );
        }
        final bytes = await _request(
          method: 'GET',
          uri: Uri.parse('$_elevenLabsBase/v1/voices'),
          headers: {'xi-api-key': key},
        );
        final decoded = jsonDecode(utf8.decode(bytes));
        if (decoded is! Map || decoded['voices'] is! List) {
          throw const CloudVoiceException(
            'INVALID_RESPONSE',
            'ElevenLabs returned a voice list this build does not understand.',
          );
        }
        return [
          for (final entry in (decoded['voices'] as List))
            if (entry is Map &&
                entry['voice_id'] is String &&
                entry['name'] is String)
              CloudVoice(
                provider: provider,
                id: entry['voice_id'] as String,
                name: entry['name'] as String,
                locale: _elevenLabsLocale(entry),
              ),
        ];
      case CloudVoiceProvider.azure:
        final key = await _keychain.read(provider.wireName);
        if (key == null || key.isEmpty) {
          throw const CloudVoiceException(
            'KEY_MISSING',
            'Add an Azure key in Settings → Narration first.',
          );
        }
        if (azureRegion.trim().isEmpty) {
          throw const CloudVoiceException(
            'REGION_MISSING',
            'Enter the Azure Speech region (for example southeastasia).',
          );
        }
        // Azure voice names are stable public identifiers, not per-account ids;
        // this small catalog is data, and the user still chooses one.
        return const [
          CloudVoice(
            provider: CloudVoiceProvider.azure,
            id: 'vi-VN-HoaiMyNeural',
            name: 'Hoai My',
            locale: 'vi-VN',
          ),
          CloudVoice(
            provider: CloudVoiceProvider.azure,
            id: 'vi-VN-NamMinhNeural',
            name: 'Nam Minh',
            locale: 'vi-VN',
          ),
          CloudVoice(
            provider: CloudVoiceProvider.azure,
            id: 'en-US-AvaMultilingualNeural',
            name: 'Ava',
            locale: 'en-US',
          ),
          CloudVoice(
            provider: CloudVoiceProvider.azure,
            id: 'en-US-AndrewMultilingualNeural',
            name: 'Andrew',
            locale: 'en-US',
          ),
        ];
    }
  }

  /// Renders [text] to a PCM WAV file at [outputPath] and returns its duration.
  Future<int> synthesize({
    required CloudVoiceProvider provider,
    required String voiceId,
    required String text,
    required String outputPath,
    String azureRegion = '',
  }) async {
    if (text.trim().isEmpty) {
      throw const CloudVoiceException(
        'EMPTY_TEXT',
        'There is nothing to speak.',
      );
    }
    final key = await _keychain.read(provider.wireName);
    if (key == null || key.isEmpty) {
      throw CloudVoiceException(
        'KEY_MISSING',
        'Add a ${provider.label} key in Settings → Narration first.',
      );
    }
    final Uint8List wav;
    switch (provider) {
      case CloudVoiceProvider.elevenLabs:
        final request = elevenLabsSpeechRequest(voiceId: voiceId, text: text);
        final pcm = await _request(
          method: 'POST',
          uri: request.uri,
          headers: {...request.headers, 'xi-api-key': key},
          body: request.body,
        );
        wav = wrapPcmAsWav(pcm, sampleRate: 44100, channels: 1);
      case CloudVoiceProvider.azure:
        if (azureRegion.trim().isEmpty) {
          throw const CloudVoiceException(
            'REGION_MISSING',
            'Enter the Azure Speech region (for example southeastasia).',
          );
        }
        final request = azureSpeechRequest(
          region: azureRegion.trim(),
          voiceId: voiceId,
          text: text,
        );
        final bytes = await _request(
          method: 'POST',
          uri: request.uri,
          headers: {...request.headers, 'Ocp-Apim-Subscription-Key': key},
          body: request.body,
        );
        if (bytes.length < 12 ||
            String.fromCharCodes(bytes.sublist(0, 4)) != 'RIFF' ||
            String.fromCharCodes(bytes.sublist(8, 12)) != 'WAVE') {
          throw const CloudVoiceException(
            'INVALID_AUDIO',
            'Azure did not return a PCM WAV this build can import.',
          );
        }
        wav = bytes;
    }
    final file = File(outputPath);
    await file.parent.create(recursive: true);
    await file.writeAsBytes(wav, flush: true);
    return wavDurationMs(wav);
  }

  /// Checks that a key can list voices (ElevenLabs) or render one short phrase
  /// (Azure, whose voice list is local data).
  Future<void> testKey(
    CloudVoiceProvider provider, {
    String azureRegion = '',
  }) async {
    if (provider == CloudVoiceProvider.elevenLabs) {
      await listVoices(provider);
      return;
    }
    final key = await _keychain.read(provider.wireName);
    if (key == null || key.isEmpty) {
      throw const CloudVoiceException(
        'KEY_MISSING',
        'Add an Azure key in Settings → Narration first.',
      );
    }
    if (azureRegion.trim().isEmpty) {
      throw const CloudVoiceException(
        'REGION_MISSING',
        'Enter the Azure Speech region (for example southeastasia).',
      );
    }
    final request = azureSpeechRequest(
      region: azureRegion.trim(),
      voiceId: 'vi-VN-HoaiMyNeural',
      text: 'Xin chào.',
    );
    await _request(
      method: 'POST',
      uri: request.uri,
      headers: {...request.headers, 'Ocp-Apim-Subscription-Key': key},
      body: request.body,
    );
  }

  /// Creates an instant voice clone from a local WAV sample.
  ///
  /// The caller is responsible for the consent acknowledgement; the API key
  /// still needs the provider account's own cloning permission.
  Future<CloudVoice> cloneVoice({
    required String name,
    required String samplePath,
    String description = 'Created from an Opencut Reinforced recording',
  }) async {
    final trimmedName = name.trim();
    if (trimmedName.isEmpty || trimmedName.length > 120) {
      throw const CloudVoiceException(
        'INVALID_NAME',
        'Give the voice a name of at most 120 characters.',
      );
    }
    final key = await _keychain.read(CloudVoiceProvider.elevenLabs.wireName);
    if (key == null || key.isEmpty) {
      throw const CloudVoiceException(
        'KEY_MISSING',
        'Add an ElevenLabs key in Settings → Narration first.',
      );
    }
    final sample = File(samplePath);
    final bytes = await sample.readAsBytes();
    if (bytes.length < 44 ||
        String.fromCharCodes(bytes.sublist(0, 4)) != 'RIFF' ||
        String.fromCharCodes(bytes.sublist(8, 12)) != 'WAVE') {
      throw const CloudVoiceException(
        'INVALID_SAMPLE',
        'The sample must be a PCM WAV recording.',
      );
    }
    if (bytes.length > 25 * 1024 * 1024) {
      throw const CloudVoiceException(
        'SAMPLE_TOO_LARGE',
        'The sample is larger than this build sends.',
      );
    }
    final boundary = '----or-voice-${DateTime.now().microsecondsSinceEpoch}';
    final body = BytesBuilder();
    void field(String fieldName, String value) {
      body.add(utf8.encode('--$boundary\r\n'));
      body.add(
        utf8.encode(
          'Content-Disposition: form-data; name="$fieldName"\r\n\r\n',
        ),
      );
      body.add(utf8.encode('$value\r\n'));
    }

    field('name', trimmedName);
    field('description', description);
    field('remove_background_noise', 'false');
    body.add(utf8.encode('--$boundary\r\n'));
    body.add(
      utf8.encode(
        'Content-Disposition: form-data; name="files"; filename="sample.wav"\r\n',
      ),
    );
    body.add(utf8.encode('Content-Type: audio/wav\r\n\r\n'));
    body.add(bytes);
    body.add(utf8.encode('\r\n--$boundary--\r\n'));

    final response = await _request(
      method: 'POST',
      uri: Uri.parse('${CloudTtsService._elevenLabsBase}/v1/voices/add'),
      headers: {
        'xi-api-key': key,
        'Content-Type': 'multipart/form-data; boundary=$boundary',
      },
      body: body.takeBytes(),
      timeout: const Duration(seconds: 60),
    );
    final decoded = jsonDecode(utf8.decode(response));
    final voiceId = decoded is Map ? decoded['voice_id'] : null;
    if (voiceId is! String || voiceId.isEmpty) {
      throw const CloudVoiceException(
        'INVALID_RESPONSE',
        'The provider did not return a voice id.',
      );
    }
    return CloudVoice(
      provider: CloudVoiceProvider.elevenLabs,
      id: voiceId,
      name: trimmedName,
    );
  }

  Future<Uint8List> _request({
    required String method,
    required Uri uri,
    required Map<String, String> headers,
    List<int>? body,
    Duration timeout = const Duration(seconds: 30),
  }) async {
    final client = _httpClientFactory();
    client.connectionTimeout = timeout;
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
        final detail = utf8
            .decode(data, allowMalformed: true)
            .replaceAll(RegExp(r'\s+'), ' ')
            .trim();
        throw CloudVoiceException(
          'HTTP_${response.statusCode}',
          detail.isEmpty
              ? '${uri.host} refused the request (${response.statusCode}).'
              : '${uri.host}: ${detail.length > 240 ? '${detail.substring(0, 240)}…' : detail}',
        );
      }
      return data;
    } on CloudVoiceException {
      rethrow;
    } on SocketException catch (error) {
      throw CloudVoiceException(
        'NETWORK',
        'Could not reach ${uri.host}: ${error.message}',
      );
    } on HttpException catch (error) {
      throw CloudVoiceException(
        'NETWORK',
        'The request to ${uri.host} failed: ${error.message}',
      );
    } finally {
      client.close(force: true);
    }
  }

  static String _elevenLabsLocale(Map entry) {
    final labels = entry['labels'];
    if (labels is Map && labels['language'] is String) {
      return labels['language'] as String;
    }
    final verified = entry['verified_languages'];
    if (verified is List && verified.isNotEmpty && verified.first is Map) {
      final language = (verified.first as Map)['language'];
      if (language is String) return language;
    }
    return '';
  }
}

class CloudSpeechRequest {
  const CloudSpeechRequest({
    required this.uri,
    required this.headers,
    required this.body,
  });

  final Uri uri;
  final Map<String, String> headers;
  final List<int> body;
}

/// ElevenLabs PCM output: 44.1 kHz mono signed 16-bit little-endian.
CloudSpeechRequest elevenLabsSpeechRequest({
  required String voiceId,
  required String text,
}) => CloudSpeechRequest(
  uri: Uri.parse(
    'https://api.elevenlabs.io/v1/text-to-speech/${Uri.encodeComponent(voiceId)}?output_format=pcm_44100',
  ),
  headers: {'Content-Type': 'application/json', 'Accept': 'audio/pcm'},
  body: utf8.encode(
    jsonEncode({
      // v3 is the multilingual generation that includes Vietnamese.
      'text': text,
      'model_id': 'eleven_v3',
    }),
  ),
);

CloudSpeechRequest azureSpeechRequest({
  required String region,
  required String voiceId,
  required String text,
}) {
  String escape(String value) => value
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&apos;');
  final ssml =
      '<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" '
      'xml:lang="vi-VN"><voice name="${escape(voiceId)}">'
      '${escape(text)}</voice></speak>';
  return CloudSpeechRequest(
    uri: Uri.parse(
      'https://${Uri.encodeComponent(region)}.tts.speech.microsoft.com/cognitiveservices/v1',
    ),
    headers: {
      'Content-Type': 'application/ssml+xml',
      'X-Microsoft-OutputFormat': 'riff-24khz-16bit-mono-pcm',
      'User-Agent': 'OpencutReinforced',
    },
    body: utf8.encode(ssml),
  );
}

/// Wraps raw signed 16-bit little-endian PCM in a canonical WAV container.
Uint8List wrapPcmAsWav(
  List<int> pcm, {
  required int sampleRate,
  required int channels,
}) {
  if (sampleRate <= 0 || channels <= 0) {
    throw const CloudVoiceException(
      'INVALID_AUDIO',
      'The sample rate is invalid.',
    );
  }
  final frameBytes = channels * 2;
  final dataBytes = pcm.length - (pcm.length % frameBytes);
  final header = ByteData(44);
  header.setUint32(0, 0x46464952, Endian.little); // RIFF
  header.setUint32(4, 36 + dataBytes, Endian.little);
  header.setUint32(8, 0x45564157, Endian.little); // WAVE
  header.setUint32(12, 0x20746d66, Endian.little); // 'fmt '
  header.setUint32(16, 16, Endian.little);
  header.setUint16(20, 1, Endian.little); // PCM
  header.setUint16(22, channels, Endian.little);
  header.setUint32(24, sampleRate, Endian.little);
  header.setUint32(28, sampleRate * frameBytes, Endian.little);
  header.setUint16(32, frameBytes, Endian.little);
  header.setUint16(34, 16, Endian.little);
  header.setUint32(36, 0x61746164, Endian.little); // data
  header.setUint32(40, dataBytes, Endian.little);
  final bytes = Uint8List(44 + dataBytes);
  bytes.setRange(0, 44, header.buffer.asUint8List());
  bytes.setRange(44, 44 + dataBytes, pcm);
  return bytes;
}

int wavDurationMs(List<int> bytes) {
  if (bytes.length < 44 ||
      String.fromCharCodes(bytes.sublist(0, 4)) != 'RIFF' ||
      String.fromCharCodes(bytes.sublist(8, 12)) != 'WAVE') {
    throw const CloudVoiceException(
      'INVALID_AUDIO',
      'The provider did not return a WAV file.',
    );
  }
  var offset = 12;
  int? channels;
  int? sampleRate;
  int? bits;
  int? dataBytes;
  while (offset + 8 <= bytes.length) {
    final id = String.fromCharCodes(bytes.sublist(offset, offset + 4));
    final size =
        bytes[offset + 4] |
        (bytes[offset + 5] << 8) |
        (bytes[offset + 6] << 16) |
        (bytes[offset + 7] << 24);
    final body = offset + 8;
    if (id == 'fmt ' && size >= 16) {
      channels = bytes[body + 2] | (bytes[body + 3] << 8);
      sampleRate =
          bytes[body + 4] |
          (bytes[body + 5] << 8) |
          (bytes[body + 6] << 16) |
          (bytes[body + 7] << 24);
      bits = bytes[body + 14] | (bytes[body + 15] << 8);
    } else if (id == 'data') {
      dataBytes = size < bytes.length - body ? size : bytes.length - body;
      break;
    }
    offset = body + size + (size & 1);
  }
  if (channels == null ||
      sampleRate == null ||
      bits == null ||
      dataBytes == null) {
    throw const CloudVoiceException(
      'INVALID_AUDIO',
      'The provider WAV has no PCM format.',
    );
  }
  if (bits != 16 || channels < 1 || sampleRate < 1) {
    throw const CloudVoiceException(
      'INVALID_AUDIO',
      'The provider WAV is not 16-bit PCM.',
    );
  }
  final frames = dataBytes ~/ (channels * 2);
  return (frames * 1000) ~/ sampleRate;
}
