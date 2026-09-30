use or_core::{
    ApplicationRequest, ApplicationResponse, ClipId, CommandEnvelope, MarkerId, ProjectDocument,
    ProjectFileSession, ProjectRevision, ProjectSession, QueryResult, TrackId, load_project_file,
    save_project_file_atomic, write_recovery_checkpoint,
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
    thread,
    time::{Duration, Instant},
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

    fn media_path(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, b"small generated CLI test input").unwrap();
        path
    }

    fn probe_stub(&self) -> PathBuf {
        let executable = self.0.join(if cfg!(windows) {
            "probe-stub.exe"
        } else {
            "probe-stub"
        });
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../or_core/tests/support/media_probe_stub.rs");
        let output = Command::new("rustc")
            .arg("--edition=2024")
            .arg(source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("run rustc to build a test-only probe helper");
        assert!(
            output.status.success(),
            "test helper compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
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

fn cli_with_probe(args: impl IntoIterator<Item = OsString>, executable: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_or"))
        .env("OR_FFPROBE_PATH", executable)
        .args(args)
        .output()
        .expect("run or executable with a test probe")
}

fn spawn_cli_with_probe_marker(
    args: impl IntoIterator<Item = OsString>,
    executable: &Path,
    marker: &Path,
) -> Child {
    Command::new(env!("CARGO_BIN_EXE_or"))
        .env("OR_FFPROBE_PATH", executable)
        .env("OR_FFPROBE_MARKER", marker)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run or executable with a marked test probe")
}

fn json_success_with_probe(
    args: impl IntoIterator<Item = OsString>,
    executable: &Path,
) -> (Value, Output) {
    let output = cli_with_probe(args, executable);
    assert!(
        output.status.success(),
        "expected success, stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    (value, output)
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
fn media_probe_cli_renders_human_and_or_json_for_unicode_path() {
    let directory = TestDirectory::new();
    let path = directory.media_path("cli sample café.mkv");
    let probe_stub = directory.probe_stub();

    let human = cli_with_probe(
        path_args(&["media", "probe"], "--file", &path, &[]),
        &probe_stub,
    );
    assert!(
        human.status.success(),
        "media probe failed: {}",
        String::from_utf8_lossy(&human.stderr)
    );
    assert!(human.stderr.is_empty());
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("Format: matroska, webm"));
    assert!(human.contains("Duration: 3/2 s"));
    assert!(human.contains("Streams: 2"));
    assert!(human.contains("Video #0: ffv1, 16x16, 24000/1001 fps"));
    assert!(human.contains("Audio #1: pcm_s16le, 48000 Hz, 2 channels"));

    let output = cli_with_probe(
        path_args(&["media", "probe"], "--file", &path, &["--json"]),
        &probe_stub,
    );
    assert!(
        output.status.success(),
        "media probe JSON failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        metadata["format_names"],
        serde_json::json!(["matroska", "webm"])
    );
    assert_eq!(metadata["duration"]["numerator"], 3);
    assert_eq!(metadata["duration"]["denominator"], 2);
    assert_eq!(metadata["streams"][0]["kind"], "video");
    assert_eq!(metadata["streams"][1]["kind"], "audio");
    assert!(metadata.get("path").is_none());
}

#[test]
fn media_probe_cli_returns_structured_backend_unavailable_error() {
    let directory = TestDirectory::new();
    let path = directory.media_path("input.mkv");
    let args = path_args(&["media", "probe"], "--file", &path, &["--json"]);
    let output = cli_with_probe(args, &directory.0.join("missing-probe"));
    assert!(!output.status.success());
    assert!(output.stderr.is_empty());
    let error: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(error["error"]["category"], "media_probe");
    assert_eq!(error["error"]["code"], "PROBE_BACKEND_UNAVAILABLE");
}

#[test]
fn media_probe_cli_reports_missing_files_and_bounded_probe_failure() {
    let directory = TestDirectory::new();
    let missing = directory.0.join("missing.mkv");
    let output = cli_with_probe(
        path_args(&["media", "probe"], "--file", &missing, &["--json"]),
        &directory.0.join("missing-probe"),
    );
    assert!(!output.status.success());
    let error: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(error["error"]["code"], "MEDIA_NOT_FOUND");

    let failure_path = directory.media_path("failure.mkv");
    let probe_stub = directory.probe_stub();
    let output = cli_with_probe(
        path_args(&["media", "probe"], "--file", &failure_path, &["--json"]),
        &probe_stub,
    );
    assert!(!output.status.success());
    let error: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(error["error"]["code"], "PROBE_FAILED");
    let diagnostic = error["error"]["details"]["diagnostic"]
        .as_str()
        .unwrap_or_else(|| panic!("expected bounded diagnostic in CLI error: {error}"));
    assert!(diagnostic.len() <= 512);
    assert!(!diagnostic.contains(&directory.0.to_string_lossy().to_string()));
}

#[test]
fn headless_media_commands_import_page_remove_and_save() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    create_project(&project_path, "Media CLI");
    let first_source = directory.media_path("cli sample café.mkv");
    let second_source = directory.media_path("path with spaces-媒体.mkv");
    let probe_stub = directory.probe_stub();

    let mut add_first_args = path_args(&["media", "add"], "--project", &project_path, &[]);
    add_first_args.extend(words(&["--source"]));
    add_first_args.push(first_source.as_os_str().to_owned());
    add_first_args.push("--json".into());
    let (add_first, _) = json_success_with_probe(add_first_args, &probe_stub);
    assert_eq!(add_first["media"]["source"]["kind"], "local_file");
    assert!(
        add_first["media"]["source"]["uri"]
            .as_str()
            .unwrap()
            .starts_with("file://")
    );
    assert_eq!(
        add_first["media"]["metadata"]["format_names"],
        serde_json::json!(["matroska", "webm"])
    );
    assert_eq!(add_first["command"]["command_id"], "media.add");
    assert_eq!(add_first["command"]["changed"], true);
    assert_eq!(add_first["command"]["after_revision"], 1);
    let first_id = add_first["media"]["id"].as_str().unwrap().to_owned();

    let mut add_second_args = path_args(&["media", "add"], "--project", &project_path, &[]);
    add_second_args.extend(words(&["--source"]));
    add_second_args.push(second_source.as_os_str().to_owned());
    add_second_args.push("--json".into());
    let (add_second, _) = json_success_with_probe(add_second_args, &probe_stub);
    let second_id = add_second["media"]["id"].as_str().unwrap().to_owned();
    assert_ne!(second_id, first_id);
    assert_eq!(
        load_project_file(&project_path)
            .unwrap()
            .media_items()
            .len(),
        2
    );
    let saved_bytes: Value = serde_json::from_slice(&fs::read(&project_path).unwrap()).unwrap();
    assert_eq!(saved_bytes["schema_version"], 5);

    let mut first_page_args = path_args(
        &["media", "list"],
        "--project",
        &project_path,
        &["--offset", "0", "--limit", "1", "--json"],
    );
    let (first_page, _) = json_success(first_page_args.drain(..));
    assert_eq!(first_page["query_id"], "media.list");
    assert_eq!(first_page["media_page"]["total_count"], 2);
    assert_eq!(first_page["media_page"]["offset"], 0);
    assert_eq!(first_page["media_page"]["limit"], 1);
    assert_eq!(first_page["media_page"]["next_offset"], 1);
    assert_eq!(first_page["media_page"]["items"][0]["id"], first_id);

    let (second_page, _) = json_success(path_args(
        &["media", "list"],
        "--project",
        &project_path,
        &["--offset", "1", "--limit", "1", "--json"],
    ));
    assert_eq!(second_page["media_page"]["items"][0]["id"], second_id);
    assert!(second_page["media_page"]["next_offset"].is_null());

    let human = cli(path_args(
        &["media", "list"],
        "--project",
        &project_path,
        &[],
    ));
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains(&first_id));
    assert!(human.contains(add_first["media"]["source"]["uri"].as_str().unwrap()));
    assert!(human.contains("matroska, webm"));
    assert!(human.contains("3/2 s"));
    assert!(human.contains("video 16x16 ffv1"));
    assert!(human.contains("audio 48000 Hz 2 ch pcm_s16le"));

    let mut duplicate_args = path_args(&["media", "add"], "--project", &project_path, &[]);
    duplicate_args.extend(words(&["--source"]));
    duplicate_args.push(first_source.as_os_str().to_owned());
    duplicate_args.push("--json".into());
    let duplicate = cli_with_probe(duplicate_args, &probe_stub);
    assert_eq!(duplicate.status.code(), Some(3));
    let duplicate_error: Value = serde_json::from_slice(&duplicate.stdout).unwrap();
    assert_eq!(
        duplicate_error["error"]["code"],
        "MEDIA_SOURCE_ALREADY_EXISTS"
    );
    assert_eq!(
        load_project_file(&project_path).unwrap().revision(),
        ProjectRevision::new(2)
    );

    let bad_limit = cli(path_args(
        &["media", "list"],
        "--project",
        &project_path,
        &["--limit", "0", "--json"],
    ));
    assert_eq!(bad_limit.status.code(), Some(3));
    let bad_limit: Value = serde_json::from_slice(&bad_limit.stdout).unwrap();
    assert_eq!(bad_limit["error"]["code"], "INVALID_ARGUMENTS");

    let wrong_file_option = cli(words(&[
        "media",
        "list",
        "--file",
        "unused.orproj",
        "--json",
    ]));
    assert_eq!(wrong_file_option.status.code(), Some(2));
    let wrong_file_option: Value = serde_json::from_slice(&wrong_file_option.stdout).unwrap();
    assert_eq!(wrong_file_option["error"]["category"], "usage");

    let remove = json_success(path_args(
        &["media", "remove"],
        "--project",
        &project_path,
        &["--id", &first_id, "--json"],
    ))
    .0;
    assert_eq!(remove["media_id"], first_id);
    assert_eq!(remove["command"]["command_id"], "media.remove");
    assert_eq!(remove["command"]["after_revision"], 3);
    let saved = load_project_file(&project_path).unwrap();
    assert_eq!(saved.revision(), ProjectRevision::new(3));
    assert_eq!(saved.media_items().len(), 1);
    assert_eq!(saved.media_items()[0].id().to_string(), second_id);
}

#[test]
fn attached_media_commands_share_history_and_wait_for_explicit_save() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    let session = ProjectFileSession::create_new(&project_path, "Attached media").unwrap();
    let mut host = LiveProjectHost::start(session, None).unwrap();
    let descriptor = host.descriptor_path().unwrap();
    let source = directory.media_path("cli sample café.mkv");
    let probe_stub = directory.probe_stub();

    let mut add_args = attach_args(&["media", "add"], &descriptor, &[]);
    add_args.extend(words(&["--source"]));
    add_args.push(source.as_os_str().to_owned());
    add_args.push("--json".into());
    let (added, _) = json_success_with_probe(add_args, &probe_stub);
    let media_id = added["media"]["id"].as_str().unwrap().to_owned();
    assert_eq!(added["command"]["after_revision"], 1);
    assert!(host.is_dirty().unwrap());
    assert!(
        load_project_file(&project_path)
            .unwrap()
            .media_items()
            .is_empty()
    );

    let (listed, _) = json_success(attach_args(
        &["media", "list"],
        &descriptor,
        &["--limit", "1", "--json"],
    ));
    assert_eq!(listed["media_page"]["items"][0]["id"], media_id);
    assert_eq!(listed["project_revision"], 1);

    let removed = json_success(attach_args(
        &["media", "remove"],
        &descriptor,
        &["--id", &media_id, "--json"],
    ))
    .0;
    assert_eq!(removed["command"]["after_revision"], 2);
    assert!(host.is_dirty().unwrap());

    let undone = json_success(attach_args(&["history", "undo"], &descriptor, &["--json"])).0;
    assert_eq!(undone["after_revision"], 3);
    let (restored, _) = json_success(attach_args(&["media", "list"], &descriptor, &["--json"]));
    assert_eq!(restored["media_page"]["items"][0], added["media"]);

    let redone = json_success(attach_args(&["history", "redo"], &descriptor, &["--json"])).0;
    assert_eq!(redone["after_revision"], 4);
    let (removed_again, _) =
        json_success(attach_args(&["media", "list"], &descriptor, &["--json"]));
    assert_eq!(removed_again["media_page"]["items"], serde_json::json!([]));

    let restored_again =
        json_success(attach_args(&["history", "undo"], &descriptor, &["--json"])).0;
    assert_eq!(restored_again["after_revision"], 5);
    assert_eq!(host.save().unwrap(), ProjectRevision::new(5));
    assert!(!host.is_dirty().unwrap());
    let saved = load_project_file(&project_path).unwrap();
    assert_eq!(saved.revision(), ProjectRevision::new(5));
    assert_eq!(saved.media_items()[0].id().to_string(), media_id);
    assert_eq!(
        saved.media_items()[0].source().uri(),
        added["media"]["source"]["uri"]
    );
    host.shutdown(false).unwrap();
}

#[test]
fn headless_marker_cli_round_trips_commands_pagination_generated_ids_and_v2_snap() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    create_project(&project_path, "Marker CLI");
    let marker_id = "11111111-1111-4111-8111-111111111111";
    let added = json_success(path_args(
        &["timeline", "add-marker"],
        "--project",
        &project_path,
        &[
            "--at", "2/1", "--label", "same", "--id", marker_id, "--json",
        ],
    ))
    .0;
    assert_eq!(added["marker_id"], marker_id);
    assert_eq!(added["command"]["command_id"], "timeline.marker.add");

    let generated = json_success(path_args(
        &["timeline", "add-marker"],
        "--project",
        &project_path,
        &["--at", "1/1", "--label", "same", "--json"],
    ))
    .0;
    let generated_id = generated["marker_id"].as_str().unwrap();
    assert!(generated_id.parse::<MarkerId>().is_ok());

    let page = json_success(path_args(
        &["timeline", "markers"],
        "--project",
        &project_path,
        &["--limit", "1", "--json"],
    ))
    .0;
    assert_eq!(page["timeline_marker_page"]["total_count"], 2);
    assert_eq!(page["timeline_marker_page"]["items"][0]["label"], "same");
    assert_eq!(page["timeline_marker_page"]["next_offset"], 1);

    let moved = json_success(path_args(
        &["timeline", "move-marker"],
        "--project",
        &project_path,
        &["--id", marker_id, "--to", "3/1", "--json"],
    ))
    .0;
    assert_eq!(moved["command"]["after_revision"], 3);
    let renamed = json_success(path_args(
        &["timeline", "rename-marker"],
        "--project",
        &project_path,
        &["--id", marker_id, "--label", "renamed", "--json"],
    ))
    .0;
    assert_eq!(renamed["command"]["after_revision"], 4);
    let deleted = json_success(path_args(
        &["timeline", "delete-marker"],
        "--project",
        &project_path,
        &["--id", generated_id, "--json"],
    ))
    .0;
    assert_eq!(deleted["command"]["after_revision"], 5);
    let source = directory.media_path("cli sample café.mkv");
    let probe_stub = directory.probe_stub();
    let (media, _) = json_success_with_probe(
        path_args(
            &["media", "add"],
            "--project",
            &project_path,
            &["--source", source.to_str().unwrap(), "--json"],
        ),
        &probe_stub,
    );
    let media_id = media["media"]["id"].as_str().unwrap();
    let track_id = "22222222-2222-4222-8222-222222222222";
    let clip_id = "33333333-3333-4333-8333-333333333333";
    json_success(path_args(
        &["timeline", "add-track"],
        "--project",
        &project_path,
        &["--kind", "video", "--id", track_id, "--json"],
    ));
    json_success(path_args(
        &["timeline", "insert-clip"],
        "--project",
        &project_path,
        &[
            "--track",
            track_id,
            "--media",
            media_id,
            "--at",
            "0/1",
            "--source-start",
            "0/1",
            "--duration",
            "1/1",
            "--id",
            clip_id,
            "--json",
        ],
    ));
    let marker_b = "55555555-5555-4555-8555-555555555555";
    json_success(path_args(
        &["timeline", "add-marker"],
        "--project",
        &project_path,
        &["--at", "5/1", "--label", "snap", "--id", marker_b, "--json"],
    ));
    let snap = json_success(path_args(
        &["timeline", "snap"],
        "--project",
        &project_path,
        &[
            "--clip",
            clip_id,
            "--operation",
            "trim-end",
            "--at",
            "81/16",
            "--json",
        ],
    ))
    .0;
    assert_eq!(snap["schema_version"], 2);
    assert_eq!(snap["timeline_snap"]["target_kind"], "marker");
    assert_eq!(snap["timeline_snap"]["target_marker_id"], marker_b);

    let persisted = load_project_file(&project_path).unwrap();
    assert_eq!(persisted.timeline().markers().len(), 2);
    let renamed = persisted
        .timeline()
        .markers()
        .iter()
        .find(|marker| marker.id().to_string() == marker_id)
        .unwrap();
    assert_eq!(renamed.label(), "renamed");
    assert_eq!(renamed.timeline_time().numerator(), 3);
}

