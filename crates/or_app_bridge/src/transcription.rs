use std::{
    collections::HashMap,
    env,
    fs::{self, DirBuilder, File, OpenOptions},
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, RecvTimeoutError},
    },
    thread,
    time::{Duration, Instant},
};

use or_ai::{
    AiTask, MAX_TRANSCRIPT_PROPOSAL_BYTES, ManagedFileRef, ModelManifest, ProtocolEnvelope,
    ProviderErrorCode, ProviderMessage, TranscribeRequest, TranscriptIdentity, TranscriptProposal,
    managed_transcription_input_path, managed_transcription_proposal_path,
};
use or_core::{
    JobCancelError, JobCancelOutcome, JobContext, JobFailure, JobId, JobKind, JobManager,
    JobManagerConfig, JobSnapshot, JobState, MediaItem, MediaSourceRef, MediaStreamMetadata,
};
use or_media::SoftwareMediaDecoder;
use or_runtime::{CancellationToken, RuntimeBudgets};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const MAX_TRANSCRIPTION_WAV_BYTES: usize = 512 * 1024 * 1024;
const SOURCE_HASH_CHUNK_BYTES: usize = 256 * 1024;
const PROVIDER_POLL_INTERVAL: Duration = Duration::from_millis(50);
const PROVIDER_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
const PROVIDER_STOP_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_PROVIDER_RUNTIME: Duration = Duration::from_secs(4 * 60 * 60);

#[derive(Clone, Debug)]
pub struct TranscriptionModelInfo {
    pub model_id: String,
    pub label: String,
    pub version: String,
    pub license: String,
    pub language: String,
    pub size_bytes: u64,
    pub source: String,
    pub installed: bool,
    pub provider_available: bool,
    pub active: bool,
}

