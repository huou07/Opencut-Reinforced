use crate::project_storage::create_project_file_new;
use crate::{
    ApplicationRequest, ApplicationResponse, CommandEnvelope, CommandResult, MediaImportError,
    MediaItem, OperationError, ProjectDocument, ProjectSession, ProjectStorageError,
    RecoveryInspection, discard_project_recovery, inspect_project_recovery, load_project_file,
    prepare_media_import, save_project_file_atomic, write_recovery_checkpoint,
};
use serde::Serialize;
use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

/// Stable failure categories for file-backed project session operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectFileSessionErrorCode {
    RecoveryRequired,
    ProjectFileChanged,
    DestinationExists,
    StorageFailure,
}

/// A safe error from opening or saving a file-backed project session.
#[derive(Debug)]
pub struct ProjectFileSessionError {
    code: ProjectFileSessionErrorCode,
    detail: String,
}

impl ProjectFileSessionError {
    pub const fn code(&self) -> ProjectFileSessionErrorCode {
        self.code
    }
}

impl fmt::Display for ProjectFileSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.detail)
    }
}

impl Error for ProjectFileSessionError {}

/// Failure while preparing or dispatching a file-backed media import.
#[derive(Debug)]
pub enum ProjectFileMediaImportError {
    Preparation(MediaImportError),
    Operation(OperationError),
}

impl fmt::Display for ProjectFileMediaImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preparation(error) => write!(formatter, "{error}"),
            Self::Operation(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for ProjectFileMediaImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Preparation(error) => Some(error),
            Self::Operation(error) => Some(error),
        }
    }
}

impl ProjectFileSessionErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RecoveryRequired => "RECOVERY_REQUIRED",
            Self::ProjectFileChanged => "PROJECT_FILE_CHANGED",
            Self::DestinationExists => "DESTINATION_EXISTS",
            Self::StorageFailure => "STORAGE_FAILURE",
        }
    }
}

/// One loaded project file, its live runtime session, and the exact saved base.
#[derive(Debug, Eq, PartialEq)]
pub struct ProjectFileSession {
    project_path: PathBuf,
    last_saved_project: ProjectDocument,
    last_autosaved_project: Option<ProjectDocument>,
    session: ProjectSession,
}

impl ProjectFileSession {
    /// Creates a new file-backed project without replacing an existing destination.
    pub fn create_new(
        path: impl AsRef<Path>,
        name: impl Into<String>,
    ) -> Result<Self, ProjectFileSessionError> {
        let project_path = path.as_ref().to_path_buf();
        check_recovery(&project_path)?;
        let project = ProjectDocument::new(name);
        create_project_file_new(&project_path, &project).map_err(storage_error)?;

        Ok(Self {
            project_path,
            last_saved_project: project.clone(),
            last_autosaved_project: None,
            session: ProjectSession::open(project),
        })
    }

    /// Loads a project without applying or discarding any recovery checkpoint.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectFileSessionError> {
        let project_path = path.as_ref().to_path_buf();
        let project = load_project_file(&project_path).map_err(storage_error)?;
        check_recovery(&project_path)?;