#[test]
fn attached_marker_cli_stays_dirty_until_explicit_save() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    let descriptor = directory.0.join("marker-session.json");
    create_project(&project_path, "Attached markers");
    let (mut server, _) = start_server(&project_path, &descriptor);
    let marker_id = "11111111-1111-4111-8111-111111111111";
    let added = json_success(attach_args(
        &["timeline", "add-marker"],
        &descriptor,
        &[
            "--at", "4/1", "--label", "attached", "--id", marker_id, "--json",
        ],
    ))
    .0;
    assert_eq!(added["command"]["after_revision"], 1);
    assert!(
        load_project_file(&project_path)
            .unwrap()
            .timeline()
            .markers()
            .is_empty()
    );
    let page = json_success(attach_args(
        &["timeline", "markers"],
        &descriptor,
        &["--json"],
    ))
    .0;
    assert_eq!(
        page["timeline_marker_page"]["items"][0]["marker_id"],
        marker_id
    );
    json_success(attach_args(&["project", "save"], &descriptor, &["--json"]));
    assert_eq!(
        load_project_file(&project_path)
            .unwrap()
            .timeline()
            .markers()[0]
            .id()
            .to_string(),
        marker_id
    );
    let shutdown = json_success(attach_args(
        &["session", "shutdown"],
        &descriptor,
        &["--json"],
    ))
    .0;
    assert_eq!(shutdown["status"], "shutdown");
    server.wait();
}

