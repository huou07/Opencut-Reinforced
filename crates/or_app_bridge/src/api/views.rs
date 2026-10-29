//! Bridge view types and their conversions from `or_core` values.
//!
//! They are data only: the opaque handle, its methods and the error conversion
//! live in `project.rs`, which re-exports this module so the generated bridge
//! keeps the same public paths.

#[cfg(target_os = "android")]
use crate::android_media_io::capability_for;
use crate::transcription::{ModelDownload, TranscriptionModelInfo};
use or_core::{JobSnapshot, JobState};

#[cfg(target_os = "android")]
use or_core::{MediaSourceRef, prepare_media_import_from_probe};



#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptionFileFormatView {
    Srt,
    WebVtt,
}

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
pub struct ProjectCaptionImportPreviewView {
    pub format_name: String,
    pub cue_count: u64,
    pub formatting_loss_count: u64,
    pub empty_cues_skipped: u64,
    pub max_caption_text_bytes: u64,
    pub cues: Vec<ProjectCaptionCueView>,
}

#[derive(Clone, Debug)]
pub struct ProjectCaptionCueView {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct ProjectExportJobView {
    pub succeeded: bool,
    pub error_code: String,
    pub message: String,
    pub job_id: String,
    pub state: String,
    pub progress_completed: Option<u64>,
    pub progress_total: Option<u64>,
    pub failure_message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProjectTranscriptionModelView {
    pub model_id: String,
    pub label: String,
    pub version: String,
    pub license: String,
    pub language: String,
    pub size_bytes: u64,
    pub source: String,
    pub installed: bool,
    pub provider_available: bool,
    pub active: bool,
}

/// One selectable speech model in the app's picker.
#[derive(Clone, Debug)]
pub struct ProjectTranscriptionModelOptionView {
    pub model_id: String,
    pub label: String,
    pub version: String,
    pub license: String,
    pub language: String,
    pub size_bytes: u64,
    pub source: String,
    pub installed: bool,
    pub provider_available: bool,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub struct ProjectModelDownloadView {
    pub download_id: String,
    pub model_id: String,
    pub destination: String,
    pub source: String,
    pub size_bytes: u64,
    pub license: String,
}

#[derive(Clone, Debug)]
pub struct ProjectTranscriptionJobView {
    pub job_id: String,
    pub state: String,
    pub progress_completed: Option<u64>,
    pub progress_total: Option<u64>,
    pub failure_message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProjectTranscriptSegmentView {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub confidence: f32,
}

#[derive(Clone, Debug)]
pub struct ProjectTranscriptProposalView {
    pub project_id: String,
    pub project_revision: u64,
    pub media_id: String,
    pub media_sha256: String,
    pub provider_id: String,
    pub model_id: String,
    pub model_version: String,
    pub model_license: String,
    pub language: String,
    pub max_caption_text_bytes: u64,
    pub segments: Vec<ProjectTranscriptSegmentView>,
}

pub(crate) fn transcription_model_view(info: TranscriptionModelInfo) -> ProjectTranscriptionModelView {
    ProjectTranscriptionModelView {
        model_id: info.model_id,
        label: info.label,
        version: info.version,
        license: info.license,
        language: info.language,
        size_bytes: info.size_bytes,
        source: info.source,
        installed: info.installed,
        provider_available: info.provider_available,
        active: info.active,
    }
}

pub(crate) fn transcription_model_option_view(
    info: TranscriptionModelInfo,
) -> ProjectTranscriptionModelOptionView {
    ProjectTranscriptionModelOptionView {
        model_id: info.model_id,
        label: info.label,
        version: info.version,
        license: info.license,
        language: info.language,
        size_bytes: info.size_bytes,
        source: info.source,
        installed: info.installed,
        provider_available: info.provider_available,
        active: info.active,
    }
}

pub(crate) fn model_download_view(download: ModelDownload) -> ProjectModelDownloadView {
    ProjectModelDownloadView {
        download_id: download.download_id,
        model_id: download.model_id,
        destination: download.destination,
        source: download.source,
        size_bytes: download.size_bytes,
        license: download.license,
    }
}

pub(crate) fn transcription_job_view(job: JobSnapshot) -> ProjectTranscriptionJobView {
    ProjectTranscriptionJobView {
        job_id: job.id.to_string(),
        state: job_state_name(job.state).to_owned(),
        progress_completed: job.progress.map(|progress| progress.completed),
        progress_total: job.progress.map(|progress| progress.total),
        failure_message: job.failure_message,
    }
}

pub(crate) fn job_state_name(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Running => "running",
        JobState::Succeeded => "succeeded",
        JobState::Failed => "failed",
        JobState::Cancelled => "cancelled",
    }
}

pub(crate) fn transcription_proposal_view(
    proposal: or_ai::TranscriptProposal,
) -> ProjectTranscriptProposalView {
    ProjectTranscriptProposalView {
        project_id: proposal.project_id.to_string(),
        project_revision: proposal.project_revision,
        media_id: proposal.media_id.to_string(),
        media_sha256: proposal.media_sha256,
        provider_id: proposal.provider_id,
        model_id: proposal.model_id,
        model_version: proposal.model_version,
        model_license: proposal.model_license,
        language: proposal.language,
        max_caption_text_bytes: or_core::MAX_TIMELINE_CAPTION_BYTES as u64,
        segments: proposal
            .segments
            .into_iter()
            .map(|segment| ProjectTranscriptSegmentView {
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
                text: segment.text,
                confidence: segment.confidence,
            })
            .collect(),
    }
}

pub(crate) fn transcription_error(
    error: crate::transcription::TranscriptionServiceError,
) -> ProjectBridgeError {
    ProjectBridgeError {
        code: "TRANSCRIPTION_UNAVAILABLE".to_owned(),
        message: error.to_string(),
    }
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

#[derive(Clone, Copy, Debug)]
pub enum PreviewPreparationActionView {
    Seek,
    Play,
    StepPrevious,
    StepNext,
    Tick,
}

#[derive(Clone, Debug)]
pub struct ProjectPreviewPreparationView {
    pub request_id: Option<u64>,
    pub sources: Vec<String>,
    pub state: ProjectPreviewStateView,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimelineClipContentKindView {
    Media,
    Text,
    Caption,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontIdentityView {
    BundledInter,
    BeVietnamPro,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextWeightView {
    Regular,
    Medium,
    Semibold,
    Bold,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextAlignmentView {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectTextColorView {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextVerticalAnchorView {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectTextBackgroundView {
    pub color: ProjectTextColorView,
    pub padding_milli_points: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectTextFormattingView {
    pub font: FontIdentityView,
    pub size_milli_points: u32,
    pub weight: TextWeightView,
    pub alignment: TextAlignmentView,
    pub color: ProjectTextColorView,
    pub anchor: TextVerticalAnchorView,
    pub offset_x_basis_points: i16,
    pub offset_y_basis_points: i16,
    pub background: Option<ProjectTextBackgroundView>,
    pub opacity_basis_points: u16,
}

#[derive(Clone, Debug)]
pub struct ProjectTimelineClipView {
    pub clip_id: String,
    pub content_kind: TimelineClipContentKindView,
    pub playback_rate: RationalRateView,
    /// True when the clip walks its source backwards.
    pub reversed: bool,
    pub frozen: bool,
    pub media_id: Option<String>,
    pub text: Option<String>,
    pub formatting: Option<ProjectTextFormattingView>,
    pub timeline_start: RationalTimeView,
    pub timeline_duration: RationalTimeView,
    pub source_start: Option<RationalTimeView>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectTimelineVisualSettingsView {
    pub x_milli_canvas: i32,
    pub y_milli_canvas: i32,
    pub scale_x_milli: u32,
    pub scale_y_milli: u32,
    pub rotation_milli_degrees: i32,
    pub anchor_x_basis_points: u16,
    pub anchor_y_basis_points: u16,
    pub crop_left_basis_points: u16,
    pub crop_top_basis_points: u16,
    pub crop_right_basis_points: u16,
    pub crop_bottom_basis_points: u16,
    pub opacity_basis_points: u16,
    pub brightness_amount_milli: i16,
    pub contrast_amount_milli: u16,
    pub saturation_amount_milli: u16,
    pub gaussian_blur_radius_milli: u32,
    pub update_brightness: bool,
    pub update_contrast: bool,
    pub update_saturation: bool,
    pub update_gaussian_blur: bool,
    pub transition_in_kind: u8,
    pub transition_in_duration: RationalTimeView,
    pub transition_out_kind: u8,
    pub transition_out_duration: RationalTimeView,
    pub update_transition_in: bool,
    pub update_transition_out: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectTimelineAudioSettingsView {
    pub gain_millidecibels: i32,
    pub pan_basis_points: i16,
    pub fade_in: RationalTimeView,
    pub fade_out: RationalTimeView,
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

/// The outcome of reducing noise in a media file.
///
/// A failure carries the caller-facing code and message; a success carries what was
/// written and the floor the filter was given, so a client can show both and a test
/// can assert the derivation.
#[derive(Clone, Debug)]
pub struct MediaCleanupResultView {
    pub succeeded: bool,
    pub error_code: String,
    pub message: String,
    pub destination: String,
    pub noise_floor_db: i32,
    pub reduction_db: u32,
    pub bytes_written: u64,
}

impl MediaCleanupResultView {
    pub(crate) fn failure(code: &str, message: impl Into<String>) -> Self {
        Self {
            succeeded: false,
            error_code: code.to_owned(),
            message: message.into(),
            destination: String::new(),
            noise_floor_db: 0,
            reduction_db: 0,
            bytes_written: 0,
        }
    }
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
