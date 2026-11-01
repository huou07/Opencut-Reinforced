import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/updates/github_release.dart';
import 'package:or_app/updates/resource_center.dart';
import 'package:or_app/updates/resource_center_panel.dart';

class _FakeResourceService extends ResourceCenterService {
  _FakeResourceService({this.release});

  final GithubRelease? release;
  final RuntimeSnapshot snapshot = const RuntimeSnapshot(
    packaged: true,
    ffmpeg: true,
    speechRuntime: true,
    orCli: true,
    assistantPath: '/Applications/DeepSeek Harness.app',
    bundlePath: '/Applications/Opencut Reinforced.app',
  );
  final openedFiles = <File>[];
  final openedUrls = <String>[];
  int checks = 0;

  @override
  Future<RuntimeSnapshot> inspectRuntime() async => snapshot;

  @override
  Future<GithubRelease?> checkForUpdate(String currentVersion) async {
    checks++;
    return release;
  }

  @override
  Future<Directory> downloadDirectory() async => Directory.systemTemp;

  @override
  Future<File> downloadUpdate(
    GithubRelease release, {
    required Directory directory,
    void Function(int received, int total)? onProgress,
  }) async {
    onProgress?.call(10, 10);
    return File('${directory.path}/Opencut-Reinforced-test.dmg');
  }

  @override
  Future<void> openFile(File file) async => openedFiles.add(file);

  @override
  Future<void> openUrl(String url) async => openedUrls.add(url);
}

const _release = GithubRelease(
  tagName: 'v0.1.2',
  title: 'Opencut Reinforced 0.1.2',
  pageUrl: 'https://github.com/huou07/Opencut-Reinforced/releases/tag/v0.1.2',
  notes: 'A newer build.',
  assets: [
    GithubReleaseAsset(
      name: 'Opencut-Reinforced-0.1.2-arm64.dmg',
      url:
          'https://github.com/huou07/Opencut-Reinforced/releases/download/v0.1.2/Opencut-Reinforced-0.1.2-arm64.dmg',
      sizeBytes: 10,
      digest:
          'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    ),
  ],
);

void main() {
  test('versions compare numerically and ignore tags and build metadata', () {
    expect(compareVersions('v0.1.2', '0.1.1'), greaterThan(0));
    expect(compareVersions('0.1.1+2', '0.1.1'), 0);
    expect(compareVersions('0.1.10', '0.1.9'), greaterThan(0));
    expect(compareVersions('1.0', '0.9.9'), greaterThan(0));
    expect(compareVersions('0.1.1', '0.1.1'), 0);
    expect(isNewerRelease('v0.1.2', '0.1.1'), isTrue);
    expect(isNewerRelease('v0.1.1', '0.1.1+2'), isFalse);
  });

  test('a packaged executable resolves to its Contents directory', () async {
    final root = await Directory.systemTemp.createTemp('or-bundle-test-');
    addTearDown(() => root.delete(recursive: true));
    final macos = Directory('${root.path}/Opencut Reinforced.app/Contents/MacOS');
    await macos.create(recursive: true);
    final executable = File('${macos.path}/or_app');
    await executable.writeAsBytes(const [0]);

    final contents = bundleContentsFor(executable.path);
    expect(contents?.path, endsWith('/Contents'));
    expect(bundleContentsFor('/tmp/not-an-app/or_app'), isNull);
  });

  test('release JSON exposes the macOS asset and its checksum', () {
    final release = GithubRelease.fromJson({
      'tag_name': 'v0.1.2',
      'name': 'Opencut Reinforced 0.1.2',
      'html_url': 'https://github.com/huou07/Opencut-Reinforced/releases/tag/v0.1.2',
      'body': 'Notes',
      'published_at': '2026-11-01T12:00:00Z',
      'assets': [
        {
          'name': 'Opencut-Reinforced-0.1.2-arm64.dmg',
          'browser_download_url':
              'https://github.com/huou07/Opencut-Reinforced/releases/download/v0.1.2/Opencut-Reinforced-0.1.2-arm64.dmg',
          'size': 1234,
          'digest':
              'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        },
      ],
    });
    expect(release.macDmg?.name, 'Opencut-Reinforced-0.1.2-arm64.dmg');
    expect(release.macDmg?.sizeBytes, 1234);
    expect(release.publishedAt, isNotNull);
  });

  testWidgets('the panel checks for a newer build and downloads it', (
    tester,
  ) async {
    final service = _FakeResourceService(release: _release);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: ResourceCenterPanel(
              currentVersion: '0.1.1',
              service: service,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const ValueKey('resource-runtime-status')),
      findsOneWidget,
    );
    expect(find.text('FFmpeg runtime and codecs'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('resource-update-check')));
    await tester.pumpAndSettle();

    expect(service.checks, 1);
    expect(
      find.byKey(const ValueKey('resource-update-status')),
      findsOneWidget,
    );
    expect(find.textContaining('v0.1.2'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('resource-update-download')),
      findsOneWidget,
    );

    await tester.ensureVisible(
      find.byKey(const ValueKey('resource-update-download')),
    );
    await tester.tap(find.byKey(const ValueKey('resource-update-download')));
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('resource-update-open')), findsOneWidget);
    expect(
      find.byKey(const ValueKey('resource-update-downloaded')),
      findsOneWidget,
    );
  });

  testWidgets('the panel says an installation is current', (tester) async {
    final service = _FakeResourceService(release: _release);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: ResourceCenterPanel(
              currentVersion: '0.1.2',
              service: service,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('resource-update-check')));
    await tester.pumpAndSettle();

    expect(find.textContaining('up to date'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('resource-update-download')),
      findsOneWidget,
    );
  });
}