impl TranscriptionModelInfo {
    pub fn unavailable() -> Self {
        let manifest = or_ai::default_whisper_model();
        Self {
            model_id: manifest.model_id.clone(),
            label: or_ai::whisper_model_label(&manifest.model_id).to_owned(),
            version: manifest.version,
            license: manifest.license,
            language: "Vietnamese and English".to_owned(),
            size_bytes: manifest.size_bytes,
            source: manifest.source,
            installed: false,
            provider_available: false,
            active: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModelDownload {
    pub download_id: String,
    pub model_id: String,
    pub destination: String,
    pub source: String,
    pub size_bytes: u64,
    pub license: String,
}

struct DownloadFile {
    directory: TempDirectory,
    path: PathBuf,
    manifest: ModelManifest,
}

#[derive(Default)]
struct TranscriptionState {
    latest_job: Option<JobId>,
    proposal: Option<TranscriptProposal>,
}

pub struct TranscriptionService {
    jobs: JobManager,
    state: Arc<Mutex<TranscriptionState>>,
    downloads: Mutex<HashMap<Uuid, DownloadFile>>,
    model_root: PathBuf,
    active_model: Mutex<String>,
    provider_binary: PathBuf,
    whisper_cli: PathBuf,
}

impl TranscriptionService {
    pub fn new() -> Option<Self> {
        let model_root = model_store_root()?;
        fs::create_dir_all(&model_root).ok()?;
        let active_model = read_active_model(&model_root);
        let jobs = JobManager::new(JobManagerConfig::new(1, 1, 1).ok()?);
        Some(Self {
            jobs,
            state: Arc::new(Mutex::new(TranscriptionState::default())),
            downloads: Mutex::new(HashMap::new()),
            model_root,
            active_model: Mutex::new(active_model),
            provider_binary: helper_path("OR_WHISPER_PROVIDER_PATH", "or-whisper-provider"),
            whisper_cli: helper_path("OR_WHISPER_CLI_PATH", "whisper-cli"),
        })
    }

    #[cfg(test)]
    fn for_test(model_root: PathBuf) -> Self {
        fs::create_dir_all(&model_root).unwrap();
        Self {
            jobs: JobManager::new(JobManagerConfig::new(1, 1, 1).unwrap()),
            state: Arc::new(Mutex::new(TranscriptionState::default())),
            downloads: Mutex::new(HashMap::new()),
            active_model: Mutex::new(read_active_model(&model_root)),
            model_root,
            provider_binary: PathBuf::from("/nonexistent/or-whisper-provider"),
            whisper_cli: PathBuf::from("/nonexistent/whisper-cli"),
        }
    }

    fn active_manifest(&self) -> ModelManifest {
        let id = lock(&self.active_model).clone();
        or_ai::whisper_model(&id).unwrap_or_else(or_ai::default_whisper_model)
    }

    fn model_info_for(&self, manifest: &ModelManifest) -> TranscriptionModelInfo {
        let active = *lock(&self.active_model) == manifest.model_id;
        TranscriptionModelInfo {
            model_id: manifest.model_id.clone(),
            label: or_ai::whisper_model_label(&manifest.model_id).to_owned(),
            version: manifest.version.clone(),
            license: manifest.license.clone(),
            language: "Vietnamese and English".to_owned(),
            size_bytes: manifest.size_bytes,
            source: manifest.source.clone(),
            installed: or_ai::model_artifact_present(&self.model_root, manifest),
            provider_available: self.provider_binary.is_file() && self.whisper_cli.is_file(),
            active,
        }
    }

    pub fn model_info(&self) -> TranscriptionModelInfo {
        self.model_info_for(&self.active_manifest())
    }

    /// Every catalog entry, with its install and active state, in the fixed
    /// fast-to-best order the picker shows.
    pub fn models(&self) -> Vec<TranscriptionModelInfo> {
        or_ai::whisper_models()
            .iter()
            .map(|manifest| self.model_info_for(manifest))
            .collect()
    }

    /// Removes one installed artifact and falls back to the catalog default
    /// when the removed model was active. The model store lock makes this safe
    /// against a concurrent download or another project host.
    pub fn remove_model(
        &self,
        model_id: &str,
    ) -> Result<TranscriptionModelInfo, TranscriptionServiceError> {
        let manifest =
            or_ai::whisper_model(model_id).ok_or(TranscriptionServiceError::ModelNotFound)?;
        if !or_ai::model_artifact_present(&self.model_root, &manifest) {
            return Err(TranscriptionServiceError::ModelNotInstalled);
        }
        or_ai::remove_installed_model(&self.model_root, &manifest)
            .map_err(|_| TranscriptionServiceError::StorageUnavailable)?;
        if *lock(&self.active_model) == manifest.model_id {
            let default = or_ai::default_whisper_model();
            write_active_model(&self.model_root, &default.model_id)
                .map_err(|_| TranscriptionServiceError::PreferenceUnavailable)?;
            *lock(&self.active_model) = default.model_id;
        }
        Ok(self.model_info())
    }

    /// Selects the model later transcriptions use and persists the choice.
    pub fn select_model(
        &self,
        model_id: &str,
    ) -> Result<TranscriptionModelInfo, TranscriptionServiceError> {
        let manifest =
            or_ai::whisper_model(model_id).ok_or(TranscriptionServiceError::ModelNotFound)?;
        write_active_model(&self.model_root, &manifest.model_id)
            .map_err(|_| TranscriptionServiceError::PreferenceUnavailable)?;
        *lock(&self.active_model) = manifest.model_id.clone();
        Ok(self.model_info_for(&manifest))
    }

    pub fn begin_model_download(
        &self,
        model_id: &str,
    ) -> Result<ModelDownload, TranscriptionServiceError> {
        let manifest =
            or_ai::whisper_model(model_id).ok_or(TranscriptionServiceError::ModelNotFound)?;
        if or_ai::verify_installed_model(&self.model_root, &manifest).is_ok() {
            return Err(TranscriptionServiceError::ModelAlreadyInstalled);
        }
        let mut downloads = lock(&self.downloads);
        if !downloads.is_empty() {
            return Err(TranscriptionServiceError::DownloadBusy);
        }
        let (id, mut file) = create_download_file()?;
        file.manifest = manifest.clone();
        let download = ModelDownload {
            download_id: id.to_string(),
            model_id: manifest.model_id.clone(),
            destination: file.path.to_string_lossy().into_owned(),
            source: manifest.source.clone(),
            size_bytes: manifest.size_bytes,
            license: manifest.license.clone(),
        };
        downloads.insert(id, file);
        Ok(download)
    }

    pub fn finish_model_download(
        &self,
        download_id: &str,
    ) -> Result<TranscriptionModelInfo, TranscriptionServiceError> {
        let id = Uuid::parse_str(download_id).map_err(|_| TranscriptionServiceError::InvalidId)?;
        let file = lock(&self.downloads)
            .remove(&id)
            .ok_or(TranscriptionServiceError::DownloadNotFound)?;
        let manifest = file.manifest.clone();
        let result = install_download(&self.model_root, &manifest, &file);
        drop(file);
        result?;
        self.select_model(&manifest.model_id)
    }

    pub fn cancel_model_download(
        &self,
        download_id: &str,
    ) -> Result<(), TranscriptionServiceError> {
        let id = Uuid::parse_str(download_id).map_err(|_| TranscriptionServiceError::InvalidId)?;
        lock(&self.downloads)
            .remove(&id)
            .map(drop)
            .ok_or(TranscriptionServiceError::DownloadNotFound)
    }

    pub fn start(
        &self,
        project_id: Uuid,
        project_revision: u64,
        media: MediaItem,
    ) -> Result<JobSnapshot, TranscriptionServiceError> {
        if !self.model_info().installed {
            return Err(TranscriptionServiceError::ModelUnavailable);
        }
        if !self.model_info().provider_available {
            return Err(TranscriptionServiceError::ProviderUnavailable);
        }
        let source_path = media
            .source()
            .to_file_path()
            .map_err(|_| TranscriptionServiceError::LocalFileRequired)?;
        let has_audio = media
            .metadata()
            .streams()
            .iter()
            .any(|stream| matches!(stream, MediaStreamMetadata::Audio(_)));
        if !has_audio {
            return Err(TranscriptionServiceError::AudioRequired);
        }
        let mut state = lock(&self.state);
        if state.latest_job.is_some_and(|id| {
            self.jobs
                .snapshot(id)
                .is_some_and(|job| matches!(job.state, JobState::Queued | JobState::Running))
        }) {
            return Err(TranscriptionServiceError::JobBusy);
        }
        state.proposal = None;
        drop(state);

        let model_root = self.model_root.clone();
        let model = self.active_manifest();
        let provider_binary = self.provider_binary.clone();
        let whisper_cli = self.whisper_cli.clone();
        let state = Arc::clone(&self.state);
        let media_id = media.id();
        let job_id = self
            .jobs
            .submit(JobKind::Transcription, move |context| {
                if context.is_cancelled() {
                    return Err(JobFailure::with_message("Transcription was cancelled."));
                }
                let result = transcribe_media(
                    context,
                    TranscriptionInput {
                        source_path: &source_path,
                        project_id,
                        project_revision,
                        media_id,
                        model_root: &model_root,
                        manifest: &model,
                        provider_binary: &provider_binary,
                        whisper_cli: &whisper_cli,
                    },
                );
                match result {
                    Ok(proposal) => {
                        lock(&state).proposal = Some(proposal);
                        Ok(())
                    }
                    Err(message) => Err(JobFailure::with_message(message)),
                }
            })
            .map_err(|_| TranscriptionServiceError::JobSubmit)?;
        let snapshot = self
            .jobs
            .snapshot(job_id)
            .ok_or(TranscriptionServiceError::JobUnavailable)?;
        lock(&self.state).latest_job = Some(job_id);
        Ok(snapshot)
    }

    pub fn snapshot(&self, id: JobId) -> Option<JobSnapshot> {
        self.jobs.snapshot(id)
    }

    pub fn proposal(&self, id: JobId) -> Option<TranscriptProposal> {
        if lock(&self.state).latest_job != Some(id) {
            return None;
        }
        lock(&self.state).proposal.clone()
    }

    pub fn clear_proposal(&self, id: JobId) {
        let mut state = lock(&self.state);
        if state.latest_job == Some(id) {
            state.proposal = None;
        }
    }

    pub fn cancel(&self, id: JobId) -> Result<JobCancelOutcome, JobCancelError> {
        self.jobs.cancel(id)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TranscriptionServiceError {
    AudioRequired,
    DownloadBusy,
    DownloadNotFound,
    InvalidId,
    JobBusy,
    JobUnavailable,
    LocalFileRequired,
    ModelAlreadyInstalled,
    ModelUnavailable,
    ModelNotFound,
    ModelNotInstalled,
    ProviderUnavailable,
    StorageUnavailable,
    PreferenceUnavailable,
    JobSubmit,
}

impl std::fmt::Display for TranscriptionServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::AudioRequired => "Select media that contains an audio stream.",
            Self::DownloadBusy => "A model download is already in progress.",
            Self::DownloadNotFound => "The model download is no longer available.",
            Self::InvalidId => "The transcription request identifier is invalid.",
            Self::JobBusy => "Another transcription is still running.",
            Self::JobUnavailable => "The transcription job could not be inspected.",
            Self::LocalFileRequired => "Local file media is required for transcription.",
            Self::ModelAlreadyInstalled => "The transcription model is already installed.",
            Self::ModelUnavailable => "Download the selected local transcription model first.",
            Self::ModelNotFound => "The selected transcription model is not in the catalog.",
            Self::ModelNotInstalled => "That transcription model is not installed.",
            Self::PreferenceUnavailable => "The selected transcription model could not be saved.",
            Self::ProviderUnavailable => "The packaged local transcription runtime is unavailable.",
            Self::StorageUnavailable => "The model could not be safely installed.",
            Self::JobSubmit => "The transcription queue is full or shutting down.",
        })
    }
}

struct TranscriptionInput<'a> {
    source_path: &'a Path,
    project_id: Uuid,
    project_revision: u64,
    media_id: or_core::MediaId,
    model_root: &'a Path,
    manifest: &'a ModelManifest,
    provider_binary: &'a Path,
    whisper_cli: &'a Path,
}

