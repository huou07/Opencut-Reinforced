import 'dart:typed_data';

abstract interface class ProjectSessionHandle {}

/// Exact project time. `secondsForDisplay` is for layout and ruler labels only.
class ProjectRationalTime {
  const ProjectRationalTime(this.numerator, this.denominator);

  static final _minI64 = -(BigInt.one << 63);
  static final _maxI64 = (BigInt.one << 63) - BigInt.one;
  static final _maxU32 = (BigInt.one << 32) - BigInt.one;
  static final _syntax = RegExp(r'^[+-]?[0-9]+/[0-9]+$');

  final BigInt numerator;
  final int denominator;

  String get canonical {
    if (denominator <= 0) return '$numerator/$denominator';
    var left = numerator.abs();
    var right = BigInt.from(denominator);
    while (right != BigInt.zero) {
      final remainder = left % right;
      left = right;
      right = remainder;
    }
    return '${numerator ~/ left}/${BigInt.from(denominator) ~/ left}';
  }

  double get secondsForDisplay => numerator.toDouble() / denominator;
  bool get isPositive => numerator > BigInt.zero;

  ProjectRationalTime add(ProjectRationalTime other) {
    final commonDenominator =
        BigInt.from(denominator) * BigInt.from(other.denominator);
    final sum =
        numerator * BigInt.from(other.denominator) +
        other.numerator * BigInt.from(denominator);
    var divisor = sum.abs();
    var remainder = commonDenominator;
    while (remainder != BigInt.zero) {
      final next = divisor % remainder;
      divisor = remainder;
      remainder = next;
    }
    return ProjectRationalTime(
      sum ~/ divisor,
      (commonDenominator ~/ divisor).toInt(),
    );
  }

  int compareTo(ProjectRationalTime other) =>
      (numerator * BigInt.from(other.denominator)).compareTo(
        other.numerator * BigInt.from(denominator),
      );

  static ProjectRationalTime? tryParse(String value) {
    final match = _syntax.firstMatch(value);
    if (match == null) return null;
    final parts = value.split('/');
    final numerator = BigInt.tryParse(parts[0]);
    final denominator = BigInt.tryParse(parts[1]);
    if (numerator == null ||
        denominator == null ||
        numerator < _minI64 ||
        numerator > _maxI64 ||
        denominator <= BigInt.zero ||
        denominator > _maxU32) {
      return null;
    }
    return ProjectRationalTime(numerator, denominator.toInt());
  }
}

class ProjectRationalRate {
  const ProjectRationalRate(this.numerator, this.denominator);

  final int numerator;
  final int denominator;

  String get canonical => '$numerator/$denominator';
}

class ProjectPreviewState {
  const ProjectPreviewState({
    required this.position,
    required this.playing,
    required this.generation,
    required this.frameSequence,
    required this.width,
    required this.height,
    this.presentedTime,
    this.sequenceFrameRate,
    this.contentEnd,
    this.errorCode,
    this.errorMessage,
  });

  final ProjectRationalTime position;
  final ProjectRationalTime? presentedTime;
  final ProjectRationalRate? sequenceFrameRate;
  final ProjectRationalTime? contentEnd;
  final bool playing;
  final BigInt generation;
  final BigInt frameSequence;
  final int width;
  final int height;
  final String? errorCode;
  final String? errorMessage;
}

enum ProjectPreviewFrameStep { previous, next }

class ProjectReadModel {
  const ProjectReadModel({
    required this.projectId,
    required this.projectInstanceId,
    required this.revision,
    required this.name,
    required this.dirty,
    required this.descriptorPath,
  });

  final String projectId;
  final String projectInstanceId;
  final BigInt revision;
  final String name;
  final bool dirty;
  final String descriptorPath;
}

class ProjectActionResult {
  const ProjectActionResult({
    required this.succeeded,
    this.errorCode = '',
    this.message = '',
    this.view,
  });

  final bool succeeded;
  final String errorCode;
  final String message;
  final ProjectReadModel? view;
}

