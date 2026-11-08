import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:or_app/project/recent_projects.dart';
import 'package:or_app/screens/recent_projects_list.dart';

RecentProject _project(String path) => RecentProject(
  path: path,
  name: 'Repro',
  lastOpened: DateTime.utc(2026, 11, 8),
);

Widget _list(List<RecentProject> projects) => MaterialApp(
  home: Scaffold(
    body: RecentProjectsList(
      projects: projects,
      loaded: true,
      unavailable: false,
      onOpen: (_) {},
    ),
  ),
);

void main() {
  testWidgets('two projects with one name stay distinguishable by folder', (
    tester,
  ) async {
    final home = Platform.environment['HOME'] ?? '/home/tester';
    await tester.pumpWidget(
      _list([
        _project('$home/Movies/repro.orproj'),
        _project('/tmp/or-repro/repro.orproj'),
      ]),
    );

    expect(find.text('Repro'), findsNWidgets(2));
    final subtitles = tester
        .widgetList<Text>(find.byType(Text))
        .map((text) => text.data ?? '')
        .toList();
    expect(subtitles.any((line) => line.startsWith('~/Movies · ')), isTrue);
    expect(
      subtitles.any((line) => line.startsWith('/tmp/or-repro · ')),
      isTrue,
    );
  });
}
