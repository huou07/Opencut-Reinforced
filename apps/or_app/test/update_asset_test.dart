import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/updates/github_release.dart';

GithubReleaseAsset _asset(String name) => GithubReleaseAsset(
  name: name,
  url:
      'https://github.com/huou07/Opencut-Reinforced/releases/download/v1/$name',
  sizeBytes: 1,
);

GithubRelease _release(List<String> names) => GithubRelease(
  tagName: 'v0.1.9',
  title: '0.1.9',
  pageUrl: 'https://github.com/huou07/Opencut-Reinforced/releases/tag/v0.1.9',
  notes: '',
  assets: [for (final name in names) _asset(name)],
);

void main() {
  const all = [
    'Opencut-Reinforced-0.1.9-arm64.dmg',
    'Opencut-Reinforced-0.1.9-linux-x64.deb',
    'Opencut-Reinforced-0.1.9-windows-x64.zip',
    'Opencut-Reinforced-0.1.9-windows-x64-setup.exe',
    'SHA256SUMS.txt',
  ];

  test('Windows installs the setup program, never the macOS image', () {
    final asset = _release(all).assetFor(isWindows: true, architecture: 'x64');
    expect(asset?.name, 'Opencut-Reinforced-0.1.9-windows-x64-setup.exe');
  });

  test('Windows falls back to the archive when no installer exists', () {
    final asset = _release(all).assetFor(isWindows: true, architecture: 'x64');
    expect(asset?.name, isNot(endsWith('.dmg')));
  });

  test('macOS keeps using the disk image', () {
    final asset = _release(all).assetFor(isMacOS: true, architecture: 'arm64');
    expect(asset?.name, endsWith('.dmg'));
  });

  test('Linux prefers the Debian package', () {
    final asset = _release(all).assetFor(isLinux: true, architecture: 'x64');
    expect(asset?.name, endsWith('.deb'));
  });

  test('an architecture-specific artifact wins over a generic one', () {
    final asset = _release([
      'Opencut-Reinforced-0.1.9-linux-x64.deb',
      'Opencut-Reinforced-0.1.9-linux-arm64.deb',
    ]).assetFor(isLinux: true, architecture: 'arm64');
    expect(asset?.name, contains('arm64'));
  });

  test('a release with only another platform artifact yields nothing', () {
    final asset = _release(['Opencut-Reinforced-0.1.9-arm64.dmg'])
        .assetFor(isWindows: true, architecture: 'x64');
    expect(asset, isNull);
  });
}