fn transcribe_media(
    context: &JobContext,
    input: TranscriptionInput<'_>,
) -> Result<TranscriptProposal, String> {
    let installed = or_ai::verify_installed_model(input.model_root, input.manifest)
        .map_err(|_| "The local transcription model is missing or failed verification.")?;
    if context.is_cancelled() {
        return Err("Transcription was cancelled.".to_owned());
    }
    let source_identity = source_file_identity(input.source_path, context)?;
    let temporary = TempDirectory::new("or-transcription-")
        .map_err(|_| "A private temporary workspace for transcription could not be created.")?;
    let wav_id = Uuid::new_v4();
    let wav_path = managed_transcription_input_path(&temporary.path, wav_id);
    let mut wav = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&wav_path)
        .map_err(|_| "Transcription audio could not be created.")?;
    let source = MediaSourceRef::from_file_path(input.source_path)
        .map_err(|_| "The selected media path is invalid.")?;
    let budgets = RuntimeBudgets::new(
        or_runtime::BudgetLimits::new(1, 64 * 1024 * 1024).expect("nonzero render budget"),
        or_runtime::BudgetLimits::new(1, 16 * 1024 * 1024).expect("nonzero audio budget"),
        or_runtime::BudgetLimits::new(1, 64 * 1024 * 1024).expect("nonzero decode budget"),
    );
    let decoder = SoftwareMediaDecoder::new(&source, budgets)
        .map_err(|_| "Audio could not be decoded for local transcription.")?;
    let cancellation = CancellationToken::from_shared_flag(context.shared_cancellation_flag());
    let sample_frames = decoder
        .write_transcription_wav(&mut wav, 4 * 60 * 60 * 1_000, &cancellation)
        .map_err(|error| match error {
            or_media::DecodeError::Cancelled => "Transcription was cancelled.".to_owned(),
            or_media::DecodeError::MissingAudioStream => {
                "The selected media does not contain a readable audio stream.".to_owned()
            }
            _ => "Audio could not be decoded for local transcription.".to_owned(),
        })?;
    wav.sync_all()
        .map_err(|_| "Transcription audio could not be finalized.")?;
    drop(wav);
    if context.is_cancelled() {
        return Err("Transcription was cancelled.".to_owned());
    }
    if source_file_identity(input.source_path, context)? != source_identity {
        return Err("The source media changed during transcription. Try again.".to_owned());
    }
    let media_sha256 = source_identity.sha256;
    let wav_reference = file_reference(wav_id, &wav_path, MAX_TRANSCRIPTION_WAV_BYTES)
        .map_err(|_| "Transcription audio exceeded its safe size limit.")?;
    let duration_ms = sample_frames
        .checked_mul(1_000)
        .and_then(|value| value.checked_add(15_999))
        .map(|value| value / 16_000)
        .ok_or("Transcription duration is out of range.")?;
    if duration_ms == 0 {
        return Err("No readable audio was found in the selected media.".to_owned());
    }
    let identity = TranscriptIdentity {
        project_id: input.project_id,
        project_revision: input.project_revision,
        media_id: Uuid::parse_str(&input.media_id.to_string())
            .map_err(|_| "The selected media identity is invalid.")?,
        media_sha256,
        provider_id: input.manifest.provider_id.clone(),
        model_id: input.manifest.model_id.clone(),
        model_version: input.manifest.version.clone(),
        model_license: input.manifest.license.clone(),
    };
    context.report_progress(1, 100);
    run_provider(
        context,
        ProviderRequest {
            provider_binary: input.provider_binary,
            whisper_cli: input.whisper_cli,
            model: &installed.path,
            managed_root: &temporary.path,
            input: wav_reference,
            identity: identity.clone(),
            duration_ms,
        },
    )
}

