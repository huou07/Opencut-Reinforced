use crate::frb_generated::StreamSink;
use crate::preview::{PreviewError, PreviewRuntime, PreviewSnapshot};
use flutter_rust_bridge::frb;
use or_core::{
    ApplicationRequest, ApplicationResponse, CacheArtifactKind, CacheKey, CacheStoreConfig, ClipId,
    CommandEnvelope, JobManagerConfig, LegacyTimelineClipState, MarkerId, MediaArtifactEvent,
    MediaArtifactEventState, MediaArtifactRequest, MediaArtifactRequestState, MediaArtifactService,
    MediaArtifactServiceConfig, MediaId, MediaItem, MediaStreamMetadata, OperationError,
    OperationErrorCode, ProjectFileSession, ProjectId, ProjectInstanceId, ProjectRecoveryError,
    ProjectRevision, QueryEnvelope, QueryResult, RationalRate, RationalTime, RecoveryApplyOutcome,
    RecoveryConflictReason, RecoveryInspection, TimeRange, TimelineClipPage, TimelineMarkerPage,
    TimelineMarkerState, TimelineSnapMovingAnchor, TimelineSnapOperation, TimelineSnapResult,
    TimelineSnapTargetKind, TimelineTrackSummaryV2, TimelineTrimEdge, TrackId, TrackKind,
    TrackState, apply_project_recovery, discard_project_recovery,
    ffmpeg_executable_from_environment, inspect_project_recovery, prepare_media_import,
};
use or_ipc::{LiveProjectHost, LiveProjectHostError, ProjectHostEvent, ProjectHostEventKind};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::mpsc,
    thread,
};

#[derive(Clone, Debug)]
pub struct ProjectView {
    pub project_id: String,
    pub project_instance_id: String,
    pub revision: u64,
    pub name: String,
    pub dirty: bool,
    pub descriptor_path: String,
}

#[derive(Clone, Debug)]
pub struct ProjectActionResult {
    pub succeeded: bool,
    pub error_code: String,
    pub message: String,
    pub view: Option<ProjectView>,
}

