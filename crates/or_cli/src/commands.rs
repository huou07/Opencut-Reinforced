use or_core::{
    ApplicationRequest, ApplicationResponse, CommandEnvelope, OperationError, ProjectFileSession,
    ProjectFileSessionError, QueryEnvelope, RecoveryApplyOutcome, RecoveryConflictReason,
    RecoveryInspection, apply_project_recovery, command_catalog, discard_project_recovery,
    inspect_project_recovery, query_catalog,
};
use or_ipc::{
    ApplicationSuccess, DescribeResponse, IpcErrorCode, IpcProtocolError, LocalIpcClient,
    LocalIpcServer,
};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

pub(super) enum CommandOutput {
    Immediate(String),
    Serve {
        server: LocalIpcServer,
        startup: String,
        json: bool,
    },
}

pub(super) struct CliError {
    exit_code: u8,
    json: bool,
    category: &'static str,
    code: String,
    message: String,
    details: Option<Value>,
}

impl CliError {
    pub(super) const fn exit_code(&self) -> u8 {
        self.exit_code
    }

    pub(super) const fn json(&self) -> bool {
        self.json
    }

    pub(super) fn render(&self) -> String {
        if self.json {
            let mut error = serde_json::Map::new();
            error.insert(
                "category".to_owned(),
                Value::String(self.category.to_owned()),
            );
            error.insert("code".to_owned(), Value::String(self.code.clone()));
            error.insert("message".to_owned(), Value::String(self.message.clone()));
            if let Some(details) = &self.details {
                error.insert("details".to_owned(), details.clone());
            }
            serde_json::to_string_pretty(&json!({"error": error}))
                .expect("JSON error values are serializable")
        } else {
            self.message.clone()
        }
    }

    fn usage(json: bool, message: impl Into<String>) -> Self {
        Self::new(2, json, "usage", "INVALID_ARGUMENTS", message)
    }

    fn operation(error: OperationError, json: bool) -> Self {
        let details = serde_json::to_value(&error).ok();
        let code = details
            .as_ref()
            .and_then(|value| value.get("code"))
            .and_then(Value::as_str)
            .unwrap_or("APPLICATION_ERROR")
            .to_owned();
        Self {
            exit_code: 3,
            json,
            category: "application",
            code,
            message: error.to_string(),
            details,
        }
    }

    fn file(error: ProjectFileSessionError, json: bool) -> Self {
        Self::new(
            4,
            json,
            "project_file",
            error.code().as_str(),
            error.to_string(),
        )
    }

    fn media(error: or_core::MediaProbeError, json: bool) -> Self {
        let diagnostic = error.diagnostic().map(str::to_owned);
        let message = match (&diagnostic, json) {
            (Some(diagnostic), false) => format!("{}: {diagnostic}", error.message()),
            _ => error.message().to_owned(),
        };
        Self {
            exit_code: 4,
            json,
            category: "media_probe",
            code: error.code_str().to_owned(),
            message,
            details: diagnostic.map(|diagnostic| json!({"diagnostic": diagnostic})),
        }
    }

