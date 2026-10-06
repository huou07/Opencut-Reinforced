import 'dart:async';
import 'dart:io';

import 'package:flutter/services.dart';

import 'package:or_app_bridge/or_app_bridge.dart' as rust;
import 'package:or_viewer_texture/or_viewer_texture.dart';

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
  Future<ProjectPreviewState> previewState(ProjectSessionHandle session) async {
    try {
      return _preview(await _host(session).previewState());
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectPreviewState> previewSeek(
    ProjectSessionHandle session,
    ProjectRationalTime position,
  ) async {
    try {
      if (Platform.isAndroid) {
        return await _androidPreview(
          session,
          rust.PreviewPreparationActionView.seek,
          position: _rustTime(position),
        );
      }
      return _preview(
        await _host(session).previewSeek(position: _rustTime(position)),
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  Future<ProjectPreviewState> _androidPreview(
    ProjectSessionHandle session,
    rust.PreviewPreparationActionView action, {
    rust.RationalTimeView? position,
  }) async {
    final host = _host(session);
    final prepared = await host.previewPrepare(
      action: action,
      position: position,
    );
    final requestId = prepared.requestId;
    if (requestId == null) return _preview(prepared.state);
    try {
      final registered = await OrViewerTexture.setMediaSources(
        prepared.sources,
        generation: requestId,
        owner: (await host.summary()).projectInstanceId,
      );
      if (!registered) {
        throw const ProjectGatewayException(
          'MEDIA_SOURCE_REGISTRATION_FAILED',
          'An Android media source could not be registered. Select it again and retry preview.',
        );
      }
      return _preview(await host.previewCompletePrepared(requestId: requestId));
    } catch (error) {
      // A concurrently closed host has already cancelled and released its work.
      try {
        await host.previewAbortPrepared(requestId: requestId);
      } on rust.ProjectBridgeError catch (abortError) {
        if (abortError.code != 'PROJECT_CLOSED') rethrow;
      }
      if (error is PlatformException) {
        throw ProjectGatewayException(
          error.code,
          error.message ??
              'Android media access is unavailable. Select the source again.',
        );
      }
      if (error is MissingPluginException) {
        throw const ProjectGatewayException(
          'MEDIA_SOURCE_UNAVAILABLE',
          'Android media access is unavailable. Restart the application.',
        );
      }
      rethrow;
    }
  }

  @override
  Future<ProjectPreviewState> previewPlay(ProjectSessionHandle session) async {
    try {
      if (Platform.isAndroid) {
        return await _androidPreview(
          session,
          rust.PreviewPreparationActionView.play,
        );
      }
      return _preview(await _host(session).previewPlay());
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectPreviewState> previewPause(ProjectSessionHandle session) async {
    try {
      return _preview(await _host(session).previewPause());
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectPreviewState> previewStep(
    ProjectSessionHandle session,
    ProjectPreviewFrameStep direction,
  ) async {
    try {
      if (Platform.isAndroid) {
        return await _androidPreview(
          session,
          direction == ProjectPreviewFrameStep.previous
              ? rust.PreviewPreparationActionView.stepPrevious
              : rust.PreviewPreparationActionView.stepNext,
        );
      }
      return _preview(
        await _host(session).previewStep(
          direction: switch (direction) {
            ProjectPreviewFrameStep.previous =>
              rust.PreviewFrameStepView.previous,
            ProjectPreviewFrameStep.next => rust.PreviewFrameStepView.next,
          },
        ),
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectPreviewState> previewTick(ProjectSessionHandle session) async {
    try {
      if (Platform.isAndroid) {
        return await _androidPreview(
          session,
          rust.PreviewPreparationActionView.tick,
        );
      }
      return _preview(await _host(session).previewTick());
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectActionResult> setTimelineSequenceFrameRate(
    ProjectSessionHandle session,
    ProjectReadModel current,
    ProjectRationalRate? sequenceFrameRate,
  ) async => _action(
    await _host(session).setTimelineSequenceFrameRate(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      sequenceFrameRate: sequenceFrameRate == null
          ? null
          : rust.RationalRateView(
              numerator: sequenceFrameRate.numerator,
              denominator: sequenceFrameRate.denominator,
            ),
    ),
  );

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
                state: ProjectTimelineTrackState(
                  locked: track.state.locked,
                  visible: track.state.visible,
                  muted: track.state.muted,
                  solo: track.state.solo,
                ),
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
                contentKind: switch (clip.contentKind) {
                  rust.TimelineClipContentKindView.media =>
                    ProjectTimelineClipContentKind.media,
                  rust.TimelineClipContentKindView.text =>
                    ProjectTimelineClipContentKind.text,
                  rust.TimelineClipContentKindView.caption =>
                    ProjectTimelineClipContentKind.caption,
                },
                mediaId: clip.mediaId,
                text: clip.text,
                formatting: clip.formatting == null
                    ? null
                    : _projectTextFormatting(clip.formatting!),
                timelineStart: _projectRationalTime(clip.timelineStart),
                timelineDuration: _projectRationalTime(clip.timelineDuration),
                sourceStart: clip.sourceStart == null
                    ? null
                    : _projectRationalTime(clip.sourceStart!),
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
  Future<ProjectTimelineVisualSettings> getTimelineClipVisualSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
  }) async {
    try {
      final settings = await _host(session).getTimelineClipVisualSettings(
        projectId: current.projectId,
        projectInstanceId: current.projectInstanceId,
        expectedRevision: current.revision,
        trackId: trackId,
        clipId: clipId,
      );
      return ProjectTimelineVisualSettings(
        xMilliCanvas: settings.xMilliCanvas,
        yMilliCanvas: settings.yMilliCanvas,
        scaleXMilli: settings.scaleXMilli,
        scaleYMilli: settings.scaleYMilli,
        rotationMilliDegrees: settings.rotationMilliDegrees,
        anchorXBasisPoints: settings.anchorXBasisPoints,
        anchorYBasisPoints: settings.anchorYBasisPoints,
        cropLeftBasisPoints: settings.cropLeftBasisPoints,
        cropTopBasisPoints: settings.cropTopBasisPoints,
        cropRightBasisPoints: settings.cropRightBasisPoints,
        cropBottomBasisPoints: settings.cropBottomBasisPoints,
        opacityBasisPoints: settings.opacityBasisPoints,
        brightnessAmountMilli: settings.brightnessAmountMilli,
        contrastAmountMilli: settings.contrastAmountMilli,
        saturationAmountMilli: settings.saturationAmountMilli,
        gaussianBlurRadiusMilli: settings.gaussianBlurRadiusMilli,
        transitionIn: _projectTransition(settings.transitionInKind),
        transitionInDuration: _projectRationalTime(
          settings.transitionInDuration,
        ),
        transitionOut: _projectTransition(settings.transitionOutKind),
        transitionOutDuration: _projectRationalTime(
          settings.transitionOutDuration,
        ),
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectActionResult> updateTimelineClipVisualSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectTimelineVisualSettings settings,
  }) async => _action(
    await _host(session).updateTimelineClipVisualSettings(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      trackId: trackId,
      clipId: clipId,
      settings: rust.ProjectTimelineVisualSettingsView(
        xMilliCanvas: settings.xMilliCanvas,
        yMilliCanvas: settings.yMilliCanvas,
        scaleXMilli: settings.scaleXMilli,
        scaleYMilli: settings.scaleYMilli,
        rotationMilliDegrees: settings.rotationMilliDegrees,
        anchorXBasisPoints: settings.anchorXBasisPoints,
        anchorYBasisPoints: settings.anchorYBasisPoints,
        cropLeftBasisPoints: settings.cropLeftBasisPoints,
        cropTopBasisPoints: settings.cropTopBasisPoints,
        cropRightBasisPoints: settings.cropRightBasisPoints,
        cropBottomBasisPoints: settings.cropBottomBasisPoints,
        opacityBasisPoints: settings.opacityBasisPoints,
        brightnessAmountMilli: settings.brightnessAmountMilli,
        contrastAmountMilli: settings.contrastAmountMilli,
        saturationAmountMilli: settings.saturationAmountMilli,
        gaussianBlurRadiusMilli: settings.gaussianBlurRadiusMilli,
        updateBrightness: settings.modifiedEffects.contains(
          ProjectTimelineEffectKind.brightness,
        ),
        updateContrast: settings.modifiedEffects.contains(
          ProjectTimelineEffectKind.contrast,
        ),
        updateSaturation: settings.modifiedEffects.contains(
          ProjectTimelineEffectKind.saturation,
        ),
        updateGaussianBlur: settings.modifiedEffects.contains(
          ProjectTimelineEffectKind.gaussianBlur,
        ),
        transitionInKind: _rustTransition(settings.transitionIn),
        transitionInDuration: _rustTime(
          settings.transitionInDuration ?? ProjectRationalTime(BigInt.zero, 1),
        ),
        updateTransitionIn: settings.updateTransitionIn,
        transitionOutKind: _rustTransition(settings.transitionOut),
        transitionOutDuration: _rustTime(
          settings.transitionOutDuration ?? ProjectRationalTime(BigInt.zero, 1),
        ),
        updateTransitionOut: settings.updateTransitionOut,
      ),
    ),
  );

  @override
  Future<ProjectTimelineAudioSettings> getTimelineClipAudioSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
  }) async {
    try {
      final settings = await _host(session).getTimelineClipAudioSettings(
        projectId: current.projectId,
        projectInstanceId: current.projectInstanceId,
        expectedRevision: current.revision,
        trackId: trackId,
        clipId: clipId,
      );
      return ProjectTimelineAudioSettings(
        gainMilliDecibels: settings.gainMillidecibels,
        panBasisPoints: settings.panBasisPoints,
        fadeIn: _projectRationalTime(settings.fadeIn),
        fadeOut: _projectRationalTime(settings.fadeOut),
      );
    } on rust.ProjectBridgeError catch (error) {
      throw ProjectGatewayException(error.code, error.message);
    }
  }

  @override
  Future<ProjectActionResult> updateTimelineClipAudioSettings(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectTimelineAudioSettings settings,
  }) async => _action(
    await _host(session).updateTimelineClipAudioSettings(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      trackId: trackId,
      clipId: clipId,
      settings: rust.ProjectTimelineAudioSettingsView(
        gainMillidecibels: settings.gainMilliDecibels,
        panBasisPoints: settings.panBasisPoints,
        fadeIn: _rustTime(settings.fadeIn),
        fadeOut: _rustTime(settings.fadeOut),
      ),
    ),
  );

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
  Future<ProjectActionResult> setTimelineTrackState(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required ProjectTimelineTrackState state,
  }) async => _action(
    await _host(session).setTimelineTrackState(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      trackId: trackId,
      state: rust.ProjectTimelineTrackStateView(
        locked: state.locked,
        visible: state.visible,
        muted: state.muted,
        solo: state.solo,
      ),
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
  Future<ProjectActionResult> insertTimelineTextClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required ProjectRationalTime timelineStart,
    required ProjectRationalTime timelineDuration,
    required ProjectTimelineTextContent content,
  }) async => _action(
    await _host(session).insertTimelineTextClip(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      trackId: trackId,
      contentKind: _rustClipContentKind(content.kind),
      timelineStartNumerator: timelineStart.numerator.toInt(),
      timelineStartDenominator: timelineStart.denominator,
      timelineDurationNumerator: timelineDuration.numerator.toInt(),
      timelineDurationDenominator: timelineDuration.denominator,
      text: content.text,
      formatting: _rustTextFormatting(content.formatting),
    ),
  );

  @override
  Future<ProjectActionResult> updateTimelineTextClip(
    ProjectSessionHandle session,
    ProjectReadModel current, {
    required String trackId,
    required String clipId,
    required ProjectRationalTime timelineDuration,
    required ProjectTimelineTextContent content,
  }) async => _action(
    await _host(session).updateTimelineTextClip(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      trackId: trackId,
      clipId: clipId,
      contentKind: _rustClipContentKind(content.kind),
      timelineDurationNumerator: timelineDuration.numerator.toInt(),
      timelineDurationDenominator: timelineDuration.denominator,
      text: content.text,
      formatting: _rustTextFormatting(content.formatting),
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
  ) =>
      _host(session)
          .subscribeMediaArtifactEvents()
          .map(_mediaArtifactEvent)
          .transform(_translating<ProjectMediaArtifactEvent>());

  static ProjectMediaArtifactEvent _mediaArtifactEvent(
    rust.MediaArtifactEventView event,
  ) => ProjectMediaArtifactEvent(
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
  );

  /// A raw bridge error must never reach the shell, which handles the typed
  /// gateway exception only.
  static StreamTransformer<T, T> _translating<T>() =>
      StreamTransformer<T, T>.fromHandlers(
        handleError: (error, stackTrace, sink) => sink.addError(
          error is rust.ProjectBridgeError
              ? ProjectGatewayException(error.code, error.message)
              : error,
          stackTrace,
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
  Future<ProjectActionResult> autosaveCheckpoint(
    ProjectSessionHandle session,
  ) async => _action(await _host(session).autosaveCheckpoint());

  @override
  Future<ProjectExportJob> startExport(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String destination,
  ) async => _exportJob(
    await _host(session).startExport(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      expectedRevision: current.revision,
      destination: destination,
    ),
  );

  @override
  Future<ProjectExportJob> exportStatus(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String jobId,
  ) async => _exportJob(
    await _host(session).exportStatus(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      jobId: jobId,
    ),
  );

  @override
  Future<ProjectExportJob> cancelExport(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String jobId,
  ) async => _exportJob(
    await _host(session).cancelExport(
      projectId: current.projectId,
      projectInstanceId: current.projectInstanceId,
      jobId: jobId,
    ),
  );

  @override
  Future<void> close(
    ProjectSessionHandle session, {
    required bool discardUnsaved,
  }) async {
    final result = await _host(session).close(discardUnsaved: discardUnsaved);
    if (!result.succeeded) {
      throw ProjectGatewayException(result.errorCode, result.message);
    }
    if (Platform.isAndroid) {
      try {
        await OrViewerTexture.clearMediaSources();
      } on PlatformException catch (error) {
        throw ProjectGatewayException(
          error.code,
          error.message ?? 'Android media descriptors could not be released.',
        );
      }
    }
  }

  @override
  Stream<ProjectHostEvent> watch(ProjectSessionHandle session) =>
      _host(session)
          .subscribeEvents()
          .map(
            (event) => ProjectHostEvent(
              sequence: event.sequence,
              kind: event.kind,
              projectId: event.projectId,
              projectInstanceId: event.projectInstanceId,
              revision: event.revision,
              dirty: event.dirty,
            ),
          )
          .transform(_translating<ProjectHostEvent>());

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

  static ProjectExportJob _exportJob(rust.ProjectExportJobView value) =>
      ProjectExportJob(
        succeeded: value.succeeded,
        errorCode: value.errorCode,
        message: value.message,
        jobId: value.jobId,
        state: value.state,
        progressCompleted: value.progressCompleted,
        progressTotal: value.progressTotal,
        failureMessage: value.failureMessage,
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

  static ProjectTextFormatting _projectTextFormatting(
    rust.ProjectTextFormattingView value,
  ) => ProjectTextFormatting(
    font: switch (value.font) {
      rust.FontIdentityView.bundledInter => ProjectFontIdentity.bundledInter,
    },
    sizeMilliPoints: value.sizeMilliPoints,
    weight: switch (value.weight) {
      rust.TextWeightView.regular => ProjectTextWeight.regular,
      rust.TextWeightView.medium => ProjectTextWeight.medium,
      rust.TextWeightView.semibold => ProjectTextWeight.semibold,
      rust.TextWeightView.bold => ProjectTextWeight.bold,
    },
    alignment: switch (value.alignment) {
      rust.TextAlignmentView.start => ProjectTextAlignment.start,
      rust.TextAlignmentView.center => ProjectTextAlignment.center,
      rust.TextAlignmentView.end => ProjectTextAlignment.end,
    },
    color: ProjectTextColor(
      red: value.color.red,
      green: value.color.green,
      blue: value.color.blue,
      alpha: value.color.alpha,
    ),
  );

  static rust.ProjectTextFormattingView _rustTextFormatting(
    ProjectTextFormatting value,
  ) => rust.ProjectTextFormattingView(
    font: switch (value.font) {
      ProjectFontIdentity.bundledInter => rust.FontIdentityView.bundledInter,
    },
    sizeMilliPoints: value.sizeMilliPoints,
    weight: switch (value.weight) {
      ProjectTextWeight.regular => rust.TextWeightView.regular,
      ProjectTextWeight.medium => rust.TextWeightView.medium,
      ProjectTextWeight.semibold => rust.TextWeightView.semibold,
      ProjectTextWeight.bold => rust.TextWeightView.bold,
    },
    alignment: switch (value.alignment) {
      ProjectTextAlignment.start => rust.TextAlignmentView.start,
      ProjectTextAlignment.center => rust.TextAlignmentView.center,
      ProjectTextAlignment.end => rust.TextAlignmentView.end,
    },
    color: rust.ProjectTextColorView(
      red: value.color.red,
      green: value.color.green,
      blue: value.color.blue,
      alpha: value.color.alpha,
    ),
  );

  static rust.TimelineClipContentKindView _rustClipContentKind(
    ProjectTimelineClipContentKind kind,
  ) => switch (kind) {
    ProjectTimelineClipContentKind.media =>
      rust.TimelineClipContentKindView.media,
    ProjectTimelineClipContentKind.text =>
      rust.TimelineClipContentKindView.text,
    ProjectTimelineClipContentKind.caption =>
      rust.TimelineClipContentKindView.caption,
  };

  static ProjectPreviewState _preview(rust.ProjectPreviewStateView value) =>
      ProjectPreviewState(
        position: _projectRationalTime(value.position),
        presentedTime: value.presentedTime == null
            ? null
            : _projectRationalTime(value.presentedTime!),
        sequenceFrameRate: value.sequenceFrameRate == null
            ? null
            : ProjectRationalRate(
                value.sequenceFrameRate!.numerator,
                value.sequenceFrameRate!.denominator,
              ),
        contentEnd: value.contentEnd == null
            ? null
            : _projectRationalTime(value.contentEnd!),
        playing: value.playing,
        generation: value.generation,
        frameSequence: value.frameSequence,
        width: value.width,
        height: value.height,
        errorCode: value.errorCode,
        errorMessage: value.errorMessage,
      );

  static rust.RationalTimeView _rustTime(ProjectRationalTime value) =>
      rust.RationalTimeView(
        numerator: value.numerator.toInt(),
        denominator: value.denominator,
      );

  static ProjectTimelineTransition _projectTransition(int value) =>
      switch (value) {
        1 => ProjectTimelineTransition.crossDissolve,
        2 => ProjectTimelineTransition.fadeThroughBlack,
        3 => ProjectTimelineTransition.wipe,
        _ => ProjectTimelineTransition.none,
      };

  static int _rustTransition(ProjectTimelineTransition value) =>
      switch (value) {
        ProjectTimelineTransition.none => 0,
        ProjectTimelineTransition.crossDissolve => 1,
        ProjectTimelineTransition.fadeThroughBlack => 2,
        ProjectTimelineTransition.wipe => 3,
      };

  static ProjectTimelineTrackKind _projectTimelineKind(
    rust.TimelineTrackKindView kind,
  ) => switch (kind) {
    rust.TimelineTrackKindView.video => ProjectTimelineTrackKind.video,
    rust.TimelineTrackKindView.audio => ProjectTimelineTrackKind.audio,
    rust.TimelineTrackKindView.text => ProjectTimelineTrackKind.text,
    rust.TimelineTrackKindView.caption => ProjectTimelineTrackKind.caption,
  };

  static rust.TimelineTrackKindView _rustTimelineKind(
    ProjectTimelineTrackKind kind,
  ) => switch (kind) {
    ProjectTimelineTrackKind.video => rust.TimelineTrackKindView.video,
    ProjectTimelineTrackKind.audio => rust.TimelineTrackKindView.audio,
    ProjectTimelineTrackKind.text => rust.TimelineTrackKindView.text,
    ProjectTimelineTrackKind.caption => rust.TimelineTrackKindView.caption,
  };
}

class _RustProjectSession implements ProjectSessionHandle {
  const _RustProjectSession(this.host);

  final rust.ProjectHostHandle host;
}
