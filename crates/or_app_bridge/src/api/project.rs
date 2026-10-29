use super::media_artifact_config::create_media_artifact_service;
#[cfg(target_os = "android")]
use crate::android_media_io::capability_for;
use crate::frb_generated::StreamSink;
use crate::preview::{PreviewError, PreviewPreparationAction, PreviewRuntime, PreviewSnapshot};
use crate::transcription::{TranscriptionModelInfo, TranscriptionService};
use flutter_rust_bridge::frb;
use or_core::{
    ApplicationRequest, ApplicationResponse, AudioSettings, CacheArtifactKind,
    CaptionFileFormat, ClipContent, ClipId, ClipSettings, CommandEnvelope, Crop, EffectReference,
    ExportRequest, ExportResponse, FontIdentity, ImportedCaption, JobId, JobSnapshot, JobState,
    MarkerId, MediaArtifactEvent, MediaArtifactEventState, MediaArtifactRequest,
    MediaArtifactRequestState, MediaArtifactService, MediaId, MediaItem, MediaStreamMetadata,
    Opacity, OperationError, OperationErrorCode, ProjectFileSession, ProjectId, ProjectInstanceId,
    ProjectRecoveryError, ProjectRevision, ProjectSession, QueryEnvelope, QueryResult,
    RationalRate, RationalTime, RecoveryApplyOutcome, RecoveryConflictReason, RecoveryInspection,
    TextAlignment, TextColor, TextFormatting, TextWeight, TimeRange, TimelineClipPageV2,
    TimelineClipState, TimelineMarkerPage, TimelineMarkerState, TimelineSnapMovingAnchor,
    TimelineSnapOperation, TimelineSnapResult, TimelineSnapTargetKind, TimelineTrackSummaryV2,
    TimelineTrimEdge, TrackId, TrackKind, TrackState, TranscriptionFailure, TranscriptionJobView,
    TranscriptionProposalView, TranscriptionRequest, TranscriptionResponse,
    TranscriptionSegmentView, Transform, TransitionKind, TransitionReference, VisualSettings,
    apply_project_recovery, discard_project_recovery, encode_caption_file,
    inspect_project_recovery, parse_caption_file, prepare_media_import,
    prepare_media_import_with_probe,
};
#[cfg(target_os = "android")]
use or_core::{MediaSourceRef, prepare_media_import_from_probe};
use or_ipc::{
    ExportRequestHandler, LiveProjectHost, LiveProjectHostError, MotionMaterializeHandler,
    ProjectHostEvent, ProjectHostEventKind, TranscriptionRequestHandler,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
};


pub use super::views::*;

/// Rust-owned opaque handle; it is the only bridge path to the live project host.
#[frb(opaque)]
pub struct ProjectHostHandle {
    pub(crate) host: LiveProjectHost,
    pub(crate) media_artifact_service: Option<MediaArtifactService>,
    pub(crate) transcription_service: Option<Arc<TranscriptionService>>,
    pub(crate) preview_runtime: Arc<PreviewRuntime>,
}

pub fn create_project(path: String, name: String) -> Result<ProjectHostHandle, ProjectBridgeError> {
    let session =
        ProjectFileSession::create_new(Path::new(&path), name).map_err(project_session_error)?;
    start_project_host(session)
}

pub fn open_project(path: String) -> Result<ProjectHostHandle, ProjectBridgeError> {
    let session = ProjectFileSession::open(Path::new(&path)).map_err(project_session_error)?;
    start_project_host(session)
}

fn start_project_host(
    session: ProjectFileSession,
) -> Result<ProjectHostHandle, ProjectBridgeError> {
    let preview_runtime = Arc::new(PreviewRuntime::new());
    let export_handler: Arc<dyn ExportRequestHandler> = preview_runtime.clone();
    let transcription_service = TranscriptionService::new().map(Arc::new);
    let transcription_handler: Option<Arc<dyn TranscriptionRequestHandler>> =
        transcription_service.as_ref().map(|service| {
            Arc::new(BridgeTranscriptionHandler {
                service: Arc::clone(service),
            }) as Arc<dyn TranscriptionRequestHandler>
        });
    #[cfg(target_os = "android")]
    let host = LiveProjectHost::in_process_with_runtimes(
        session,
        Some(export_handler),
        None,
        transcription_handler,
    );
    #[cfg(not(target_os = "android"))]
    let host = {
        let motion_handler: Arc<dyn MotionMaterializeHandler> = preview_runtime.clone();
        LiveProjectHost::start_with_runtimes(
            session,
            None,
            Some(export_handler),
            Some(motion_handler),
            transcription_handler,
        )
        .map_err(host_error)?
    };
    Ok(ProjectHostHandle {
        host,
        media_artifact_service: create_media_artifact_service(),
        transcription_service,
        preview_runtime,
    })
}

/// Bridges the runtime transcription service onto the IPC application contract.
///
/// The host holds the project lock while this runs, so every branch returns
/// immediately: the model runs in the service's own background job.
struct BridgeTranscriptionHandler {
    service: Arc<TranscriptionService>,
}

impl TranscriptionRequestHandler for BridgeTranscriptionHandler {
    fn handle_transcription_request(
        &self,
        session: &ProjectSession,
        request: TranscriptionRequest,
    ) -> TranscriptionResponse {
        let (project_id, project_instance_id) = match request_identity(&request) {
            Some(identity) => identity,
            None => {
                return transcription_failure(
                    "TRANSCRIPTION_REQUEST_INVALID",
                    "The transcription request is incomplete.",
                );
            }
        };
        if session.project_id() != project_id
            || session.project_instance_id() != project_instance_id
        {
            return transcription_failure(
                "PROJECT_SESSION_MISMATCH",
                "The transcription request belongs to another project session.",
            );
        }
        match request {
            TranscriptionRequest::Start {
                expected_project_revision,
                media_id,
                ..
            } => {
                if session.project_revision() != expected_project_revision {
                    return transcription_failure(
                        "PROJECT_REVISION_CONFLICT",
                        "The project changed before transcription started.",
                    );
                }
                let Some(media) = session
                    .project()
                    .media_items()
                    .iter()
                    .find(|item| item.id() == media_id)
                    .cloned()
                else {
                    return transcription_failure(
                        "MEDIA_NOT_FOUND",
                        "The media item is not part of this project.",
                    );
                };
                let project_uuid = match uuid::Uuid::parse_str(&project_id.to_string()) {
                    Ok(uuid) => uuid,
                    Err(_) => {
                        return transcription_failure(
                            "TRANSCRIPTION_REQUEST_INVALID",
                            "The project identity is invalid.",
                        );
                    }
                };
                match self
                    .service
                    .start(project_uuid, expected_project_revision.value(), media)
                {
                    Ok(job) => transcription_job(job),
                    Err(error) => transcription_service_failure(error),
                }
            }
            TranscriptionRequest::Status { job_id, .. } => match self.service.snapshot(job_id) {
                Some(job) => transcription_job(job),
                None => transcription_failure(
                    "JOB_NOT_FOUND",
                    "The transcription job is no longer available.",
                ),
            },
            TranscriptionRequest::Proposal { job_id, .. } => {
                let Some(proposal) = self.service.proposal(job_id) else {
                    return transcription_failure(
                        "PROPOSAL_UNAVAILABLE",
                        "The transcript proposal is no longer available.",
                    );
                };
                if proposal.project_id.to_string() != project_id.to_string() {
                    return transcription_failure(
                        "PROJECT_SESSION_MISMATCH",
                        "The transcript proposal belongs to another project.",
                    );
                }
                let media_id = match MediaId::from_str(&proposal.media_id.to_string()) {
                    Ok(media_id) => media_id,
                    Err(_) => {
                        return transcription_failure(
                            "PROPOSAL_UNAVAILABLE",
                            "The transcript proposal media identity is invalid.",
                        );
                    }
                };
                TranscriptionResponse::Proposal(TranscriptionProposalView {
                    project_id,
                    project_revision: ProjectRevision::new(proposal.project_revision),
                    media_id,
                    provider_id: proposal.provider_id,
                    model_id: proposal.model_id,
                    model_version: proposal.model_version,
                    model_license: proposal.model_license,
                    language: proposal.language,
                    segments: proposal
                        .segments
                        .into_iter()
                        .map(|segment| TranscriptionSegmentView {
                            start_ms: segment.start_ms,
                            end_ms: segment.end_ms,
                            text: segment.text,
                        })
                        .collect(),
                })
            }
            TranscriptionRequest::Cancel { job_id, .. } => match self.service.cancel(job_id) {
                Ok(_) => match self.service.snapshot(job_id) {
                    Some(job) => transcription_job(job),
                    None => transcription_failure(
                        "JOB_NOT_FOUND",
                        "The transcription job is no longer available.",
                    ),
                },
                Err(_) => transcription_failure(
                    "JOB_NOT_FOUND",
                    "The transcription job is no longer available.",
                ),
            },
        }
    }
}

fn request_identity(request: &TranscriptionRequest) -> Option<(ProjectId, ProjectInstanceId)> {
    match request {
        TranscriptionRequest::Start {
            project_id,
            project_instance_id,
            ..
        }
        | TranscriptionRequest::Status {
            project_id,
            project_instance_id,
            ..
        }
        | TranscriptionRequest::Proposal {
            project_id,
            project_instance_id,
            ..
        }
        | TranscriptionRequest::Cancel {
            project_id,
            project_instance_id,
            ..
        } => Some((*project_id, *project_instance_id)),
    }
}

fn transcription_job(job: JobSnapshot) -> TranscriptionResponse {
    TranscriptionResponse::Job(TranscriptionJobView {
        job_id: job.id,
        state: job.state,
        failure_message: job.failure_message,
    })
}

fn transcription_failure(code: &str, message: &str) -> TranscriptionResponse {
    TranscriptionResponse::Failure(TranscriptionFailure::new(code, message))
}

fn transcription_service_failure(
    error: crate::transcription::TranscriptionServiceError,
) -> TranscriptionResponse {
    let code = match error {
        crate::transcription::TranscriptionServiceError::AudioRequired => {
            "TRANSCRIPTION_AUDIO_REQUIRED"
        }
        crate::transcription::TranscriptionServiceError::DownloadBusy => "MODEL_DOWNLOAD_BUSY",
        crate::transcription::TranscriptionServiceError::DownloadNotFound => {
            "MODEL_DOWNLOAD_NOT_FOUND"
        }
        crate::transcription::TranscriptionServiceError::InvalidId => {
            "TRANSCRIPTION_REQUEST_INVALID"
        }
        crate::transcription::TranscriptionServiceError::JobBusy => "TRANSCRIPTION_BUSY",
        crate::transcription::TranscriptionServiceError::JobUnavailable => "JOB_NOT_FOUND",
        crate::transcription::TranscriptionServiceError::LocalFileRequired => {
            "TRANSCRIPTION_LOCAL_FILE_REQUIRED"
        }
        crate::transcription::TranscriptionServiceError::ModelAlreadyInstalled => {
            "MODEL_ALREADY_INSTALLED"
        }
        crate::transcription::TranscriptionServiceError::ModelUnavailable => {
            "TRANSCRIPTION_MODEL_UNAVAILABLE"
        }
        crate::transcription::TranscriptionServiceError::ModelNotFound => {
            "TRANSCRIPTION_MODEL_UNKNOWN"
        }
        crate::transcription::TranscriptionServiceError::ModelNotInstalled => {
            "TRANSCRIPTION_MODEL_NOT_INSTALLED"
        }
        crate::transcription::TranscriptionServiceError::PreferenceUnavailable => {
            "TRANSCRIPTION_PREFERENCE_UNAVAILABLE"
        }
        crate::transcription::TranscriptionServiceError::ProviderUnavailable => {
            "TRANSCRIPTION_RUNTIME_UNAVAILABLE"
        }
        crate::transcription::TranscriptionServiceError::StorageUnavailable => {
            "TRANSCRIPTION_STORAGE_UNAVAILABLE"
        }
        crate::transcription::TranscriptionServiceError::JobSubmit => {
            "TRANSCRIPTION_QUEUE_UNAVAILABLE"
        }
    };
    TranscriptionResponse::Failure(TranscriptionFailure::new(code, error.to_string()))
}
impl Drop for ProjectHostHandle {
    fn drop(&mut self) {
        if let Some(service) = self.media_artifact_service.take() {
            service.shutdown();
        }
    }
}

pub fn inspect_recovery(path: String) -> RecoveryInspectionView {
    match inspect_project_recovery(Path::new(&path)) {
        Ok(RecoveryInspection::None) => RecoveryInspectionView {
            status: "none".to_owned(),
            project_id: String::new(),
            base_revision: 0,
            recovery_revision: 0,
            recovery_name: String::new(),
            conflict_reason: String::new(),
            message: String::new(),
        },
        Ok(RecoveryInspection::Candidate(candidate)) => {
            let metadata = candidate.metadata();
            RecoveryInspectionView {
                status: "candidate".to_owned(),
                project_id: metadata.project_id.to_string(),
                base_revision: metadata.base_revision.value(),
                recovery_revision: metadata.recovery_revision.value(),
                recovery_name: candidate.recovery_project().name().to_owned(),
                conflict_reason: String::new(),
                message: String::new(),
            }
        }
        Ok(RecoveryInspection::Stale(metadata)) => RecoveryInspectionView {
            status: "stale".to_owned(),
            project_id: metadata.project_id.to_string(),
            base_revision: metadata.base_revision.value(),
            recovery_revision: metadata.recovery_revision.value(),
            recovery_name: String::new(),
            conflict_reason: String::new(),
            message: "A stale recovery checkpoint is present.".to_owned(),
        },
        Ok(RecoveryInspection::Conflict { metadata, reason }) => RecoveryInspectionView {
            status: "conflict".to_owned(),
            project_id: metadata.project_id.to_string(),
            base_revision: metadata.base_revision.value(),
            recovery_revision: metadata.recovery_revision.value(),
            recovery_name: String::new(),
            conflict_reason: recovery_conflict_name(reason).to_owned(),
            message: "The recovery checkpoint does not match this project lineage.".to_owned(),
        },
        Err(error) => RecoveryInspectionView {
            status: "invalid".to_owned(),
            project_id: String::new(),
            base_revision: 0,
            recovery_revision: 0,
            recovery_name: String::new(),
            conflict_reason: String::new(),
            message: error.to_string(),
        },
    }
}

pub fn apply_recovery(path: String) -> RecoveryActionResult {
    match apply_project_recovery(Path::new(&path)) {
        Ok(RecoveryApplyOutcome::AppliedAndCleaned) => RecoveryActionResult {
            succeeded: true,
            changed: true,
            message: "Recovery applied.".to_owned(),
        },
        Ok(RecoveryApplyOutcome::AppliedCleanupPending { error }) => RecoveryActionResult {
            succeeded: true,
            changed: true,
            message: format!("Recovery applied; checkpoint cleanup is pending: {error}"),
        },
        Err(error) => recovery_action_error(error),
    }
}

pub fn discard_recovery(path: String) -> RecoveryActionResult {
    match discard_project_recovery(Path::new(&path)) {
        Ok(changed) => RecoveryActionResult {
            succeeded: true,
            changed,
            message: if changed {
                "Recovery checkpoint discarded.".to_owned()
            } else {
                "No recovery checkpoint was present.".to_owned()
            },
        },
        Err(error) => recovery_action_error(error),
    }
}

impl ProjectHostHandle {
    pub fn transcription_model(&self) -> ProjectTranscriptionModelView {
        let info = self
            .transcription_service
            .as_deref()
            .map(TranscriptionService::model_info)
            .unwrap_or_else(TranscriptionModelInfo::unavailable);
        transcription_model_view(info)
    }

    pub fn transcription_models(&self) -> Vec<ProjectTranscriptionModelOptionView> {
        self.transcription_service
            .as_deref()
            .map(TranscriptionService::models)
            .unwrap_or_default()
            .into_iter()
            .map(transcription_model_option_view)
            .collect()
    }

    pub fn select_transcription_model(
        &self,
        model_id: String,
    ) -> Result<ProjectTranscriptionModelView, ProjectBridgeError> {
        let info = self
            .transcription_service()?
            .select_model(&model_id)
            .map_err(transcription_error)?;
        Ok(transcription_model_view(info))
    }

    pub fn remove_transcription_model(
        &self,
        model_id: String,
    ) -> Result<ProjectTranscriptionModelView, ProjectBridgeError> {
        let info = self
            .transcription_service()?
            .remove_model(&model_id)
            .map_err(transcription_error)?;
        Ok(transcription_model_view(info))
    }

    pub fn begin_transcription_model_download(
        &self,
        model_id: String,
    ) -> Result<ProjectModelDownloadView, ProjectBridgeError> {
        let download = self
            .transcription_service()?
            .begin_model_download(&model_id)
            .map_err(transcription_error)?;
        Ok(model_download_view(download))
    }

    pub fn finish_transcription_model_download(
        &self,
        download_id: String,
    ) -> Result<ProjectTranscriptionModelView, ProjectBridgeError> {
        let info = self
            .transcription_service()?
            .finish_model_download(&download_id)
            .map_err(transcription_error)?;
        Ok(transcription_model_view(info))
    }

