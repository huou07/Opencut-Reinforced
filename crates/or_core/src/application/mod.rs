use crate::{
    ClipContent, ClipId, MAX_TIMELINE_MARKER_LABEL_BYTES, MarkerId, MediaId, MediaItem,
    ProjectDocument, ProjectId, ProjectInstanceId, ProjectRevision, RationalRate, RationalTime,
    TimeRange, TimelineClip, TimelineTrack, TrackId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{cmp::Ordering, collections::HashSet};

const PROJECT_RENAME_ID: &str = "project.rename";
const PROJECT_SUMMARY_ID: &str = "project.summary";
const HISTORY_UNDO_ID: &str = "history.undo";
const HISTORY_REDO_ID: &str = "history.redo";
const MEDIA_ADD_ID: &str = "media.add";
const MEDIA_REMOVE_ID: &str = "media.remove";
const MEDIA_RELINK_ID: &str = "media.relink";
const MEDIA_LIST_ID: &str = "media.list";
const MEDIA_GET_ID: &str = "media.get";
const TIMELINE_TRACK_ADD_ID: &str = "timeline.track.add";
const TIMELINE_TRACK_REMOVE_ID: &str = "timeline.track.remove";
const TIMELINE_TRACK_SET_STATE_ID: &str = "timeline.track.set_state";
const TIMELINE_TRACK_MOVE_ID: &str = "timeline.track.move";
const TIMELINE_CLIP_INSERT_ID: &str = "timeline.clip.insert";
const TIMELINE_CLIP_INSERT_CONTENT_ID: &str = "timeline.clip.insert_content";
const TIMELINE_CAPTIONS_IMPORT_ID: &str = "timeline.captions.import";
const TIMELINE_CLIP_MOVE_ID: &str = "timeline.clip.move";
const TIMELINE_CLIPS_MOVE_ID: &str = "timeline.clips.move";
const TIMELINE_CLIP_UPDATE_ID: &str = "timeline.clip.update";
const TIMELINE_CLIP_DELETE_ID: &str = "timeline.clip.delete";
const TIMELINE_CLIP_TRIM_ID: &str = "timeline.clip.trim";
const TIMELINE_CLIP_SPLIT_ID: &str = "timeline.clip.split";
const TIMELINE_CLIP_DUCK_ID: &str = "timeline.clip.duck";
const TIMELINE_CLIP_SLIP_ID: &str = "timeline.clip.slip";
const TIMELINE_CLIP_ROLL_ID: &str = "timeline.clip.roll";
const TIMELINE_CLIP_SLIDE_ID: &str = "timeline.clip.slide";
const TIMELINE_CLIP_RIPPLE_DELETE_ID: &str = "timeline.clip.ripple_delete";
const TIMELINE_MARKER_ADD_ID: &str = "timeline.marker.add";
const TIMELINE_MARKER_MOVE_ID: &str = "timeline.marker.move";
const TIMELINE_MARKER_RENAME_ID: &str = "timeline.marker.rename";
const TIMELINE_MARKER_DELETE_ID: &str = "timeline.marker.delete";
const TIMELINE_TRACKS_ID: &str = "timeline.tracks";
const TIMELINE_CLIPS_ID: &str = "timeline.clips";
const TIMELINE_SNAP_ID: &str = "timeline.snap";
const TIMELINE_MARKERS_ID: &str = "timeline.markers";
const TIMELINE_SEQUENCE_SET_FRAME_RATE_ID: &str = "timeline.sequence.set_frame_rate";
const TIMELINE_SEQUENCE_SETTINGS_ID: &str = "timeline.sequence.settings";
const OPERATION_SCHEMA_VERSION: u64 = 1;
pub const MAX_MEDIA_PAGE_SIZE: usize = 100;
pub const MAX_TIMELINE_CLIP_PAGE_SIZE: usize = 100;
pub const MAX_TIMELINE_MARKER_PAGE_SIZE: usize = 100;
pub const CURRENT_TRANSACTION_SCHEMA_VERSION: u64 = 1;

mod history;
mod media;
mod queries;
mod timeline;
mod types;
use types::*;
pub use types::*;

#[cfg(test)]
mod export_profile_tests {
    use super::{ExportProfile, ExportQuality, ExportRequest};
    use crate::{ProjectId, ProjectInstanceId};

    #[test]
    fn legacy_export_start_defaults_to_the_lossless_profile() {
        let request: ExportRequest = serde_json::from_value(serde_json::json!({
            "operation": "start",
            "request": {
                "project_id": ProjectId::generate(),
                "project_instance_id": ProjectInstanceId::generate(),
                "expected_project_revision": 0,
                "destination": "/tmp/legacy.mkv"
            }
        }))
        .unwrap();

        assert!(matches!(
            request,
            ExportRequest::Start {
                profile: ExportProfile::MatroskaFfv1PcmS16le,
                quality: ExportQuality::Quality,
                ..
            }
        ));
    }

    #[test]
    fn an_export_start_carries_a_quality_preference_and_defaults_to_quality() {
        assert_eq!(ExportQuality::default(), ExportQuality::Quality);

        let request: ExportRequest = serde_json::from_value(serde_json::json!({
            "operation": "start",
            "request": {
                "project_id": ProjectId::generate(),
                "project_instance_id": ProjectInstanceId::generate(),
                "expected_project_revision": 3,
                "destination": "/tmp/faster.webm",
                "profile": "webm_vp9_opus",
                "quality": "speed"
            }
        }))
        .unwrap();
        assert!(matches!(
            request,
            ExportRequest::Start {
                profile: ExportProfile::WebmVp9Opus,
                quality: ExportQuality::Speed,
                ..
            }
        ));

        // A request that names no quality keeps the default, which is the quality
        // preference: an older client must not silently get the fastest mode.
        let value = serde_json::to_value(ExportRequest::Start {
            project_id: ProjectId::generate(),
            project_instance_id: ProjectInstanceId::generate(),
            expected_project_revision: crate::ProjectRevision::new(3),
            destination: "/tmp/default.webm".to_owned(),
            profile: ExportProfile::WebmVp9Opus,
            quality: ExportQuality::default(),
        })
        .unwrap();
        assert_eq!(value["request"]["quality"], "quality");
    }

    #[test]
    fn motion_materialize_round_trips_on_the_application_contract() {
        use super::{
            ApplicationRequest, ApplicationResponse, MotionMaterializeRequest,
            MotionMaterializeResponse, RationalTime,
        };
        let project_id = ProjectId::generate();
        let project_instance_id = ProjectInstanceId::generate();
        let request = ApplicationRequest::MotionMaterialize(MotionMaterializeRequest::FromCode {
            project_id,
            project_instance_id,
            source_path: "/tmp/sample.rs".to_owned(),
            destination: "/tmp/out.mkv".to_owned(),
            frame_rate_numerator: 12,
            frame_rate_denominator: 1,
            width: 640,
            height: 360,
            title: "Fixture".to_owned(),
        });
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["kind"], "motion_materialize");
        assert_eq!(value["request"]["operation"], "from_code");
        let decoded: ApplicationRequest = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, request);

        let response = ApplicationResponse::MotionMaterialize(MotionMaterializeResponse {
            succeeded: true,
            error_code: String::new(),
            message: "Rendered.".to_owned(),
            output_path: "/tmp/out.mkv".to_owned(),
            scene_id: "scene".to_owned(),
            scene_name: "Fixture".to_owned(),
            width: 640,
            height: 360,
            frame_rate_numerator: 12,
            frame_rate_denominator: 1,
            frame_count: 24,
            duration: RationalTime::new(2, 1).unwrap(),
            bytes_written: 1024,
        });
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["kind"], "motion_materialize");
        let decoded: ApplicationResponse = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, response);
    }

    #[test]
    fn transcription_round_trips_on_the_application_contract() {
        use super::{
            ApplicationRequest, ApplicationResponse, TranscriptionJobView,
            TranscriptionProposalView, TranscriptionRequest, TranscriptionResponse,
            TranscriptionSegmentView,
        };
        use crate::{MediaId, ProjectRevision};
        let project_id = ProjectId::generate();
        let project_instance_id = ProjectInstanceId::generate();
        let media_id = MediaId::generate();
        let job_id = crate::JobId::generate();
        let request = ApplicationRequest::Transcription(TranscriptionRequest::Start {
            project_id,
            project_instance_id,
            expected_project_revision: ProjectRevision::new(3),
            media_id,
        });
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["kind"], "transcription");
        assert_eq!(value["request"]["operation"], "start");
        let decoded: ApplicationRequest = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, request);

        let response = ApplicationResponse::Transcription(TranscriptionResponse::Proposal(
            TranscriptionProposalView {
                project_id,
                project_revision: ProjectRevision::new(3),
                media_id,
                provider_id: "whisper.cpp".to_owned(),
                model_id: "whisper-small-multilingual".to_owned(),
                model_version: "1.0".to_owned(),
                model_license: "MIT".to_owned(),
                language: "vi".to_owned(),
                segments: vec![TranscriptionSegmentView {
                    start_ms: 0,
                    end_ms: 1200,
                    text: "Xin chào".to_owned(),
                }],
            },
        ));
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["kind"], "transcription");
        assert_eq!(value["result"]["status"], "proposal");
        let decoded: ApplicationResponse = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, response);

        let job =
            ApplicationResponse::Transcription(TranscriptionResponse::Job(TranscriptionJobView {
                job_id,
                state: crate::JobState::Running,
                failure_message: None,
            }));
        assert_eq!(
            serde_json::to_value(&job).unwrap()["result"]["body"]["state"],
            "running"
        );
    }

    #[test]
    fn webm_profile_has_a_stable_wire_name() {
        assert_eq!(
            serde_json::to_value(ExportProfile::WebmVp9Opus).unwrap(),
            serde_json::Value::String("webm_vp9_opus".to_owned())
        );
    }
}

