import 'dart:async';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:file_selector/file_selector.dart' as picker;
import 'package:path_provider/path_provider.dart';

import '../assistant/assistant_settings.dart';
import '../support/host_platform.dart';
import 'github_release.dart';
import 'update_installer.dart';

/// What the packaged application actually carries on disk.
class RuntimeSnapshot {
  const RuntimeSnapshot({
    required this.packaged,
    required this.ffmpeg,
    required this.orCli,
    required this.assistantPath,
    required this.bundlePath,
    bool speechRuntime = true,
    bool? whisperEngine,
    bool? whisperProvider,
    this.speechRuntimeDirectory,
  }) : whisperEngine = whisperEngine ?? speechRuntime,
       whisperProvider = whisperProvider ?? speechRuntime;

  final bool packaged;
  final bool ffmpeg;
  final bool orCli;
  final String? assistantPath;
  final String? bundlePath;

  /// The Whisper command-line engine and the adapter the bridge spawns are two
  /// files: a package can carry one without the other, and the setup surface
  /// has to say which half is missing.
  final bool whisperEngine;
  final bool whisperProvider;

  /// Where the engine was found when it lives in the per-user runtime folder
  /// rather than beside the application binary.
  final String? speechRuntimeDirectory;

  bool get speechRuntime => whisperEngine && whisperProvider;

  bool get complete => ffmpeg && speechRuntime && orCli;
}

