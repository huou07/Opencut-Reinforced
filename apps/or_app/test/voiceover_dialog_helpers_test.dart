import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/production/voiceover_dialog.dart';

void main() {
  test('a chosen voice survives only while its list still has it', () {
    expect(presentVoiceId(['a', 'b'], 'b'), 'b');
    expect(presentVoiceId(['a', 'b'], 'c'), isNull);
    expect(presentVoiceId(const <String>[], 'cloned'), isNull);
    expect(presentVoiceId(const <String>[], ''), isNull);
  });
}