/// Result for an export request. Export work never changes the project revision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportResponse {
    pub succeeded: bool,
    pub error_code: String,
    pub message: String,
    pub job: Option<crate::JobSnapshot>,
}

impl ExportResponse {
    pub fn success(message: impl Into<String>, job: crate::JobSnapshot) -> Self {
        Self {
            succeeded: true,
            error_code: String::new(),
            message: message.into(),
            job: Some(job),
        }
    }

    pub fn failure(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            succeeded: false,
            error_code: code.into(),
            message: message.into(),
            job: None,
        }
    }
}

/// The typed result of dispatching an [`ApplicationRequest`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "result",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ApplicationResponse {
    Command(CommandResult),
    Query(QueryResult),
    Transaction(TransactionResult),
    Export(ExportResponse),
    MotionMaterialize(MotionMaterializeResponse),
    Transcription(TranscriptionResponse),
    Error(OperationError),
}

/// Result of materialising a MotionScene. It never changes the project revision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionMaterializeResponse {
    pub succeeded: bool,
    pub error_code: String,
    pub message: String,
    pub output_path: String,
    pub scene_id: String,
    pub scene_name: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate_numerator: u32,
    pub frame_rate_denominator: u32,
    pub frame_count: u64,
    pub duration: RationalTime,
    pub bytes_written: u64,
}