#[test]
fn attached_media_import_rejects_revision_change_during_probe_without_retry() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    let session = ProjectFileSession::create_new(&project_path, "Before probe").unwrap();
    let mut host = LiveProjectHost::start(session, None).unwrap();
    let descriptor = host.descriptor_path().unwrap();
    let source = directory.media_path("sleep.mkv");
    let probe_stub = directory.probe_stub();
    let marker = directory.0.join("probe-started");
    let mut args = attach_args(&["media", "add"], &descriptor, &[]);
    args.extend(words(&["--source"]));
    args.push(source.as_os_str().to_owned());
    args.push("--json".into());
    let child = spawn_cli_with_probe_marker(args, &probe_stub, &marker);

    let deadline = Instant::now() + Duration::from_secs(5);
    while !marker.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        marker.exists(),
        "fake probe did not announce it had started"
    );

    let during_probe = host.describe().unwrap();
    let change = host
        .handle_application_request(live_command(
            &during_probe,
            "project.rename",
            Some("Changed during probe"),
        ))
        .unwrap();
    assert!(matches!(change, ApplicationResponse::Command(result) if result.changed));

    let output = child.wait_with_output().expect("wait for attached import");
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let error: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(error["error"]["code"], "REVISION_CONFLICT");
    let current = host.describe().unwrap();
    assert_eq!(current.summary.name, "Changed during probe");
    assert_eq!(current.summary.project_revision, ProjectRevision::new(1));
    let (list, _) = json_success(attach_args(&["media", "list"], &descriptor, &["--json"]));
    assert_eq!(list["media_page"]["items"], serde_json::json!([]));
    assert!(host.is_dirty().unwrap());
    host.shutdown(true).unwrap();
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

