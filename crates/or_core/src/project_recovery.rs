use crate::project_storage::{atomic_replace_bytes, parent_directory};
use crate::{
    MAX_PROJECT_FILE_BYTES, ProjectCodecError, ProjectDocument, ProjectId, ProjectRevision,
    ProjectStorageError, decode_project, encode_project, load_project_file,
    save_project_file_atomic,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{
    error::Error,
    ffi::OsString,
    fmt,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

const RECOVERY_FORMAT_MARKER: &str = "opencut-reinforced-recovery";
pub const CURRENT_RECOVERY_SCHEMA_VERSION: u32 = 1;

/// Maximum recovery sidecar size; an initial safety limit, not a target file size.
pub const MAX_RECOVERY_FILE_BYTES: u64 = 136 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryEnvelopeV1<'a> {
    format: String,
    schema_version: u64,
    #[serde(borrow)]
    base_project: &'a RawValue,
    #[serde(borrow)]
    recovery_project: &'a RawValue,
}

#[derive(Serialize)]
struct RecoveryEnvelopeWriteV1<'a> {
    format: &'static str,
    schema_version: u32,
    base_project: &'a RawValue,
    recovery_project: &'a RawValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryMetadata {
    pub project_id: ProjectId,
    pub base_revision: ProjectRevision,
    pub recovery_revision: ProjectRevision,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryCandidate {
    metadata: RecoveryMetadata,
    recovery_project: ProjectDocument,
}

impl RecoveryCandidate {
    pub const fn metadata(&self) -> RecoveryMetadata {
        self.metadata
    }

    pub const fn recovery_project(&self) -> &ProjectDocument {
        &self.recovery_project
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryConflictReason {
    Orphaned,
    ForeignProject,
    ChangedLineage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryInspection {
    None,
    Candidate(RecoveryCandidate),
    Stale(RecoveryMetadata),
    Conflict {
        metadata: RecoveryMetadata,
        reason: RecoveryConflictReason,
    },
}

#[derive(Debug)]
pub enum RecoveryApplyOutcome {
    AppliedAndCleaned,
    AppliedCleanupPending { error: io::Error },
}

#[derive(Debug)]
pub enum ProjectRecoveryError {
    Io(io::Error),
    FileTooLarge { max_bytes: u64 },
    ProjectSnapshotTooLarge { max_bytes: u64 },
    InvalidUtf8,
    InvalidEnvelope,
    WrongFormatMarker,
    UnsupportedSchemaVersion(u64),
    InvalidBaseProject(ProjectCodecError),
    InvalidRecoveryProject(ProjectCodecError),
    ProjectIdMismatch,
    InvalidRevisionOrdering,
    DiskBaseMismatch,
    EncodingFailure,
    CheckpointWrite(ProjectStorageError),
    CanonicalProject(ProjectStorageError),
    NoCheckpoint,
    StaleCheckpoint,
    RecoveryConflict,
    ApplySave(ProjectStorageError),
}

impl fmt::Display for ProjectRecoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "recovery sidecar I/O failed: {error}"),
            Self::FileTooLarge { max_bytes } => {
                write!(
                    formatter,
                    "recovery sidecar exceeds the {max_bytes}-byte limit"
                )
            }
            Self::ProjectSnapshotTooLarge { max_bytes } => write!(
                formatter,
                "project snapshot exceeds the {max_bytes}-byte project limit"
            ),
            Self::InvalidUtf8 => formatter.write_str("recovery sidecar is not valid UTF-8"),
            Self::InvalidEnvelope => formatter.write_str("recovery sidecar envelope is invalid"),
            Self::WrongFormatMarker => {
                formatter.write_str("recovery sidecar format marker is not supported")
            }
            Self::UnsupportedSchemaVersion(version) => {
                write!(
                    formatter,
                    "recovery schema version {version} is not supported"
                )
            }
            Self::InvalidBaseProject(error) => {
                write!(formatter, "recovery base project is invalid: {error}")
            }
            Self::InvalidRecoveryProject(error) => {
                write!(formatter, "recovery project is invalid: {error}")
            }
            Self::ProjectIdMismatch => {
                formatter.write_str("recovery base and project IDs do not match")
            }
            Self::InvalidRevisionOrdering => formatter.write_str(
                "recovery project revision must be newer than the base project revision",
            ),
            Self::DiskBaseMismatch => {
                formatter.write_str("canonical project does not match the recovery checkpoint base")
            }
            Self::EncodingFailure => formatter.write_str("recovery sidecar could not be encoded"),
            Self::CheckpointWrite(error) => {
                write!(formatter, "recovery checkpoint write failed: {error}")
            }
            Self::CanonicalProject(error) => {
                write!(formatter, "canonical project could not be loaded: {error}")
            }
            Self::NoCheckpoint => formatter.write_str("no recovery checkpoint is available"),
            Self::StaleCheckpoint => formatter.write_str("recovery checkpoint is stale"),
            Self::RecoveryConflict => {
                formatter.write_str("recovery checkpoint conflicts with the canonical project")
            }
            Self::ApplySave(error) => {
                write!(formatter, "recovered project could not be saved: {error}")
            }
        }
    }
}