/// Read-only media data returned by the bounded project query.
class ProjectMediaItem {
  const ProjectMediaItem({
    required this.mediaId,
    required this.sourceUri,
    required this.formatNames,
    required this.duration,
    required this.videoDetails,
    required this.audioDetails,
    this.containerDuration,
    this.firstVideoDuration,
    this.firstAudioDuration,
  });

  final String mediaId;
  final String sourceUri;
  final List<String> formatNames;
  final String? duration;
  final String? videoDetails;
  final String? audioDetails;
  final ProjectRationalTime? containerDuration;
  final ProjectRationalTime? firstVideoDuration;
  final ProjectRationalTime? firstAudioDuration;
}

/// One bounded page from the project's insertion-ordered media library.
class ProjectMediaPage {
  const ProjectMediaPage({
    required this.projectId,
    required this.projectInstanceId,
    required this.projectRevision,
    required this.items,
    required this.totalCount,
    required this.offset,
    required this.limit,
    required this.nextOffset,
  });

  final String projectId;
  final String projectInstanceId;
  final BigInt projectRevision;
  final List<ProjectMediaItem> items;
  final int totalCount;
  final int offset;
  final int limit;
  final int? nextOffset;
}

enum ProjectTimelineTrackKind { video, audio }

enum ProjectTimelineTrimEdge { start, end }

enum ProjectTimelineSnapOperation { move, trimStart, trimEnd }

enum ProjectTimelineSnapMovingAnchor { none, start, end }

enum ProjectTimelineSnapTargetKind {
  none,
  timelineZero,
  clipStart,
  clipEnd,
  marker,
}

class ProjectTimelineTrack {
  const ProjectTimelineTrack({
    required this.trackId,
    required this.kind,
    required this.clipCount,
  });

  final String trackId;
  final ProjectTimelineTrackKind kind;
  final int clipCount;
}

class ProjectTimelineTracks {
  ProjectTimelineTracks({
    required this.projectId,
    required this.projectInstanceId,
    required this.projectRevision,
    required List<ProjectTimelineTrack> items,
  }) : items = List.unmodifiable(items);

  final String projectId;
  final String projectInstanceId;
  final BigInt projectRevision;
  final List<ProjectTimelineTrack> items;
}

class ProjectTimelineClip {
  const ProjectTimelineClip({
    required this.clipId,
    required this.mediaId,
    required this.timelineStart,
    required this.sourceStart,
    required this.sourceDuration,
  });

  final String clipId;
  final String mediaId;
  final ProjectRationalTime timelineStart;
  final ProjectRationalTime sourceStart;
  final ProjectRationalTime sourceDuration;

  ProjectRationalTime get timelineEnd => timelineStart.add(sourceDuration);
}

class ProjectTimelineClipPage {
  ProjectTimelineClipPage({
    required this.projectId,
    required this.projectInstanceId,
    required this.projectRevision,
    required this.trackId,
    required List<ProjectTimelineClip> items,
    required this.totalCount,
    required this.offset,
    required this.limit,
    required this.nextOffset,
  }) : items = List.unmodifiable(items);

  final String projectId;
  final String projectInstanceId;
  final BigInt projectRevision;
  final String trackId;
  final List<ProjectTimelineClip> items;
  final int totalCount;
  final int offset;
  final int limit;
  final int? nextOffset;
}

class ProjectTimelineMarker {
  const ProjectTimelineMarker({
    required this.markerId,
    required this.timelineTime,
    required this.label,
  });

  final String markerId;
  final ProjectRationalTime timelineTime;
  final String label;
}

class ProjectTimelineMarkerPage {
  ProjectTimelineMarkerPage({
    required this.projectId,
    required this.projectInstanceId,
    required this.projectRevision,
    required List<ProjectTimelineMarker> items,
    required this.totalCount,
    required this.offset,
    required this.limit,
    required this.nextOffset,
  }) : items = List.unmodifiable(items);

  final String projectId;
  final String projectInstanceId;
  final BigInt projectRevision;
  final List<ProjectTimelineMarker> items;
  final int totalCount;
  final int offset;
  final int limit;
  final int? nextOffset;
}

