//! A local MCP (Model Context Protocol) server that lets an external agent
//! drive the running Opencut Reinforced project.
//!
//! The server owns no project state. Every tool call attaches to the live
//! application over the same local IPC descriptor the `or --attach` verbs use,
//! so an agent is another client of the validated command and query path. The
//! only privileged step is motion materialisation, which the application
//! runtime performs because it owns the FFmpeg encoder.

use or_core::RationalTime;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    process::Command,
};

const SERVER_NAME: &str = "opencut-reinforced";
const PROTOCOL_VERSIONS: [&str; 3] = ["2024-11-05", "2025-03-26", "2025-06-18"];
const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";
const MAX_PAGE: u32 = 100;

/// Runs the MCP stdio loop until stdin closes.
pub(super) fn run(args: &[OsString]) -> Result<(), String> {
    let mut descriptor: Option<PathBuf> = None;
    let mut pointer: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index]
            .to_str()
            .ok_or_else(|| "mcp option name must be valid UTF-8".to_owned())?;
        match flag {
            "--descriptor" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--descriptor requires a path".to_owned())?;
                descriptor = Some(PathBuf::from(value));
            }
            "--pointer" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--pointer requires a path".to_owned())?;
                pointer = Some(PathBuf::from(value));
            }
            "-h" | "--help" => {
                return Err(usage());
            }
            other => return Err(format!("unknown option: {other}\n\n{}", usage())),
        }
        index += 1;
    }
    if descriptor.is_none() && pointer.is_none() {
        return Err(format!(
            "one of --descriptor or --pointer is required\n\n{}",
            usage()
        ));
    }

    let server = Server {
        descriptor,
        pointer,
    };
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| format!("could not read a request: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let message: Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(error) => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {"code": -32700, "message": format!("parse error: {error}")},
                });
                write_message(&mut stdout, &response)?;
                continue;
            }
        };
        if let Some(response) = server.handle(&message) {
            write_message(&mut stdout, &response)?;
        }
    }
    Ok(())
}

fn usage() -> String {
    "Usage:\n  or mcp --descriptor PATH\n  or mcp --pointer PATH\n\nServes Model Context Protocol tools over stdio for the live project behind the local IPC descriptor. --pointer names a small JSON file the application rewrites when the open project changes; it is re-read on every tool call.".to_owned()
}

fn write_message(stdout: &mut impl Write, message: &Value) -> Result<(), String> {
    let encoded =
        serde_json::to_string(message).map_err(|error| format!("could not encode: {error}"))?;
    stdout
        .write_all(encoded.as_bytes())
        .and_then(|()| stdout.write_all(b"\n"))
        .and_then(|()| stdout.flush())
        .map_err(|error| format!("could not write a response: {error}"))
}

struct Server {
    descriptor: Option<PathBuf>,
    pointer: Option<PathBuf>,
}