impl Error for ProjectRecoveryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::InvalidBaseProject(error) | Self::InvalidRecoveryProject(error) => Some(error),
            Self::CheckpointWrite(error)
            | Self::CanonicalProject(error)
            | Self::ApplySave(error) => Some(error),
            Self::FileTooLarge { .. }
            | Self::ProjectSnapshotTooLarge { .. }
            | Self::InvalidUtf8
            | Self::InvalidEnvelope
            | Self::WrongFormatMarker
            | Self::UnsupportedSchemaVersion(_)
            | Self::ProjectIdMismatch
            | Self::InvalidRevisionOrdering
            | Self::DiskBaseMismatch
            | Self::EncodingFailure
            | Self::NoCheckpoint
            | Self::StaleCheckpoint
            | Self::RecoveryConflict => None,
        }
    }
}

struct RecoveryCheckpoint {
    base_project: ProjectDocument,
    recovery_project: ProjectDocument,
    metadata: RecoveryMetadata,
}

/// Writes a recovery snapshot only when the canonical file still equals its saved base.
pub fn write_recovery_checkpoint(
    project_path: &Path,
    base_project: &ProjectDocument,
    recovery_project: &ProjectDocument,
) -> Result<(), ProjectRecoveryError> {
    validate_pair(base_project, recovery_project)?;

    let disk_project =
        load_project_file(project_path).map_err(ProjectRecoveryError::CanonicalProject)?;
    if disk_project != *base_project {
        return Err(ProjectRecoveryError::DiskBaseMismatch);
    }

    let bytes = encode_checkpoint(base_project, recovery_project)?;
    let checkpoint_path = recovery_sidecar_path(project_path)?;
    atomic_replace_bytes(&checkpoint_path, &bytes).map_err(ProjectRecoveryError::CheckpointWrite)
}

/// Inspects a recovery sidecar without changing the canonical project or sidecar.
pub fn inspect_project_recovery(
    project_path: &Path,
) -> Result<RecoveryInspection, ProjectRecoveryError> {
    let checkpoint_path = recovery_sidecar_path(project_path)?;
    let Some(checkpoint) = read_checkpoint(&checkpoint_path)? else {
        return Ok(RecoveryInspection::None);
    };

    let canonical = match load_project_file(project_path) {
        Ok(project) => project,
        Err(ProjectStorageError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(RecoveryInspection::Conflict {
                metadata: checkpoint.metadata,
                reason: RecoveryConflictReason::Orphaned,
            });
        }
        Err(error) => return Err(ProjectRecoveryError::CanonicalProject(error)),
    };

    if canonical.id() != checkpoint.metadata.project_id {
        return Ok(RecoveryInspection::Conflict {
            metadata: checkpoint.metadata,
            reason: RecoveryConflictReason::ForeignProject,
        });
    }
    if canonical == checkpoint.recovery_project
        || canonical.revision() > checkpoint.metadata.recovery_revision
    {
        return Ok(RecoveryInspection::Stale(checkpoint.metadata));
    }
    if canonical == checkpoint.base_project {
        return Ok(RecoveryInspection::Candidate(RecoveryCandidate {
            metadata: checkpoint.metadata,
            recovery_project: checkpoint.recovery_project,
        }));
    }

    Ok(RecoveryInspection::Conflict {
        metadata: checkpoint.metadata,
        reason: RecoveryConflictReason::ChangedLineage,
    })
}

/// Explicitly applies a freshly revalidated recovery candidate to the canonical file.
pub fn apply_project_recovery(
    project_path: &Path,
) -> Result<RecoveryApplyOutcome, ProjectRecoveryError> {
    apply_project_recovery_with(
        project_path,
        |path, project| save_project_file_atomic(path, project),
        |path| fs::remove_file(path),
    )
}

