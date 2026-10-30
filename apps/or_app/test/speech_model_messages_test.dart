import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/shell/speech_model_messages.dart';

void main() {
  test('a refused source names the HTTPS rule', () {
    expect(
      speechModelFailureMessage(
        const FormatException('Model source must remain HTTPS.'),
      ),
      'The model source must stay on HTTPS.',
    );
  });

  test('an HTTP failure keeps its reason', () {
    expect(
      speechModelFailureMessage(const HttpException('incomplete')),
      'The model download failed: incomplete',
    );
  });

  test('an unreachable network is named as such', () {
    expect(
      speechModelFailureMessage(
        const SocketException('Failed host lookup: huggingface.co'),
      ),
      'The model download could not reach the network.',
    );
  });

  test('an unknown failure keeps the generic sentence', () {
    expect(
      speechModelFailureMessage(StateError('boom')),
      'The speech model could not be changed.',
    );
  });
}
