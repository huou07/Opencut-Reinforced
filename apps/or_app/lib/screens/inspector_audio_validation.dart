import '../project/project_gateway.dart';

import 'editor_rational_time.dart';

/// The one sentence the inspector speaks when an audio field cannot be read.
///
/// Kept beside the reader so the copy and the rule cannot drift: the message
/// names every field it validates, and tests assert both the null result and
/// this text.
const String inspectorAudioValidationMessage =
    'Enter a gain between -96 and 24 dB, a pan between -100 and 100%, and '
    'fade times in seconds.';

/// Parses seconds or an exact rational like `1/4`; negative times are refused.
ProjectRationalTime? parseSecondsOrRational(String text) {
  final trimmed = text.trim();
  if (trimmed.isEmpty) return null;
  final direct = ProjectRationalTime.tryParse(trimmed);
  if (direct != null) return direct;
  final seconds = double.tryParse(trimmed);
  if (seconds == null || !seconds.isFinite || seconds < 0) return null;
  return rationalTimeFromSeconds(seconds);
}

/// Reads the four audio texts into a command payload, or null when any field is
/// out of range. Null is the only failure signal the commit bar needs.
ProjectTimelineAudioSettings? audioSettingsFromTexts({
  required String gain,
  required String pan,
  required String fadeIn,
  required String fadeOut,
}) {
  final gainValue = double.tryParse(gain.trim());
  final panValue = double.tryParse(pan.trim());
  final fadeInValue = parseSecondsOrRational(fadeIn);
  final fadeOutValue = parseSecondsOrRational(fadeOut);
  if (gainValue == null ||
      !gainValue.isFinite ||
      gainValue < -96 ||
      gainValue > 24 ||
      panValue == null ||
      !panValue.isFinite ||
      panValue < -100 ||
      panValue > 100 ||
      fadeInValue == null ||
      fadeInValue.numerator < BigInt.zero ||
      fadeOutValue == null ||
      fadeOutValue.numerator < BigInt.zero) {
    return null;
  }
  return ProjectTimelineAudioSettings(
    gainMilliDecibels: (gainValue * 1000).round(),
    panBasisPoints: (panValue * 100).round(),
    fadeIn: fadeInValue,
    fadeOut: fadeOutValue,
  );
}