fn apply_project_recovery_with(
    project_path: &Path,
    save: impl FnOnce(&Path, &ProjectDocument) -> Result<(), ProjectStorageError>,
    remove: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<RecoveryApplyOutcome, ProjectRecoveryError> {
    let recovery_project = match inspect_project_recovery(project_path)? {
        RecoveryInspection::None => return Err(ProjectRecoveryError::NoCheckpoint),
        RecoveryInspection::Candidate(candidate) => candidate.recovery_project,
        RecoveryInspection::Stale(_) => return Err(ProjectRecoveryError::StaleCheckpoint),
        RecoveryInspection::Conflict { .. } => return Err(ProjectRecoveryError::RecoveryConflict),
    };

    save(project_path, &recovery_project).map_err(ProjectRecoveryError::ApplySave)?;
    let checkpoint_path = recovery_sidecar_path(project_path)?;
    match remove(&checkpoint_path) {
        Ok(()) => Ok(RecoveryApplyOutcome::AppliedAndCleaned),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(RecoveryApplyOutcome::AppliedAndCleaned)
        }
        Err(error) => Ok(RecoveryApplyOutcome::AppliedCleanupPending { error }),
    }
}

/// Explicitly removes a recovery sidecar without decoding it or changing the project.
pub fn discard_project_recovery(project_path: &Path) -> Result<bool, ProjectRecoveryError> {
    let checkpoint_path = recovery_sidecar_path(project_path)?;
    match fs::remove_file(checkpoint_path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(ProjectRecoveryError::Io(error)),
    }
}

fn validate_pair(
    base_project: &ProjectDocument,
    recovery_project: &ProjectDocument,
) -> Result<(), ProjectRecoveryError> {
    if base_project.id() != recovery_project.id() {
        return Err(ProjectRecoveryError::ProjectIdMismatch);
    }
    if recovery_project.revision() <= base_project.revision() {
        return Err(ProjectRecoveryError::InvalidRevisionOrdering);
    }
    Ok(())
}

fn encode_checkpoint(
    base_project: &ProjectDocument,
    recovery_project: &ProjectDocument,
) -> Result<Vec<u8>, ProjectRecoveryError> {
    let base_project = encode_project_value(base_project, true)?;
    let recovery_project = encode_project_value(recovery_project, false)?;
    let envelope = RecoveryEnvelopeWriteV1 {
        format: RECOVERY_FORMAT_MARKER,
        schema_version: CURRENT_RECOVERY_SCHEMA_VERSION,
        base_project: &base_project,
        recovery_project: &recovery_project,
    };
    let mut bytes =
        serde_json::to_vec_pretty(&envelope).map_err(|_| ProjectRecoveryError::EncodingFailure)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > MAX_RECOVERY_FILE_BYTES {
        return Err(ProjectRecoveryError::FileTooLarge {
            max_bytes: MAX_RECOVERY_FILE_BYTES,
        });
    }
    Ok(bytes)
}

fn encode_project_value(
    project: &ProjectDocument,
    is_base: bool,
) -> Result<Box<RawValue>, ProjectRecoveryError> {
    let encoded = encode_project(project).map_err(|error| {
        if is_base {
            ProjectRecoveryError::InvalidBaseProject(error)
        } else {
            ProjectRecoveryError::InvalidRecoveryProject(error)
        }
    })?;
    if encoded.len() as u64 > MAX_PROJECT_FILE_BYTES {
        return Err(ProjectRecoveryError::ProjectSnapshotTooLarge {
            max_bytes: MAX_PROJECT_FILE_BYTES,
        });
    }
    RawValue::from_string(encoded).map_err(|_| ProjectRecoveryError::EncodingFailure)
}

fn read_checkpoint(path: &Path) -> Result<Option<RecoveryCheckpoint>, ProjectRecoveryError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(ProjectRecoveryError::Io(error)),
    };
    if file.metadata().map_err(ProjectRecoveryError::Io)?.len() > MAX_RECOVERY_FILE_BYTES {
        return Err(ProjectRecoveryError::FileTooLarge {
            max_bytes: MAX_RECOVERY_FILE_BYTES,
        });
    }

    let mut bytes = Vec::new();
    file.take(MAX_RECOVERY_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(ProjectRecoveryError::Io)?;
    if bytes.len() as u64 > MAX_RECOVERY_FILE_BYTES {
        return Err(ProjectRecoveryError::FileTooLarge {
            max_bytes: MAX_RECOVERY_FILE_BYTES,
        });
    }
    let encoded = std::str::from_utf8(&bytes).map_err(|_| ProjectRecoveryError::InvalidUtf8)?;
    let envelope: RecoveryEnvelopeV1<'_> =
        serde_json::from_str(encoded).map_err(|_| ProjectRecoveryError::InvalidEnvelope)?;
    if envelope.format != RECOVERY_FORMAT_MARKER {
        return Err(ProjectRecoveryError::WrongFormatMarker);
    }
    if envelope.schema_version != u64::from(CURRENT_RECOVERY_SCHEMA_VERSION) {
        return Err(ProjectRecoveryError::UnsupportedSchemaVersion(
            envelope.schema_version,
        ));
    }

    let base_project = decode_project_value(envelope.base_project, true)?;
    let recovery_project = decode_project_value(envelope.recovery_project, false)?;
    validate_pair(&base_project, &recovery_project)?;
    let metadata = RecoveryMetadata {
        project_id: base_project.id(),
        base_revision: base_project.revision(),
        recovery_revision: recovery_project.revision(),
    };
    Ok(Some(RecoveryCheckpoint {
        base_project,
        recovery_project,
        metadata,
    }))
}

