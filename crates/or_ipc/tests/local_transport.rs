use or_core::{
    ApplicationRequest, CommandCall, CommandEnvelope, ProjectDocument, ProjectFileSession,
    ProjectId, ProjectInstanceId, ProjectRevision, ProjectSession, QueryEnvelope,
    RecoveryInspection, TransactionEnvelope, discard_project_recovery, inspect_project_recovery,
    load_project_file, save_project_file_atomic, write_recovery_checkpoint,
};
use or_ipc::{
    ApplicationSuccess, IpcClientError, IpcErrorCode, IpcProtocolError, LocalIpcClient,
    LocalIpcServer,
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use uuid::Uuid;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("or-ipc-integration-{}", Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn project_path(&self) -> PathBuf {
        self.0.join("sample.orproj")
    }

    fn descriptor_path(&self) -> PathBuf {
        self.0.join("session.json")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn new_project(path: &PathBuf) -> ProjectDocument {
    let project = ProjectDocument::new("A");
    save_project_file_atomic(path, &project).unwrap();
    project
}

fn command(
    command_id: &str,
    project_id: ProjectId,
    instance_id: ProjectInstanceId,
    revision: u64,
    arguments: Value,
) -> CommandEnvelope {
    CommandEnvelope {
        command_id: command_id.to_owned(),
        schema_version: 1,
        project_id,
        project_instance_id: instance_id,
        expected_project_revision: ProjectRevision::new(revision),
        arguments,
    }
}

fn client_application_error(
    result: Result<ApplicationSuccess, IpcClientError>,
) -> or_core::OperationError {
    match result.unwrap_err() {
        IpcProtocolError::Application(error) => error,
        other => panic!("expected application error, received {other}"),
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn real_local_transport_runs_semantic_requests_saves_and_shuts_down_cleanly() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    let descriptor_path = directory.descriptor_path();
    let original = new_project(&project_path);
    let session = ProjectFileSession::open(&project_path).unwrap();
    let server = LocalIpcServer::start(session, Some(&descriptor_path)).unwrap();
    let endpoint = server.descriptor().endpoint().to_owned();
    let client = LocalIpcClient::open(&descriptor_path).unwrap();

    let descriptor_contents = fs::read_to_string(&descriptor_path).unwrap();
    assert!(!descriptor_contents.contains(&project_path.to_string_lossy().to_string()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&descriptor_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&endpoint).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(PathBuf::from(&endpoint).parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }

    let describe = client.describe().unwrap();
    assert_eq!(describe.protocol_version, 1);
    assert_eq!(describe.project_id, original.id());
    assert_eq!(describe.project_revision, ProjectRevision::INITIAL);
    assert!(!describe.dirty);
    assert_eq!(
        describe
            .commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>(),
        ["project.rename", "history.undo", "history.redo"]
    );
    assert_eq!(
        describe
            .queries
            .iter()
            .map(|query| query.id.as_str())
            .collect::<Vec<_>>(),
        ["project.summary"]
    );
    let describe_json = serde_json::to_string(&describe).unwrap();
    assert!(!describe_json.contains("auth_token"));
    assert!(!describe_json.contains(&project_path.to_string_lossy().to_string()));
    let descriptor_value: Value = serde_json::from_str(&descriptor_contents).unwrap();
    assert_ne!(
        descriptor_value["auth_token"],
        describe.project_id.to_string()
    );
    assert_ne!(
        descriptor_value["auth_token"],
        describe.project_instance_id.to_string()
    );

    let summary = client
        .application(ApplicationRequest::Query(QueryEnvelope {
            query_id: "project.summary".to_owned(),
            schema_version: 1,
            project_id: describe.project_id,
            project_instance_id: describe.project_instance_id,
            arguments: json!({}),
        }))
        .unwrap();
    assert!(matches!(summary, ApplicationSuccess::Query(result) if result.summary.name == "A"));

    let renamed = client
        .application(ApplicationRequest::Command(command(
            "project.rename",
            describe.project_id,
            describe.project_instance_id,
            0,
            json!({ "name": "B" }),
        )))
        .unwrap();
    assert!(
        matches!(renamed, ApplicationSuccess::Command(result) if result.after_revision == ProjectRevision::new(1))
    );
    assert_eq!(load_project_file(&project_path).unwrap(), original);

    let instance_error =
        client_application_error(client.application(ApplicationRequest::Command(command(
            "project.rename",
            describe.project_id,
            ProjectInstanceId::generate(),
            1,
            json!({ "name": "wrong instance" }),
        ))));
    assert_eq!(
        instance_error.code,
        or_core::OperationErrorCode::ProjectInstanceMismatch
    );

    let stale_error =
        client_application_error(client.application(ApplicationRequest::Command(command(
            "project.rename",
            describe.project_id,
            describe.project_instance_id,
            0,
            json!({ "name": "stale" }),
        ))));
    assert_eq!(
        stale_error.code,
        or_core::OperationErrorCode::RevisionConflict
    );
    assert_eq!(stale_error.current_revision, Some(ProjectRevision::new(1)));

    let transaction = client
        .application(ApplicationRequest::Transaction(TransactionEnvelope {
            schema_version: 1,
            project_id: describe.project_id,
            project_instance_id: describe.project_instance_id,
            expected_project_revision: ProjectRevision::new(1),
            commands: vec![
                CommandCall {
                    command_id: "project.rename".to_owned(),
                    schema_version: 1,
                    arguments: json!({ "name": "C" }),
                },
                CommandCall {
                    command_id: "project.rename".to_owned(),
                    schema_version: 1,
                    arguments: json!({ "name": "D" }),
                },
            ],
        }))
        .unwrap();
    assert!(
        matches!(transaction, ApplicationSuccess::Transaction(result) if result.after_revision == ProjectRevision::new(2))
    );

    let undo = client
        .application(ApplicationRequest::Command(command(
            "history.undo",
            describe.project_id,
            describe.project_instance_id,
            2,
            json!({}),
        )))
        .unwrap();
    assert!(
        matches!(undo, ApplicationSuccess::Command(result) if result.after_revision == ProjectRevision::new(3))
    );
    let redo = client
        .application(ApplicationRequest::Command(command(
            "history.redo",
            describe.project_id,
            describe.project_instance_id,
            3,
            json!({}),
        )))
        .unwrap();
    assert!(
        matches!(redo, ApplicationSuccess::Command(result) if result.after_revision == ProjectRevision::new(4))
    );

    let save = client.save().unwrap();
    assert_eq!(save.project_revision, ProjectRevision::new(4));
    assert!(!save.dirty);
    let saved = load_project_file(&project_path).unwrap();
    assert_eq!(saved.name(), "D");
    assert_eq!(saved.revision(), ProjectRevision::new(4));

    let dirty = client
        .application(ApplicationRequest::Command(command(
            "project.rename",
            describe.project_id,
            describe.project_instance_id,
            4,
            json!({ "name": "unsaved" }),
        )))
        .unwrap();
    assert!(matches!(dirty, ApplicationSuccess::Command(_)));
    assert!(matches!(
        client.shutdown(false),
        Err(IpcProtocolError::Remote(IpcErrorCode::UnsavedChanges))
    ));
    assert!(client.describe().unwrap().dirty);
    let stale_descriptor_path = directory.0.join("stale.json");
    fs::copy(&descriptor_path, &stale_descriptor_path).unwrap();
    client.shutdown(true).unwrap();
    server.wait().unwrap();

    assert!(!descriptor_path.exists());
    assert!(stale_descriptor_path.exists());
    assert!(matches!(
        LocalIpcClient::open(&stale_descriptor_path)
            .unwrap()
            .describe(),
        Err(IpcProtocolError::EndpointUnavailable)
    ));
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    assert!(!PathBuf::from(endpoint).exists());
    assert_eq!(load_project_file(project_path).unwrap().name(), "D");
}

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn wrong_token_external_disk_change_and_new_recovery_are_protected() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    let descriptor_path = directory.descriptor_path();
    new_project(&project_path);
    let server = LocalIpcServer::start(
        ProjectFileSession::open(&project_path).unwrap(),
        Some(&descriptor_path),
    )
    .unwrap();
    let first_descriptor: Value =
        serde_json::from_slice(&fs::read(&descriptor_path).unwrap()).unwrap();
    let first_token = first_descriptor["auth_token"].as_str().unwrap().to_owned();
    let client = LocalIpcClient::open(&descriptor_path).unwrap();

    let mut wrong_descriptor: Value =
        serde_json::from_slice(&fs::read(&descriptor_path).unwrap()).unwrap();
    wrong_descriptor["auth_token"] = Value::String(Uuid::new_v4().to_string());
    let wrong_path = directory.0.join("wrong-token.json");
    fs::write(&wrong_path, serde_json::to_vec(&wrong_descriptor).unwrap()).unwrap();
    let wrong_client = LocalIpcClient::open(&wrong_path).unwrap();
    assert!(matches!(
        wrong_client.describe(),
        Err(IpcProtocolError::Remote(IpcErrorCode::AuthenticationFailed))
    ));
    assert_eq!(
        client.describe().unwrap().project_revision,
        ProjectRevision::INITIAL
    );

    let description = client.describe().unwrap();
    client
        .application(ApplicationRequest::Command(command(
            "project.rename",
            description.project_id,
            description.project_instance_id,
            0,
            json!({ "name": "live" }),
        )))
        .unwrap();
    let external = ProjectDocument::new("external");
    save_project_file_atomic(&project_path, &external).unwrap();
    assert!(matches!(
        client.save(),
        Err(IpcProtocolError::Remote(IpcErrorCode::ProjectFileChanged))
    ));
    assert_eq!(load_project_file(&project_path).unwrap(), external);
    assert!(client.describe().unwrap().dirty);
    client.shutdown(true).unwrap();
    server.wait().unwrap();

    let second_descriptor = directory.0.join("recovery-session.json");
    let second_server = LocalIpcServer::start(
        ProjectFileSession::open(&project_path).unwrap(),
        Some(&second_descriptor),
    )
    .unwrap();
    let second_descriptor_contents: Value =
        serde_json::from_slice(&fs::read(&second_descriptor).unwrap()).unwrap();
    assert_ne!(
        first_token,
        second_descriptor_contents["auth_token"].as_str().unwrap()
    );
    let second_client = LocalIpcClient::open(&second_descriptor).unwrap();
    let base = load_project_file(&project_path).unwrap();
    let mut recovery_session = ProjectSession::open(base.clone());
    recovery_session
        .execute_command(command(
            "project.rename",
            recovery_session.project_id(),
            recovery_session.project_instance_id(),
            0,
            json!({ "name": "recovered" }),
        ))
        .unwrap();
    write_recovery_checkpoint(&project_path, &base, recovery_session.project()).unwrap();
    assert!(matches!(
        inspect_project_recovery(&project_path).unwrap(),
        RecoveryInspection::Candidate(_)
    ));
    assert!(matches!(
        second_client.save(),
        Err(IpcProtocolError::Remote(IpcErrorCode::RecoveryRequired))
    ));
    assert!(matches!(
        inspect_project_recovery(&project_path).unwrap(),
        RecoveryInspection::Candidate(_)
    ));
    second_client.shutdown(false).unwrap();
    second_server.wait().unwrap();
    assert!(discard_project_recovery(&project_path).unwrap());
    assert_eq!(load_project_file(project_path).unwrap().name(), "external");
}

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn existing_descriptor_path_is_never_overwritten() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    let descriptor_path = directory.descriptor_path();
    new_project(&project_path);
    fs::write(&descriptor_path, b"user-owned").unwrap();

    let result = LocalIpcServer::start(
        ProjectFileSession::open(&project_path).unwrap(),
        Some(&descriptor_path),
    );
    assert!(matches!(result, Err(IpcProtocolError::Io(_))));
    assert_eq!(fs::read(descriptor_path).unwrap(), b"user-owned");
}
