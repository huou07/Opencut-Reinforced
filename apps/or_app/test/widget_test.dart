import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/main.dart';

void main() {
  testWidgets('wide navigation opens the Phase 3 areas', (tester) async {
    await tester.pumpWidget(const OrApp());

    expect(find.text('Pre-MVP architecture foundation'), findsOneWidget);
    await tester.tap(find.text('Projects').first);
    await tester.pumpAndSettle();
    expect(find.text('Not implemented in Phase 3.'), findsOneWidget);

    await tester.tap(find.text('Settings').first);
    await tester.pumpAndSettle();
    expect(find.text('About / Developer Diagnostics'), findsOneWidget);
    expect(
      find.text('Core diagnostics will be connected in Phase 3.'),
      findsOneWidget,
    );
  });

  testWidgets('compact navigation uses the bottom bar', (tester) async {
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(const OrApp());

    expect(find.byType(NavigationBar), findsOneWidget);
    await tester.tap(find.byIcon(Icons.video_library_outlined));
    await tester.pumpAndSettle();
    expect(find.text('Not implemented in Phase 3.'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
