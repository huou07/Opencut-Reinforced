use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use or_ai::{
    MAX_TRANSCRIPT_PROPOSAL_BYTES, ManagedFileRef, ProtocolEnvelope, ProviderErrorCode,
    ProviderMessage, TranscribeRequest, TranscriptProposal, managed_transcription_input_path,
    managed_transcription_proposal_path, read_message, write_message,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const MAX_INPUT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_AUDIO_DURATION_MS: u64 = 4 * 60 * 60 * 1_000;
const MAX_PROVIDER_RUNTIME: Duration = Duration::from_secs(4 * 60 * 60);
const FILE_CHUNK_BYTES: usize = 64 * 1024;
const PROVIDER_ID: &str = "whisper.cpp";
const MODEL_ID: &str = "whisper-small-multilingual";
const MODEL_VERSION: &str = "1.0";
const MODEL_LICENSE: &str = "MIT";

struct Config {
    whisper_cli: PathBuf,
    model: PathBuf,
    managed_root: PathBuf,
}

enum Input {
    Message(Box<ProtocolEnvelope>),
    Failed,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("or-whisper-provider: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let config = parse_arguments()?;
    let config = Config {
        whisper_cli: existing_file(config.whisper_cli)
            .map_err(|_| "provider runtime unavailable")?,
        model: existing_file(config.model).map_err(|_| "model unavailable")?,
        managed_root: fs::canonicalize(config.managed_root)
            .map_err(|_| "managed workspace unavailable")?,
    };

    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("or-whisper-provider-control".to_owned())
        .spawn(move || {
            let stdin = io::stdin();
            let mut input = BufReader::new(stdin.lock());
            loop {
                match read_message(&mut input) {
                    Ok(message) => {
                        if sender.send(Input::Message(Box::new(message))).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = sender.send(Input::Failed);
                        break;
                    }
                }
            }
        })
        .map_err(|_| "provider control channel unavailable")?;

    let stdout = io::stdout();
    let mut output = stdout.lock();
    send(
        &mut output,
        Uuid::new_v4(),
        ProviderMessage::Ready {
            capabilities: vec![or_ai::AiTask::Transcribe],
        },
    )?;

    while let Ok(input) = receiver.recv() {
        match input {
            Input::Message(envelope) => match envelope.message {
                ProviderMessage::StartTranscription { request } => {
                    handle_transcription(
                        &config,
                        request,
                        envelope.request_id,
                        &receiver,
                        &mut output,
                    )?;
                }
                ProviderMessage::Shutdown => {
                    send(
                        &mut output,
                        envelope.request_id,
                        ProviderMessage::ShutdownComplete,
                    )?;
                    return Ok(());
                }
                ProviderMessage::Cancel => {
                    send(&mut output, envelope.request_id, ProviderMessage::Cancelled)?;
                }
                _ => send_error(
                    &mut output,
                    envelope.request_id,
                    ProviderErrorCode::UnsupportedTask,
                    "The provider request is not supported.",
                )?,
            },
            Input::Failed => return Ok(()),
        }
    }
    Ok(())
}

fn parse_arguments() -> Result<Config, String> {
    let mut args = env::args_os().skip(1);
    let mut whisper_cli = None;
    let mut model = None;
    let mut managed_root = None;
    while let Some(argument) = args.next() {
        let value = args.next().ok_or("provider configuration is incomplete")?;
        match argument.to_str() {
            Some("--whisper-cli") if whisper_cli.is_none() => whisper_cli = Some(value.into()),
            Some("--model") if model.is_none() => model = Some(value.into()),
            Some("--managed-root") if managed_root.is_none() => managed_root = Some(value.into()),
            _ => return Err("provider configuration is invalid".to_owned()),
        }
    }
    Ok(Config {
        whisper_cli: whisper_cli.ok_or("provider runtime is not configured")?,
        model: model.ok_or("model is not configured")?,
        managed_root: managed_root.ok_or("managed workspace is not configured")?,
    })
}

fn existing_file(path: PathBuf) -> Result<PathBuf, ()> {
    let path = fs::canonicalize(path).map_err(|_| ())?;
    fs::metadata(&path)
        .map_err(|_| ())?
        .is_file()
        .then_some(path)
        .ok_or(())
}

fn handle_transcription(
    config: &Config,
    request: TranscribeRequest,
    request_id: Uuid,
    receiver: &Receiver<Input>,
    output: &mut impl Write,
) -> Result<(), String> {
    if request.identity.provider_id != PROVIDER_ID
        || request.identity.model_id != MODEL_ID
        || request.identity.model_version != MODEL_VERSION
        || request.identity.model_license != MODEL_LICENSE
        || request.language.as_deref().is_some_and(|language| {
            language != "auto"
                && (language.len() < 2
                    || language.len() > 8
                    || !language.bytes().all(|byte| byte.is_ascii_alphabetic()))
        })
        || request.max_duration_ms == 0
        || request.max_duration_ms > MAX_AUDIO_DURATION_MS
    {
        return send_error(
            output,
            request_id,
            ProviderErrorCode::InvalidRequest,
            "This provider accepts Vietnamese, English or auto-detected transcription requests up to four hours.",
        );
    }
    let language = request.language.as_deref().unwrap_or("auto").to_owned();
    let input_path =
        match verified_input(&config.managed_root, &request.input, request_id, receiver) {
            Ok(path) => path,
            Err(VerificationError::Cancelled) => {
                return send(output, request_id, ProviderMessage::Cancelled);
            }
            Err(VerificationError::Unavailable) => {
                return send_error(
                    output,
                    request_id,
                    ProviderErrorCode::InputUnavailable,
                    "The managed audio input is unavailable or failed verification.",
                );
            }
        };

    send(
        output,
        request_id,
        ProviderMessage::Progress {
            fraction: 0.0,
            stage: "Transcribing locally".to_owned(),
        },
    )?;
    let output_id = Uuid::new_v4();
    let output_path = managed_transcription_proposal_path(&config.managed_root, output_id);
    let json_prefix = managed_path(&config.managed_root, Uuid::new_v4(), ".whisper");
    let whisper_json = PathBuf::from(format!("{}.json", json_prefix.to_string_lossy()));
    let whisper_output = WhisperOutput(whisper_json.clone());
    let child = Command::new(&config.whisper_cli)
        .arg("--model")
        .arg(&config.model)
        .arg("--file")
        .arg(&input_path)
        .arg("--output-json-full")
        .arg("--output-file")
        .arg(&json_prefix)
        .arg("--language")
        .arg(&language)
        .arg("--threads")
        .arg("4")
        .arg("--no-gpu")
        .arg("--no-prints")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(_) => {
            drop(whisper_output);
            return send_error(
                output,
                request_id,
                ProviderErrorCode::Unavailable,
                "The packaged local transcription runtime could not start.",
            );
        }
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    drop(whisper_output);
                    return send_error(
                        output,
                        request_id,
                        ProviderErrorCode::Failed,
                        "Local transcription did not complete. Check that the model is valid and that the selected media contains supported audio.",
                    );
                }
                if output_exceeds_limit(&whisper_json, MAX_TRANSCRIPT_PROPOSAL_BYTES as u64)
                    .unwrap_or(true)
                {
                    drop(whisper_output);
                    return send_error(
                        output,
                        request_id,
                        ProviderErrorCode::ResourceLimit,
                        "The local transcription result exceeded its safe size limit.",
                    );
                }
                break;
            }
            Ok(None) => {}
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                drop(whisper_output);
                return send_error(
                    output,
                    request_id,
                    ProviderErrorCode::Failed,
                    "The local transcription runtime stopped unexpectedly.",
                );
            }
        }
        match output_exceeds_limit(&whisper_json, MAX_TRANSCRIPT_PROPOSAL_BYTES as u64) {
            Ok(true) => {
                let _ = child.kill();
                let _ = child.wait();
                drop(whisper_output);
                return send_error(
                    output,
                    request_id,
                    ProviderErrorCode::ResourceLimit,
                    "The local transcription result exceeded its safe size limit.",
                );
            }
            Ok(false) => {}
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                drop(whisper_output);
                return send_error(
                    output,
                    request_id,
                    ProviderErrorCode::Failed,
                    "The local transcription result could not be monitored.",
                );
            }
        }
        if start.elapsed() >= MAX_PROVIDER_RUNTIME {
            let _ = child.kill();
            let _ = child.wait();
            drop(whisper_output);
            return send_error(
                output,
                request_id,
                ProviderErrorCode::ResourceLimit,
                "Local transcription exceeded its four-hour time limit.",
            );
        }
        match receiver.try_recv() {
            Ok(Input::Message(envelope))
                if envelope.request_id == request_id
                    && matches!(envelope.message, ProviderMessage::Cancel) =>
            {
                let _ = child.kill();
                let _ = child.wait();
                drop(whisper_output);
                return send(output, request_id, ProviderMessage::Cancelled);
            }
            Ok(Input::Message(_)) => {
                let _ = child.kill();
                let _ = child.wait();
                drop(whisper_output);
                return send_error(
                    output,
                    request_id,
                    ProviderErrorCode::InvalidRequest,
                    "A new request arrived while transcription was running.",
                );
            }
            Ok(Input::Failed) | Err(TryRecvError::Disconnected) => {
                let _ = child.kill();
                let _ = child.wait();
                drop(whisper_output);
                return Ok(());
            }
            Err(TryRecvError::Empty) => thread::sleep(Duration::from_millis(50)),
        }
    }
    drop(child);
    let whisper_bytes = match read_bounded(&whisper_json, MAX_TRANSCRIPT_PROPOSAL_BYTES) {
        Ok(bytes) => bytes,
        Err(_) => {
            drop(whisper_output);
            return send_error(
                output,
                request_id,
                ProviderErrorCode::ResourceLimit,
                "The local transcription result exceeded its safe size limit.",
            );
        }
    };
    let proposal = match TranscriptProposal::from_whisper_cpp_json(
        &whisper_bytes,
        &request.identity,
        request.max_duration_ms,
    ) {
        Ok(proposal) => proposal,
        Err(_) => {
            drop(whisper_output);
            return send_error(
                output,
                request_id,
                ProviderErrorCode::Failed,
                "The local transcription result could not be validated.",
            );
        }
    };
    drop(whisper_output);
    let proposal_bytes =
        serde_json::to_vec(&proposal).map_err(|_| "proposal serialization failed")?;
    if proposal_bytes.len() > MAX_TRANSCRIPT_PROPOSAL_BYTES {
        return send_error(
            output,
            request_id,
            ProviderErrorCode::ResourceLimit,
            "The local transcription result exceeded its safe size limit.",
        );
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output_path)
        .map_err(|_| "could not create the managed proposal")?;
    file.write_all(&proposal_bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "could not save the managed proposal")?;
    let reference = ManagedFileRef {
        id: output_id,
        size_bytes: proposal_bytes.len() as u64,
        sha256: digest(&proposal_bytes),
    };
    send(
        output,
        request_id,
        ProviderMessage::Progress {
            fraction: 1.0,
            stage: "Proposal ready for review".to_owned(),
        },
    )?;
    send(
        output,
        request_id,
        ProviderMessage::Result { output: reference },
    )
}