#[test]
fn headless_timeline_cli_uses_commands_saves_v5_and_keeps_exact_times() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    create_project(&project_path, "Timeline CLI");
    let source = directory.media_path("cli sample café.mkv");
    let probe_stub = directory.probe_stub();
    let mut add_media_args = path_args(&["media", "add"], "--project", &project_path, &[]);
    add_media_args.extend(words(&["--source"]));
    add_media_args.push(source.as_os_str().to_owned());
    add_media_args.push("--json".into());
    let (media, _) = json_success_with_probe(add_media_args, &probe_stub);
    let media_id = media["media"]["id"].as_str().unwrap().to_owned();

    let empty_tracks = json_success(path_args(
        &["timeline", "tracks"],
        "--project",
        &project_path,
        &["--json"],
    ))
    .0;
    assert_eq!(empty_tracks["query_id"], "timeline.tracks");
    assert_eq!(empty_tracks["timeline_tracks"], serde_json::json!([]));
    let track_id = "22222222-2222-4222-8222-222222222222";
    let added_track = json_success(path_args(
        &["timeline", "add-track"],
        "--project",
        &project_path,
        &["--kind", "video", "--id", track_id, "--json"],
    ))
    .0;
    assert_eq!(added_track["track_id"], track_id);
    assert_eq!(added_track["command"]["command_id"], "timeline.track.add");
    assert_eq!(added_track["command"]["after_revision"], 2);

    let generated_track = json_success(path_args(
        &["timeline", "add-track"],
        "--project",
        &project_path,
        &["--kind", "audio", "--json"],
    ))
    .0;
    let generated_track_id = generated_track["track_id"].as_str().unwrap();
    assert!(generated_track_id.parse::<TrackId>().is_ok());
    assert_eq!(generated_track["command"]["after_revision"], 3);
    let track_query = json_success(path_args(
        &["timeline", "tracks"],
        "--project",
        &project_path,
        &["--json"],
    ))
    .0;
    assert_eq!(track_query["timeline_tracks"][0]["track_id"], track_id);
    assert_eq!(track_query["timeline_tracks"][0]["kind"], "video");
    assert_eq!(
        track_query["timeline_tracks"][1]["track_id"],
        generated_track_id
    );
    assert_eq!(track_query["timeline_tracks"][1]["kind"], "audio");
    let human_tracks = cli(path_args(
        &["timeline", "tracks"],
        "--project",
        &project_path,
        &[],
    ));
    assert!(human_tracks.status.success());
    let human_tracks = String::from_utf8(human_tracks.stdout).unwrap();
    assert!(human_tracks.starts_with("Tracks at revision 3:\n"));
    assert!(human_tracks.contains(&format!("{track_id} video (0 clips)")));
    assert!(human_tracks.contains(&format!("{generated_track_id} audio (0 clips)")));

    let supplied_clip_id = "33333333-3333-4333-8333-333333333333";
    let inserted = json_success(path_args(
        &["timeline", "insert-clip"],
        "--project",
        &project_path,
        &[
            "--track",
            track_id,
            "--media",
            &media_id,
            "--at",
            "0/1",
            "--source-start",
            "1/2",
            "--duration",
            "1/2",
            "--id",
            supplied_clip_id,
            "--json",
        ],
    ))
    .0;
    assert_eq!(inserted["clip_id"], supplied_clip_id);
    assert_eq!(inserted["command"]["command_id"], "timeline.clip.insert");
    assert_eq!(inserted["command"]["after_revision"], 4);

    let snap = json_success(path_args(
        &["timeline", "snap"],
        "--project",
        &project_path,
        &[
            "--clip",
            supplied_clip_id,
            "--operation",
            "move",
            "--track",
            track_id,
            "--at",
            "1/16",
            "--json",
        ],
    ))
    .0;
    assert_eq!(snap["query_id"], "timeline.snap");
    assert_eq!(
        snap["timeline_snap"]["raw_target_time"],
        serde_json::json!({"numerator": 1, "denominator": 16})
    );
    assert_eq!(
        snap["timeline_snap"]["resolved_target_time"],
        serde_json::json!({"numerator": 0, "denominator": 1})
    );
    assert_eq!(snap["timeline_snap"]["snapped"], true);
    assert_eq!(snap["timeline_snap"]["target_kind"], "timeline_zero");

    let generated_clip = json_success(path_args(
        &["timeline", "insert-clip"],
        "--project",
        &project_path,
        &[
            "--track",
            track_id,
            "--media",
            &media_id,
            "--at",
            "4/1",
            "--source-start",
            "0/1",
            "--duration",
            "1/2",
            "--json",
        ],
    ))
    .0;
    let generated_clip_id = generated_clip["clip_id"].as_str().unwrap();
    assert!(generated_clip_id.parse::<ClipId>().is_ok());
    assert_eq!(generated_clip["command"]["after_revision"], 5);

    let moved = json_success(path_args(
        &["timeline", "move-clip"],
        "--project",
        &project_path,
        &[
            "--clip",
            supplied_clip_id,
            "--track",
            track_id,
            "--at",
            "3004/1001",
            "--json",
        ],
    ))
    .0;
    assert_eq!(moved["command"]["command_id"], "timeline.clip.move");
    assert_eq!(moved["command"]["after_revision"], 6);

    let page = json_success(path_args(
        &["timeline", "clips"],
        "--project",
        &project_path,
        &["--track", track_id, "--limit", "1", "--json"],
    ))
    .0;
    assert_eq!(page["query_id"], "timeline.clips");
    assert_eq!(page["timeline_clip_page"]["total_count"], 2);
    assert_eq!(
        page["timeline_clip_page"]["items"][0]["clip_id"],
        supplied_clip_id
    );
    assert_eq!(
        page["timeline_clip_page"]["items"][0]["timeline_start"]["numerator"],
        3004
    );
    assert_eq!(
        page["timeline_clip_page"]["items"][0]["timeline_start"]["denominator"],
        1001
    );
    assert_eq!(
        page["timeline_clip_page"]["items"][0]["source_range"]["start"]["numerator"],
        1
    );
    assert_eq!(
        page["timeline_clip_page"]["items"][0]["source_range"]["duration"]["denominator"],
        2
    );
    assert_eq!(page["timeline_clip_page"]["next_offset"], 1);

    let saved_bytes: Value = serde_json::from_slice(&fs::read(&project_path).unwrap()).unwrap();
    assert_eq!(saved_bytes["schema_version"], 5);
    let saved = load_project_file(&project_path).unwrap();
    assert_eq!(saved.revision(), ProjectRevision::new(6));
    assert_eq!(saved.timeline().tracks()[0].clips().len(), 2);
    assert_eq!(
        saved.timeline().tracks()[0].clips()[0].id().to_string(),
        supplied_clip_id
    );
    assert_eq!(
        saved.timeline().tracks()[0].clips()[1].id().to_string(),
        generated_clip_id
    );

    let invalid_ids = cli(path_args(
        &["timeline", "add-track"],
        "--project",
        &project_path,
        &[
            "--kind",
            "audio",
            "--id",
            "AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA",
            "--json",
        ],
    ));
    assert_eq!(invalid_ids.status.code(), Some(2));
    let invalid_ids: Value = serde_json::from_slice(&invalid_ids.stdout).unwrap();
    assert_eq!(invalid_ids["error"]["code"], "INVALID_ARGUMENTS");
    let invalid_clip_id = cli(path_args(
        &["timeline", "insert-clip"],
        "--project",
        &project_path,
        &[
            "--track",
            track_id,
            "--media",
            &media_id,
            "--at",
            "8/1",
            "--source-start",
            "0/1",
            "--duration",
            "1/2",
            "--id",
            "33333333-3333-4333-8333-33333333333A",
            "--json",
        ],
    ));
    assert_eq!(invalid_clip_id.status.code(), Some(2));
    let invalid_clip_id: Value = serde_json::from_slice(&invalid_clip_id.stdout).unwrap();
    assert_eq!(invalid_clip_id["error"]["code"], "INVALID_ARGUMENTS");
    assert_eq!(
        load_project_file(&project_path).unwrap().revision(),
        ProjectRevision::new(6)
    );

    for clip_id in [supplied_clip_id, generated_clip_id] {
        let deleted = json_success(path_args(
            &["timeline", "delete-clip"],
            "--project",
            &project_path,
            &["--clip", clip_id, "--json"],
        ))
        .0;
        assert_eq!(deleted["command"]["command_id"], "timeline.clip.delete");
    }
    for track_id in [track_id, generated_track_id] {
        let removed = json_success(path_args(
            &["timeline", "remove-track"],
            "--project",
            &project_path,
            &["--track", track_id, "--json"],
        ))
        .0;
        assert_eq!(removed["command"]["command_id"], "timeline.track.remove");
    }
    let empty_after_delete = json_success(path_args(
        &["timeline", "tracks"],
        "--project",
        &project_path,
        &["--json"],
    ))
    .0;
    assert_eq!(empty_after_delete["timeline_tracks"], serde_json::json!([]));
    assert_eq!(
        load_project_file(&project_path).unwrap().revision(),
        ProjectRevision::new(10)
    );
}

