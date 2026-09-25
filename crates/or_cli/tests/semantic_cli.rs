use or_core::{
    ApplicationRequest, ApplicationResponse, CommandEnvelope, ProjectDocument, ProjectFileSession,
    ProjectRevision, ProjectSession, QueryResult, load_project_file, save_project_file_atomic,
    write_recovery_checkpoint,
};
use or_ipc::{LiveProjectHost, ProjectHostEventKind};
use serde_json::Value;
use std::{
    ffi::OsString,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "or-cli-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn project_path(&self) -> PathBuf {
        self.0.join("project with spaces.orproj")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct RunningServer {
    child: Child,
    stopped: bool,
}

impl RunningServer {
    fn wait(&mut self) {
        let status = self.child.wait().expect("wait for CLI session host");
        self.stopped = true;
        assert!(status.success(), "session host exited with {status}");
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        if !self.stopped && self.child.try_wait().unwrap().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn cli(args: impl IntoIterator<Item = OsString>) -> Output {
    Command::new(env!("CARGO_BIN_EXE_or"))
        .args(args)
        .output()
        .expect("run or executable")
}

fn words(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn path_args(before: &[&str], flag: &str, path: &Path, after: &[&str]) -> Vec<OsString> {
    let mut args = words(before);
    args.push(flag.into());
    args.push(path.as_os_str().to_owned());
    args.extend(words(after));
    args
}

fn json_success(args: impl IntoIterator<Item = OsString>) -> (Value, Output) {
    let output = cli(args);
    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    (value, output)
}

fn create_project(path: &Path, name: &str) -> ProjectDocument {
    let project = ProjectDocument::new(name);
    save_project_file_atomic(path, &project).unwrap();
    project
}

fn live_command(summary: &QueryResult, command_id: &str, name: Option<&str>) -> ApplicationRequest {
    ApplicationRequest::Command(CommandEnvelope {
        command_id: command_id.to_owned(),
        schema_version: 1,
        project_id: summary.summary.project_id,
        project_instance_id: summary.summary.project_instance_id,
        expected_project_revision: summary.summary.project_revision,
        arguments: match name {
            Some(name) => serde_json::json!({ "name": name }),
            None => serde_json::json!({}),
        },
    })
}

fn rename_document(project: ProjectDocument, name: &str) -> ProjectDocument {
    let mut session = ProjectSession::open(project);
    let result = session.handle_application_request(ApplicationRequest::Command(CommandEnvelope {
        command_id: "project.rename".to_owned(),
        schema_version: 1,
        project_id: session.project_id(),
        project_instance_id: session.project_instance_id(),
        expected_project_revision: session.project_revision(),
        arguments: serde_json::json!({"name": name}),
    }));
    assert!(matches!(result, ApplicationResponse::Command(result) if result.changed));
    session.project().clone()
}

fn sidecar_path(path: &Path) -> PathBuf {
    let mut name = OsString::from(".");
    name.push(path.file_name().unwrap());
    name.push(".or-recovery");
    path.parent().unwrap().join(name)
}

fn start_server(path: &Path, descriptor: &Path) -> (RunningServer, String) {
    let mut args = path_args(&["session", "serve"], "--file", path, &["--descriptor"]);
    args.push(descriptor.as_os_str().to_owned());
    args.push("--json".into());
    let child = Command::new(env!("CARGO_BIN_EXE_or"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start CLI session host");
    let mut server = RunningServer {
        child,
        stopped: false,
    };
    let mut startup = String::new();
    BufReader::new(server.child.stdout.take().unwrap())
        .read_line(&mut startup)
        .expect("read startup event");
    assert!(
        !startup.is_empty(),
        "session host did not print startup data"
    );
    let value: Value = serde_json::from_str(&startup).expect("startup is one JSON object");
    assert_eq!(value["status"], "session_started");
    assert_eq!(
        PathBuf::from(value["descriptor_path"].as_str().unwrap()),
        descriptor
    );
    let descriptor_json: Value = serde_json::from_slice(&fs::read(descriptor).unwrap()).unwrap();
    let token = descriptor_json["auth_token"].as_str().unwrap().to_owned();
    assert!(!startup.contains(&token));
    (server, token)
}

fn attach_args(before: &[&str], descriptor: &Path, after: &[&str]) -> Vec<OsString> {
    path_args(before, "--attach", descriptor, after)
}

fn assert_no_token(output: &Output, token: &str) {
    assert!(!String::from_utf8_lossy(&output.stdout).contains(token));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(token));
}

fn without_instance_id(mut value: Value) -> Value {
    value
        .as_object_mut()
        .expect("semantic result is an object")
        .remove("project_instance_id");
    value
}

#[test]
fn discovery_and_headless_project_commands_use_core_contracts() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let original = create_project(&path, "Starter");

    let (commands, _) = json_success(words(&["commands", "--json"]));
    let expected_commands = serde_json::to_value(or_core::command_catalog()).unwrap();
    assert_eq!(commands, expected_commands);
    let (queries, _) = json_success(words(&["queries", "--json"]));
    assert_eq!(
        queries,
        serde_json::to_value(or_core::query_catalog()).unwrap()
    );

    let mut summary_args = path_args(&["project", "summary"], "--file", &path, &[]);
    summary_args.push("--json".into());
    let (summary, _) = json_success(summary_args);
    assert_eq!(summary["query_id"], "project.summary");
    assert_eq!(summary["project_id"], original.id().to_string());
    assert_eq!(summary["project_revision"], 0);
    assert_eq!(summary["name"], "Starter");

    let mut rename_args = path_args(
        &["project", "rename"],
        "--file",
        &path,
        &["--name", "Edited"],
    );
    rename_args.push("--json".into());
    let (rename, _) = json_success(rename_args);
    assert_eq!(rename["command_id"], "project.rename");
    assert_eq!(rename["changed"], true);
    assert_eq!(rename["after_revision"], 1);
    let saved = load_project_file(&path).unwrap();
    assert_eq!(saved.name(), "Edited");
    assert_eq!(saved.revision(), ProjectRevision::new(1));

    let mut no_op_args = path_args(
        &["project", "rename"],
        "--file",
        &path,
        &["--name", "Edited"],
    );
    no_op_args.push("--json".into());
    let (no_op, _) = json_success(no_op_args);
    assert_eq!(no_op["changed"], false);
    assert_eq!(no_op["before_revision"], 1);
    assert_eq!(no_op["after_revision"], 1);
    assert_eq!(load_project_file(&path).unwrap(), saved);
}

#[test]
fn recovery_cli_reports_all_states_and_requires_explicit_actions() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let original = create_project(&path, "Canonical");

    let (none, _) = json_success(path_args(
        &["recovery", "status"],
        "--file",
        &path,
        &["--json"],
    ));
    assert_eq!(none["status"], "none");

    let candidate = rename_document(original.clone(), "Recovered");
    write_recovery_checkpoint(&path, &original, &candidate).unwrap();
    let (status, _) = json_success(path_args(
        &["recovery", "status"],
        "--file",
        &path,
        &["--json"],
    ));
    assert_eq!(status["status"], "candidate");
    assert_eq!(status["metadata"]["recovery_revision"], 1);

    let blocked = cli(path_args(
        &["project", "rename"],
        "--file",
        &path,
        &["--name", "Should not save", "--json"],
    ));
    assert_eq!(blocked.status.code(), Some(4));
    let blocked_error: Value = serde_json::from_slice(&blocked.stdout).unwrap();
    assert_eq!(blocked_error["error"]["code"], "RECOVERY_REQUIRED");
    assert_eq!(load_project_file(&path).unwrap(), original);

    let (applied, _) = json_success(path_args(
        &["recovery", "apply"],
        "--file",
        &path,
        &["--json"],
    ));
    assert_eq!(applied["status"], "applied");
    assert_eq!(load_project_file(&path).unwrap(), candidate);

    let current = load_project_file(&path).unwrap();
    let next = rename_document(current.clone(), "Saved after checkpoint");
    write_recovery_checkpoint(&path, &current, &next).unwrap();
    save_project_file_atomic(&path, &next).unwrap();
    let (stale, _) = json_success(path_args(
        &["recovery", "status"],
        "--file",
        &path,
        &["--json"],
    ));
    assert_eq!(stale["status"], "stale");
    let (discarded, _) = json_success(path_args(
        &["recovery", "discard"],
        "--file",
        &path,
        &["--json"],
    ));
    assert_eq!(discarded["discarded"], true);
    assert_eq!(load_project_file(&path).unwrap(), next);

    let base = load_project_file(&path).unwrap();
    let recovery = rename_document(base.clone(), "Conflict checkpoint");
    write_recovery_checkpoint(&path, &base, &recovery).unwrap();
    let foreign = ProjectDocument::new("Foreign project");
    save_project_file_atomic(&path, &foreign).unwrap();
    let (conflict, _) = json_success(path_args(
        &["recovery", "status"],
        "--file",
        &path,
        &["--json"],
    ));
    assert_eq!(conflict["status"], "conflict");
    assert_eq!(conflict["reason"], "FOREIGN_PROJECT");
    assert_eq!(load_project_file(&path).unwrap(), foreign);
    let (discarded, _) = json_success(path_args(
        &["recovery", "discard"],
        "--file",
        &path,
        &["--json"],
    ));
    assert_eq!(discarded["discarded"], true);

    fs::write(sidecar_path(&path), b"{").unwrap();
    let malformed = cli(path_args(
        &["recovery", "status"],
        "--file",
        &path,
        &["--json"],
    ));
    assert_eq!(malformed.status.code(), Some(4));
    let malformed_error: Value = serde_json::from_slice(&malformed.stdout).unwrap();
    assert_eq!(malformed_error["error"]["category"], "recovery");
    assert_eq!(malformed_error["error"]["code"], "INVALID_ENVELOPE");
    let (discarded, _) = json_success(path_args(
        &["recovery", "discard"],
        "--file",
        &path,
        &["--json"],
    ));
    assert_eq!(discarded["discarded"], true);
}

#[test]
fn attached_cli_uses_live_session_and_saves_only_on_request() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let descriptor = directory.0.join("session.json");
    create_project(&path, "Attached project");
    let headless_path = directory.0.join("headless.orproj");
    fs::copy(&path, &headless_path).unwrap();
    let (mut server, token) = start_server(&path, &descriptor);

    let (description, output) = json_success(attach_args(
        &["session", "describe"],
        &descriptor,
        &["--json"],
    ));
    assert_no_token(&output, &token);
    assert_eq!(description["protocol_version"], 1);
    assert_eq!(description["dirty"], false);
    assert!(
        description["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "project.rename")
    );

    let (summary, output) = json_success(attach_args(
        &["project", "summary"],
        &descriptor,
        &["--json"],
    ));
    let (headless_summary, _) = json_success(path_args(
        &["project", "summary"],
        "--file",
        &headless_path,
        &["--json"],
    ));
    assert_no_token(&output, &token);
    assert_eq!(summary["name"], "Attached project");
    assert_eq!(summary["query_id"], "project.summary");
    assert_eq!(
        without_instance_id(summary.clone()),
        without_instance_id(headless_summary)
    );

    let (headless_rename, _) = json_success(path_args(
        &["project", "rename"],
        "--file",
        &headless_path,
        &["--name", "Live edit", "--json"],
    ));

    let mut rename_args = attach_args(
        &["project", "rename"],
        &descriptor,
        &["--name", "Live edit"],
    );
    rename_args.push("--json".into());
    let (rename, output) = json_success(rename_args);
    assert_no_token(&output, &token);
    assert_eq!(rename["command_id"], "project.rename");
    assert_eq!(rename["after_revision"], 1);
    assert_eq!(
        without_instance_id(rename.clone()),
        without_instance_id(headless_rename)
    );
    assert_eq!(load_project_file(&path).unwrap().name(), "Attached project");

    let (dirty, output) = json_success(attach_args(
        &["session", "describe"],
        &descriptor,
        &["--json"],
    ));
    assert_no_token(&output, &token);
    assert_eq!(dirty["dirty"], true);

    let (undo, output) = json_success(attach_args(&["history", "undo"], &descriptor, &["--json"]));
    assert_no_token(&output, &token);
    assert_eq!(undo["command_id"], "history.undo");
    assert_eq!(undo["after_revision"], 2);

    let (redo, output) = json_success(attach_args(&["history", "redo"], &descriptor, &["--json"]));
    assert_no_token(&output, &token);
    assert_eq!(redo["command_id"], "history.redo");
    assert_eq!(redo["after_revision"], 3);

    let (saved, output) = json_success(attach_args(&["project", "save"], &descriptor, &["--json"]));
    assert_no_token(&output, &token);
    assert_eq!(saved["project_revision"], 3);
    assert_eq!(saved["dirty"], false);
    assert_eq!(load_project_file(&path).unwrap().name(), "Live edit");
    assert_eq!(
        load_project_file(&path).unwrap().revision(),
        ProjectRevision::new(3)
    );

    let (shutdown, output) = json_success(attach_args(
        &["session", "shutdown"],
        &descriptor,
        &["--json"],
    ));
    assert_no_token(&output, &token);
    assert_eq!(shutdown["status"], "shutdown");
    server.wait();
    assert!(!descriptor.exists());
}

#[test]
fn attached_cli_and_direct_live_host_access_share_one_session() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let session = ProjectFileSession::create_new(&path, "Native Project").unwrap();
    let mut host = LiveProjectHost::start(session, None).unwrap();
    let descriptor = host.descriptor_path().unwrap();
    let events = host.subscribe_events().unwrap();
    let initial = host.describe().unwrap();

    let direct_rename = host
        .handle_application_request(live_command(
            &initial,
            "project.rename",
            Some("From Flutter"),
        ))
        .unwrap();
    assert!(matches!(direct_rename, ApplicationResponse::Command(_)));
    let (attached_after_flutter, _) = json_success(attach_args(
        &["session", "describe"],
        &descriptor,
        &["--json"],
    ));
    assert_eq!(attached_after_flutter["protocol_version"], 1);
    assert_eq!(
        attached_after_flutter["project_id"],
        initial.summary.project_id.to_string()
    );
    assert_eq!(
        attached_after_flutter["project_instance_id"],
        initial.summary.project_instance_id.to_string()
    );
    assert_eq!(attached_after_flutter["project_revision"], 1);
    assert_eq!(attached_after_flutter["dirty"], true);
    let (summary_after_flutter, _) = json_success(attach_args(
        &["project", "summary"],
        &descriptor,
        &["--json"],
    ));
    assert_eq!(summary_after_flutter["name"], "From Flutter");

    let (cli_rename, _) = json_success(attach_args(
        &["project", "rename"],
        &descriptor,
        &["--name", "From attached CLI", "--json"],
    ));
    assert_eq!(cli_rename["after_revision"], 2);
    let after_cli_rename = host.describe().unwrap();
    assert_eq!(after_cli_rename.summary.name, "From attached CLI");
    assert_eq!(
        after_cli_rename.summary.project_instance_id,
        initial.summary.project_instance_id
    );

    let (cli_undo, _) = json_success(attach_args(&["history", "undo"], &descriptor, &["--json"]));
    assert_eq!(cli_undo["after_revision"], 3);
    assert_eq!(host.describe().unwrap().summary.name, "From Flutter");

    let after_undo = host.describe().unwrap();
    let direct_redo = host
        .handle_application_request(live_command(&after_undo, "history.redo", None))
        .unwrap();
    assert!(matches!(direct_redo, ApplicationResponse::Command(_)));
    let (cli_after_redo, _) = json_success(attach_args(
        &["project", "summary"],
        &descriptor,
        &["--json"],
    ));
    assert_eq!(cli_after_redo["name"], "From attached CLI");
    assert_eq!(cli_after_redo["project_revision"], 4);
    assert_eq!(
        cli_after_redo["project_instance_id"],
        initial.summary.project_instance_id.to_string()
    );

    assert_eq!(host.save().unwrap(), ProjectRevision::new(4));
    let (saved, _) = json_success(attach_args(
        &["session", "describe"],
        &descriptor,
        &["--json"],
    ));
    assert_eq!(saved["dirty"], false);
    assert_eq!(saved["project_revision"], 4);
    assert_eq!(
        load_project_file(&path).unwrap().name(),
        "From attached CLI"
    );

    let observed_events = (0..5)
        .map(|_| {
            events
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        observed_events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5]
    );
    assert_eq!(
        observed_events
            .iter()
            .map(|event| event.kind)
            .collect::<Vec<_>>(),
        [
            ProjectHostEventKind::ProjectChanged,
            ProjectHostEventKind::ProjectChanged,
            ProjectHostEventKind::ProjectChanged,
            ProjectHostEventKind::ProjectChanged,
            ProjectHostEventKind::ProjectSaved,
        ]
    );
    host.shutdown(false).unwrap();
}

#[test]
fn attached_save_reports_exact_disk_conflicts_without_overwriting() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let descriptor = directory.0.join("session.json");
    let original = create_project(&path, "Canonical project");
    let (mut server, token) = start_server(&path, &descriptor);

    let mut rename_args = attach_args(
        &["project", "rename"],
        &descriptor,
        &["--name", "Session edit"],
    );
    rename_args.push("--json".into());
    let (_, output) = json_success(rename_args);
    assert_no_token(&output, &token);

    let external = rename_document(original, "External edit");
    save_project_file_atomic(&path, &external).unwrap();
    let save = cli(attach_args(&["project", "save"], &descriptor, &["--json"]));
    assert_eq!(save.status.code(), Some(4));
    let save_error: Value = serde_json::from_slice(&save.stdout).unwrap();
    assert_eq!(save_error["error"]["category"], "project_file");
    assert_eq!(save_error["error"]["code"], "PROJECT_FILE_CHANGED");
    assert_eq!(load_project_file(&path).unwrap(), external);

    let (description, output) = json_success(attach_args(
        &["session", "describe"],
        &descriptor,
        &["--json"],
    ));
    assert_no_token(&output, &token);
    assert_eq!(description["dirty"], true);

    let (_, output) = json_success(attach_args(
        &["session", "shutdown"],
        &descriptor,
        &["--discard-unsaved", "--json"],
    ));
    assert_no_token(&output, &token);
    server.wait();
    assert_eq!(load_project_file(&path).unwrap(), external);
}

#[test]
fn attached_shutdown_requires_discard_and_cleans_server_resources() {
    let directory = TestDirectory::new();
    let path = directory.project_path();
    let descriptor = directory.0.join("session.json");
    create_project(&path, "Unsaved project");
    let (mut server, token) = start_server(&path, &descriptor);

    let mut rename_args = attach_args(
        &["project", "rename"],
        &descriptor,
        &["--name", "Unsaved edit"],
    );
    rename_args.push("--json".into());
    let (_, output) = json_success(rename_args);
    assert_no_token(&output, &token);

    let shutdown = cli(attach_args(
        &["session", "shutdown"],
        &descriptor,
        &["--json"],
    ));
    assert_eq!(shutdown.status.code(), Some(5));
    let shutdown_error: Value = serde_json::from_slice(&shutdown.stdout).unwrap();
    assert_eq!(shutdown_error["error"]["code"], "UNSAVED_CHANGES");
    assert!(server.child.try_wait().unwrap().is_none());

    let (shutdown, output) = json_success(attach_args(
        &["session", "shutdown"],
        &descriptor,
        &["--discard-unsaved", "--json"],
    ));
    assert_no_token(&output, &token);
    assert_eq!(shutdown["status"], "shutdown");
    server.wait();
    assert!(!descriptor.exists());
    assert_eq!(load_project_file(&path).unwrap().name(), "Unsaved project");
}

#[cfg(unix)]
#[test]
fn project_paths_remain_os_strings_and_names_require_utf8() {
    use std::os::unix::ffi::OsStringExt;

    let directory = TestDirectory::new();
    let path = directory.project_path();
    create_project(&path, "Unicode name");

    let (summary, _) = json_success(path_args(
        &["project", "summary"],
        "--file",
        &path,
        &["--json"],
    ));
    assert_eq!(summary["name"], "Unicode name");

    let invalid_name = OsString::from_vec(b"name-\xff".to_vec());
    let mut rename_args = words(&["project", "rename", "--file"]);
    rename_args.push(path.as_os_str().to_owned());
    rename_args.push("--name".into());
    rename_args.push(invalid_name);
    rename_args.push("--json".into());
    let error = cli(rename_args);
    assert_eq!(error.status.code(), Some(2));
    let value: Value = serde_json::from_slice(&error.stdout).unwrap();
    assert_eq!(value["error"]["code"], "INVALID_ARGUMENTS");
    assert_eq!(load_project_file(&path).unwrap().name(), "Unicode name");
}
