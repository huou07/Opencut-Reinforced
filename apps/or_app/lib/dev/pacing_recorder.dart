import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/scheduler.dart';

/// Records real frame timings into a file so a packaged release build can be
/// measured without the Flutter test binding, which cannot run release.
///
/// Enabled only when the binary is built with
/// `--dart-define=OR_PACING_LOG=/absolute/path.jsonl`; otherwise [install] is a
/// no-op and costs nothing at runtime. One JSON line is written per
/// [windowFrames] frames so a long session does not grow memory.
class PacingRecorder {
  PacingRecorder({required this.logPath, this.windowFrames = 120});

  static const String _logPathDefine = String.fromEnvironment('OR_PACING_LOG');

  /// Installs the recorder when the build asked for one.
  static PacingRecorder? install() {
    if (_logPathDefine.isEmpty) {
      return null;
    }
    final recorder = PacingRecorder(logPath: _logPathDefine);
    recorder.start();
    return recorder;
  }

  final String logPath;
  final int windowFrames;
  final List<FrameTiming> _window = <FrameTiming>[];
  IOSink? _sink;

  void start() {
    try {
      _sink = File(logPath).openWrite(mode: FileMode.append);
    } on FileSystemException {
      // A probe that cannot write must not break the editor it measures.
      _sink = null;
      return;
    }
    SchedulerBinding.instance.addTimingsCallback(_onTimings);
  }

  void _onTimings(List<FrameTiming> timings) {
    _window.addAll(timings);
    if (_window.length < windowFrames) {
      return;
    }
    final frames = List<FrameTiming>.of(_window);
    _window.clear();
    _write(frames);
  }

  void _write(List<FrameTiming> frames) {
    final builds = <double>[];
    final rasters = <double>[];
    final gaps = <double>[];
    int? previousVsync;
    for (final frame in frames) {
      final vsync = frame.timestampInMicroseconds(ui.FramePhase.vsyncStart);
      if (previousVsync != null) {
        gaps.add((vsync - previousVsync) / 1000);
      }
      previousVsync = vsync;
      builds.add(frame.buildDuration.inMicroseconds / 1000);
      rasters.add(frame.rasterDuration.inMicroseconds / 1000);
    }
    builds.sort();
    rasters.sort();
    gaps.sort();
    final span =
        (frames.last.timestampInMicroseconds(ui.FramePhase.vsyncStart) -
            frames.first.timestampInMicroseconds(ui.FramePhase.vsyncStart)) /
        1e6;
    final fps = span <= 0 ? 0.0 : frames.length / span;
    final line =
        '{"frames":${frames.length},'
        '"span_s":${span.toStringAsFixed(3)},'
        '"fps":${fps.toStringAsFixed(1)},'
        '"build_p50_ms":${_percentile(builds, 0.50)},'
        '"build_p95_ms":${_percentile(builds, 0.95)},'
        '"raster_p50_ms":${_percentile(rasters, 0.50)},'
        '"raster_p95_ms":${_percentile(rasters, 0.95)},'
        '"gap_p50_ms":${_percentile(gaps, 0.50)},'
        '"gap_p95_ms":${_percentile(gaps, 0.95)},'
        '"gap_max_ms":${gaps.isEmpty ? '0.00' : gaps.last.toStringAsFixed(2)}}';
    _sink?.writeln(line);
  }

  static String _percentile(List<double> sorted, double fraction) {
    if (sorted.isEmpty) {
      return '0.00';
    }
    final index = ((sorted.length - 1) * fraction).round();
    return sorted[index].toStringAsFixed(2);
  }

  /// Flushes the pending window. Called by tests; a packaged build relies on
  /// process exit.
  Future<void> stop() async {
    SchedulerBinding.instance.removeTimingsCallback(_onTimings);
    if (_window.isNotEmpty) {
      _write(List<FrameTiming>.of(_window));
      _window.clear();
    }
    await _sink?.flush();
    await _sink?.close();
    _sink = null;
  }
}
