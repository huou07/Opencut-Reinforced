/// What a MotionScene render actually produced, kept so the editor can report it
/// after the dialog closes. A scene is a file plus an insert, and both facts are
/// the author's business: the card states the file, its size and whether it is
/// already on the timeline.
///
/// This lives in the project layer rather than beside the dialog because the
/// editor reads it and the editor must not depend on shell widgets.
class MotionSceneSnapshot {
  const MotionSceneSnapshot({
    required this.title,
    required this.sourcePath,
    required this.destinationPath,
    required this.frameRate,
    required this.width,
    required this.height,
    required this.frameCount,
    required this.bytesWritten,
    this.inserted = false,
  });

  final String title;
  final String sourcePath;
  final String destinationPath;
  final int frameRate;
  final int width;
  final int height;
  final int frameCount;
  final int bytesWritten;
  final bool inserted;

  MotionSceneSnapshot copyWith({bool? inserted}) => MotionSceneSnapshot(
    title: title,
    sourcePath: sourcePath,
    destinationPath: destinationPath,
    frameRate: frameRate,
    width: width,
    height: height,
    frameCount: frameCount,
    bytesWritten: bytesWritten,
    inserted: inserted ?? this.inserted,
  );
}