    fn recovery(error: or_core::ProjectRecoveryError, json: bool) -> Self {
        let code = match &error {
            or_core::ProjectRecoveryError::Io(_) => "IO_ERROR",
            or_core::ProjectRecoveryError::FileTooLarge { .. } => "FILE_TOO_LARGE",
            or_core::ProjectRecoveryError::ProjectSnapshotTooLarge { .. } => {
                "PROJECT_SNAPSHOT_TOO_LARGE"
            }
            or_core::ProjectRecoveryError::InvalidUtf8 => "INVALID_UTF8",
            or_core::ProjectRecoveryError::InvalidEnvelope => "INVALID_ENVELOPE",
            or_core::ProjectRecoveryError::WrongFormatMarker => "WRONG_FORMAT_MARKER",
            or_core::ProjectRecoveryError::UnsupportedSchemaVersion(_) => {
                "UNSUPPORTED_SCHEMA_VERSION"
            }
            or_core::ProjectRecoveryError::InvalidBaseProject(_) => "INVALID_BASE_PROJECT",
            or_core::ProjectRecoveryError::InvalidRecoveryProject(_) => "INVALID_RECOVERY_PROJECT",
            or_core::ProjectRecoveryError::ProjectIdMismatch => "PROJECT_ID_MISMATCH",
            or_core::ProjectRecoveryError::InvalidRevisionOrdering => "INVALID_REVISION_ORDERING",
            or_core::ProjectRecoveryError::DiskBaseMismatch => "DISK_BASE_MISMATCH",
            or_core::ProjectRecoveryError::EncodingFailure => "ENCODING_FAILURE",
            or_core::ProjectRecoveryError::CheckpointWrite(_) => "CHECKPOINT_WRITE_FAILED",
            or_core::ProjectRecoveryError::CanonicalProject(_) => "CANONICAL_PROJECT_FAILED",
            or_core::ProjectRecoveryError::NoCheckpoint => "NO_CHECKPOINT",
            or_core::ProjectRecoveryError::StaleCheckpoint => "STALE_CHECKPOINT",
            or_core::ProjectRecoveryError::RecoveryConflict => "RECOVERY_CONFLICT",
            or_core::ProjectRecoveryError::ApplySave(_) => "APPLY_SAVE_FAILED",
        };
        Self::new(4, json, "recovery", code, error.to_string())
    }

    pub(super) fn ipc(error: IpcProtocolError, json: bool) -> Self {
        match error {
            IpcProtocolError::Application(error) => Self::operation(error, json),
            IpcProtocolError::Remote(IpcErrorCode::ProjectFileChanged) => Self::new(
                4,
                json,
                "project_file",
                "PROJECT_FILE_CHANGED",
                "the project file no longer matches the session's saved base",
            ),
            IpcProtocolError::Remote(IpcErrorCode::RecoveryRequired) => Self::new(
                4,
                json,
                "recovery",
                "RECOVERY_REQUIRED",
                "a recovery checkpoint needs explicit attention",
            ),
            IpcProtocolError::Remote(code) => Self::new(
                5,
                json,
                "ipc",
                serialized_code(serde_json::to_value(code).unwrap_or(Value::Null)),
                error_message_for_remote(code),
            ),
            error => {
                let code = match &error {
                    IpcProtocolError::Io(_) => "IO_ERROR",
                    IpcProtocolError::InvalidFrame => "INVALID_FRAME",
                    IpcProtocolError::FrameTooLarge => "FRAME_TOO_LARGE",
                    IpcProtocolError::InvalidUtf8 => "INVALID_UTF8",
                    IpcProtocolError::InvalidJson => "INVALID_JSON",
                    IpcProtocolError::InvalidRequest => "INVALID_REQUEST",
                    IpcProtocolError::InvalidDescriptor => "INVALID_DESCRIPTOR",
                    IpcProtocolError::UnsupportedProtocol => "UNSUPPORTED_PROTOCOL",
                    IpcProtocolError::UnsupportedPlatform => "UNSUPPORTED_PLATFORM",
                    IpcProtocolError::EndpointUnavailable => "ENDPOINT_UNAVAILABLE",
                    IpcProtocolError::ResponseMismatch => "RESPONSE_MISMATCH",
                    IpcProtocolError::Remote(_) | IpcProtocolError::Application(_) => {
                        unreachable!()
                    }
                };
                Self::new(5, json, "ipc", code, error.to_string())
            }
        }
    }

    fn new(
        exit_code: u8,
        json: bool,
        category: &'static str,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            exit_code,
            json,
            category,
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }
}