#[derive(Debug)]
enum VerificationError {
    Cancelled,
    Unavailable,
}

fn verified_input(
    root: &Path,
    reference: &ManagedFileRef,
    request_id: Uuid,
    receiver: &Receiver<Input>,
) -> Result<PathBuf, VerificationError> {
    if reference.validate().is_err() || reference.size_bytes > MAX_INPUT_BYTES {
        return Err(VerificationError::Unavailable);
    }
    let path = managed_transcription_input_path(root, reference.id);
    let metadata = fs::symlink_metadata(&path).map_err(|_| VerificationError::Unavailable)?;
    if !metadata.is_file() || metadata.len() != reference.size_bytes {
        return Err(VerificationError::Unavailable);
    }
    let resolved = fs::canonicalize(&path).map_err(|_| VerificationError::Unavailable)?;
    if resolved.parent() != Some(root) {
        return Err(VerificationError::Unavailable);
    }
    let mut file = File::open(&resolved).map_err(|_| VerificationError::Unavailable)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; FILE_CHUNK_BYTES];
    loop {
        match receiver.try_recv() {
            Ok(Input::Message(envelope))
                if envelope.request_id == request_id
                    && matches!(envelope.message, ProviderMessage::Cancel) =>
            {
                return Err(VerificationError::Cancelled);
            }
            Ok(Input::Message(_)) => return Err(VerificationError::Unavailable),
            Ok(Input::Failed) | Err(TryRecvError::Disconnected) => {
                return Err(VerificationError::Cancelled);
            }
            Err(TryRecvError::Empty) => {}
        }
        let count = file
            .read(&mut buffer)
            .map_err(|_| VerificationError::Unavailable)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let actual = to_hex(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(&reference.sha256) {
        return Err(VerificationError::Unavailable);
    }
    Ok(resolved)
}

fn managed_path(root: &Path, id: Uuid, suffix: &str) -> PathBuf {
    root.join(format!("{id}{suffix}"))
}

fn read_bounded(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    if file.metadata()?.len() > limit as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
    }
    Ok(bytes)
}