struct SourceFileIdentity {
    size: u64,
    modified: Option<std::time::SystemTime>,
    sha256: String,
}

fn source_file_identity(path: &Path, context: &JobContext) -> Result<SourceFileIdentity, String> {
    let metadata = fs::metadata(path).map_err(|_| "The selected media file is unavailable.")?;
    if !metadata.is_file() {
        return Err("The selected media file is unavailable.".to_owned());
    }
    let mut file = File::open(path).map_err(|_| "The selected media file is unavailable.")?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; SOURCE_HASH_CHUNK_BYTES];
    loop {
        if context.is_cancelled() {
            return Err("Transcription was cancelled.".to_owned());
        }
        let count = file
            .read(&mut buffer)
            .map_err(|_| "The selected media file could not be read.")?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(SourceFileIdentity {
        size: metadata.len(),
        modified: metadata.modified().ok(),
        sha256: to_hex(&hasher.finalize()),
    })
}

impl PartialEq for SourceFileIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.modified == other.modified && self.sha256 == other.sha256
    }
}

fn file_reference(id: Uuid, path: &Path, max_bytes: usize) -> io::Result<ManagedFileRef> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > max_bytes as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "managed file size",
        ));
    }
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; SOURCE_HASH_CHUNK_BYTES];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(ManagedFileRef {
        id,
        size_bytes: metadata.len(),
        sha256: to_hex(&hasher.finalize()),
    })
}