    pub fn cancel_transcription_model_download(
        &self,
        download_id: String,
    ) -> Result<(), ProjectBridgeError> {
        self.transcription_service()?
            .cancel_model_download(&download_id)
            .map_err(transcription_error)
    }

    pub fn start_transcription(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        media_id: String,
    ) -> Result<ProjectTranscriptionJobView, ProjectBridgeError> {
        let (project_id, project_instance_id, revision) =
            parse_session_identity(&project_id, &project_instance_id, expected_revision)?;
        let id = MediaId::from_str(&media_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_MEDIA_ID".to_owned(),
            message: error.to_string(),
        })?;
        let summary = self.host.describe().map_err(host_error)?;
        if summary.summary.project_id != project_id
            || summary.summary.project_instance_id != project_instance_id
            || summary.summary.project_revision != revision
        {
            return Err(revision_conflict_error());
        }
        let request = QueryEnvelope::media_get(project_id, project_instance_id, id);
        let media = match self
            .host
            .handle_application_request(ApplicationRequest::Query(request))
            .map_err(host_error)?
        {
            ApplicationResponse::Query(result) => result
                .media_item
                .map(|item| *item)
                .ok_or_else(unexpected_response_error)?,
            ApplicationResponse::Error(error) => return Err(operation_bridge_error(error)),
            _ => return Err(unexpected_response_error()),
        };
        let job = self
            .transcription_service()?
            .start(
                uuid::Uuid::parse_str(&project_id.to_string()).map_err(|error| {
                    ProjectBridgeError {
                        code: "INVALID_PROJECT_ID".to_owned(),
                        message: error.to_string(),
                    }
                })?,
                revision.value(),
                media,
            )
            .map_err(transcription_error)?;
        Ok(transcription_job_view(job))
    }

    pub fn transcription_job(
        &self,
        job_id: String,
    ) -> Result<ProjectTranscriptionJobView, ProjectBridgeError> {
        let id = JobId::from_str(&job_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_JOB_ID".to_owned(),
            message: error.to_string(),
        })?;
        let job = self
            .transcription_service()?
            .snapshot(id)
            .ok_or_else(|| ProjectBridgeError {
                code: "JOB_NOT_FOUND".to_owned(),
                message: "The transcription job is no longer available.".to_owned(),
            })?;
        Ok(transcription_job_view(job))
    }

    pub fn transcription_proposal(
        &self,
        job_id: String,
    ) -> Result<Option<ProjectTranscriptProposalView>, ProjectBridgeError> {
        let id = JobId::from_str(&job_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_JOB_ID".to_owned(),
            message: error.to_string(),
        })?;
        Ok(self
            .transcription_service()?
            .proposal(id)
            .map(transcription_proposal_view))
    }

    pub fn clear_transcription_proposal(&self, job_id: String) -> Result<(), ProjectBridgeError> {
        let id = JobId::from_str(&job_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_JOB_ID".to_owned(),
            message: error.to_string(),
        })?;
        self.transcription_service()?.clear_proposal(id);
        Ok(())
    }

    pub fn cancel_transcription(&self, job_id: String) -> Result<String, ProjectBridgeError> {
        let id = JobId::from_str(&job_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_JOB_ID".to_owned(),
            message: error.to_string(),
        })?;
        let outcome = self
            .transcription_service()?
            .cancel(id)
            .map_err(|_| ProjectBridgeError {
                code: "JOB_NOT_FOUND".to_owned(),
                message: "The transcription job is no longer available.".to_owned(),
            })?;
        Ok(match outcome {
            or_core::JobCancelOutcome::CancelledQueued => "cancelled".to_owned(),
            or_core::JobCancelOutcome::CancellationRequested => "cancelling".to_owned(),
            or_core::JobCancelOutcome::AlreadyTerminal(state) => job_state_name(state).to_owned(),
        })
    }

    fn transcription_service(&self) -> Result<&TranscriptionService, ProjectBridgeError> {
        self.transcription_service.as_deref().ok_or_else(|| {
            transcription_error(
                crate::transcription::TranscriptionServiceError::ProviderUnavailable,
            )
        })
    }

    pub fn export_timeline_captions(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        path: String,
        format: CaptionFileFormatView,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => return action_error(error),
            };
        match self.export_captions_at_revision(
            project_id,
            project_instance_id,
            revision.value(),
            Path::new(&path),
            match format {
                CaptionFileFormatView::Srt => CaptionFileFormat::Srt,
                CaptionFileFormatView::WebVtt => CaptionFileFormat::WebVtt,
            },
        ) {
            Ok(count) => ProjectActionResult {
                succeeded: true,
                error_code: String::new(),
                message: format!("Exported {count} captions."),
                view: self.project_view().ok(),
            },
            Err(error) => action_error(error),
        }
    }

    fn export_captions_at_revision(
        &self,
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_revision: u64,
        path: &Path,
        format: CaptionFileFormat,
    ) -> Result<usize, ProjectBridgeError> {
        let tracks_result = self.query(QueryEnvelope::timeline_tracks_v2(
            project_id,
            project_instance_id,
        ))?;
        if tracks_result.summary.project_revision.value() != expected_revision {
            return Err(revision_conflict_error());
        }
        let tracks = tracks_result
            .timeline_tracks_v2
            .ok_or_else(unexpected_response_error)?;
        let mut captions = Vec::new();
        for track in tracks
            .into_iter()
            .filter(|track| track.kind == TrackKind::Caption)
        {
            let mut offset = 0usize;
            loop {
                let page_result = self.query(QueryEnvelope::timeline_clips_v2(
                    tracks_result.summary.project_id,
                    tracks_result.summary.project_instance_id,
                    track.track_id,
                    offset,
                    100,
                ))?;
                if page_result.summary.project_revision.value() != expected_revision {
                    return Err(revision_conflict_error());
                }
                let page = page_result
                    .timeline_clip_page_v2
                    .ok_or_else(unexpected_response_error)?;
                for clip in page.items {
                    if let ClipContent::Caption { text, .. } = clip.content {
                        captions.push(ImportedCaption {
                            start: clip.timeline_start,
                            duration: clip.timeline_duration,
                            text,
                        });
                    }
                }
                match page.next_offset {
                    Some(next) => offset = next,
                    None => break,
                }
            }
        }
        captions.sort_by_key(|caption| caption.start);
        if captions.is_empty() {
            return Err(ProjectBridgeError {
                code: "CAPTION_TRACK_EMPTY".to_owned(),
                message: "The project has no captions to export.".to_owned(),
            });
        }
        let bytes = encode_caption_file(format, &captions).map_err(|error| ProjectBridgeError {
            code: error.code().to_owned(),
            message: error.to_string(),
        })?;
        write_caption_file_atomically(path, &bytes)?;
        Ok(captions.len())
    }

    pub fn preview_timeline_captions(
        &self,
        path: String,
    ) -> Result<ProjectCaptionImportPreviewView, ProjectBridgeError> {
        let bytes = read_bounded_caption_file(Path::new(&path))?;
        let plan = parse_caption_file(&bytes).map_err(|error| ProjectBridgeError {
            code: error.code().to_owned(),
            message: error.to_string(),
        })?;
        let format_name = match plan.format {
            or_core::CaptionFileFormat::Srt => "SubRip",
            or_core::CaptionFileFormat::WebVtt => "WebVTT",
        }
        .to_owned();
        let mut cues = Vec::with_capacity(plan.captions.len());
        for caption in plan.captions {
            let end = caption
                .start
                .checked_add(caption.duration)
                .map_err(|_| caption_time_error())?;
            cues.push(ProjectCaptionCueView {
                start_ms: rational_time_millis(caption.start)?,
                end_ms: rational_time_millis(end)?,
                text: caption.text,
            });
        }
        Ok(ProjectCaptionImportPreviewView {
            cue_count: cues.len() as u64,
            formatting_loss_count: plan.cues_with_formatting_loss as u64,
            empty_cues_skipped: plan.empty_cues_skipped as u64,
            max_caption_text_bytes: or_core::MAX_TIMELINE_CAPTION_BYTES as u64,
            format_name,
            cues,
        })
    }

    /// Applies user-reviewed caption cues through the same validated project
    /// command used by caption-file import. The bridge never mutates project
    /// state while the review dialog is open.
    pub fn import_reviewed_timeline_captions(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        cues: Vec<ProjectCaptionCueView>,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => return action_error(error),
            };
        if cues.is_empty() || cues.len() > or_core::MAX_IMPORTED_CAPTIONS {
            return action_error(ProjectBridgeError {
                code: "CAPTION_LIMIT_EXCEEDED".to_owned(),
                message: "Choose between 1 and 10,000 non-empty caption cues.".to_owned(),
            });
        }
        let mut clips = Vec::with_capacity(cues.len());
        for cue in cues {
            if cue.text.trim().is_empty() {
                continue;
            }
            if cue.text.len() > or_core::MAX_TIMELINE_CAPTION_BYTES {
                return action_error(ProjectBridgeError {
                    code: "CAPTION_TEXT_TOO_LARGE".to_owned(),
                    message: "A caption exceeds the project text limit.".to_owned(),
                });
            }
            let (Ok(start_ms), Ok(end_ms)) =
                (i64::try_from(cue.start_ms), i64::try_from(cue.end_ms))
            else {
                return action_error(caption_time_error());
            };
            if end_ms <= start_ms {
                return action_error(ProjectBridgeError {
                    code: "CAPTION_TIMING_INVALID".to_owned(),
                    message: "A caption must end after it starts.".to_owned(),
                });
            }
            let (Ok(timeline_start), Ok(end)) = (
                RationalTime::new(start_ms, 1_000),
                RationalTime::new(end_ms, 1_000),
            ) else {
                return action_error(caption_time_error());
            };
            let Ok(timeline_duration) = end.checked_sub(timeline_start) else {
                return action_error(ProjectBridgeError {
                    code: "CAPTION_TIMING_INVALID".to_owned(),
                    message: "A caption has an invalid time range.".to_owned(),
                });
            };
            clips.push(TimelineClipState {
                clip_id: ClipId::generate(),
                timeline_start,
                timeline_duration,
                content: ClipContent::Caption {
                    text: cue.text,
                    formatting: TextFormatting::CAPTION_DEFAULT,
                },
                settings: ClipSettings::Visual(VisualSettings::default()),
            });
        }
        if clips.is_empty() {
            return action_error(ProjectBridgeError {
                code: "CAPTION_FILE_EMPTY".to_owned(),
                message: "At least one caption must contain text.".to_owned(),
            });
        }
        let imported_count = clips.len();
        let result = self.dispatch_command(CommandEnvelope::import_timeline_captions(
            project_id,
            project_instance_id,
            revision,
            TrackId::generate(),
            clips,
        ));
        if result.succeeded {
            ProjectActionResult {
                message: format!("Imported {imported_count} reviewed captions."),
                ..result
            }
        } else {
            result
        }
    }

    pub fn summary(&self) -> Result<ProjectView, ProjectBridgeError> {
        self.project_view().map_err(host_error)
    }

    pub fn list_media_page(
        &self,
        offset: u64,
        limit: u64,
    ) -> Result<ProjectMediaPageView, ProjectBridgeError> {
        let summary = self.host.describe().map_err(host_error)?;
        let offset = usize::try_from(offset).map_err(|_| invalid_arguments())?;
        let limit = usize::try_from(limit).map_err(|_| invalid_arguments())?;
        let request = QueryEnvelope::media_list(
            summary.summary.project_id,
            summary.summary.project_instance_id,
            offset,
            limit,
        );
        let result = match self
            .host
            .handle_application_request(ApplicationRequest::Query(request))
        {
            Ok(ApplicationResponse::Query(result)) => result,
            Ok(ApplicationResponse::Error(error)) => {
                return Err(operation_bridge_error(error));
            }
            Ok(_) => return Err(unexpected_response_error()),
            Err(error) => return Err(host_error(error)),
        };
        let page = result
            .media_page
            .as_ref()
            .ok_or_else(unexpected_response_error)?;
        Ok(ProjectMediaPageView {
            project_id: result.summary.project_id.to_string(),
            project_instance_id: result.summary.project_instance_id.to_string(),
            project_revision: result.summary.project_revision.value(),
            items: page.items.iter().map(media_item_view).collect(),
            total_count: u64::try_from(page.total_count).expect("usize fits in u64"),
            offset: u64::try_from(page.offset).expect("usize fits in u64"),
            limit: u64::try_from(page.limit).expect("usize fits in u64"),
            next_offset: page
                .next_offset
                .map(|offset| u64::try_from(offset).expect("usize fits in u64")),
        })
    }

    pub fn list_timeline_tracks(&self) -> Result<ProjectTimelineTracksView, ProjectBridgeError> {
        let described = self.host.describe().map_err(host_error)?;
        let result = self.query(QueryEnvelope::timeline_tracks_v2(
            described.summary.project_id,
            described.summary.project_instance_id,
        ))?;
        let tracks = result
            .timeline_tracks_v2
            .ok_or_else(unexpected_response_error)?;
        Ok(ProjectTimelineTracksView {
            project_id: result.summary.project_id.to_string(),
            project_instance_id: result.summary.project_instance_id.to_string(),
            project_revision: result.summary.project_revision.value(),
            items: tracks.iter().map(timeline_track_view).collect(),
        })
    }

    pub fn list_timeline_clips(
        &self,
        track_id: String,
        offset: u64,
        limit: u64,
    ) -> Result<ProjectTimelineClipPageView, ProjectBridgeError> {
        let track_id = TrackId::from_str(&track_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_TRACK_ID".to_owned(),
            message: error.to_string(),
        })?;
        let offset = usize::try_from(offset).map_err(|_| timeline_query_arguments_error())?;
        let limit = usize::try_from(limit).map_err(|_| timeline_query_arguments_error())?;
        let described = self.host.describe().map_err(host_error)?;
        let result = self.query(QueryEnvelope::timeline_clips_v2(
            described.summary.project_id,
            described.summary.project_instance_id,
            track_id,
            offset,
            limit,
        ))?;
        let page = result
            .timeline_clip_page_v2
            .as_ref()
            .ok_or_else(unexpected_response_error)?;
        Ok(timeline_clip_page_view(&result, page))
    }

    pub fn get_timeline_clip_visual_settings(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
    ) -> Result<ProjectTimelineVisualSettingsView, ProjectBridgeError> {
        let (_, _, _, clip) = self.find_visual_clip(
            &project_id,
            &project_instance_id,
            expected_revision,
            &track_id,
            &clip_id,
        )?;
        let ClipSettings::Visual(settings) = clip.settings else {
            return Err(unsupported_visual_settings());
        };
        Ok(visual_settings_view(&settings))
    }

    pub fn update_timeline_clip_visual_settings(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        settings: ProjectTimelineVisualSettingsView,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision, clip) = match self.find_visual_clip(
            &project_id,
            &project_instance_id,
            expected_revision,
            &track_id,
            &clip_id,
        ) {
            Ok(result) => result,
            Err(error) => return action_error(error),
        };
        let ClipSettings::Visual(mut visual) = clip.settings else {
            return action_error(unsupported_visual_settings());
        };
        visual.transform = Transform {
            x_milli_canvas: settings.x_milli_canvas,
            y_milli_canvas: settings.y_milli_canvas,
            scale_x_milli: settings.scale_x_milli,
            scale_y_milli: settings.scale_y_milli,
            rotation_milli_degrees: settings.rotation_milli_degrees,
            anchor_x_basis_points: settings.anchor_x_basis_points,
            anchor_y_basis_points: settings.anchor_y_basis_points,
        };
        visual.crop = Crop {
            left_basis_points: settings.crop_left_basis_points,
            top_basis_points: settings.crop_top_basis_points,
            right_basis_points: settings.crop_right_basis_points,
            bottom_basis_points: settings.crop_bottom_basis_points,
        };
        visual.opacity = Opacity {
            basis_points: settings.opacity_basis_points,
        };
        visual.effects = effects_from_view(&visual.effects, &settings);
        if settings.update_transition_in {
            visual.transition_in = match transition_from_view(
                settings.transition_in_kind,
                settings.transition_in_duration,
            ) {
                Ok(transition) => transition,
                Err(error) => return action_error(error),
            };
        }
        if settings.update_transition_out {
            visual.transition_out = match transition_from_view(
                settings.transition_out_kind,
                settings.transition_out_duration,
            ) {
                Ok(transition) => transition,
                Err(error) => return action_error(error),
            };
        }
        self.dispatch_command(CommandEnvelope::update_timeline_clip(
            project_id,
            project_instance_id,
            revision,
            clip.clip_id,
            clip.timeline_duration,
            clip.content,
            ClipSettings::Visual(visual),
        ))
    }

    pub fn get_timeline_clip_audio_settings(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
    ) -> Result<ProjectTimelineAudioSettingsView, ProjectBridgeError> {
        let (_, _, _, clip) = self.find_audio_clip(
            &project_id,
            &project_instance_id,
            expected_revision,
            &track_id,
            &clip_id,
        )?;
        let ClipSettings::Audio(settings) = clip.settings else {
            return Err(unsupported_audio_settings());
        };
        Ok(audio_settings_view(settings))
    }

    pub fn update_timeline_clip_audio_settings(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        settings: ProjectTimelineAudioSettingsView,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision, clip) = match self.find_audio_clip(
            &project_id,
            &project_instance_id,
            expected_revision,
            &track_id,
            &clip_id,
        ) {
            Ok(result) => result,
            Err(error) => return action_error(error),
        };
        let fade_in = match time_from_view(settings.fade_in) {
            Ok(time) => time,
            Err(error) => return action_error(error),
        };
        let fade_out = match time_from_view(settings.fade_out) {
            Ok(time) => time,
            Err(error) => return action_error(error),
        };
        self.dispatch_command(CommandEnvelope::update_timeline_clip(
            project_id,
            project_instance_id,
            revision,
            clip.clip_id,
            clip.timeline_duration,
            clip.content,
            ClipSettings::Audio(AudioSettings {
                gain_millidecibels: settings.gain_millidecibels,
                pan_basis_points: settings.pan_basis_points,
                fade_in,
                fade_out,
            }),
        ))
    }

    pub fn update_timeline_clip_playback_rate(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        rate: RationalRateView,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision, track_kind, clip) = match self
            .find_timeline_clip_state(
                &project_id,
                &project_instance_id,
                expected_revision,
                &track_id,
                &clip_id,
            ) {
            Ok(result) => result,
            Err(error) => return action_error(error),
        };
        if !matches!(track_kind, TrackKind::Video | TrackKind::Audio) {
            return action_error(ProjectBridgeError {
                code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                message: "Playback speed is available for video and audio media clips.".to_owned(),
            });
        }
        let rate = match RationalRate::new(rate.numerator, rate.denominator) {
            Ok(rate) => rate,
            Err(error) => {
                return action_error(ProjectBridgeError {
                    code: "INVALID_PLAYBACK_RATE".to_owned(),
                    message: format!("The playback rate is invalid: {error}"),
                });
            }
        };
        let source_range = match &clip.content {
            ClipContent::Media { source_range, .. } => *source_range,
            ClipContent::Text { .. } | ClipContent::Caption { .. } => {
                return action_error(ProjectBridgeError {
                    code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                    message: "Playback speed is available for video and audio media clips."
                        .to_owned(),
                });
            }
        };
        let timeline_duration = match RationalRate::new(rate.denominator(), rate.numerator())
            .and_then(|inverse_rate| inverse_rate.apply(source_range.duration()))
        {
            Ok(duration) => duration,
            Err(error) => {
                return action_error(ProjectBridgeError {
                    code: "INVALID_PLAYBACK_RATE".to_owned(),
                    message: format!("The playback rate cannot be represented exactly: {error}"),
                });
            }
        };
        let mut content = clip.content;
        if let ClipContent::Media { playback_rate, .. } = &mut content {
            *playback_rate = rate;
        }
        self.dispatch_command(CommandEnvelope::update_timeline_clip(
            project_id,
            project_instance_id,
            revision,
            clip.clip_id,
            timeline_duration,
            content,
            clip.settings,
        ))
    }

    /// Flip the direction a media clip plays without touching its duration.
    ///
    /// Reverse is not a rate: the clip still covers the same source span and the same
    /// timeline span, so only the direction changes and the caller keeps every other
    /// property of the clip.
    pub fn set_timeline_clip_reversed(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        reversed: bool,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision, track_kind, clip) = match self
            .find_timeline_clip_state(
                &project_id,
                &project_instance_id,
                expected_revision,
                &track_id,
                &clip_id,
            ) {
            Ok(result) => result,
            Err(error) => return action_error(error),
        };
        if !matches!(track_kind, TrackKind::Video | TrackKind::Audio) {
            return action_error(ProjectBridgeError {
                code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                message: "Reverse is available for video and audio media clips.".to_owned(),
            });
        }
        let mut content = clip.content;
        match &mut content {
            ClipContent::Media {
                reversed: current, ..
            } => *current = reversed,
            ClipContent::Text { .. } | ClipContent::Caption { .. } => {
                return action_error(ProjectBridgeError {
                    code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                    message: "Reverse is available for video and audio media clips.".to_owned(),
                });
            }
        }
        self.dispatch_command(CommandEnvelope::update_timeline_clip(
            project_id,
            project_instance_id,
            revision,
            clip.clip_id,
            clip.timeline_duration,
            content,
            clip.settings,
        ))
    }

    /// Holds one frame for the clip's whole duration, or releases it.
    ///
    /// The source range is left untouched, so unfreezing restores the original
    /// footage instead of a shortened clip.
    pub fn set_timeline_clip_frozen(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        frozen: bool,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision, track_kind, clip) = match self
            .find_timeline_clip_state(
                &project_id,
                &project_instance_id,
                expected_revision,
                &track_id,
                &clip_id,
            ) {
            Ok(result) => result,
            Err(error) => return action_error(error),
        };
        if !matches!(track_kind, TrackKind::Video) {
            return action_error(ProjectBridgeError {
                code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                message: "Freeze is available for video clips.".to_owned(),
            });
        }
        let mut content = clip.content;
        match &mut content {
            ClipContent::Media {
                frozen: current, ..
            } => *current = frozen,
            ClipContent::Text { .. } | ClipContent::Caption { .. } => {
                return action_error(ProjectBridgeError {
                    code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                    message: "Freeze is available for video clips.".to_owned(),
                });
            }
        }
        self.dispatch_command(CommandEnvelope::update_timeline_clip(
            project_id,
            project_instance_id,
            revision,
            clip.clip_id,
            clip.timeline_duration,
            content,
            clip.settings,
        ))
    }

    pub fn list_timeline_markers(
        &self,
        offset: u64,
        limit: u64,
    ) -> Result<ProjectTimelineMarkerPageView, ProjectBridgeError> {
        let offset =
            usize::try_from(offset).map_err(|_| timeline_marker_query_arguments_error())?;
        let limit = usize::try_from(limit).map_err(|_| timeline_marker_query_arguments_error())?;
        let described = self.host.describe().map_err(host_error)?;
        let result = self.query(QueryEnvelope::timeline_markers(
            described.summary.project_id,
            described.summary.project_instance_id,
            offset,
            limit,
        ))?;
        let page = result
            .timeline_marker_page
            .as_ref()
            .ok_or_else(unexpected_response_error)?;
        Ok(timeline_marker_page_view(&result, page))
    }

    pub fn timeline_sequence_settings(
        &self,
    ) -> Result<ProjectTimelineSequenceSettingsView, ProjectBridgeError> {
        let described = self.host.describe().map_err(host_error)?;
        let result = self.query(QueryEnvelope::timeline_sequence_settings(
            described.summary.project_id,
            described.summary.project_instance_id,
        ))?;
        let settings = result
            .timeline_sequence_settings
            .ok_or_else(unexpected_response_error)?;
        Ok(ProjectTimelineSequenceSettingsView {
            project_id: result.summary.project_id.to_string(),
            project_instance_id: result.summary.project_instance_id.to_string(),
            project_revision: result.summary.project_revision.value(),
            sequence_frame_rate: settings.sequence_frame_rate.map(rational_rate_view),
        })
    }

    pub fn set_timeline_sequence_frame_rate(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        sequence_frame_rate: Option<RationalRateView>,
    ) -> ProjectActionResult {
        let rate = match sequence_frame_rate {
            Some(rate) => match RationalRate::new(rate.numerator, rate.denominator) {
                Ok(rate) => Some(rate),
                Err(error) => {
                    return action_error(ProjectBridgeError {
                        code: "INVALID_SEQUENCE_FRAME_RATE".to_owned(),
                        message: error.to_string(),
                    });
                }
            },
            None => None,
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::set_timeline_sequence_frame_rate(
                    project_id,
                    project_instance_id,
                    revision,
                    rate,
                )
            },
        )
    }

    pub fn preview_state(&self) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .status(&self.host)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_seek(
        &self,
        position: RationalTimeView,
    ) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        let position =
            RationalTime::new(position.numerator, position.denominator).map_err(|error| {
                ProjectBridgeError {
                    code: "INVALID_PREVIEW_TIME".to_owned(),
                    message: error.to_string(),
                }
            })?;
        self.preview_runtime
            .seek(&self.host, position)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_play(&self) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .play(&self.host)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_pause(&self) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .pause(&self.host)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_step(
        &self,
        direction: PreviewFrameStepView,
    ) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .step(
                &self.host,
                match direction {
                    PreviewFrameStepView::Previous => or_runtime::PreviewFrameStep::Previous,
                    PreviewFrameStepView::Next => or_runtime::PreviewFrameStep::Next,
                },
            )
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_tick(&self) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .tick(&self.host)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_prepare(
        &self,
        action: PreviewPreparationActionView,
        position: Option<RationalTimeView>,
    ) -> Result<ProjectPreviewPreparationView, ProjectBridgeError> {
        let action = match action {
            PreviewPreparationActionView::Seek => {
                let position = position.ok_or_else(|| ProjectBridgeError {
                    code: "INVALID_PREVIEW_TIME".to_owned(),
                    message: "Seek requires a preview position.".to_owned(),
                })?;
                PreviewPreparationAction::Seek(
                    RationalTime::new(position.numerator, position.denominator).map_err(
                        |error| ProjectBridgeError {
                            code: "INVALID_PREVIEW_TIME".to_owned(),
                            message: error.to_string(),
                        },
                    )?,
                )
            }
            PreviewPreparationActionView::Play => PreviewPreparationAction::Play,
            PreviewPreparationActionView::StepPrevious => {
                PreviewPreparationAction::Step(or_runtime::PreviewFrameStep::Previous)
            }
            PreviewPreparationActionView::StepNext => {
                PreviewPreparationAction::Step(or_runtime::PreviewFrameStep::Next)
            }
            PreviewPreparationActionView::Tick => PreviewPreparationAction::Tick,
        };
        self.preview_runtime
            .prepare(&self.host, action)
            .map(|prepared| ProjectPreviewPreparationView {
                request_id: prepared.request_id,
                sources: prepared.sources,
                state: preview_state_view(prepared.snapshot),
            })
            .map_err(preview_bridge_error)
    }

    pub fn preview_complete_prepared(
        &self,
        request_id: u64,
    ) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .complete_prepared(&self.host, request_id)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_abort_prepared(&self, request_id: u64) -> Result<(), ProjectBridgeError> {
        self.preview_runtime
            .abort_prepared(request_id)
            .map_err(preview_bridge_error)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve_timeline_snap(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        operation: TimelineSnapOperationView,
        clip_id: String,
        target_track_id: Option<String>,
        target_time_numerator: i64,
        target_time_denominator: u32,
    ) -> Result<ProjectTimelineSnapView, ProjectBridgeError> {
        let (project_id, project_instance_id, _) =
            parse_session_identity(&project_id, &project_instance_id, expected_revision)?;
        let clip_id = ClipId::from_str(&clip_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_CLIP_ID".to_owned(),
            message: error.to_string(),
        })?;
        let target_track_id = target_track_id
            .map(|track_id| {
                TrackId::from_str(&track_id).map_err(|error| ProjectBridgeError {
                    code: "INVALID_TRACK_ID".to_owned(),
                    message: error.to_string(),
                })
            })
            .transpose()?;
        let target_time = RationalTime::new(target_time_numerator, target_time_denominator)
            .map_err(|_| timeline_arguments_error())?;
        let operation = match operation {
            TimelineSnapOperationView::Move => TimelineSnapOperation::Move,
            TimelineSnapOperationView::TrimStart => TimelineSnapOperation::TrimStart,
            TimelineSnapOperationView::TrimEnd => TimelineSnapOperation::TrimEnd,
        };
        let result = self.query(QueryEnvelope::timeline_snap_v2(
            project_id,
            project_instance_id,
            operation,
            clip_id,
            target_track_id,
            target_time,
        ))?;
        let snap = result
            .timeline_snap
            .as_deref()
            .ok_or_else(unexpected_response_error)?;
        Ok(timeline_snap_view(&result, snap))
    }

    pub fn import_timeline_captions(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        path: String,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => return action_error(error),
            };
        let bytes = match read_bounded_caption_file(Path::new(&path)) {
            Ok(bytes) => bytes,
            Err(error) => return action_error(error),
        };
        let plan = match parse_caption_file(&bytes) {
            Ok(plan) => plan,
            Err(error) => {
                return action_error(ProjectBridgeError {
                    code: error.code().to_owned(),
                    message: error.to_string(),
                });
            }
        };
        let format_name = match plan.format {
            or_core::CaptionFileFormat::Srt => "SubRip",
            or_core::CaptionFileFormat::WebVtt => "WebVTT",
        };
        let imported_count = plan.captions.len();
        let formatting_loss_count = plan.cues_with_formatting_loss;
        let empty_count = plan.empty_cues_skipped;
        let clips = plan
            .captions
            .into_iter()
            .map(|caption| TimelineClipState {
                clip_id: ClipId::generate(),
                timeline_start: caption.start,
                timeline_duration: caption.duration,
                content: ClipContent::Caption {
                    text: caption.text,
                    formatting: TextFormatting::CAPTION_DEFAULT,
                },
                settings: ClipSettings::Visual(VisualSettings::default()),
            })
            .collect();
        let mut result = self.dispatch_command(CommandEnvelope::import_timeline_captions(
            project_id,
            project_instance_id,
            revision,
            TrackId::generate(),
            clips,
        ));
        if result.succeeded {
            result.message = format!("Imported {imported_count} {format_name} captions.");
            if formatting_loss_count > 0 {
                result.message.push_str(&format!(
                    " Styling or placement was simplified for {formatting_loss_count} cues."
                ));
            }
            if empty_count > 0 {
                result
                    .message
                    .push_str(&format!(" Skipped {empty_count} empty cues."));
            }
        }
        result
    }

    pub fn add_timeline_track(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        kind: TimelineTrackKindView,
    ) -> ProjectActionResult {
        let track_id = TrackId::generate();
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::add_timeline_track(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                    match kind {
                        TimelineTrackKindView::Video => TrackKind::Video,
                        TimelineTrackKindView::Audio => TrackKind::Audio,
                        TimelineTrackKindView::Text => TrackKind::Text,
                        TimelineTrackKindView::Caption => TrackKind::Caption,
                    },
                )
            },
        )
    }

    pub fn remove_timeline_track(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::remove_timeline_track(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                )
            },
        )
    }

    pub fn set_timeline_track_state(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        state: ProjectTimelineTrackStateView,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        let state = TrackState::new(state.locked, state.visible, state.muted, state.solo);
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::set_timeline_track_state(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                    state,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        media_id: String,
        timeline_start_numerator: i64,
        timeline_start_denominator: u32,
        source_start_numerator: i64,
        source_start_denominator: u32,
        duration_numerator: i64,
        duration_denominator: u32,
        // `insert` (the default) or `overwrite`: an explicit command input, never
        // canonical project state.
        mode: Option<String>,
        // Only read by overwrite when the new span lands strictly inside one
        // existing clip, mirroring `timeline.clip.split`'s `new_clip_id`.
        overwrite_split_clip_id: Option<String>,
    ) -> ProjectActionResult {
        let edit_mode = match mode.as_deref() {
            None => or_core::TimelineEditMode::Insert,
            Some(value) => match or_core::TimelineEditMode::parse_wire(value) {
                Some(edit_mode) => edit_mode,
                None => return action_error(timeline_arguments_error()),
            },
        };
        // The bridge owns id generation for this command, exactly as it does for
        // the inserted clip itself. A client may name the fragment; when it does
        // not, one is generated here so an overwrite that lands inside an existing
        // clip completes instead of refusing for want of an id no user could type.
        let split_clip_id = match overwrite_split_clip_id {
            None => None,
            Some(value) => match ClipId::from_str(&value) {
                Ok(clip_id) => Some(clip_id),
                Err(_) => return action_error(timeline_arguments_error()),
            },
        };
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        let media_id = match MediaId::from_str(&media_id) {
            Ok(media_id) => media_id,
            Err(error) => return invalid_timeline_id("INVALID_MEDIA_ID", error.to_string()),
        };
        let timeline_start =
            match RationalTime::new(timeline_start_numerator, timeline_start_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        let source_start = match RationalTime::new(source_start_numerator, source_start_denominator)
        {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        let duration = match RationalTime::new(duration_numerator, duration_denominator) {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        let source_range = match TimeRange::new(source_start, duration) {
            Ok(range) => range,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::insert_timeline_clip_with_mode(
                    project_id,
                    project_instance_id,
                    revision,
                    ClipId::generate(),
                    track_id,
                    media_id,
                    timeline_start,
                    source_range,
                    edit_mode,
                    if edit_mode == or_core::TimelineEditMode::Overwrite {
                        Some(split_clip_id.unwrap_or_else(ClipId::generate))
                    } else {
                        None
                    },
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_timeline_text_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        content_kind: TimelineClipContentKindView,
        timeline_start_numerator: i64,
        timeline_start_denominator: u32,
        timeline_duration_numerator: i64,
        timeline_duration_denominator: u32,
        text: String,
        formatting: ProjectTextFormattingView,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        let timeline_start =
            match RationalTime::new(timeline_start_numerator, timeline_start_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        let timeline_duration =
            match RationalTime::new(timeline_duration_numerator, timeline_duration_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        let content = match text_clip_content(content_kind, text, formatting) {
            Ok(content) => content,
            Err(error) => return action_error(error),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::insert_timeline_clip_content(
                    project_id,
                    project_instance_id,
                    revision,
                    ClipId::generate(),
                    track_id,
                    timeline_start,
                    timeline_duration,
                    content,
                    ClipSettings::Visual(VisualSettings::default()),
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_timeline_text_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        content_kind: TimelineClipContentKindView,
        timeline_duration_numerator: i64,
        timeline_duration_denominator: u32,
        text: String,
        formatting: ProjectTextFormattingView,
    ) -> ProjectActionResult {
        let (_, _, _, track_kind, clip) = match self.find_timeline_clip_state(
            &project_id,
            &project_instance_id,
            expected_revision,
            &track_id,
            &clip_id,
        ) {
            Ok(found) => found,
            Err(error) => return action_error(error),
        };
        let content = match text_clip_content(content_kind, text, formatting) {
            Ok(content) => content,
            Err(error) => return action_error(error),
        };
        let matching_content = matches!(
            (&clip.content, content_kind),
            (ClipContent::Text { .. }, TimelineClipContentKindView::Text)
                | (
                    ClipContent::Caption { .. },
                    TimelineClipContentKindView::Caption
                )
        );
        let matching_track = matches!(
            (track_kind, content_kind),
            (TrackKind::Text, TimelineClipContentKindView::Text)
                | (TrackKind::Caption, TimelineClipContentKindView::Caption)
        );
        if !matching_content || !matching_track {
            return action_error(ProjectBridgeError {
                code: "TIMELINE_CLIP_CONTENT_MISMATCH".to_owned(),
                message: "The selected clip is not the requested text or caption type.".to_owned(),
            });
        }
        let timeline_duration =
            match RationalTime::new(timeline_duration_numerator, timeline_duration_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::update_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    timeline_duration,
                    content,
                    clip.settings,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn move_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        track_id: String,
        timeline_start_numerator: i64,
        timeline_start_denominator: u32,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        let timeline_start =
            match RationalTime::new(timeline_start_numerator, timeline_start_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::move_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    track_id,
                    timeline_start,
                )
            },
        )
    }

    /// Moves every listed clip on one track by the same delta, as one undo step.
    ///
    /// A group drag is one gesture: five separate moves would cost five undos
    /// and could stop half-applied, so the whole selection travels in a single
    /// command whose change carries the track before and after.
    pub fn move_timeline_clips(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_ids: Vec<String>,
        delta_numerator: i64,
        delta_denominator: u32,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        if clip_ids.is_empty() {
            return action_error(ProjectBridgeError {
                code: "INVALID_CLIP_IDS".to_owned(),
                message: "Select at least one clip to move.".to_owned(),
            });
        }
        let mut parsed = Vec::with_capacity(clip_ids.len());
        for clip_id in &clip_ids {
            match ClipId::from_str(clip_id) {
                Ok(clip_id) => parsed.push(clip_id),
                Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
            }
        }
        let delta = match RationalTime::new(delta_numerator, delta_denominator) {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::move_timeline_clips(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                    parsed,
                    delta,
                )
            },
        )
    }

    pub fn delete_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::delete_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trim_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        edge: TimelineTrimEdgeView,
        timeline_time_numerator: i64,
        timeline_time_denominator: u32,
        // Opt-in end-edge ripple; absent keeps the plain trim meaning exactly.
        ripple: Option<bool>,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let timeline_time =
            match RationalTime::new(timeline_time_numerator, timeline_time_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::trim_timeline_clip_with_ripple(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    match edge {
                        TimelineTrimEdgeView::Start => TimelineTrimEdge::Start,
                        TimelineTrimEdgeView::End => TimelineTrimEdge::End,
                    },
                    timeline_time,
                    ripple.unwrap_or(false),
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    /// Moves a track to another position in the track list.
    pub fn move_timeline_track(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        to_index: u32,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::move_timeline_track(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                    to_index as usize,
                )
            },
        )
    }

    /// Moves a clip's source window by the given delta without moving the clip.
    pub fn slip_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        delta_numerator: i64,
        delta_denominator: u32,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let delta = match RationalTime::new(delta_numerator, delta_denominator) {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::slip_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    delta,
                )
            },
        )
    }

    /// Moves the boundary between two adjacent clips by the given delta.
    pub fn roll_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        delta_numerator: i64,
        delta_denominator: u32,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let delta = match RationalTime::new(delta_numerator, delta_denominator) {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::roll_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    delta,
                )
            },
        )
    }

    /// Moves a clip in time by the given delta, letting its neighbours absorb it.
    pub fn slide_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        delta_numerator: i64,
        delta_denominator: u32,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let delta = match RationalTime::new(delta_numerator, delta_denominator) {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::slide_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    delta,
                )
            },
        )
    }

    pub fn split_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        timeline_time_numerator: i64,
        timeline_time_denominator: u32,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let timeline_time =
            match RationalTime::new(timeline_time_numerator, timeline_time_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        let new_clip_id = ClipId::generate();
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::split_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    new_clip_id,
                    timeline_time,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn duck_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        voice_track_id: String,
        duck_millidecibels: i32,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let voice_track_id = match TrackId::from_str(&voice_track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::duck_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                    clip_id,
                    voice_track_id,
                    duck_millidecibels,
                )
            },
        )
    }

    pub fn ripple_delete_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::ripple_delete_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_timeline_marker(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        timeline_time_numerator: i64,
        timeline_time_denominator: u32,
        label: String,
    ) -> ProjectActionResult {
        let timeline_time =
            match RationalTime::new(timeline_time_numerator, timeline_time_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::add_timeline_marker(
                    project_id,
                    project_instance_id,
                    revision,
                    MarkerId::generate(),
                    timeline_time,
                    label,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn move_timeline_marker(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        marker_id: String,
        timeline_time_numerator: i64,
        timeline_time_denominator: u32,
    ) -> ProjectActionResult {
        let marker_id = match MarkerId::from_str(&marker_id) {
            Ok(marker_id) => marker_id,
            Err(error) => return invalid_timeline_id("INVALID_MARKER_ID", error.to_string()),
        };
        let timeline_time =
            match RationalTime::new(timeline_time_numerator, timeline_time_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::move_timeline_marker(
                    project_id,
                    project_instance_id,
                    revision,
                    marker_id,
                    timeline_time,
                )
            },
        )
    }

    pub fn rename_timeline_marker(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        marker_id: String,
        label: String,
    ) -> ProjectActionResult {
        let marker_id = match MarkerId::from_str(&marker_id) {
            Ok(marker_id) => marker_id,
            Err(error) => return invalid_timeline_id("INVALID_MARKER_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::rename_timeline_marker(
                    project_id,
                    project_instance_id,
                    revision,
                    marker_id,
                    label,
                )
            },
        )
    }

    pub fn delete_timeline_marker(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        marker_id: String,
    ) -> ProjectActionResult {
        let marker_id = match MarkerId::from_str(&marker_id) {
            Ok(marker_id) => marker_id,
            Err(error) => return invalid_timeline_id("INVALID_MARKER_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::delete_timeline_marker(
                    project_id,
                    project_instance_id,
                    revision,
                    marker_id,
                )
            },
        )
    }

    /// Prepares media without holding the live-host lock, then dispatches with the
    /// identity and revision captured by the caller before probing started.
    pub fn import_media(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        path: String,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => return action_error(error),
            };
        let item = match prepare_import_source(&path) {
            Ok(item) => item,
            Err(error) => {
                let mut message = error.to_string();
                // A missing `moov` is the signature of a recording that is still
                // being written or a copy that was interrupted, not of a codec
                // the editor refuses. Say what to do instead of repeating the
                // backend's English reason alone.
                if let or_core::MediaImportError::Probe(probe) = &error
                    && probe
                        .diagnostic()
                        .is_some_and(|detail| detail.contains("moov atom not found"))
                {
                    message.push_str(
                        " The file may still be recording or was not fully copied; \
                         finish writing it and import again.",
                    );
                }
                return ProjectActionResult {
                    succeeded: false,
                    error_code: error.code_str().to_owned(),
                    message,
                    view: None,
                };
            }
        };
        self.dispatch_command(CommandEnvelope::add_media(
            project_id,
            project_instance_id,
            revision,
            item,
        ))
    }

    /// Reduces noise in one media file and writes the result to `destination`.
    ///
    /// This is a file-to-file media operation, not a project mutation: no project is
    /// opened, nothing in one changes, and the result is ordinary media the caller
    /// can import and compare. The noise floor is derived from the material when
    /// `noise_floor_db` is `None`, because a constant would leave the noise in one
    /// recording and eat the audio in another.
    pub fn clean_up_audio(
        &self,
        source: String,
        destination: String,
        noise_floor_db: Option<i32>,
        reduction_db: u32,
    ) -> MediaCleanupResultView {
        if reduction_db == 0 {
            return MediaCleanupResultView::failure(
                "INVALID_ARGUMENTS",
                "reduction cannot be zero",
            );
        }
        let source_path = std::path::PathBuf::from(&source);
        let destination_path = std::path::PathBuf::from(&destination);
        let noise_floor_db = match noise_floor_db {
            Some(value) => value,
            None => match or_core::measure_noise_floor(&source_path) {
                Ok(value) => value,
                Err(error) => {
                    return MediaCleanupResultView::failure(
                        error.code().as_str(),
                        format!(
                            "could not measure the noise floor: {}",
                            error.code().as_str()
                        ),
                    );
                }
            },
        };
        match or_core::clean_audio_file(
            &source_path,
            &destination_path,
            noise_floor_db,
            reduction_db,
        ) {
            Ok(result) => MediaCleanupResultView {
                succeeded: true,
                error_code: String::new(),
                message: String::new(),
                destination: result.destination.display().to_string(),
                noise_floor_db: result.noise_floor_db,
                reduction_db: result.reduction_db,
                bytes_written: result.bytes_written,
            },
            Err(error) => MediaCleanupResultView::failure(
                error.code().as_str(),
                format!("noise reduction failed: {}", error.code().as_str()),
            ),
        }
    }

    /// Re-probes a replacement source without changing the target media identity.
    pub fn relink_media(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        media_id: String,
        path: String,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => return action_error(error),
            };
        let media_id = match MediaId::from_str(&media_id) {
            Ok(media_id) => media_id,
            Err(error) => {
                return action_error(ProjectBridgeError {
                    code: "INVALID_MEDIA_ID".to_owned(),
                    message: error.to_string(),
                });
            }
        };
        let prepared = match prepare_import_source(&path) {
            Ok(item) => item,
            Err(error) => {
                return ProjectActionResult {
                    succeeded: false,
                    error_code: error.code_str().to_owned(),
                    message: error.to_string(),
                    view: None,
                };
            }
        };
        let item = match MediaItem::new(
            media_id,
            prepared.source().clone(),
            prepared.metadata().clone(),
        ) {
            Ok(item) => item,
            Err(error) => {
                return action_error(ProjectBridgeError {
                    code: "INVALID_MEDIA_METADATA".to_owned(),
                    message: error.to_string(),
                });
            }
        };
        self.dispatch_command(CommandEnvelope::relink_media(
            project_id,
            project_instance_id,
            revision,
            item,
        ))
    }

    pub fn remove_media(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        media_id: String,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => return action_error(error),
            };
        let media_id = match MediaId::from_str(&media_id) {
            Ok(media_id) => media_id,
            Err(error) => {
                return action_error(ProjectBridgeError {
                    code: "INVALID_MEDIA_ID".to_owned(),
                    message: error.to_string(),
                });
            }
        };
        self.dispatch_command(CommandEnvelope::remove_media(
            project_id,
            project_instance_id,
            revision,
            media_id,
        ))
    }

    pub fn rename(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        name: String,
    ) -> ProjectActionResult {
        self.command(
            project_id,
            project_instance_id,
            expected_revision,
            "project.rename",
            Some(name),
        )
    }

    pub fn undo(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
    ) -> ProjectActionResult {
        self.command(
            project_id,
            project_instance_id,
            expected_revision,
            "history.undo",
            None,
        )
    }

    pub fn redo(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
    ) -> ProjectActionResult {
        self.command(
            project_id,
            project_instance_id,
            expected_revision,
            "history.redo",
            None,
        )
    }

    pub fn save(&self) -> ProjectActionResult {
        match self.host.save() {
            Ok(_) => ProjectActionResult {
                succeeded: true,
                error_code: String::new(),
                message: "Project saved.".to_owned(),
                view: self.project_view().ok(),
            },
            Err(error) => action_error(host_error(error)),
        }
    }

    pub fn autosave_checkpoint(&self) -> ProjectActionResult {
        match self.host.autosave_checkpoint() {
            Ok(written) => ProjectActionResult {
                succeeded: true,
                error_code: String::new(),
                message: if written {
                    "Recovery checkpoint updated.".to_owned()
                } else {
                    "No unsaved recovery update was needed.".to_owned()
                },
                view: None,
            },
            Err(error) => action_error(host_error(error)),
        }
    }

    pub fn start_export(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        destination: String,
        quality: Option<String>,
    ) -> ProjectExportJobView {
        // An optional argument keeps every existing generated caller compiling and
        // gives an old client the quality preference rather than the fastest mode.
        let quality = match quality
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("speed") => or_core::ExportQuality::Speed,
            _ => or_core::ExportQuality::Quality,
        };
        let profile = std::path::Path::new(&destination)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .map(str::to_ascii_lowercase)
            .as_deref()
            .and_then(|extension| match extension {
                "mkv" => Some(or_core::ExportProfile::MatroskaFfv1PcmS16le),
                "webm" => Some(or_core::ExportProfile::WebmVp9Opus),
                "mp4" => Some(or_core::ExportProfile::Mp4H264Aac),
                _ => None,
            });
        let Some(profile) = profile else {
            return export_job_view(ExportResponse::failure(
                "INVALID_EXPORT_PROFILE",
                "choose a destination ending in .mp4, .mkv, or .webm",
            ));
        };
        let (project_id, project_instance_id, expected_project_revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => {
                    return export_job_view(ExportResponse::failure(error.code, error.message));
                }
            };
        self.handle_export_application_request(ExportRequest::Start {
            project_id,
            project_instance_id,
            expected_project_revision,
            destination,
            profile,
            quality,
        })
    }

    pub fn export_status(
        &self,
        project_id: String,
        project_instance_id: String,
        job_id: String,
    ) -> ProjectExportJobView {
        let (project_id, project_instance_id) =
            match parse_export_identity(&project_id, &project_instance_id) {
                Ok(identity) => identity,
                Err(error) => {
                    return export_job_view(ExportResponse::failure(error.code, error.message));
                }
            };
        let job_id = match JobId::from_str(&job_id) {
            Ok(job_id) => job_id,
            Err(error) => {
                return export_job_view(ExportResponse::failure(
                    "INVALID_EXPORT_JOB_ID",
                    error.to_string(),
                ));
            }
        };
        self.handle_export_application_request(ExportRequest::Status {
            project_id,
            project_instance_id,
            job_id,
        })
    }

    pub fn cancel_export(
        &self,
        project_id: String,
        project_instance_id: String,
        job_id: String,
    ) -> ProjectExportJobView {
        let (project_id, project_instance_id) =
            match parse_export_identity(&project_id, &project_instance_id) {
                Ok(identity) => identity,
                Err(error) => {
                    return export_job_view(ExportResponse::failure(error.code, error.message));
                }
            };
        let job_id = match JobId::from_str(&job_id) {
            Ok(job_id) => job_id,
            Err(error) => {
                return export_job_view(ExportResponse::failure(
                    "INVALID_EXPORT_JOB_ID",
                    error.to_string(),
                ));
            }
        };
        self.handle_export_application_request(ExportRequest::Cancel {
            project_id,
            project_instance_id,
            job_id,
        })
    }

    fn handle_export_application_request(&self, request: ExportRequest) -> ProjectExportJobView {
        match self
            .host
            .handle_application_request(ApplicationRequest::Export(request))
        {
            Ok(ApplicationResponse::Export(response)) => export_job_view(response),
            Ok(ApplicationResponse::Error(error)) => export_job_view(ExportResponse::failure(
                "EXPORT_REQUEST_FAILED",
                error.to_string(),
            )),
            Ok(_) => export_job_view(ExportResponse::failure(
                "UNEXPECTED_EXPORT_RESPONSE",
                "project host returned an unexpected export response",
            )),
            Err(error) => export_job_view(ExportResponse::failure(
                "EXPORT_HOST_UNAVAILABLE",
                error.to_string(),
            )),
        }
    }

    pub fn close(
        &mut self,
        discard_unsaved: bool,
        discard_recovery_checkpoint: bool,
    ) -> ProjectActionResult {
        match self
            .host
            .shutdown_with_recovery_policy(discard_unsaved, discard_recovery_checkpoint)
        {
            Ok(()) => {
                let preview_shutdown = self.preview_runtime.shutdown();
                if let Some(service) = self.media_artifact_service.take() {
                    service.shutdown();
                }
                if let Err(error) = preview_shutdown {
                    return ProjectActionResult {
                        succeeded: false,
                        error_code: error.code.to_owned(),
                        message: error.message,
                        view: None,
                    };
                }
                ProjectActionResult {
                    succeeded: true,
                    error_code: String::new(),
                    message: String::new(),
                    view: None,
                }
            }
            Err(error) => action_error(host_error(error)),
        }
    }

    pub fn subscribe_events(
        &self,
        sink: StreamSink<ProjectHostEventView>,
    ) -> Result<(), ProjectBridgeError> {
        let receiver = self.host.subscribe_events().map_err(host_error)?;
        thread::Builder::new()
            .name("or-flutter-project-events".to_owned())
            .spawn(move || forward_events(receiver, sink))
            .map_err(|error| ProjectBridgeError {
                code: "EVENT_SUBSCRIPTION_FAILED".to_owned(),
                message: error.to_string(),
            })?;
        Ok(())
    }

    fn find_visual_clip(
        &self,
        project_id: &str,
        project_instance_id: &str,
        expected_revision: u64,
        track_id: &str,
        clip_id: &str,
    ) -> Result<
        (
            ProjectId,
            ProjectInstanceId,
            ProjectRevision,
            TimelineClipState,
        ),
        ProjectBridgeError,
    > {
        let (project_id, project_instance_id, revision, track_kind, clip) = self
            .find_timeline_clip_state(
                project_id,
                project_instance_id,
                expected_revision,
                track_id,
                clip_id,
            )?;
        if !track_kind.is_visual() {
            return Err(unsupported_visual_settings());
        }
        Ok((project_id, project_instance_id, revision, clip))
    }

    fn find_audio_clip(
        &self,
        project_id: &str,
        project_instance_id: &str,
        expected_revision: u64,
        track_id: &str,
        clip_id: &str,
    ) -> Result<
        (
            ProjectId,
            ProjectInstanceId,
            ProjectRevision,
            TimelineClipState,
        ),
        ProjectBridgeError,
    > {
        let (project_id, project_instance_id, revision, track_kind, clip) = self
            .find_timeline_clip_state(
                project_id,
                project_instance_id,
                expected_revision,
                track_id,
                clip_id,
            )?;
        if track_kind != TrackKind::Audio {
            return Err(unsupported_audio_settings());
        }
        Ok((project_id, project_instance_id, revision, clip))
    }

    fn find_timeline_clip_state(
        &self,
        project_id: &str,
        project_instance_id: &str,
        expected_revision: u64,
        track_id: &str,
        clip_id: &str,
    ) -> Result<
        (
            ProjectId,
            ProjectInstanceId,
            ProjectRevision,
            TrackKind,
            TimelineClipState,
        ),
        ProjectBridgeError,
    > {
        let (project_id, project_instance_id, revision) =
            parse_session_identity(project_id, project_instance_id, expected_revision)?;
        let track_id = TrackId::from_str(track_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_TRACK_ID".to_owned(),
            message: error.to_string(),
        })?;
        let clip_id = ClipId::from_str(clip_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_CLIP_ID".to_owned(),
            message: error.to_string(),
        })?;

        let tracks = self.query(QueryEnvelope::timeline_tracks_v2(
            project_id,
            project_instance_id,
        ))?;
        ensure_visual_settings_snapshot(&tracks, project_id, project_instance_id, revision)?;
        let track = tracks
            .timeline_tracks_v2
            .as_ref()
            .and_then(|items| items.iter().find(|item| item.track_id == track_id))
            .ok_or_else(|| ProjectBridgeError {
                code: "TIMELINE_TRACK_NOT_FOUND".to_owned(),
                message: "The selected timeline track no longer exists.".to_owned(),
            })?;
        let track_kind = track.kind;

        let mut offset = 0;
        loop {
            let result = self.query(QueryEnvelope::timeline_clips_v2(
                project_id,
                project_instance_id,
                track_id,
                offset,
                or_core::MAX_TIMELINE_CLIP_PAGE_SIZE,
            ))?;
            ensure_visual_settings_snapshot(&result, project_id, project_instance_id, revision)?;
            let page: &TimelineClipPageV2 = result
                .timeline_clip_page_v2
                .as_ref()
                .ok_or_else(unexpected_response_error)?;
            if let Some(clip) = page.items.iter().find(|item| item.clip_id == clip_id) {
                return Ok((
                    project_id,
                    project_instance_id,
                    revision,
                    track_kind,
                    clip.clone(),
                ));
            }
            let Some(next_offset) = page.next_offset else {
                break;
            };
            if next_offset <= offset {
                return Err(timeline_query_arguments_error());
            }
            offset = next_offset;
        }
        Err(ProjectBridgeError {
            code: "TIMELINE_CLIP_NOT_FOUND".to_owned(),
            message: "The selected timeline clip no longer exists.".to_owned(),
        })
    }

    fn command(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        command_id: &str,
        name: Option<String>,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => return action_error(error),
            };
        let envelope = match (command_id, name) {
            ("project.rename", Some(name)) => {
                CommandEnvelope::rename_project(project_id, project_instance_id, revision, name)
            }
            ("history.undo", None) => {
                CommandEnvelope::undo(project_id, project_instance_id, revision)
            }
            ("history.redo", None) => {
                CommandEnvelope::redo(project_id, project_instance_id, revision)
            }
            _ => {
                return ProjectActionResult {
                    succeeded: false,
                    error_code: "INVALID_COMMAND".to_owned(),
                    message: "The requested project action is not supported.".to_owned(),
                    view: None,
                };
            }
        };
        self.dispatch_command(envelope)
    }

    fn timeline_command(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        build: impl FnOnce(ProjectId, ProjectInstanceId, ProjectRevision) -> CommandEnvelope,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => return action_error(error),
            };
        self.dispatch_command(build(project_id, project_instance_id, revision))
    }

    fn query(&self, request: QueryEnvelope) -> Result<QueryResult, ProjectBridgeError> {
        match self
            .host
            .handle_application_request(ApplicationRequest::Query(request))
        {
            Ok(ApplicationResponse::Query(result)) => Ok(result),
            Ok(ApplicationResponse::Error(error)) => Err(operation_bridge_error(error)),
            Ok(_) => Err(unexpected_response_error()),
            Err(error) => Err(host_error(error)),
        }
    }

    fn dispatch_command(&self, envelope: CommandEnvelope) -> ProjectActionResult {
        match self
            .host
            .handle_application_request(ApplicationRequest::Command(envelope))
        {
            Ok(ApplicationResponse::Command(_) | ApplicationResponse::Transaction(_)) => {
                ProjectActionResult {
                    succeeded: true,
                    error_code: String::new(),
                    message: String::new(),
                    view: self.project_view().ok(),
                }
            }
            Ok(ApplicationResponse::Error(error)) => action_operation_error(error),
            Ok(_) => ProjectActionResult {
                succeeded: false,
                error_code: "UNEXPECTED_RESPONSE".to_owned(),
                message: "The project host returned an unexpected response.".to_owned(),
                view: None,
            },
            Err(error) => action_error(host_error(error)),
        }
    }

    fn project_view(&self) -> Result<ProjectView, LiveProjectHostError> {
        let result = self.host.describe()?;
        Ok(view_from_query(
            &result,
            self.host.is_dirty()?,
            self.host.descriptor_path(),
        ))
    }
}

