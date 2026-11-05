import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/design/or_theme.dart';
import 'package:or_app/screens/home_screen.dart';

void main() {
  Future<void> pumpWelcome(
    WidgetTester tester, {
    String? machineSummary,
    String? exportSummary,
    String? machineWarning,
  }) async {
    await tester.binding.setSurfaceSize(const Size(1440, 900));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      MaterialApp(
        theme: OrTheme.dark,
        home: Scaffold(
          body: SingleChildScrollView(
            child: HomeScreen(
              canPickProjects: true,
              onNewProject: () {},
              onOpenProject: () {},
              onOpenProjects: () {},
              recentProjects: const [],
              recentProjectsLoaded: true,
              recentProjectsUnavailable: false,
              onOpenRecentProject: (_) {},
              onOpenResources: () {},
              onOpenLibrary: () {},
              onOpenGuide: () {},
              showWelcome: true,
              onDismissWelcome: () {},
              machineSummary: machineSummary,
              exportSummary: exportSummary,
              machineWarning: machineWarning,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('the welcome panel states the measured machine and formats', (
    tester,
  ) async {
    await pumpWelcome(
      tester,
      machineSummary: 'macOS Version 15.0 · 10 cores · 16 GB RAM · 120 GB free',
      exportSummary:
          'This build exports MP4 (VideoToolbox, hardware), WebM (VP9), '
          'Lossless (FFV1).',
    );

    expect(find.byKey(const ValueKey('home-welcome')), findsOneWidget);
    expect(
      tester
          .widget<Text>(find.byKey(const ValueKey('home-welcome-machine')))
          .data,
      'macOS Version 15.0 · 10 cores · 16 GB RAM · 120 GB free',
    );
    expect(
      tester
          .widget<Text>(find.byKey(const ValueKey('home-welcome-export')))
          .data,
      contains('MP4 (VideoToolbox, hardware)'),
    );
    expect(find.byKey(const ValueKey('home-welcome-warning')), findsNothing);
  });

  testWidgets('a short disk is warned about before the first project', (
    tester,
  ) async {
    await pumpWelcome(
      tester,
      machineSummary: 'Linux · 4 cores · 512 MB free',
      machineWarning:
          'Less than 4 GB free where exports are staged. Free some space '
          'before a long export.',
    );

    expect(find.byKey(const ValueKey('home-welcome-warning')), findsOneWidget);
  });

  testWidgets('without a measurement the panel stays the same as before', (
    tester,
  ) async {
    await pumpWelcome(tester);

    expect(find.byKey(const ValueKey('home-welcome')), findsOneWidget);
    expect(find.byKey(const ValueKey('home-welcome-machine')), findsNothing);
    expect(find.byKey(const ValueKey('home-welcome-export')), findsNothing);
    expect(find.byKey(const ValueKey('home-welcome-new')), findsOneWidget);
  });
}