impl Server {
    fn handle(&self, message: &Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let method = message.get("method").and_then(Value::as_str);
        // A notification carries no id and never receives a response.
        let id = id.filter(|value| !value.is_null())?;
        let Some(method) = method else {
            return Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32600, "message": "invalid request"},
            }));
        };
        match method {
            "initialize" => {
                let requested = message
                    .get("params")
                    .and_then(|params| params.get("protocolVersion"))
                    .and_then(Value::as_str)
                    .unwrap_or(DEFAULT_PROTOCOL_VERSION);
                let version = if PROTOCOL_VERSIONS.contains(&requested) {
                    requested
                } else {
                    DEFAULT_PROTOCOL_VERSION
                };
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": version,
                        "capabilities": {"tools": {}},
                        "serverInfo": {
                            "name": SERVER_NAME,
                            "version": env!("CARGO_PKG_VERSION"),
                        },
                    },
                }))
            }
            "ping" => Some(json!({"jsonrpc": "2.0", "id": id, "result": {}})),
            "tools/list" => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"tools": tool_definitions()},
            })),
            "tools/call" => {
                let params = message.get("params").cloned().unwrap_or(Value::Null);
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .filter(|value| value.is_object())
                    .unwrap_or_else(|| json!({}));
                let result = self.call_tool(&name, &arguments);
                Some(match result {
                    Ok(text) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {"content": [{"type": "text", "text": text}], "isError": false},
                    }),
                    Err(message) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {"content": [{"type": "text", "text": message}], "isError": true},
                    }),
                })
            }
            other => Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": format!("unknown method: {other}")},
            })),
        }
    }

    fn call_tool(&self, name: &str, arguments: &Value) -> Result<String, String> {
        match name {
            "project_status" => self.project_status(),
            "timeline_overview" => self.timeline_overview(),
            "media_list" => self.media_list(arguments),
            "save" => self.run_or(&["project", "save"]).and_then(pretty),
            "undo" => self.run_or(&["history", "undo"]).and_then(pretty),
            "redo" => self.run_or(&["history", "redo"]).and_then(pretty),
            "add_media" => self.add_media(arguments),
            "add_track" => self.add_track(arguments),
            "insert_clip" => self.insert_clip(arguments),
            "insert_text" => self.insert_text(arguments),
            "move_clip" => self.move_clip(arguments),
            "trim_clip" => self.trim_clip(arguments),
            "split_clip" => self.split_clip(arguments),
            "duck_music" => self.duck_music(arguments),
            "delete_clip" => self.delete_clip(arguments),
            "set_frame_rate" => self.set_frame_rate(arguments),
            "add_marker" => self.add_marker(arguments),
            "captions_transcribe" => self.captions_transcribe(arguments),
            "motion_insert_scene" => self.motion_insert_scene(arguments),
            "motion_asset_list" => self.motion_asset_list(arguments),
            "motion_asset_insert" => self.motion_asset_insert(arguments),
            "motion_asset_save" => self.motion_asset_save(arguments),
            other => Err(format!(
                "unknown tool: {other}. Call tools/list to see the supported tools."
            )),
        }
    }

    fn project_status(&self) -> Result<String, String> {
        let summary = self.run_or(&["project", "summary"])?;
        let mut value = summary;
        if let Some(object) = value.as_object_mut()
            && let Some(pointer) = self.pointer_state()
            && let Some(project_path) = pointer.get("project_path").and_then(Value::as_str)
        {
            object.insert(
                "project_path".to_owned(),
                Value::String(project_path.to_owned()),
            );
        }
        pretty(value)
    }

    fn timeline_overview(&self) -> Result<String, String> {
        let tracks = self.run_or(&["timeline", "tracks"])?;
        let track_items = tracks
            .get("timeline_tracks_v2")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut combined = Vec::with_capacity(track_items.len());
        for track in &track_items {
            let track_id = track
                .get("track_id")
                .and_then(Value::as_str)
                .ok_or_else(|| "the tracks query returned a track without an id".to_owned())?;
            let clips = self.run_or(&[
                "timeline",
                "clips",
                "--track",
                track_id,
                "--limit",
                &MAX_PAGE.to_string(),
            ])?;
            let mut entry = track.clone();
            if let Some(object) = entry.as_object_mut() {
                object.insert(
                    "clips".to_owned(),
                    clips
                        .get("timeline_clip_page_v2")
                        .and_then(|page| page.get("items"))
                        .cloned()
                        .unwrap_or_else(|| json!([])),
                );
            }
            combined.push(entry);
        }
        let settings = self.run_or(&["timeline", "settings"])?;
        let markers = self.run_or(&["timeline", "markers", "--limit", "100"])?;
        let summary = json!({
            "project_id": tracks.get("project_id").cloned().unwrap_or(Value::Null),
            "project_instance_id": tracks
                .get("project_instance_id")
                .cloned()
                .unwrap_or(Value::Null),
            "project_revision": tracks
                .get("project_revision")
                .cloned()
                .unwrap_or(Value::Null),
            "name": tracks.get("name").cloned().unwrap_or(Value::Null),
        });
        pretty(json!({
            "summary": summary,
            "frame_rate": settings
                .get("timeline_sequence_settings")
                .and_then(|settings| settings.get("sequence_frame_rate"))
                .cloned()
                .unwrap_or(Value::Null),
            "tracks": combined,
            "markers": markers
                .get("timeline_marker_page")
                .and_then(|page| page.get("items"))
                .cloned()
                .unwrap_or_else(|| json!([])),
        }))
    }

    fn media_list(&self, arguments: &Value) -> Result<String, String> {
        let offset = optional_u32(arguments, "offset")?.unwrap_or(0);
        let limit = optional_u32(arguments, "limit")?
            .unwrap_or(MAX_PAGE)
            .min(MAX_PAGE);
        let value = self.run_or(&[
            "media",
            "list",
            "--offset",
            &offset.to_string(),
            "--limit",
            &limit.to_string(),
        ])?;
        pretty(value)
    }

    fn add_media(&self, arguments: &Value) -> Result<String, String> {
        let source = absolute_path(required_str(arguments, "source_path")?)?;
        self.run_or(&["media", "add", "--source", &source])
            .and_then(pretty)
    }

    fn add_track(&self, arguments: &Value) -> Result<String, String> {
        let kind = required_str(arguments, "kind")?;
        if !matches!(kind.as_str(), "video" | "audio" | "text" | "caption") {
            return Err("kind must be video, audio, text or caption".to_owned());
        }
        let mut args = vec!["timeline", "add-track", "--kind", kind.as_str()];
        let track_id = optional_str(arguments, "track_id")?;
        if let Some(track_id) = &track_id {
            args.push("--id");
            args.push(track_id);
        }
        self.run_or(&args).and_then(pretty)
    }

    fn insert_clip(&self, arguments: &Value) -> Result<String, String> {
        let track_id = required_str(arguments, "track_id")?;
        let media_id = required_str(arguments, "media_id")?;
        let at = normalize_time(&required_str(arguments, "at")?)?;
        let source_start = match optional_str(arguments, "source_start")? {
            Some(value) => normalize_time(&value)?,
            None => "0/1".to_owned(),
        };
        let duration = match optional_str(arguments, "duration")? {
            Some(value) => normalize_time(&value)?,
            None => self.media_duration(&media_id)?,
        };
        let mode = optional_str(arguments, "mode")?.unwrap_or_else(|| "insert".to_owned());
        if !matches!(mode.as_str(), "insert" | "overwrite") {
            return Err("mode must be insert or overwrite".to_owned());
        }
        let mut args = vec![
            "timeline",
            "insert-clip",
            "--track",
            track_id.as_str(),
            "--media",
            media_id.as_str(),
            "--at",
            at.as_str(),
            "--source-start",
            source_start.as_str(),
            "--duration",
            duration.as_str(),
            "--mode",
            mode.as_str(),
        ];
        let clip_id = optional_str(arguments, "clip_id")?;
        if let Some(clip_id) = &clip_id {
            args.push("--id");
            args.push(clip_id);
        }
        self.run_or(&args).and_then(pretty)
    }

    fn insert_text(&self, arguments: &Value) -> Result<String, String> {
        let track_id = required_str(arguments, "track_id")?;
        let at = normalize_time(&required_str(arguments, "at")?)?;
        let duration = normalize_time(&required_str(arguments, "duration")?)?;
        let text = required_str(arguments, "text")?;
        let kind = optional_str(arguments, "kind")?.unwrap_or_else(|| "text".to_owned());
        let verb = match kind.as_str() {
            "text" => "insert-text",
            "caption" => "insert-caption",
            _ => return Err("kind must be text or caption".to_owned()),
        };
        let mut args = vec![
            "timeline",
            verb,
            "--track",
            track_id.as_str(),
            "--at",
            at.as_str(),
            "--duration",
            duration.as_str(),
            "--text",
            text.as_str(),
        ];
        let clip_id = optional_str(arguments, "clip_id")?;
        if let Some(clip_id) = &clip_id {
            args.push("--id");
            args.push(clip_id);
        }
        self.run_or(&args).and_then(pretty)
    }

    fn move_clip(&self, arguments: &Value) -> Result<String, String> {
        let clip_id = required_str(arguments, "clip_id")?;
        let track_id = required_str(arguments, "track_id")?;
        let at = normalize_time(&required_str(arguments, "at")?)?;
        self.run_or(&[
            "timeline",
            "move-clip",
            "--clip",
            &clip_id,
            "--track",
            &track_id,
            "--at",
            &at,
        ])
        .and_then(pretty)
    }

    fn trim_clip(&self, arguments: &Value) -> Result<String, String> {
        let clip_id = required_str(arguments, "clip_id")?;
        let edge = required_str(arguments, "edge")?;
        if !matches!(edge.as_str(), "start" | "end") {
            return Err("edge must be start or end".to_owned());
        }
        let at = normalize_time(&required_str(arguments, "at")?)?;
        self.run_or(&[
            "timeline",
            "trim-clip",
            "--clip",
            &clip_id,
            "--edge",
            &edge,
            "--to",
            &at,
        ])
        .and_then(pretty)
    }

    fn split_clip(&self, arguments: &Value) -> Result<String, String> {
        let clip_id = required_str(arguments, "clip_id")?;
        let at = normalize_time(&required_str(arguments, "at")?)?;
        let mut args = vec!["timeline", "split-clip", "--clip", &clip_id, "--at", &at];
        let new_clip_id = optional_str(arguments, "new_clip_id")?;
        if let Some(new_clip_id) = &new_clip_id {
            args.push("--id");
            args.push(new_clip_id);
        }
        self.run_or(&args).and_then(pretty)
    }

    fn duck_music(&self, arguments: &Value) -> Result<String, String> {
        let track_id = required_str(arguments, "track_id")?;
        let clip_id = required_str(arguments, "clip_id")?;
        let voice_track_id = required_str(arguments, "voice_track_id")?;
        let duck_db = arguments
            .get("duck_db")
            .and_then(Value::as_i64)
            .unwrap_or(-12);
        if !(-40..=-1).contains(&duck_db) {
            return Err("duck_db must be between -40 and -1".to_owned());
        }
        self.run_or(&[
            "timeline",
            "duck-clip",
            "--track",
            &track_id,
            "--clip",
            &clip_id,
            "--voice-track",
            &voice_track_id,
            "--duck-db",
            &duck_db.to_string(),
        ])
        .and_then(pretty)
    }

    fn delete_clip(&self, arguments: &Value) -> Result<String, String> {
        let clip_id = required_str(arguments, "clip_id")?;
        let ripple = optional_bool(arguments, "ripple")?.unwrap_or(false);
        let verb = if ripple {
            "ripple-delete-clip"
        } else {
            "delete-clip"
        };
        self.run_or(&["timeline", verb, "--clip", &clip_id])
            .and_then(pretty)
    }

    fn captions_transcribe(&self, arguments: &Value) -> Result<String, String> {
        let media_id = required_str(arguments, "media_id")?;
        self.run_or(&["captions", "transcribe", "--media", &media_id])
            .and_then(pretty)
    }

    fn set_frame_rate(&self, arguments: &Value) -> Result<String, String> {
        let rate = match optional_str(arguments, "rate")? {
            Some(rate) => rate,
            None => {
                let numerator = optional_u32(arguments, "numerator")?
                    .ok_or_else(|| "pass rate or numerator and denominator".to_owned())?;
                let denominator = optional_u32(arguments, "denominator")?.unwrap_or(1);
                format!("{numerator}/{denominator}")
            }
        };
        let normalized = normalize_rate(&rate)?;
        self.run_or(&["timeline", "set-frame-rate", "--rate", &normalized])
            .and_then(pretty)
    }

    fn add_marker(&self, arguments: &Value) -> Result<String, String> {
        let at = normalize_time(&required_str(arguments, "at")?)?;
        let label = required_str(arguments, "label")?;
        let mut args = vec!["timeline", "add-marker", "--at", &at, "--label", &label];
        let marker_id = optional_str(arguments, "marker_id")?;
        if let Some(marker_id) = &marker_id {
            args.push("--id");
            args.push(marker_id);
        }
        self.run_or(&args).and_then(pretty)
    }

    /// Materialises a code file or scene through the application runtime, then
    /// registers the result as ordinary media and inserts it on a video track.
    fn motion_insert_scene(&self, arguments: &Value) -> Result<String, String> {
        let source = optional_str(arguments, "source_path")?;
        let scene = optional_str(arguments, "scene_path")?;
        if source.is_some() == scene.is_some() {
            return Err("pass exactly one of source_path or scene_path".to_owned());
        }
        let title = optional_str(arguments, "title")?.unwrap_or_default();
        let width = optional_u32(arguments, "width")?.unwrap_or(1920);
        let height = optional_u32(arguments, "height")?.unwrap_or(1080);
        let rate = match optional_str(arguments, "frame_rate")? {
            Some(rate) => normalize_rate(&rate)?,
            None => "30/1".to_owned(),
        };
        let output = match optional_str(arguments, "output_path")? {
            Some(path) => PathBuf::from(absolute_path(path)?),
            None => self.default_motion_output(&title, source.as_deref(), scene.as_deref())?,
        };
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }
        let width_text = width.to_string();
        let height_text = height.to_string();
        let mut args = vec![
            "motion",
            "materialize",
            "--output",
            output
                .to_str()
                .ok_or_else(|| "the output path must be valid UTF-8".to_owned())?,
            "--frame-rate",
            rate.as_str(),
            "--width",
            width_text.as_str(),
            "--height",
            height_text.as_str(),
        ];
        let source_path = match source {
            Some(source) => Some(absolute_path(source)?),
            None => None,
        };
        let scene_path = match scene {
            Some(scene) => Some(absolute_path(scene)?),
            None => None,
        };
        if let Some(source_path) = &source_path {
            args.push("--source");
            args.push(source_path);
        }
        if let Some(scene_path) = &scene_path {
            args.push("--scene");
            args.push(scene_path);
        }
        if !title.is_empty() {
            args.push("--title");
            args.push(&title);
        }
        let materialized = self.run_or(&args)?;
        let facts = materialized
            .get("motion_materialization")
            .cloned()
            .ok_or_else(|| "the motion materialise response had no facts".to_owned())?;
        if !facts
            .get("succeeded")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let code = facts
                .get("error_code")
                .and_then(Value::as_str)
                .unwrap_or("MOTION_FAILED");
            let message = facts
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("the scene could not be materialised");
            return Err(format!("{code}: {message}"));
        }
        let output_path = facts
            .get("output_path")
            .and_then(Value::as_str)
            .ok_or_else(|| "the materialise response had no output path".to_owned())?;
        let duration = facts
            .get("duration")
            .cloned()
            .map(|duration| {
                let numerator = duration
                    .get("numerator")
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                let denominator = duration
                    .get("denominator")
                    .and_then(Value::as_u64)
                    .unwrap_or(1);
                format!("{numerator}/{denominator}")
            })
            .unwrap_or_else(|| "0/1".to_owned());

        let added = self.run_or(&["media", "add", "--source", output_path])?;
        let media_id = added
            .get("media")
            .and_then(|media| media.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| "the media add response had no media id".to_owned())?
            .to_owned();

        let (track_id, at) = match optional_str(arguments, "track_id")? {
            Some(track_id) => {
                let at = match optional_str(arguments, "at")? {
                    Some(at) => normalize_time(&at)?,
                    None => self.track_end(&track_id)?,
                };
                (track_id, at)
            }
            None => {
                let track_id = self.first_video_track()?;
                let at = match optional_str(arguments, "at")? {
                    Some(at) => normalize_time(&at)?,
                    None => self.track_end(&track_id)?,
                };
                (track_id, at)
            }
        };
        let inserted = self.run_or(&[
            "timeline",
            "insert-clip",
            "--track",
            &track_id,
            "--media",
            &media_id,
            "--at",
            &at,
            "--source-start",
            "0/1",
            "--duration",
            &duration,
        ])?;
        pretty(json!({
            "output_path": output_path,
            "media_id": media_id,
            "track_id": track_id,
            "at": at,
            "duration": duration,
            "scene_name": facts.get("scene_name").cloned().unwrap_or(Value::Null),
            "frame_count": facts.get("frame_count").cloned().unwrap_or(Value::Null),
            "insert": inserted,
            "next": "The clip is on the timeline but the project is not saved yet. Call save when the edit is complete.",
        }))
    }

    /// The library path the application records in its session pointer.
    fn motion_library_root(&self) -> Result<PathBuf, String> {
        let pointer = self
            .pointer_state()
            .ok_or_else(|| "no session pointer is available".to_owned())?;
        pointer
            .get("motion_library_path")
            .and_then(Value::as_str)
            .filter(|path| !path.trim().is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| {
                "the application did not record a motion library path; update the app or pass one through the workspace".to_owned()
            })
    }

    fn motion_asset_list(&self, arguments: &Value) -> Result<String, String> {
        let root = self.motion_library_root()?;
        let library = or_motion::MotionLibrary::open(&root)
            .map_err(|error| format!("{}: {error}", error.code()))?;
        let entries = library
            .list()
            .map_err(|error| format!("{}: {error}", error.code()))?;
        let offset = optional_u32(arguments, "offset")?.unwrap_or(0) as usize;
        let limit = optional_u32(arguments, "limit")?.unwrap_or(50).min(100) as usize;
        let page: Vec<&or_motion::MotionLibraryEntry> =
            entries.iter().skip(offset).take(limit).collect();
        let has_more = offset.saturating_add(page.len()) < entries.len();
        pretty(json!({
            "root": library.root().display().to_string(),
            "total": entries.len(),
            "offset": offset,
            "has_more": has_more,
            "entries": page,
        }))
    }

    /// Instantiates a library asset, prepares its image files beside a scratch
    /// scene, and reuses the ordinary materialise/import/insert path.
    fn motion_asset_insert(&self, arguments: &Value) -> Result<String, String> {
        let root = self.motion_library_root()?;
        let library = or_motion::MotionLibrary::open(&root)
            .map_err(|error| format!("{}: {error}", error.code()))?;
        let asset_id = required_str(arguments, "asset_id")?;
        let asset = library
            .load(&asset_id)
            .map_err(|error| format!("{}: {error}", error.code()))?;
        let values = match arguments.get("values") {
            None | Some(Value::Null) => BTreeMap::new(),
            Some(Value::Object(map)) => serde_json::from_value::<
                BTreeMap<String, or_motion::MotionParameterValue>,
            >(Value::Object(map.clone()))
            .map_err(|error| format!("values must be parameter values by id: {error}"))?,
            Some(_) => return Err("values must be an object keyed by parameter id".to_owned()),
        };
        let scene_id = or_motion::new_motion_asset_id();
        let scene = or_motion::instantiate_motion_asset(&asset, &values, &scene_id, &asset.name)
            .map_err(|error| format!("{}: {error}", error.code()))?;
        let scratch = std::env::temp_dir().join(format!("or-mcp-motion-asset-{scene_id}"));
        let prepared = library
            .prepare_scene_assets(&scene, &scratch)
            .map_err(|error| format!("{}: {error}", error.code()))?;
        let scene_path = scratch.join("scene.ormotion.json");
        let encoded = serde_json::to_vec_pretty(&prepared)
            .map_err(|error| format!("the composed scene could not be encoded: {error}"))?;
        std::fs::write(&scene_path, encoded)
            .map_err(|error| format!("the composed scene could not be written: {error}"))?;

        let width = optional_u32(arguments, "width")?.unwrap_or(asset.canvas.width);
        let height = optional_u32(arguments, "height")?.unwrap_or(asset.canvas.height);
        let mut insert = json!({
            "scene_path": scene_path.display().to_string(),
            "title": asset.name,
            "width": width,
            "height": height,
        });
        for key in ["at", "track_id", "frame_rate", "output_path"] {
            if let Some(value) = arguments.get(key)
                && !value.is_null()
            {
                insert[key] = value.clone();
            }
        }
        let result = self.motion_insert_scene(&insert);
        let _ = std::fs::remove_dir_all(&scratch);
        let text = result?;
        let mut value: Value = serde_json::from_str(&text)
            .map_err(|error| format!("the insert response was not JSON: {error}"))?;
        if let Some(object) = value.as_object_mut() {
            object.insert("asset_id".to_owned(), json!(asset.asset_id));
            object.insert("asset_name".to_owned(), json!(asset.name));
        }
        if let Some(output) = value.get("output_path").and_then(Value::as_str) {
            let provenance = json!({
                "version": 1,
                "kind": "motion-asset",
                "asset_id": asset.asset_id,
                "asset_name": asset.name,
                "values": arguments.get("values").cloned().unwrap_or_else(|| json!({})),
                "library": library.root().display().to_string(),
                "scene_id": scene_id,
            });
            if let Ok(encoded) = serde_json::to_vec_pretty(&provenance) {
                let _ = std::fs::write(format!("{output}.provenance.json"), encoded);
            }
        }
        pretty(value)
    }

    /// Saves an authored scene file as a library scene asset.
    fn motion_asset_save(&self, arguments: &Value) -> Result<String, String> {
        let root = self.motion_library_root()?;
        let library = or_motion::MotionLibrary::open(&root)
            .map_err(|error| format!("{}: {error}", error.code()))?;
        let scene_path = required_str(arguments, "scene_path")?;
        let bytes = std::fs::read(&scene_path)
            .map_err(|error| format!("the scene could not be read: {error}"))?;
        let scene = or_motion::parse_motion_scene(&bytes)
            .map_err(|error| format!("{}: {error}", error.code()))?;
        let validation = or_motion::validate_scene(&scene);
        if !validation.ok {
            return Err(format!(
                "the scene does not validate: {}",
                validation
                    .errors
                    .iter()
                    .map(|error| error.code.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !scene.assets.is_empty() {
            return Err(
                "a scene with external image assets cannot be saved through MCP yet; save it from the app"
                    .to_owned(),
            );
        }
        let name = optional_str(arguments, "name")?.unwrap_or_else(|| scene.name.clone());
        let category = match optional_str(arguments, "category")? {
            Some(value) => Some(
                or_motion::MotionAssetCategory::parse(value.trim())
                    .ok_or_else(|| format!("unknown category: {value}"))?,
            ),
            None => None,
        };
        let tags: Vec<String> = arguments
            .get("tags")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let asset_id = or_motion::new_motion_asset_id();
        let preview = (|| {
            let frame = or_motion::evaluate_scene(&scene, RationalTime::ZERO)
                .map_err(|error| format!("{}: the preview frame did not evaluate", error.code()))?;
            let canvas = or_render::rasterize_motion_frame(&frame)
                .map_err(|error| format!("{}: the preview did not rasterise", error.code()))?;
            let png = or_render::encode_png(canvas.width(), canvas.height(), canvas.pixels())
                .map_err(|error| format!("{}: the preview did not encode", error.code()))?;
            library
                .write_preview(&asset_id, &png)
                .map_err(|error| format!("{}: the preview was not stored", error.code()))
        })()
        .ok();
        let classification = or_motion::classify_motion_asset(
            &name,
            &tags,
            or_motion::MotionAssetKind::Scene,
            category,
        );
        let document = or_motion::MotionAssetDocument {
            format: or_motion::MOTION_ASSET_FORMAT.to_owned(),
            schema_version: or_motion::MOTION_ASSET_SCHEMA_VERSION,
            asset_id,
            name: name.clone(),
            category: classification.category,
            category_source: classification.source,
            tags,
            kind: or_motion::MotionAssetKind::Scene,
            canvas: scene.canvas,
            duration: scene.duration,
            parameters: Vec::new(),
            scene: Some(or_motion::MotionAssetPayload {
                assets: Vec::new(),
                nodes: scene.nodes.clone(),
                animations: scene.animations.clone(),
            }),
            preset: None,
            image: None,
            preview,
        };
        let entry = library
            .save(&document, category)
            .map_err(|error| format!("{}: {error}", error.code()))?;
        pretty(json!({
            "library": library.root().display().to_string(),
            "motion_asset": entry,
        }))
    }

    fn first_video_track(&self) -> Result<String, String> {
        let tracks = self.run_or(&["timeline", "tracks"])?;
        let items = tracks
            .get("timeline_tracks_v2")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if let Some(track_id) = items.iter().find_map(|track| {
            (track.get("kind").and_then(Value::as_str) == Some("video"))
                .then(|| track.get("track_id").and_then(Value::as_str))
                .flatten()
        }) {
            return Ok(track_id.to_owned());
        }
        let added = self.run_or(&["timeline", "add-track", "--kind", "video"])?;
        added
            .get("track_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| "could not create a video track".to_owned())
    }

    /// The end of the last clip on a track, used when the caller gave no time.
    fn track_end(&self, track_id: &str) -> Result<String, String> {
        let clips = self.run_or(&[
            "timeline",
            "clips",
            "--track",
            track_id,
            "--limit",
            &MAX_PAGE.to_string(),
        ])?;
        let mut end = RationalTime::ZERO;
        if let Some(items) = clips
            .get("timeline_clip_page_v2")
            .and_then(|page| page.get("items"))
            .and_then(Value::as_array)
        {
            for clip in items {
                let start = rational_from_value(clip.get("timeline_start"))?;
                let duration = rational_from_value(clip.get("timeline_duration"))?;
                let clip_end = start
                    .checked_add(duration)
                    .map_err(|_| "a clip end is outside the supported range".to_owned())?;
                if clip_end > end {
                    end = clip_end;
                }
            }
        }
        Ok(format!("{}/{}", end.numerator(), end.denominator()))
    }

    fn media_duration(&self, media_id: &str) -> Result<String, String> {
        let page = self.run_or(&["media", "list", "--limit", "100"])?;
        let items = page
            .get("media_page")
            .and_then(|page| page.get("items"))
            .and_then(Value::as_array)
            .ok_or_else(|| "the media list response had no items".to_owned())?;
        let item = items
            .iter()
            .find(|item| item.get("id").and_then(Value::as_str) == Some(media_id))
            .ok_or_else(|| format!("media {media_id} is not in this project"))?;
        let duration = item
            .get("metadata")
            .and_then(|metadata| metadata.get("duration"))
            .ok_or_else(|| {
                format!(
                    "media {media_id} has no known duration; pass duration explicitly as seconds or NUM/DEN"
                )
            })?;
        let value = rational_from_value(Some(duration))?;
        Ok(format!("{}/{}", value.numerator(), value.denominator()))
    }

    /// Names a persistent destination beside the project file.
    fn default_motion_output(
        &self,
        title: &str,
        source: Option<&str>,
        scene: Option<&str>,
    ) -> Result<PathBuf, String> {
        let pointer = self
            .pointer_state()
            .ok_or_else(|| "pass output_path: no session pointer is available".to_owned())?;
        let project_path = pointer
            .get("project_path")
            .and_then(Value::as_str)
            .filter(|path| !path.is_empty())
            .ok_or_else(|| {
                "pass output_path: the assistant could not locate the project file".to_owned()
            })?;
        let directory = Path::new(project_path)
            .parent()
            .ok_or_else(|| "pass output_path: the project file has no directory".to_owned())?;
        let basis = if title.trim().is_empty() {
            source.or(scene).unwrap_or("motion")
        } else {
            title
        };
        let mut slug = String::new();
        for character in basis.chars() {
            if character.is_ascii_alphanumeric() {
                slug.push(character.to_ascii_lowercase());
            } else if character == ' ' || character == '-' || character == '_' {
                slug.push('-');
            }
            if slug.len() >= 48 {
                break;
            }
        }
        let slug = slug.trim_matches('-');
        let slug = if slug.is_empty() { "motion" } else { slug };
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        Ok(directory
            .join("generated")
            .join(format!("{slug}-{stamp}.mkv")))
    }

    fn pointer_state(&self) -> Option<Value> {
        let path = self.pointer.as_ref()?;
        let text = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn descriptor(&self) -> Result<PathBuf, String> {
        if let Some(descriptor) = &self.descriptor {
            return Ok(descriptor.clone());
        }
        let pointer = self
            .pointer
            .as_ref()
            .ok_or_else(|| "no descriptor is configured".to_owned())?;
        let text = std::fs::read_to_string(pointer).map_err(|error| {
            format!(
                "No Opencut Reinforced project is open (could not read {}: {error}).",
                pointer.display()
            )
        })?;
        let value: Value = serde_json::from_str(&text)
            .map_err(|error| format!("the session pointer is invalid: {error}"))?;
        let descriptor = value
            .get("descriptor_path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if descriptor.is_empty() {
            return Err("No Opencut Reinforced project is open.".to_owned());
        }
        Ok(PathBuf::from(descriptor))
    }

    /// Runs one `or` verb against the live host and returns its JSON result.
    fn run_or(&self, args: &[&str]) -> Result<Value, String> {
        let descriptor = self.descriptor()?;
        if !descriptor.exists() {
            return Err(
                "The Opencut Reinforced session is not running. Reopen the project and try again."
                    .to_owned(),
            );
        }
        let executable = std::env::current_exe()
            .map_err(|error| format!("could not locate the or executable: {error}"))?;
        let output = Command::new(executable)
            .args(args)
            .arg("--attach")
            .arg(&descriptor)
            .arg("--json")
            .output()
            .map_err(|error| format!("could not run the or CLI: {error}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            if let Ok(value) = serde_json::from_str::<Value>(&stdout)
                && let Some(error) = value.get("error")
            {
                let code = error
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("INVALID_ARGUMENTS");
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("the edit was rejected");
                return Err(format!("{code}: {message}"));
            }
            let message = if stderr.trim().is_empty() {
                stdout.trim().to_owned()
            } else {
                stderr.trim().to_owned()
            };
            return Err(message);
        }
        serde_json::from_str(stdout.trim()).map_err(|error| {
            format!(
                "the or CLI returned invalid JSON: {error}: {}",
                stdout.trim()
            )
        })
    }
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "project_status",
            "description": "Read the open project's identity: name, revision, dirty state and project file path. Call this first when you need to orient yourself.",
            "inputSchema": empty_schema(),
        },
        {
            "name": "timeline_overview",
            "description": "Read the whole timeline: sequence frame rate, tracks with their clips, and markers. Times are exact rational seconds. Use this before editing so track_id, clip_id, media_id and durations are real values.",
            "inputSchema": empty_schema(),
        },
        {
            "name": "media_list",
            "description": "List the project's media library with ids, source paths and metadata (including duration). Import files with add_media before inserting them. Use offset when a page reports more items.",
            "inputSchema": object_schema(json!({
                "offset": {"type": "integer", "minimum": 0},
                "limit": {"type": "integer", "minimum": 1, "maximum": 100}
            }), &[]),
        },
        {
            "name": "save",
            "description": "Save the live project to its file. Edits are live but unsaved until this is called; call it once after finishing a requested change unless the user asked to review first.",
            "inputSchema": empty_schema(),
        },
        {
            "name": "undo",
            "description": "Undo the most recent project edit.",
            "inputSchema": empty_schema(),
        },
        {
            "name": "redo",
            "description": "Redo the most recently undone project edit.",
            "inputSchema": empty_schema(),
        },
        {
            "name": "add_media",
            "description": "Import a local audio, video or image file into the project media library. Returns the new media id.",
            "inputSchema": object_schema(json!({
                "source_path": {"type": "string", "description": "Absolute path to the media file."}
            }), &["source_path"]),
        },
        {
            "name": "add_track",
            "description": "Add an empty timeline track. Video/text/caption tracks carry visual clips; audio tracks carry audio clips.",
            "inputSchema": object_schema(json!({
                "kind": {"type": "string", "enum": ["video", "audio", "text", "caption"]},
                "track_id": {"type": "string", "description": "Optional canonical UUIDv4 for the new track."}
            }), &["kind"]),
        },
        {
            "name": "insert_clip",
            "description": "Place an imported media item on a track. `at` is the timeline position; source_start and duration address the media. When duration is omitted the media's full duration is used. Default mode is insert (existing clips move right); overwrite clears the range.",
            "inputSchema": object_schema(json!({
                "track_id": {"type": "string"},
                "media_id": {"type": "string"},
                "at": {"type": "string", "description": "Timeline position as exact NUM/DEN seconds or decimal seconds, e.g. 2.5."},
                "source_start": {"type": "string", "description": "Optional media in-point, same time form. Defaults to 0."},
                "duration": {"type": "string", "description": "Optional clip duration, same time form. Defaults to the media's full duration."},
                "mode": {"type": "string", "enum": ["insert", "overwrite"]},
                "clip_id": {"type": "string", "description": "Optional canonical UUIDv4 for the new clip."}
            }), &["track_id", "media_id", "at"]),
        },
        {
            "name": "insert_text",
            "description": "Add a text or caption clip with plain text. The track must be a text or caption track.",
            "inputSchema": object_schema(json!({
                "track_id": {"type": "string"},
                "text": {"type": "string"},
                "at": {"type": "string"},
                "duration": {"type": "string"},
                "kind": {"type": "string", "enum": ["text", "caption"]},
                "clip_id": {"type": "string"}
            }), &["track_id", "text", "at", "duration"]),
        },
        {
            "name": "move_clip",
            "description": "Move a clip to a new timeline position, optionally on another track.",
            "inputSchema": object_schema(json!({
                "clip_id": {"type": "string"},
                "track_id": {"type": "string", "description": "Target track; pass the current track to move in time only."},
                "at": {"type": "string"}
            }), &["clip_id", "track_id", "at"]),
        },
        {
            "name": "trim_clip",
            "description": "Trim a clip's start or end edge to an exact timeline time.",
            "inputSchema": object_schema(json!({
                "clip_id": {"type": "string"},
                "edge": {"type": "string", "enum": ["start", "end"]},
                "at": {"type": "string"}
            }), &["clip_id", "edge", "at"]),
        },
        {
            "name": "split_clip",
            "description": "Split a clip at a timeline time, producing a second clip that starts there.",
            "inputSchema": object_schema(json!({
                "clip_id": {"type": "string"},
                "at": {"type": "string"},
                "new_clip_id": {"type": "string", "description": "Optional canonical UUIDv4 for the right-hand clip."}
            }), &["clip_id", "at"]),
        },
        {
            "name": "delete_clip",
            "description": "Delete a clip. ripple=true closes the gap on that track.",
            "inputSchema": object_schema(json!({
                "clip_id": {"type": "string"},
                "ripple": {"type": "boolean"}
            }), &["clip_id"]),
        },
        {
            "name": "set_frame_rate",
            "description": "Set the sequence frame rate from an exact NUM/DEN rate or numerator/denominator.",
            "inputSchema": object_schema(json!({
                "rate": {"type": "string", "description": "e.g. 30/1 or 30000/1001"},
                "numerator": {"type": "integer"},
                "denominator": {"type": "integer"}
            }), &[]),
        },
        {
            "name": "add_marker",
            "description": "Add a labelled marker at a timeline time.",
            "inputSchema": object_schema(json!({
                "at": {"type": "string"},
                "label": {"type": "string"},
                "marker_id": {"type": "string"}
            }), &["at", "label"]),
        },
        {
            "name": "duck_music",
            "description": "Lower one music clip wherever clips on a chosen voice track overlap it, as one undoable command. The music clip is split at each voice boundary and the overlapping segments get the duck gain; nothing else on the track changes. Use media/timeline reads first to obtain the track and clip ids.",
            "inputSchema": object_schema(json!({
                "track_id": {"type": "string", "description": "The audio track that holds the music clip."},
                "clip_id": {"type": "string", "description": "The music clip id on that track."},
                "voice_track_id": {"type": "string", "description": "The audio track whose clips define where to duck."},
                "duck_db": {"type": "integer", "minimum": -40, "maximum": -1, "description": "Duck level; defaults to -12 dB."}
            }), &["track_id", "clip_id", "voice_track_id"]),
        },
        {
            "name": "captions_transcribe",
            "description": "Transcribe one media item's audio locally with the installed multilingual model and return bounded cues with millisecond timing. Language is detected automatically (Vietnamese and English are the supported targets). WAIT: local transcription can take a while; poll-free, this call returns only when the job finishes. This never changes the project: review the cues, then apply them with the `or timeline import-captions --file` CLI verb or edit them by hand.",
            "inputSchema": object_schema(json!({
                "media_id": {"type": "string", "description": "The media id from media_list."}
            }), &["media_id"]),
        },
        {
            "name": "motion_insert_scene",
            "description": "Turn a code file (or an authored .ormotion.json scene) into a rendered video through the app's MotionScene materialiser, import it, and insert it on a video track as an ordinary editable clip. This is how you create an illustration/motion graphic from code. WAIT: materialisation renders every frame and can take a while. When `at` is omitted the clip is appended after the track's last clip; when track_id is omitted the first video track is used (created if none exists).",
            "inputSchema": object_schema(json!({
                "source_path": {"type": "string", "description": "Absolute path to the code file to illustrate."},
                "scene_path": {"type": "string", "description": "Absolute path to an existing .ormotion.json scene; pass instead of source_path."},
                "title": {"type": "string"},
                "at": {"type": "string", "description": "Optional timeline position; defaults to the end of the target track."},
                "track_id": {"type": "string"},
                "width": {"type": "integer"},
                "height": {"type": "integer"},
                "frame_rate": {"type": "string", "description": "e.g. 30/1"},
                "output_path": {"type": "string", "description": "Optional absolute .mkv destination; defaults beside the project file."}
            }), &[]),
        },
        {
            "name": "motion_asset_list",
            "description": "List the reusable motion assets in the application's motion library: id, name, category, kind, canvas and duration. Use this before inserting one so you can choose a real asset id and see its category.",
            "inputSchema": object_schema(json!({
                "offset": {"type": "integer", "minimum": 0},
                "limit": {"type": "integer", "minimum": 1, "maximum": 100}
            }), &[]),
        },
        {
            "name": "motion_asset_insert",
            "description": "Instantiate a reusable motion asset with optional parameter values, render it through the application's MotionScene materialiser, import it, and insert an ordinary editable clip. WAIT: materialisation renders every frame. When `at` is omitted the clip is appended after the track's last clip; when track_id is omitted the first video track is used.",
            "inputSchema": object_schema(json!({
                "asset_id": {"type": "string", "description": "The asset id from motion_asset_list."},
                "values": {"type": "object", "description": "Optional parameter values keyed by parameter id; text and choice are strings, number is a number, colour is {red,green,blue,alpha}."},
                "at": {"type": "string", "description": "Optional timeline position; defaults to the end of the target track."},
                "track_id": {"type": "string"},
                "width": {"type": "integer"},
                "height": {"type": "integer"},
                "frame_rate": {"type": "string", "description": "e.g. 30/1"},
                "output_path": {"type": "string", "description": "Optional absolute .mkv destination; defaults beside the project file."}
            }), &["asset_id"]),
        },
        {
            "name": "motion_asset_save",
            "description": "Save an authored .ormotion.json scene as a reusable, classified library asset with a generated preview. The scene must not reference external image assets yet; save those from the app. Returns the catalogue entry.",
            "inputSchema": object_schema(json!({
                "scene_path": {"type": "string", "description": "Absolute path to a valid .ormotion.json scene."},
                "name": {"type": "string", "description": "Asset name; defaults to the scene name."},
                "category": {"type": "string", "description": "Optional manual category such as background, people, nature, animal, vehicle, object, text, shape, icon, chart, effect or motion."},
                "tags": {"type": "array", "items": {"type": "string"}}
            }), &["scene_path"]),
        }
    ])
}

fn empty_schema() -> Value {
    object_schema(json!({}), &[])
}

fn object_schema(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

fn required_str(arguments: &Value, key: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{key} is required"))
}

fn optional_str(arguments: &Value, key: &str) -> Result<Option<String>, String> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if value.trim().is_empty() => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("{key} must be a string")),
    }
}

fn optional_u32(arguments: &Value, key: &str) -> Result<Option<u32>, String> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0)
            .map(Some)
            .ok_or_else(|| format!("{key} must be a positive integer")),
    }
}