pub(super) fn run(args: Vec<OsString>) -> Result<CommandOutput, CliError> {
    let json = args.iter().any(|argument| is(argument, "--json"));
    let args = without_json_flag(args, json)?;
    let Some(root) = args.first().and_then(|value| value.to_str()) else {
        return Err(CliError::usage(json, "expected a command"));
    };

    let output = match root {
        "commands" => {
            require_no_arguments(&args[1..], json)?;
            if json {
                json_string(
                    serde_json::to_value(command_catalog())
                        .expect("command catalog is serializable"),
                )
            } else {
                command_catalog()
                    .iter()
                    .map(|command| {
                        let behavior = if command.id == "project.rename" {
                            "mutates, transaction allowed"
                        } else {
                            "mutates"
                        };
                        format!("{} v{} ({behavior})", command.id, command.schema_version)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        "queries" => {
            require_no_arguments(&args[1..], json)?;
            if json {
                json_string(
                    serde_json::to_value(query_catalog()).expect("query catalog is serializable"),
                )
            } else {
                query_catalog()
                    .iter()
                    .map(|query| format!("{} v{}", query.id, query.schema_version))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        "project" => run_project(&args[1..], json)?,
        "history" => run_history(&args[1..], json)?,
        "recovery" => run_recovery(&args[1..], json)?,
        "media" => run_media(&args[1..], json)?,
        "session" => return run_session(&args[1..], json),
        _ => return Err(CliError::usage(json, "unknown command")),
    };
    Ok(CommandOutput::Immediate(output))
}

fn run_project(args: &[OsString], json: bool) -> Result<String, CliError> {
    let Some(action) = args.first().and_then(|value| value.to_str()) else {
        return Err(CliError::usage(
            json,
            "expected project summary, rename, or save",
        ));
    };
    match action {
        "summary" => {
            let options = Options::parse(&args[1..], &["--file", "--attach"], &[], json)?;
            let (path, attached) = project_path(&options, json)?;
            if attached {
                attached_summary(&path, json)
            } else {
                headless_summary(&path, json)
            }
        }
        "rename" => {
            let options = Options::parse(&args[1..], &["--file", "--attach", "--name"], &[], json)?;
            let (path, attached) = project_path(&options, json)?;
            let name = required_name(&options, "--name", json)?;
            if attached {
                attached_rename(&path, name, json)
            } else {
                headless_rename(&path, name, json)
            }
        }
        "save" => {
            let options = Options::parse(&args[1..], &["--attach"], &[], json)?;
            let path = required_path(&options, "--attach", json)?;
            let client = LocalIpcClient::open(path).map_err(|error| CliError::ipc(error, json))?;
            let saved = client.save().map_err(|error| CliError::ipc(error, json))?;
            Ok(if json {
                json_string(serde_json::to_value(saved).expect("save response is serializable"))
            } else {
                format!("Project saved at revision {}.", saved.project_revision)
            })
        }
        _ => Err(CliError::usage(
            json,
            "unknown project action; expected summary, rename, or save",
        )),
    }
}

fn run_media(args: &[OsString], json: bool) -> Result<String, CliError> {
    let Some(action) = args.first().and_then(|value| value.to_str()) else {
        return Err(CliError::usage(json, "expected media probe"));
    };
    if action != "probe" {
        return Err(CliError::usage(
            json,
            "unknown media action; expected probe",
        ));
    }
    let options = Options::parse(&args[1..], &["--file"], &[], json)?;
    let path = required_path(&options, "--file", json)?;
    let metadata =
        or_core::probe_media_file(&path).map_err(|error| CliError::media(error, json))?;
    if json {
        Ok(json_string(
            serde_json::to_value(metadata).expect("media metadata is serializable"),
        ))
    } else {
        Ok(render_media_metadata(&metadata))
    }
}

fn render_media_metadata(metadata: &or_core::MediaMetadata) -> String {
    let format = if metadata.format_names().is_empty() {
        "unknown".to_owned()
    } else {
        metadata.format_names().join(", ")
    };
    let duration = metadata
        .duration()
        .map(format_rational_time)
        .unwrap_or_else(|| "unknown".to_owned());
    let mut lines = vec![
        format!("Format: {format}"),
        format!("Duration: {duration}"),
        format!("File size: {} bytes", metadata.file_size_bytes()),
        format!("Streams: {}", metadata.streams().len()),
    ];

    for stream in metadata.streams() {
        match stream {
            or_core::MediaStreamMetadata::Video(stream) => {
                let mut details = vec![
                    stream.codec_name().unwrap_or("unknown").to_owned(),
                    format!("{}x{}", stream.width(), stream.height()),
                ];
                if let Some(rate) = stream.average_frame_rate() {
                    details.push(format!("{}/{} fps", rate.numerator(), rate.denominator()));
                }
                lines.push(format!("Video #{}: {}", stream.index(), details.join(", ")));
            }
            or_core::MediaStreamMetadata::Audio(stream) => {
                let sample_rate = stream
                    .sample_rate()
                    .map(|rate| format!("{rate} Hz"))
                    .unwrap_or_else(|| "unknown sample rate".to_owned());
                let channels = stream
                    .channels()
                    .map(|count| format!("{count} channels"))
                    .unwrap_or_else(|| "unknown channels".to_owned());
                lines.push(format!(
                    "Audio #{}: {}, {sample_rate}, {channels}",
                    stream.index(),
                    stream.codec_name().unwrap_or("unknown")
                ));
            }
            or_core::MediaStreamMetadata::Other(stream) => {
                let kind = stream.codec_type().unwrap_or("unknown");
                let codec = stream.codec_name().unwrap_or("unknown");
                lines.push(format!("Other #{}: {kind}, {codec}", stream.index()));
            }
        }
    }
    lines.join("\n")
}

fn format_rational_time(time: or_core::RationalTime) -> String {
    if time.denominator() == 1 {
        format!("{} s", time.numerator())
    } else {
        format!("{}/{} s", time.numerator(), time.denominator())
    }
}

fn run_history(args: &[OsString], json: bool) -> Result<String, CliError> {
    let Some(action) = args.first().and_then(|value| value.to_str()) else {
        return Err(CliError::usage(json, "expected history undo or redo"));
    };
    if !matches!(action, "undo" | "redo") {
        return Err(CliError::usage(
            json,
            "unknown history action; expected undo or redo",
        ));
    }
    let options = Options::parse(&args[1..], &["--attach"], &[], json)?;
    let path = required_path(&options, "--attach", json)?;
    let client = LocalIpcClient::open(path).map_err(|error| CliError::ipc(error, json))?;
    let description = client
        .describe()
        .map_err(|error| CliError::ipc(error, json))?;
    let command_id = if action == "undo" {
        "history.undo"
    } else {
        "history.redo"
    };
    let request = command_request(
        command_id,
        description.project_id,
        description.project_instance_id,
        description.project_revision,
        json!({}),
        json,
    )?;
    let result = expect_remote_command(
        client
            .application(ApplicationRequest::Command(request))
            .map_err(|error| CliError::ipc(error, json))?,
        json,
    )?;
    Ok(if json {
        json_string(serde_json::to_value(result).expect("command result is serializable"))
    } else {
        format!(
            "{} completed at revision {}.",
            if action == "undo" { "Undo" } else { "Redo" },
            result.after_revision
        )
    })
}

fn run_recovery(args: &[OsString], json: bool) -> Result<String, CliError> {
    let Some(action) = args.first().and_then(|value| value.to_str()) else {
        return Err(CliError::usage(
            json,
            "expected recovery status, apply, or discard",
        ));
    };
    if !matches!(action, "status" | "apply" | "discard") {
        return Err(CliError::usage(
            json,
            "unknown recovery action; expected status, apply, or discard",
        ));
    }
    let options = Options::parse(&args[1..], &["--file"], &[], json)?;
    let path = required_path(&options, "--file", json)?;
    match action {
        "status" => recovery_status(&path, json),
        "apply" => recovery_apply(&path, json),
        "discard" => recovery_discard(&path, json),
        _ => unreachable!(),
    }
}

fn run_session(args: &[OsString], json: bool) -> Result<CommandOutput, CliError> {
    let Some(action) = args.first().and_then(|value| value.to_str()) else {
        return Err(CliError::usage(
            json,
            "expected session serve, describe, or shutdown",
        ));
    };
    match action {
        "serve" => {
            let options = Options::parse(&args[1..], &["--file", "--descriptor"], &[], json)?;
            let project_path = required_path(&options, "--file", json)?;
            let descriptor_path = options.optional_path("--descriptor");
            let session = ProjectFileSession::open(project_path)
                .map_err(|error| CliError::file(error, json))?;
            let server = LocalIpcServer::start(session, descriptor_path.as_deref())
                .map_err(|error| CliError::ipc(error, json))?;
            let descriptor = server.descriptor();
            let descriptor_path = server.descriptor_path().to_string_lossy();
            let startup = if json {
                serde_json::to_string(&json!({
                    "status": "session_started",
                    "project_id": descriptor.project_id(),
                    "project_instance_id": descriptor.project_instance_id(),
                    "descriptor_path": descriptor_path.as_ref(),
                }))
                .expect("session startup is serializable")
            } else {
                format!(
                    "Session started\nProject ID: {}\nProject instance ID: {}\nDescriptor: {}",
                    descriptor.project_id(),
                    descriptor.project_instance_id(),
                    descriptor_path
                )
            };
            Ok(CommandOutput::Serve {
                server,
                startup,
                json,
            })
        }
        "describe" => {
            let options = Options::parse(&args[1..], &["--attach"], &[], json)?;
            let path = required_path(&options, "--attach", json)?;
            let client = LocalIpcClient::open(path).map_err(|error| CliError::ipc(error, json))?;
            let description = client
                .describe()
                .map_err(|error| CliError::ipc(error, json))?;
            Ok(CommandOutput::Immediate(if json {
                json_string(
                    serde_json::to_value(description).expect("session description is serializable"),
                )
            } else {
                human_description(&description)
            }))
        }
        "shutdown" => {
            let options = Options::parse(&args[1..], &["--attach"], &["--discard-unsaved"], json)?;
            let path = required_path(&options, "--attach", json)?;
            let discard_unsaved = options.has_flag("--discard-unsaved");
            let client = LocalIpcClient::open(path).map_err(|error| CliError::ipc(error, json))?;
            client
                .shutdown(discard_unsaved)
                .map_err(|error| CliError::ipc(error, json))?;
            Ok(CommandOutput::Immediate(if json {
                json_string(json!({"status": "shutdown"}))
            } else {
                "Session shut down.".to_owned()
            }))
        }
        _ => Err(CliError::usage(
            json,
            "unknown session action; expected serve, describe, or shutdown",
        )),
    }
}

fn headless_summary(path: &Path, json: bool) -> Result<String, CliError> {
    let mut session =
        ProjectFileSession::open(path).map_err(|error| CliError::file(error, json))?;
    let request = summary_request(
        session.session().project_id(),
        session.session().project_instance_id(),
        json,
    )?;
    let result = expect_query(
        session.handle_application_request(ApplicationRequest::Query(request)),
        json,
    )?;
    Ok(render_summary(&result, json))
}

fn headless_rename(path: &Path, name: String, json: bool) -> Result<String, CliError> {
    let mut session =
        ProjectFileSession::open(path).map_err(|error| CliError::file(error, json))?;
    let request = command_request(
        "project.rename",
        session.session().project_id(),
        session.session().project_instance_id(),
        session.session().project_revision(),
        json!({"name": name}),
        json,
    )?;
    let result = expect_command(
        session.handle_application_request(ApplicationRequest::Command(request)),
        json,
    )?;
    if result.changed {
        session
            .save()
            .map_err(|error| CliError::file(error, json))?;
    }
    Ok(if json {
        json_string(serde_json::to_value(result).expect("query result is serializable"))
    } else if result.changed {
        format!(
            "Project renamed and saved at revision {}.",
            result.after_revision
        )
    } else {
        "Project name is unchanged.".to_owned()
    })
}

fn attached_summary(path: &Path, json: bool) -> Result<String, CliError> {
    let client = LocalIpcClient::open(path).map_err(|error| CliError::ipc(error, json))?;
    let description = client
        .describe()
        .map_err(|error| CliError::ipc(error, json))?;
    let request = summary_request(
        description.project_id,
        description.project_instance_id,
        json,
    )?;
    let result = expect_remote_query(
        client
            .application(ApplicationRequest::Query(request))
            .map_err(|error| CliError::ipc(error, json))?,
        json,
    )?;
    Ok(render_summary(&result, json))
}

fn attached_rename(path: &Path, name: String, json: bool) -> Result<String, CliError> {
    let client = LocalIpcClient::open(path).map_err(|error| CliError::ipc(error, json))?;
    let description = client
        .describe()
        .map_err(|error| CliError::ipc(error, json))?;
    let request = command_request(
        "project.rename",
        description.project_id,
        description.project_instance_id,
        description.project_revision,
        json!({"name": name}),
        json,
    )?;
    let result = expect_remote_command(
        client
            .application(ApplicationRequest::Command(request))
            .map_err(|error| CliError::ipc(error, json))?,
        json,
    )?;
    Ok(if json {
        json_string(serde_json::to_value(result).expect("command result is serializable"))
    } else if result.changed {
        format!(
            "Project renamed in the live session at revision {}. Save explicitly to persist it.",
            result.after_revision
        )
    } else {
        "Project name is unchanged in the live session.".to_owned()
    })
}

fn summary_request(
    project_id: or_core::ProjectId,
    project_instance_id: or_core::ProjectInstanceId,
    json: bool,
) -> Result<QueryEnvelope, CliError> {
    let schema_version = query_catalog()
        .iter()
        .find(|descriptor| descriptor.id == "project.summary")
        .map(|descriptor| descriptor.schema_version)
        .ok_or_else(|| {
            CliError::operation_message(json, "query catalog is missing project.summary")
        })?;
    Ok(QueryEnvelope {
        query_id: "project.summary".to_owned(),
        schema_version,
        project_id,
        project_instance_id,
        arguments: json!({}),
    })
}

fn command_request(
    command_id: &str,
    project_id: or_core::ProjectId,
    project_instance_id: or_core::ProjectInstanceId,
    expected_project_revision: or_core::ProjectRevision,
    arguments: Value,
    json: bool,
) -> Result<CommandEnvelope, CliError> {
    let schema_version = command_catalog()
        .iter()
        .find(|descriptor| descriptor.id == command_id)
        .map(|descriptor| descriptor.schema_version)
        .ok_or_else(|| {
            CliError::operation_message(json, "command catalog is missing a requested command")
        })?;
    Ok(CommandEnvelope {
        command_id: command_id.to_owned(),
        schema_version,
        project_id,
        project_instance_id,
        expected_project_revision,
        arguments,
    })
}

fn expect_query(
    response: ApplicationResponse,
    json: bool,
) -> Result<or_core::QueryResult, CliError> {
    match response {
        ApplicationResponse::Query(result) => Ok(result),
        ApplicationResponse::Error(error) => Err(CliError::operation(error, json)),
        _ => Err(CliError::operation_message(
            json,
            "unexpected application response",
        )),
    }
}

fn expect_command(
    response: ApplicationResponse,
    json: bool,
) -> Result<or_core::CommandResult, CliError> {
    match response {
        ApplicationResponse::Command(result) => Ok(result),
        ApplicationResponse::Error(error) => Err(CliError::operation(error, json)),
        _ => Err(CliError::operation_message(
            json,
            "unexpected application response",
        )),
    }
}

fn expect_remote_query(
    response: ApplicationSuccess,
    json: bool,
) -> Result<or_core::QueryResult, CliError> {
    match response {
        ApplicationSuccess::Query(result) => Ok(result),
        _ => Err(CliError::ipc_message(
            json,
            "server returned an unexpected application response",
        )),
    }
}

fn expect_remote_command(
    response: ApplicationSuccess,
    json: bool,
) -> Result<or_core::CommandResult, CliError> {
    match response {
        ApplicationSuccess::Command(result) => Ok(result),
        _ => Err(CliError::ipc_message(
            json,
            "server returned an unexpected application response",
        )),
    }
}

fn render_summary(summary: &or_core::QueryResult, json: bool) -> String {
    if json {
        json_string(serde_json::to_value(summary).expect("query result is serializable"))
    } else {
        format!(
            "{}\nProject ID: {}\nProject instance ID: {}\nRevision: {}",
            summary.summary.name,
            summary.summary.project_id,
            summary.summary.project_instance_id,
            summary.summary.project_revision
        )
    }
}

fn human_description(description: &DescribeResponse) -> String {
    let commands = description
        .commands
        .iter()
        .map(|command| format!("{} v{}", command.id, command.schema_version))
        .collect::<Vec<_>>()
        .join(", ");
    let queries = description
        .queries
        .iter()
        .map(|query| format!("{} v{}", query.id, query.schema_version))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Protocol: {}\nProject ID: {}\nProject instance ID: {}\nRevision: {}\nDirty: {}\nCommands: {}\nQueries: {}",
        description.protocol_version,
        description.project_id,
        description.project_instance_id,
        description.project_revision,
        description.dirty,
        commands,
        queries
    )
}

fn recovery_status(path: &Path, json: bool) -> Result<String, CliError> {
    let status = inspect_project_recovery(path).map_err(|error| CliError::recovery(error, json))?;
    let (value, human) = match status {
        RecoveryInspection::None => (json!({"status": "none"}), "Recovery: none".to_owned()),
        RecoveryInspection::Candidate(candidate) => {
            let metadata = candidate.metadata();
            let details = recovery_metadata(metadata);
            (
                json!({"status": "candidate", "metadata": details}),
                format!(
                    "Recovery: candidate\nProject ID: {}\nBase revision: {}\nRecovery revision: {}",
                    metadata.project_id, metadata.base_revision, metadata.recovery_revision
                ),
            )
        }
        RecoveryInspection::Stale(metadata) => (
            json!({"status": "stale", "metadata": recovery_metadata(metadata)}),
            format!(
                "Recovery: stale\nProject ID: {}\nBase revision: {}\nRecovery revision: {}",
                metadata.project_id, metadata.base_revision, metadata.recovery_revision
            ),
        ),
        RecoveryInspection::Conflict { metadata, reason } => (
            json!({
                "status": "conflict",
                "reason": recovery_conflict_reason(reason),
                "metadata": recovery_metadata(metadata),
            }),
            format!(
                "Recovery: conflict ({})\nProject ID: {}\nBase revision: {}\nRecovery revision: {}",
                recovery_conflict_reason(reason),
                metadata.project_id,
                metadata.base_revision,
                metadata.recovery_revision
            ),
        ),
    };
    Ok(if json { json_string(value) } else { human })
}

fn recovery_apply(path: &Path, json: bool) -> Result<String, CliError> {
    let outcome = apply_project_recovery(path).map_err(|error| CliError::recovery(error, json))?;
    let value = match &outcome {
        RecoveryApplyOutcome::AppliedAndCleaned => {
            json!({"status": "applied", "cleanup_pending": false})
        }
        RecoveryApplyOutcome::AppliedCleanupPending { error } => json!({
            "status": "applied",
            "cleanup_pending": true,
            "cleanup_error": error.to_string(),
        }),
    };
    Ok(if json {
        json_string(value)
    } else {
        match outcome {
            RecoveryApplyOutcome::AppliedAndCleaned => {
                "Recovery applied and cleaned up.".to_owned()
            }
            RecoveryApplyOutcome::AppliedCleanupPending { error } => {
                format!("Recovery applied; checkpoint cleanup is pending: {error}")
            }
        }
    })
}

fn recovery_discard(path: &Path, json: bool) -> Result<String, CliError> {
    let discarded =
        discard_project_recovery(path).map_err(|error| CliError::recovery(error, json))?;
    Ok(if json {
        json_string(json!({"discarded": discarded}))
    } else if discarded {
        "Recovery checkpoint discarded.".to_owned()
    } else {
        "No recovery checkpoint was present.".to_owned()
    })
}

fn recovery_metadata(metadata: or_core::RecoveryMetadata) -> Value {
    json!({
        "project_id": metadata.project_id,
        "base_revision": metadata.base_revision,
        "recovery_revision": metadata.recovery_revision,
    })
}

fn recovery_conflict_reason(reason: RecoveryConflictReason) -> &'static str {
    match reason {
        RecoveryConflictReason::Orphaned => "ORPHANED",
        RecoveryConflictReason::ForeignProject => "FOREIGN_PROJECT",
        RecoveryConflictReason::ChangedLineage => "CHANGED_LINEAGE",
    }
}

fn project_path(options: &Options, json: bool) -> Result<(PathBuf, bool), CliError> {
    match (
        options.optional_path("--file"),
        options.optional_path("--attach"),
    ) {
        (Some(path), None) => Ok((path, false)),
        (None, Some(path)) => Ok((path, true)),
        (Some(_), Some(_)) => Err(CliError::usage(json, "choose either --file or --attach")),
        (None, None) => Err(CliError::usage(
            json,
            "one of --file or --attach is required",
        )),
    }
}

fn required_name(options: &Options, flag: &str, json: bool) -> Result<String, CliError> {
    let value = options
        .value(flag)
        .ok_or_else(|| CliError::usage(json, format!("{flag} requires a value")))?;
    value
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| CliError::usage(json, "--name must be valid UTF-8"))
}

fn required_path(options: &Options, flag: &str, json: bool) -> Result<PathBuf, CliError> {
    options
        .optional_path(flag)
        .ok_or_else(|| CliError::usage(json, format!("{flag} requires a path")))
}

fn require_no_arguments(args: &[OsString], json: bool) -> Result<(), CliError> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(CliError::usage(json, "unexpected argument"))
    }
}

fn without_json_flag(args: Vec<OsString>, json: bool) -> Result<Vec<OsString>, CliError> {
    let mut args = args;
    if json {
        if !args.last().is_some_and(|argument| is(argument, "--json")) {
            return Err(CliError::usage(true, "--json must be the final option"));
        }
        args.pop();
    }
    Ok(args)
}

struct Options {
    values: HashMap<String, OsString>,
    flags: HashSet<String>,
}

impl Options {
    fn parse(
        args: &[OsString],
        value_flags: &[&str],
        boolean_flags: &[&str],
        json: bool,
    ) -> Result<Self, CliError> {
        let mut options = Self {
            values: HashMap::new(),
            flags: HashSet::new(),
        };
        let mut index = 0;
        while index < args.len() {
            let key = args[index]
                .to_str()
                .ok_or_else(|| CliError::usage(json, "option name must be valid UTF-8"))?;
            if boolean_flags.contains(&key) {
                if !options.flags.insert(key.to_owned()) {
                    return Err(CliError::usage(json, format!("duplicate option: {key}")));
                }
                index += 1;
                continue;
            }
            if value_flags.contains(&key) {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| CliError::usage(json, format!("{key} requires a value")))?;
                if options
                    .values
                    .insert(key.to_owned(), value.clone())
                    .is_some()
                {
                    return Err(CliError::usage(json, format!("duplicate option: {key}")));
                }
                index += 2;
                continue;
            }
            return Err(CliError::usage(
                json,
                format!("unknown option: {}", args[index].to_string_lossy()),
            ));
        }
        Ok(options)
    }

    fn value(&self, flag: &str) -> Option<&OsStr> {
        self.values.get(flag).map(OsString::as_os_str)
    }

    fn optional_path(&self, flag: &str) -> Option<PathBuf> {
        self.values.get(flag).map(PathBuf::from)
    }

    fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }
}

