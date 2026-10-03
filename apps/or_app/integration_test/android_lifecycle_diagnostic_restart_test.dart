import 'package:flutter/foundation.dart';

import 'android_lifecycle_diagnostic_test.dart' as assertions;

// Different compiled target forces APK replacement; all assertions are identical.
void main() {
  debugPrint('DIAGNOSTIC_SECOND_COMPILED_TARGET');
  assertions.main();
}
