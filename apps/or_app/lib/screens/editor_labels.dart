import 'package:flutter/foundation.dart';

import '../project/project_gateway.dart';

/// `0.5×`, `1.5×`, `2×`: a speed badge reads better as a decimal than as the
/// canonical `3/2` the model stores.
String rateLabel(ProjectRationalRate rate) {
  final value = rate.numerator / rate.denominator;
  final text = value == value.roundToDouble()
      ? value.round().toString()
      : value
            .toStringAsFixed(2)
            .replaceAll(RegExp(r'0+$'), '')
            .replaceAll(RegExp(r'\.$'), '');
  return text;
}

/// Minutes, seconds and milliseconds for a timeline clock readout.
String clockLabel(double seconds) {
  final milliseconds = (seconds * 1000).round();
  final minutes = milliseconds ~/ 60000;
  final wholeSeconds = milliseconds ~/ 1000 % 60;
  final fraction = milliseconds % 1000;
  return '${minutes.toString().padLeft(2, '0')}:${wholeSeconds.toString().padLeft(2, '0')}.${fraction.toString().padLeft(3, '0')}';
}

/// Platform-correct shortcut label for tooltips and menus. macOS uses the
/// command glyph; every other target keeps the explicit Ctrl prefix.
String shortcutLabel(String key) =>
    defaultTargetPlatform == TargetPlatform.macOS ? '⌘$key' : 'Ctrl+$key';

double rationalSeconds(ProjectRationalTime time) =>
    time.numerator.toInt() / time.denominator;

/// The Delete key hint follows the host keyboard, like every other shortcut.
String deleteShortcutLabel({bool shift = false}) {
  final mac = defaultTargetPlatform == TargetPlatform.macOS;
  final key = mac ? '⌫' : 'Del';
  return shift ? '⇧$key' : key;
}