fn serialized_code(value: Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| "IPC_ERROR".to_owned())
}

fn error_message_for_remote(code: IpcErrorCode) -> String {
    match code {
        IpcErrorCode::UnsupportedProtocol => "local IPC protocol version is unsupported",
        IpcErrorCode::AuthenticationFailed => "local IPC authentication failed",
        IpcErrorCode::InvalidFrame => "local IPC frame is invalid or truncated",
        IpcErrorCode::FrameTooLarge => "local IPC frame exceeds the allowed size",
        IpcErrorCode::InvalidRequest => "local IPC request is invalid",
        IpcErrorCode::ServerStateError => "local IPC server state rejected the request",
        IpcErrorCode::ProjectFileChanged => "the project file changed outside this session",
        IpcErrorCode::RecoveryRequired => "a recovery checkpoint needs explicit attention",
        IpcErrorCode::UnsavedChanges => "the session has unsaved changes",
        IpcErrorCode::EndpointUnavailable => "local IPC endpoint is unavailable",
        IpcErrorCode::UnsupportedPlatform => "local IPC is unsupported on this platform",
    }
    .to_owned()
}

impl CliError {
    fn operation_message(json: bool, message: impl Into<String>) -> Self {
        Self::new(3, json, "application", "APPLICATION_ERROR", message)
    }

    fn ipc_message(json: bool, message: impl Into<String>) -> Self {
        Self::new(5, json, "ipc", "INVALID_RESPONSE", message)
    }
}

fn json_string(value: Value) -> String {
    serde_json::to_string_pretty(&value).expect("CLI response values are serializable")
}

fn is(value: &OsStr, expected: &str) -> bool {
    value == OsStr::new(expected)
}