struct ProviderRequest<'a> {
    provider_binary: &'a Path,
    whisper_cli: &'a Path,
    model: &'a Path,
    managed_root: &'a Path,
    input: ManagedFileRef,
    identity: TranscriptIdentity,
    duration_ms: u64,
}

fn run_provider(
    context: &JobContext,
    request: ProviderRequest<'_>,
) -> Result<TranscriptProposal, String> {
    if context.is_cancelled() {
        return Err("Transcription was cancelled.".to_owned());
    }
    let mut command = Command::new(request.provider_binary);
    command
        .arg("--whisper-cli")
        .arg(request.whisper_cli)
        .arg("--model")
        .arg(request.model)
        .arg("--managed-root")
        .arg(request.managed_root)
        .arg("--model-id")
        .arg(&request.identity.model_id)
        .arg("--model-version")
        .arg(&request.identity.model_version)
        .arg("--model-license")
        .arg(&request.identity.model_license)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    if let Some(system_root) = env::var_os("SystemRoot") {
        command.env("SystemRoot", system_root);
    }
    let mut child = command
        .spawn()
        .map_err(|_| "The packaged local transcription provider could not start.")?;
    let Some(mut stdin) = child.stdin.take() else {
        stop_child(&mut child);
        return Err("The local transcription provider could not be started.".to_owned());
    };
    let Some(stdout) = child.stdout.take() else {
        stop_child(&mut child);
        return Err("The local transcription provider could not be started.".to_owned());
    };
    let (sender, receiver) = mpsc::channel();
    let reader = thread::Builder::new()
        .name("or-transcription-provider-output".to_owned())
        .spawn(move || {
            let mut output = BufReader::new(stdout);
            loop {
                match or_ai::read_message(&mut output) {
                    Ok(message) => {
                        if sender.send(Some(message)).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = sender.send(None);
                        break;
                    }
                }
            }
        })
        .map_err(|_| {
            stop_child(&mut child);
            "The local transcription provider could not be monitored."
        })?;
    let mut process = ProviderChild {
        child,
        reader: Some(reader),
    };
    let request_id = Uuid::new_v4();
    match receive_provider_message(&receiver, PROVIDER_HANDSHAKE_TIMEOUT, None)
        .map_err(provider_read_error_message)?
    {
        ProviderMessage::Ready { capabilities } if capabilities.contains(&AiTask::Transcribe) => {}
        _ => return Err("The local transcription provider is incompatible.".to_owned()),
    }
    write_provider_message(
        &mut stdin,
        request_id,
        ProviderMessage::StartTranscription {
            request: TranscribeRequest {
                input: request.input,
                identity: request.identity.clone(),
                language: None,
                max_duration_ms: request.duration_ms,
            },
        },
    )?;
    let started = Instant::now();
    loop {
        if context.is_cancelled() {
            write_provider_message(&mut stdin, request_id, ProviderMessage::Cancel)?;
            let _ = receive_provider_message(&receiver, PROVIDER_STOP_TIMEOUT, Some(request_id));
            return Err("Transcription was cancelled.".to_owned());
        }
        if started.elapsed() >= MAX_PROVIDER_RUNTIME {
            write_provider_message(&mut stdin, request_id, ProviderMessage::Cancel)?;
            let _ = receive_provider_message(&receiver, PROVIDER_STOP_TIMEOUT, Some(request_id));
            return Err("Local transcription exceeded its four-hour time limit.".to_owned());
        }
        match receive_provider_message(&receiver, PROVIDER_POLL_INTERVAL, Some(request_id)) {
            Ok(ProviderMessage::Progress { fraction, .. }) => {
                context.report_progress((fraction * 100.0) as u64, 100);
            }
            Ok(ProviderMessage::Result { output }) => {
                let path = managed_transcription_proposal_path(request.managed_root, output.id);
                let actual = file_reference(output.id, &path, MAX_TRANSCRIPT_PROPOSAL_BYTES)
                    .map_err(|_| "The local transcription proposal failed verification.")?;
                if actual.size_bytes != output.size_bytes
                    || !actual.sha256.eq_ignore_ascii_case(&output.sha256)
                {
                    return Err("The local transcription proposal failed verification.".to_owned());
                }
                let bytes = read_bounded(&path, MAX_TRANSCRIPT_PROPOSAL_BYTES)
                    .map_err(|_| "The local transcription proposal exceeded its safe size.")?;
                let proposal =
                    TranscriptProposal::decode(&bytes, &request.identity, request.duration_ms)
                        .map_err(|_| "The local transcription proposal failed validation.")?;
                write_provider_message(&mut stdin, request_id, ProviderMessage::Shutdown)?;
                match receive_provider_message(&receiver, PROVIDER_STOP_TIMEOUT, Some(request_id))
                    .map_err(provider_read_error_message)?
                {
                    ProviderMessage::ShutdownComplete => {}
                    _ => {
                        return Err(
                            "The local transcription provider did not stop cleanly.".to_owned()
                        );
                    }
                }
                process.finish()?;
                return Ok(proposal);
            }
            Ok(ProviderMessage::Error { code, .. }) => {
                return Err(provider_error_message(code).to_owned());
            }
            Ok(ProviderMessage::Cancelled) => {
                return Err("Transcription was cancelled.".to_owned());
            }
            Ok(_) => {
                return Err(
                    "The local transcription provider returned an invalid response.".to_owned(),
                );
            }
            Err(ProviderReadError::Timeout) => {}
            Err(ProviderReadError::Closed) => {
                return Err("The local transcription provider stopped unexpectedly.".to_owned());
            }
            Err(ProviderReadError::Invalid) => {
                return Err(
                    "The local transcription provider returned an invalid response.".to_owned(),
                );
            }
        }
    }
}