#[test]
fn timeline_cli_rational_parser_rejects_rounded_or_malformed_times() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    create_project(&project_path, "Timeline rational CLI");
    let source = directory.media_path("cli sample café.mkv");
    let probe_stub = directory.probe_stub();
    let mut add_media_args = path_args(&["media", "add"], "--project", &project_path, &[]);
    add_media_args.extend(words(&["--source"]));
    add_media_args.push(source.as_os_str().to_owned());
    add_media_args.push("--json".into());
    let (media, _) = json_success_with_probe(add_media_args, &probe_stub);
    let media_id = media["media"]["id"].as_str().unwrap().to_owned();
    let track_id = "22222222-2222-4222-8222-222222222222";
    json_success(path_args(
        &["timeline", "add-track"],
        "--project",
        &project_path,
        &["--kind", "video", "--id", track_id, "--json"],
    ));
    let accepted = json_success(path_args(
        &["timeline", "insert-clip"],
        "--project",
        &project_path,
        &[
            "--track",
            track_id,
            "--media",
            &media_id,
            "--at",
            "3003/1001",
            "--source-start",
            "0/1",
            "--duration",
            "1/2",
            "--id",
            "33333333-3333-4333-8333-333333333333",
            "--json",
        ],
    ))
    .0;
    assert_eq!(accepted["command"]["changed"], true);
    let accepted_page = json_success(path_args(
        &["timeline", "clips"],
        "--project",
        &project_path,
        &["--track", track_id, "--json"],
    ))
    .0;
    assert_eq!(
        accepted_page["timeline_clip_page"]["items"][0]["timeline_start"]["numerator"],
        3
    );
    assert_eq!(
        accepted_page["timeline_clip_page"]["items"][0]["timeline_start"]["denominator"],
        1
    );
    let revision_before = load_project_file(&project_path).unwrap().revision();

    for value in [
        "1.5",
        "1/0",
        "abc",
        "1/",
        "/1",
        "9223372036854775808/1",
        "0/4294967296",
    ] {
        let output = cli(path_args(
            &["timeline", "insert-clip"],
            "--project",
            &project_path,
            &[
                "--track",
                track_id,
                "--media",
                &media_id,
                "--at",
                value,
                "--source-start",
                "0/1",
                "--duration",
                "1/2",
                "--json",
            ],
        ));
        assert_eq!(output.status.code(), Some(2), "value {value}");
        let error: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(error["error"]["code"], "INVALID_ARGUMENTS", "value {value}");
        assert_eq!(
            load_project_file(&project_path).unwrap().revision(),
            revision_before
        );
    }
}

