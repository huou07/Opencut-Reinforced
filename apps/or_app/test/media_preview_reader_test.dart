import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/production/media_preview_reader.dart';

const _kind = ProjectMediaArtifactKind.thumbnail;

ProjectMediaPreview _stored({String cacheKey = 'cache-1'}) =>
    ProjectMediaPreview(
      kind: _kind,
      state: ProjectMediaArtifactRequestState.running,
      cacheKey: cacheKey,
    );

void main() {
  late Map<String, ProjectMediaPreview> previews;
  late bool currentSession;

  MediaPreviewReader build(
    Future<ProjectMediaArtifact?> Function(
      Object,
      ProjectMediaArtifactKind,
      String,
    )
    read,
  ) => MediaPreviewReader(
    isCurrentSession: (_) => currentSession,
    readArtifact: read,
    current: (mediaId) => previews[mediaId],
  );

  setUp(() {
    previews = {'media-1': _stored()};
    currentSession = true;
  });

  Future<ProjectMediaPreview?> read(
    MediaPreviewReader reader, {
    String mediaId = 'media-1',
    String cacheKey = 'cache-1',
  }) =>
      reader.read(Object(), mediaId: mediaId, kind: _kind, cacheKey: cacheKey);

  test('a PNG artifact becomes a ready preview with bytes', () async {
    final reader = build(
      (_, _, _) async =>
          ProjectMediaArtifact(mimeType: 'image/png', bytes: _bytes),
    );
    final preview = await read(reader);
    expect(preview?.state, ProjectMediaArtifactRequestState.ready);
    expect(preview?.bytes, _bytes);
    expect(preview?.errorCode, isNull);
  });

  test('a non-PNG artifact is a failure without an error code', () async {
    final reader = build(
      (_, _, _) async =>
          ProjectMediaArtifact(mimeType: 'audio/wav', bytes: _bytes),
    );
    final preview = await read(reader);
    expect(preview?.state, ProjectMediaArtifactRequestState.failed);
    expect(preview?.bytes, isNull);
    expect(preview?.errorCode, isNull);
  });

  test('a missing artifact reports ARTIFACT_NOT_FOUND', () async {
    final reader = build((_, _, _) async => null);
    final preview = await read(reader);
    expect(preview?.state, ProjectMediaArtifactRequestState.failed);
    expect(preview?.errorCode, 'ARTIFACT_NOT_FOUND');
  });

  test('a throwing read reports ARTIFACT_READ_FAILED', () async {
    final reader = build((_, _, _) async => throw StateError('gone'));
    final preview = await read(reader);
    expect(preview?.errorCode, 'ARTIFACT_READ_FAILED');
  });

  test('a replaced cache key or session yields nothing', () async {
    var reads = 0;
    final reader = build((_, _, _) async {
      reads += 1;
      return ProjectMediaArtifact(mimeType: 'image/png', bytes: _bytes);
    });
    expect(await read(reader, cacheKey: 'cache-2'), isNull);
    expect(reads, 0);
    previews = {'media-1': _stored(cacheKey: 'cache-2')};
    expect(await read(reader), isNull);
    currentSession = false;
    previews = {'media-1': _stored()};
    expect(await read(reader), isNull);
  });
}

final _bytes = Uint8List.fromList(<int>[137, 80, 78, 71]);
