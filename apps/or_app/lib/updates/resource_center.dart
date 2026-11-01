import 'dart:async';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:path_provider/path_provider.dart';

import '../assistant/assistant_settings.dart';
import 'github_release.dart';

/// What the packaged application actually carries on disk.
class RuntimeSnapshot {
  const RuntimeSnapshot({
    required this.packaged,
    required this.ffmpeg,
    required this.speechRuntime,
    required this.orCli,
    required this.assistantPath,
    required this.bundlePath,
  });

  final bool packaged;
  final bool ffmpeg;
  final bool speechRuntime;
  final bool orCli;
  final String? assistantPath;
  final String? bundlePath;

  bool get complete => ffmpeg && speechRuntime && orCli;
}

/// Small platform seam around the release API and the filesystem.
///
/// The panel takes one of these so widget tests never touch GitHub or the
/// user's Downloads folder.
class ResourceCenterService {
  const ResourceCenterService();

  Future<RuntimeSnapshot> inspectRuntime() async {
    final contents = _bundleContents();
    final macos = contents == null
        ? File(Platform.resolvedExecutable).parent
        : Directory('${contents.path}/MacOS');
    final packaged = contents != null;
    final frameworks = Directory(
      '${contents?.path ?? '${macos.path}/..'}/Frameworks',
    );
    var ffmpegLibraries = false;
    if (await frameworks.exists()) {
      try {
        await for (final entry in frameworks.list(
          recursive: true,
          followLinks: false,
        )) {
          final name = entry.uri.pathSegments.last;
          if (name.startsWith('libavutil.') && name.endsWith('.dylib')) {
            ffmpegLibraries = true;
            break;
          }
        }
      } on FileSystemException {
        ffmpegLibraries = false;
      }
    }
    return RuntimeSnapshot(
      packaged: packaged,
      ffmpeg:
          ffmpegLibraries &&
          await File('${macos.path}/ffmpeg').exists() &&
          await File('${macos.path}/ffprobe').exists(),
      speechRuntime:
          await File('${macos.path}/whisper-cli').exists() &&
          await File('${macos.path}/or-whisper-provider').exists(),
      orCli: await File('${macos.path}/or').exists(),
      assistantPath: await _findAssistant(),
      bundlePath: contents?.parent.path,
    );
  }

  /// Finds `Foo.app/Contents` even when Flutter reports a nested binary as the
  /// resolved executable. The packaged check must not depend on a single shape
  /// of [Platform.resolvedExecutable].
  Directory? _bundleContents() =>
      bundleContentsFor(Platform.resolvedExecutable);

  Future<String?> _findAssistant() async {
    try {
      return await resolveHarnessPath(
        settings: const AssistantSettings(),
        environment: Platform.environment,
      );
    } catch (_) {
      return null;
    }
  }

  Future<GithubRelease?> checkForUpdate(String currentVersion) =>
      fetchLatestGithubRelease(userAgent: 'Opencut-Reinforced/$currentVersion');

  Future<Directory> downloadDirectory() async {
    final downloads = await getDownloadsDirectory();
    if (downloads != null) return downloads;
    return getApplicationSupportDirectory();
  }

