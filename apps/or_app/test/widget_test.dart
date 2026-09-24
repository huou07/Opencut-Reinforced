import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/core_gateway.dart';
import 'package:or_app/main.dart';

void main() {
  testWidgets('wide navigation opens the Phase 3 areas', (tester) async {
    await tester.pumpWidget(OrApp(gateway: _FakeCoreGateway()));

    expect(find.text('Pre-MVP architecture foundation'), findsOneWidget);
    await tester.tap(find.text('Projects').first);
    await tester.pumpAndSettle();
    expect(find.text('Not implemented in Phase 3.'), findsOneWidget);

    await tester.tap(find.text('Settings').first);
    await tester.pumpAndSettle();
    expect(find.text('About / Developer Diagnostics'), findsOneWidget);
    expect(find.text('Opencut Reinforced'), findsWidgets);
    expect(find.text('core.app_info · v1'), findsOneWidget);
  });

  testWidgets('compact navigation uses the bottom bar', (tester) async {
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(OrApp(gateway: _FakeCoreGateway()));

    expect(find.byType(NavigationBar), findsOneWidget);
    await tester.tap(find.byIcon(Icons.video_library_outlined));
    await tester.pumpAndSettle();
    expect(find.text('Not implemented in Phase 3.'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

class _FakeCoreGateway implements CoreGateway {
  @override
  Future<AppInfo> appInfo() async => const AppInfo(
    name: 'Opencut Reinforced',
    version: '0.1.0',
    coreApiVersion: 1,
  );

  @override
  Future<HealthStatus> health() async => const HealthStatus(status: 'ok');

  @override
  Future<List<Capability>> capabilities() async => const [
    Capability(id: 'core.app_info', version: 1),
    Capability(id: 'core.health', version: 1),
    Capability(id: 'core.capabilities', version: 1),
  ];
}