impl MotionMaterializeResponse {
    pub fn failure(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            succeeded: false,
            error_code: code.into(),
            message: message.into(),
            output_path: String::new(),
            scene_id: String::new(),
            scene_name: String::new(),
            width: 0,
            height: 0,
            frame_rate_numerator: 0,
            frame_rate_denominator: 0,
            frame_count: 0,
            duration: RationalTime::ZERO,
            bytes_written: 0,
        }
    }
}

/// A runtime-only local transcription request routed through the same
/// application and IPC path as edits.
///
/// Only the application runtime links the media decoder and the local model, so
/// the CLI and agents ask the host to transcribe instead of linking a second
/// pipeline. A transcription never changes the project; the reviewed cues are
/// applied later through the ordinary caption import command.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "request",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TranscriptionRequest {
    Start {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        media_id: MediaId,
    },
    Status {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        job_id: crate::JobId,
    },
    Proposal {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        job_id: crate::JobId,
    },
    Cancel {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        job_id: crate::JobId,
    },
}

/// One background transcription job as the host reports it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptionJobView {
    pub job_id: crate::JobId,
    pub state: crate::JobState,
    pub failure_message: Option<String>,
}

/// One reviewed transcript cue. Confidence is intentionally not part of this
/// transport contract: the UI shows it from its own bounded view, and no
/// command consumes it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptionSegmentView {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// A provider-independent transcript proposal bound to the exact project,
/// revision and media the host asked about.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptionProposalView {
    pub project_id: ProjectId,
    pub project_revision: ProjectRevision,
    pub media_id: MediaId,
    pub provider_id: String,
    pub model_id: String,
    pub model_version: String,
    pub model_license: String,
    pub language: String,
    pub segments: Vec<TranscriptionSegmentView>,
}

