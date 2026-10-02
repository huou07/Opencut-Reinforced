import 'package:flutter/services.dart';

abstract final class OrViewerTexture {
  static const _channel = MethodChannel('or_viewer_texture');

  static Future<bool> setMediaSources(List<String> sources) async {
    try {
      return await _channel.invokeMethod<bool>('setMediaSources', sources) ??
          false;
    } on MissingPluginException {
      return false;
    } on PlatformException {
      return false;
    }
  }

  static Future<void> clearMediaSources() async {
    try {
      await _channel.invokeMethod<bool>('clearMediaSources');
    } on MissingPluginException {
      // The native media adapter is not registered on this platform.
    } on PlatformException {
      // The native media adapter is optional outside Android.
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
