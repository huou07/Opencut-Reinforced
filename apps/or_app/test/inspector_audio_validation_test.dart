import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/screens/inspector_audio_validation.dart';

void main() {
  test('valid texts become a command payload', () {
    final settings = audioSettingsFromTexts(
      gain: '-6.5',
      pan: '25',
      fadeIn: '0.5',
      fadeOut: '1',
    );
    expect(settings, isNotNull);
    expect(settings!.gainMilliDecibels, -6500);
    expect(settings.panBasisPoints, 2500);
    // ProjectRationalTime has no value equality; compare the ratvalue.
    expect(
      settings.fadeIn.numerator.toInt() / settings.fadeIn.denominator,
      closeTo(0.5, 1e-9),
    );
    expect(
      settings.fadeOut.numerator.toInt() / settings.fadeOut.denominator,
      closeTo(1.0, 1e-9),
    );
  });

  test('a gain above 24 dB is refused with the spoken message', () {
    final settings = audioSettingsFromTexts(
      gain: '100',
      pan: '0',
      fadeIn: '0',
      fadeOut: '0',
    );
    expect(settings, isNull);
    expect(inspectorAudioValidationMessage, contains('gain between -96 and 24 dB'));
  });

  test('a pan outside -100..100 is refused', () {
    expect(
      audioSettingsFromTexts(
        gain: '0',
        pan: '200',
        fadeIn: '0',
        fadeOut: '0',
      ),
      isNull,
    );
  });

  test('a fade that is not a time is refused', () {
    expect(
      audioSettingsFromTexts(
        gain: '0',
        pan: '0',
        fadeIn: 'soon',
        fadeOut: '0',
      ),
      isNull,
    );
  });

  test('exact rationals are accepted for fades', () {
    final settings = audioSettingsFromTexts(
      gain: '0',
      pan: '0',
      fadeIn: '1/4',
      fadeOut: '1/2',
    );
    expect(settings, isNotNull);
    expect(settings!.fadeIn.numerator, BigInt.from(1));
    expect(settings.fadeIn.denominator, 4);
  });
}
