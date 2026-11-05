import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/export_capability.dart';
import 'package:or_app/project/export_profile.dart';
import 'package:or_app/updates/export_capability_panel.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester,
    ExportCapability capability,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: ExportCapabilityPanel(
              capability: ExportCapabilityCache(measured: capability),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('the panel lists each format the engine can write', (
    tester,
  ) async {
    await pumpPanel(
      tester,
      const ExportCapability(
        h264Encoder: 'h264_videotoolbox',
        h264Hardware: true,
        mp4Available: true,
        webmAvailable: true,
        losslessAvailable: true,
        preferredProfile: ExportProfile.mp4H264Aac,
      ),
    );

    expect(
      find.byKey(const ValueKey('settings-export-formats')),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('export-capability-mp4')), findsOneWidget);
    expect(
      find.byKey(const ValueKey('export-capability-webm')),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('export-capability-mkv')), findsOneWidget);
    expect(find.text('Ready'), findsNWidgets(3));
    expect(
      find.byKey(const ValueKey('export-capability-summary')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('export-capability-caveat')),
      findsNothing,
    );
  });

  testWidgets('an unavailable format is marked and explained', (tester) async {
    await pumpPanel(
      tester,
      const ExportCapability(
        webmAvailable: true,
        losslessAvailable: true,
        preferredProfile: ExportProfile.webmVp9Opus,
      ),
    );

    expect(find.text('Unavailable'), findsOneWidget);
    expect(find.text('Ready'), findsNWidgets(2));
    expect(
      find.byKey(const ValueKey('export-capability-caveat')),
      findsOneWidget,
    );
  });
}
