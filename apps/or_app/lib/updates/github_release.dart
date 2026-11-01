import 'dart:async';
import 'dart:convert';
import 'dart:io';

/// A downloadable file attached to a GitHub release.
class GithubReleaseAsset {
  const GithubReleaseAsset({
    required this.name,
    required this.url,
    required this.sizeBytes,
    this.digest,
  });

  final String name;
  final String url;
  final int sizeBytes;

  /// The API's `sha256:<hex>` digest when GitHub supplies one.
  final String? digest;

  factory GithubReleaseAsset.fromJson(Map<String, Object?> json) {
    final name = json['name'];
    final url = json['browser_download_url'];
    final size = json['size'];
    if (name is! String || url is! String) {
      throw const FormatException(
        'A release asset is missing its name or URL.',
      );
    }
    return GithubReleaseAsset(
      name: name,
      url: url,
      sizeBytes: size is int ? size : 0,
      digest: json['digest'] is String ? json['digest'] as String : null,
    );
  }
}

/// The latest published release of the OR repository.
class GithubRelease {
  const GithubRelease({
    required this.tagName,
    required this.title,
    required this.pageUrl,
    required this.notes,
    this.publishedAt,
    required this.assets,
  });

  final String tagName;
  final String title;
  final String pageUrl;
  final String notes;
  final DateTime? publishedAt;
  final List<GithubReleaseAsset> assets;

  /// The Apple Silicon disk image, when the release carries one.
  GithubReleaseAsset? get macDmg {
    for (final asset in assets) {
      if (asset.name.toLowerCase().endsWith('.dmg')) return asset;
    }
    return null;
  }

  factory GithubRelease.fromJson(Map<String, Object?> json) {
    final tag = json['tag_name'];
    final page = json['html_url'];
    if (tag is! String || page is! String) {
      throw const FormatException(
        'The release is missing its tag or page URL.',
      );
    }
    final assets = json['assets'];
    return GithubRelease(
      tagName: tag,
      title: json['name'] is String ? json['name'] as String : tag,
      pageUrl: page,
      notes: json['body'] is String ? json['body'] as String : '',
      publishedAt: DateTime.tryParse(
        json['published_at'] is String ? json['published_at'] as String : '',
      ),
      assets: assets is List
          ? [
              for (final asset in assets)
                if (asset is Map)
                  GithubReleaseAsset.fromJson(
                    asset.map((key, value) => MapEntry('$key', value)),
                  ),
            ]
          : const [],
    );
  }
}

class GithubReleaseException implements Exception {
  const GithubReleaseException(this.message);

  final String message;

  @override
  String toString() => message;
}

/// Reads the newest public, non-prerelease release without requiring a token.
///
/// GitHub allows 60 anonymous API requests per hour per address; the updater
/// checks only when the user asks, so that budget is ample.
Future<GithubRelease?> fetchLatestGithubRelease({
  String owner = 'huou07',
  String repository = 'Opencut-Reinforced',
  HttpClient? client,
  String? userAgent,
}) async {
  final uri = Uri.https(
    'api.github.com',
    '/repos/$owner/$repository/releases/latest',
  );
  final ownsClient = client == null;
  final http = client ?? HttpClient();
  try {
    final request = await http.getUrl(uri).timeout(const Duration(seconds: 15));
    request.headers.set(
      HttpHeaders.acceptHeader,
      'application/vnd.github+json',
    );
    request.headers.set(
      HttpHeaders.userAgentHeader,
      userAgent ?? 'Opencut-Reinforced',
    );
    final response = await request.close().timeout(const Duration(seconds: 15));
    final body = await response.transform(utf8.decoder).join();
    if (response.statusCode == HttpStatus.notFound) {
      throw const GithubReleaseException(
        'This repository does not have a published release yet.',
      );
    }
    if (response.statusCode == HttpStatus.forbidden ||
        response.statusCode == HttpStatus.tooManyRequests) {
      throw const GithubReleaseException(
        'GitHub is rate-limiting release checks. Try again later.',
      );
    }
    if (response.statusCode != HttpStatus.ok) {
      throw GithubReleaseException(
        'GitHub returned HTTP ${response.statusCode} while checking for updates.',
      );
    }
    final decoded = jsonDecode(body);
    if (decoded is! Map) {
      throw const GithubReleaseException(
        'GitHub returned an unexpected release response.',
      );
    }
    final json = decoded.map((key, value) => MapEntry('$key', value));
    if (json['draft'] == true || json['prerelease'] == true) return null;
    return GithubRelease.fromJson(json);
  } on GithubReleaseException {
    rethrow;
  } on SocketException {
    throw const GithubReleaseException(
      'The update server could not be reached. Check the internet connection.',
    );
  } on TimeoutException {
    throw const GithubReleaseException(
      'The update check timed out. Try again.',
    );
  } finally {
    if (ownsClient) http.close(force: true);
  }
}

/// Compares dotted application versions, ignoring a leading `v` and build
/// metadata after `+`. Nested prerelease tags are ordered below a release.
int compareVersions(String left, String right) {
  final a = _versionParts(left);
  final b = _versionParts(right);
  for (var index = 0; index < 3; index++) {
    final compared = a[index].compareTo(b[index]);
    if (compared != 0) return compared;
  }
  return 0;
}

List<int> _versionParts(String version) {
  final core = version
      .trim()
      .replaceFirst(RegExp(r'^[vV]'), '')
      .split('+')
      .first
      .split('-')
      .first;
  final parts = core.split('.');
  return [
    for (var index = 0; index < 3; index++)
      index < parts.length ? (int.tryParse(parts[index]) ?? 0) : 0,
  ];
}

bool isNewerRelease(String tagName, String currentVersion) =>
    compareVersions(tagName, currentVersion) > 0;