/// Outcome of one transcription request. It never carries executable edits.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "body",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TranscriptionResponse {
    Job(TranscriptionJobView),
    Proposal(TranscriptionProposalView),
    Failure(TranscriptionFailure),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptionFailure {
    pub code: String,
    pub message: String,
}

impl TranscriptionFailure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy)]
enum HistoryDirection {
    Undo,
    Redo,
}

/// One loaded runtime instance of a canonical project document.
///
/// The instance ID and transaction history are session-only and never part of the
/// persisted document.
#[derive(Debug, Eq, PartialEq)]
pub struct ProjectSession {
    project: ProjectDocument,
    project_instance_id: ProjectInstanceId,
    history: SessionHistory,
}

impl ProjectSession {
    /// Opens a document into a new runtime instance without changing its state.
    pub fn open(project: ProjectDocument) -> Self {
        Self {
            project,
            project_instance_id: ProjectInstanceId::generate(),
            history: SessionHistory::default(),
        }
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project.id()
    }

    pub const fn project_instance_id(&self) -> ProjectInstanceId {
        self.project_instance_id
    }

    pub const fn project_revision(&self) -> ProjectRevision {
        self.project.revision()
    }

    pub const fn project(&self) -> &ProjectDocument {
        &self.project
    }

    /// Dispatches a supported application operation through the existing semantic paths.
    pub fn handle_application_request(
        &mut self,
        request: ApplicationRequest,
    ) -> ApplicationResponse {
        match request {
            ApplicationRequest::Command(envelope) => self
                .execute_command(envelope)
                .map(ApplicationResponse::Command)
                .unwrap_or_else(ApplicationResponse::Error),
            ApplicationRequest::Query(envelope) => self
                .execute_query(envelope)
                .map(ApplicationResponse::Query)
                .unwrap_or_else(ApplicationResponse::Error),
            ApplicationRequest::Transaction(envelope) => self
                .execute_transaction(envelope)
                .map(ApplicationResponse::Transaction)
                .unwrap_or_else(ApplicationResponse::Error),
            ApplicationRequest::Export(_) => ApplicationResponse::Export(ExportResponse::failure(
                "EXPORT_RUNTIME_UNAVAILABLE",
                "the project host has no export runtime",
            )),
            ApplicationRequest::MotionMaterialize(_) => {
                ApplicationResponse::MotionMaterialize(MotionMaterializeResponse::failure(
                    "MOTION_RUNTIME_UNAVAILABLE",
                    "the project host has no motion materialisation runtime",
                ))
            }
            ApplicationRequest::Transcription(_) => ApplicationResponse::Transcription(
                TranscriptionResponse::Failure(TranscriptionFailure::new(
                    "TRANSCRIPTION_RUNTIME_UNAVAILABLE",
                    "the project host has no transcription runtime",
                )),
            ),
        }
    }

    pub fn into_project(self) -> ProjectDocument {
        self.project
    }

