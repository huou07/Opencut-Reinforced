import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/updates/external_tools_panel.dart';

CompanionTool _tool({
  String id = 'blender',
  String state = 'missing',
  String? version,
  bool launchable = false,
  String? installCommand = 'sudo apt install blender',
}) {
  return CompanionTool(
    id: id,
    name: 'Blender',
    category: '3D',
    summary: 'Build a 3D scene and render frames for the timeline.',
    benefit: '3D motion graphics without OR growing a renderer.',
    license: 'GPL-3.0-or-later',
    licenseNote: 'Run as a separate program; OR does not link GPL code.',
    homepage: 'https://www.blender.org',
    installPage: 'https://www.blender.org/download/',
    installCommand: installCommand,
    installNote: 'Large download.',
    sizeHint: 'about 350 MB',
    runtime: 'Native executable',
    outputs: const ['image sequence', 'video'],
    projectScoped: false,
    removeHint: 'sudo apt remove blender',
    updateHint: 'sudo apt upgrade blender',
    state: state,
    version: version,
    detail: 'blender: command not found',
    launchable: launchable,
  );
}

Widget _host({
  required List<CompanionTool> tools,
  Future<List<CompanionTool>> Function()? loader,
  Future<CompanionTool> Function(String)? checker,
  Future<String> Function(String)? launcher,
  Future<void> Function(String)? opener,
}) {
  return MaterialApp(
    home: Scaffold(
      body: SingleChildScrollView(
        child: ExternalToolsPanel(
          loader: loader ?? () async => tools,
          checker: checker ?? (id) async => _tool(id: id),
          launcher: launcher ?? (id) async => 'opened',
          opener: opener ?? (url) async {},
        ),
      ),
    ),
  );
}

void main() {
  testWidgets('a missing tool says so and shows what it is for', (
    tester,
  ) async {
    await tester.pumpWidget(_host(tools: [_tool()]));
    await tester.pumpAndSettle();

    expect(find.text('Blender'), findsOneWidget);
    expect(find.text('Not installed'), findsOneWidget);
    expect(find.text('0 of 1 ready on this machine'), findsOneWidget);
    expect(find.textContaining('Build a 3D scene'), findsOneWidget);
  });

  testWidgets('a ready tool offers Open and shows its version', (tester) async {
    await tester.pumpWidget(
      _host(
        tools: [_tool(state: 'ready', version: '4.3.2', launchable: true)],
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Ready · 4.3.2'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('companion-tool-blender-open')),
      findsOneWidget,
    );
  });

  testWidgets('expanding shows licence, install command and removal steps', (
    tester,
  ) async {
    await tester.pumpWidget(_host(tools: [_tool()]));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('companion-tool-blender')));
    await tester.pumpAndSettle();

    expect(find.textContaining('GPL-3.0-or-later'), findsOneWidget);
    expect(find.text('sudo apt install blender'), findsOneWidget);
    expect(
      find.textContaining('Remove: sudo apt remove blender'),
      findsOneWidget,
    );
    expect(
      find.textContaining('Update: sudo apt upgrade blender'),
      findsOneWidget,
    );
  });

  testWidgets('Get it opens the official page instead of installing', (
    tester,
  ) async {
    final opened = <String>[];
    await tester.pumpWidget(
      _host(tools: [_tool()], opener: (url) async => opened.add(url)),
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const ValueKey('companion-tool-blender-install')),
    );
    await tester.pumpAndSettle();

    expect(opened, ['https://www.blender.org/download/']);
  });

  testWidgets('checking a tool replaces only that row', (tester) async {
    await tester.pumpWidget(
      _host(
        tools: [_tool()],
        checker: (id) async =>
            _tool(id: id, state: 'ready', version: '4.3.2', launchable: true),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const ValueKey('companion-tool-blender-check')),
    );
    await tester.pumpAndSettle();

    expect(find.text('Ready · 4.3.2'), findsOneWidget);
    expect(find.text('1 of 1 ready on this machine'), findsOneWidget);
  });

  testWidgets('opening a companion reports what happened', (tester) async {
    await tester.pumpWidget(
      _host(
        tools: [_tool(state: 'ready', version: '4.3.2', launchable: true)],
        launcher: (id) async => 'Blender opened (process 42).',
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('companion-tool-blender-open')));
    await tester.pumpAndSettle();

    expect(find.text('Blender opened (process 42).'), findsOneWidget);
  });

  testWidgets('a load failure is explained and retryable', (tester) async {
    var attempts = 0;
    await tester.pumpWidget(
      _host(
        tools: [_tool()],
        loader: () async {
          attempts++;
          if (attempts == 1) throw StateError('bridge unavailable');
          return [_tool(state: 'ready', version: '4.3.2')];
        },
      ),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('could not be read'), findsOneWidget);
    await tester.tap(find.text('Try again'));
    await tester.pumpAndSettle();

    expect(find.text('Ready · 4.3.2'), findsOneWidget);
  });
}