fn output_exceeds_limit(path: &Path, limit: u64) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(metadata.len() > limit),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "transcription output is not a regular file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

struct WhisperOutput(PathBuf);

impl Drop for WhisperOutput {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn send(output: &mut impl Write, request_id: Uuid, message: ProviderMessage) -> Result<(), String> {
    write_message(output, &ProtocolEnvelope::new(request_id, message))
        .map_err(|_| "provider response could not be written".to_owned())
}

fn send_error(
    output: &mut impl Write,
    request_id: Uuid,
    code: ProviderErrorCode,
    message: &str,
) -> Result<(), String> {
    send(
        output,
        request_id,
        ProviderMessage::Error {
            code,
            message: message.to_owned(),
        },
    )
}

fn digest(bytes: &[u8]) -> String {
    to_hex(&Sha256::digest(bytes))
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new() -> Self {
            let path = env::temp_dir().join(format!("or-whisper-provider-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn input(root: &Path, id: Uuid, bytes: &[u8]) -> ManagedFileRef {
        fs::write(managed_transcription_input_path(root, id), bytes).unwrap();
        ManagedFileRef {
            id,
            size_bytes: bytes.len() as u64,
            sha256: digest(bytes),
        }
    }

    #[test]
    fn provider_paths_derive_only_from_uuid_file_references() {
        let id = Uuid::parse_str("a3b2ad96-0cdf-4dce-9291-1ae80e490765").unwrap();
        assert_eq!(
            managed_transcription_input_path(Path::new("/private/jobs"), id),
            PathBuf::from("/private/jobs/a3b2ad96-0cdf-4dce-9291-1ae80e490765.wav")
        );
        assert_eq!(
            to_hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn provider_stops_monitoring_at_the_configured_output_bound() {
        let root = TempRoot::new();
        let path = root.0.join("result.json");
        assert!(!output_exceeds_limit(&path, 8).unwrap());
        fs::write(&path, b"12345678").unwrap();
        assert!(!output_exceeds_limit(&path, 8).unwrap());
        fs::write(&path, b"123456789").unwrap();
        assert!(output_exceeds_limit(&path, 8).unwrap());
    }

    #[test]
    fn managed_input_requires_exact_length_and_sha256() {
        let root = TempRoot::new();
        let id = Uuid::new_v4();
        let bytes = b"small private PCM input";
        let reference = input(&root.0, id, bytes);
        let (_sender, receiver) = mpsc::channel();
        assert_eq!(
            verified_input(&root.0, &reference, Uuid::new_v4(), &receiver).unwrap(),
            root.0.join(format!("{id}.wav"))
        );

        let wrong_digest = ManagedFileRef {
            sha256: "0".repeat(64),
            ..reference.clone()
        };
        assert!(matches!(
            verified_input(&root.0, &wrong_digest, Uuid::new_v4(), &receiver),
            Err(VerificationError::Unavailable)
        ));
        let wrong_length = ManagedFileRef {
            size_bytes: reference.size_bytes + 1,
            ..reference
        };
        assert!(matches!(
            verified_input(&root.0, &wrong_length, Uuid::new_v4(), &receiver),
            Err(VerificationError::Unavailable)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn managed_input_rejects_symbolic_links() {
        let root = TempRoot::new();
        let outside = TempRoot::new();
        let outside_path = outside.0.join("source.wav");
        let bytes = b"external";
        fs::write(&outside_path, bytes).unwrap();
        let id = Uuid::new_v4();
        std::os::unix::fs::symlink(&outside_path, managed_transcription_input_path(&root.0, id))
            .unwrap();
        let reference = ManagedFileRef {
            id,
            size_bytes: bytes.len() as u64,
            sha256: digest(bytes),
        };
        let (_sender, receiver) = mpsc::channel();
        assert!(matches!(
            verified_input(&root.0, &reference, Uuid::new_v4(), &receiver),
            Err(VerificationError::Unavailable)
        ));
    }

    #[test]
    fn managed_input_honors_matching_cancellation_before_reading() {
        let root = TempRoot::new();
        let id = Uuid::new_v4();
        let reference = input(&root.0, id, b"audio");
        let request_id = Uuid::new_v4();
        let (sender, receiver) = mpsc::channel();
        sender
            .send(Input::Message(Box::new(ProtocolEnvelope::new(
                request_id,
                ProviderMessage::Cancel,
            ))))
            .unwrap();
        assert!(matches!(
            verified_input(&root.0, &reference, request_id, &receiver),
            Err(VerificationError::Cancelled)
        ));
    }
}