class ProjectTimelineSnapResult {
  const ProjectTimelineSnapResult({
    required this.projectId,
    required this.projectInstanceId,
    required this.projectRevision,
    required this.rawTargetTime,
    required this.resolvedTargetTime,
    required this.snapped,
    required this.movingAnchor,
    required this.targetKind,
    required this.targetTime,
    this.targetTrackId,
    this.targetClipId,
    this.targetMarkerId,
  });

  final String projectId;
  final String projectInstanceId;
  final BigInt projectRevision;
  final ProjectRationalTime rawTargetTime;
  final ProjectRationalTime resolvedTargetTime;
  final bool snapped;
  final ProjectTimelineSnapMovingAnchor movingAnchor;
  final ProjectTimelineSnapTargetKind targetKind;
  final ProjectRationalTime targetTime;
  final String? targetTrackId;
  final String? targetClipId;
  final String? targetMarkerId;
}

enum ProjectMediaArtifactKind { thumbnail, waveform }

enum ProjectMediaArtifactRequestState {
  ready,
  queued,
  running,
  notApplicable,
  failed,
}

enum ProjectMediaArtifactEventState { succeeded, failed, cancelled }

class ProjectMediaArtifactRequest {
  const ProjectMediaArtifactRequest({
    required this.mediaId,
    required this.kind,
    required this.state,
    this.cacheKey,
    this.jobId,
    this.errorCode,
    this.message,
  });

  final String mediaId;
  final ProjectMediaArtifactKind kind;
  final ProjectMediaArtifactRequestState state;
  final String? cacheKey;
  final String? jobId;
  final String? errorCode;
  final String? message;
}

class ProjectMediaArtifact {
  const ProjectMediaArtifact({required this.bytes, required this.mimeType});

  final Uint8List bytes;
  final String mimeType;
}

class ProjectMediaArtifactEvent {
  const ProjectMediaArtifactEvent({
    required this.sequence,
    required this.mediaId,
    required this.kind,
    required this.cacheKey,
    required this.jobId,
    required this.state,
    this.errorCode,
  });

  final BigInt sequence;
  final String mediaId;
  final ProjectMediaArtifactKind kind;
  final String cacheKey;
  final String jobId;
  final ProjectMediaArtifactEventState state;
  final String? errorCode;
}

class ProjectMediaPreview {
  const ProjectMediaPreview({
    required this.kind,
    required this.state,
    this.cacheKey,
    this.jobId,
    this.bytes,
    this.errorCode,
  });

  final ProjectMediaArtifactKind kind;
  final ProjectMediaArtifactRequestState state;
  final String? cacheKey;
  final String? jobId;
  final Uint8List? bytes;
  final String? errorCode;
}

class ProjectHostEvent {
  const ProjectHostEvent({
    required this.sequence,
    required this.kind,
    required this.projectId,
    required this.projectInstanceId,
    required this.revision,
    required this.dirty,
  });

  final BigInt sequence;
  final String kind;
  final String projectId;
  final String projectInstanceId;
  final BigInt revision;
  final bool dirty;
}

enum ProjectRecoveryKind { none, candidate, stale, conflict, invalid }

class ProjectRecoveryInspection {
  const ProjectRecoveryInspection({
    required this.kind,
    required this.projectId,
    required this.baseRevision,
    required this.recoveryRevision,
    required this.recoveryName,
    required this.conflictReason,
    required this.message,
  });

  final ProjectRecoveryKind kind;
  final String projectId;
  final BigInt baseRevision;
  final BigInt recoveryRevision;
  final String recoveryName;
  final String conflictReason;
  final String message;
}

class ProjectRecoveryActionResult {
  const ProjectRecoveryActionResult({
    required this.succeeded,
    required this.changed,
    required this.message,
  });

  final bool succeeded;
  final bool changed;
  final String message;
}

class ProjectGatewayException implements Exception {
  const ProjectGatewayException(this.code, this.message);

  final String code;
  final String message;

  @override
  String toString() => '$code: $message';
}