fn rational_time_millis(value: RationalTime) -> Result<u64, ProjectBridgeError> {
    let scaled = i128::from(value.numerator()) * 1_000;
    let denominator = i128::from(value.denominator());
    if scaled < 0 || scaled % denominator != 0 {
        return Err(caption_time_error());
    }
    u64::try_from(scaled / denominator).map_err(|_| caption_time_error())
}

fn caption_time_error() -> ProjectBridgeError {
    ProjectBridgeError {
        code: "CAPTION_TIME_OVERFLOW".to_owned(),
        message: "A caption time is outside the supported project range.".to_owned(),
    }
}

fn prepare_import_source(source: &str) -> Result<MediaItem, or_core::MediaImportError> {
    #[cfg(target_os = "android")]
    if source.starts_with("content://") {
        let source = MediaSourceRef::android_saf_document_uri(source)
            .map_err(|error| or_core::MediaImportError::InvalidSourceUri(error))?;
        let capability =
            capability_for(&source).ok_or(or_core::MediaImportError::SourceUnavailable)?;
        let probe_output = or_media::probe_seekable_media(&capability)
            .map_err(|_| or_core::MediaImportError::SourceUnavailable)?;
        return prepare_media_import_from_probe(source, &probe_output, capability.len());
    }
    let path = Path::new(source);
    match prepare_media_import(path) {
        Ok(item) => Ok(item),
        // The packaged helper is a separate executable. If it cannot start or
        // exits without reading the file, fall back to the libavformat this
        // process already links for playback and export. A file the helper
        // rejects on its import matrix is not retried: the fallback would refuse
        // it for the same reason and the helper's diagnostic is the better one.
        Err(error) => {
            let helper_failed = matches!(
                &error,
                or_core::MediaImportError::Probe(probe)
                    if matches!(
                        probe.code(),
                        or_core::MediaProbeErrorCode::ProbeFailed
                            | or_core::MediaProbeErrorCode::ProbeBackendUnavailable
                            | or_core::MediaProbeErrorCode::InvalidProbeOutput
                    )
            );
            if !helper_failed {
                return Err(error);
            }
            match or_media::probe_media_path(path) {
                Ok(probe_output) => {
                    prepare_media_import_with_probe(path, &probe_output).map_err(|_| error)
                }
                Err(_) => Err(error),
            }
        }
    }
}

