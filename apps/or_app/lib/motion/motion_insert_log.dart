import 'package:flutter/foundation.dart';

/// Where a materialised motion clip came from. The source decides whether the
/// Motion panel can render the clip again: a clip produced from the panel's own
/// draft or selection can, a clip produced by the illustration-video dialog
/// cannot be reproduced from the panel and must not pretend to be.
enum MotionInsertSource { draft, asset, dialog }

/// One materialised motion clip that was added to the active timeline.
@immutable
class MotionInsertRecord {
  const MotionInsertRecord({
    required this.outputPath,
    required this.sceneName,
    required this.width,
    required this.height,
    required this.frameCount,
    required this.source,
  });

  final String outputPath;
  final String sceneName;
  final int width;
  final int height;
  final int frameCount;
  final MotionInsertSource source;

  bool get rerenderable =>
      source == MotionInsertSource.draft || source == MotionInsertSource.asset;
}

/// The last motion clip the Motion panel put on the timeline, so the panel can
/// keep showing its facts and offer to render it again instead of dropping the
/// result into a one-shot snackbar.
class MotionInsertLog extends ChangeNotifier {
  MotionInsertRecord? _last;

  MotionInsertRecord? get last => _last;

  void record(MotionInsertRecord record) {
    _last = record;
    notifyListeners();
  }

  /// A different project has no relation to the previous clip.
  void clear() {
    if (_last == null) return;
    _last = null;
    notifyListeners();
  }
}