    /// Dispatches a versioned command after validating its project/session state.
    pub fn execute_command(
        &mut self,
        envelope: CommandEnvelope,
    ) -> Result<CommandResult, OperationError> {
        let descriptor = COMMANDS
            .iter()
            .find(|descriptor| descriptor.id == envelope.command_id.as_str())
            .ok_or_else(|| OperationError::new(OperationErrorCode::UnknownCommand))?;
        let schema_supported = if descriptor.id == TIMELINE_TRACK_ADD_ID {
            matches!(envelope.schema_version, 1 | 2)
        } else {
            envelope.schema_version == descriptor.schema_version
        };
        if !schema_supported {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedCommandSchema,
            ));
        }
        self.check_project_preconditions(
            envelope.project_id,
            envelope.project_instance_id,
            envelope.expected_project_revision,
        )?;

        let command_id = envelope.command_id;
        let schema_version = envelope.schema_version;
        let before_revision = self.project_revision();
        let change_set = match command_id.as_str() {
            PROJECT_RENAME_ID => {
                let call = CommandCall {
                    command_id: command_id.clone(),
                    schema_version,
                    arguments: envelope.arguments,
                };
                self.apply_forward_commands(std::slice::from_ref(&call))?.1
            }
            HISTORY_UNDO_ID => {
                parse_empty_arguments(envelope.arguments)?;
                self.apply_history(HistoryDirection::Undo)?.2
            }
            HISTORY_REDO_ID => {
                parse_empty_arguments(envelope.arguments)?;
                self.apply_history(HistoryDirection::Redo)?.2
            }
            MEDIA_ADD_ID => self.apply_media_add(envelope.arguments)?,
            MEDIA_REMOVE_ID => self.apply_media_remove(envelope.arguments)?,
            MEDIA_RELINK_ID => self.apply_media_relink(envelope.arguments)?,
            TIMELINE_TRACK_ADD_ID => {
                self.apply_timeline_track_add(envelope.arguments, schema_version)?
            }
            TIMELINE_TRACK_REMOVE_ID => self.apply_timeline_track_remove(envelope.arguments)?,
            TIMELINE_TRACK_SET_STATE_ID => {
                self.apply_timeline_track_set_state(envelope.arguments)?
            }
            TIMELINE_TRACK_MOVE_ID => self.apply_timeline_track_move(envelope.arguments)?,
            TIMELINE_CLIP_INSERT_ID => self.apply_timeline_clip_insert(envelope.arguments)?,
            TIMELINE_CLIP_INSERT_CONTENT_ID => {
                self.apply_timeline_clip_insert_content(envelope.arguments)?
            }
            TIMELINE_CAPTIONS_IMPORT_ID => {
                self.apply_timeline_captions_import(envelope.arguments)?
            }
            TIMELINE_CLIP_MOVE_ID => self.apply_timeline_clip_move(envelope.arguments)?,
            TIMELINE_CLIPS_MOVE_ID => self.apply_timeline_clips_move(envelope.arguments)?,
            TIMELINE_CLIP_UPDATE_ID => self.apply_timeline_clip_update(envelope.arguments)?,
            TIMELINE_CLIP_DELETE_ID => self.apply_timeline_clip_delete(envelope.arguments)?,
            TIMELINE_CLIP_TRIM_ID => self.apply_timeline_clip_trim(envelope.arguments)?,
            TIMELINE_CLIP_SPLIT_ID => self.apply_timeline_clip_split(envelope.arguments)?,
            TIMELINE_CLIP_DUCK_ID => self.apply_timeline_clip_duck(envelope.arguments)?,
            TIMELINE_CLIP_SLIP_ID => self.apply_timeline_clip_slip(envelope.arguments)?,
            TIMELINE_CLIP_ROLL_ID => self.apply_timeline_clip_roll(envelope.arguments)?,
            TIMELINE_CLIP_SLIDE_ID => self.apply_timeline_clip_slide(envelope.arguments)?,
            TIMELINE_CLIP_RIPPLE_DELETE_ID => {
                self.apply_timeline_clip_ripple_delete(envelope.arguments)?
            }
            TIMELINE_MARKER_ADD_ID => self.apply_timeline_marker_add(envelope.arguments)?,
            TIMELINE_MARKER_MOVE_ID => self.apply_timeline_marker_move(envelope.arguments)?,
            TIMELINE_MARKER_RENAME_ID => self.apply_timeline_marker_rename(envelope.arguments)?,
            TIMELINE_MARKER_DELETE_ID => self.apply_timeline_marker_delete(envelope.arguments)?,
            TIMELINE_SEQUENCE_SET_FRAME_RATE_ID => {
                self.apply_timeline_sequence_set_frame_rate(envelope.arguments)?
            }
            _ => return Err(OperationError::new(OperationErrorCode::UnknownCommand)),
        };

        let after_revision = self.project_revision();
        Ok(CommandResult {
            command_id,
            schema_version,
            project_id: self.project_id(),
            project_instance_id: self.project_instance_id,
            before_revision,
            after_revision,
            changed: !change_set.is_empty(),
            change_set,
        })
    }

    /// Evaluates a command group in staged state and commits its net change atomically.
    pub fn execute_transaction(
        &mut self,
        envelope: TransactionEnvelope,
    ) -> Result<TransactionResult, OperationError> {
        if envelope.schema_version != CURRENT_TRANSACTION_SCHEMA_VERSION {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedTransactionSchema,
            ));
        }
        self.check_project_preconditions(
            envelope.project_id,
            envelope.project_instance_id,
            envelope.expected_project_revision,
        )?;
        if envelope.commands.is_empty() {
            return Err(OperationError::new(OperationErrorCode::EmptyTransaction));
        }

        let before_revision = self.project_revision();
        let command_count = envelope.commands.len();
        let (after_revision, change_set) = self.apply_forward_commands(&envelope.commands)?;
        Ok(TransactionResult {
            schema_version: CURRENT_TRANSACTION_SCHEMA_VERSION,
            project_id: self.project_id(),
            project_instance_id: self.project_instance_id,
            before_revision,
            after_revision,
            changed: !change_set.is_empty(),
            command_count,
            change_set,
        })
    }

    /// Dispatches and executes a read-only query against current session state.
    pub fn execute_query(&self, envelope: QueryEnvelope) -> Result<QueryResult, OperationError> {
        let descriptor = QUERIES
            .iter()
            .find(|descriptor| descriptor.id == envelope.query_id)
            .ok_or_else(|| OperationError::new(OperationErrorCode::UnknownQuery))?;
        let schema_supported = if matches!(
            envelope.query_id.as_str(),
            TIMELINE_SNAP_ID | TIMELINE_TRACKS_ID | TIMELINE_CLIPS_ID
        ) {
            matches!(envelope.schema_version, 1 | 2)
        } else {
            envelope.schema_version == descriptor.schema_version
        };
        if !schema_supported {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedQuerySchema,
            ));
        }
        if envelope.project_id != self.project_id() {
            return Err(OperationError::new(OperationErrorCode::ProjectIdMismatch));
        }
        if envelope.project_instance_id != self.project_instance_id {
            return Err(OperationError::new(
                OperationErrorCode::ProjectInstanceMismatch,
            ));
        }
        let mut result = QueryResult {
            query_id: envelope.query_id,
            schema_version: envelope.schema_version,
            summary: ProjectSummary {
                project_id: self.project_id(),
                project_instance_id: self.project_instance_id,
                project_revision: self.project_revision(),
                name: self.project.name().to_owned(),
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

        if result.query_id == PROJECT_SUMMARY_ID {
            if !is_empty_object(&envelope.arguments) {
                return Err(OperationError::new(OperationErrorCode::InvalidArguments));
            }
        } else if result.query_id == MEDIA_LIST_ID {
            result.media_page = Some(self.media_list(envelope.arguments)?);
        } else if result.query_id == MEDIA_GET_ID {
            result.media_item = Some(Box::new(self.media_get(envelope.arguments)?));
        } else if result.query_id == TIMELINE_TRACKS_ID {
            if envelope.schema_version == 2 {
                result.timeline_tracks_v2 = Some(self.timeline_tracks_v2(envelope.arguments)?);
            } else {
                result.timeline_tracks = Some(self.timeline_tracks(envelope.arguments)?);
            }
        } else if result.query_id == TIMELINE_CLIPS_ID {
            if envelope.schema_version == 2 {
                result.timeline_clip_page_v2 =
                    Some(Box::new(self.timeline_clips_v2(envelope.arguments)?));
            } else {
                result.timeline_clip_page =
                    Some(Box::new(self.timeline_clips(envelope.arguments)?));
            }
        } else if result.query_id == TIMELINE_SNAP_ID {
            result.timeline_snap = Some(Box::new(
                self.timeline_snap(envelope.arguments, envelope.schema_version == 2)?,
            ));
        } else if result.query_id == TIMELINE_MARKERS_ID {
            result.timeline_marker_page =
                Some(Box::new(self.timeline_markers(envelope.arguments)?));
        } else if result.query_id == TIMELINE_SEQUENCE_SETTINGS_ID {
            if !is_empty_object(&envelope.arguments) {
                return Err(OperationError::new(OperationErrorCode::InvalidArguments));
            }
            result.timeline_sequence_settings = Some(Box::new(TimelineSequenceSettings {
                sequence_frame_rate: self.project.timeline().sequence_frame_rate(),
            }));
        } else {
            return Err(OperationError::new(OperationErrorCode::UnknownQuery));
        }

        Ok(result)
    }

    fn check_project_preconditions(
        &self,
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_revision: ProjectRevision,
    ) -> Result<(), OperationError> {
        if project_id != self.project_id() {
            return Err(OperationError::new(OperationErrorCode::ProjectIdMismatch));
        }
        if project_instance_id != self.project_instance_id {
            return Err(OperationError::new(
                OperationErrorCode::ProjectInstanceMismatch,
            ));
        }
        if expected_revision != self.project_revision() {
            return Err(OperationError::revision_conflict(self.project_revision()));
        }
        Ok(())
    }

    fn apply_forward_commands(
        &mut self,
        commands: &[CommandCall],
    ) -> Result<(ProjectRevision, ChangeSet), OperationError> {
        let before_revision = self.project_revision();
        let original_name = self.project.name().to_owned();
        let mut staged_name = original_name.clone();
        for command in commands {
            stage_groupable_command(command, &mut staged_name)?;
        }

        let change_set = ChangeSet::project_name(&original_name, &staged_name);
        if change_set.is_empty() {
            return Ok((before_revision, change_set));
        }

        let after_revision = before_revision
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let history_entry = change_set.clone();
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        // All validation, staging, revision checks, and history allocation are complete.
        self.project.rename_for_command(staged_name, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok((after_revision, change_set))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RationalTimeArguments {
    numerator: i64,
    denominator: u32,
}

impl RationalTimeArguments {
    pub(super) fn into_time(self) -> Result<RationalTime, OperationError> {
        RationalTime::new(self.numerator, self.denominator)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))
    }
}

/// How an insert treats the range it lands on.
///
/// The mode is an explicit command input, never canonical project state: the same
/// project can be edited in either mode, and the mode a client is using is that
/// client's presentation choice.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEditMode {
    /// Refuse to overlap: the target range must be free.
    #[default]
    Insert,
    /// Replace the target range: covered clips are removed, crossed clips are
    /// trimmed, and a clip strictly containing the new span is split.
    Overwrite,
}

