import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:or_app/main.dart';
import 'package:or_app/rust_core_gateway.dart';
import 'package:or_app/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(RustLib.init);

  testWidgets('native bridge diagnostics match the CLI snapshot', (
    tester,
  ) async {
    const expectedFile = String.fromEnvironment('OR_CLI_BOOTSTRAP_FILE');
    expect(expectedFile, isNotEmpty);
    final expected = jsonDecode(
      await File(expectedFile).readAsString(),
    ) as Map<String, dynamic>;
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
    await tester.tap(find.text('Settings').first);
    await tester.pumpAndSettle();
    expect(find.text(appInfo.name), findsWidgets);
    expect(find.text(appInfo.version), findsOneWidget);
    expect(find.text(health.status), findsOneWidget);
    for (final capability in capabilities) {
      expect(
        find.text('${capability.id} · v${capability.version}'),
        findsOneWidget,
      );
    }
  });
}
