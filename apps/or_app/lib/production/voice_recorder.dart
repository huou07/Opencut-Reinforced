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
///
/// macOS grants microphone access through TCC, and the grant only exists because
/// the app asked for it: [requestPermission] presents the system prompt and puts
/// the app under Privacy & Security → Microphone. Reading [permission] never
/// prompts, so it is safe to call on open.
class VoiceRecorder {
  const VoiceRecorder({MethodChannel? channel})
    : _channel = channel ?? const MethodChannel('or_narration');

  final MethodChannel _channel;

  /// `granted`, `denied`, `restricted`, `undetermined` or `unknown`.
  Future<String> permission() => _status(
    'recordPermission',
    'Microphone permission could not be checked.',
  );

  /// Asks macOS for microphone access and reports the resulting state.
  ///
  /// The first call presents the system prompt; an already-decided state is
  /// returned without asking again.
  Future<String> requestPermission() => _status(
    'recordPermissionRequest',
    'Microphone permission could not be requested.',
  );

  /// Opens System Settings on the microphone pane.
  ///
  /// This is the recovery path after a denial: the app is listed there only
  /// because it asked for access at least once.
  Future<bool> openSystemSettings() async {
    try {
      return await _channel.invokeMethod<bool>('recordPermissionSettings') ??
          false;
    } on PlatformException catch (error) {
      throw VoiceRecorderException(
        error.code,
        error.message ?? 'System Settings could not be opened.',
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
    // A take is the first moment the app needs the microphone. Ask here too, so
    // every caller presents the macOS prompt instead of failing with a denial
    // the user never had a chance to answer.
    if (await permission() == 'undetermined') {
      await requestPermission();
    }
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

  Future<String> _status(String method, String fallback) async {
    try {
      return await _channel.invokeMethod<String>(method) ?? 'unknown';
    } on PlatformException catch (error) {
      throw VoiceRecorderException(error.code, error.message ?? fallback);
    } on MissingPluginException {
      throw const VoiceRecorderException(
        'RECORDER_UNAVAILABLE',
        'Microphone recording is not available in this build.',
      );
    }
  }
}
