import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/design/or_theme.dart';
import 'package:or_app/library/project_templates.dart';
import 'package:or_app/library/template_previews.dart';
import 'package:or_app/screens/library_screen.dart';

/// A one pixel PNG, so the test proves a still reached Image.memory without
/// needing a renderer.
final Uint8List _pixel = Uint8List.fromList(<int>[
  0x89,
  0x50,
  0x4E,
  0x47,
  0x0D,
  0x0A,
  0x1A,
  0x0A,
  0x00,
  0x00,
  0x00,
  0x0D,
  0x49,
  0x48,
  0x44,
  0x52,
  0x00,
  0x00,
  0x00,
  0x01,
  0x00,
  0x00,
  0x00,
  0x01,
  0x08,
  0x06,
  0x00,
  0x00,
  0x00,
  0x1F,
  0x15,
  0xC4,
  0x89,
  0x00,
  0x00,
  0x00,
  0x0D,
  0x49,
  0x44,
  0x41,
  0x54,
  0x78,
  0x9C,
  0x63,
  0x00,
  0x01,
  0x00,
  0x00,
  0x05,
  0x00,
  0x01,
  0x0D,
  0x0A,
  0x2D,
  0xB4,
  0x00,
  0x00,
  0x00,
  0x00,
  0x49,
  0x45,
  0x4E,
  0x44,
  0xAE,
  0x42,
  0x60,
  0x82,
]);

class _FakePreviews implements TemplatePreviews {
  _FakePreviews(this.png);

  final Uint8List? png;
  int calls = 0;

  @override
  Future<Uint8List?> still(ProjectTemplate template) async {
    calls++;
    return png;
  }
}

Future<void> _pump(WidgetTester tester, TemplatePreviews previews) async {
  tester.view.physicalSize = const Size(1600, 1400);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(
    MaterialApp(
      theme: OrTheme.dark,
      home: Scaffold(
        body: LibraryScreen(
          canPickProjects: true,
          projectOpen: false,
          onUseTemplate: (_) {},
          onAddTextPreset: (_) {},
          onAddSoundEffect: (_) {},
          onOpenEditor: () {},
          templatePreviews: previews,
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

bool _isStill(Widget widget) =>
    widget is Image && widget.semanticLabel == 'Title card opening frame';

void main() {
  testWidgets('a template card shows the still its scene renders', (
    tester,
  ) async {
    final previews = _FakePreviews(_pixel);
    await _pump(tester, previews);
    expect(find.byWidgetPredicate(_isStill), findsOneWidget);
    expect(previews.calls, projectTemplates.length);
    // The image is a decoration: the card still names the template and keeps
    // its use button tappable.
    expect(find.text('Title card'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('library-template-use-title-card')),
      findsOneWidget,
    );
  });

  testWidgets('a failed still leaves the text-only card', (tester) async {
    final previews = _FakePreviews(null);
    await _pump(tester, previews);
    expect(find.byWidgetPredicate(_isStill), findsNothing);
    expect(find.text('Title card'), findsOneWidget);
  });
}