#[test]
fn headless_advanced_timeline_cli_uses_absolute_times_and_saves_the_v5_result() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    create_project(&project_path, "Advanced timeline CLI");
    let source = directory.media_path("cli sample café.mkv");
    let probe_stub = directory.probe_stub();
    let mut add_media_args = path_args(&["media", "add"], "--project", &project_path, &[]);
    add_media_args.extend(words(&["--source"]));
    add_media_args.push(source.as_os_str().to_owned());
    add_media_args.push("--json".into());
    let (media, _) = json_success_with_probe(add_media_args, &probe_stub);
    let media_id = media["media"]["id"].as_str().unwrap().to_owned();
    let track_id = "22222222-2222-4222-8222-222222222222";
    let clip_id = "33333333-3333-4333-8333-333333333333";
    json_success(path_args(
        &["timeline", "add-track"],
        "--project",
        &project_path,
        &["--kind", "video", "--id", track_id, "--json"],
    ));
    json_success(path_args(
        &["timeline", "insert-clip"],
        "--project",
        &project_path,
        &[
            "--track",
            track_id,
            "--media",
            &media_id,
            "--at",
            "0/1",
            "--source-start",
            "0/1",
            "--duration",
            "1/1",
            "--id",
            clip_id,
            "--json",
        ],
    ));
    let second_clip_id = "55555555-5555-4555-8555-555555555555";
    json_success(path_args(
        &["timeline", "insert-clip"],
        "--project",
        &project_path,
        &[
            "--track",
            track_id,
            "--media",
            &media_id,
            "--at",
            "3/1",
            "--source-start",
            "1/1",
            "--duration",
            "1/2",
            "--id",
            second_clip_id,
            "--json",
        ],
    ));

    let trimmed = json_success(path_args(
        &["timeline", "trim-clip"],
        "--project",
        &project_path,
        &["--clip", clip_id, "--edge", "end", "--to", "3/4", "--json"],
    ))
    .0;
    assert_eq!(trimmed["command"]["command_id"], "timeline.clip.trim");
    assert_eq!(trimmed["command"]["after_revision"], 5);

    let split = json_success(path_args(
        &["timeline", "split-clip"],
        "--project",
        &project_path,
        &["--clip", clip_id, "--at", "1/2", "--json"],
    ))
    .0;
    let generated_right_id = split["clip_id"].as_str().unwrap();
    assert!(generated_right_id.parse::<ClipId>().is_ok());
    assert_eq!(split["command"]["command_id"], "timeline.clip.split");
    assert_eq!(split["command"]["after_revision"], 6);

    let ripple = json_success(path_args(
        &["timeline", "ripple-delete-clip"],
        "--project",
        &project_path,
        &["--clip", generated_right_id, "--json"],
    ))
    .0;
    assert_eq!(
        ripple["command"]["command_id"],
        "timeline.clip.ripple_delete"
    );
    assert_eq!(ripple["command"]["after_revision"], 7);

    let no_op = json_success(path_args(
        &["timeline", "trim-clip"],
        "--project",
        &project_path,
        &["--clip", clip_id, "--edge", "end", "--to", "1/2", "--json"],
    ))
    .0;
    assert_eq!(no_op["command"]["changed"], false);
    assert_eq!(no_op["command"]["after_revision"], 7);

    let clips = json_success(path_args(
        &["timeline", "clips"],
        "--project",
        &project_path,
        &["--track", track_id, "--json"],
    ))
    .0;
    assert_eq!(clips["timeline_clip_page"]["total_count"], 2);
    assert_eq!(clips["timeline_clip_page"]["items"][0]["clip_id"], clip_id);
    assert_eq!(
        clips["timeline_clip_page"]["items"][0]["source_range"]["duration"]["numerator"],
        1
    );
    assert_eq!(
        clips["timeline_clip_page"]["items"][1]["clip_id"],
        second_clip_id
    );
    assert_eq!(
        clips["timeline_clip_page"]["items"][1]["timeline_start"]["numerator"],
        11
    );
    assert_eq!(
        clips["timeline_clip_page"]["items"][1]["timeline_start"]["denominator"],
        4
    );
    assert_eq!(
        load_project_file(&project_path).unwrap().revision(),
        ProjectRevision::new(7)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&project_path).unwrap()).unwrap()["schema_version"],
        5
    );
}

