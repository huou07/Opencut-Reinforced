use crate::{
    ApplicationRequest, ApplicationResponse, ProjectDocument, ProjectSession, ProjectStorageError,
    RecoveryInspection, inspect_project_recovery, load_project_file, save_project_file_atomic,
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

impl ProjectFileSessionErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RecoveryRequired => "RECOVERY_REQUIRED",
            Self::ProjectFileChanged => "PROJECT_FILE_CHANGED",
            Self::StorageFailure => "STORAGE_FAILURE",
        }
    }
}

/// One loaded project file, its live runtime session, and the exact saved base.
#[derive(Debug, Eq, PartialEq)]
pub struct ProjectFileSession {
    project_path: PathBuf,
    last_saved_project: ProjectDocument,
    session: ProjectSession,
}

impl ProjectFileSession {
    /// Loads a project without applying or discarding any recovery checkpoint.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectFileSessionError> {
        let project_path = path.as_ref().to_path_buf();
        let project = load_project_file(&project_path).map_err(storage_error)?;
        check_recovery(&project_path)?;

        Ok(Self {
            project_path,
            last_saved_project: project.clone(),
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

    /// Dispatches through the same command, query, and transaction paths as ProjectSession.
    pub fn handle_application_request(
        &mut self,
        request: ApplicationRequest,
    ) -> ApplicationResponse {
        self.session.handle_application_request(request)
    }

    /// Saves only if recovery state is safe and the canonical file still equals the saved base.
    pub fn save(&mut self) -> Result<(), ProjectFileSessionError> {
        check_recovery(&self.project_path)?;
        let current_disk = load_project_file(&self.project_path).map_err(storage_error)?;
        if current_disk != self.last_saved_project {
            return Err(ProjectFileSessionError {
                code: ProjectFileSessionErrorCode::ProjectFileChanged,
                detail: "the project file no longer matches this session's saved base".to_owned(),
            });
        }

        let current_project = self.session.project();
        save_project_file_atomic(&self.project_path, current_project).map_err(storage_error)?;
        self.last_saved_project = current_project.clone();
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
    ProjectFileSessionError {
        code: ProjectFileSessionErrorCode::StorageFailure,
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
        CommandEnvelope, ProjectRevision, RecoveryInspection, decode_project,
        inspect_project_recovery, save_project_file_atomic, write_recovery_checkpoint,
    };
    use serde_json::json;
    use std::fs;
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