        Ok(Self {
            project_path,
            last_saved_project: project.clone(),
            last_autosaved_project: None,
            session: ProjectSession::open(project),
        })
    }

    pub fn project_path(&self) -> &Path {
        &self.project_path
    }

    pub const fn session(&self) -> &ProjectSession {
        &self.session
    }

    pub fn is_dirty(&self) -> bool {
        self.session.project() != &self.last_saved_project
    }

    /// Writes a recovery sidecar for dirty in-memory state without touching the canonical file.
    /// Returns whether a checkpoint was written.
    pub fn autosave_checkpoint(&mut self) -> Result<bool, ProjectFileSessionError> {
        let current_project = self.session.project().clone();
        let inspection = inspect_project_recovery(&self.project_path)
            .map_err(|error| recovery_error(&error.to_string()))?;

        if current_project == self.last_saved_project {
            if let (Some(autosaved), RecoveryInspection::Candidate(candidate)) =
                (&self.last_autosaved_project, &inspection)
                && candidate.recovery_project() == autosaved
            {
                discard_project_recovery(&self.project_path)
                    .map_err(|error| recovery_error(&error.to_string()))?;
            }
            self.last_autosaved_project = None;
            return Ok(false);
        }

        match inspection {
            RecoveryInspection::None | RecoveryInspection::Stale(_) => {}
            RecoveryInspection::Candidate(candidate)
                if candidate.recovery_project() == &current_project
                    || self.last_autosaved_project.as_ref()
                        == Some(candidate.recovery_project()) => {}
            RecoveryInspection::Candidate(_) | RecoveryInspection::Conflict { .. } => {
                return Err(recovery_error(
                    "an unrelated recovery checkpoint needs explicit attention",
                ));
            }
        }

        write_recovery_checkpoint(
            &self.project_path,
            &self.last_saved_project,
            &current_project,
        )
        .map_err(|error| match error {
            crate::ProjectRecoveryError::DiskBaseMismatch => ProjectFileSessionError {
                code: ProjectFileSessionErrorCode::ProjectFileChanged,
                detail: "the project file no longer matches this session's saved base".to_owned(),
            },
            error => recovery_error(&error.to_string()),
        })?;
        self.last_autosaved_project = Some(current_project);
        Ok(true)
    }

    /// Dispatches through the same command, query, and transaction paths as ProjectSession.
    pub fn handle_application_request(
        &mut self,
        request: ApplicationRequest,
    ) -> ApplicationResponse {
        self.session.handle_application_request(request)
    }

    /// Prepares an external source, then adds it through the normal command path.
    pub fn import_media(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<(MediaItem, CommandResult), ProjectFileMediaImportError> {
        let project_id = self.session.project_id();
        let project_instance_id = self.session.project_instance_id();
        let expected_revision = self.session.project_revision();
        let item = prepare_media_import(path.as_ref())
            .map_err(ProjectFileMediaImportError::Preparation)?;
        let command = CommandEnvelope::add_media(
            project_id,
            project_instance_id,
            expected_revision,
            item.clone(),
        );
        let result = self
            .session
            .execute_command(command)
            .map_err(ProjectFileMediaImportError::Operation)?;
        Ok((item, result))
    }

    /// Saves only if recovery state is safe and the canonical file still equals the saved base.
    pub fn save(&mut self) -> Result<(), ProjectFileSessionError> {
        let current_disk = load_project_file(&self.project_path).map_err(storage_error)?;
        if current_disk != self.last_saved_project {
            return Err(ProjectFileSessionError {
                code: ProjectFileSessionErrorCode::ProjectFileChanged,
                detail: "the project file no longer matches this session's saved base".to_owned(),
            });
        }

        let current_project = self.session.project();
        match inspect_project_recovery(&self.project_path)
            .map_err(|error| recovery_error(&error.to_string()))?
        {
            RecoveryInspection::None | RecoveryInspection::Stale(_) => {}
            RecoveryInspection::Candidate(candidate)
                if candidate.recovery_project() == current_project => {}
            RecoveryInspection::Candidate(_) | RecoveryInspection::Conflict { .. } => {
                return Err(recovery_error(
                    "a recovery checkpoint needs explicit attention",
                ));
            }
        }
        save_project_file_atomic(&self.project_path, current_project).map_err(storage_error)?;
        self.last_saved_project = current_project.clone();
        self.last_autosaved_project = None;
        Ok(())
    }
}

fn check_recovery(path: &Path) -> Result<(), ProjectFileSessionError> {
    match inspect_project_recovery(path) {
        Ok(RecoveryInspection::None | RecoveryInspection::Stale(_)) => Ok(()),
        Ok(RecoveryInspection::Candidate(_) | RecoveryInspection::Conflict { .. }) => Err(
            recovery_error("a recovery checkpoint needs explicit attention"),
        ),
        Err(error) => Err(recovery_error(&error.to_string())),
    }
}

