import 'dart:convert';
import 'dart:io';

import 'package:integration_test/integration_test_driver_extended.dart';

Future<void> main() async {
  final directory = Directory(
    Platform.environment['OR_ANDROID_RECOVERY_ACCEPTANCE_OUTPUT'] ??
        'build/android-saf-recovery-acceptance',
  )..createSync(recursive: true);
  await integrationDriver(
    onScreenshot: (name, bytes, [args]) async {
      await File('${directory.path}/$name.png').writeAsBytes(bytes);
      return true;
    },
    responseDataCallback: (data) async {
      if (data == null) return;
      final screenshots = data['screenshots'] as List? ?? const [];
      for (final rawScreenshot in screenshots) {
        final screenshot = Map<String, dynamic>.from(rawScreenshot as Map);
        final name = screenshot['screenshotName'] as String;
        final bytes = (screenshot['bytes'] as List).cast<int>();
        await File('${directory.path}/$name.png').writeAsBytes(bytes);
      }
      final report = Map<String, dynamic>.from(data)..remove('screenshots');
      await File('${directory.path}/report.json')
          .writeAsString(const JsonEncoder.withIndent('  ').convert(report));
    },
    writeResponseOnFailure: true,
  );
}
