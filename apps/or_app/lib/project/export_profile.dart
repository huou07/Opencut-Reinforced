import 'dart:io' show Platform;

enum ExportProfile {
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

  static List<ExportProfile> get available =>
      Platform.isAndroid ? const [ExportProfile.matroskaFfv1PcmS16le] : values;

  static ExportProfile get defaultForPlatform =>
      Platform.isAndroid ? matroskaFfv1PcmS16le : webmVp9Opus;

  String suggestedName(String projectName) {
    final base = projectName.replaceAll(RegExp(r'[\\/:*?"<>|]'), '_').trim();
    return '${base.isEmpty ? 'Untitled' : base}.$extension';
  }
}
