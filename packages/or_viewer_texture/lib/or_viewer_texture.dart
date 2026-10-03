import 'package:flutter/services.dart';

abstract final class OrViewerTexture {
  static const _channel = MethodChannel('or_viewer_texture');

  static Future<bool> setMediaSources(
    List<String> sources, {
    BigInt? generation,
    String? owner,
  }) async =>
      await _channel.invokeMethod<bool>('setMediaSources', {
        'sources': sources,
        if (generation != null) 'generation': generation.toInt(),
        if (owner != null) 'owner': owner,
      }) ??
      false;

  static Future<void> clearMediaSources() async {
    if (await _channel.invokeMethod<bool>('clearMediaSources') != true) {
      throw PlatformException(
        code: 'MEDIA_SOURCE_UNAVAILABLE',
        message: 'Android media descriptors could not be released.',
      );
    }
  }

  static Future<int?> textureId() async {
    try {
      return await _channel.invokeMethod<int>('textureId');
    } on MissingPluginException {
      return null;
    } on PlatformException {
      return null;
    }
  }

  static Future<void> frameAvailable() async {
    try {
      await _channel.invokeMethod<bool>('frameAvailable');
    } on MissingPluginException {
      // The native viewer texture is not registered on this platform.
    } on PlatformException {
      // The viewer is optional on platforms without its native adapter.
    }
  }
}
