//! End-to-end coverage for `or mcp`: a real live host, a real child process,
//! and edits that must be visible in the saved project file.

use or_core::{ProjectFileSession, TrackKind};
use or_ipc::LiveProjectHost;
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("or-mcp-{}-{stamp}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn project_path(&self) -> PathBuf {
        self.0.join("assistant.orproj")
    }

    fn descriptor_path(&self) -> PathBuf {
        self.0.join("endpoint.json")
    }

    fn pointer_path(&self) -> PathBuf {
        self.0.join("session-pointer.json")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct McpProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpProcess {
    fn start(pointer: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_or"))
            .arg("mcp")
            .arg("--pointer")
            .arg(pointer)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn or mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout,
            next_id: 0,
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        writeln!(self.stdin, "{message}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        let response: Value = serde_json::from_str(&line).expect("MCP response is JSON");
        assert_eq!(response["id"], id, "response id");
        response
    }

    fn call(&mut self, name: &str, arguments: Value) -> Result<String, String> {
        let response = self.request("tools/call", json!({"name": name, "arguments": arguments}));
        let result = &response["result"];
        let text = result["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        if result["isError"].as_bool().unwrap_or(false) {
            Err(text)
        } else {
            Ok(text)
        }
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn write_pointer(directory: &TestDirectory) {
    let pointer = json!({
        "version": 1,
        "descriptor_path": directory.descriptor_path().display().to_string(),
        "project_path": directory.project_path().display().to_string(),
        "project_name": "Assistant test",
        "workspace_path": directory.0.display().to_string(),
    });
    fs::write(directory.pointer_path(), pointer.to_string()).unwrap();
}

#[test]
fn mcp_lists_tools_and_edits_the_live_project() {
    let directory = TestDirectory::new();
    let session = ProjectFileSession::create_new(directory.project_path().as_path(), "Assistant")
        .expect("create project");
    let host = LiveProjectHost::start(session, Some(directory.descriptor_path().as_path()))
        .expect("start live host");
    write_pointer(&directory);
    let mut mcp = McpProcess::start(directory.pointer_path().as_path());

    let initialized = mcp.request(
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "or-test", "version": "0"},
        }),
    );
    assert_eq!(
        initialized["result"]["serverInfo"]["name"],
        "opencut-reinforced"
    );

    let tools = mcp.request("tools/list", json!({}));
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(names.contains(&"timeline_overview"));
    assert!(names.contains(&"insert_text"));
    assert!(names.contains(&"motion_insert_scene"));
    for tool in tools["result"]["tools"].as_array().unwrap() {
        assert_eq!(tool["inputSchema"]["type"], "object", "{}", tool["name"]);
    }

    let status: Value =
        serde_json::from_str(&mcp.call("project_status", json!({})).unwrap()).unwrap();
    assert_eq!(status["name"], "Assistant");
    assert_eq!(
        status["project_path"],
        directory.project_path().display().to_string()
    );

    let added: Value =
        serde_json::from_str(&mcp.call("add_track", json!({"kind": "text"})).unwrap()).unwrap();
    let track_id = added["track_id"].as_str().expect("track id").to_owned();

    mcp.call(
        "insert_text",
        json!({
            "track_id": track_id,
            "text": "Hello from the assistant",
            "at": "1.5",
            "duration": "2",
        }),
    )
    .expect("insert text");

    mcp.call("add_marker", json!({"at": "2", "label": "AI marker"}))
        .expect("add marker");
    mcp.call("save", json!({})).expect("save project");

    // A fresh session reads the file, so the assertion is about persisted state
    // rather than the live host's cached document.
    let reopened = ProjectFileSession::open(directory.project_path().as_path()).unwrap();
    let project = reopened.session().project();
    let track = project
        .timeline()
        .tracks()
        .iter()
        .find(|track| track.kind() == TrackKind::Text)
        .expect("text track persisted");
    assert_eq!(track.clips().len(), 1);
    assert_eq!(track.clips()[0].timeline_start().numerator(), 3);
    assert_eq!(track.clips()[0].timeline_start().denominator(), 2);
    assert_eq!(project.timeline().markers().len(), 1);
    drop(host);
}

#[test]
fn motion_materialize_reports_when_the_host_has_no_runtime() {
    let directory = TestDirectory::new();
    let session = ProjectFileSession::create_new(directory.project_path().as_path(), "Assistant")
        .expect("create project");
    let host = LiveProjectHost::start(session, Some(directory.descriptor_path().as_path()))
        .expect("start live host");
    let source = directory.0.join("sample.rs");
    fs::write(&source, "fn main() { println!(\"hi\"); }\n").unwrap();
    let output = directory.0.join("out.mkv");

    let result = Command::new(env!("CARGO_BIN_EXE_or"))
        .args(["motion", "materialize", "--attach"])
        .arg(directory.descriptor_path())
        .args(["--source"])
        .arg(&source)
        .args(["--output"])
        .arg(&output)
        .arg("--json")
        .output()
        .expect("run or motion materialize");
    assert!(!result.status.success());
    let response: Value = serde_json::from_slice(&result.stdout).expect("JSON error on stdout");
    assert_eq!(response["error"]["code"], "MOTION_RUNTIME_UNAVAILABLE");
    drop(host);
}