impl MediaArtifactRequestView {
    pub(crate) fn failed(
        media_id: String,
        kind: MediaArtifactKindView,
        error_code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            media_id,
            kind,
            cache_key: None,
            job_id: None,
            state: MediaArtifactRequestStateView::Failed,
            error_code: Some(error_code.into()),
            message: Some(message.into()),
        }
    }

    pub(crate) fn from_request(
        media_id: String,
        kind: MediaArtifactKindView,
        request: MediaArtifactRequest,
    ) -> Self {
        Self {
            media_id,
            kind,
            cache_key: request.cache_key.map(|cache_key| cache_key.to_hex()),
            job_id: request.job_id.map(|job_id| job_id.to_string()),
            state: match request.state {
                MediaArtifactRequestState::Ready => MediaArtifactRequestStateView::Ready,
                MediaArtifactRequestState::Queued => MediaArtifactRequestStateView::Queued,
                MediaArtifactRequestState::Running => MediaArtifactRequestStateView::Running,
                MediaArtifactRequestState::NotApplicable => {
                    MediaArtifactRequestStateView::NotApplicable
                }
            },
            error_code: None,
            message: None,
        }
    }
}

pub(crate) fn cache_artifact_kind(kind: MediaArtifactKindView) -> CacheArtifactKind {
    match kind {
        MediaArtifactKindView::Thumbnail => CacheArtifactKind::Thumbnail,
        MediaArtifactKindView::Waveform => CacheArtifactKind::Waveform,
    }
}

