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

class ProjectExportJob {
  const ProjectExportJob({
    required this.succeeded,
    required this.errorCode,
    required this.message,
    required this.jobId,
    required this.state,
    this.progressCompleted,
    this.progressTotal,
    this.failureMessage,
  });

  final bool succeeded;
  final String errorCode;
  final String message;
  final String jobId;
  final String state;
  final BigInt? progressCompleted;
  final BigInt? progressTotal;
  final String? failureMessage;

  bool get isActive => state == 'queued' || state == 'running';
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

enum ProjectTimelineTrackKind { video, audio, text, caption }

enum ProjectTimelineClipContentKind { media, text, caption }

enum ProjectFontIdentity { bundledInter }

enum ProjectTextWeight { regular, medium, semibold, bold }

enum ProjectTextAlignment { start, center, end }

class ProjectTextColor {
  const ProjectTextColor({
    required this.red,
    required this.green,
    required this.blue,
    required this.alpha,
  });

  static const white = ProjectTextColor(
    red: 255,
    green: 255,
    blue: 255,
    alpha: 255,
  );

  final int red;
  final int green;
  final int blue;
  final int alpha;
}

class ProjectTextFormatting {
  const ProjectTextFormatting({
    required this.font,
    required this.sizeMilliPoints,
    required this.weight,
    required this.alignment,
    required this.color,
  });

  static const defaults = ProjectTextFormatting(
    font: ProjectFontIdentity.bundledInter,
    sizeMilliPoints: 48000,
    weight: ProjectTextWeight.regular,
    alignment: ProjectTextAlignment.center,
    color: ProjectTextColor.white,
  );

  final ProjectFontIdentity font;
  final int sizeMilliPoints;
  final ProjectTextWeight weight;
  final ProjectTextAlignment alignment;
  final ProjectTextColor color;
}

class ProjectTimelineTextContent {
  const ProjectTimelineTextContent({
    required this.kind,
    required this.text,
    required this.formatting,
  });

  final ProjectTimelineClipContentKind kind;
  final String text;
  final ProjectTextFormatting formatting;
}

class ProjectTimelineTrackState {
  const ProjectTimelineTrackState({
    required this.locked,
    required this.visible,
    required this.muted,
    required this.solo,
  });

  static const defaults = ProjectTimelineTrackState(
    locked: false,
    visible: true,
    muted: false,
    solo: false,
  );

  final bool locked;
  final bool visible;
  final bool muted;
  final bool solo;

  ProjectTimelineTrackState copyWith({
    bool? locked,
    bool? visible,
    bool? muted,
    bool? solo,
  }) => ProjectTimelineTrackState(
    locked: locked ?? this.locked,
    visible: visible ?? this.visible,
    muted: muted ?? this.muted,
    solo: solo ?? this.solo,
  );
}

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
    this.state = ProjectTimelineTrackState.defaults,
  });

  final String trackId;
  final ProjectTimelineTrackKind kind;
  final int clipCount;
  final ProjectTimelineTrackState state;
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
    this.contentKind = ProjectTimelineClipContentKind.media,
    this.mediaId,
    this.text,
    this.formatting,
    required this.timelineStart,
    required this.timelineDuration,
    this.sourceStart,
  });

  final String clipId;
  final ProjectTimelineClipContentKind contentKind;
  final String? mediaId;
  final String? text;
  final ProjectTextFormatting? formatting;
  final ProjectRationalTime timelineStart;
  final ProjectRationalTime timelineDuration;
  final ProjectRationalTime? sourceStart;

  ProjectRationalTime get timelineEnd => timelineStart.add(timelineDuration);
}

class ProjectTimelineVisualSettings {
  const ProjectTimelineVisualSettings({
    required this.xMilliCanvas,
    required this.yMilliCanvas,
    required this.scaleXMilli,
    required this.scaleYMilli,
    required this.rotationMilliDegrees,
    required this.anchorXBasisPoints,
    required this.anchorYBasisPoints,
    required this.cropLeftBasisPoints,
    required this.cropTopBasisPoints,
    required this.cropRightBasisPoints,
    required this.cropBottomBasisPoints,
    required this.opacityBasisPoints,
    this.brightnessAmountMilli = 0,
    this.contrastAmountMilli = 1000,
    this.saturationAmountMilli = 1000,
    this.gaussianBlurRadiusMilli = 0,
    this.transitionIn = ProjectTimelineTransition.none,
    this.transitionInDuration,
    this.transitionOut = ProjectTimelineTransition.none,
    this.transitionOutDuration,
    this.modifiedEffects = const {},
    this.updateTransitionIn = false,
    this.updateTransitionOut = false,
  });

