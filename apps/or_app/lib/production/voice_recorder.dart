import 'dart:io';

import 'package:flutter/services.dart';

class VoiceRecorderException implements Exception {
  const VoiceRecorderException(this.code, this.message);

  final String code;
  final String message;

  @override
  String toString() => message;
}

/// Microphone capture through the app's native macOS bridge.
///
/// The recorder writes one 48 kHz mono 16-bit PCM WAV; the same file is what the
/// import path probes, so a recording is ordinary media.
class VoiceRecorder {
  const VoiceRecorder({MethodChannel? channel})
    : _channel = channel ?? const MethodChannel('or_narration');

  final MethodChannel _channel;

  /// `granted`, `denied`, `restricted`, `undetermined` or `unknown`.
  Future<String> permission() async {
    try {
      return await _channel.invokeMethod<String>('recordPermission') ??
          'unknown';
    } on PlatformException catch (error) {
      throw VoiceRecorderException(
        error.code,
        error.message ?? 'Microphone permission could not be checked.',
      );
    } on MissingPluginException {
      throw const VoiceRecorderException(
        'RECORDER_UNAVAILABLE',
        'Microphone recording is not available in this build.',
      );
    }
  }

  Future<void> start(File output) async {
    await output.parent.create(recursive: true);
    try {
      final started = await _channel.invokeMethod<bool>('recordStart', {
        'path': output.path,
      });
      if (started != true) {
        throw const VoiceRecorderException(
          'RECORDING_FAILED',
          'The microphone could not start recording.',
        );
      }
    } on PlatformException catch (error) {
      throw VoiceRecorderException(
        error.code,
        error.message ?? 'The microphone could not start recording.',
      );
    } on MissingPluginException {
      throw const VoiceRecorderException(
        'RECORDER_UNAVAILABLE',
        'Microphone recording is not available in this build.',
      );
    }
  }

  /// Stops capture and returns the duration the recorder measured in ms.
  Future<int> stop() async {
    try {
      final result = await _channel.invokeMapMethod<String, Object?>(
        'recordStop',
      );
      final duration = result?['duration_ms'];
      return duration is int && duration >= 0 ? duration : 0;
    } on PlatformException catch (error) {
      throw VoiceRecorderException(
        error.code,
        error.message ?? 'The recording could not be stopped.',
      );
    } on MissingPluginException {
      throw const VoiceRecorderException(
        'RECORDER_UNAVAILABLE',
        'Microphone recording is not available in this build.',
      );
    }
  }
}
