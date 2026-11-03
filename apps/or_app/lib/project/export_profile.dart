import 'dart:io';

enum ExportProfile {
  mp4H264Aac(
    wireName: 'mp4_h264_aac',
    extension: 'mp4',
    mimeType: 'video/mp4',
    label: 'MP4 video',
    description: 'H.264 video and AAC audio for broad compatibility.',
  ),
  webmVp9Opus(
    wireName: 'webm_vp9_opus',
    extension: 'webm',
    mimeType: 'video/webm',
    label: 'WebM video',
    description:
        'Smaller file for sharing, with lossy VP9 video and Opus audio.',
  ),
  matroskaFfv1PcmS16le(
    wireName: 'matroska_ffv1_pcm_s16le',
    extension: 'mkv',
    mimeType: 'video/x-matroska',
    label: 'Matroska lossless',
    description:
        'Lossless video and uncompressed audio. Files are much larger.',
  );

  const ExportProfile({
    required this.wireName,
    required this.extension,
    required this.mimeType,
    required this.label,
    required this.description,
  });

  final String wireName;
  final String extension;
  final String mimeType;
  final String label;
  final String description;

  /// Whether the profile has a rate control the quality preference can trade.
  bool get hasQualityChoice => this != ExportProfile.matroskaFfv1PcmS16le;

  /// Every profile is offered on every desktop platform.
  ///
  /// MP4 used to disappear outside macOS because the encoder named the Apple
  /// VideoToolbox encoder directly. The core now picks whichever H.264 encoder
  /// the build carries on this host (VideoToolbox on macOS, Media Foundation on
  /// Windows), so hiding the format would remove a feature the product can
  /// deliver.
  static List<ExportProfile> get available => values;

  /// The profile a new export starts on.
  ///
  /// MP4 is the format people expect from a video editor. Linux keeps WebM as
  /// the default until that package carries a system H.264 encoder, so the
  /// first export there does not fail on a machine with no vendor encoder.
  static ExportProfile get defaultForPlatform =>
      Platform.isLinux ? webmVp9Opus : mp4H264Aac;

  String suggestedName(String projectName) {
    final base = projectName.replaceAll(RegExp(r'[\\/:*?"<>|]'), '_').trim();
    return '${base.isEmpty ? 'Untitled' : base}.$extension';
  }
}

/// How much a lossy export may spend on the picture.
///
/// The lossless profile has no rate control, so the choice only applies to WebM
/// and MP4. Quality is the default: an export is usually the last thing a project
/// does, and the local encoder is faster than the renderer at delivery sizes.
enum ExportQuality {
  quality(
    wireName: 'quality',
    label: 'Quality',
    description: 'Slower encode, cleaner picture, larger file.',
  ),
  speed(
    wireName: 'speed',
    label: 'Faster',
    description: 'Quickest encode, smallest file, softer picture.',
  );

  const ExportQuality({
    required this.wireName,
    required this.label,
    required this.description,
  });

  final String wireName;
  final String label;
  final String description;

  static ExportQuality get defaultChoice => ExportQuality.quality;
}
