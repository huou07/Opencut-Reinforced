use crate::frb_generated::StreamSink;
use flutter_rust_bridge::frb;
use or_core::{
    ApplicationRequest, ApplicationResponse, CommandEnvelope, MediaId, MediaItem,
    MediaStreamMetadata, OperationError, OperationErrorCode, ProjectFileSession, ProjectId,
    ProjectInstanceId, ProjectRecoveryError, ProjectRevision, QueryEnvelope, QueryResult,
    RecoveryApplyOutcome, RecoveryConflictReason, RecoveryInspection, apply_project_recovery,
    discard_project_recovery, inspect_project_recovery, prepare_media_import,
};
use or_ipc::{LiveProjectHost, LiveProjectHostError, ProjectHostEvent, ProjectHostEventKind};
use std::{path::Path, str::FromStr, sync::mpsc, thread};

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
}

pub fn create_project(path: String, name: String) -> Result<ProjectHostHandle, ProjectBridgeError> {
    let session =
        ProjectFileSession::create_new(Path::new(&path), name).map_err(project_session_error)?;
    LiveProjectHost::start(session, None)
        .map(|host| ProjectHostHandle { host })
        .map_err(host_error)
}

pub fn open_project(path: String) -> Result<ProjectHostHandle, ProjectBridgeError> {
    let session = ProjectFileSession::open(Path::new(&path)).map_err(project_session_error)?;
    LiveProjectHost::start(session, None)
        .map(|host| ProjectHostHandle { host })
        .map_err(host_error)
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
            Ok(()) => ProjectActionResult {
                succeeded: true,
                error_code: String::new(),
                message: String::new(),
                view: None,
            },
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
    }
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