enum ProviderReadError {
    Timeout,
    Closed,
    Invalid,
}

fn receive_provider_message(
    receiver: &Receiver<Option<ProtocolEnvelope>>,
    timeout: Duration,
    expected_request_id: Option<Uuid>,
) -> Result<ProviderMessage, ProviderReadError> {
    match receiver.recv_timeout(timeout) {
        Ok(Some(envelope))
            if expected_request_id.is_none_or(|expected| envelope.request_id == expected) =>
        {
            Ok(envelope.message)
        }
        Ok(Some(_)) => Err(ProviderReadError::Invalid),
        Ok(None) | Err(RecvTimeoutError::Disconnected) => Err(ProviderReadError::Closed),
        Err(RecvTimeoutError::Timeout) => Err(ProviderReadError::Timeout),
    }
}

fn provider_read_error_message(error: ProviderReadError) -> String {
    match error {
        ProviderReadError::Timeout => "The local transcription provider did not respond in time.",
        ProviderReadError::Closed => "The local transcription provider stopped unexpectedly.",
        ProviderReadError::Invalid => {
            "The local transcription provider returned an invalid response."
        }
    }
    .to_owned()
}

fn write_provider_message(
    output: &mut impl Write,
    request_id: Uuid,
    message: ProviderMessage,
) -> Result<(), String> {
    or_ai::write_message(output, &ProtocolEnvelope::new(request_id, message))
        .map_err(|_| "The local transcription provider could not be controlled.".to_owned())
}