  static const identity = ProjectTimelineVisualSettings(
    xMilliCanvas: 0,
    yMilliCanvas: 0,
    scaleXMilli: 1000,
    scaleYMilli: 1000,
    rotationMilliDegrees: 0,
    anchorXBasisPoints: 5000,
    anchorYBasisPoints: 5000,
    cropLeftBasisPoints: 0,
    cropTopBasisPoints: 0,
    cropRightBasisPoints: 0,
    cropBottomBasisPoints: 0,
    opacityBasisPoints: 10000,
    brightnessAmountMilli: 0,
    contrastAmountMilli: 1000,
    saturationAmountMilli: 1000,
    gaussianBlurRadiusMilli: 0,
    transitionIn: ProjectTimelineTransition.none,
    transitionOut: ProjectTimelineTransition.none,
    modifiedEffects: {
      ProjectTimelineEffectKind.brightness,
      ProjectTimelineEffectKind.contrast,
      ProjectTimelineEffectKind.saturation,
      ProjectTimelineEffectKind.gaussianBlur,
    },
    updateTransitionIn: true,
    updateTransitionOut: true,
  );

  final int xMilliCanvas;
  final int yMilliCanvas;
  final int scaleXMilli;
  final int scaleYMilli;
  final int rotationMilliDegrees;
  final int anchorXBasisPoints;
  final int anchorYBasisPoints;
  final int cropLeftBasisPoints;
  final int cropTopBasisPoints;
  final int cropRightBasisPoints;
  final int cropBottomBasisPoints;
  final int opacityBasisPoints;
  final int brightnessAmountMilli;
  final int contrastAmountMilli;
  final int saturationAmountMilli;
  final int gaussianBlurRadiusMilli;
  final ProjectTimelineTransition transitionIn;
  final ProjectRationalTime? transitionInDuration;
  final ProjectTimelineTransition transitionOut;
  final ProjectRationalTime? transitionOutDuration;
  final Set<ProjectTimelineEffectKind> modifiedEffects;
  final bool updateTransitionIn;
  final bool updateTransitionOut;
}

enum ProjectTimelineTransition { none, crossDissolve, fadeThroughBlack, wipe }

enum ProjectTimelineEffectKind {
  brightness,
  contrast,
  saturation,
  gaussianBlur,
}

class ProjectTimelineAudioSettings {
  const ProjectTimelineAudioSettings({
    required this.gainMilliDecibels,
    required this.panBasisPoints,
    required this.fadeIn,
    required this.fadeOut,
  });

  static final identity = ProjectTimelineAudioSettings(
    gainMilliDecibels: 0,
    panBasisPoints: 0,
    fadeIn: ProjectRationalTime(BigInt.zero, 1),
    fadeOut: ProjectRationalTime(BigInt.zero, 1),
  );

  final int gainMilliDecibels;
  final int panBasisPoints;
  final ProjectRationalTime fadeIn;
  final ProjectRationalTime fadeOut;
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
  Future<ProjectTimelineVisualSettings> getTimelineClipVisualSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
  });
  Future<ProjectActionResult> updateTimelineClipVisualSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectTimelineVisualSettings settings,
  });
  Future<ProjectTimelineAudioSettings> getTimelineClipAudioSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
  });
  Future<ProjectActionResult> updateTimelineClipAudioSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectTimelineAudioSettings settings,
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
  Future<ProjectActionResult> setTimelineTrackState(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required ProjectTimelineTrackState state,
  });
  Future<ProjectActionResult> insertTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String mediaId,
    required ProjectRationalTime timelineStart,
    required ProjectRationalTime sourceStart,
    required ProjectRationalTime duration,
  });
  Future<ProjectActionResult> insertTimelineTextClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required ProjectRationalTime timelineStart,
    required ProjectRationalTime timelineDuration,
    required ProjectTimelineTextContent content,
  });
  Future<ProjectActionResult> updateTimelineTextClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectRationalTime timelineDuration,
    required ProjectTimelineTextContent content,
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
  Future<ProjectActionResult> autosaveCheckpoint(ProjectSessionHandle session);
  Future<ProjectExportJob> startExport(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String destination,
  );
  Future<ProjectExportJob> exportStatus(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String jobId,
  );
  Future<ProjectExportJob> cancelExport(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String jobId,
  );
  Future<void> close(
    ProjectSessionHandle session, {
    required bool discardUnsaved,
  });
  Stream<ProjectHostEvent> watch(ProjectSessionHandle session);
  Future<ProjectRecoveryInspection> inspectRecovery(String path);
  Future<ProjectRecoveryActionResult> applyRecovery(String path);
  Future<ProjectRecoveryActionResult> discardRecovery(String path);
}