#[test]
fn sequence_settings_cli_uses_the_shared_command_and_query_paths() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    create_project(&project_path, "Sequence settings CLI");

    let initial = json_success(path_args(
        &["timeline", "settings"],
        "--project",
        &project_path,
        &["--json"],
    ))
    .0;
    assert_eq!(initial["query_id"], "timeline.sequence.settings");
    assert!(initial["timeline_sequence_settings"]["sequence_frame_rate"].is_null());

    let set = json_success(path_args(
        &["timeline", "set-frame-rate"],
        "--project",
        &project_path,
        &["--rate", "30000/1001", "--json"],
    ))
    .0;
    assert_eq!(
        set["command"]["command_id"],
        "timeline.sequence.set_frame_rate"
    );
    assert_eq!(set["command"]["after_revision"], 1);
    let settings = json_success(path_args(
        &["timeline", "settings"],
        "--project",
        &project_path,
        &["--json"],
    ))
    .0;
    assert_eq!(
        settings["timeline_sequence_settings"]["sequence_frame_rate"],
        serde_json::json!({"numerator": 30_000, "denominator": 1_001})
    );
    let no_op = json_success(path_args(
        &["timeline", "set-frame-rate"],
        "--project",
        &project_path,
        &["--rate", "30000/1001", "--json"],
    ))
    .0;
    assert_eq!(no_op["command"]["changed"], false);
    assert_eq!(no_op["command"]["after_revision"], 1);

    for value in ["24", "0/1", "24/0", "not-a-rate", "24/1/2"] {
        let invalid = cli(path_args(
            &["timeline", "set-frame-rate"],
            "--project",
            &project_path,
            &["--rate", value, "--json"],
        ));
        assert_eq!(invalid.status.code(), Some(2), "rate {value}");
        assert_eq!(
            load_project_file(&project_path).unwrap().revision(),
            ProjectRevision::new(1),
            "rate {value} must not mutate the project"
        );
    }

    let cleared = json_success(path_args(
        &["timeline", "clear-frame-rate"],
        "--project",
        &project_path,
        &["--json"],
    ))
    .0;
    assert_eq!(cleared["command"]["changed"], true);
    assert_eq!(cleared["command"]["after_revision"], 2);
    let cleared_again = json_success(path_args(
        &["timeline", "clear-frame-rate"],
        "--project",
        &project_path,
        &["--json"],
    ))
    .0;
    assert_eq!(cleared_again["command"]["changed"], false);
    assert_eq!(cleared_again["command"]["after_revision"], 2);

    let mut host =
        LiveProjectHost::start(ProjectFileSession::open(&project_path).unwrap(), None).unwrap();
    let descriptor = host.descriptor_path().unwrap();
    let attached = json_success(attach_args(
        &["timeline", "set-frame-rate"],
        &descriptor,
        &["--rate", "24/1", "--json"],
    ))
    .0;
    assert_eq!(attached["command"]["after_revision"], 3);
    assert!(host.is_dirty().unwrap());
    let attached_settings = json_success(attach_args(
        &["timeline", "settings"],
        &descriptor,
        &["--json"],
    ))
    .0;
    assert_eq!(
        attached_settings["timeline_sequence_settings"]["sequence_frame_rate"],
        serde_json::json!({"numerator": 24, "denominator": 1})
    );
    json_success(attach_args(&["project", "save"], &descriptor, &["--json"]));
    host.shutdown(false).unwrap();
    let saved = load_project_file(&project_path).unwrap();
    assert_eq!(saved.revision(), ProjectRevision::new(3));
    assert_eq!(
        saved.timeline().sequence_frame_rate(),
        Some(or_core::RationalRate::new(24, 1).unwrap())
    );
}

#[test]
fn attached_advanced_timeline_cli_dispatches_shared_commands_until_explicit_save() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    create_project(&project_path, "Attached advanced timeline CLI");
    let mut host =
        LiveProjectHost::start(ProjectFileSession::open(&project_path).unwrap(), None).unwrap();
    let descriptor = host.descriptor_path().unwrap();
    let source = directory.media_path("cli sample café.mkv");
    let probe_stub = directory.probe_stub();
    let mut add_media_args = attach_args(&["media", "add"], &descriptor, &[]);
    add_media_args.extend(words(&["--source"]));
    add_media_args.push(source.as_os_str().to_owned());
    add_media_args.push("--json".into());
    let (media, _) = json_success_with_probe(add_media_args, &probe_stub);
    let media_id = media["media"]["id"].as_str().unwrap().to_owned();
    let track_id = "22222222-2222-4222-8222-222222222222";
    let clip_id = "33333333-3333-4333-8333-333333333333";
    let right_id = "55555555-5555-4555-8555-555555555555";
    json_success(attach_args(
        &["timeline", "add-track"],
        &descriptor,
        &["--kind", "video", "--id", track_id, "--json"],
    ));
    json_success(attach_args(
        &["timeline", "insert-clip"],
        &descriptor,
        &[
            "--track",
            track_id,
            "--media",
            &media_id,
            "--at",
            "0/1",
            "--source-start",
            "0/1",
            "--duration",
            "1/1",
            "--id",
            clip_id,
            "--json",
        ],
    ));
    let snap = json_success(attach_args(
        &["timeline", "snap"],
        &descriptor,
        &[
            "--clip",
            clip_id,
            "--operation",
            "trim-start",
            "--at",
            "0/1",
            "--json",
        ],
    ))
    .0;
    assert_eq!(snap["query_id"], "timeline.snap");
    assert_eq!(snap["timeline_snap"]["snapped"], true);
    assert_eq!(
        host.describe().unwrap().summary.project_revision,
        ProjectRevision::new(3)
    );
    json_success(attach_args(
        &["timeline", "trim-clip"],
        &descriptor,
        &["--clip", clip_id, "--edge", "end", "--to", "3/4", "--json"],
    ));
    json_success(attach_args(
        &["timeline", "split-clip"],
        &descriptor,
        &["--clip", clip_id, "--at", "1/2", "--id", right_id, "--json"],
    ));
    let ripple = json_success(attach_args(
        &["timeline", "ripple-delete-clip"],
        &descriptor,
        &["--clip", right_id, "--json"],
    ))
    .0;
    assert_eq!(
        ripple["command"]["command_id"],
        "timeline.clip.ripple_delete"
    );
    assert!(host.is_dirty().unwrap());
    assert_eq!(
        load_project_file(&project_path).unwrap().revision(),
        ProjectRevision::INITIAL
    );
    let saved = json_success(attach_args(&["project", "save"], &descriptor, &["--json"])).0;
    assert_eq!(saved["project_revision"], 6);
    assert_eq!(
        load_project_file(&project_path).unwrap().revision(),
        ProjectRevision::new(6)
    );
    host.shutdown(false).unwrap();
}

