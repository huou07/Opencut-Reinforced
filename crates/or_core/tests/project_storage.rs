use or_core::{
    ApplicationRequest, ApplicationResponse, CommandEnvelope, MAX_PROJECT_FILE_BYTES,
    OperationErrorCode, ProjectDocument, ProjectFileSession, ProjectFileSessionErrorCode,
    ProjectRevision, ProjectSession, ProjectStorageError, load_project_file,
    save_project_file_atomic,
};
use serde_json::json;
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};
use uuid::Uuid;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("or-project-storage-{}", Uuid::new_v4()));
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

fn rename(session: &mut ProjectSession, name: &str) {
    session
        .execute_command(CommandEnvelope {
            command_id: "project.rename".to_owned(),
            schema_version: 1,
            project_id: session.project_id(),
            project_instance_id: session.project_instance_id(),
            expected_project_revision: session.project_revision(),
            arguments: json!({ "name": name }),
        })
        .unwrap();
}

fn names(directory: &Path) -> Vec<String> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect()
}

#[test]
fn saves_and_loads_canonical_state_without_runtime_session_history() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let mut session = ProjectSession::open(ProjectDocument::new("Initial"));
    rename(&mut session, "Renamed");
    let original = session.project().clone();
    let instance_id = session.project_instance_id().to_string();

    save_project_file_atomic(&path, session.project()).unwrap();

    assert_eq!(session.project(), &original);
    let bytes = fs::read_to_string(&path).unwrap();
    assert!(!bytes.contains(&instance_id));
    for runtime_state in ["instance_id", "history", "undo", "redo", "change_set"] {
        assert!(!bytes.contains(runtime_state));
    }

    let loaded = load_project_file(&path).unwrap();
    assert_eq!(loaded.id(), original.id());
    assert_eq!(loaded.revision(), ProjectRevision::new(1));
    assert_eq!(loaded.name(), "Renamed");

    let mut reopened = ProjectSession::open(loaded);
    let undo = reopened.execute_command(CommandEnvelope {
        command_id: "history.undo".to_owned(),
        schema_version: 1,
        project_id: reopened.project_id(),
        project_instance_id: reopened.project_instance_id(),
        expected_project_revision: reopened.project_revision(),
        arguments: json!({}),
    });
    assert_eq!(undo.unwrap_err().code, OperationErrorCode::NothingToUndo);
}

#[test]
fn atomically_replaces_an_existing_project_file() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    fs::write(&path, b"previous file contents").unwrap();
    let document = ProjectDocument::new("Replacement");

    save_project_file_atomic(&path, &document).unwrap();

    assert_eq!(load_project_file(path).unwrap(), document);
    assert_eq!(names(&directory.0), vec!["example.orproj".to_owned()]);
}

#[test]
fn saves_to_and_loads_from_unicode_paths() {
    let directory = TestDirectory::new();
    let unicode_directory = directory.0.join("Tiếng Việt 🎬");
    fs::create_dir(&unicode_directory).unwrap();
    let path = unicode_directory.join("dự án.orproj");
    let document = ProjectDocument::new("Ví dụ");

    save_project_file_atomic(&path, &document).unwrap();

    assert_eq!(load_project_file(path).unwrap(), document);
}

#[test]
fn rejects_invalid_utf8_and_invalid_project_codec_data() {
    let directory = TestDirectory::new();
    let invalid_utf8 = directory.0.join("invalid-utf8.orproj");
    fs::write(&invalid_utf8, [0xff, 0xfe]).unwrap();
    assert!(matches!(
        load_project_file(invalid_utf8),
        Err(ProjectStorageError::InvalidUtf8)
    ));

    let invalid_project = directory.0.join("invalid-project.orproj");
    fs::write(&invalid_project, b"not a project").unwrap();
    assert!(matches!(
        load_project_file(invalid_project),
        Err(ProjectStorageError::Codec(_))
    ));
}

