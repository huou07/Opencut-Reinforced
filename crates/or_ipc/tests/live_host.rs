use or_core::{
    ApplicationRequest, ApplicationResponse, CommandEnvelope, OperationErrorCode,
    ProjectFileSession, ProjectRevision, QueryEnvelope, QueryResult, RationalRate,
};
use or_ipc::{
    ApplicationSuccess, IpcProtocolError, LiveProjectHost, LocalIpcClient,
    OR_LOCAL_IPC_PROTOCOL_VERSION, ProjectHostEventKind,
};
use serde_json::json;
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("or-live-host-{}", Uuid::new_v4()));
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

fn command(summary: &QueryResult, id: &str, name: Option<&str>) -> ApplicationRequest {
    ApplicationRequest::Command(CommandEnvelope {
        command_id: id.to_owned(),
        schema_version: 1,
        project_id: summary.summary.project_id,
        project_instance_id: summary.summary.project_instance_id,
        expected_project_revision: summary.summary.project_revision,
        arguments: match name {
            Some(name) => json!({ "name": name }),
            None => json!({}),
        },
    })
}

fn summary(client: &LocalIpcClient) -> QueryResult {
    let descriptor = client.describe().unwrap();
    match client
        .application(ApplicationRequest::Query(QueryEnvelope {
            query_id: "project.summary".to_owned(),
            schema_version: 1,
            project_id: descriptor.project_id,
            project_instance_id: descriptor.project_instance_id,
            arguments: json!({}),
        }))
        .unwrap()
    {
        ApplicationSuccess::Query(summary) => summary,
        _ => panic!("expected project.summary query"),
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn sequence_settings_use_generic_application_requests_over_ipc_v1() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    let descriptor_path = directory.descriptor_path();
    let session = ProjectFileSession::create_new(&project_path, "Sequence").unwrap();
    let mut host = LiveProjectHost::start(session, Some(&descriptor_path)).unwrap();
    let events = host.subscribe_events().unwrap();
    let client = LocalIpcClient::open(&descriptor_path).unwrap();
    let initial = summary(&client);

    assert_eq!(
        client.describe().unwrap().protocol_version,
        OR_LOCAL_IPC_PROTOCOL_VERSION
    );
    let settings_query = QueryEnvelope::timeline_sequence_settings(
        initial.summary.project_id,
        initial.summary.project_instance_id,
    );
    assert!(matches!(
        client
            .application(ApplicationRequest::Query(settings_query))
            .unwrap(),
        ApplicationSuccess::Query(result)
            if result.timeline_sequence_settings.as_ref().is_some_and(|settings| settings.sequence_frame_rate.is_none())
    ));

    let rate = RationalRate::new(30_000, 1_001).unwrap();
    let set_rate = CommandEnvelope::set_timeline_sequence_frame_rate(
        initial.summary.project_id,
        initial.summary.project_instance_id,
        initial.summary.project_revision,
        Some(rate),
    );
    assert!(matches!(
        client
            .application(ApplicationRequest::Command(set_rate))
            .unwrap(),
        ApplicationSuccess::Command(result)
            if result.changed && result.after_revision == ProjectRevision::new(1)
    ));
    let event = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(event.kind, ProjectHostEventKind::ProjectChanged);
    assert_eq!(event.project_revision, 1);
    assert!(event.dirty);

    let updated = summary(&client);
    assert!(matches!(
        client
            .application(ApplicationRequest::Query(QueryEnvelope::timeline_sequence_settings(
                updated.summary.project_id,
                updated.summary.project_instance_id,
            )))
            .unwrap(),
        ApplicationSuccess::Query(result)
            if result.timeline_sequence_settings.as_ref().is_some_and(|settings| settings.sequence_frame_rate == Some(rate))
    ));
    client.save().unwrap();
    host.shutdown(false).unwrap();
    let reloaded = ProjectFileSession::open(&project_path).unwrap();
    assert_eq!(
        reloaded
            .session()
            .project()
            .timeline()
            .sequence_frame_rate(),
        Some(rate)
    );
}

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn direct_access_and_attached_ipc_share_one_session_history_save_and_events() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    let descriptor_path = directory.descriptor_path();
    let session = ProjectFileSession::create_new(&project_path, "A").unwrap();
    let mut host = LiveProjectHost::start(session, Some(&descriptor_path)).unwrap();
    let events = host.subscribe_events().unwrap();
    let client = LocalIpcClient::open(&descriptor_path).unwrap();

    let initial = host.describe().unwrap();
    let attached = client.describe().unwrap();
    assert_eq!(attached.project_id, initial.summary.project_id);
    assert_eq!(
        attached.project_instance_id,
        initial.summary.project_instance_id
    );
    assert_eq!(attached.project_revision, initial.summary.project_revision);

    assert!(matches!(
        host.handle_application_request(command(&initial, "project.rename", Some("B")))
            .unwrap(),
        ApplicationResponse::Command(_)
    ));
    let direct_change = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(direct_change.sequence, 1);
    assert_eq!(direct_change.kind, ProjectHostEventKind::ProjectChanged);
    assert_eq!(direct_change.project_id, attached.project_id.to_string());
    assert_eq!(
        direct_change.project_instance_id,
        attached.project_instance_id.to_string()
    );
    assert_eq!(direct_change.project_revision, 1);
    assert!(direct_change.dirty);
    assert_eq!(summary(&client).summary.name, "B");

    let direct = host.describe().unwrap();
    client
        .application(command(&direct, "project.rename", Some("C")))
        .unwrap();
    let ipc_change = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(ipc_change.sequence, 2);
    assert_eq!(ipc_change.kind, ProjectHostEventKind::ProjectChanged);
    assert_eq!(ipc_change.project_revision, 2);
    assert_eq!(host.describe().unwrap().summary.name, "C");

    let ipc_state = host.describe().unwrap();
    client
        .application(command(&ipc_state, "history.undo", None))
        .unwrap();
    let undo_event = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(undo_event.sequence, 3);
    assert_eq!(host.describe().unwrap().summary.name, "B");
    assert_eq!(
        host.describe().unwrap().summary.project_revision,
        ProjectRevision::new(3)
    );

    let direct_state = host.describe().unwrap();
    host.handle_application_request(command(&direct_state, "history.redo", None))
        .unwrap();
    let redo_event = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(redo_event.sequence, 4);
    assert_eq!(summary(&client).summary.name, "C");
    assert_eq!(
        summary(&client).summary.project_revision,
        ProjectRevision::new(4)
    );

    client.save().unwrap();
    let cli_save = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(cli_save.sequence, 5);
    assert_eq!(cli_save.kind, ProjectHostEventKind::ProjectSaved);
    assert!(!cli_save.dirty);
    assert!(!host.is_dirty().unwrap());

    let clean = host.describe().unwrap();
    host.handle_application_request(command(&clean, "project.rename", Some("D")))
        .unwrap();
    let direct_dirty = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(direct_dirty.sequence, 6);
    assert!(direct_dirty.dirty);
    assert_eq!(host.save().unwrap(), ProjectRevision::new(5));
    let direct_save = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(direct_save.sequence, 7);
    assert_eq!(direct_save.kind, ProjectHostEventKind::ProjectSaved);
    assert!(!client.describe().unwrap().dirty);
    assert_eq!(
        summary(&client).summary.project_revision,
        ProjectRevision::new(5)
    );
    assert!(
        fs::read_to_string(&project_path)
            .unwrap()
            .contains("\"name\": \"D\"")
    );

    host.shutdown(false).unwrap();
    let closing = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(closing.sequence, 8);
    assert_eq!(closing.kind, ProjectHostEventKind::SessionClosing);
    assert!(!descriptor_path.exists());
    assert!(matches!(
        client.describe(),
        Err(IpcProtocolError::EndpointUnavailable)
    ));
}

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn dirty_host_refuses_implicit_shutdown_and_explicit_discard_keeps_disk_unchanged() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    let descriptor_path = directory.descriptor_path();
    let session = ProjectFileSession::create_new(&project_path, "Saved").unwrap();
    let original = fs::read(&project_path).unwrap();
    let mut host = LiveProjectHost::start(session, Some(&descriptor_path)).unwrap();
    let client = LocalIpcClient::open(&descriptor_path).unwrap();
    let summary = host.describe().unwrap();
    host.handle_application_request(command(&summary, "project.rename", Some("Unsaved")))
        .unwrap();

    assert!(host.shutdown(false).is_err());
    assert!(client.describe().is_ok());
    assert_eq!(fs::read(&project_path).unwrap(), original);

    host.shutdown(true).unwrap();
    assert!(!descriptor_path.exists());
    assert_eq!(fs::read(&project_path).unwrap(), original);
}

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
#[test]
fn revision_conflicts_reach_the_direct_host_without_automatic_retry() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    let descriptor_path = directory.descriptor_path();
    let session = ProjectFileSession::create_new(&project_path, "A").unwrap();
    let mut host = LiveProjectHost::start(session, Some(&descriptor_path)).unwrap();
    let client = LocalIpcClient::open(&descriptor_path).unwrap();
    let stale = host.describe().unwrap();
    client
        .application(command(&stale, "project.rename", Some("CLI")))
        .unwrap();

    let response = host
        .handle_application_request(command(&stale, "project.rename", Some("Flutter")))
        .unwrap();
    assert!(matches!(
        response,
        ApplicationResponse::Error(error) if error.code == OperationErrorCode::RevisionConflict
    ));
    assert_eq!(host.describe().unwrap().summary.name, "CLI");
    host.shutdown(true).unwrap();
}
