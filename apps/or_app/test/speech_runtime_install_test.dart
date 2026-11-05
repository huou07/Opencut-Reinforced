import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/updates/github_release.dart';
import 'package:or_app/updates/resource_center.dart';

/// The pinned URL the app ships, so the trusted-host rule is exercised for real.
const _url =
    'https://github.com/ggml-org/whisper.cpp/releases/download/b5454/'
    'whisper-bin-ubuntu-x64.tar.gz';

class _TempService extends ResourceCenterService {
  _TempService(this.directory);

  final Directory directory;

  @override
  Future<Directory> speechRuntimeDirectory() async => directory;
}

/// Streams fixed bytes as the response to one request, so the download path —
/// trusted host, size cap, streaming write, checksum — runs without a network.
class _CannedResponse extends Stream<List<int>> implements HttpClientResponse {
  _CannedResponse(this.bytes);

  final List<int> bytes;

  @override
  int get statusCode => HttpStatus.ok;

  @override
  StreamSubscription<List<int>> listen(
    void Function(List<int> event)? onData, {
    Function? onError,
    void Function()? onDone,
    bool? cancelOnError,
  }) {
    return Stream<List<int>>.value(bytes).listen(
      onData,
      onError: onError,
      onDone: onDone,
      cancelOnError: cancelOnError,
    );
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _CannedClient implements HttpClient {
  _CannedClient(this.bytes, {this.requests = 0});

  final List<int> bytes;
  int requests;

  @override
  Future<HttpClientRequest> getUrl(Uri url) async {
    requests++;
    expect(url.toString(), _url, reason: 'the pinned asset URL is used as-is');
    return _CannedRequest(bytes);
  }

  @override
  void close({bool force = false}) {}

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _CannedRequest implements HttpClientRequest {
  _CannedRequest(this.bytes);

  final List<int> bytes;

  @override
  bool followRedirects = false;

  @override
  final HttpHeaders headers = _CannedHeaders();

  @override
  Future<HttpClientResponse> close() async => _CannedResponse(bytes);

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _CannedHeaders implements HttpHeaders {
  @override
  void set(String name, Object value, {bool preserveHeaderCase = false}) {}

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  late Directory root;
  late Directory home;
  late File archive;

  setUp(() async {
    root = await Directory.systemTemp.createTemp('or-speech-target-');
    home = await Directory.systemTemp.createTemp('or-speech-home-');
    final stage = Directory('${home.path}/stage/whisper-bin-ubuntu-x64');
    await stage.create(recursive: true);
    await File('${stage.path}/whisper-cli').writeAsString('engine');
    await File('${stage.path}/or-whisper-provider').writeAsString('adapter');
    await File('${stage.path}/README.md').writeAsString('release notes');
    archive = File('${home.path}/fixture.tar.gz');
    final tar = await Process.run('tar', [
      '-czf',
      archive.path,
      '-C',
      Directory('${home.path}/stage').path,
      'whisper-bin-ubuntu-x64',
    ]);
    expect(tar.exitCode, 0, reason: '${tar.stderr}');
  });

  tearDown(() async {
    for (final directory in [root, home]) {
      if (directory.existsSync()) await directory.delete(recursive: true);
    }
  });

  Future<SpeechRuntimeAsset> assetFor(File file, {String? sha256}) async {
    return SpeechRuntimeAsset(
      url: _url,
      fileName: 'whisper-bin-ubuntu-x64.tar.gz',
      sizeBytes: await file.length(),
      sha256: sha256 ?? await sha256Of(file),
      zip: false,
    );
  }

  test(
    'a checksummed download installs, is recorded, and is removed',
    () async {
      final bytes = await archive.readAsBytes();
      final service = _TempService(root);
      final client = _CannedClient(bytes);
      late Directory installed;

      await HttpOverrides.runZoned(() async {
        installed = await service.installSpeechRuntime(
          asset: await assetFor(archive),
        );
      }, createHttpClient: (_) => client);

      expect(client.requests, 1);
      expect(installed.path, root.path);
      expect(File('${root.path}/whisper-cli').readAsStringSync(), 'engine');
      expect(File('${root.path}/or-whisper-provider').existsSync(), isTrue);
      expect(await service.hasSpeechInstallRecord(), isTrue);

      final removed = await service.removeSpeechRuntime();
      expect(removed, greaterThanOrEqualTo(3));
      expect(File('${root.path}/whisper-cli').existsSync(), isFalse);
      expect(await service.hasSpeechInstallRecord(), isFalse);
      // The temporary copy of the archive never survives the install.
      expect(
        home
            .listSync(recursive: true)
            .whereType<File>()
            .any((file) => file.path.endsWith('.part')),
        isFalse,
      );
    },
  );

  test('a tampered download is refused and installs nothing', () async {
    final bytes = await archive.readAsBytes();
    final service = _TempService(root);
    final tampered = await assetFor(archive, sha256: 'f' * 64);

    await expectLater(
      HttpOverrides.runZoned(
        () => service.installSpeechRuntime(asset: tampered),
        createHttpClient: (_) => _CannedClient(bytes),
      ),
      throwsA(
        isA<GithubReleaseException>().having(
          (error) => error.message,
          'message',
          contains('checksum does not match'),
        ),
      ),
    );

    expect(root.listSync(), isEmpty);
    expect(await service.hasSpeechInstallRecord(), isFalse);
  });
}
