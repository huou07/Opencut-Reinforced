import 'dart:convert';

import 'package:flutter/foundation.dart' show ValueKey;
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:or_app/main.dart';
import 'package:or_app/rust_core_gateway.dart';
import 'package:or_app_bridge/or_app_bridge.dart' show RustLib;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(RustLib.init);

  testWidgets('native bridge diagnostics match the CLI snapshot', (
    tester,
  ) async {
    const expectedJson = String.fromEnvironment('OR_CLI_BOOTSTRAP_JSON');
    expect(expectedJson, isNotEmpty);
    final expected = jsonDecode(expectedJson) as Map<String, dynamic>;
    final gateway = RustCoreGateway();

    final appInfo = await gateway.appInfo();
    expect({
      'name': appInfo.name,
      'version': appInfo.version,
      'core_api_version': appInfo.coreApiVersion,
    }, expected['app_info']);

    final health = await gateway.health();
    expect({'status': health.status}, expected['health']);

    final capabilities = await gateway.capabilities();
    expect(
      capabilities
          .map(
            (capability) => {
              'id': capability.id,
              'version': capability.version,
            },
          )
          .toList(),
      (expected['capabilities'] as Map<String, dynamic>)['capabilities'],
    );

    await tester.pumpWidget(const OrApp(gateway: RustCoreGateway()));
    await tester.tap(find.byKey(const ValueKey('nav-settings')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('settings-section-advanced')));
    await tester.pumpAndSettle();
    expect(find.text(appInfo.name), findsWidgets);
    expect(find.text(appInfo.version), findsOneWidget);
    expect(find.text(health.status), findsOneWidget);
    for (final capability in capabilities) {
      expect(find.text(capability.id), findsOneWidget);
    }
    final versions = <String, int>{};
    for (final capability in capabilities) {
      final label = 'v${capability.version}';
      versions.update(label, (count) => count + 1, ifAbsent: () => 1);
    }
    for (final entry in versions.entries) {
      expect(find.text(entry.key), findsNWidgets(entry.value));
    }
  });
}
