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
  testWidgets('two projects with one name stay distinguishable by path', (
    tester,
  ) async {
    // The list shortens a path with the home directory it knows about, which is
    // HOME on macOS and Linux and USERPROFILE on Windows. The second project
    // has to live outside that directory, so the platform that has no /tmp uses
    // a drive-rooted path instead.
    final home =
        Platform.environment['HOME'] ??
        Platform.environment['USERPROFILE'] ??
        '/home/tester';
    final elsewhere = Platform.isWindows
        ? r'C:\or-repro\repro.orproj'
        : '/tmp/or-repro/repro.orproj';
    await tester.pumpWidget(
      _list([_project('$home/Movies/repro.orproj'), _project(elsewhere)]),
    );

    expect(find.text('Repro'), findsNWidgets(2));
    final subtitles = tester
        .widgetList<Text>(find.byType(Text))
        .map((text) => text.data ?? '')
        .toList();
    expect(
      subtitles.any((line) => line.startsWith('~/Movies/repro.orproj · ')),
      isTrue,
    );
    expect(subtitles.any((line) => line.startsWith('$elsewhere · ')), isTrue);
  });
}