fn optional_bool(arguments: &Value, key: &str) -> Result<Option<bool>, String> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("{key} must be true or false")),
    }
}

/// Accepts exact `NUM/DEN` seconds or decimal seconds and returns `NUM/DEN`.
fn normalize_time(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.contains('/') {
        let (numerator, denominator) = trimmed
            .split_once('/')
            .ok_or_else(|| format!("{value} is not an exact time"))?;
        let numerator: i64 = numerator
            .parse()
            .map_err(|_| format!("{value} is not an exact time"))?;
        let denominator: u32 = denominator
            .parse()
            .map_err(|_| format!("{value} is not an exact time"))?;
        let time = RationalTime::new(numerator, denominator)
            .map_err(|_| format!("{value} is not an exact time"))?;
        return Ok(format!("{}/{}", time.numerator(), time.denominator()));
    }
    let seconds = trimmed.strip_suffix('s').unwrap_or(trimmed);
    let seconds: f64 = seconds
        .parse()
        .map_err(|_| format!("{value} is not a time; use NUM/DEN or decimal seconds"))?;
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(format!("{value} is not a nonnegative time"));
    }
    let millis = (seconds * 1000.0).round();
    if millis > i64::MAX as f64 {
        return Err(format!("{value} is too large"));
    }
    let time = RationalTime::new(millis as i64, 1000)
        .map_err(|_| format!("{value} is not an exact time"))?;
    Ok(format!("{}/{}", time.numerator(), time.denominator()))
}

