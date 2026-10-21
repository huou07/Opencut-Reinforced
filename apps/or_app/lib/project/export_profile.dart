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

  static List<ExportProfile> get available => Platform.isMacOS
      ? values
      : values.where((profile) => profile != mp4H264Aac).toList();

  static ExportProfile get defaultForPlatform =>
      Platform.isMacOS ? mp4H264Aac : webmVp9Opus;

  String suggestedName(String projectName) {
    final base = projectName.replaceAll(RegExp(r'[\\/:*?"<>|]'), '_').trim();
    return '${base.isEmpty ? 'Untitled' : base}.$extension';
  }
}
