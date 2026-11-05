import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/export_capability.dart';
import 'package:or_app/project/export_profile.dart';

void main() {
  group('export capability', () {
    test('names the hardware encoder a macOS package carries', () {
      const capability = ExportCapability(
        h264Encoder: 'h264_videotoolbox',
        h264Hardware: true,
        mp4Available: true,
        webmAvailable: true,
        losslessAvailable: true,
        preferredProfile: ExportProfile.mp4H264Aac,
      );

      expect(
        capability.summary,
        'This build exports MP4 (VideoToolbox, hardware), WebM (VP9), '
        'Lossless (FFV1).',
      );
      expect(capability.caveat, isEmpty);
      expect(capability.deliveryIsLimited, isFalse);
      for (final profile in ExportProfile.values) {
        expect(capability.supports(profile), isTrue);
      }
    });

    test('a build without H.264 says so and starts on WebM', () {
      const capability = ExportCapability(
        webmAvailable: true,
        losslessAvailable: true,
        preferredProfile: ExportProfile.webmVp9Opus,
      );

      expect(capability.supports(ExportProfile.mp4H264Aac), isFalse);
      expect(capability.preferredProfile, ExportProfile.webmVp9Opus);
      expect(
        capability.summary,
        'This build exports WebM (VP9), Lossless (FFV1).',
      );
      expect(
        capability.caveat,
        'This build has no H.264 encoder, so MP4 is unavailable and new '
        'exports start on WebM.',
      );
      expect(capability.deliveryIsLimited, isFalse);
    });

    test('a lossless-only build warns that delivery files are large', () {
      const capability = ExportCapability(
        losslessAvailable: true,
        preferredProfile: ExportProfile.matroskaFfv1PcmS16le,
      );

      expect(capability.deliveryIsLimited, isTrue);
      expect(
        capability.caveat,
        'This build has no H.264 or VP9 encoder, so exports are lossless '
        'and large. Use a packaged build to export MP4.',
      );
    });

    test('a build with no encoder tells the user to reinstall', () {
      const capability = ExportCapability();

      expect(
        capability.summary,
        'No export encoder in this build. Reinstall the application package.',
      );
      expect(capability.supports(ExportProfile.mp4H264Aac), isFalse);
    });

    test('a software encoder is labelled as one', () {
      const capability = ExportCapability(
        h264Encoder: 'libx264',
        mp4Available: true,
        preferredProfile: ExportProfile.mp4H264Aac,
      );

      expect(capability.summary, contains('x264, software'));
    });

    test('the assumption falls back to the platform default', () {
      final assumed = ExportCapability.assumed();

      expect(assumed.preferredProfile, ExportProfile.defaultForPlatform);
      expect(assumed.supports(assumed.preferredProfile), isTrue);
      expect(assumed.webmAvailable, isTrue);
      expect(assumed.losslessAvailable, isTrue);
    });

    test('a fixed cache answers every time without the engine', () async {
      const fixed = ExportCapability(mp4Available: false, webmAvailable: true);
      final cache = ExportCapabilityCache(measured: fixed);

      expect(await cache.load(), same(fixed));
      expect(await cache.load(), same(fixed));
    });
  });
}