  /// Downloads and checksums the release DMG, returning the completed file.
  Future<File> downloadUpdate(
    GithubRelease release, {
    required Directory directory,
    void Function(int received, int total)? onProgress,
  }) async {
    final asset = release.macDmg;
    if (asset == null) {
      throw const GithubReleaseException(
        'This release does not contain a macOS disk image.',
      );
    }
    if (asset.sizeBytes <= 0) {
      throw const GithubReleaseException(
        'The release asset has no usable size.',
      );
    }
    final expected = _sha256FromDigest(asset.digest);
    final destination = File('${directory.path}/${asset.name}');
    final partial = File('${destination.path}.part');
    await directory.create(recursive: true);
    final client = HttpClient();
    var received = 0;
    IOSink? sink;
    try {
      var uri = Uri.parse(asset.url);
      HttpClientResponse? response;
      for (var redirect = 0; redirect <= 6; redirect++) {
        if (uri.scheme != 'https' || !_isGithubHost(uri.host)) {
          throw const GithubReleaseException(
            'The update URL is not a trusted GitHub HTTPS address.',
          );
        }
        final request = await client
            .getUrl(uri)
            .timeout(const Duration(seconds: 20));
        request.followRedirects = false;
        request.headers.set(HttpHeaders.userAgentHeader, 'Opencut-Reinforced');
        response = await request.close().timeout(const Duration(seconds: 30));
        if (response.statusCode >= 300 && response.statusCode < 400) {
          final location = response.headers.value(HttpHeaders.locationHeader);
          if (location == null || redirect == 6) {
            throw const GithubReleaseException(
              'The update download redirected incorrectly.',
            );
          }
          await response.drain<void>();
          uri = uri.resolve(location);
          continue;
        }
        break;
      }
      if (response == null || response.statusCode != HttpStatus.ok) {
        throw GithubReleaseException(
          'The update download failed with HTTP ${response?.statusCode ?? 0}.',
        );
      }
      sink = partial.openWrite(mode: FileMode.write);
      await for (final chunk in response) {
        received += chunk.length;
        if (received > asset.sizeBytes) {
          throw const GithubReleaseException(
            'The update download exceeded its advertised size.',
          );
        }
        sink.add(chunk);
        onProgress?.call(received, asset.sizeBytes);
      }
      await sink.flush();
      await sink.close();
      sink = null;
      if (received != asset.sizeBytes) {
        throw const GithubReleaseException(
          'The update download was incomplete.',
        );
      }
      if (expected != null) {
        final digest = await sha256.bind(partial.openRead()).first;
        if (digest.toString().toLowerCase() != expected) {
          throw const GithubReleaseException(
            'The update checksum does not match the published release.',
          );
        }
      }
      if (await destination.exists()) await destination.delete();
      await partial.rename(destination.path);
      return destination;
    } finally {
      await sink?.close();
      client.close(force: true);
      if (await partial.exists()) {
        try {
          await partial.delete();
        } on FileSystemException {
          // A failed cleanup must not hide the real download error.
        }
      }
    }
  }

  Future<void> openFile(File file) => _open(file.path);

  Future<void> openUrl(String url) async {
    final uri = Uri.tryParse(url);
    if (uri == null || uri.scheme != 'https') {
      throw const GithubReleaseException('Only HTTPS links can be opened.');
    }
    await _open(uri.toString());
  }

  Future<void> _open(String target) async {
    final command = Platform.isMacOS
        ? 'open'
        : Platform.isLinux
        ? 'xdg-open'
        : null;
    if (command == null) {
      throw const GithubReleaseException(
        'Opening this link is not supported on this platform.',
      );
    }
    final result = await Process.run(command, [target]);
    if (result.exitCode != 0) {
      throw GithubReleaseException(
        'The system could not open the downloaded item.',
      );
    }
  }
}

/// Finds `Foo.app/Contents` for a packaged executable path.
///
/// `Directory.uri` ends in an empty path segment, so callers must not compare
/// `pathSegments.last` to `Contents` directly. Exposed for the packaging test.
Directory? bundleContentsFor(String executablePath) {
  var directory = File(executablePath).parent;
  for (var depth = 0; depth < 8; depth++) {
    final segments = directory.uri.pathSegments.where(
      (part) => part.isNotEmpty,
    );
    final name = segments.isEmpty ? '' : segments.last;
    if (name == 'Contents' &&
        File('${directory.path}/MacOS/or_app').existsSync()) {
      return directory;
    }
    final parent = directory.parent;
    if (parent.path == directory.path) break;
    directory = parent;
  }
  return null;
}

String? _sha256FromDigest(String? digest) {
  if (digest == null) return null;
  final normalized = digest.toLowerCase();
  if (!normalized.startsWith('sha256:')) return null;
  final hex = normalized.substring('sha256:'.length);
  return RegExp(r'^[0-9a-f]{64}$').hasMatch(hex) ? hex : null;
}

bool _isGithubHost(String host) =>
    host == 'github.com' ||
    host == 'api.github.com' ||
    host == 'objects.githubusercontent.com' ||
    host == 'release-assets.githubusercontent.com' ||
    host.endsWith('.githubusercontent.com');