#[test]
fn attached_timeline_cli_uses_the_shared_host_history_dirty_state_and_explicit_save() {
    let directory = TestDirectory::new();
    let project_path = directory.project_path();
    create_project(&project_path, "Attached timeline");
    let mut host =
        LiveProjectHost::start(ProjectFileSession::open(&project_path).unwrap(), None).unwrap();
    let descriptor = host.descriptor_path().unwrap();
    let events = host.subscribe_events().unwrap();
    let initial = host.describe().unwrap().summary;
    let source = directory.media_path("cli sample café.mkv");
    let probe_stub = directory.probe_stub();

    let mut add_media_args = attach_args(&["media", "add"], &descriptor, &[]);
    add_media_args.extend(words(&["--source"]));
    add_media_args.push(source.as_os_str().to_owned());
    add_media_args.push("--json".into());
    let (media, _) = json_success_with_probe(add_media_args, &probe_stub);
    let media_id = media["media"]["id"].as_str().unwrap().to_owned();
    assert_eq!(media["command"]["after_revision"], 1);
    assert!(host.is_dirty().unwrap());
    assert_eq!(
        events.recv().unwrap().kind,
        ProjectHostEventKind::ProjectChanged
    );
    assert!(
        load_project_file(&project_path)
            .unwrap()
            .media_items()
            .is_empty()
    );

    let track_id = "22222222-2222-4222-8222-222222222222";
    let added_track = json_success(attach_args(
        &["timeline", "add-track"],
        &descriptor,
        &["--kind", "video", "--id", track_id, "--json"],
    ))
    .0;
    assert_eq!(added_track["command"]["after_revision"], 2);
    assert_eq!(
        events.recv().unwrap().kind,
        ProjectHostEventKind::ProjectChanged
    );

    let inserted = json_success(attach_args(
        &["timeline", "insert-clip"],
        &descriptor,
        &[
            "--track",
            track_id,
            "--media",
            &media_id,
            "--at",
            "0/1",
            "--source-start",
            "1/2",
            "--duration",
            "1/2",
            "--json",
        ],
    ))
    .0;
    let clip_id = inserted["clip_id"].as_str().unwrap();
    assert!(clip_id.parse::<ClipId>().is_ok());
    assert_eq!(inserted["command"]["after_revision"], 3);
    assert_eq!(
        events.recv().unwrap().kind,
        ProjectHostEventKind::ProjectChanged
    );

    let tracks = json_success(attach_args(
        &["timeline", "tracks"],
        &descriptor,
        &["--json"],
    ))
    .0;
    assert_eq!(
        tracks["project_instance_id"],
        initial.project_instance_id.to_string()
    );
    assert_eq!(tracks["project_revision"], 3);
    assert_eq!(tracks["timeline_tracks"][0]["track_id"], track_id);
    let clips = json_success(attach_args(
        &["timeline", "clips"],
        &descriptor,
        &["--track", track_id, "--json"],
    ))
    .0;
    assert_eq!(clips["timeline_clip_page"]["items"][0]["clip_id"], clip_id);

    let moved = json_success(attach_args(
        &["timeline", "move-clip"],
        &descriptor,
        &[
            "--clip", clip_id, "--track", track_id, "--at", "2/1", "--json",
        ],
    ))
    .0;
    assert_eq!(moved["command"]["command_id"], "timeline.clip.move");
    assert_eq!(moved["command"]["after_revision"], 4);
    assert_eq!(
        events.recv().unwrap().kind,
        ProjectHostEventKind::ProjectChanged
    );

    let deleted = json_success(attach_args(
        &["timeline", "delete-clip"],
        &descriptor,
        &["--clip", clip_id, "--json"],
    ))
    .0;
    assert_eq!(deleted["command"]["command_id"], "timeline.clip.delete");
    assert_eq!(deleted["command"]["after_revision"], 5);
    assert_eq!(
        events.recv().unwrap().kind,
        ProjectHostEventKind::ProjectChanged
    );

    let undone = json_success(attach_args(&["history", "undo"], &descriptor, &["--json"])).0;
    assert_eq!(undone["after_revision"], 6);
    assert_eq!(
        events.recv().unwrap().kind,
        ProjectHostEventKind::ProjectChanged
    );
    let after_undo = json_success(attach_args(
        &["timeline", "clips"],
        &descriptor,
        &["--track", track_id, "--json"],
    ))
    .0;
    assert_eq!(
        after_undo["timeline_clip_page"]["items"][0]["timeline_start"]["numerator"],
        2
    );
    let undo_move = json_success(attach_args(&["history", "undo"], &descriptor, &["--json"])).0;
    assert_eq!(undo_move["after_revision"], 7);
    assert_eq!(
        events.recv().unwrap().kind,
        ProjectHostEventKind::ProjectChanged
    );
    let back_at_start = json_success(attach_args(
        &["timeline", "clips"],
        &descriptor,
        &["--track", track_id, "--json"],
    ))
    .0;
    assert_eq!(
        back_at_start["timeline_clip_page"]["items"][0]["timeline_start"]["numerator"],
        0
    );
    let redone = json_success(attach_args(&["history", "redo"], &descriptor, &["--json"])).0;
    assert_eq!(redone["after_revision"], 8);
    assert_eq!(
        events.recv().unwrap().kind,
        ProjectHostEventKind::ProjectChanged
    );
    let deleted_again = json_success(attach_args(
        &["timeline", "delete-clip"],
        &descriptor,
        &["--clip", clip_id, "--json"],
    ))
    .0;
    assert_eq!(deleted_again["command"]["after_revision"], 9);
    assert_eq!(
        events.recv().unwrap().kind,
        ProjectHostEventKind::ProjectChanged
    );
    let removed_track = json_success(attach_args(
        &["timeline", "remove-track"],
        &descriptor,
        &["--track", track_id, "--json"],
    ))
    .0;
    assert_eq!(removed_track["command"]["after_revision"], 10);
    assert_eq!(
        events.recv().unwrap().kind,
        ProjectHostEventKind::ProjectChanged
    );
    let tracks_after_remove = json_success(attach_args(
        &["timeline", "tracks"],
        &descriptor,
        &["--json"],
    ))
    .0;
    assert_eq!(
        tracks_after_remove["timeline_tracks"],
        serde_json::json!([])
    );
    assert!(host.is_dirty().unwrap());
    assert_eq!(
        load_project_file(&project_path).unwrap().revision(),
        ProjectRevision::INITIAL
    );

    let saved = json_success(attach_args(&["project", "save"], &descriptor, &["--json"])).0;
    assert_eq!(saved["project_revision"], 10);
    assert_eq!(
        events.recv().unwrap().kind,
        ProjectHostEventKind::ProjectSaved
    );
    assert!(!host.is_dirty().unwrap());
    let persisted = load_project_file(&project_path).unwrap();
    assert_eq!(persisted.revision(), ProjectRevision::new(10));
    assert!(persisted.timeline().tracks().is_empty());
    assert_eq!(persisted.media_items()[0].id().to_string(), media_id);
    assert_eq!(
        host.describe().unwrap().summary.project_instance_id,
        initial.project_instance_id
    );
    host.shutdown(false).unwrap();
}