fn provider_error_message(code: ProviderErrorCode) -> &'static str {
    match code {
        ProviderErrorCode::Unavailable => {
            "The packaged local transcription runtime is unavailable."
        }
        ProviderErrorCode::InvalidRequest => "The media could not be prepared for transcription.",
        ProviderErrorCode::UnsupportedTask => "This local provider does not support transcription.",
        ProviderErrorCode::InputUnavailable => {
            "The private transcription audio failed verification."
        }
        ProviderErrorCode::ModelUnavailable => "The local transcription model is unavailable.",
        ProviderErrorCode::ModelInvalid => "The local transcription model failed verification.",
        ProviderErrorCode::ResourceLimit => "Local transcription exceeded its safe resource limit.",
        ProviderErrorCode::Cancelled => "Transcription was cancelled.",
        ProviderErrorCode::Failed => {
            "Local transcription did not complete. Check that the selected media contains supported speech audio."
        }
    }
}

struct ProviderChild {
    child: Child,
    reader: Option<thread::JoinHandle<()>>,
}

impl ProviderChild {
    fn finish(&mut self) -> Result<(), String> {
        let status = self
            .child
            .wait()
            .map_err(|_| "The local transcription provider did not stop cleanly.")?;
        if let Some(reader) = self.reader.take() {
            reader
                .join()
                .map_err(|_| "The local transcription provider could not be closed.")?;
        }
        if !status.success() {
            return Err("The local transcription provider did not stop cleanly.".to_owned());
        }
        Ok(())
    }
}

impl Drop for ProviderChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn new(prefix: &str) -> io::Result<Self> {
        for _ in 0..8 {
            let path = env::temp_dir().join(format!("{prefix}{}", Uuid::new_v4()));
            let mut builder = DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "temporary name bound",
        ))
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn create_download_file() -> Result<(Uuid, DownloadFile), TranscriptionServiceError> {
    let directory = TempDirectory::new("or-model-download-")
        .map_err(|_| TranscriptionServiceError::StorageUnavailable)?;
    let path = directory.path.join("model.bin");
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| TranscriptionServiceError::StorageUnavailable)?;
    let id = Uuid::new_v4();
    let file = DownloadFile {
        directory,
        path,
        manifest: or_ai::default_whisper_model(),
    };
    Ok((id, file))
}

fn active_model_path(model_root: &Path) -> PathBuf {
    model_root.join("active-model")
}

/// Reads the persisted model choice. An unknown or missing file falls back to
/// the catalog default rather than failing the whole service.
fn read_active_model(model_root: &Path) -> String {
    fs::read_to_string(active_model_path(model_root))
        .ok()
        .map(|text| text.trim().to_owned())
        .filter(|model_id| or_ai::whisper_model(model_id).is_some())
        .unwrap_or_else(|| or_ai::default_whisper_model().model_id)
}

/// Replaces the model choice atomically; a partial write can never point the
/// service at a model that is not in the catalog.
fn write_active_model(model_root: &Path, model_id: &str) -> io::Result<()> {
    let temporary = model_root.join(format!(".active-model-{}", Uuid::new_v4()));
    fs::write(&temporary, model_id)?;
    if let Err(error) = fs::rename(&temporary, active_model_path(model_root)) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

fn install_download(
    model_root: &Path,
    manifest: &ModelManifest,
    file: &DownloadFile,
) -> Result<(), TranscriptionServiceError> {
    let metadata = fs::symlink_metadata(&file.path)
        .map_err(|_| TranscriptionServiceError::StorageUnavailable)?;
    if !metadata.is_file() || metadata.len() > manifest.size_bytes {
        return Err(TranscriptionServiceError::StorageUnavailable);
    }
    let root = file
        .directory
        .path
        .canonicalize()
        .map_err(|_| TranscriptionServiceError::StorageUnavailable)?;
    let path = file
        .path
        .canonicalize()
        .map_err(|_| TranscriptionServiceError::StorageUnavailable)?;
    if path.parent() != Some(root.as_path()) {
        return Err(TranscriptionServiceError::StorageUnavailable);
    }
    let source = File::open(path).map_err(|_| TranscriptionServiceError::StorageUnavailable)?;
    or_ai::install_verified_model(model_root, manifest, source, || false)
        .map(|_| ())
        .map_err(|_| TranscriptionServiceError::StorageUnavailable)
}

fn model_store_root() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        return env::var_os("HOME").map(PathBuf::from).map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("Opencut Reinforced")
                .join("Models")
        });
    }
    #[cfg(target_os = "windows")]
    {
        return env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|root| root.join("Opencut Reinforced").join("Models"));
    }
    #[cfg(target_os = "linux")]
    {
        return env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".local/share"))
            })
            .map(|root| root.join("opencut-reinforced").join("models"));
    }
    #[allow(unreachable_code)]
    None
}