/// Describes what a desktop package actually carries beside its executable.
///
/// Every desktop package ships its runtime next to the application binary:
/// macOS inside `Contents/MacOS`, Linux and Windows beside the executable in
/// the install directory. A user who repaired a missing component without
/// reinstalling keeps it in the per-user runtime folder, so detection reads
/// both places and reports which half of the speech runtime is present.
Future<RuntimeSnapshot> describeRuntime({
  required Directory runtimeDirectory,
  Directory? frameworks,
  required String? bundlePath,
  required String? assistantPath,
  List<Directory> extraRuntimeDirectories = const [],
  HostPlatform? platform,
}) async {
  final host = platform ?? HostPlatform.current();
  final directories = <Directory>[runtimeDirectory, ...extraRuntimeDirectories];

  Future<String?> locate(String name, {bool packagedOnly = false}) async {
    for (final directory in packagedOnly ? [runtimeDirectory] : directories) {
      final candidate = host.join(
        directory.path,
        '$name${host.executableSuffix}',
      );
      if (await File(candidate).exists()) return candidate;
    }
    return null;
  }

  var ffmpegLibraries = frameworks == null;
  if (frameworks != null) {
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
  final whisper = await locate('whisper-cli');
  final provider = await locate('or-whisper-provider');
  final orCli = await locate('or');
  return RuntimeSnapshot(
    packaged: bundlePath != null,
    ffmpeg:
        ffmpegLibraries &&
        await locate('ffmpeg', packagedOnly: true) != null &&
        await locate('ffprobe', packagedOnly: true) != null,
    speechRuntime: whisper != null && provider != null,
    whisperEngine: whisper != null,
    whisperProvider: provider != null,
    orCli: orCli != null,
    assistantPath: assistantPath,
    bundlePath: bundlePath,
    speechRuntimeDirectory: whisper == null ? null : File(whisper).parent.path,
  );
}

/// A pinned, checksummed Whisper runtime a user can install without a package.
///
/// The archives come from the whisper.cpp release the app is built against, so
/// a development build and a repair path install the same engine.
class SpeechRuntimeAsset {
  const SpeechRuntimeAsset({
    required this.url,
    required this.fileName,
    required this.sizeBytes,
    required this.sha256,
    required this.zip,
  });

  final String url;
  final String fileName;
  final int sizeBytes;
  final String sha256;
  final bool zip;
}

const _whisperRelease =
    'https://github.com/ggml-org/whisper.cpp/releases/download/b5454';

/// The asset for this host, or null where the app has no downloadable build
/// (macOS ships the engine inside the application bundle).
SpeechRuntimeAsset? speechRuntimeAssetFor(
  HostPlatform host,
  String architecture,
) {
  final arm = architecture.toLowerCase().contains('arm');
  if (host.isWindows) {
    return arm
        ? const SpeechRuntimeAsset(
            url: '$_whisperRelease/whisper-bin-win-cpu-arm64.zip',
            fileName: 'whisper-bin-win-cpu-arm64.zip',
            sizeBytes: 4371193,
            sha256: '28c37e7b598c3d9bbfef94f3bd67f2ee6f12b7c86f3da5edcf5e4308d615ff6d',
            zip: true,
          )
        : const SpeechRuntimeAsset(
            url: '$_whisperRelease/whisper-bin-x64.zip',
            fileName: 'whisper-bin-x64.zip',
            sizeBytes: 8928640,
            sha256: '6ba69e3482d7826214f90a6a9c84ca07782aec1e1d0c6a7c30c994fd5d816ccb',
            zip: true,
          );
  }
  if (host.isLinux) {
    return arm
        ? const SpeechRuntimeAsset(
            url: '$_whisperRelease/whisper-bin-ubuntu-arm64.tar.gz',
            fileName: 'whisper-bin-ubuntu-arm64.tar.gz',
            sizeBytes: 4608377,
            sha256: '6b95ebfc60447df48e70ef00a73bdc3f41679ed2d01ef465827206a2ff325149',
            zip: false,
          )
        : const SpeechRuntimeAsset(
            url: '$_whisperRelease/whisper-bin-ubuntu-x64.tar.gz',
            fileName: 'whisper-bin-ubuntu-x64.tar.gz',
            sizeBytes: 10364195,
            sha256: 'a72becf15d7917f990f6313867a52638b82b7f9ef237fb0c980dac56a135781c',
            zip: false,
          );
  }
  return null;
}

/// Small platform seam around the release API and the filesystem.
///
/// The panel takes one of these so widget tests never touch GitHub or the
/// user's Downloads folder.
class ResourceCenterService {
  const ResourceCenterService();

  Future<RuntimeSnapshot> inspectRuntime() async {
    final contents = _bundleContents();
    final runtime = contents == null
        ? File(Platform.resolvedExecutable).parent
        : Directory('${contents.path}/MacOS');
    final frameworks = Directory(
      '${contents?.path ?? '${runtime.path}/..'}/Frameworks',
    );
    return describeRuntime(
      runtimeDirectory: runtime,
      frameworks: await frameworks.exists() ? frameworks : null,
      bundlePath: contents?.parent.path,
      assistantPath: await _findAssistant(),
      extraRuntimeDirectories: [await speechRuntimeDirectory()],
    );
  }

  /// The per-user folder that holds a repaired speech runtime, which works for
  /// a user who cannot write into the application's install directory.
  Future<Directory> speechRuntimeDirectory() async {
    final directory = HostPlatform.current().speechRuntimeDirectory;
    return Directory(directory);
  }

  /// Downloads, verifies and installs the pinned Whisper engine for this host.
  Future<Directory> installSpeechRuntime({
    void Function(int received, int total)? onProgress,
  }) async {
    final host = HostPlatform.current();
    final asset = speechRuntimeAssetFor(host, await hostArchitecture());
    if (asset == null) {
      throw const GithubReleaseException(
        'This build does not have a downloadable speech engine for this '
        'platform. Reinstall the application package to restore it.',
      );
    }
    final temporary = await Directory.systemTemp.createTemp('or-speech-');
    try {
      final archive = File(
        '${temporary.path}${Platform.pathSeparator}${asset.fileName}',
      );
      await _downloadAsset(
        url: asset.url,
        sizeBytes: asset.sizeBytes,
        sha256: asset.sha256,
        destination: archive,
        onProgress: onProgress,
      );
      await _extractArchive(archive, temporary, zip: asset.zip);
      final binary = await _findFile(
        temporary,
        'whisper-cli${host.executableSuffix}',
      );
      if (binary == null) {
        throw const GithubReleaseException(
          'The downloaded speech engine archive had no whisper command.',
        );
      }
      final target = await speechRuntimeDirectory();
      await target.create(recursive: true);
      await _copyDirectory(binary.parent, target);
      return target;
    } finally {
      try {
        await temporary.delete(recursive: true);
      } on FileSystemException {
        // A leftover temporary folder must not hide the real result.
      }
    }
  }

  /// Copies a folder the user picked (an extracted release, or a development
  /// build's `target/release`) into the per-user runtime folder.
  Future<Directory> addSpeechRuntimeFiles(Directory source) async {
    final target = await speechRuntimeDirectory();
    await target.create(recursive: true);
    await _copyDirectory(source, target);
    return target;
  }

  /// Unpacks a pinned archive with the system `tar`.
  ///
  /// Both Windows 10+ and the Linux desktop ship a `tar` that reads zip and
  /// gzip, so the app adds no archive dependency for one install path.
  Future<void> _extractArchive(
    File archive,
    Directory target, {
    required bool zip,
  }) async {
    final result = await Process.run('tar', [
      if (zip) '-xf' else '-xzf',
      archive.path,
      '-C',
      target.path,
    ]);
    if (result.exitCode != 0) {
      throw GithubReleaseException(
        'The speech engine archive could not be unpacked: '
        '${'${result.stderr}'.trim()}',
      );
    }
  }

  Future<File?> _findFile(Directory directory, String name) async {
    try {
      await for (final entry in directory.list(
        recursive: true,
        followLinks: false,
      )) {
        if (entry is File && entry.uri.pathSegments.last == name) {
          return entry;
        }
      }
    } on FileSystemException {
      return null;
    }
    return null;
  }

  Future<void> _copyDirectory(Directory source, Directory target) async {
    await target.create(recursive: true);
    await for (final entry in source.list(followLinks: false)) {
      final name = entry.uri.pathSegments.where((part) => part.isNotEmpty).last;
      final path = '${target.path}${Platform.pathSeparator}$name';
      if (entry is File) {
        await entry.copy(path);
      } else if (entry is Directory) {
        await _copyDirectory(entry, Directory(path));
      }
    }
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

  /// Lets the user point the assistant at a harness binary they installed
  /// somewhere this machine's `PATH` does not cover.
  Future<String?> chooseHarnessExecutable() async {
    final file = await picker.openFile(confirmButtonText: 'Use this harness');
    return file?.path;
  }

  /// Lets the user point the app at a folder of runtime files they already
  /// have, e.g. an extracted Whisper release or a local `target/release`.
  Future<String?> chooseRuntimeDirectory() async {
    return picker.getDirectoryPath(confirmButtonText: 'Use this folder');
  }

  /// Persists a chosen harness path, keeping every other assistant preference.
  Future<void> saveHarnessPath(String path) async {
    final store = AssistantSettingsStore();
    final settings = await store.load();
    await store.save(settings.copyWith(harnessPath: path));
  }

  Future<GithubRelease?> checkForUpdate(String currentVersion) =>
      fetchLatestGithubRelease(userAgent: 'Opencut-Reinforced/$currentVersion');

  Future<Directory> downloadDirectory() async {
    final downloads = await getDownloadsDirectory();
    if (downloads != null) return downloads;
    return getApplicationSupportDirectory();
  }

  /// The artifact this host installs for [release].
  Future<GithubReleaseAsset?> updateAsset(GithubRelease release) async {
    final host = HostPlatform.current();
    return release.assetFor(
      isWindows: host.isWindows,
      isMacOS: host.isMacOS,
      isLinux: host.isLinux,
      architecture: await hostArchitecture(),
    );
  }

  /// Installs a downloaded update, returning what the user still has to do.
  Future<UpdateInstallResult> installUpdate(File artifact) =>
      UpdateInstaller().install(artifact);

  /// Quits the app so the freshly installed build starts.
  void restartForUpdate() => UpdateInstaller().quit();

  /// Downloads and checksums this host's installer, returning the file.
  Future<File> downloadUpdate(
    GithubRelease release, {
    required Directory directory,
    void Function(int received, int total)? onProgress,
  }) async {
    final asset = await updateAsset(release);
    if (asset == null) {
      final host = HostPlatform.current();
      throw GithubReleaseException(
        'This release does not contain an installer for ${host.name}.',
      );
    }
    if (asset.sizeBytes <= 0) {
      throw const GithubReleaseException(
        'The release asset has no usable size.',
      );
    }
    final destination = File('${directory.path}/${asset.name}');
    await directory.create(recursive: true);
    return _downloadAsset(
      url: asset.url,
      sizeBytes: asset.sizeBytes,
      sha256: _sha256FromDigest(asset.digest),
      destination: destination,
      onProgress: onProgress,
    );
  }

  /// Streams one checksummed HTTPS download from a trusted GitHub host.
  ///
  /// The release DMG and the pinned speech runtime come through here, so a
  /// single implementation enforces the trusted host list, the advertised size
  /// and the published digest for everything the app downloads.
  Future<File> _downloadAsset({
    required String url,
    required int sizeBytes,
    required File destination,
    String? sha256,
    void Function(int received, int total)? onProgress,
  }) async {
    final partial = File('${destination.path}.part');
    await destination.parent.create(recursive: true);
    final client = HttpClient();
    var received = 0;
    IOSink? sink;
    try {
      var uri = Uri.parse(url);
      HttpClientResponse? response;
      for (var redirect = 0; redirect <= 6; redirect++) {
        if (uri.scheme != 'https' || !_isGithubHost(uri.host)) {
          throw const GithubReleaseException(
            'The download URL is not a trusted GitHub HTTPS address.',
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
              'The download redirected incorrectly.',
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
          'The download failed with HTTP ${response?.statusCode ?? 0}.',
        );
      }
      sink = partial.openWrite(mode: FileMode.write);
      await for (final chunk in response) {
        received += chunk.length;
        if (received > sizeBytes) {
          throw const GithubReleaseException(
            'The download exceeded its advertised size.',
          );
        }
        sink.add(chunk);
        onProgress?.call(received, sizeBytes);
      }
      await sink.flush();
      await sink.close();
      sink = null;
      if (received != sizeBytes) {
        throw const GithubReleaseException('The download was incomplete.');
      }
      if (sha256 != null) {
        final digest = await sha256Of(partial);
        if (digest != sha256) {
          throw const GithubReleaseException(
            'The download checksum does not match the published release.',
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
    try {
      await HostPlatform.current().openExternal(target);
    } on ProcessException {
      throw const GithubReleaseException(
        'The system could not open the downloaded item.',
      );
    }
  }
}

/// The CPU family this build runs on, in the spelling the pinned archives use.
///
/// Windows reports it in the environment; the POSIX hosts answer `uname -m`.
/// Unknown values fall back to x64, which is what the release assets default
/// to.
Future<String> hostArchitecture() async {
  final host = HostPlatform.current();
  if (host.isWindows) {
    final arch =
        host.environment['PROCESSOR_ARCHITECTURE'] ??
        host.environment['PROCESSOR_ARCHITEW6432'] ??
        '';
    return arch.toLowerCase().contains('arm') ? 'arm64' : 'x64';
  }
  try {
    final result = await Process.run('uname', ['-m']);
    if (result.exitCode == 0) {
      final machine = '${result.stdout}'.trim().toLowerCase();
      if (machine.contains('aarch64') || machine.contains('arm64')) {
        return 'arm64';
      }
      if (machine.isNotEmpty) return 'x64';
    }
  } on ProcessException {
    // Fall through to the default.
  }
  return 'x64';
}

/// The SHA-256 of a file as lowercase hex.
Future<String> sha256Of(File file) async {
  final digest = await sha256.bind(file.openRead()).first;
  return digest.toString().toLowerCase();
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