#[derive(Clone, Copy)]
enum TimelineSnapMode {
    Move { duration: RationalTime },
    Trim { edge: TimelineTrimEdge },
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SessionHistory {
    undo: Vec<ChangeSet>,
    redo: Vec<ChangeSet>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameArguments {
    name: String,
}

pub(super) fn is_empty_object(arguments: &Value) -> bool {
    arguments
        .as_object()
        .is_some_and(|arguments| arguments.is_empty())
}

pub(super) fn stage_groupable_command(
    command: &CommandCall,
    staged_name: &mut String,
) -> Result<(), OperationError> {
    let descriptor = COMMANDS
        .iter()
        .find(|descriptor| descriptor.id == command.command_id.as_str())
        .ok_or_else(|| OperationError::new(OperationErrorCode::UnknownCommand))?;
    let schema_supported = if descriptor.id == TIMELINE_TRACK_ADD_ID {
        matches!(command.schema_version, 1 | 2)
    } else {
        command.schema_version == descriptor.schema_version
    };
    if !schema_supported {
        return Err(OperationError::new(
            OperationErrorCode::UnsupportedCommandSchema,
        ));
    }
    if !descriptor.allowed_in_transaction {
        return Err(OperationError::new(
            OperationErrorCode::CommandNotAllowedInTransaction,
        ));
    }

    let arguments: RenameArguments = serde_json::from_value(command.arguments.clone())
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
    *staged_name = arguments.name;
    Ok(())
}

pub(super) fn parse_empty_arguments(arguments: Value) -> Result<(), OperationError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct EmptyArguments {}

    serde_json::from_value::<EmptyArguments>(arguments)
        .map(|_| ())
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))
}

#[cfg(test)]
mod tests;
