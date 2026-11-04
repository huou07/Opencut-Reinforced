import '../project/project_gateway.dart';

/// One reusable text look.
///
/// A preset is only a value: it carries the exact [ProjectTextFormatting] the
/// text command takes, so applying one creates an ordinary text clip and never a
/// second styling model. The library screen renders [sample] with the same
/// values, which is why the preview cannot drift from the result.
class TextPreset {
  const TextPreset({
    required this.id,
    required this.name,
    required this.description,
    required this.sample,
    required this.formatting,
    required this.useCase,
  });

  final String id;
  final String name;
  final String description;
  final String sample;
  final ProjectTextFormatting formatting;

  /// Plain-language guidance for when the look works, shown under the sample.
  final String useCase;
}

/// Text presets, ordered from the largest title to the smallest caption.
///
/// Sizes follow the 1080p frame: a title is readable at 96 pt, a caption at
/// 40 pt. Every preset stays inside the monochrome palette; a coloured caption
/// is a per-project edit, not a preset.
const List<TextPreset> textPresets = [
  TextPreset(
    id: 'title',
    name: 'Title',
    description: 'Large centred headline for an opening frame.',
    sample: 'Chapter one',
    useCase: 'Opening frame of a video or a hard cut to a new section.',
    formatting: ProjectTextFormatting(
      font: ProjectFontIdentity.bundledInter,
      sizeMilliPoints: 96000,
      weight: ProjectTextWeight.semibold,
      alignment: ProjectTextAlignment.center,
      color: ProjectTextColor.white,
      anchor: ProjectTextVerticalAnchor.center,
    ),
  ),
  TextPreset(
    id: 'subtitle',
    name: 'Subtitle',
    description: 'Secondary line under a title.',
    sample: 'A short series on editing at home',
    useCase: 'Directly under a Title, at 60–70 % of its size.',
    formatting: ProjectTextFormatting(
      font: ProjectFontIdentity.bundledInter,
      sizeMilliPoints: 56000,
      weight: ProjectTextWeight.regular,
      alignment: ProjectTextAlignment.center,
      color: ProjectTextColor(red: 214, green: 214, blue: 218, alpha: 255),
      anchor: ProjectTextVerticalAnchor.center,
      offsetYBasisPoints: 900,
    ),
  ),
  TextPreset(
    id: 'lower-third',
    name: 'Lower third',
    description: 'Name plate in the lower-left corner.',
    sample: 'Mai Phạm · Sound designer',
    useCase: 'Naming a speaker without covering their face.',
    formatting: ProjectTextFormatting(
      font: ProjectFontIdentity.beVietnamPro,
      sizeMilliPoints: 36000,
      weight: ProjectTextWeight.medium,
      alignment: ProjectTextAlignment.start,
      color: ProjectTextColor.white,
      anchor: ProjectTextVerticalAnchor.bottom,
      offsetXBasisPoints: -3200,
      offsetYBasisPoints: -1200,
      background: ProjectTextBackground(
        color: ProjectTextColor(red: 9, green: 9, blue: 9, alpha: 190),
      ),
    ),
  ),
  TextPreset(
    id: 'caption',
    name: 'Caption',
    description: 'Bottom-centred line with a readable backing.',
    sample: 'Bạn có thể bắt đầu bằng một clip duy nhất.',
    useCase: 'Spoken content where the words must survive a busy frame.',
    formatting: ProjectTextFormatting(
      font: ProjectFontIdentity.beVietnamPro,
      sizeMilliPoints: 40000,
      weight: ProjectTextWeight.medium,
      alignment: ProjectTextAlignment.center,
      color: ProjectTextColor.white,
      anchor: ProjectTextVerticalAnchor.bottom,
      offsetYBasisPoints: -1500,
      background: ProjectTextBackground(
        color: ProjectTextColor(red: 9, green: 9, blue: 9, alpha: 170),
      ),
    ),
  ),
  TextPreset(
    id: 'quote',
    name: 'Quote',
    description: 'Centred pull quote with room around it.',
    sample: 'Edit the cut, not the file.',
    useCase: 'A statement you want the viewer to read at their own pace.',
    formatting: ProjectTextFormatting(
      font: ProjectFontIdentity.beVietnamPro,
      sizeMilliPoints: 52000,
      weight: ProjectTextWeight.medium,
      alignment: ProjectTextAlignment.center,
      color: ProjectTextColor.white,
      anchor: ProjectTextVerticalAnchor.center,
      opacityBasisPoints: 9500,
    ),
  ),
  TextPreset(
    id: 'chapter',
    name: 'Chapter marker',
    description: 'Small label in the top-left corner.',
    sample: '02 · Lighting',
    useCase: 'Orienting the viewer inside a longer tutorial.',
    formatting: ProjectTextFormatting(
      font: ProjectFontIdentity.bundledInter,
      sizeMilliPoints: 30000,
      weight: ProjectTextWeight.semibold,
      alignment: ProjectTextAlignment.start,
      color: ProjectTextColor(red: 226, green: 226, blue: 230, alpha: 255),
      anchor: ProjectTextVerticalAnchor.top,
      offsetXBasisPoints: -3200,
      offsetYBasisPoints: 1200,
    ),
  ),
  TextPreset(
    id: 'end-card',
    name: 'End card',
    description: 'Closing line with a smaller follow-up.',
    sample: 'Thanks for watching',
    useCase: 'The last two seconds, after the content has finished.',
    formatting: ProjectTextFormatting(
      font: ProjectFontIdentity.bundledInter,
      sizeMilliPoints: 68000,
      weight: ProjectTextWeight.semibold,
      alignment: ProjectTextAlignment.center,
      color: ProjectTextColor.white,
      anchor: ProjectTextVerticalAnchor.center,
    ),
  ),
];

/// The preset with [id], or null when a template names one that no longer
/// exists. Callers treat null as "skip the overlay" instead of guessing.
TextPreset? textPresetById(String id) {
  for (final preset in textPresets) {
    if (preset.id == id) return preset;
  }
  return null;
}