fn decode_project_value(
    value: &RawValue,
    is_base: bool,
) -> Result<ProjectDocument, ProjectRecoveryError> {
    let project = decode_project(value.get()).map_err(|error| {
        if is_base {
            ProjectRecoveryError::InvalidBaseProject(error)
        } else {
            ProjectRecoveryError::InvalidRecoveryProject(error)
        }
    })?;
    let canonical = encode_project(&project).map_err(|error| {
        if is_base {
            ProjectRecoveryError::InvalidBaseProject(error)
        } else {
            ProjectRecoveryError::InvalidRecoveryProject(error)
        }
    })?;
    if canonical.len() as u64 > MAX_PROJECT_FILE_BYTES {
        return Err(ProjectRecoveryError::ProjectSnapshotTooLarge {
            max_bytes: MAX_PROJECT_FILE_BYTES,
        });
    }
    Ok(project)
}

fn recovery_sidecar_path(project_path: &Path) -> Result<PathBuf, ProjectRecoveryError> {
    let target_name = project_path.file_name().ok_or_else(|| {
        ProjectRecoveryError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "project path must name a file",
        ))
    })?;
    let mut sidecar_name = OsString::from(".");
    sidecar_name.push(target_name);
    sidecar_name.push(".or-recovery");
    Ok(parent_directory(project_path).join(sidecar_name))
}

#[cfg(test)]
mod tests {
    use super::{
        RecoveryApplyOutcome, apply_project_recovery_with, discard_project_recovery,
        inspect_project_recovery, write_recovery_checkpoint,
    };
    use crate::{
        ProjectDocument, ProjectStorageError, load_project_file, save_project_file_atomic,
    };
    use serde_json::json;
    use std::{fs, io, path::PathBuf};
    use uuid::Uuid;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("or-recovery-unit-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn project_path(&self) -> PathBuf {
            self.0.join("example.orproj")
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn next_project(base: &ProjectDocument) -> ProjectDocument {
        let mut wire: serde_json::Value =
            serde_json::from_str(&crate::encode_project(base).unwrap()).unwrap();
        wire["project"]["revision"] = json!(base.revision().value() + 1);
        wire["project"]["name"] = json!("Recovered");
        crate::decode_project(&serde_json::to_string(&wire).unwrap()).unwrap()
    }

    #[test]
    fn apply_save_failure_preserves_the_canonical_file_and_checkpoint() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let base = ProjectDocument::new("Base");
        let recovery = next_project(&base);
        save_project_file_atomic(&path, &base).unwrap();
        write_recovery_checkpoint(&path, &base, &recovery).unwrap();

        let result = apply_project_recovery_with(
            &path,
            |_, _| {
                Err(ProjectStorageError::Io(io::Error::other(
                    "injected save failure",
                )))
            },
            |path| fs::remove_file(path),
        );

        assert!(result.is_err());
        assert_eq!(load_project_file(&path).unwrap(), base);
        assert!(matches!(
            inspect_project_recovery(&path).unwrap(),
            crate::RecoveryInspection::Candidate(_)
        ));
    }

    #[test]
    fn cleanup_failure_reports_applied_state_and_leaves_a_stale_checkpoint() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let base = ProjectDocument::new("Base");
        let recovery = next_project(&base);
        save_project_file_atomic(&path, &base).unwrap();
        write_recovery_checkpoint(&path, &base, &recovery).unwrap();

        let outcome = apply_project_recovery_with(
            &path,
            |project_path, project| save_project_file_atomic(project_path, project),
            |_| Err(io::Error::other("injected cleanup failure")),
        )
        .unwrap();

        assert!(matches!(
            outcome,
            RecoveryApplyOutcome::AppliedCleanupPending { .. }
        ));
        assert_eq!(load_project_file(&path).unwrap(), recovery);
        assert!(matches!(
            inspect_project_recovery(&path).unwrap(),
            crate::RecoveryInspection::Stale(_)
        ));
        assert!(discard_project_recovery(&path).unwrap());
    }
}