pub(crate) fn cache_unavailable_error() -> ProjectBridgeError {
    ProjectBridgeError {
        code: "CACHE_UNAVAILABLE".to_owned(),
        message: "media preview cache is unavailable".to_owned(),
    }
}

fn parse_session_identity(
    project_id: &str,
    project_instance_id: &str,
    expected_revision: u64,
) -> Result<(ProjectId, ProjectInstanceId, ProjectRevision), ProjectBridgeError> {
    let project_id = ProjectId::from_str(project_id).map_err(|error| ProjectBridgeError {
        code: "INVALID_PROJECT_ID".to_owned(),
        message: error.to_string(),
    })?;
    let project_instance_id =
        ProjectInstanceId::from_str(project_instance_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_PROJECT_INSTANCE_ID".to_owned(),
            message: error.to_string(),
        })?;
    Ok((
        project_id,
        project_instance_id,
        ProjectRevision::new(expected_revision),
    ))
}

fn read_bounded_caption_file(path: &Path) -> Result<Vec<u8>, ProjectBridgeError> {
    let read_error = || ProjectBridgeError {
        code: "CAPTION_FILE_READ_FAILED".to_owned(),
        message: "The selected subtitle file could not be read.".to_owned(),
    };
    let file = File::open(path).map_err(|_| read_error())?;
    let max_bytes = or_core::MAX_CAPTION_FILE_BYTES;
    if file.metadata().map_err(|_| read_error())?.len() > max_bytes as u64 {
        return Err(ProjectBridgeError {
            code: "CAPTION_FILE_TOO_LARGE".to_owned(),
            message: "The subtitle file exceeds the 8 MiB import limit.".to_owned(),
        });
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve(max_bytes.min(file.metadata().map_err(|_| read_error())?.len() as usize))
        .map_err(|_| ProjectBridgeError {
            code: "CAPTION_FILE_READ_FAILED".to_owned(),
            message: "There is not enough memory to read the selected subtitle file.".to_owned(),
        })?;
    file.take(max_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| read_error())?;
    if bytes.len() > max_bytes {
        return Err(ProjectBridgeError {
            code: "CAPTION_FILE_TOO_LARGE".to_owned(),
            message: "The subtitle file exceeds the 8 MiB import limit.".to_owned(),
        });
    }
    Ok(bytes)
}

fn revision_conflict_error() -> ProjectBridgeError {
    ProjectBridgeError {
        code: "REVISION_CONFLICT".to_owned(),
        message: "The project changed while captions were being exported.".to_owned(),
    }
}

fn write_caption_file_atomically(path: &Path, bytes: &[u8]) -> Result<(), ProjectBridgeError> {
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| ProjectBridgeError {
            code: "CAPTION_EXPORT_PATH_INVALID".to_owned(),
            message: "Choose a valid caption export location.".to_owned(),
        })?;
    let name = path.file_name().ok_or_else(|| ProjectBridgeError {
        code: "CAPTION_EXPORT_PATH_INVALID".to_owned(),
        message: "Choose a valid caption export location.".to_owned(),
    })?;
    let temporary = parent.join(format!(
        ".{}.or-caption-{}-{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed),
    ));
    let operation = || -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    };
    if let Err(error) = operation() {
        let _ = fs::remove_file(&temporary);
        return Err(ProjectBridgeError {
            code: "CAPTION_EXPORT_FAILED".to_owned(),
            message: format!("The caption file could not be saved: {error}"),
        });
    }
    Ok(())
}

fn media_item_view(item: &MediaItem) -> ProjectMediaItemView {
    let metadata = item.metadata();
    let video_details = metadata.streams().iter().find_map(|stream| match stream {
        MediaStreamMetadata::Video(video) => {
            let codec = video
                .codec_name()
                .map(|codec| format!(" {codec}"))
                .unwrap_or_default();
            Some(format!("{}×{}{}", video.width(), video.height(), codec))
        }
        _ => None,
    });
    let audio_details = metadata.streams().iter().find_map(|stream| match stream {
        MediaStreamMetadata::Audio(audio) => {
            let mut details = Vec::new();
            if let Some(rate) = audio.sample_rate() {
                details.push(format!("{rate} Hz"));
            }
            if let Some(channels) = audio.channels() {
                details.push(format!("{channels} ch"));
            }
            if let Some(layout) = audio.channel_layout() {
                details.push(layout.to_owned());
            }
            if let Some(codec) = audio.codec_name() {
                details.push(codec.to_owned());
            }
            Some(if details.is_empty() {
                "Audio".to_owned()
            } else {
                details.join(" ")
            })
        }
        _ => None,
    });
    ProjectMediaItemView {
        media_id: item.id().to_string(),
        source_uri: item.source().uri().to_owned(),
        format_names: metadata.format_names().to_vec(),
        duration: metadata.duration().map(|duration| {
            let value = duration.numerator();
            let denominator = duration.denominator();
            if denominator == 1 {
                format!("{value} s")
            } else {
                format!("{value}/{denominator} s")
            }
        }),
        video_details,
        audio_details,
        container_duration: metadata.duration().map(rational_time_view),
        first_video_duration: metadata
            .streams()
            .iter()
            .find_map(|stream| match stream {
                MediaStreamMetadata::Video(video) => Some(video.duration()),
                _ => None,
            })
            .flatten()
            .map(rational_time_view),
        first_audio_duration: metadata
            .streams()
            .iter()
            .find_map(|stream| match stream {
                MediaStreamMetadata::Audio(audio) => Some(audio.duration()),
                _ => None,
            })
            .flatten()
            .map(rational_time_view),
    }
}

fn rational_time_view(time: RationalTime) -> RationalTimeView {
    RationalTimeView {
        numerator: time.numerator(),
        denominator: time.denominator(),
    }
}

fn rational_rate_view(rate: RationalRate) -> RationalRateView {
    RationalRateView {
        numerator: rate.numerator(),
        denominator: rate.denominator(),
    }
}

fn preview_state_view(snapshot: PreviewSnapshot) -> ProjectPreviewStateView {
    ProjectPreviewStateView {
        position: rational_time_view(snapshot.playback.position),
        presented_time: snapshot.playback.presented_time.map(rational_time_view),
        sequence_frame_rate: snapshot.playback.frame_rate.map(rational_rate_view),
        content_end: snapshot.playback.content_end.map(rational_time_view),
        playing: snapshot.playback.playing,
        generation: snapshot.playback.generation,
        frame_sequence: snapshot.playback.frame_sequence,
        width: snapshot.width,
        height: snapshot.height,
        error_code: snapshot.error_code,
        error_message: snapshot.error_message,
    }
}

fn parse_export_identity(
    project_id: &str,
    project_instance_id: &str,
) -> Result<(ProjectId, ProjectInstanceId), ProjectBridgeError> {
    let project_id = ProjectId::from_str(project_id).map_err(|error| ProjectBridgeError {
        code: "INVALID_PROJECT_ID".to_owned(),
        message: error.to_string(),
    })?;
    let project_instance_id =
        ProjectInstanceId::from_str(project_instance_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_PROJECT_INSTANCE_ID".to_owned(),
            message: error.to_string(),
        })?;
    Ok((project_id, project_instance_id))
}

