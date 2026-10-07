import 'dart:convert';
import 'dart:io';

import 'package:integration_test/integration_test_driver_extended.dart';

Future<void> main() async {
  final directory = Directory(
    Platform.environment['OR_ANDROID_RECOVERY_ACCEPTANCE_OUTPUT'] ??
        'build/android-saf-recovery-acceptance',
  )..createSync(recursive: true);
  await integrationDriver(
    responseDataCallback: (data) async {
      if (data == null) return;
      final report = Map<String, dynamic>.from(data)..remove('screenshots');
      await File('${directory.path}/report.json')
          .writeAsString(const JsonEncoder.withIndent('  ').convert(report));
    },
    writeResponseOnFailure: true,
  );
}