#[test]
fn rejects_files_over_the_limit_before_decoding_them() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    File::create(&path)
        .unwrap()
        .set_len(MAX_PROJECT_FILE_BYTES + 1)
        .unwrap();

    assert!(matches!(
        load_project_file(path),
        Err(ProjectStorageError::TooLarge { max_bytes }) if max_bytes == MAX_PROJECT_FILE_BYTES
    ));
}

#[test]
fn rejects_missing_files_and_does_not_create_missing_parent_directories() {
    let directory = TestDirectory::new();
    let missing = directory.project_path();
    assert!(matches!(
        load_project_file(&missing),
        Err(ProjectStorageError::Io(error)) if error.kind() == io::ErrorKind::NotFound
    ));

    let parent = directory.0.join("missing-parent");
    let error = save_project_file_atomic(parent.join("example.orproj"), &ProjectDocument::new("A"))
        .unwrap_err();
    assert!(matches!(
        error,
        ProjectStorageError::TemporaryFile {
            operation: or_core::TempFileOperation::Create,
            ..
        }
    ));
    assert!(!parent.exists());
}

#[test]
fn failed_replacement_preserves_destination_and_removes_the_temporary_sibling() {
    let directory = TestDirectory::new();
    let destination = directory.0.join("existing-directory");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("marker"), b"keep").unwrap();

    assert!(matches!(
        save_project_file_atomic(&destination, &ProjectDocument::new("A")),
        Err(ProjectStorageError::Replace(_))
    ));

    assert_eq!(fs::read(destination.join("marker")).unwrap(), b"keep");
    assert_eq!(names(&directory.0), vec!["existing-directory".to_owned()]);
}

#[test]
fn rejects_oversized_encoded_state_before_creating_a_temporary_file() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    fs::write(&path, b"preserve the old project").unwrap();
    let too_long_name = "x".repeat(MAX_PROJECT_FILE_BYTES as usize);
    let document = ProjectDocument::new(too_long_name);

    assert!(matches!(
        save_project_file_atomic(&path, &document),
        Err(ProjectStorageError::TooLarge { max_bytes }) if max_bytes == MAX_PROJECT_FILE_BYTES
    ));

    assert_eq!(fs::read(&path).unwrap(), b"preserve the old project");
    assert_eq!(names(&directory.0), vec!["example.orproj".to_owned()]);
}

#[test]
fn create_new_project_round_trips_without_clobbering_existing_files() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let mut session = ProjectFileSession::create_new(&path, "Exact project name ").unwrap();
    let project_id = session.session().project_id();
    let instance_id = session.session().project_instance_id();
    let initial = load_project_file(&path).unwrap();

    assert_eq!(initial.name(), "Exact project name ");
    assert_eq!(initial.revision(), ProjectRevision::INITIAL);
    assert_eq!(
        session.session().project_revision(),
        ProjectRevision::INITIAL
    );
    assert!(!session.is_dirty());
    assert!(
        std::str::from_utf8(&fs::read(&path).unwrap())
            .unwrap()
            .contains("\"schema_version\": 2")
    );

    let undo = session.handle_application_request(ApplicationRequest::Command(CommandEnvelope {
        command_id: "history.undo".to_owned(),
        schema_version: 1,
        project_id,
        project_instance_id: instance_id,
        expected_project_revision: ProjectRevision::INITIAL,
        arguments: json!({}),
    }));
    assert!(matches!(undo, ApplicationResponse::Error(_)));

    let original = ProjectDocument::new("Do not replace");
    save_project_file_atomic(&path, &original).unwrap();
    let original_bytes = fs::read(&path).unwrap();
    let error = ProjectFileSession::create_new(&path, "Replacement").unwrap_err();

    assert_eq!(error.code(), ProjectFileSessionErrorCode::DestinationExists);
    assert_eq!(fs::read(&path).unwrap(), original_bytes);
    assert_eq!(load_project_file(path).unwrap(), original);
}