fn export_job_view(response: ExportResponse) -> ProjectExportJobView {
    let job = response.job;
    ProjectExportJobView {
        succeeded: response.succeeded,
        error_code: response.error_code,
        message: response.message,
        job_id: job
            .as_ref()
            .map_or_else(String::new, |job| job.id.to_string()),
        state: job.as_ref().map_or_else(String::new, |job| {
            match job.state {
                JobState::Queued => "queued",
                JobState::Running => "running",
                JobState::Succeeded => "succeeded",
                JobState::Failed => "failed",
                JobState::Cancelled => "cancelled",
            }
            .to_owned()
        }),
        progress_completed: job
            .as_ref()
            .and_then(|job: &JobSnapshot| job.progress.map(|progress| progress.completed)),
        progress_total: job
            .as_ref()
            .and_then(|job: &JobSnapshot| job.progress.map(|progress| progress.total)),
        failure_message: job.and_then(|job| job.failure_message),
    }
}

fn preview_bridge_error(error: PreviewError) -> ProjectBridgeError {
    ProjectBridgeError {
        code: error.code,
        message: error.message,
    }
}

fn timeline_track_view(track: &TimelineTrackSummaryV2) -> ProjectTimelineTrackView {
    ProjectTimelineTrackView {
        track_id: track.track_id.to_string(),
        kind: match track.kind {
            TrackKind::Video => TimelineTrackKindView::Video,
            TrackKind::Audio => TimelineTrackKindView::Audio,
            TrackKind::Text => TimelineTrackKindView::Text,
            TrackKind::Caption => TimelineTrackKindView::Caption,
        },
        state: ProjectTimelineTrackStateView {
            locked: track.state.locked(),
            visible: track.state.visible(),
            muted: track.state.muted(),
            solo: track.state.solo(),
        },
        clip_count: u64::try_from(track.clip_count).expect("bounded count fits in u64"),
    }
}

fn timeline_clip_page_view(
    result: &QueryResult,
    page: &TimelineClipPageV2,
) -> ProjectTimelineClipPageView {
    ProjectTimelineClipPageView {
        project_id: result.summary.project_id.to_string(),
        project_instance_id: result.summary.project_instance_id.to_string(),
        project_revision: result.summary.project_revision.value(),
        track_id: page.track_id.to_string(),
        items: page.items.iter().map(timeline_clip_view).collect(),
        total_count: u64::try_from(page.total_count).expect("bounded count fits in u64"),
        offset: u64::try_from(page.offset).expect("bounded offset fits in u64"),
        limit: u64::try_from(page.limit).expect("bounded limit fits in u64"),
        next_offset: page
            .next_offset
            .map(|value| u64::try_from(value).expect("bounded offset fits in u64")),
    }
}

fn timeline_clip_view(clip: &TimelineClipState) -> ProjectTimelineClipView {
    let (content_kind, media_id, text, formatting, source_start) = match &clip.content {
        ClipContent::Media {
            media_id,
            source_range,
            playback_rate: _,
            reversed: _,
            frozen: _,
        } => (
            TimelineClipContentKindView::Media,
            Some(media_id.to_string()),
            None,
            None,
            Some(rational_time_view(source_range.start())),
        ),
        ClipContent::Text { text, formatting } => (
            TimelineClipContentKindView::Text,
            None,
            Some(text.clone()),
            Some(text_formatting_view(*formatting)),
            None,
        ),
        ClipContent::Caption { text, formatting } => (
            TimelineClipContentKindView::Caption,
            None,
            Some(text.clone()),
            Some(text_formatting_view(*formatting)),
            None,
        ),
    };
    ProjectTimelineClipView {
        clip_id: clip.clip_id.to_string(),
        content_kind,
        playback_rate: rational_rate_view(clip.content.playback_rate()),
        reversed: clip.content.reversed(),
        frozen: clip.content.frozen(),
        media_id,
        text,
        formatting,
        timeline_start: rational_time_view(clip.timeline_start),
        timeline_duration: rational_time_view(clip.timeline_duration),
        source_start,
    }
}

fn text_formatting_view(formatting: TextFormatting) -> ProjectTextFormattingView {
    ProjectTextFormattingView {
        font: match formatting.font {
            FontIdentity::BundledInter => FontIdentityView::BundledInter,
            FontIdentity::BeVietnamPro => FontIdentityView::BeVietnamPro,
        },
        size_milli_points: formatting.size_milli_points,
        weight: match formatting.weight {
            TextWeight::Regular => TextWeightView::Regular,
            TextWeight::Medium => TextWeightView::Medium,
            TextWeight::Semibold => TextWeightView::Semibold,
            TextWeight::Bold => TextWeightView::Bold,
        },
        alignment: match formatting.alignment {
            TextAlignment::Start => TextAlignmentView::Start,
            TextAlignment::Center => TextAlignmentView::Center,
            TextAlignment::End => TextAlignmentView::End,
        },
        color: ProjectTextColorView {
            red: formatting.color.red,
            green: formatting.color.green,
            blue: formatting.color.blue,
            alpha: formatting.color.alpha,
        },
        anchor: match formatting.anchor {
            or_core::TextVerticalAnchor::Top => TextVerticalAnchorView::Top,
            or_core::TextVerticalAnchor::Center => TextVerticalAnchorView::Center,
            or_core::TextVerticalAnchor::Bottom => TextVerticalAnchorView::Bottom,
        },
        offset_x_basis_points: formatting.offset_x_basis_points,
        offset_y_basis_points: formatting.offset_y_basis_points,
        background: formatting
            .background
            .map(|background| ProjectTextBackgroundView {
                color: ProjectTextColorView {
                    red: background.color.red,
                    green: background.color.green,
                    blue: background.color.blue,
                    alpha: background.color.alpha,
                },
                padding_milli_points: background.padding_milli_points,
            }),
        opacity_basis_points: formatting.opacity_basis_points,
    }
}

fn text_formatting_from_view(view: ProjectTextFormattingView) -> TextFormatting {
    TextFormatting {
        font: match view.font {
            FontIdentityView::BundledInter => FontIdentity::BundledInter,
            FontIdentityView::BeVietnamPro => FontIdentity::BeVietnamPro,
        },
        size_milli_points: view.size_milli_points,
        weight: match view.weight {
            TextWeightView::Regular => TextWeight::Regular,
            TextWeightView::Medium => TextWeight::Medium,
            TextWeightView::Semibold => TextWeight::Semibold,
            TextWeightView::Bold => TextWeight::Bold,
        },
        alignment: match view.alignment {
            TextAlignmentView::Start => TextAlignment::Start,
            TextAlignmentView::Center => TextAlignment::Center,
            TextAlignmentView::End => TextAlignment::End,
        },
        color: TextColor {
            red: view.color.red,
            green: view.color.green,
            blue: view.color.blue,
            alpha: view.color.alpha,
        },
        anchor: match view.anchor {
            TextVerticalAnchorView::Top => or_core::TextVerticalAnchor::Top,
            TextVerticalAnchorView::Center => or_core::TextVerticalAnchor::Center,
            TextVerticalAnchorView::Bottom => or_core::TextVerticalAnchor::Bottom,
        },
        offset_x_basis_points: view.offset_x_basis_points,
        offset_y_basis_points: view.offset_y_basis_points,
        background: view.background.map(|background| or_core::TextBackground {
            color: TextColor {
                red: background.color.red,
                green: background.color.green,
                blue: background.color.blue,
                alpha: background.color.alpha,
            },
            padding_milli_points: background.padding_milli_points,
        }),
        opacity_basis_points: view.opacity_basis_points,
    }
}

fn text_clip_content(
    kind: TimelineClipContentKindView,
    text: String,
    formatting: ProjectTextFormattingView,
) -> Result<ClipContent, ProjectBridgeError> {
    let formatting = text_formatting_from_view(formatting);
    match kind {
        TimelineClipContentKindView::Text => Ok(ClipContent::Text { text, formatting }),
        TimelineClipContentKindView::Caption => Ok(ClipContent::Caption { text, formatting }),
        TimelineClipContentKindView::Media => Err(ProjectBridgeError {
            code: "INVALID_TIMELINE_TEXT_CONTENT".to_owned(),
            message: "Only text and caption content can use the text editor.".to_owned(),
        }),
    }
}

fn visual_settings_view(settings: &VisualSettings) -> ProjectTimelineVisualSettingsView {
    let mut brightness_amount_milli = 0;
    let mut contrast_amount_milli = 1_000;
    let mut saturation_amount_milli = 1_000;
    let mut gaussian_blur_radius_milli = 0;
    for effect in &settings.effects {
        match *effect {
            EffectReference::Brightness { amount_milli } => {
                brightness_amount_milli = amount_milli;
            }
            EffectReference::Contrast { amount_milli } => {
                contrast_amount_milli = amount_milli;
            }
            EffectReference::Saturation { amount_milli } => {
                saturation_amount_milli = amount_milli;
            }
            EffectReference::GaussianBlur { radius_milli } => {
                gaussian_blur_radius_milli = radius_milli;
            }
        }
    }
    ProjectTimelineVisualSettingsView {
        x_milli_canvas: settings.transform.x_milli_canvas,
        y_milli_canvas: settings.transform.y_milli_canvas,
        scale_x_milli: settings.transform.scale_x_milli,
        scale_y_milli: settings.transform.scale_y_milli,
        rotation_milli_degrees: settings.transform.rotation_milli_degrees,
        anchor_x_basis_points: settings.transform.anchor_x_basis_points,
        anchor_y_basis_points: settings.transform.anchor_y_basis_points,
        crop_left_basis_points: settings.crop.left_basis_points,
        crop_top_basis_points: settings.crop.top_basis_points,
        crop_right_basis_points: settings.crop.right_basis_points,
        crop_bottom_basis_points: settings.crop.bottom_basis_points,
        opacity_basis_points: settings.opacity.basis_points,
        brightness_amount_milli,
        contrast_amount_milli,
        saturation_amount_milli,
        gaussian_blur_radius_milli,
        update_brightness: false,
        update_contrast: false,
        update_saturation: false,
        update_gaussian_blur: false,
        transition_in_kind: transition_kind_code(settings.transition_in),
        transition_in_duration: settings
            .transition_in
            .map(|transition| rational_time_view(transition.duration))
            .unwrap_or_else(|| rational_time_view(RationalTime::ZERO)),
        transition_out_kind: transition_kind_code(settings.transition_out),
        transition_out_duration: settings
            .transition_out
            .map(|transition| rational_time_view(transition.duration))
            .unwrap_or_else(|| rational_time_view(RationalTime::ZERO)),
        update_transition_in: false,
        update_transition_out: false,
    }
}

fn effects_from_view(
    existing: &[EffectReference],
    settings: &ProjectTimelineVisualSettingsView,
) -> Vec<EffectReference> {
    let mut effects = Vec::with_capacity(existing.len().saturating_add(4));
    for effect in existing {
        let kind = effect_kind_code(*effect);
        if !effect_kind_modified(settings, kind) {
            effects.push(*effect);
        } else if let Some(updated) = effect_from_view(settings, kind) {
            effects.push(updated);
        }
    }
    for kind in 1..=4 {
        if effect_kind_modified(settings, kind)
            && !existing
                .iter()
                .any(|effect| effect_kind_code(*effect) == kind)
            && let Some(effect) = effect_from_view(settings, kind)
        {
            effects.push(effect);
        }
    }
    effects
}

fn effect_kind_code(effect: EffectReference) -> u8 {
    match effect {
        EffectReference::Brightness { .. } => 1,
        EffectReference::Contrast { .. } => 2,
        EffectReference::Saturation { .. } => 3,
        EffectReference::GaussianBlur { .. } => 4,
    }
}

fn effect_kind_modified(settings: &ProjectTimelineVisualSettingsView, kind: u8) -> bool {
    match kind {
        1 => settings.update_brightness,
        2 => settings.update_contrast,
        3 => settings.update_saturation,
        4 => settings.update_gaussian_blur,
        _ => false,
    }
}

fn effect_from_view(
    settings: &ProjectTimelineVisualSettingsView,
    kind: u8,
) -> Option<EffectReference> {
    match kind {
        1 if settings.brightness_amount_milli != 0 => Some(EffectReference::Brightness {
            amount_milli: settings.brightness_amount_milli,
        }),
        2 if settings.contrast_amount_milli != 1_000 => Some(EffectReference::Contrast {
            amount_milli: settings.contrast_amount_milli,
        }),
        3 if settings.saturation_amount_milli != 1_000 => Some(EffectReference::Saturation {
            amount_milli: settings.saturation_amount_milli,
        }),
        4 if settings.gaussian_blur_radius_milli != 0 => Some(EffectReference::GaussianBlur {
            radius_milli: settings.gaussian_blur_radius_milli,
        }),
        _ => None,
    }
}

fn transition_kind_code(transition: Option<TransitionReference>) -> u8 {
    match transition.map(|transition| transition.kind) {
        None => 0,
        Some(TransitionKind::CrossDissolve) => 1,
        Some(TransitionKind::FadeThroughBlack) => 2,
        Some(TransitionKind::Wipe) => 3,
    }
}

fn transition_from_view(
    kind: u8,
    duration: RationalTimeView,
) -> Result<Option<TransitionReference>, ProjectBridgeError> {
    if kind == 0 {
        return Ok(None);
    }
    let kind = match kind {
        1 => TransitionKind::CrossDissolve,
        2 => TransitionKind::FadeThroughBlack,
        3 => TransitionKind::Wipe,
        _ => {
            return Err(ProjectBridgeError {
                code: "INVALID_TRANSITION_KIND".to_owned(),
                message: "The selected transition is not supported.".to_owned(),
            });
        }
    };
    let duration = time_from_view(duration)?;
    Ok(Some(TransitionReference { kind, duration }))
}

fn audio_settings_view(settings: AudioSettings) -> ProjectTimelineAudioSettingsView {
    ProjectTimelineAudioSettingsView {
        gain_millidecibels: settings.gain_millidecibels,
        pan_basis_points: settings.pan_basis_points,
        fade_in: rational_time_view(settings.fade_in),
        fade_out: rational_time_view(settings.fade_out),
    }
}

fn time_from_view(time: RationalTimeView) -> Result<RationalTime, ProjectBridgeError> {
    RationalTime::new(time.numerator, time.denominator).map_err(|error| ProjectBridgeError {
        code: "INVALID_TIMELINE_TIME".to_owned(),
        message: error.to_string(),
    })
}

fn ensure_visual_settings_snapshot(
    result: &QueryResult,
    project_id: ProjectId,
    project_instance_id: ProjectInstanceId,
    revision: ProjectRevision,
) -> Result<(), ProjectBridgeError> {
    if result.summary.project_id != project_id
        || result.summary.project_instance_id != project_instance_id
        || result.summary.project_revision != revision
    {
        return Err(ProjectBridgeError {
            code: "REVISION_CONFLICT".to_owned(),
            message: "The project changed while video settings were being read.".to_owned(),
        });
    }
    Ok(())
}

fn unsupported_visual_settings() -> ProjectBridgeError {
    ProjectBridgeError {
        code: "UNSUPPORTED_CLIP_SETTINGS".to_owned(),
        message: "Transform, effect, and transition settings are only available for visual clips."
            .to_owned(),
    }
}

fn unsupported_audio_settings() -> ProjectBridgeError {
    ProjectBridgeError {
        code: "UNSUPPORTED_CLIP_SETTINGS".to_owned(),
        message: "Audio gain, pan, and fade settings are only available for audio clips."
            .to_owned(),
    }
}

fn timeline_marker_page_view(
    result: &QueryResult,
    page: &TimelineMarkerPage,
) -> ProjectTimelineMarkerPageView {
    ProjectTimelineMarkerPageView {
        project_id: result.summary.project_id.to_string(),
        project_instance_id: result.summary.project_instance_id.to_string(),
        project_revision: result.summary.project_revision.value(),
        items: page.items.iter().map(timeline_marker_view).collect(),
        total_count: u64::try_from(page.total_count).expect("bounded count fits in u64"),
        offset: u64::try_from(page.offset).expect("bounded offset fits in u64"),
        limit: u64::try_from(page.limit).expect("bounded limit fits in u64"),
        next_offset: page
            .next_offset
            .map(|value| u64::try_from(value).expect("bounded offset fits in u64")),
    }
}

fn timeline_marker_view(marker: &TimelineMarkerState) -> ProjectTimelineMarkerView {
    ProjectTimelineMarkerView {
        marker_id: marker.marker_id.to_string(),
        timeline_time: rational_time_view(marker.timeline_time),
        label: marker.label.clone(),
    }
}