fn helper_path(environment_name: &str, binary_name: &str) -> PathBuf {
    if let Some(path) = env::var_os(environment_name).filter(|value| !value.is_empty()) {
        return PathBuf::from(path);
    }
    let file_name = if cfg!(windows) {
        format!("{binary_name}.exe")
    } else {
        binary_name.to_owned()
    };
    if let Some(directory) = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        let packaged = directory.join(&file_name);
        if packaged.is_file() {
            return packaged;
        }
    }
    PathBuf::from(file_name)
}

fn read_bounded(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    if file.metadata()?.len() > limit as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "proposal too large",
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "proposal too large",
        ));
    }
    Ok(bytes)
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temporary_model_root() -> PathBuf {
        std::env::temp_dir().join(format!("or-transcription-service-{}", Uuid::new_v4()))
    }

    #[test]
    fn catalog_selection_is_persisted_and_unknown_models_are_refused() {
        let root = temporary_model_root();
        let service = TranscriptionService::for_test(root.clone());
        let models = service.models();
        assert_eq!(models.len(), 5);
        assert_eq!(
            models
                .iter()
                .find(|model| model.active)
                .expect("an active model")
                .model_id,
            "whisper-small-multilingual"
        );
        assert!(models.iter().all(|model| !model.installed));

        let selected = service.select_model("whisper-medium-multilingual").unwrap();
        assert_eq!(selected.model_id, "whisper-medium-multilingual");
        assert!(selected.active);
        assert!(!selected.installed);
        assert_eq!(
            fs::read_to_string(root.join("active-model")).unwrap(),
            "whisper-medium-multilingual"
        );

        // A new service instance reads the same persisted choice.
        let reopened = TranscriptionService::for_test(root.clone());
        assert_eq!(
            reopened.model_info().model_id,
            "whisper-medium-multilingual"
        );

        assert!(matches!(
            service.select_model("not-a-model"),
            Err(TranscriptionServiceError::ModelNotFound)
        ));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn removing_an_installed_model_deletes_it_and_falls_back_to_small() {
        let root = temporary_model_root();
        fs::create_dir_all(&root).unwrap();
        let service = TranscriptionService::for_test(root.clone());
        let manifest = or_ai::whisper_model("whisper-medium-multilingual").unwrap();
        let installed = root.join(format!(
            "{}-{}-{}-{}.model",
            manifest.model_id.len(),
            manifest.model_id,
            manifest.version.len(),
            manifest.version
        ));
        let file = File::create(&installed).unwrap();
        file.set_len(manifest.size_bytes).unwrap();
        drop(file);
        assert!(or_ai::model_artifact_present(&root, &manifest));
        service
            .select_model("whisper-medium-multilingual")
            .unwrap();
        assert_eq!(
            service.model_info().model_id,
            "whisper-medium-multilingual"
        );

        let info = service.remove_model("whisper-medium-multilingual").unwrap();
        assert_eq!(info.model_id, "whisper-small-multilingual");
        assert!(!installed.exists());
        assert!(matches!(
            service.remove_model("whisper-medium-multilingual"),
            Err(TranscriptionServiceError::ModelNotInstalled)
        ));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn removing_an_uninstalled_or_unknown_model_is_refused() {
        let root = temporary_model_root();
        let service = TranscriptionService::for_test(root.clone());
        assert!(matches!(
            service.remove_model("not-a-model"),
            Err(TranscriptionServiceError::ModelNotFound)
        ));
        assert!(matches!(
            service.remove_model("whisper-base-multilingual"),
            Err(TranscriptionServiceError::ModelNotInstalled)
        ));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn downloads_are_bound_to_the_requested_manifest() {
        let root = temporary_model_root();
        let service = TranscriptionService::for_test(root.clone());
        let download = service
            .begin_model_download("whisper-base-multilingual")
            .unwrap();
        assert_eq!(download.model_id, "whisper-base-multilingual");
        assert_eq!(download.size_bytes, 147_951_465);
        service
            .cancel_model_download(&download.download_id)
            .unwrap();
        assert!(matches!(
            service.begin_model_download("not-a-model"),
            Err(TranscriptionServiceError::ModelNotFound)
        ));
        fs::remove_dir_all(root).ok();
    }
}