fn normalize_rate(value: &str) -> Result<String, String> {
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or_else(|| format!("{value} is not an exact rate; use NUM/DEN"))?;
    let numerator: u32 = numerator
        .parse()
        .map_err(|_| format!("{value} is not an exact rate"))?;
    let denominator: u32 = denominator
        .parse()
        .map_err(|_| format!("{value} is not an exact rate"))?;
    if numerator == 0 || denominator == 0 {
        return Err(format!("{value} is not a positive rate"));
    }
    Ok(format!("{numerator}/{denominator}"))
}

fn rational_from_value(value: Option<&Value>) -> Result<RationalTime, String> {
    let value = value.ok_or_else(|| "a time value is missing".to_owned())?;
    let numerator = value
        .get("numerator")
        .and_then(Value::as_i64)
        .ok_or_else(|| "a time numerator is missing".to_owned())?;
    let denominator = value
        .get("denominator")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| "a time denominator is missing".to_owned())?;
    RationalTime::new(numerator, denominator).map_err(|_| "a time value is invalid".to_owned())
}

fn absolute_path(path: String) -> Result<String, String> {
    let path = Path::new(&path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("could not resolve the working directory: {error}"))?
            .join(path)
    };
    Ok(absolute.display().to_string())
}

fn pretty(value: Value) -> Result<String, String> {
    serde_json::to_string_pretty(&value)
        .map_err(|error| format!("could not encode the tool result: {error}"))
}
