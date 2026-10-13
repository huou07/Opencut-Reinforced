import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/recent_projects.dart';

void main() {
  late Directory supportDirectory;
  var clockTick = 0;

  setUp(() {
    supportDirectory = Directory.systemTemp.createTempSync('or-recents-');
    clockTick = 0;
  });

  tearDown(() async {
    if (await supportDirectory.exists()) {
      await supportDirectory.delete(recursive: true);
    }
  });

  PersistentRecentProjectStore store() => PersistentRecentProjectStore(
    supportDirectory: () async => supportDirectory,
    now: () => DateTime.utc(2026, 10, 13, 12, 0, clockTick++),
  );

  test(
    'records persist, sort by last open, and update duplicate locations',
    () async {
      final recent = store();
      expect(await recent.load(), isEmpty);

      await recent.record(path: '/projects/first.orproj', name: 'First');
      final openedAt = (await recent.load()).first.lastOpened;
      await recent.record(path: '/projects/second.orproj', name: 'Second');
      final updated = await recent.updateName(
        path: '/projects/first.orproj',
        name: 'Renamed First',
      );

      expect(updated.map((project) => project.name), [
        'Second',
        'Renamed First',
      ]);
      expect((await store().load()).map((project) => project.path), [
        '/projects/second.orproj',
        '/projects/first.orproj',
      ]);
      expect(updated.last.lastOpened, openedAt);
    },
  );

  test(
    'Android recent identity retains its SAF URI and deduplicates by URI',
    () async {
      final recent = store();
      const uri = 'content://com.example.documents/document/project%3A1';
      await recent.record(
        path: '/data/user/0/or/files/or-projects/first.orproj',
        documentUri: uri,
        name: 'First',
      );
      final openedAt = (await recent.load()).single.lastOpened;
      final updated = await recent.updateName(
        path: '/data/user/0/or/files/or-projects/first.orproj',
        documentUri: uri,
        name: 'Renamed First',
      );

      expect(updated, hasLength(1));
      expect(updated.single.documentUri, uri);
      expect(updated.single.name, 'Renamed First');
      expect(updated.single.lastOpened, openedAt);
    },
  );

  test('history is bounded to the newest twelve projects', () async {
    final recent = store();
    for (var index = 0; index < 14; index++) {
      await recent.record(
        path: '/projects/project-$index.orproj',
        name: 'Project $index',
      );
    }

    final projects = await store().load();
    expect(projects, hasLength(maxRecentProjects));
    expect(projects.first.name, 'Project 13');
    expect(projects.last.name, 'Project 2');
  });

  test('invalid recent metadata never becomes an openable project', () async {
    final file = File('${supportDirectory.path}/recent-projects.v1.json');
    await file.writeAsString(
      jsonEncode({
        'version': 1,
        'projects': [
          {
            'path': '/projects/unsafe.orproj',
            'name': 'Unsafe',
            'lastOpened': '2026-10-13T12:00:00Z',
            'documentUri': 'file:///outside/orproj',
          },
          {
            'path': '/projects/safe.orproj',
            'name': 'Safe',
            'lastOpened': '2026-10-13T11:00:00Z',
          },
        ],
      }),
    );

    final projects = await store().load();
    expect(projects, hasLength(1));
    expect(projects.single.path, '/projects/safe.orproj');
  });

  test(
    'corrupt history is ignored and rebuilt after a successful open',
    () async {
      final file = File('${supportDirectory.path}/recent-projects.v1.json');
      await file.writeAsString('{ broken');
      final recent = store();
      expect(await recent.load(), isEmpty);

      await recent.record(
        path: '/projects/recovered.orproj',
        name: 'Recovered',
      );
      expect((await recent.load()).single.name, 'Recovered');
    },
  );
}
