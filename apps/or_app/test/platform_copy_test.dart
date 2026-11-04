import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// Copy that names one desktop platform is a defect on the other two.
///
/// This guard scans the shipped sources rather than a widget, because the copy
/// lives in settings, dialogs and panels that a single pump would not reach.
void main() {
  final sources = Directory('lib')
      .listSync(recursive: true)
      .whereType<File>()
      .where((file) => file.path.endsWith('.dart'))
      .toList();

  test('the sources were found', () {
    expect(sources, isNotEmpty);
  });

  test('copy does not point at one desktop platform', () {
    final forbidden = RegExp(
      r'\bthis Mac\b|\bon this mac\b',
      caseSensitive: false,
    );
    final offenders = <String>[
      for (final file in sources)
        if (forbidden.hasMatch(file.readAsStringSync())) file.path,
    ];
    expect(
      offenders,
      isEmpty,
      reason:
          'Name the device or installation instead of one operating system: '
          '$offenders',
    );
  });

  test('the assistant wait state does not hardcode one harness', () {
    final offenders = <String>[
      for (final file in sources)
        if (file.readAsStringSync().contains('Starting DeepSeek Harness'))
          file.path,
    ];
    expect(
      offenders,
      isEmpty,
      reason:
          'The wait state must name the harness that is starting: $offenders',
    );
  });
}