fn timeline_snap_view(result: &QueryResult, snap: &TimelineSnapResult) -> ProjectTimelineSnapView {
    ProjectTimelineSnapView {
        project_id: result.summary.project_id.to_string(),
        project_instance_id: result.summary.project_instance_id.to_string(),
        project_revision: result.summary.project_revision.value(),
        raw_target_time: rational_time_view(snap.raw_target_time),
        resolved_target_time: rational_time_view(snap.resolved_target_time),
        snapped: snap.snapped,
        moving_anchor: match snap.moving_anchor {
            TimelineSnapMovingAnchor::None => TimelineSnapMovingAnchorView::None,
            TimelineSnapMovingAnchor::Start => TimelineSnapMovingAnchorView::Start,
            TimelineSnapMovingAnchor::End => TimelineSnapMovingAnchorView::End,
        },
        target_kind: match snap.target_kind {
            TimelineSnapTargetKind::None => TimelineSnapTargetKindView::None,
            TimelineSnapTargetKind::TimelineZero => TimelineSnapTargetKindView::TimelineZero,
            TimelineSnapTargetKind::ClipStart => TimelineSnapTargetKindView::ClipStart,
            TimelineSnapTargetKind::ClipEnd => TimelineSnapTargetKindView::ClipEnd,
            TimelineSnapTargetKind::Marker => TimelineSnapTargetKindView::Marker,
        },
        target_time: rational_time_view(snap.target_time),
        target_track_id: snap.target_track_id.map(|id| id.to_string()),
        target_clip_id: snap.target_clip_id.map(|id| id.to_string()),
        target_marker_id: snap.target_marker_id.map(|id| id.to_string()),
    }
}

fn invalid_timeline_id(code: &str, message: String) -> ProjectActionResult {
    action_error(ProjectBridgeError {
        code: code.to_owned(),
        message,
    })
}

pub(crate) fn operation_bridge_error(error: OperationError) -> ProjectBridgeError {
    ProjectBridgeError {
        code: operation_error_code(error.code).to_owned(),
        message: error.to_string(),
    }
}

fn invalid_arguments() -> ProjectBridgeError {
    ProjectBridgeError {
        code: "INVALID_ARGUMENTS".to_owned(),
        message: "media page bounds are invalid".to_owned(),
    }
}

fn timeline_arguments_error() -> ProjectBridgeError {
    ProjectBridgeError {
        code: "INVALID_ARGUMENTS".to_owned(),
        message: "timeline time values are invalid".to_owned(),
    }
}

fn timeline_query_arguments_error() -> ProjectBridgeError {
    ProjectBridgeError {
        code: "INVALID_ARGUMENTS".to_owned(),
        message: "timeline clip page bounds are invalid".to_owned(),
    }
}

fn timeline_marker_query_arguments_error() -> ProjectBridgeError {
    ProjectBridgeError {
        code: "INVALID_ARGUMENTS".to_owned(),
        message: "timeline marker page bounds are invalid".to_owned(),
    }
}

pub(crate) fn unexpected_response_error() -> ProjectBridgeError {
    ProjectBridgeError {
        code: "UNEXPECTED_RESPONSE".to_owned(),
        message: "The project host returned an unexpected response.".to_owned(),
    }
}

fn view_from_query(
    result: &QueryResult,
    dirty: bool,
    descriptor_path: Option<std::path::PathBuf>,
) -> ProjectView {
    ProjectView {
        project_id: result.summary.project_id.to_string(),
        project_instance_id: result.summary.project_instance_id.to_string(),
        revision: result.summary.project_revision.value(),
        name: result.summary.name.clone(),
        dirty,
        descriptor_path: descriptor_path
            .map_or_else(String::new, |path| path.to_string_lossy().into_owned()),
    }
}

fn forward_events(
    receiver: mpsc::Receiver<ProjectHostEvent>,
    sink: StreamSink<ProjectHostEventView>,
) {
    for event in receiver {
        if sink.add(event_view(event)).is_err() {
            break;
        }
    }
}

pub(crate) fn forward_media_artifact_events(
    receiver: mpsc::Receiver<MediaArtifactEvent>,
    sink: StreamSink<MediaArtifactEventView>,
) {
    for event in receiver {
        let Some(event) = media_artifact_event_view(event) else {
            continue;
        };
        if sink.add(event).is_err() {
            break;
        }
    }
}

fn media_artifact_event_view(event: MediaArtifactEvent) -> Option<MediaArtifactEventView> {
    let kind = match event.kind {
        CacheArtifactKind::Thumbnail => MediaArtifactKindView::Thumbnail,
        CacheArtifactKind::Waveform => MediaArtifactKindView::Waveform,
        CacheArtifactKind::Proxy => return None,
    };
    Some(MediaArtifactEventView {
        sequence: event.sequence,
        media_id: event.media_id.to_string(),
        kind,
        cache_key: event.cache_key.to_hex(),
        job_id: event.job_id.to_string(),
        state: match event.state {
            MediaArtifactEventState::Succeeded => MediaArtifactEventStateView::Succeeded,
            MediaArtifactEventState::Failed => MediaArtifactEventStateView::Failed,
            MediaArtifactEventState::Cancelled => MediaArtifactEventStateView::Cancelled,
        },
        error_code: event.error_code.map(|code| code.as_str().to_owned()),
    })
}

fn event_view(event: ProjectHostEvent) -> ProjectHostEventView {
    ProjectHostEventView {
        sequence: event.sequence,
        kind: match event.kind {
            ProjectHostEventKind::ProjectChanged => "project_changed",
            ProjectHostEventKind::ProjectSaved => "project_saved",
            ProjectHostEventKind::SessionClosing => "session_closing",
        }
        .to_owned(),
        project_id: event.project_id,
        project_instance_id: event.project_instance_id,
        revision: event.project_revision,
        dirty: event.dirty,
    }
}

fn action_operation_error(error: OperationError) -> ProjectActionResult {
    ProjectActionResult {
        succeeded: false,
        error_code: operation_error_code(error.code).to_owned(),
        message: error.to_string(),
        view: None,
    }
}

fn action_error(error: ProjectBridgeError) -> ProjectActionResult {
    ProjectActionResult {
        succeeded: false,
        error_code: error.code,
        message: error.message,
        view: None,
    }
}

fn project_session_error(error: or_core::ProjectFileSessionError) -> ProjectBridgeError {
    ProjectBridgeError {
        code: error.code().as_str().to_owned(),
        message: error.to_string(),
    }
}

pub(crate) fn host_error(error: LiveProjectHostError) -> ProjectBridgeError {
    match error {
        LiveProjectHostError::Session(error) => project_session_error(error),
        LiveProjectHostError::Operation(error) => ProjectBridgeError {
            code: operation_error_code(error.code).to_owned(),
            message: error.to_string(),
        },
        LiveProjectHostError::Ipc(error) => ProjectBridgeError {
            code: "LOCAL_IPC_ERROR".to_owned(),
            message: error.to_string(),
        },
        LiveProjectHostError::UnexpectedResponse => ProjectBridgeError {
            code: "UNEXPECTED_RESPONSE".to_owned(),
            message: "The project host returned an unexpected response.".to_owned(),
        },
        LiveProjectHostError::LockPoisoned => ProjectBridgeError {
            code: "PROJECT_HOST_UNAVAILABLE".to_owned(),
            message: "The project host state is unavailable.".to_owned(),
        },
        LiveProjectHostError::SessionClosing => ProjectBridgeError {
            code: "PROJECT_CLOSING".to_owned(),
            message: "The project is closing.".to_owned(),
        },
        LiveProjectHostError::UnsavedChanges => ProjectBridgeError {
            code: "UNSAVED_CHANGES".to_owned(),
            message: "The project has unsaved changes.".to_owned(),
        },
    }
}

fn operation_error_code(code: OperationErrorCode) -> &'static str {
    match code {
        OperationErrorCode::UnknownCommand => "UNKNOWN_COMMAND",
        OperationErrorCode::UnsupportedCommandSchema => "UNSUPPORTED_COMMAND_SCHEMA",
        OperationErrorCode::UnknownQuery => "UNKNOWN_QUERY",
        OperationErrorCode::UnsupportedQuerySchema => "UNSUPPORTED_QUERY_SCHEMA",
        OperationErrorCode::UnsupportedTransactionSchema => "UNSUPPORTED_TRANSACTION_SCHEMA",
        OperationErrorCode::EmptyTransaction => "EMPTY_TRANSACTION",
        OperationErrorCode::CommandNotAllowedInTransaction => "COMMAND_NOT_ALLOWED_IN_TRANSACTION",
        OperationErrorCode::ProjectIdMismatch => "PROJECT_ID_MISMATCH",
        OperationErrorCode::ProjectInstanceMismatch => "PROJECT_INSTANCE_MISMATCH",
        OperationErrorCode::RevisionConflict => "REVISION_CONFLICT",
        OperationErrorCode::InvalidArguments => "INVALID_ARGUMENTS",
        OperationErrorCode::RevisionOverflow => "REVISION_OVERFLOW",
        OperationErrorCode::NothingToUndo => "NOTHING_TO_UNDO",
        OperationErrorCode::NothingToRedo => "NOTHING_TO_REDO",
        OperationErrorCode::HistoryConflict => "HISTORY_CONFLICT",
        OperationErrorCode::HistoryStorageFailure => "HISTORY_STORAGE_FAILURE",
        OperationErrorCode::MediaIdAlreadyExists => "MEDIA_ID_ALREADY_EXISTS",
        OperationErrorCode::MediaSourceAlreadyExists => "MEDIA_SOURCE_ALREADY_EXISTS",
        OperationErrorCode::MediaNotFound => "MEDIA_NOT_FOUND",
        OperationErrorCode::MediaInUse => "MEDIA_IN_USE",
        OperationErrorCode::TimelineTrackIdAlreadyExists => "TIMELINE_TRACK_ID_ALREADY_EXISTS",
        OperationErrorCode::TimelineTrackNotFound => "TIMELINE_TRACK_NOT_FOUND",
        OperationErrorCode::TimelineTrackNotEmpty => "TIMELINE_TRACK_NOT_EMPTY",
        OperationErrorCode::TimelineTrackLocked => "TIMELINE_TRACK_LOCKED",
        OperationErrorCode::TimelineClipIdAlreadyExists => "TIMELINE_CLIP_ID_ALREADY_EXISTS",
        OperationErrorCode::TimelineClipNotFound => "TIMELINE_CLIP_NOT_FOUND",
        OperationErrorCode::TimelineMediaIncompatible => "TIMELINE_MEDIA_INCOMPATIBLE",
        OperationErrorCode::TimelineOverlap => "TIMELINE_OVERLAP",
        OperationErrorCode::TimelineNothingToDuck => "TIMELINE_NOTHING_TO_DUCK",
        OperationErrorCode::TimelineOverwriteSplitRequiresClipId => {
            "TIMELINE_OVERWRITE_SPLIT_REQUIRES_CLIP_ID"
        }
        OperationErrorCode::TimelineLimitExceeded => "TIMELINE_LIMIT_EXCEEDED",
        OperationErrorCode::TimelineMarkerIdAlreadyExists => "TIMELINE_MARKER_ID_ALREADY_EXISTS",
        OperationErrorCode::TimelineMarkerNotFound => "TIMELINE_MARKER_NOT_FOUND",
    }
}

fn recovery_action_error(error: ProjectRecoveryError) -> RecoveryActionResult {
    RecoveryActionResult {
        succeeded: false,
        changed: false,
        message: error.to_string(),
    }
}

fn recovery_conflict_name(reason: RecoveryConflictReason) -> &'static str {
    match reason {
        RecoveryConflictReason::Orphaned => "orphaned",
        RecoveryConflictReason::ForeignProject => "foreign_project",
        RecoveryConflictReason::ChangedLineage => "changed_lineage",
    }
}

#[cfg(test)]
mod tests {
    use super::super::media_artifact_config::{
        CachePlatform, configured_media_artifact_cache_root, media_artifact_worker_count_for,
    };
    use super::BridgeTranscriptionHandler;
    use super::{
        MediaArtifactKindView, PreviewRuntime, ProjectCaptionCueView, ProjectHostHandle,
        audio_settings_view, effects_from_view, media_artifact_event_view, operation_error_code,
        rational_time_millis, rational_time_view, timeline_clip_page_view,
        timeline_marker_page_view, timeline_snap_view, timeline_track_view, visual_settings_view,
    };
    use crate::transcription::TranscriptionService;
    use or_core::{
        ExportRequest, ExportResponse, ProjectFileSession, TranscriptionRequest,
        TranscriptionResponse,
    };
    use or_ipc::{ExportRequestHandler, LiveProjectHost, TranscriptionRequestHandler};
    use std::sync::Arc;

    struct TestHostDirectory(PathBuf);

    impl TestHostDirectory {
        fn new() -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("or-bridge-test-{}-{unique}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn project(&self) -> PathBuf {
            self.0.join("unused.orproj")
        }
    }

    impl Drop for TestHostDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Two workers keep playback responsive on small machines; four halve the
    /// cold-import fill time once a machine has cores to spare.
    #[test]
    fn artifact_pool_width_follows_core_count() {
        assert_eq!(media_artifact_worker_count_for(1), 2);
        assert_eq!(media_artifact_worker_count_for(4), 2);
        assert_eq!(media_artifact_worker_count_for(5), 2);
        assert_eq!(media_artifact_worker_count_for(6), 4);
        assert_eq!(media_artifact_worker_count_for(10), 4);
    }

    fn handle_without_artifact_service(directory: &TestHostDirectory) -> ProjectHostHandle {
        ProjectHostHandle {
            host: LiveProjectHost::in_process_with_export_handler(
                ProjectFileSession::create_new(directory.project(), "A").unwrap(),
                Arc::new(TestExportHandler),
            ),
            media_artifact_service: None,
            transcription_service: None,
            preview_runtime: Arc::new(PreviewRuntime::new()),
        }
    }

    struct TestExportHandler;

    impl ExportRequestHandler for TestExportHandler {
        fn handle_export_request(
            &self,
            _session: &or_core::ProjectSession,
            _request: ExportRequest,
        ) -> ExportResponse {
            ExportResponse::failure("UNUSED_TEST_HANDLER", "export is unused in this test")
        }
    }
    use or_core::{
        AudioSettings, CacheArtifactKind, CacheKey, ClipContent, ClipId, ClipSettings,
        EffectReference, JobId, MarkerId, MediaArtifactEvent, MediaArtifactEventState, MediaId,
        OperationErrorCode, ParametersFingerprint, ProjectId, ProjectInstanceId, ProjectRevision,
        ProjectSummary, QueryResult, RationalTime, SourceFingerprint, TextFormatting, TimeRange,
        TimelineClipPageV2, TimelineClipState, TimelineMarkerPage, TimelineMarkerState,
        TimelineSnapMovingAnchor, TimelineSnapResult, TimelineSnapTargetKind,
        TimelineTrackSummaryV2, TrackId, TrackKind, TrackState, TransitionKind,
        TransitionReference, VisualSettings,
    };
    use std::path::PathBuf;

    #[test]
    fn typed_effect_and_audio_settings_survive_bridge_view_conversion() {
        let visual = VisualSettings {
            effects: vec![
                EffectReference::Brightness { amount_milli: 250 },
                EffectReference::Contrast {
                    amount_milli: 1_250,
                },
                EffectReference::Saturation { amount_milli: 750 },
                EffectReference::GaussianBlur {
                    radius_milli: 1_500,
                },
            ],
            transition_in: Some(TransitionReference {
                kind: TransitionKind::CrossDissolve,
                duration: RationalTime::new(1, 2).unwrap(),
            }),
            transition_out: Some(TransitionReference {
                kind: TransitionKind::Wipe,
                duration: RationalTime::new(1, 3).unwrap(),
            }),
            ..VisualSettings::default()
        };

        let mut view = visual_settings_view(&visual);
        view.update_brightness = true;
        view.update_contrast = true;
        view.update_saturation = true;
        view.update_gaussian_blur = true;
        assert_eq!(effects_from_view(&visual.effects, &view), visual.effects);
        assert_eq!(view.transition_in_kind, 1);
        assert_eq!(
            view.transition_in_duration,
            rational_time_view(RationalTime::new(1, 2).unwrap())
        );
        assert_eq!(view.transition_out_kind, 3);
        assert_eq!(
            view.transition_out_duration,
            rational_time_view(RationalTime::new(1, 3).unwrap())
        );

        view.update_brightness = true;
        view.update_contrast = false;
        view.update_saturation = false;
        view.update_gaussian_blur = false;
        view.brightness_amount_milli = -250;
        assert_eq!(
            effects_from_view(&visual.effects, &view),
            vec![
                EffectReference::Brightness { amount_milli: -250 },
                visual.effects[1],
                visual.effects[2],
                visual.effects[3],
            ]
        );

        let audio = AudioSettings {
            gain_millidecibels: -6_250,
            pan_basis_points: -3_750,
            fade_in: RationalTime::new(1, 2).unwrap(),
            fade_out: RationalTime::new(3, 4).unwrap(),
        };
        let audio_view = audio_settings_view(audio);
        assert_eq!(audio_view.gain_millidecibels, -6_250);
        assert_eq!(audio_view.pan_basis_points, -3_750);
        assert_eq!(audio_view.fade_in, rational_time_view(audio.fade_in));
        assert_eq!(audio_view.fade_out, rational_time_view(audio.fade_out));
    }

