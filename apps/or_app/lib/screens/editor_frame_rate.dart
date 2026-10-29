import '../project/project_gateway.dart';

/// One choice in the sequence frame-rate menu.
class EditorFrameRateChoice {
  const EditorFrameRateChoice({required this.rate, required this.label});
  const EditorFrameRateChoice.unset()
    : rate = null,
      label = 'Remove frame rate';

  final ProjectRationalRate? rate;
  final String label;
}

/// Frame rates the sequence can carry. One list, because the viewer transport
/// and the inspector both offer the same choice and a second copy would drift.
const editorSequenceRateChoices = [
  EditorFrameRateChoice(
    rate: ProjectRationalRate(24000, 1001),
    label: '23.976 (24000/1001) fps',
  ),
  EditorFrameRateChoice(rate: ProjectRationalRate(24, 1), label: '24 fps'),
  EditorFrameRateChoice(rate: ProjectRationalRate(25, 1), label: '25 fps'),
  EditorFrameRateChoice(rate: ProjectRationalRate(30, 1), label: '30 fps'),
  EditorFrameRateChoice(rate: ProjectRationalRate(50, 1), label: '50 fps'),
  EditorFrameRateChoice(rate: ProjectRationalRate(60, 1), label: '60 fps'),
  EditorFrameRateChoice.unset(),
];

/// Human frame-rate label: `30 fps`, `23.976 fps`, never `30/1 fps`.
String frameRateLabel(ProjectRationalRate rate) {
  final value = rate.numerator / rate.denominator;
  final text = value == value.roundToDouble()
      ? value.toStringAsFixed(0)
      : value.toStringAsFixed(3);
  return '$text fps';
}
