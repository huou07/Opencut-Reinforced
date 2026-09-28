import 'package:or_app_bridge/or_app_bridge.dart' as rust;

import 'project_gateway.dart';

class RustProjectGateway implements ProjectGateway {
  const RustProjectGateway();

  @override
  Future<ProjectSessionHandle> createProject(String path, String name) async {
    try {
      return _RustProjectSession(
        await rust.createProject(path: path, name: name),
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectSessionHandle> openProject(String path) async {
    try {
      return _RustProjectSession(await rust.openProject(path: path));
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectReadModel> summary(ProjectSessionHandle session) async {
    try {
      return _readModel(await _host(session).summary());
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectActionResult> rename(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String name,
  ) async => _action(
    await _host(session).rename(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      name: name,
    ),
  );

  @override
  Future<ProjectActionResult> undo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  ) async => _action(
    await _host(session).undo(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
    ),
  );

  @override
  Future<ProjectActionResult> redo(
    ProjectSessionHandle session,
    ProjectReadModel current,
  ) async => _action(
    await _host(session).redo(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
    ),
  );

  @override
  Future<ProjectMediaPage> listMediaPage(
    ProjectSessionHandle session, {
    required int offset,
    required int limit,
  }) async {
    try {
      final page = await _host(
        session,
      ).listMediaPage(offset: BigInt.from(offset), limit: BigInt.from(limit));
      return ProjectMediaPage(
        projectId: page.projectId,
        projectInstanceId: page.projectInstanceId,
        projectRevision: page.projectRevision,
        items: page.items
            .map(
              (item) => ProjectMediaItem(
                mediaId: item.mediaId,
                sourceUri: item.sourceUri,
                formatNames: List.unmodifiable(item.formatNames),
                duration: item.duration,
                videoDetails: item.videoDetails,
                audioDetails: item.audioDetails,
                containerDuration: item.containerDuration == null
                    ? null
                    : _projectRationalTime(item.containerDuration!),
                firstVideoDuration: item.firstVideoDuration == null
                    ? null
                    : _projectRationalTime(item.firstVideoDuration!),
                firstAudioDuration: item.firstAudioDuration == null
                    ? null
                    : _projectRationalTime(item.firstAudioDuration!),
              ),
            )
            .toList(growable: false),
        totalCount: page.totalCount.toInt(),
        offset: page.offset.toInt(),
        limit: page.limit.toInt(),
        nextOffset: page.nextOffset?.toInt(),
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectTimelineTracks> listTimelineTracks(
    ProjectSessionHandle session,
  ) async {
    try {
      final result = await _host(session).listTimelineTracks();
      return ProjectTimelineTracks(
        projectId: result.projectId,
        projectInstanceId: result.projectInstanceId,
        projectRevision: result.projectRevision,
        items: result.items
            .map(
              (track) => ProjectTimelineTrack(
                trackId: track.trackId,
                kind: _projectTimelineKind(track.kind),
                clipCount: track.clipCount.toInt(),
              ),
            )
            .toList(growable: false),
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectTimelineClipPage> listTimelineClips(
    ProjectSessionHandle session, {
    required String trackId,
    required int offset,
    required int limit,
  }) async {
    try {
      final page = await _host(session).listTimelineClips(
        trackId: trackId,
        offset: BigInt.from(offset),
        limit: BigInt.from(limit),
      );
      return ProjectTimelineClipPage(
        projectId: page.projectId,
        projectInstanceId: page.projectInstanceId,
        projectRevision: page.projectRevision,
        trackId: page.trackId,
        items: page.items
            .map(
              (clip) => ProjectTimelineClip(
                clipId: clip.clipId,
                mediaId: clip.mediaId,
                timelineStart: _projectRationalTime(clip.timelineStart),
                sourceStart: _projectRationalTime(clip.sourceStart),
                sourceDuration: _projectRationalTime(clip.sourceDuration),
              ),
            )
            .toList(growable: false),
        totalCount: page.totalCount.toInt(),
        offset: page.offset.toInt(),
        limit: page.limit.toInt(),
        nextOffset: page.nextOffset?.toInt(),
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectTimelineMarkerPage> listTimelineMarkers(
    ProjectSessionHandle session, {
    required int offset,
    required int limit,
  }) async {
    try {
      final page = await _host(session).listTimelineMarkers(
        offset: BigInt.from(offset),
        limit: BigInt.from(limit),
      );
      return ProjectTimelineMarkerPage(
        projectId: page.projectId,
        projectInstanceId: page.projectInstanceId,
        projectRevision: page.projectRevision,
        items: page.items
            .map(
              (marker) => ProjectTimelineMarker(
                markerId: marker.markerId,
                timelineTime: _projectRationalTime(marker.timelineTime),
                label: marker.label,
              ),
            )
            .toList(growable: false),
        totalCount: page.totalCount.toInt(),
        offset: page.offset.toInt(),
        limit: page.limit.toInt(),
        nextOffset: page.nextOffset?.toInt(),
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectTimelineSnapResult> resolveTimelineSnap(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required ProjectTimelineSnapOperation operation,
    required String clipId,
    String? targetTrackId,
    required ProjectRationalTime targetTime,
  }) async {
    try {
      final result = await _host(session).resolveTimelineSnap(
        projectId: current.projectId,
        projectInstanceId: current.projectInstanceId,
        expectedRevision: current.revision,
        operation: switch (operation) {
          ProjectTimelineSnapOperation.move =>
            rust.TimelineSnapOperationView.move,
          ProjectTimelineSnapOperation.trimStart =>
            rust.TimelineSnapOperationView.trimStart,
          ProjectTimelineSnapOperation.trimEnd =>
            rust.TimelineSnapOperationView.trimEnd,
        },
        clipId: clipId,
        targetTrackId: targetTrackId,
        targetTimeNumerator: targetTime.numerator.toInt(),
        targetTimeDenominator: targetTime.denominator,
      );
      return ProjectTimelineSnapResult(
        projectId: result.projectId,
        projectInstanceId: result.projectInstanceId,
        projectRevision: result.projectRevision,
        rawTargetTime: _projectRationalTime(result.rawTargetTime),
        resolvedTargetTime: _projectRationalTime(result.resolvedTargetTime),
        snapped: result.snapped,
        movingAnchor: switch (result.movingAnchor) {
          rust.TimelineSnapMovingAnchorView.none =>
            ProjectTimelineSnapMovingAnchor.none,
          rust.TimelineSnapMovingAnchorView.start =>
            ProjectTimelineSnapMovingAnchor.start,
          rust.TimelineSnapMovingAnchorView.end =>
            ProjectTimelineSnapMovingAnchor.end,
        },
        targetKind: switch (result.targetKind) {
          rust.TimelineSnapTargetKindView.none =>
            ProjectTimelineSnapTargetKind.none,
          rust.TimelineSnapTargetKindView.timelineZero =>
            ProjectTimelineSnapTargetKind.timelineZero,
          rust.TimelineSnapTargetKindView.clipStart =>
            ProjectTimelineSnapTargetKind.clipStart,
          rust.TimelineSnapTargetKindView.clipEnd =>
            ProjectTimelineSnapTargetKind.clipEnd,
          rust.TimelineSnapTargetKindView.marker =>
            ProjectTimelineSnapTargetKind.marker,
        },
        targetTime: _projectRationalTime(result.targetTime),
        targetTrackId: result.targetTrackId,
        targetClipId: result.targetClipId,
        targetMarkerId: result.targetMarkerId,
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectActionResult> addTimelineTrack(
    ProjectSessionHandle session,
    ProjectReadModel current,
    ProjectTimelineTrackKind kind,
  ) async => _action(
    await _host(session).addTimelineTrack(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      kind: _rustTimelineKind(kind),
    ),
  );

  @override
  Future<ProjectActionResult> removeTimelineTrack(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String trackId,
  ) async => _action(
    await _host(session).removeTimelineTrack(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      trackId: trackId,
    ),
  );

  @override
  Future<ProjectActionResult> insertTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String mediaId,
    required ProjectRationalTime timelineStart,
    required ProjectRationalTime sourceStart,
    required ProjectRationalTime duration,
  }) async => _action(
    await _host(session).insertTimelineClip(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      trackId: trackId,
      mediaId: mediaId,
      timelineStartNumerator: timelineStart.numerator.toInt(),
      timelineStartDenominator: timelineStart.denominator,
      sourceStartNumerator: sourceStart.numerator.toInt(),
      sourceStartDenominator: sourceStart.denominator,
      durationNumerator: duration.numerator.toInt(),
      durationDenominator: duration.denominator,
    ),
  );

  @override
  Future<ProjectActionResult> moveTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String clipId,
    required String trackId,
    required ProjectRationalTime timelineStart,
  }) async => _action(
    await _host(session).moveTimelineClip(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      clipId: clipId,
      trackId: trackId,
      timelineStartNumerator: timelineStart.numerator.toInt(),
      timelineStartDenominator: timelineStart.denominator,
    ),
  );

  @override
  Future<ProjectActionResult> deleteTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String clipId,
  ) async => _action(
    await _host(session).deleteTimelineClip(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      clipId: clipId,
    ),
  );

  @override
  Future<ProjectActionResult> trimTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String clipId,
    required ProjectTimelineTrimEdge edge,
    required ProjectRationalTime timelineTime,
  }) async => _action(
    await _host(session).trimTimelineClip(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      clipId: clipId,
      edge: switch (edge) {
        ProjectTimelineTrimEdge.start => rust.TimelineTrimEdgeView.start,
        ProjectTimelineTrimEdge.end => rust.TimelineTrimEdgeView.end,
      },
      timelineTimeNumerator: timelineTime.numerator.toInt(),
      timelineTimeDenominator: timelineTime.denominator,
    ),
  );

  @override
  Future<ProjectActionResult> splitTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String clipId,
    required ProjectRationalTime timelineTime,
  }) async => _action(
    await _host(session).splitTimelineClip(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      clipId: clipId,
      timelineTimeNumerator: timelineTime.numerator.toInt(),
      timelineTimeDenominator: timelineTime.denominator,
    ),
  );

  @override
  Future<ProjectActionResult> rippleDeleteTimelineClip(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String clipId,
  ) async => _action(
    await _host(session).rippleDeleteTimelineClip(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      clipId: clipId,
    ),
  );

  @override
  Future<ProjectActionResult> addTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required ProjectRationalTime timelineTime,
    required String label,
  }) async => _action(
    await _host(session).addTimelineMarker(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      timelineTimeNumerator: timelineTime.numerator.toInt(),
      timelineTimeDenominator: timelineTime.denominator,
      label: label,
    ),
  );

  @override
  Future<ProjectActionResult> moveTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String markerId,
    required ProjectRationalTime timelineTime,
  }) async => _action(
    await _host(session).moveTimelineMarker(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      markerId: markerId,
      timelineTimeNumerator: timelineTime.numerator.toInt(),
      timelineTimeDenominator: timelineTime.denominator,
    ),
  );

  @override
  Future<ProjectActionResult> renameTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String markerId,
    required String label,
  }) async => _action(
    await _host(session).renameTimelineMarker(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      markerId: markerId,
      label: label,
    ),
  );

  @override
  Future<ProjectActionResult> deleteTimelineMarker(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String markerId,
  ) async => _action(
    await _host(session).deleteTimelineMarker(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      markerId: markerId,
    ),
  );

  @override
  Future<ProjectMediaArtifactRequest> requestMediaThumbnail(
    ProjectSessionHandle session,
    String mediaId,
  ) async => _artifactRequest(
    await _host(session).requestMediaThumbnail(mediaId: mediaId),
  );

  @override
  Future<ProjectMediaArtifactRequest> requestMediaWaveform(
    ProjectSessionHandle session,
    String mediaId,
  ) async => _artifactRequest(
    await _host(session).requestMediaWaveform(mediaId: mediaId),
  );

  @override
  Future<ProjectMediaArtifact?> readMediaArtifact(
    ProjectSessionHandle session, {
    required ProjectMediaArtifactKind kind,
    required String cacheKey,
  }) async {
    try {
      final artifact = await _host(session)
          .readMediaArtifact(kind: _rustArtifactKind(kind), cacheKey: cacheKey);
      if (artifact == null) return null;
      return ProjectMediaArtifact(
        bytes: artifact.bytes,
        mimeType: artifact.mimeType,
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Stream<ProjectMediaArtifactEvent> watchMediaArtifacts(
    ProjectSessionHandle session,
  ) => _host(session).subscribeMediaArtifactEvents().map(
    (event) => ProjectMediaArtifactEvent(
      sequence: event.sequence,
      mediaId: event.mediaId,
      kind: _artifactKind(event.kind),
      cacheKey: event.cacheKey,
      jobId: event.jobId,
      state: switch (event.state) {
        rust.MediaArtifactEventStateView.succeeded =>
          ProjectMediaArtifactEventState.succeeded,
        rust.MediaArtifactEventStateView.failed =>
          ProjectMediaArtifactEventState.failed,
        rust.MediaArtifactEventStateView.cancelled =>
          ProjectMediaArtifactEventState.cancelled,
      },
      errorCode: event.errorCode,
    ),
  );

  @override
  Future<ProjectActionResult> importMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String path,
  ) async => _action(
    await _host(session).importMedia(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      path: path,
    ),
  );

  @override
  Future<ProjectActionResult> removeMedia(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String mediaId,
  ) async => _action(
    await _host(session).removeMedia(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      mediaId: mediaId,
    ),
  );

  @override
  Future<ProjectActionResult> save(ProjectSessionHandle session) async =>
      _action(await _host(session).save());

  @override
  Future<void> close(
    ProjectSessionHandle session, {
    required bool discardUnsaved,
  }) async {
    final result = await _host(session).close(discardUnsaved: discardUnsaved);
    if (!result.succeeded) {
      throw ProjectGatewayException(result.errorCode, result.message);
    }
  }

  @override
  Stream<ProjectHostEvent> watch(ProjectSessionHandle session) =>
      _host(session).subscribeEvents().map(
        (event) => ProjectHostEvent(
          sequence: event.sequence,
          kind: event.kind,
          projectId: event.projectId,
          projectInstanceId: event.projectInstanceId,
          revision: event.revision,
          dirty: event.dirty,
        ),
      );

  @override
  Future<ProjectRecoveryInspection> inspectRecovery(String path) async {
    final inspection = await rust.inspectRecovery(path: path);
    return ProjectRecoveryInspection(
      kind: switch (inspection.status) {
        'candidate' => ProjectRecoveryKind.candidate,
        'stale' => ProjectRecoveryKind.stale,
        'conflict' => ProjectRecoveryKind.conflict,
        'invalid' => ProjectRecoveryKind.invalid,
        _ => ProjectRecoveryKind.none,
      },
      projectId: inspection.projectId,
      baseRevision: inspection.baseRevision,
      recoveryRevision: inspection.recoveryRevision,
      recoveryName: inspection.recoveryName,
      conflictReason: inspection.conflictReason,
      message: inspection.message,
    );
  }

  @override
  Future<ProjectRecoveryActionResult> applyRecovery(String path) async =>
      _recoveryAction(await rust.applyRecovery(path: path));

  @override
  Future<ProjectRecoveryActionResult> discardRecovery(String path) async =>
      _recoveryAction(await rust.discardRecovery(path: path));

  static rust.ProjectHostHandle _host(ProjectSessionHandle session) {
    if (session is _RustProjectSession) return session.host;
    throw ArgumentError.value(session, 'session', 'belongs to another gateway');
  }

  static ProjectReadModel _readModel(rust.ProjectView view) => ProjectReadModel(
    projectId: view.projectId,
    projectInstanceId: view.projectInstanceId,
    revision: view.revision,
    name: view.name,
    dirty: view.dirty,
    descriptorPath: view.descriptorPath,
  );

  static ProjectActionResult _action(rust.ProjectActionResult result) =>
      ProjectActionResult(
        succeeded: result.succeeded,
        errorCode: result.errorCode,
        message: result.message,
        view: result.view == null ? null : _readModel(result.view!),
      );

  static ProjectRecoveryActionResult _recoveryAction(
    rust.RecoveryActionResult result,
  ) => ProjectRecoveryActionResult(
    succeeded: result.succeeded,
    changed: result.changed,
    message: result.message,
  );

  static ProjectMediaArtifactKind _artifactKind(
    rust.MediaArtifactKindView kind,
  ) => switch (kind) {
    rust.MediaArtifactKindView.thumbnail => ProjectMediaArtifactKind.thumbnail,
    rust.MediaArtifactKindView.waveform => ProjectMediaArtifactKind.waveform,
  };

  static rust.MediaArtifactKindView _rustArtifactKind(
    ProjectMediaArtifactKind kind,
  ) => switch (kind) {
    ProjectMediaArtifactKind.thumbnail => rust.MediaArtifactKindView.thumbnail,
    ProjectMediaArtifactKind.waveform => rust.MediaArtifactKindView.waveform,
  };

  static ProjectMediaArtifactRequest _artifactRequest(
    rust.MediaArtifactRequestView request,
  ) => ProjectMediaArtifactRequest(
    mediaId: request.mediaId,
    kind: _artifactKind(request.kind),
    cacheKey: request.cacheKey,
    jobId: request.jobId,
    state: switch (request.state) {
      rust.MediaArtifactRequestStateView.ready =>
        ProjectMediaArtifactRequestState.ready,
      rust.MediaArtifactRequestStateView.queued =>
        ProjectMediaArtifactRequestState.queued,
      rust.MediaArtifactRequestStateView.running =>
        ProjectMediaArtifactRequestState.running,
      rust.MediaArtifactRequestStateView.notApplicable =>
        ProjectMediaArtifactRequestState.notApplicable,
      rust.MediaArtifactRequestStateView.failed =>
        ProjectMediaArtifactRequestState.failed,
    },
    errorCode: request.errorCode,
    message: request.message,
  );

  static ProjectRationalTime _projectRationalTime(
    rust.RationalTimeView value,
  ) => ProjectRationalTime(BigInt.from(value.numerator), value.denominator);

  static ProjectTimelineTrackKind _projectTimelineKind(
    rust.TimelineTrackKindView kind,
  ) => switch (kind) {
    rust.TimelineTrackKindView.video => ProjectTimelineTrackKind.video,
    rust.TimelineTrackKindView.audio => ProjectTimelineTrackKind.audio,
  };

  static rust.TimelineTrackKindView _rustTimelineKind(
    ProjectTimelineTrackKind kind,
  ) => switch (kind) {
    ProjectTimelineTrackKind.video => rust.TimelineTrackKindView.video,
    ProjectTimelineTrackKind.audio => rust.TimelineTrackKindView.audio,
  };
}

class _RustProjectSession implements ProjectSessionHandle {
  const _RustProjectSession(this.host);

  final rust.ProjectHostHandle host;
}
