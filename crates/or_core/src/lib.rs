use serde::Serialize;

mod application;
mod jobs;
mod media;
mod media_import;
mod media_probe;
mod project;
mod project_document;
mod project_file_session;
mod project_recovery;
mod project_storage;
mod time;

pub use application::{
    ApplicationRequest, ApplicationResponse, CURRENT_TRANSACTION_SCHEMA_VERSION, ChangeSet,
    CommandCall, CommandDescriptor, CommandEnvelope, CommandResult, MAX_MEDIA_PAGE_SIZE,
    MediaListPage, OperationError, OperationErrorCode, ProjectChange, ProjectSession,
    ProjectSummary, QueryDescriptor, QueryEnvelope, QueryResult, TransactionEnvelope,
    TransactionResult, command_catalog, query_catalog,
};
pub use jobs::{
    JobCancelError, JobCancelOutcome, JobContext, JobFailure, JobId, JobKind, JobManager,
    JobManagerConfig, JobManagerConfigError, JobSnapshot, JobState, JobSubmitError,
};
pub use media::{
    AudioStreamMetadata, MAX_MEDIA_CHANNEL_LAYOUT_BYTES, MAX_MEDIA_CODEC_NAME_BYTES,
    MAX_MEDIA_CODEC_TYPE_BYTES, MAX_MEDIA_FORMAT_NAME_BYTES, MAX_MEDIA_FORMAT_NAMES,
    MAX_MEDIA_PIXEL_FORMAT_BYTES, MAX_MEDIA_SOURCE_URI_BYTES, MAX_MEDIA_STREAMS, MediaId,
    MediaItem, MediaMetadata, MediaMetadataValidationError, MediaSourceRef, MediaSourceUri,
    MediaSourceUriError, MediaStreamMetadata, OtherStreamMetadata, VideoStreamMetadata,
};
pub use media_import::{MediaImportError, prepare_media_import};
pub use media_probe::{MediaProbeError, MediaProbeErrorCode, probe_media_file};
pub use project::{
    ProjectId, ProjectInstanceId, ProjectRevision, ProjectRevisionOverflow, UuidV4ParseError,
};
pub use project_document::{
    CURRENT_PROJECT_SCHEMA_VERSION, ProjectCodecError, ProjectDocument, decode_project,
    encode_project,
};
pub use project_file_session::{
    ProjectFileMediaImportError, ProjectFileSession, ProjectFileSessionError,
    ProjectFileSessionErrorCode,
};
pub use project_recovery::{
    CURRENT_RECOVERY_SCHEMA_VERSION, MAX_RECOVERY_FILE_BYTES, ProjectRecoveryError,
    RecoveryApplyOutcome, RecoveryCandidate, RecoveryConflictReason, RecoveryInspection,
    RecoveryMetadata, apply_project_recovery, discard_project_recovery, inspect_project_recovery,
    write_recovery_checkpoint,
};
pub use project_storage::{
    MAX_PROJECT_FILE_BYTES, ProjectStorageError, TempFileOperation, load_project_file,
    save_project_file_atomic,
};
pub use time::{RationalRate, RationalTime, TimeError, TimeRange};

const APP_NAME: &str = "Opencut Reinforced";
const CORE_API_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub core_api_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthStatus {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Capability {
    pub id: String,
    pub version: u32,
}

pub fn app_info() -> AppInfo {
    AppInfo {
        name: APP_NAME.to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        core_api_version: CORE_API_VERSION,
    }
}

pub fn health() -> HealthStatus {
    HealthStatus {
        status: "ok".to_owned(),
    }
}

pub fn capabilities() -> Vec<Capability> {
    ["core.app_info", "core.health", "core.capabilities"]
        .into_iter()
        .map(|id| Capability {
            id: id.to_owned(),
            version: 1,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{app_info, capabilities, health};
    use std::collections::HashSet;

    #[test]
    fn app_info_has_bootstrap_identity() {
        let info = app_info();

        assert_eq!(info.name, "Opencut Reinforced");
        assert!(!info.version.is_empty());
        assert_eq!(info.core_api_version, 1);
    }

    #[test]
    fn health_is_ok() {
        assert_eq!(health().status, "ok");
    }

    #[test]
    fn capabilities_are_unique_and_limited_to_implemented_core_api() {
        let capabilities = capabilities();
        let ids: Vec<_> = capabilities
            .iter()
            .map(|capability| capability.id.as_str())
            .collect();
        let unique_ids: HashSet<_> = ids.iter().collect();

        assert_eq!(ids, ["core.app_info", "core.health", "core.capabilities"]);
        assert_eq!(unique_ids.len(), capabilities.len());
        assert!(
            capabilities
                .iter()
                .all(|capability| capability.version == 1)
        );
    }
}