#[derive(Clone, Debug)]
pub struct ProjectMediaItemView {
    pub media_id: String,
    pub source_uri: String,
    pub format_names: Vec<String>,
    pub duration: Option<String>,
    pub video_details: Option<String>,
    pub audio_details: Option<String>,
    pub container_duration: Option<RationalTimeView>,
    pub first_video_duration: Option<RationalTimeView>,
    pub first_audio_duration: Option<RationalTimeView>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RationalTimeView {
    pub numerator: i64,
    pub denominator: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RationalRateView {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Clone, Debug)]
pub struct ProjectTimelineSequenceSettingsView {
    pub project_id: String,
    pub project_instance_id: String,
    pub project_revision: u64,
    pub sequence_frame_rate: Option<RationalRateView>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreviewFrameStepView {
    Previous,
    Next,
}

#[derive(Clone, Debug)]
pub struct ProjectPreviewStateView {
    pub position: RationalTimeView,
    pub presented_time: Option<RationalTimeView>,
    pub sequence_frame_rate: Option<RationalRateView>,
    pub content_end: Option<RationalTimeView>,
    pub playing: bool,
    pub generation: u64,
    pub frame_sequence: u64,
    pub width: u32,
    pub height: u32,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimelineTrackKindView {
    Video,
    Audio,
    Text,
    Caption,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimelineTrimEdgeView {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimelineSnapOperationView {
    Move,
    TrimStart,
    TrimEnd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimelineSnapMovingAnchorView {
    None,
    Start,
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimelineSnapTargetKindView {
    None,
    TimelineZero,
    ClipStart,
    ClipEnd,
    Marker,
}

#[derive(Clone, Debug)]
pub struct ProjectTimelineTrackView {
    pub track_id: String,
    pub kind: TimelineTrackKindView,
    pub state: ProjectTimelineTrackStateView,
    pub clip_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectTimelineTrackStateView {
    pub locked: bool,
    pub visible: bool,
    pub muted: bool,
    pub solo: bool,
}

#[derive(Clone, Debug)]
pub struct ProjectTimelineTracksView {
    pub project_id: String,
    pub project_instance_id: String,
    pub project_revision: u64,
    pub items: Vec<ProjectTimelineTrackView>,
}

#[derive(Clone, Debug)]
pub struct ProjectTimelineClipView {
    pub clip_id: String,
    pub media_id: String,
    pub timeline_start: RationalTimeView,
    pub source_start: RationalTimeView,
    pub source_duration: RationalTimeView,
}

#[derive(Clone, Debug)]
pub struct ProjectTimelineClipPageView {
    pub project_id: String,
    pub project_instance_id: String,
    pub project_revision: u64,
    pub track_id: String,
    pub items: Vec<ProjectTimelineClipView>,
    pub total_count: u64,
    pub offset: u64,
    pub limit: u64,
    pub next_offset: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct ProjectTimelineMarkerView {
    pub marker_id: String,
    pub timeline_time: RationalTimeView,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct ProjectTimelineMarkerPageView {
    pub project_id: String,
    pub project_instance_id: String,
    pub project_revision: u64,
    pub items: Vec<ProjectTimelineMarkerView>,
    pub total_count: u64,
    pub offset: u64,
    pub limit: u64,
    pub next_offset: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct ProjectTimelineSnapView {
    pub project_id: String,
    pub project_instance_id: String,
    pub project_revision: u64,
    pub raw_target_time: RationalTimeView,
    pub resolved_target_time: RationalTimeView,
    pub snapped: bool,
    pub moving_anchor: TimelineSnapMovingAnchorView,
    pub target_kind: TimelineSnapTargetKindView,
    pub target_time: RationalTimeView,
    pub target_track_id: Option<String>,
    pub target_clip_id: Option<String>,
    pub target_marker_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProjectMediaPageView {
    pub project_id: String,
    pub project_instance_id: String,
    pub project_revision: u64,
    pub items: Vec<ProjectMediaItemView>,
    pub total_count: u64,
    pub offset: u64,
    pub limit: u64,
    pub next_offset: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct ProjectHostEventView {
    pub sequence: u64,
    pub kind: String,
    pub project_id: String,
    pub project_instance_id: String,
    pub revision: u64,
    pub dirty: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaArtifactKindView {
    Thumbnail,
    Waveform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaArtifactRequestStateView {
    Ready,
    Queued,
    Running,
    NotApplicable,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaArtifactEventStateView {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug)]
pub struct MediaArtifactRequestView {
    pub media_id: String,
    pub kind: MediaArtifactKindView,
    pub cache_key: Option<String>,
    pub job_id: Option<String>,
    pub state: MediaArtifactRequestStateView,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MediaArtifactBytesView {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

#[derive(Clone, Debug)]
pub struct MediaArtifactEventView {
    pub sequence: u64,
    pub media_id: String,
    pub kind: MediaArtifactKindView,
    pub cache_key: String,
    pub job_id: String,
    pub state: MediaArtifactEventStateView,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RecoveryInspectionView {
    pub status: String,
    pub project_id: String,
    pub base_revision: u64,
    pub recovery_revision: u64,
    pub recovery_name: String,
    pub conflict_reason: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct RecoveryActionResult {
    pub succeeded: bool,
    pub changed: bool,
    pub message: String,
}

#[derive(Debug)]
pub struct ProjectBridgeError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for ProjectBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProjectBridgeError {}

/// Rust-owned opaque handle; it is the only bridge path to the live project host.
#[frb(opaque)]
pub struct ProjectHostHandle {
    host: LiveProjectHost,
    media_artifact_service: Option<MediaArtifactService>,
    preview_runtime: PreviewRuntime,
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
    let host = LiveProjectHost::start(session, None).map_err(host_error)?;
    Ok(ProjectHostHandle {
        host,
        media_artifact_service: create_media_artifact_service(),
        preview_runtime: PreviewRuntime::new(),
    })
}

fn create_media_artifact_service() -> Option<MediaArtifactService> {
    #[cfg(target_os = "android")]
    {
        None
    }

    #[cfg(not(target_os = "android"))]
    {
        let cache_root = media_artifact_cache_root()?;
        let jobs = JobManagerConfig::new(2, 32, 128).ok()?;
        let cache = CacheStoreConfig::new(8 * 1024 * 1024, 256 * 1024 * 1024).ok()?;
        MediaArtifactService::new(MediaArtifactServiceConfig::new(
            cache_root,
            jobs,
            cache,
            ffmpeg_executable_from_environment(),
        ))
        .ok()
    }
}

#[allow(dead_code)] // Every desktop target constructs only its own variant; tests cover all paths.
#[derive(Clone, Copy)]
enum CachePlatform {
    MacOs,
    Linux,
    Windows,
    Unsupported,
}

fn configured_media_artifact_cache_root(
    platform: CachePlatform,
    home: Option<PathBuf>,
    xdg_cache_home: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
) -> Option<PathBuf> {
    match platform {
        CachePlatform::MacOs => home.map(|home| {
            home.join("Library")
                .join("Caches")
                .join("Opencut-Reinforced")
                .join("media-artifacts")
        }),
        CachePlatform::Linux => xdg_cache_home
            .or_else(|| home.map(|home| home.join(".cache")))
            .map(|root| root.join("opencut-reinforced").join("media-artifacts")),
        CachePlatform::Windows => local_app_data.map(|root| {
            root.join("Opencut Reinforced")
                .join("Cache")
                .join("media-artifacts")
        }),
        CachePlatform::Unsupported => None,
    }
}

fn non_empty_environment_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn media_artifact_cache_root() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let platform = CachePlatform::MacOs;
    #[cfg(target_os = "linux")]
    let platform = CachePlatform::Linux;
    #[cfg(target_os = "windows")]
    let platform = CachePlatform::Windows;
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let platform = CachePlatform::Unsupported;

    configured_media_artifact_cache_root(
        platform,
        non_empty_environment_path("HOME"),
        non_empty_environment_path("XDG_CACHE_HOME"),
        non_empty_environment_path("LOCALAPPDATA"),
    )
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
        let result = self.query(QueryEnvelope::timeline_clips(
            described.summary.project_id,
            described.summary.project_instance_id,
            track_id,
            offset,
            limit,
        ))?;
        let page = result
            .timeline_clip_page
            .as_ref()
            .ok_or_else(unexpected_response_error)?;
        Ok(timeline_clip_page_view(&result, page))
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
    ) -> ProjectActionResult {
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
                CommandEnvelope::insert_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    ClipId::generate(),
                    track_id,
                    media_id,
                    timeline_start,
                    source_range,
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
                CommandEnvelope::trim_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    match edge {
                        TimelineTrimEdgeView::Start => TimelineTrimEdge::Start,
                        TimelineTrimEdgeView::End => TimelineTrimEdge::End,
                    },
                    timeline_time,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
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

    pub fn request_media_thumbnail(&self, media_id: String) -> MediaArtifactRequestView {
        self.request_media_artifact(media_id, MediaArtifactKindView::Thumbnail)
    }

    pub fn request_media_waveform(&self, media_id: String) -> MediaArtifactRequestView {
        self.request_media_artifact(media_id, MediaArtifactKindView::Waveform)
    }

    pub fn read_media_artifact(
        &self,
        kind: MediaArtifactKindView,
        cache_key: String,
    ) -> Result<Option<MediaArtifactBytesView>, ProjectBridgeError> {
        let cache_key = CacheKey::from_hex(&cache_key).map_err(|error| ProjectBridgeError {
            code: "INVALID_CACHE_KEY".to_owned(),
            message: error.to_string(),
        })?;
        let service = self
            .media_artifact_service
            .as_ref()
            .ok_or_else(cache_unavailable_error)?;
        let bytes = service
            .read_artifact(cache_artifact_kind(kind), cache_key)
            .map_err(|error| ProjectBridgeError {
                code: error.code().to_owned(),
                message: error.to_string(),
            })?;
        Ok(bytes.map(|bytes| MediaArtifactBytesView {
            bytes,
            mime_type: "image/png".to_owned(),
        }))
    }

    pub fn subscribe_media_artifact_events(
        &self,
        sink: StreamSink<MediaArtifactEventView>,
    ) -> Result<(), ProjectBridgeError> {
        let service = self
            .media_artifact_service
            .as_ref()
            .ok_or_else(cache_unavailable_error)?;
        let receiver = service.subscribe_events();
        thread::Builder::new()
            .name("or-flutter-media-artifact-events".to_owned())
            .spawn(move || forward_media_artifact_events(receiver, sink))
            .map_err(|error| ProjectBridgeError {
                code: "EVENT_SUBSCRIPTION_FAILED".to_owned(),
                message: error.to_string(),
            })?;
        Ok(())
    }

    fn request_media_artifact(
        &self,
        media_id: String,
        kind: MediaArtifactKindView,
    ) -> MediaArtifactRequestView {
        let id = match MediaId::from_str(&media_id) {
            Ok(id) => id,
            Err(error) => {
                return MediaArtifactRequestView::failed(
                    media_id,
                    kind,
                    "INVALID_MEDIA_ID",
                    error.to_string(),
                );
            }
        };
        let summary = match self.host.describe() {
            Ok(summary) => summary,
            Err(error) => {
                let error = host_error(error);
                return MediaArtifactRequestView::failed(media_id, kind, error.code, error.message);
            }
        };
        let request = QueryEnvelope::media_get(
            summary.summary.project_id,
            summary.summary.project_instance_id,
            id,
        );
        let item = match self
            .host
            .handle_application_request(ApplicationRequest::Query(request))
        {
            Ok(ApplicationResponse::Query(result)) => match result.media_item {
                Some(item) => *item,
                None => {
                    let error = unexpected_response_error();
                    return MediaArtifactRequestView::failed(
                        media_id,
                        kind,
                        error.code,
                        error.message,
                    );
                }
            },
            Ok(ApplicationResponse::Error(error)) => {
                let error = operation_bridge_error(error);
                return MediaArtifactRequestView::failed(media_id, kind, error.code, error.message);
            }
            Ok(_) => {
                let error = unexpected_response_error();
                return MediaArtifactRequestView::failed(media_id, kind, error.code, error.message);
            }
            Err(error) => {
                let error = host_error(error);
                return MediaArtifactRequestView::failed(media_id, kind, error.code, error.message);
            }
        };
        let Some(service) = self.media_artifact_service.as_ref() else {
            let error = cache_unavailable_error();
            return MediaArtifactRequestView::failed(media_id, kind, error.code, error.message);
        };
        let result = match kind {
            MediaArtifactKindView::Thumbnail => service.request_thumbnail(&item),
            MediaArtifactKindView::Waveform => service.request_waveform(&item),
        };
        match result {
            Ok(result) => MediaArtifactRequestView::from_request(media_id, kind, result),
            Err(error) => {
                MediaArtifactRequestView::failed(media_id, kind, error.code(), error.to_string())
            }
        }
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
        let item = match prepare_media_import(Path::new(&path)) {
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
        self.dispatch_command(CommandEnvelope::add_media(
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

    pub fn close(&mut self, discard_unsaved: bool) -> ProjectActionResult {
        match self.host.shutdown(discard_unsaved) {
            Ok(()) => {
                if let Some(service) = self.media_artifact_service.take() {
                    service.shutdown();
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
            self.host.descriptor_path()?,
        ))
    }
}

impl MediaArtifactRequestView {
    fn failed(
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

    fn from_request(
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

fn cache_artifact_kind(kind: MediaArtifactKindView) -> CacheArtifactKind {
    match kind {
        MediaArtifactKindView::Thumbnail => CacheArtifactKind::Thumbnail,
        MediaArtifactKindView::Waveform => CacheArtifactKind::Waveform,
    }
}

fn cache_unavailable_error() -> ProjectBridgeError {
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
    page: &TimelineClipPage,
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

fn timeline_clip_view(clip: &LegacyTimelineClipState) -> ProjectTimelineClipView {
    ProjectTimelineClipView {
        clip_id: clip.clip_id.to_string(),
        media_id: clip.media_id.to_string(),
        timeline_start: rational_time_view(clip.timeline_start),
        source_start: rational_time_view(clip.source_range.start()),
        source_duration: rational_time_view(clip.source_range.duration()),
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

fn operation_bridge_error(error: OperationError) -> ProjectBridgeError {
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

fn unexpected_response_error() -> ProjectBridgeError {
    ProjectBridgeError {
        code: "UNEXPECTED_RESPONSE".to_owned(),
        message: "The project host returned an unexpected response.".to_owned(),
    }
}

fn view_from_query(
    result: &QueryResult,
    dirty: bool,
    descriptor_path: std::path::PathBuf,
) -> ProjectView {
    ProjectView {
        project_id: result.summary.project_id.to_string(),
        project_instance_id: result.summary.project_instance_id.to_string(),
        revision: result.summary.project_revision.value(),
        name: result.summary.name.clone(),
        dirty,
        descriptor_path: descriptor_path.to_string_lossy().into_owned(),
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

fn forward_media_artifact_events(
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

fn host_error(error: LiveProjectHostError) -> ProjectBridgeError {
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
    use super::{
        CachePlatform, MediaArtifactKindView, configured_media_artifact_cache_root,
        media_artifact_event_view, operation_error_code, rational_time_view,
        timeline_clip_page_view, timeline_marker_page_view, timeline_snap_view,
        timeline_track_view,
    };
    use or_core::{
        CacheArtifactKind, CacheKey, ClipId, JobId, LegacyTimelineClipState, MarkerId,
        MediaArtifactEvent, MediaArtifactEventState, MediaId, OperationErrorCode,
        ParametersFingerprint, ProjectId, ProjectInstanceId, ProjectRevision, ProjectSummary,
        QueryResult, RationalTime, SourceFingerprint, TimeRange, TimelineClipPage,
        TimelineMarkerPage, TimelineMarkerState, TimelineSnapMovingAnchor, TimelineSnapResult,
        TimelineSnapTargetKind, TimelineTrackSummaryV2, TrackId, TrackKind, TrackState,
    };
    use std::path::PathBuf;

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
        let media_id = MediaId::generate();
        let clip_page = TimelineClipPage {
            track_id,
            items: vec![LegacyTimelineClipState {
                clip_id,
                media_id,
                timeline_start: RationalTime::new(3003, 1001).unwrap(),
                source_range: TimeRange::new(
                    RationalTime::new(1, 2).unwrap(),
                    RationalTime::new(5, 2).unwrap(),
                )
                .unwrap(),
            }],
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
        assert_eq!(clip.media_id, media_id.to_string());
        assert_eq!(
            clip.timeline_start,
            rational_time_view(RationalTime::new(3003, 1001).unwrap())
        );
        assert_eq!(
            clip.source_start,
            rational_time_view(RationalTime::new(1, 2).unwrap())
        );
        assert_eq!(
            clip.source_duration,
            rational_time_view(RationalTime::new(5, 2).unwrap())
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
}