abstract interface class ProjectGateway {
  Future<ProjectSessionHandle> createProject(String path, String name);
  Future<ProjectSessionHandle> openProject(String path);
  Future<ProjectReadModel> summary(ProjectSessionHandle session);
  Future<ProjectPreviewState> previewState(ProjectSessionHandle session);
  Future<ProjectPreviewState> previewSeek(
    ProjectSessionHandle session,
    ProjectRationalTime position,
  );
  Future<ProjectPreviewState> previewPlay(ProjectSessionHandle session);
  Future<ProjectPreviewState> previewPause(ProjectSessionHandle session);
  Future<ProjectPreviewState> previewStep(
    ProjectSessionHandle session,
    ProjectPreviewFrameStep direction,
  );
  Future<ProjectPreviewState> previewTick(ProjectSessionHandle session);
  Future<ProjectActionResult> setTimelineSequenceFrameRate(
    ProjectSessionHandle session,
    ProjectReadModel current,
    ProjectRationalRate? sequenceFrameRate,
  );
  Future<ProjectActionResult> rename(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String name,
  );
  Future<ProjectActionResult> undo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  );
  Future<ProjectActionResult> redo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  );
  Future<ProjectMediaPage> listMediaPage(
    ProjectSessionHandle session, {
    required int offset,
    required int limit,
  });
  Future<ProjectTimelineTracks> listTimelineTracks(
    ProjectSessionHandle session,
  );
  Future<ProjectTimelineClipPage> listTimelineClips(
    ProjectSessionHandle session, {
    required String trackId,
    required int offset,
    required int limit,
  });
  Future<ProjectTimelineMarkerPage> listTimelineMarkers(
    ProjectSessionHandle session, {
    required int offset,
    required int limit,
  });
  Future<ProjectTimelineSnapResult> resolveTimelineSnap(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required ProjectTimelineSnapOperation operation,
    required String clipId,
    String? targetTrackId,
    required ProjectRationalTime targetTime,
  });
  Future<ProjectActionResult> addTimelineTrack(
    ProjectSessionHandle session,
    ProjectReadModel current,
    ProjectTimelineTrackKind kind,
  );
  Future<ProjectActionResult> removeTimelineTrack(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String trackId,
  );
  Future<ProjectActionResult> insertTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String mediaId,
    required ProjectRationalTime timelineStart,
    required ProjectRationalTime sourceStart,
    required ProjectRationalTime duration,
  });
  Future<ProjectActionResult> moveTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String clipId,
    required String trackId,
    required ProjectRationalTime timelineStart,
  });
  Future<ProjectActionResult> deleteTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String clipId,
  );
  Future<ProjectActionResult> trimTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String clipId,
    required ProjectTimelineTrimEdge edge,
    required ProjectRationalTime timelineTime,
  });
  Future<ProjectActionResult> splitTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String clipId,
    required ProjectRationalTime timelineTime,
  });
  Future<ProjectActionResult> rippleDeleteTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String clipId,
  );
  Future<ProjectActionResult> addTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required ProjectRationalTime timelineTime,
    required String label,
  });
  Future<ProjectActionResult> moveTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String markerId,
    required ProjectRationalTime timelineTime,
  });
  Future<ProjectActionResult> renameTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String markerId,
    required String label,
  });
  Future<ProjectActionResult> deleteTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String markerId,
  );
  Future<ProjectMediaArtifactRequest> requestMediaThumbnail(
    ProjectSessionHandle session,
    String mediaId,
  );
  Future<ProjectMediaArtifactRequest> requestMediaWaveform(
    ProjectSessionHandle session,
    String mediaId,
  );
  Future<ProjectMediaArtifact?> readMediaArtifact(
    ProjectSessionHandle session, {
    required ProjectMediaArtifactKind kind,
    required String cacheKey,
  });
  Stream<ProjectMediaArtifactEvent> watchMediaArtifacts(
    ProjectSessionHandle session,
  );
  Future<ProjectActionResult> importMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String path,
  );
  Future<ProjectActionResult> removeMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String mediaId,
  );
  Future<ProjectActionResult> save(ProjectSessionHandle session);
  Future<void> close(
    ProjectSessionHandle session, {
    required bool discardUnsaved,
  });
  Stream<ProjectHostEvent> watch(ProjectSessionHandle session);
  Future<ProjectRecoveryInspection> inspectRecovery(String path);
  Future<ProjectRecoveryActionResult> applyRecovery(String path);
  Future<ProjectRecoveryActionResult> discardRecovery(String path);
}