    #[test]
    fn artifact_cache_roots_follow_desktop_platform_conventions() {
        assert_eq!(
            configured_media_artifact_cache_root(
                CachePlatform::MacOs,
                Some(PathBuf::from("/Users/test")),
                None,
                None,
            ),
            Some(PathBuf::from(
                "/Users/test/Library/Caches/Opencut-Reinforced/media-artifacts"
            )),
        );
        assert_eq!(
            configured_media_artifact_cache_root(
                CachePlatform::Linux,
                Some(PathBuf::from("/home/test")),
                Some(PathBuf::from("/tmp/cache")),
                None,
            ),
            Some(PathBuf::from(
                "/tmp/cache/opencut-reinforced/media-artifacts"
            )),
        );
        assert_eq!(
            configured_media_artifact_cache_root(
                CachePlatform::Linux,
                Some(PathBuf::from("/home/test")),
                None,
                None,
            ),
            Some(PathBuf::from(
                "/home/test/.cache/opencut-reinforced/media-artifacts"
            )),
        );
        assert_eq!(
            configured_media_artifact_cache_root(
                CachePlatform::Windows,
                None,
                None,
                Some(PathBuf::from("C:/Users/test/AppData/Local")),
            ),
            Some(PathBuf::from(
                "C:/Users/test/AppData/Local/Opencut Reinforced/Cache/media-artifacts"
            )),
        );
    }

    #[test]
    fn media_in_use_keeps_its_stable_bridge_error_code() {
        assert_eq!(
            operation_error_code(OperationErrorCode::MediaInUse),
            "MEDIA_IN_USE"
        );
    }

    #[test]
    fn timeline_bridge_views_keep_ids_identity_and_exact_rational_values() {
        let track_id = TrackId::generate();
        let track = timeline_track_view(&TimelineTrackSummaryV2 {
            track_id,
            kind: TrackKind::Audio,
            state: TrackState::new(true, true, true, true),
            clip_count: 3,
        });
        assert_eq!(track.track_id, track_id.to_string());
        assert_eq!(track.kind, super::TimelineTrackKindView::Audio);
        assert!(track.state.locked);
        assert!(track.state.visible);
        assert!(track.state.muted);
        assert!(track.state.solo);
        assert_eq!(track.clip_count, 3);

        let project_id = ProjectId::generate();
        let project_instance_id = ProjectInstanceId::generate();
        let result = QueryResult {
            query_id: "timeline.clips".to_owned(),
            schema_version: 1,
            summary: ProjectSummary {
                project_id,
                project_instance_id,
                project_revision: ProjectRevision::new(17),
                name: "Bridge fixture".to_owned(),
            },
            media_page: None,
            media_item: None,
            timeline_tracks: None,
            timeline_tracks_v2: None,
            timeline_clip_page: None,
            timeline_clip_page_v2: None,
            timeline_snap: None,
            timeline_marker_page: None,
            timeline_sequence_settings: None,
        };
        let clip_id = ClipId::generate();
        let text_clip_id = ClipId::generate();
        let caption_clip_id = ClipId::generate();
        let media_id = MediaId::generate();
        let clip_page = TimelineClipPageV2 {
            track_id,
            items: vec![
                TimelineClipState {
                    clip_id,
                    timeline_start: RationalTime::new(3003, 1001).unwrap(),
                    timeline_duration: RationalTime::new(5, 2).unwrap(),
                    content: ClipContent::Media {
                        playback_rate: or_core::RationalRate::ONE,
                        reversed: false,
                        frozen: false,
                        media_id,
                        source_range: TimeRange::new(
                            RationalTime::new(1, 2).unwrap(),
                            RationalTime::new(5, 2).unwrap(),
                        )
                        .unwrap(),
                    },
                    settings: ClipSettings::Visual(VisualSettings::default()),
                },
                TimelineClipState {
                    clip_id: text_clip_id,
                    timeline_start: RationalTime::new(1, 3).unwrap(),
                    timeline_duration: RationalTime::new(7, 3).unwrap(),
                    content: ClipContent::Text {
                        text: "Title".to_owned(),
                        formatting: TextFormatting::default(),
                    },
                    settings: ClipSettings::Visual(VisualSettings::default()),
                },
                TimelineClipState {
                    clip_id: caption_clip_id,
                    timeline_start: RationalTime::new(3, 2).unwrap(),
                    timeline_duration: RationalTime::new(1, 2).unwrap(),
                    content: ClipContent::Caption {
                        text: "Cue".to_owned(),
                        formatting: TextFormatting::default(),
                    },
                    settings: ClipSettings::Visual(VisualSettings::default()),
                },
            ],
            total_count: 9,
            offset: 4,
            limit: 5,
            next_offset: Some(9),
        };
        let view = timeline_clip_page_view(&result, &clip_page);
        assert_eq!(view.project_id, project_id.to_string());
        assert_eq!(view.project_instance_id, project_instance_id.to_string());
        assert_eq!(view.project_revision, 17);
        assert_eq!(view.track_id, track_id.to_string());
        assert_eq!(view.total_count, 9);
        assert_eq!(view.offset, 4);
        assert_eq!(view.limit, 5);
        assert_eq!(view.next_offset, Some(9));
        let clip = &view.items[0];
        assert_eq!(clip.clip_id, clip_id.to_string());
        assert_eq!(clip.content_kind, super::TimelineClipContentKindView::Media);
        assert_eq!(
            clip.media_id.as_deref(),
            Some(media_id.to_string().as_str())
        );
        assert_eq!(
            clip.timeline_start,
            rational_time_view(RationalTime::new(3003, 1001).unwrap())
        );
        assert_eq!(
            clip.source_start,
            Some(rational_time_view(RationalTime::new(1, 2).unwrap()))
        );
        assert_eq!(
            clip.timeline_duration,
            rational_time_view(RationalTime::new(5, 2).unwrap())
        );
        let text = &view.items[1];
        assert_eq!(text.clip_id, text_clip_id.to_string());
        assert_eq!(text.content_kind, super::TimelineClipContentKindView::Text);
        assert_eq!(text.media_id, None);
        assert_eq!(text.source_start, None);
        assert_eq!(text.text.as_deref(), Some("Title"));
        assert_eq!(
            text.formatting,
            Some(super::text_formatting_view(TextFormatting::default()))
        );
        assert_eq!(
            text.timeline_duration,
            rational_time_view(RationalTime::new(7, 3).unwrap())
        );
        assert_eq!(
            view.items[2].content_kind,
            super::TimelineClipContentKindView::Caption
        );
    }

    #[test]
    fn timeline_marker_bridge_views_keep_labels_and_exact_rational_values() {
        let project_id = ProjectId::generate();
        let project_instance_id = ProjectInstanceId::generate();
        let result = QueryResult {
            query_id: "timeline.markers".to_owned(),
            schema_version: 1,
            summary: ProjectSummary {
                project_id,
                project_instance_id,
                project_revision: ProjectRevision::new(23),
                name: "Bridge marker fixture".to_owned(),
            },
            media_page: None,
            media_item: None,
            timeline_tracks: None,
            timeline_tracks_v2: None,
            timeline_clip_page: None,
            timeline_clip_page_v2: None,
            timeline_snap: None,
            timeline_marker_page: None,
            timeline_sequence_settings: None,
        };
        let marker_id = MarkerId::generate();
        let marker_page = TimelineMarkerPage {
            items: vec![TimelineMarkerState {
                marker_id,
                timeline_time: RationalTime::new(3003, 1001).unwrap(),
                label: "Act two".to_owned(),
            }],
            total_count: 4,
            offset: 2,
            limit: 1,
            next_offset: Some(3),
        };

        let view = timeline_marker_page_view(&result, &marker_page);

        assert_eq!(view.project_id, project_id.to_string());
        assert_eq!(view.project_instance_id, project_instance_id.to_string());
        assert_eq!(view.project_revision, 23);
        assert_eq!(view.total_count, 4);
        assert_eq!(view.offset, 2);
        assert_eq!(view.limit, 1);
        assert_eq!(view.next_offset, Some(3));
        let marker = &view.items[0];
        assert_eq!(marker.marker_id, marker_id.to_string());
        assert_eq!(marker.label, "Act two");
        assert_eq!(
            marker.timeline_time,
            rational_time_view(RationalTime::new(3003, 1001).unwrap())
        );
    }

    #[test]
    fn timeline_snap_bridge_views_preserve_marker_targets() {
        let project_id = ProjectId::generate();
        let project_instance_id = ProjectInstanceId::generate();
        let marker_id = MarkerId::generate();
        let result = QueryResult {
            query_id: "timeline.snap".to_owned(),
            schema_version: 2,
            summary: ProjectSummary {
                project_id,
                project_instance_id,
                project_revision: ProjectRevision::new(31),
                name: "Bridge snap fixture".to_owned(),
            },
            media_page: None,
            media_item: None,
            timeline_tracks: None,
            timeline_tracks_v2: None,
            timeline_clip_page: None,
            timeline_clip_page_v2: None,
            timeline_snap: None,
            timeline_marker_page: None,
            timeline_sequence_settings: None,
        };
        let snap = TimelineSnapResult {
            raw_target_time: RationalTime::new(5, 2).unwrap(),
            resolved_target_time: RationalTime::new(3, 1).unwrap(),
            snapped: true,
            moving_anchor: TimelineSnapMovingAnchor::Start,
            target_kind: TimelineSnapTargetKind::Marker,
            target_time: RationalTime::new(3, 1).unwrap(),
            target_track_id: None,
            target_clip_id: None,
            target_marker_id: Some(marker_id),
        };

        let view = timeline_snap_view(&result, &snap);

        assert_eq!(view.target_kind, super::TimelineSnapTargetKindView::Marker);
        assert_eq!(view.target_marker_id, Some(marker_id.to_string()));
        assert_eq!(view.project_revision, 31);
    }

    #[test]
    fn artifact_cache_root_is_unavailable_without_required_environment_paths() {
        assert_eq!(
            configured_media_artifact_cache_root(CachePlatform::MacOs, None, None, None),
            None,
        );
        assert_eq!(
            configured_media_artifact_cache_root(CachePlatform::Linux, None, None, None),
            None,
        );
        assert_eq!(
            configured_media_artifact_cache_root(CachePlatform::Windows, None, None, None),
            None,
        );
        assert_eq!(
            configured_media_artifact_cache_root(
                CachePlatform::Unsupported,
                Some(PathBuf::from("/home/test")),
                None,
                None,
            ),
            None,
        );
    }

    #[test]
    fn artifact_events_are_absent_rather_than_failed_without_a_service() {
        // Android has no cached-artifact service. The generated stream wrapper
        // discards the subscribe call's future, so returning an error here became
        // an unhandled ProjectBridgeError that escaped every Dart catch site.
        let directory = TestHostDirectory::new();
        let handle = handle_without_artifact_service(&directory);

        assert!(
            handle.media_artifact_event_receiver().is_none(),
            "a platform without cached artifacts must subscribe to nothing, not fail",
        );
        let error = handle
            .read_media_artifact(
                MediaArtifactKindView::Thumbnail,
                CacheKey::new(
                    CacheArtifactKind::Thumbnail,
                    SourceFingerprint::from_bytes(b"source"),
                    ParametersFingerprint::from_bytes(b"thumbnail"),
                )
                .to_hex(),
            )
            .expect_err("a missing cache must be reported, not ignored");
        assert_eq!(error.code, "CACHE_UNAVAILABLE");
    }

    #[test]
    fn audio_cleanup_reports_a_missing_source_with_the_core_code() {
        let directory = TestHostDirectory::new();
        let handle = handle_without_artifact_service(&directory);
        let missing = directory.0.join("no-such-source-9f2.wav");
        let destination = directory.0.join("cleaned.wav");

        let result = handle.clean_up_audio(
            missing.display().to_string(),
            destination.display().to_string(),
            None,
            12,
        );

        assert!(!result.succeeded);
        // The code is the core's, so a client switches on one vocabulary whichever
        // layer refuses the request.
        assert_eq!(result.error_code, "INVALID_ARGUMENTS");
        assert!(!destination.exists());
    }

    #[test]
    fn audio_cleanup_refuses_a_zero_reduction_before_measuring_anything() {
        let directory = TestHostDirectory::new();
        let handle = handle_without_artifact_service(&directory);

        let result = handle.clean_up_audio(
            directory.0.join("anything.wav").display().to_string(),
            directory.0.join("cleaned.wav").display().to_string(),
            Some(-30),
            0,
        );

        assert!(!result.succeeded);
        assert_eq!(result.error_code, "INVALID_ARGUMENTS");
        assert!(result.message.contains("reduction"));
    }

    #[test]
    fn caption_preview_returns_editable_exact_millisecond_cues_without_mutation() {
        let directory = TestHostDirectory::new();
        let handle = handle_without_artifact_service(&directory);
        let captions = directory.0.join("captions.srt");
        std::fs::write(
            &captions,
            "1\n00:00:01,250 --> 00:00:03,000\nA caption to review\n\n",
        )
        .unwrap();
        let before = handle.summary().unwrap();

        let preview = handle
            .preview_timeline_captions(captions.display().to_string())
            .unwrap();

        assert_eq!(preview.format_name, "SubRip");
        assert_eq!(preview.cue_count, 1);
        assert_eq!(preview.cues[0].start_ms, 1_250);
        assert_eq!(preview.cues[0].end_ms, 3_000);
        assert_eq!(preview.cues[0].text, "A caption to review");
        assert_eq!(handle.summary().unwrap().revision, before.revision);
    }

    #[test]
    fn reviewed_caption_import_is_revision_checked_and_uses_the_project_command() {
        let directory = TestHostDirectory::new();
        let handle = handle_without_artifact_service(&directory);
        let before = handle.summary().unwrap();
        let cue = ProjectCaptionCueView {
            start_ms: 1_250,
            end_ms: 3_000,
            text: "Corrected by the editor".to_owned(),
        };

        let stale = handle.import_reviewed_timeline_captions(
            before.project_id.clone(),
            before.project_instance_id.clone(),
            before.revision + 1,
            vec![cue.clone()],
        );
        assert!(!stale.succeeded);
        assert_eq!(stale.error_code, "REVISION_CONFLICT");
        assert_eq!(handle.summary().unwrap().revision, before.revision);

        let applied = handle.import_reviewed_timeline_captions(
            before.project_id,
            before.project_instance_id,
            before.revision,
            vec![cue],
        );
        assert!(applied.succeeded, "{}", applied.message);
        assert!(applied.message.contains("Imported 1 reviewed captions"));
        assert_eq!(handle.summary().unwrap().revision, before.revision + 1);
    }

    #[test]
    fn caption_time_view_rejects_negative_and_non_millisecond_values() {
        assert!(rational_time_millis(RationalTime::new(1, 2).unwrap()).is_ok());
        assert!(rational_time_millis(RationalTime::new(-1, 1).unwrap()).is_err());
        assert!(rational_time_millis(RationalTime::new(1, 3).unwrap()).is_err());
    }

    #[test]
    fn proxy_artifact_events_stay_internal_to_the_core_service() {
        let mut event = MediaArtifactEvent {
            sequence: 1,
            media_id: MediaId::generate(),
            kind: CacheArtifactKind::Thumbnail,
            cache_key: CacheKey::new(
                CacheArtifactKind::Thumbnail,
                SourceFingerprint::from_bytes(b"source"),
                ParametersFingerprint::from_bytes(b"thumbnail"),
            ),
            job_id: JobId::generate(),
            state: MediaArtifactEventState::Succeeded,
            error_code: None,
        };

        for (kind, expected) in [
            (
                CacheArtifactKind::Thumbnail,
                Some(MediaArtifactKindView::Thumbnail),
            ),
            (
                CacheArtifactKind::Waveform,
                Some(MediaArtifactKindView::Waveform),
            ),
            (CacheArtifactKind::Proxy, None),
        ] {
            event.kind = kind;
            assert_eq!(
                media_artifact_event_view(event.clone()).map(|view| view.kind),
                expected
            );
        }
    }
    #[test]
    fn bridge_transcription_rejects_another_session_a_stale_revision_and_an_unknown_job() {
        let directory = TestHostDirectory::new();
        let session =
            ProjectFileSession::create_new(directory.0.join("captions.orproj"), "B").unwrap();
        let Some(service) = TranscriptionService::new() else {
            return;
        };
        let handler = BridgeTranscriptionHandler {
            service: Arc::new(service),
        };
        let project_id = session.session().project_id();
        let project_instance_id = session.session().project_instance_id();

        let other_session = handler.handle_transcription_request(
            session.session(),
            TranscriptionRequest::Status {
                project_id: ProjectId::generate(),
                project_instance_id,
                job_id: JobId::generate(),
            },
        );
        assert!(matches!(
            other_session,
            TranscriptionResponse::Failure(failure) if failure.code == "PROJECT_SESSION_MISMATCH"
        ));

        let stale = handler.handle_transcription_request(
            session.session(),
            TranscriptionRequest::Start {
                project_id,
                project_instance_id,
                expected_project_revision: ProjectRevision::new(99),
                media_id: MediaId::generate(),
            },
        );
        assert!(matches!(
            stale,
            TranscriptionResponse::Failure(failure) if failure.code == "PROJECT_REVISION_CONFLICT"
        ));

        let unknown = handler.handle_transcription_request(
            session.session(),
            TranscriptionRequest::Proposal {
                project_id,
                project_instance_id,
                job_id: JobId::generate(),
            },
        );
        assert!(matches!(
            unknown,
            TranscriptionResponse::Failure(failure) if failure.code == "PROPOSAL_UNAVAILABLE"
        ));
    }
}
