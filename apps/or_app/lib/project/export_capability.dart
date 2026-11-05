import 'package:or_app_bridge/or_app_bridge.dart' as rust;

import 'export_profile.dart';

/// Which export profiles this build can actually deliver, measured by the
/// linked encoder.
///
/// The engine decides; this type only carries the answer, so the Export dialog
/// and the Settings panel cannot claim a format the writer would refuse. A
/// hardware encoder that is present but cannot open on this machine is reported
/// by the export itself, not guessed at here.
class ExportCapability {
  const ExportCapability({
    this.h264Encoder,
    this.h264Hardware = false,
    this.mp4Available = false,
    this.webmAvailable = false,
    this.losslessAvailable = false,
    this.preferredProfile = ExportProfile.mp4H264Aac,
  });

  /// What the app assumes when the engine cannot be asked (a unit test, or a
  /// build whose bridge is not loaded): the platform default it shipped with.
  factory ExportCapability.assumed() => ExportCapability(
    preferredProfile: ExportProfile.defaultForPlatform,
    mp4Available: ExportProfile.defaultForPlatform == ExportProfile.mp4H264Aac,
    webmAvailable: true,
    losslessAvailable: true,
  );

  factory ExportCapability.fromRust(rust.ExportCapabilityView view) =>
      ExportCapability(
        h264Encoder: view.h264Encoder,
        h264Hardware: view.h264Hardware,
        mp4Available: view.mp4Available,
        webmAvailable: view.webmAvailable,
        losslessAvailable: view.losslessAvailable,
        preferredProfile: _profileFromWire(view.preferredProfile),
      );

  final String? h264Encoder;
  final bool h264Hardware;
  final bool mp4Available;
  final bool webmAvailable;
  final bool losslessAvailable;
  final ExportProfile preferredProfile;

  bool supports(ExportProfile profile) => switch (profile) {
    ExportProfile.mp4H264Aac => mp4Available,
    ExportProfile.webmVp9Opus => webmAvailable,
    ExportProfile.matroskaFfv1PcmS16le => losslessAvailable,
  };

  /// True when the build has no lossy delivery format and only lossless works.
  bool get deliveryIsLimited =>
      !mp4Available && !webmAvailable && losslessAvailable;

  /// A one-line statement of what an export will use, for Settings.
  ///
  /// It names the encoder rather than the profile when the profile is MP4,
  /// because "H.264 via VideoToolbox" and "H.264 via libx264" are different
  /// answers to "why is my export slow".
  String get summary {
    if (!mp4Available && !webmAvailable && !losslessAvailable) {
      return 'No export encoder in this build. Reinstall the application package.';
    }
    final parts = <String>[
      if (mp4Available) 'MP4 (${_h264Label()})',
      if (webmAvailable) 'WebM (VP9)',
      if (losslessAvailable) 'Lossless (FFV1)',
    ];
    return 'This build exports ${parts.join(', ')}.';
  }

  /// What a beginner should be told when the build cannot deliver the format
  /// they expect.
  String get caveat {
    if (deliveryIsLimited) {
      return 'This build has no H.264 or VP9 encoder, so exports are lossless '
          'and large. Use a packaged build to export MP4.';
    }
    if (!mp4Available) {
      return 'This build has no H.264 encoder, so MP4 is unavailable and new '
          'exports start on WebM.';
    }
    return '';
  }

  String _h264Label() {
    final encoder = h264Encoder;
    if (encoder == null) return 'H.264';
    final name = switch (encoder) {
      'h264_videotoolbox' => 'VideoToolbox',
      'h264_nvenc' => 'NVENC',
      'h264_mf' => 'Media Foundation',
      'h264_vaapi' => 'VAAPI',
      'h264_open' => 'OpenH264',
      'libx264' => 'x264',
      _ => encoder,
    };
    return h264Hardware ? '$name, hardware' : '$name, software';
  }

  static ExportProfile _profileFromWire(String wire) => switch (wire) {
    'webm_vp9_opus' => ExportProfile.webmVp9Opus,
    'matroska_ffv1_pcm_s16le' => ExportProfile.matroskaFfv1PcmS16le,
    _ => ExportProfile.mp4H264Aac,
  };
}

/// Asks the engine what this build carries, once per launch.
class ExportCapabilityCache {
  ExportCapabilityCache({ExportCapability? measured}) : _value = measured;

  ExportCapability? _value;

  /// The measured capability, or the platform assumption when the engine
  /// cannot answer. Never throws: an unavailable bridge must not stop an export.
  Future<ExportCapability> load() async {
    final cached = _value;
    if (cached != null) return cached;
    try {
      final measured = ExportCapability.fromRust(
        await rust.exportCapabilities(),
      );
      _value = measured;
      return measured;
    } catch (_) {
      return ExportCapability.assumed();
    }
  }
}