fn storage_error(error: ProjectStorageError) -> ProjectFileSessionError {
    let code = match &error {
        ProjectStorageError::DestinationExists => ProjectFileSessionErrorCode::DestinationExists,
        _ => ProjectFileSessionErrorCode::StorageFailure,
    };
    ProjectFileSessionError {
        code,
        detail: error.to_string(),
    }
}

fn recovery_error(detail: &str) -> ProjectFileSessionError {
    ProjectFileSessionError {
        code: ProjectFileSessionErrorCode::RecoveryRequired,
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CommandEnvelope, MediaId, MediaItem, MediaMetadata, MediaSourceRef, ProjectRevision,
        RecoveryInspection, decode_project, inspect_project_recovery, save_project_file_atomic,
        write_recovery_checkpoint,
    };
    use serde_json::json;
    use std::{fs, str::FromStr};
    use uuid::Uuid;

    const PROJECT_ID: &str = "01234567-89ab-4def-8123-456789abcdef";

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("or-core-session-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn project_path(&self) -> PathBuf {
            self.0.join("sample.orproj")
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn document(name: &str, revision: u64) -> ProjectDocument {
        decode_project(&format!(
            r#"{{"format":"opencut-reinforced-project","schema_version":1,"project":{{"id":"{PROJECT_ID}","revision":{revision},"name":{}}}}}"#,
            serde_json::to_string(name).unwrap()
        ))
        .unwrap()
    }

    fn rename(session: &ProjectSession, name: &str) -> ApplicationRequest {
        ApplicationRequest::Command(CommandEnvelope {
            command_id: "project.rename".to_owned(),
            schema_version: 1,
            project_id: session.project_id(),
            project_instance_id: session.project_instance_id(),
            expected_project_revision: session.project_revision(),
            arguments: json!({ "name": name }),
        })
    }

    #[test]
    fn open_preserves_revision_and_dispatch_tracks_runtime_dirty_state() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let original = document("A", 8);
        save_project_file_atomic(&path, &original).unwrap();

        let mut session = ProjectFileSession::open(&path).unwrap();
        assert_eq!(
            session.session().project_revision(),
            ProjectRevision::new(8)
        );
        assert!(!session.is_dirty());
        assert_eq!(session.session().project_id(), original.id());

        assert!(matches!(
            session.handle_application_request(rename(session.session(), "B")),
            ApplicationResponse::Command(_)
        ));
        assert!(session.is_dirty());
        assert_eq!(load_project_file(&path).unwrap(), original);

        session.save().unwrap();
        assert!(!session.is_dirty());
        assert_eq!(load_project_file(path).unwrap().name(), "B");
        assert_eq!(
            session.session().project_revision(),
            ProjectRevision::new(9)
        );
    }

    #[test]
    fn create_new_writes_a_v6_project_with_fresh_identity_and_empty_history() {
        let directory = TestDirectory::new();
        let path = directory.project_path();

        let mut session = ProjectFileSession::create_new(&path, "Exact Name ").unwrap();
        let summary = session.session().project();
        let instance_id = session.session().project_instance_id();

        assert_eq!(summary.name(), "Exact Name ");
        assert_eq!(summary.revision(), ProjectRevision::INITIAL);
        assert!(!session.is_dirty());
        assert_eq!(load_project_file(&path).unwrap(), *summary);
        let reopened = ProjectFileSession::open(&path).unwrap();
        assert_eq!(reopened.session().project_id(), summary.id());
        assert_ne!(reopened.session().project_instance_id(), instance_id);
        let undo =
            session.handle_application_request(ApplicationRequest::Command(CommandEnvelope {
                command_id: "history.undo".to_owned(),
                schema_version: 1,
                project_id: session.session().project_id(),
                project_instance_id: session.session().project_instance_id(),
                expected_project_revision: ProjectRevision::INITIAL,
                arguments: serde_json::json!({}),
            }));
        assert!(matches!(undo, ApplicationResponse::Error(_)));
    }

    #[test]
    fn opening_v1_migrates_in_memory_and_explicit_save_writes_v6_without_revision_change() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let legacy = format!(
            r#"{{"format":"opencut-reinforced-project","schema_version":1,"project":{{"id":"{PROJECT_ID}","revision":7,"name":"Legacy"}}}}"#
        );
        std::fs::write(&path, &legacy).unwrap();

        let mut session = ProjectFileSession::open(&path).unwrap();
        assert!(!session.is_dirty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), legacy);
        assert_eq!(
            session.session().project_revision(),
            ProjectRevision::new(7)
        );
        assert!(session.session().project().media_items().is_empty());

        session.save().unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        let encoded: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(encoded["schema_version"], 6);
        assert_eq!(
            encoded["project"]["timeline"]["sequence_frame_rate"],
            serde_json::Value::Null
        );
        assert_eq!(encoded["project"]["revision"], 7);
        assert!(!session.is_dirty());
    }

    #[test]
    fn opening_v2_with_media_stays_clean_until_explicit_v6_save() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let item = MediaItem::new(
            MediaId::from_str("22222222-2222-4222-8222-222222222222").unwrap(),
            MediaSourceRef::local_file("file:///offline/clip.mov").unwrap(),
            MediaMetadata::from_probe(vec!["mov".to_owned()], None, 123, Vec::new()),
        )
        .unwrap();
        let legacy = serde_json::to_string(&json!({
            "format": "opencut-reinforced-project",
            "schema_version": 2,
            "project": {
                "id": PROJECT_ID,
                "revision": 11,
                "name": "Legacy with media",
                "media": [item]
            }
        }))
        .unwrap();
        fs::write(&path, &legacy).unwrap();

        let mut session = ProjectFileSession::open(&path).unwrap();
        assert!(!session.is_dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), legacy);
        assert_eq!(
            session.session().project_revision(),
            ProjectRevision::new(11)
        );
        assert_eq!(session.session().project().media_items(), &[item]);
        assert!(session.session().project().timeline().tracks().is_empty());

        session.save().unwrap();

        let saved = fs::read_to_string(&path).unwrap();
        let encoded: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(encoded["schema_version"], 6);
        assert_eq!(encoded["project"]["revision"], 11);
        assert!(!session.is_dirty());
    }

    #[test]
    fn opening_v3_with_timeline_stays_clean_until_explicit_v6_save() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let legacy = serde_json::to_string(&json!({
            "format": "opencut-reinforced-project",
            "schema_version": 3,
            "project": {
                "id": PROJECT_ID,
                "revision": 17,
                "name": "Legacy timeline",
                "media": [],
                "timeline": {"tracks": []}
            }
        }))
        .unwrap();
        fs::write(&path, &legacy).unwrap();

        let mut session = ProjectFileSession::open(&path).unwrap();
        assert!(!session.is_dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), legacy);
        assert_eq!(
            session.session().project_revision(),
            ProjectRevision::new(17)
        );
        assert!(session.session().project().timeline().tracks().is_empty());
        assert!(session.session().project().timeline().markers().is_empty());

        session.save().unwrap();

        let saved = fs::read_to_string(&path).unwrap();
        let encoded: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(encoded["schema_version"], 6);
        assert_eq!(encoded["project"]["revision"], 17);
        assert!(!session.is_dirty());
    }

    #[test]
    fn opening_v4_stays_clean_until_explicit_v6_save_with_unset_rate() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let legacy = serde_json::to_string(&json!({
            "format": "opencut-reinforced-project",
            "schema_version": 4,
            "project": {
                "id": PROJECT_ID,
                "revision": 23,
                "name": "Legacy v4",
                "media": [],
                "timeline": {"tracks": [], "markers": []}
            }
        }))
        .unwrap();
        fs::write(&path, &legacy).unwrap();

        let mut session = ProjectFileSession::open(&path).unwrap();
        assert!(!session.is_dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), legacy);
        assert_eq!(
            session.session().project_revision(),
            ProjectRevision::new(23)
        );
        assert_eq!(
            session.session().project().timeline().sequence_frame_rate(),
            None
        );

        session.save().unwrap();
        let encoded: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(encoded["schema_version"], 6);
        assert_eq!(encoded["project"]["revision"], 23);
        assert_eq!(
            encoded["project"]["timeline"]["sequence_frame_rate"],
            serde_json::Value::Null
        );
        assert!(!session.is_dirty());
    }

    #[test]
    fn create_new_never_clobbers_an_existing_destination() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let existing = document("Keep me", 17);
        save_project_file_atomic(&path, &existing).unwrap();
        let original_bytes = std::fs::read(&path).unwrap();

        let error = ProjectFileSession::create_new(&path, "Replacement").unwrap_err();

        assert_eq!(error.code(), ProjectFileSessionErrorCode::DestinationExists);
        assert_eq!(std::fs::read(&path).unwrap(), original_bytes);
        assert_eq!(load_project_file(&path).unwrap(), existing);
    }

    #[test]
    fn save_rejects_same_revision_external_replacement_by_exact_document() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let original = document("A", 4);
        save_project_file_atomic(&path, &original).unwrap();
        let mut session = ProjectFileSession::open(&path).unwrap();
        session.handle_application_request(rename(session.session(), "B"));

        let external = document("X", 4);
        save_project_file_atomic(&path, &external).unwrap();
        let error = session.save().unwrap_err();

        assert_eq!(
            error.code(),
            ProjectFileSessionErrorCode::ProjectFileChanged
        );
        assert!(session.is_dirty());
        assert_eq!(load_project_file(path).unwrap(), external);
    }

    #[test]
    fn autosave_updates_only_a_recovery_checkpoint_and_preserves_the_canonical_file() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let base = document("A", 4);
        save_project_file_atomic(&path, &base).unwrap();
        let canonical_bytes = fs::read(&path).unwrap();
        let mut session = ProjectFileSession::open(&path).unwrap();

        session.handle_application_request(rename(session.session(), "B"));
        assert!(session.autosave_checkpoint().unwrap());
        assert_eq!(fs::read(&path).unwrap(), canonical_bytes);
        let first = match inspect_project_recovery(&path).unwrap() {
            RecoveryInspection::Candidate(candidate) => candidate.recovery_project().clone(),
            inspection => panic!("expected candidate, got {inspection:?}"),
        };
        assert_eq!(first, *session.session().project());

        session.handle_application_request(rename(session.session(), "C"));
        assert!(session.autosave_checkpoint().unwrap());
        assert_eq!(fs::read(&path).unwrap(), canonical_bytes);
        let latest = match inspect_project_recovery(&path).unwrap() {
            RecoveryInspection::Candidate(candidate) => candidate.recovery_project().clone(),
            inspection => panic!("expected updated candidate, got {inspection:?}"),
        };
        assert_eq!(latest, *session.session().project());
    }

    #[test]
    fn explicit_save_promotes_its_autosaved_snapshot_to_the_canonical_project() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let base = document("A", 4);
        save_project_file_atomic(&path, &base).unwrap();
        let mut session = ProjectFileSession::open(&path).unwrap();
        session.handle_application_request(rename(session.session(), "B"));
        assert!(session.autosave_checkpoint().unwrap());

        session.save().unwrap();

        assert!(!session.is_dirty());
        assert_eq!(
            load_project_file(&path).unwrap(),
            *session.session().project()
        );
        assert!(matches!(
            inspect_project_recovery(&path).unwrap(),
            RecoveryInspection::Stale(_)
        ));
    }

    #[test]
    fn autosave_preserves_an_unrelated_recovery_candidate() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let base = document("A", 4);
        let unrelated = document("Recovery from another session", 5);
        save_project_file_atomic(&path, &base).unwrap();
        write_recovery_checkpoint(&path, &base, &unrelated).unwrap();
        let checkpoint_bytes = fs::read(directory.0.join(".sample.orproj.or-recovery")).unwrap();
        let session = ProjectFileSession::open(&path).unwrap_err();
        assert_eq!(
            session.code(),
            ProjectFileSessionErrorCode::RecoveryRequired
        );
        discard_checkpoint_for_test(&path);
        let mut session = ProjectFileSession::open(&path).unwrap();
        session.handle_application_request(rename(session.session(), "Local edit"));
        write_recovery_checkpoint(&path, &base, &unrelated).unwrap();

        assert_eq!(
            session.autosave_checkpoint().unwrap_err().code(),
            ProjectFileSessionErrorCode::RecoveryRequired
        );
        assert_eq!(
            fs::read(directory.0.join(".sample.orproj.or-recovery")).unwrap(),
            checkpoint_bytes
        );
    }

    #[test]
    fn autosave_refuses_to_checkpoint_after_an_exact_disk_base_conflict() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let base = document("A", 4);
        save_project_file_atomic(&path, &base).unwrap();
        let mut session = ProjectFileSession::open(&path).unwrap();
        session.handle_application_request(rename(session.session(), "Local edit"));
        let external = document("External edit", 4);
        save_project_file_atomic(&path, &external).unwrap();

        assert_eq!(
            session.autosave_checkpoint().unwrap_err().code(),
            ProjectFileSessionErrorCode::ProjectFileChanged
        );
        assert_eq!(load_project_file(path).unwrap(), external);
    }

    #[test]
    fn candidate_recovery_blocks_open_and_save_without_removing_checkpoint() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let base = document("A", 4);
        let recovery = document("B", 5);
        save_project_file_atomic(&path, &base).unwrap();
        write_recovery_checkpoint(&path, &base, &recovery).unwrap();

        assert_eq!(
            ProjectFileSession::open(&path).unwrap_err().code(),
            ProjectFileSessionErrorCode::RecoveryRequired
        );
        assert!(matches!(
            inspect_project_recovery(&path).unwrap(),
            RecoveryInspection::Candidate(_)
        ));

        discard_checkpoint_for_test(&path);
        let mut session = ProjectFileSession::open(&path).unwrap();
        session.handle_application_request(rename(session.session(), "C"));
        write_recovery_checkpoint(&path, &base, &recovery).unwrap();
        assert_eq!(
            session.save().unwrap_err().code(),
            ProjectFileSessionErrorCode::RecoveryRequired
        );
        assert!(matches!(
            inspect_project_recovery(&path).unwrap(),
            RecoveryInspection::Candidate(_)
        ));
    }

    #[test]
    fn stale_recovery_allows_open_and_save_without_sidecar_cleanup() {
        let directory = TestDirectory::new();
        let path = directory.project_path();
        let base = document("A", 4);
        let recovery = document("B", 5);
        save_project_file_atomic(&path, &base).unwrap();
        write_recovery_checkpoint(&path, &base, &recovery).unwrap();
        save_project_file_atomic(&path, &recovery).unwrap();

        assert!(matches!(
            inspect_project_recovery(&path).unwrap(),
            RecoveryInspection::Stale(_)
        ));
        let mut session = ProjectFileSession::open(&path).unwrap();
        session.save().unwrap();
        assert!(matches!(
            inspect_project_recovery(&path).unwrap(),
            RecoveryInspection::Stale(_)
        ));
    }

    fn discard_checkpoint_for_test(path: &Path) {
        crate::discard_project_recovery(path).unwrap();
    }
}
