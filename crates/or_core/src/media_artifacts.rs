use crate::{
    cache::{
        CacheArtifactKind, CacheKey, CacheStagingFile, CacheStore, CacheStoreConfig,
        PROXY_MAX_ARTIFACT_BYTES, ParametersFingerprint, SourceFingerprint,
    },
    jobs::{
        JobCancelError, JobCancelOutcome, JobContext, JobFailure, JobId, JobKind, JobManager,
        JobManagerConfig, JobSnapshot, JobState, JobSubmitError,
    },
    media::{MediaId, MediaItem, MediaStreamMetadata},
    time::RationalTime,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    error::Error,
    ffi::OsString,
    fmt,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, MutexGuard, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const SOURCE_FINGERPRINT_SAMPLE_WINDOW_BYTES: usize = 256 * 1024;
pub const SOURCE_FINGERPRINT_MAX_TOTAL_SAMPLE_BYTES: usize = 1024 * 1024;

const SOURCE_FINGERPRINT_DOMAIN: &str = "opencut-reinforced-source-fingerprint-v1";
const THUMBNAIL_PROFILE: &str =
    "opencut-reinforced-thumbnail-v1\nformat=png\nframe=first\nmax_edge=320\npreserve_aspect=true";
const WAVEFORM_PROFILE: &str = "opencut-reinforced-waveform-v1\nformat=png\nwidth=512\nheight=96\nstream=first_audio\nchannels=combined\ncolor=white";
const PROXY_PROFILE: &str = "opencut-reinforced-proxy-v1\ncontainer=matroska\nvideo_codec=mpeg4\nmax_width=960\nmax_height=540\nupscale=false\nsquare_pixels=true\npixel_format=yuv420p\nqscale=6\ngop=12\nbframes=0\nfps_mode=passthrough\npts=start_at_zero\naudio=none\nmetadata=none";
const THUMBNAIL_TIMEOUT: Duration = Duration::from_secs(20);
const WAVEFORM_TIMEOUT: Duration = Duration::from_secs(30);
const PROXY_UNKNOWN_DURATION_TIMEOUT: Duration = Duration::from_secs(1800);
const PROXY_MIN_TIMEOUT: u64 = 120;
const PROXY_MAX_TIMEOUT: u64 = 7200;
const MAX_ARTIFACT_STDOUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_ARTIFACT_STDERR_BYTES: usize = 64 * 1024;
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(10);
const PROXY_PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceFingerprintError {
    NotFound,
    NotRegularFile,
    MetadataFailure,
    ReadFailure,
}

impl SourceFingerprintError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotFound => "SOURCE_NOT_FOUND",
            Self::NotRegularFile => "SOURCE_NOT_REGULAR_FILE",
            Self::MetadataFailure => "SOURCE_METADATA_FAILURE",
            Self::ReadFailure => "SOURCE_READ_FAILURE",
        }
    }
}

impl fmt::Display for SourceFingerprintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotFound => "media source was not found",
            Self::NotRegularFile => "media source is not a regular file",
            Self::MetadataFailure => "media source metadata could not be read",
            Self::ReadFailure => "media source samples could not be read",
        })
    }
}

impl Error for SourceFingerprintError {}

/// Builds a bounded cache-invalidation fingerprint from a native file path.
/// This is not a full-file content identity or an integrity proof.
pub fn fingerprint_media_source(path: &Path) -> Result<SourceFingerprint, SourceFingerprintError> {
    let metadata = fs::metadata(path).map_err(fingerprint_metadata_error)?;
    if !metadata.is_file() {
        return Err(SourceFingerprintError::NotRegularFile);
    }
    let resolved = fs::canonicalize(path).map_err(fingerprint_metadata_error)?;
    let mut file = File::open(resolved).map_err(fingerprint_metadata_error)?;
    let metadata = file
        .metadata()
        .map_err(|_| SourceFingerprintError::MetadataFailure)?;
    if !metadata.is_file() {
        return Err(SourceFingerprintError::NotRegularFile);
    }
    fingerprint_open_file(&mut file, metadata.len(), metadata.modified().ok())
}

fn fingerprint_metadata_error(error: io::Error) -> SourceFingerprintError {
    if error.kind() == io::ErrorKind::NotFound {
        SourceFingerprintError::NotFound
    } else {
        SourceFingerprintError::MetadataFailure
    }
}

fn fingerprint_open_file(
    file: &mut File,
    file_size: u64,
    modified: Option<SystemTime>,
) -> Result<SourceFingerprint, SourceFingerprintError> {
    let samples = sample_windows(file_size);
    let mut hasher = Sha256::new();
    hasher.update(SOURCE_FINGERPRINT_DOMAIN.as_bytes());
    hasher.update([0]);
    hasher.update(file_size.to_be_bytes());
    update_modified_time(&mut hasher, modified);
    hasher.update((samples.len() as u32).to_be_bytes());

    let mut buffer = vec![0_u8; SOURCE_FINGERPRINT_SAMPLE_WINDOW_BYTES];
    for (offset, length) in samples {
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| SourceFingerprintError::ReadFailure)?;
        file.read_exact(&mut buffer[..length])
            .map_err(|_| SourceFingerprintError::ReadFailure)?;
        hasher.update(offset.to_be_bytes());
        hasher.update((length as u32).to_be_bytes());
        hasher.update(&buffer[..length]);
    }
    Ok(SourceFingerprint::from_digest(hasher.finalize().into()))
}

fn update_modified_time(hasher: &mut Sha256, modified: Option<SystemTime>) {
    let Some(modified) = modified else {
        hasher.update([0]);
        return;
    };
    hasher.update([1]);
    let (direction, duration) = match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => (1_u8, duration),
        Err(error) => (2_u8, error.duration()),
    };
    hasher.update([direction]);
    hasher.update(duration.as_secs().to_be_bytes());
    hasher.update(duration.subsec_nanos().to_be_bytes());
}

fn sample_windows(file_size: u64) -> Vec<(u64, usize)> {
    let window = SOURCE_FINGERPRINT_SAMPLE_WINDOW_BYTES as u64;
    let mut offsets = if file_size <= SOURCE_FINGERPRINT_MAX_TOTAL_SAMPLE_BYTES as u64 {
        if file_size == 0 {
            vec![0]
        } else {
            let mut offsets = Vec::new();
            let mut offset = 0;
            while offset < file_size {
                offsets.push(offset);
                offset += window;
            }
            offsets
        }
    } else {
        vec![0, file_size / 3, (file_size / 3) * 2, file_size - window]
    };
    offsets.sort_unstable();
    offsets.dedup();
    offsets
        .into_iter()
        .map(|offset| {
            (
                offset,
                usize::try_from((file_size - offset).min(window)).expect("sample fits usize"),
            )
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ArtifactRequestKey {
    kind: CacheArtifactKind,
    cache_key: CacheKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaArtifactRequestState {
    Ready,
    Queued,
    Running,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaArtifactNotApplicableReason {
    NoVideoStream,
    NoAudioStream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MediaArtifactRequest {
    pub media_id: MediaId,
    pub kind: CacheArtifactKind,
    pub cache_key: Option<CacheKey>,
    pub job_id: Option<JobId>,
    pub state: MediaArtifactRequestState,
    pub not_applicable_reason: Option<MediaArtifactNotApplicableReason>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaArtifactErrorCode {
    BackendUnavailable,
    SourceUnavailable,
    GenerationTimeout,
    OutputTooLarge,
    GenerationFailed,
    InvalidGeneratedArtifact,
    CacheError,
    Cancelled,
}

impl MediaArtifactErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BackendUnavailable => "BACKEND_UNAVAILABLE",
            Self::SourceUnavailable => "SOURCE_UNAVAILABLE",
            Self::GenerationTimeout => "GENERATION_TIMEOUT",
            Self::OutputTooLarge => "OUTPUT_TOO_LARGE",
            Self::GenerationFailed => "GENERATION_FAILED",
            Self::InvalidGeneratedArtifact => "INVALID_GENERATED_ARTIFACT",
            Self::CacheError => "CACHE_ERROR",
            Self::Cancelled => "CANCELLED",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaArtifactRequestError {
    SourceUnavailable,
    CacheUnavailable,
    ArtifactNotReadable,
    QueueFull,
    RecordCapacityExceeded,
    ServiceShutdown,
}

impl MediaArtifactRequestError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::SourceUnavailable => "SOURCE_UNAVAILABLE",
            Self::CacheUnavailable => "CACHE_UNAVAILABLE",
            Self::ArtifactNotReadable => "ARTIFACT_NOT_READABLE",
            Self::QueueFull => "QUEUE_FULL",
            Self::RecordCapacityExceeded => "JOB_RECORD_CAPACITY_EXCEEDED",
            Self::ServiceShutdown => "ARTIFACT_SERVICE_SHUTDOWN",
        }
    }
}

impl fmt::Display for MediaArtifactRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SourceUnavailable => "media source is unavailable",
            Self::CacheUnavailable => "media preview cache is unavailable",
            Self::ArtifactNotReadable => {
                "proxy artifacts are file-backed and cannot be read as bytes"
            }
            Self::QueueFull => "media preview queue is full",
            Self::RecordCapacityExceeded => "media preview job capacity is full",
            Self::ServiceShutdown => "media preview service is shutting down",
        })
    }
}

impl Error for MediaArtifactRequestError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaArtifactEventState {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaArtifactEvent {
    pub sequence: u64,
    pub media_id: MediaId,
    pub kind: CacheArtifactKind,
    pub cache_key: CacheKey,
    pub job_id: JobId,
    pub state: MediaArtifactEventState,
    pub error_code: Option<MediaArtifactErrorCode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaArtifactServiceConfig {
    pub cache_root: PathBuf,
    pub job_manager: JobManagerConfig,
    pub cache: CacheStoreConfig,
    pub ffmpeg_executable: PathBuf,
}

impl MediaArtifactServiceConfig {
    pub fn new(
        cache_root: impl Into<PathBuf>,
        job_manager: JobManagerConfig,
        cache: CacheStoreConfig,
        ffmpeg_executable: impl Into<PathBuf>,
    ) -> Self {
        Self {
            cache_root: cache_root.into(),
            job_manager,
            cache,
            ffmpeg_executable: ffmpeg_executable.into(),
        }
    }

    pub fn system_ffmpeg(
        cache_root: impl Into<PathBuf>,
        job_manager: JobManagerConfig,
        cache: CacheStoreConfig,
    ) -> Self {
        Self::new(
            cache_root,
            job_manager,
            cache,
            ffmpeg_executable_from_environment(),
        )
    }
}

pub fn ffmpeg_executable_from_environment() -> PathBuf {
    std::env::var_os("OR_FFMPEG_PATH")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MediaArtifactServiceInitError;

impl fmt::Display for MediaArtifactServiceInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("media preview cache root is unavailable")
    }
}

impl Error for MediaArtifactServiceInitError {}

#[derive(Clone, Copy)]
struct GenerationLimits {
    thumbnail_timeout: Duration,
    waveform_timeout: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
    poll_interval: Duration,
    proxy_poll_interval: Duration,
    proxy_timeout_override: Option<Duration>,
}

const DEFAULT_GENERATION_LIMITS: GenerationLimits = GenerationLimits {
    thumbnail_timeout: THUMBNAIL_TIMEOUT,
    waveform_timeout: WAVEFORM_TIMEOUT,
    stdout_limit: MAX_ARTIFACT_STDOUT_BYTES,
    stderr_limit: MAX_ARTIFACT_STDERR_BYTES,
    poll_interval: PROCESS_POLL_INTERVAL,
    proxy_poll_interval: PROXY_PROCESS_POLL_INTERVAL,
    proxy_timeout_override: None,
};

#[derive(Default)]
struct ServiceState {
    in_flight: HashMap<ArtifactRequestKey, JobId>,
    sequence: u64,
    subscribers: Vec<mpsc::Sender<MediaArtifactEvent>>,
}

pub struct MediaArtifactService {
    jobs: JobManager,
    cache: CacheStore,
    ffmpeg_executable: PathBuf,
    limits: GenerationLimits,
    state: Arc<Mutex<ServiceState>>,
}

impl MediaArtifactService {
    pub fn new(config: MediaArtifactServiceConfig) -> Result<Self, MediaArtifactServiceInitError> {
        Self::new_with_limits(config, DEFAULT_GENERATION_LIMITS)
    }

    fn new_with_limits(
        config: MediaArtifactServiceConfig,
        limits: GenerationLimits,
    ) -> Result<Self, MediaArtifactServiceInitError> {
        fs::create_dir_all(&config.cache_root).map_err(|_| MediaArtifactServiceInitError)?;
        if !fs::metadata(&config.cache_root)
            .map_err(|_| MediaArtifactServiceInitError)?
            .is_dir()
        {
            return Err(MediaArtifactServiceInitError);
        }
        Ok(Self {
            jobs: JobManager::new(config.job_manager),
            cache: CacheStore::new(config.cache_root, config.cache),
            ffmpeg_executable: config.ffmpeg_executable,
            limits,
            state: Arc::new(Mutex::new(ServiceState::default())),
        })
    }

    pub fn request_thumbnail(
        &self,
        item: &MediaItem,
    ) -> Result<MediaArtifactRequest, MediaArtifactRequestError> {
        self.request(item, CacheArtifactKind::Thumbnail)
    }

    pub fn request_waveform(
        &self,
        item: &MediaItem,
    ) -> Result<MediaArtifactRequest, MediaArtifactRequestError> {
        self.request(item, CacheArtifactKind::Waveform)
    }

    /// Requests a disposable, file-backed video proxy through the shared job service.
    pub fn request_proxy(
        &self,
        item: &MediaItem,
    ) -> Result<MediaArtifactRequest, MediaArtifactRequestError> {
        self.request(item, CacheArtifactKind::Proxy)
    }

    fn request(
        &self,
        item: &MediaItem,
        kind: CacheArtifactKind,
    ) -> Result<MediaArtifactRequest, MediaArtifactRequestError> {
        let applicable = item.metadata().streams().iter().any(|stream| match kind {
            CacheArtifactKind::Thumbnail => matches!(stream, MediaStreamMetadata::Video(_)),
            CacheArtifactKind::Waveform => matches!(stream, MediaStreamMetadata::Audio(_)),
            CacheArtifactKind::Proxy => matches!(stream, MediaStreamMetadata::Video(_)),
        });
        if !applicable {
            return Ok(MediaArtifactRequest {
                media_id: item.id(),
                kind,
                cache_key: None,
                job_id: None,
                state: MediaArtifactRequestState::NotApplicable,
                not_applicable_reason: Some(match kind {
                    CacheArtifactKind::Thumbnail => MediaArtifactNotApplicableReason::NoVideoStream,
                    CacheArtifactKind::Waveform => MediaArtifactNotApplicableReason::NoAudioStream,
                    CacheArtifactKind::Proxy => MediaArtifactNotApplicableReason::NoVideoStream,
                }),
            });
        }

        let path = item
            .source()
            .to_file_path()
            .map_err(|_| MediaArtifactRequestError::SourceUnavailable)?;
        let source = fingerprint_media_source(&path)
            .map_err(|_| MediaArtifactRequestError::SourceUnavailable)?;
        let parameters = parameters_fingerprint(kind);
        let cache_key = CacheKey::new(kind, source, parameters);
        if self.cached_artifact_ready(kind, cache_key)? {
            return Ok(request_view(
                item.id(),
                kind,
                cache_key,
                None,
                MediaArtifactRequestState::Ready,
            ));
        }

        let request_key = ArtifactRequestKey { kind, cache_key };
        let mut state = lock_state(&self.state);
        if let Some(job_id) = state.in_flight.get(&request_key).copied() {
            match self.jobs.snapshot(job_id).map(|snapshot| snapshot.state) {
                Some(JobState::Queued) => {
                    return Ok(request_view(
                        item.id(),
                        kind,
                        cache_key,
                        Some(job_id),
                        MediaArtifactRequestState::Queued,
                    ));
                }
                Some(JobState::Running) => {
                    return Ok(request_view(
                        item.id(),
                        kind,
                        cache_key,
                        Some(job_id),
                        MediaArtifactRequestState::Running,
                    ));
                }
                Some(JobState::Succeeded | JobState::Failed | JobState::Cancelled) | None => {
                    state.in_flight.remove(&request_key);
                }
            }
        }

        // A generator may have populated the cache just before its terminal
        // callback removed the in-flight entry. Recheck under the service lock
        // so that race returns READY instead of submitting duplicate work.
        if self.cached_artifact_ready(kind, cache_key)? {
            return Ok(request_view(
                item.id(),
                kind,
                cache_key,
                None,
                MediaArtifactRequestState::Ready,
            ));
        }

        let failed_with = Arc::new(Mutex::new(None));
        let body_failure = Arc::clone(&failed_with);
        let source_path = path;
        let cache = self.cache.clone();
        let executable = self.ffmpeg_executable.clone();
        let limits = self.limits;
        let proxy_duration = item
            .metadata()
            .streams()
            .iter()
            .find_map(|stream| match stream {
                MediaStreamMetadata::Video(video) => video.duration(),
                MediaStreamMetadata::Audio(_) | MediaStreamMetadata::Other(_) => None,
            })
            .or_else(|| item.metadata().duration());
        let completion_state = Arc::clone(&self.state);
        let completion_job_id = Arc::new(Mutex::new(None));
        let job_id_slot = Arc::clone(&completion_job_id);
        let media_id = item.id();
        let job_kind = match kind {
            CacheArtifactKind::Thumbnail => JobKind::ThumbnailGenerate,
            CacheArtifactKind::Waveform => JobKind::WaveformGenerate,
            CacheArtifactKind::Proxy => JobKind::ProxyGenerate,
        };
        let body = move |context: &JobContext| {
            let result = if kind == CacheArtifactKind::Proxy {
                generate_proxy(
                    &executable,
                    &source_path,
                    cache_key,
                    &cache,
                    context,
                    limits,
                    proxy_duration,
                )
            } else {
                generate_artifact(
                    &executable,
                    &source_path,
                    kind,
                    cache_key,
                    &cache,
                    context,
                    limits,
                )
            };
            let code = result.err();
            *lock_value(&body_failure) = code;
            if code.is_some() {
                Err(JobFailure::new())
            } else {
                Ok(())
            }
        };
        let completion = move |job_state| {
            let error_code = *lock_value(&failed_with);
            publish_completion(
                &completion_state,
                request_key,
                media_id,
                &job_id_slot,
                job_state,
                error_code,
            );
        };
        let job_id = self
            .jobs
            .submit_with_completion(job_kind, body, completion)
            .map_err(map_submit_error)?;
        state.in_flight.insert(request_key, job_id);
        *lock_value(&completion_job_id) = Some(job_id);
        Ok(request_view(
            item.id(),
            kind,
            cache_key,
            Some(job_id),
            MediaArtifactRequestState::Queued,
        ))
    }

    pub fn read_artifact(
        &self,
        kind: CacheArtifactKind,
        cache_key: CacheKey,
    ) -> Result<Option<Vec<u8>>, MediaArtifactRequestError> {
        if kind == CacheArtifactKind::Proxy {
            return Err(MediaArtifactRequestError::ArtifactNotReadable);
        }
        self.cache
            .get(kind, cache_key)
            .map_err(|_| MediaArtifactRequestError::CacheUnavailable)
    }

    fn cached_artifact_ready(
        &self,
        kind: CacheArtifactKind,
        cache_key: CacheKey,
    ) -> Result<bool, MediaArtifactRequestError> {
        if kind == CacheArtifactKind::Proxy {
            return self
                .cache
                .proxy_path_if_present(cache_key)
                .map(|path| path.is_some())
                .map_err(|_| MediaArtifactRequestError::CacheUnavailable);
        }
        match self.cache.get(kind, cache_key) {
            Ok(Some(bytes)) if validate_png(kind, &bytes) => Ok(true),
            Ok(Some(_)) => {
                self.cache
                    .remove(kind, cache_key)
                    .map_err(|_| MediaArtifactRequestError::CacheUnavailable)?;
                Ok(false)
            }
            Ok(None) => Ok(false),
            Err(_) => Err(MediaArtifactRequestError::CacheUnavailable),
        }
    }

    pub fn subscribe_events(&self) -> mpsc::Receiver<MediaArtifactEvent> {
        let (sender, receiver) = mpsc::channel();
        lock_state(&self.state).subscribers.push(sender);
        receiver
    }

    pub fn cancel(&self, job_id: JobId) -> Result<JobCancelOutcome, JobCancelError> {
        self.jobs.cancel(job_id)
    }

    pub fn job_snapshot(&self, job_id: JobId) -> Option<JobSnapshot> {
        self.jobs.snapshot(job_id)
    }

    pub fn shutdown(&self) {
        self.jobs.shutdown();
    }
}

fn request_view(
    media_id: MediaId,
    kind: CacheArtifactKind,
    cache_key: CacheKey,
    job_id: Option<JobId>,
    state: MediaArtifactRequestState,
) -> MediaArtifactRequest {
    MediaArtifactRequest {
        media_id,
        kind,
        cache_key: Some(cache_key),
        job_id,
        state,
        not_applicable_reason: None,
    }
}

fn map_submit_error(error: JobSubmitError) -> MediaArtifactRequestError {
    match error {
        JobSubmitError::QueueFull => MediaArtifactRequestError::QueueFull,
        JobSubmitError::RecordCapacityExceeded => MediaArtifactRequestError::RecordCapacityExceeded,
        JobSubmitError::Shutdown => MediaArtifactRequestError::ServiceShutdown,
    }
}

fn parameters_fingerprint(kind: CacheArtifactKind) -> ParametersFingerprint {
    ParametersFingerprint::from_bytes(match kind {
        CacheArtifactKind::Thumbnail => THUMBNAIL_PROFILE.as_bytes(),
        CacheArtifactKind::Waveform => WAVEFORM_PROFILE.as_bytes(),
        CacheArtifactKind::Proxy => PROXY_PROFILE.as_bytes(),
    })
}

fn generate_artifact(
    executable: &Path,
    source: &Path,
    kind: CacheArtifactKind,
    cache_key: CacheKey,
    cache: &CacheStore,
    context: &JobContext,
    limits: GenerationLimits,
) -> Result<(), MediaArtifactErrorCode> {
    if context.is_cancelled() {
        return Err(MediaArtifactErrorCode::Cancelled);
    }
    let arguments = ffmpeg_arguments(source, kind);
    let timeout = match kind {
        CacheArtifactKind::Thumbnail => limits.thumbnail_timeout,
        CacheArtifactKind::Waveform => limits.waveform_timeout,
        CacheArtifactKind::Proxy => unreachable!("proxy generation is handled above"),
    };
    let bytes = run_ffmpeg(
        executable,
        &arguments,
        context,
        timeout,
        limits.stdout_limit,
        limits.stderr_limit,
        limits.poll_interval,
    )?;
    if context.is_cancelled() {
        return Err(MediaArtifactErrorCode::Cancelled);
    }
    if !validate_png(kind, &bytes) {
        return Err(MediaArtifactErrorCode::InvalidGeneratedArtifact);
    }
    if context.is_cancelled() {
        return Err(MediaArtifactErrorCode::Cancelled);
    }
    cache
        .put(kind, cache_key, &bytes)
        .map_err(|_| MediaArtifactErrorCode::CacheError)
}

fn proxy_timeout(duration: Option<RationalTime>) -> Duration {
    let seconds = match duration {
        Some(duration) if !duration.is_negative() => {
            let numerator = i128::from(duration.numerator());
            let denominator = i128::from(duration.denominator());
            let rounded_up = (numerator + denominator - 1) / denominator;
            (rounded_up * 3).clamp(i128::from(PROXY_MIN_TIMEOUT), i128::from(PROXY_MAX_TIMEOUT))
                as u64
        }
        Some(_) => PROXY_MIN_TIMEOUT,
        None => PROXY_UNKNOWN_DURATION_TIMEOUT.as_secs(),
    };
    Duration::from_secs(seconds)
}

fn generate_proxy(
    executable: &Path,
    source: &Path,
    cache_key: CacheKey,
    cache: &CacheStore,
    context: &JobContext,
    limits: GenerationLimits,
    duration: Option<RationalTime>,
) -> Result<(), MediaArtifactErrorCode> {
    let staging = cache
        .create_proxy_staging_file(cache_key)
        .map_err(|_| MediaArtifactErrorCode::CacheError)?;
    let arguments = proxy_ffmpeg_arguments(source, staging.path());
    let timeout = limits
        .proxy_timeout_override
        .unwrap_or_else(|| proxy_timeout(duration));
    run_ffmpeg_to_file(
        executable,
        &arguments,
        context,
        timeout,
        limits.stderr_limit,
        limits.proxy_poll_interval,
        &staging,
    )?;
    if context.is_cancelled() {
        return Err(MediaArtifactErrorCode::Cancelled);
    }
    if !validate_proxy_file(staging.path()) {
        return Err(MediaArtifactErrorCode::InvalidGeneratedArtifact);
    }
    if context.is_cancelled() {
        return Err(MediaArtifactErrorCode::Cancelled);
    }
    cache
        .commit_proxy(staging)
        .map(|_| ())
        .map_err(|_| MediaArtifactErrorCode::CacheError)
}

fn proxy_ffmpeg_arguments(source: &Path, output: &Path) -> Vec<OsString> {
    vec![
        OsString::from("-v"),
        OsString::from("error"),
        OsString::from("-nostdin"),
        OsString::from("-i"),
        source.as_os_str().to_owned(),
        OsString::from("-map"),
        OsString::from("0:v:0"),
        OsString::from("-vf"),
        OsString::from(
            "setpts=PTS-STARTPTS,scale=w='min(960\\,iw)':h='min(540\\,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2:reset_sar=1,format=yuv420p",
        ),
        OsString::from("-an"),
        OsString::from("-sn"),
        OsString::from("-dn"),
        OsString::from("-map_metadata"),
        OsString::from("-1"),
        OsString::from("-map_metadata:s:v:0"),
        OsString::from("-1"),
        OsString::from("-map_chapters"),
        OsString::from("-1"),
        OsString::from("-c:v"),
        OsString::from("mpeg4"),
        OsString::from("-q:v"),
        OsString::from("6"),
        OsString::from("-g"),
        OsString::from("12"),
        OsString::from("-bf"),
        OsString::from("0"),
        OsString::from("-fps_mode"),
        OsString::from("passthrough"),
        OsString::from("-f"),
        OsString::from("matroska"),
        OsString::from("-y"),
        output.as_os_str().to_owned(),
    ]
}

fn validate_proxy_file(path: &Path) -> bool {
    const EBML_HEADER: &[u8; 4] = b"\x1a\x45\xdf\xa3";
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > PROXY_MAX_ARTIFACT_BYTES
    {
        return false;
    }
    let mut header = [0_u8; 4];
    File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .is_ok()
        && &header == EBML_HEADER
}

fn run_ffmpeg_to_file(
    executable: &Path,
    arguments: &[OsString],
    context: &JobContext,
    timeout: Duration,
    stderr_limit: usize,
    poll_interval: Duration,
    staging: &CacheStagingFile,
) -> Result<(), MediaArtifactErrorCode> {
    if context.is_cancelled() {
        return Err(MediaArtifactErrorCode::Cancelled);
    }
    let mut child = Command::new(executable)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| MediaArtifactErrorCode::BackendUnavailable)?;
    let Some(stderr) = child.stderr.take() else {
        stop_child(&mut child);
        return Err(MediaArtifactErrorCode::GenerationFailed);
    };
    let (sender, receiver) = mpsc::channel();
    let stderr_reader = match spawn_output_reader(OutputKind::Stderr, stderr, stderr_limit, sender)
    {
        Ok(reader) => reader,
        Err(_) => {
            stop_child(&mut child);
            return Err(MediaArtifactErrorCode::GenerationFailed);
        }
    };
    let started = Instant::now();
    let mut status = None;
    let mut stderr_complete = false;
    while status.is_none() || !stderr_complete {
        if context.is_cancelled() {
            stop_child(&mut child);
            let _ = stderr_reader.join();
            return Err(MediaArtifactErrorCode::Cancelled);
        }
        match staging_file_size(staging.path()) {
            Ok(Some(size)) if size > staging.max_bytes() => {
                stop_child(&mut child);
                let _ = stderr_reader.join();
                return Err(MediaArtifactErrorCode::OutputTooLarge);
            }
            Ok(Some(_)) | Ok(None) => {}
            Err(_) => {
                stop_child(&mut child);
                let _ = stderr_reader.join();
                return Err(MediaArtifactErrorCode::GenerationFailed);
            }
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(Some(exited)) => status = Some(exited),
                Ok(None) => {}
                Err(_) => {
                    stop_child(&mut child);
                    let _ = stderr_reader.join();
                    return Err(MediaArtifactErrorCode::GenerationFailed);
                }
            }
        }
        if status.is_some() && stderr_complete {
            break;
        }
        if started.elapsed() >= timeout {
            stop_child(&mut child);
            let _ = stderr_reader.join();
            return Err(MediaArtifactErrorCode::GenerationTimeout);
        }
        match receiver.recv_timeout(poll_interval) {
            Ok(OutputEvent::Complete(OutputKind::Stderr, Ok(_))) => stderr_complete = true,
            Ok(OutputEvent::Complete(_, Err(_))) | Ok(OutputEvent::TooLarge) => {
                stop_child(&mut child);
                let _ = stderr_reader.join();
                return Err(MediaArtifactErrorCode::GenerationFailed);
            }
            Ok(OutputEvent::Complete(OutputKind::Stdout, Ok(_))) => {
                stop_child(&mut child);
                let _ = stderr_reader.join();
                return Err(MediaArtifactErrorCode::GenerationFailed);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) if stderr_complete => {
                thread::sleep(poll_interval);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                stop_child(&mut child);
                let _ = stderr_reader.join();
                return Err(MediaArtifactErrorCode::GenerationFailed);
            }
        }
    }
    if stderr_reader.join().is_err()
        || !status
            .expect("child status is read before success")
            .success()
    {
        return Err(MediaArtifactErrorCode::GenerationFailed);
    }
    Ok(())
}

fn staging_file_size(path: &Path) -> io::Result<Option<u64>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(Some(metadata.len())),
        Ok(_) => Err(io::Error::other("proxy staging path is not a regular file")),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn ffmpeg_arguments(source: &Path, kind: CacheArtifactKind) -> Vec<OsString> {
    let mut arguments = vec![
        OsString::from("-v"),
        OsString::from("error"),
        OsString::from("-nostdin"),
        OsString::from("-i"),
        source.as_os_str().to_owned(),
    ];
    match kind {
        CacheArtifactKind::Thumbnail => arguments.extend([
            OsString::from("-map"),
            OsString::from("0:v:0"),
            OsString::from("-vf"),
            OsString::from(
                "scale=w='min(320\\,iw)':h='min(320\\,ih)':force_original_aspect_ratio=decrease",
            ),
            OsString::from("-frames:v"),
            OsString::from("1"),
        ]),
        CacheArtifactKind::Waveform => arguments.extend([
            OsString::from("-filter_complex"),
            OsString::from("[0:a:0]showwavespic=s=512x96:colors=white:split_channels=0[wave]"),
            OsString::from("-map"),
            OsString::from("[wave]"),
            OsString::from("-frames:v"),
            OsString::from("1"),
        ]),
        CacheArtifactKind::Proxy => unreachable!("proxy output is file-backed"),
    }
    arguments.extend([
        OsString::from("-f"),
        OsString::from("image2pipe"),
        OsString::from("-vcodec"),
        OsString::from("png"),
        OsString::from("pipe:1"),
    ]);
    arguments
}

fn validate_png(kind: CacheArtifactKind, bytes: &[u8]) -> bool {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 33
        || &bytes[..8] != PNG_SIGNATURE
        || u32::from_be_bytes(bytes[8..12].try_into().unwrap()) != 13
        || &bytes[12..16] != b"IHDR"
    {
        return false;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    match kind {
        CacheArtifactKind::Thumbnail => width > 0 && height > 0 && width <= 320 && height <= 320,
        CacheArtifactKind::Waveform => width == 512 && height == 96,
        CacheArtifactKind::Proxy => false,
    }
}

fn run_ffmpeg(
    executable: &Path,
    arguments: &[OsString],
    context: &JobContext,
    timeout: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
    poll_interval: Duration,
) -> Result<Vec<u8>, MediaArtifactErrorCode> {
    if context.is_cancelled() {
        return Err(MediaArtifactErrorCode::Cancelled);
    }
    let mut child = Command::new(executable)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| MediaArtifactErrorCode::BackendUnavailable)?;

    let Some(stdout) = child.stdout.take() else {
        stop_child(&mut child);
        return Err(MediaArtifactErrorCode::GenerationFailed);
    };
    let Some(stderr) = child.stderr.take() else {
        stop_child(&mut child);
        return Err(MediaArtifactErrorCode::GenerationFailed);
    };
    let (sender, receiver) = mpsc::channel();
    let stdout_reader =
        match spawn_output_reader(OutputKind::Stdout, stdout, stdout_limit, sender.clone()) {
            Ok(reader) => reader,
            Err(_) => {
                stop_child(&mut child);
                return Err(MediaArtifactErrorCode::GenerationFailed);
            }
        };
    let stderr_reader = match spawn_output_reader(OutputKind::Stderr, stderr, stderr_limit, sender)
    {
        Ok(reader) => reader,
        Err(_) => {
            stop_child(&mut child);
            let _ = stdout_reader.join();
            return Err(MediaArtifactErrorCode::GenerationFailed);
        }
    };
    let started = Instant::now();
    let mut status = None;
    let mut stdout_bytes = None;
    let mut stderr_bytes = None;
    while status.is_none() || stdout_bytes.is_none() || stderr_bytes.is_none() {
        if context.is_cancelled() {
            stop_child(&mut child);
            let _ = join_readers(stdout_reader, stderr_reader);
            return Err(MediaArtifactErrorCode::Cancelled);
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(Some(exited)) => status = Some(exited),
                Ok(None) => {}
                Err(_) => {
                    stop_child(&mut child);
                    let _ = join_readers(stdout_reader, stderr_reader);
                    return Err(MediaArtifactErrorCode::GenerationFailed);
                }
            }
        }
        if status.is_some() && stdout_bytes.is_some() && stderr_bytes.is_some() {
            break;
        }
        if started.elapsed() >= timeout {
            stop_child(&mut child);
            let _ = join_readers(stdout_reader, stderr_reader);
            return Err(MediaArtifactErrorCode::GenerationTimeout);
        }
        match receiver.recv_timeout(poll_interval) {
            Ok(OutputEvent::Complete(OutputKind::Stdout, Ok(bytes))) => {
                stdout_bytes = Some(bytes);
            }
            Ok(OutputEvent::Complete(OutputKind::Stderr, Ok(bytes))) => {
                stderr_bytes = Some(bytes);
            }
            Ok(OutputEvent::TooLarge) => {
                stop_child(&mut child);
                let _ = join_readers(stdout_reader, stderr_reader);
                return Err(MediaArtifactErrorCode::OutputTooLarge);
            }
            Ok(OutputEvent::Complete(_, Err(_))) => {
                stop_child(&mut child);
                let _ = join_readers(stdout_reader, stderr_reader);
                return Err(MediaArtifactErrorCode::GenerationFailed);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if stdout_bytes.is_some() && stderr_bytes.is_some() {
                    thread::sleep(poll_interval);
                    continue;
                }
                stop_child(&mut child);
                let _ = join_readers(stdout_reader, stderr_reader);
                return Err(MediaArtifactErrorCode::GenerationFailed);
            }
        }
    }
    if !join_readers(stdout_reader, stderr_reader) {
        return Err(MediaArtifactErrorCode::GenerationFailed);
    }
    if !status
        .expect("child status is read before success")
        .success()
    {
        return Err(MediaArtifactErrorCode::GenerationFailed);
    }
    Ok(stdout_bytes.expect("stdout is read before success"))
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[derive(Clone, Copy)]
enum OutputKind {
    Stdout,
    Stderr,
}

enum OutputEvent {
    Complete(OutputKind, Result<Vec<u8>, io::Error>),
    TooLarge,
}

fn spawn_output_reader<R: Read + Send + 'static>(
    kind: OutputKind,
    mut reader: R,
    limit: usize,
    sender: mpsc::Sender<OutputEvent>,
) -> io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("or-media-artifact-pipe".to_owned())
        .spawn(move || match read_bounded(&mut reader, limit) {
            Ok(bytes) => {
                let _ = sender.send(OutputEvent::Complete(kind, Ok(bytes)));
            }
            Err(BoundedReadError::TooLarge) => {
                let _ = sender.send(OutputEvent::TooLarge);
            }
            Err(BoundedReadError::Io(error)) => {
                let _ = sender.send(OutputEvent::Complete(kind, Err(error)));
            }
        })
}

enum BoundedReadError {
    TooLarge,
    Io(io::Error),
}

fn read_bounded(reader: &mut impl Read, limit: usize) -> Result<Vec<u8>, BoundedReadError> {
    let mut output = Vec::with_capacity(limit.min(8192));
    let mut buffer = [0_u8; 8192];
    loop {
        let remaining = limit
            .checked_add(1)
            .and_then(|bound| bound.checked_sub(output.len()))
            .ok_or(BoundedReadError::TooLarge)?;
        let read_length = remaining.min(buffer.len());
        let count = match reader.read(&mut buffer[..read_length]) {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(BoundedReadError::Io(error)),
        };
        if count == 0 {
            return Ok(output);
        }
        output.extend_from_slice(&buffer[..count]);
        if output.len() > limit {
            return Err(BoundedReadError::TooLarge);
        }
    }
}

fn join_readers(stdout: JoinHandle<()>, stderr: JoinHandle<()>) -> bool {
    let stdout_ok = stdout.join().is_ok();
    let stderr_ok = stderr.join().is_ok();
    stdout_ok && stderr_ok
}

fn publish_completion(
    state: &Arc<Mutex<ServiceState>>,
    request_key: ArtifactRequestKey,
    media_id: MediaId,
    job_id_slot: &Arc<Mutex<Option<JobId>>>,
    job_state: JobState,
    error_code: Option<MediaArtifactErrorCode>,
) {
    let mut state = lock_state(state);
    let Some(job_id) = *lock_value(job_id_slot) else {
        return;
    };
    if state.in_flight.get(&request_key) == Some(&job_id) {
        state.in_flight.remove(&request_key);
    }
    let (event_state, event_error) = match job_state {
        JobState::Succeeded => (MediaArtifactEventState::Succeeded, None),
        JobState::Failed => (
            MediaArtifactEventState::Failed,
            Some(error_code.unwrap_or(MediaArtifactErrorCode::GenerationFailed)),
        ),
        JobState::Cancelled => (
            MediaArtifactEventState::Cancelled,
            Some(MediaArtifactErrorCode::Cancelled),
        ),
        JobState::Queued | JobState::Running => return,
    };
    let Some(sequence) = state.sequence.checked_add(1) else {
        return;
    };
    state.sequence = sequence;
    let event = MediaArtifactEvent {
        sequence,
        media_id,
        kind: request_key.kind,
        cache_key: request_key.cache_key,
        job_id,
        state: event_state,
        error_code: event_error,
    };
    state
        .subscribers
        .retain(|subscriber| subscriber.send(event.clone()).is_ok());
}

fn lock_state(state: &Mutex<ServiceState>) -> MutexGuard<'_, ServiceState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lock_value<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{AudioStreamMetadata, MediaMetadata, MediaSourceRef, VideoStreamMetadata};
    use std::{
        fs::OpenOptions, io::Write, num::NonZeroU32, str::FromStr, sync::OnceLock, time::SystemTime,
    };
    use uuid::Uuid;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("or-media-artifacts-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn media(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, bytes).unwrap();
            path
        }

        fn output(&self, name: &str, bytes: &[u8]) {
            fs::write(self.0.join(name), bytes).unwrap();
        }

        fn process_count(&self) -> usize {
            fs::read(self.0.join("process-count"))
                .map(|bytes| bytes.len())
                .unwrap_or(0)
        }

        fn proxy_staging_count(&self) -> usize {
            let root = self.0.join("cache").join("proxy");
            if !root.exists() {
                return 0;
            }
            let mut pending = vec![root];
            let mut count = 0;
            while let Some(directory) = pending.pop() {
                for entry in fs::read_dir(directory).unwrap() {
                    let entry = entry.unwrap();
                    if entry.file_type().unwrap().is_dir() {
                        pending.push(entry.path());
                    } else if entry
                        .file_name()
                        .to_string_lossy()
                        .contains(".or-proxy-tmp-")
                    {
                        count += 1;
                    }
                }
            }
            count
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fake_executable() -> &'static PathBuf {
        static EXECUTABLE: OnceLock<PathBuf> = OnceLock::new();
        EXECUTABLE.get_or_init(|| {
            let root =
                std::env::temp_dir().join(format!("or-media-artifact-stub-{}", std::process::id()));
            fs::create_dir_all(&root).unwrap();
            let source = root.join("ffmpeg_stub.rs");
            let executable = root.join(if cfg!(windows) {
                "ffmpeg_stub.exe"
            } else {
                "ffmpeg_stub"
            });
            fs::write(&source, FAKE_FFMPEG_SOURCE).unwrap();
            let status = std::process::Command::new("rustc")
                .arg("--edition=2021")
                .arg(&source)
                .arg("-o")
                .arg(&executable)
                .status()
                .expect("rustc must be available during Rust tests");
            assert!(status.success(), "could not build fake ffmpeg executable");
            executable
        })
    }

    const FAKE_FFMPEG_SOURCE: &str = r#"
use std::{
    env, fs::OpenOptions, io::{self, Write}, path::PathBuf,
    thread, time::Duration,
};
fn main() {
    let args = env::args_os().collect::<Vec<_>>();
    let input_index = args.iter().position(|arg| arg == "-i").unwrap_or_else(|| std::process::exit(71));
    let input = PathBuf::from(args.get(input_index + 1).unwrap_or_else(|| std::process::exit(72)));
    if !input.file_name().is_some_and(|name| name.to_string_lossy().ends_with("path with spaces-媒体.mkv")) {
        std::process::exit(73);
    }
    let parent = input.parent().unwrap();
    let mut count = OpenOptions::new().create(true).append(true).open(parent.join("process-count")).unwrap();
    count.write_all(b"x").unwrap();
    let name = input.file_name().unwrap().to_string_lossy();
    if name.starts_with("block-") || name.starts_with("timeout-") {
        loop { thread::sleep(Duration::from_secs(60)); }
    }
    if name.starts_with("failure-") {
        eprintln!("controlled stub failure");
        std::process::exit(1);
    }
    let proxy = args.windows(2).any(|pair| pair[0] == "-fps_mode" && pair[1] == "passthrough");
    if proxy {
        let expected_scale = "setpts=PTS-STARTPTS,scale=w='min(960\\,iw)':h='min(540\\,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2:reset_sar=1,format=yuv420p";
        let has = |key: &str, value: &str| args.windows(2).any(|pair| pair[0] == key && pair[1] == value);
        if !has("-map", "0:v:0") || !has("-vf", expected_scale)
            || !has("-c:v", "mpeg4") || !has("-q:v", "6") || !has("-g", "12")
            || !has("-bf", "0") || !has("-f", "matroska")
            || !has("-map_metadata", "-1") || !has("-map_chapters", "-1")
            || !args.iter().any(|arg| arg == "-an") || !args.iter().any(|arg| arg == "-sn")
            || !args.iter().any(|arg| arg == "-dn") || args.iter().any(|arg| arg == "-r")
        {
            std::process::exit(79);
        }
        let output = PathBuf::from(args.last().unwrap());
        if output.extension().is_none_or(|extension| extension != "mkv") {
            std::process::exit(80);
        }
        if name.starts_with("proxy-grow-") {
            let mut file = OpenOptions::new().write(true).open(output).unwrap();
            file.write_all(&vec![b'x'; 4096]).unwrap();
            file.flush().unwrap();
            loop { thread::sleep(Duration::from_secs(60)); }
        }
        let output_name = if name.starts_with("oversized-") { "proxy-oversized-output" }
            else if name.starts_with("malformed-") { "proxy-malformed-output" }
            else { "proxy-output" };
        let bytes = std::fs::read(parent.join(output_name)).unwrap_or_else(|_| std::process::exit(81));
        std::fs::write(output, bytes).unwrap_or_else(|_| std::process::exit(82));
        return;
    }
    if !args.windows(2).any(|pair| pair[0] == "-frames:v" && pair[1] == "1")
        || !args.windows(2).any(|pair| pair[0] == "-f" && pair[1] == "image2pipe")
        || !args.windows(2).any(|pair| pair[0] == "-vcodec" && pair[1] == "png") {
        std::process::exit(74);
    }
    if name.starts_with("stderr-oversized-") {
        io::stderr().write_all(&vec![b'x'; 8192]).unwrap();
        return;
    }
    let waveform = args.iter().any(|arg| arg.to_string_lossy().contains("showwavespic"));
    if waveform {
        if !args.iter().any(|arg| arg.to_string_lossy().contains("[0:a:0]showwavespic=s=512x96:colors=white:split_channels=0")) {
            std::process::exit(75);
        }
    } else if !args.windows(2).any(|pair| pair[0] == "-map" && pair[1] == "0:v:0")
        || !args.iter().any(|arg| arg == "scale=w='min(320\\,iw)':h='min(320\\,ih)':force_original_aspect_ratio=decrease") {
        std::process::exit(76);
    }
    let output_name = if name.starts_with("oversized-") { "oversized-output" }
        else if name.starts_with("malformed-") { "malformed-output" }
        else if waveform { "waveform-output" }
        else { "thumbnail-output" };
    let bytes = std::fs::read(parent.join(output_name)).unwrap_or_else(|_| std::process::exit(77));
    io::stdout().write_all(&bytes).unwrap_or_else(|_| std::process::exit(78));
}
"#;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let row_bytes = usize::try_from(width).unwrap() + 1;
        let mut raw = Vec::with_capacity(row_bytes * usize::try_from(height).unwrap());
        for _ in 0..height {
            raw.push(0);
            raw.resize(raw.len() + usize::try_from(width).unwrap(), 0);
        }
        let mut zlib = vec![0x78, 0x01];
        for (index, block) in raw.chunks(65_535).enumerate() {
            let final_block = index + 1 == raw.chunks(65_535).len();
            zlib.push(u8::from(final_block));
            let length = u16::try_from(block.len()).unwrap();
            zlib.extend_from_slice(&length.to_le_bytes());
            zlib.extend_from_slice(&(!length).to_le_bytes());
            zlib.extend_from_slice(block);
        }
        let mut a = 1_u32;
        let mut b = 0_u32;
        for byte in &raw {
            a = (a + u32::from(*byte)) % 65_521;
            b = (b + a) % 65_521;
        }
        zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());

        let mut output = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut header = Vec::with_capacity(13);
        header.extend_from_slice(&width.to_be_bytes());
        header.extend_from_slice(&height.to_be_bytes());
        header.extend_from_slice(&[8, 0, 0, 0, 0]);
        png_chunk(&mut output, b"IHDR", &header);
        png_chunk(&mut output, b"IDAT", &zlib);
        png_chunk(&mut output, b"IEND", &[]);
        output
    }

    fn png_chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        output.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        output.extend_from_slice(kind);
        output.extend_from_slice(data);
        let crc = crc32(&[kind.as_slice(), data].concat());
        output.extend_from_slice(&crc.to_be_bytes());
    }

    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = !0_u32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    fn service(
        directory: &TestDirectory,
        limits: GenerationLimits,
        max_workers: usize,
        queue_capacity: usize,
        max_records: usize,
        max_entry_bytes: u64,
        max_total_bytes: u64,
    ) -> MediaArtifactService {
        MediaArtifactService::new_with_limits(
            MediaArtifactServiceConfig::new(
                directory.0.join("cache"),
                JobManagerConfig::new(max_workers, queue_capacity, max_records).unwrap(),
                CacheStoreConfig::new(max_entry_bytes, max_total_bytes).unwrap(),
                fake_executable(),
            ),
            limits,
        )
        .unwrap()
    }

    const TEST_LIMITS: GenerationLimits = GenerationLimits {
        thumbnail_timeout: Duration::from_secs(2),
        waveform_timeout: Duration::from_secs(2),
        stdout_limit: 1024 * 1024,
        stderr_limit: 4096,
        poll_interval: Duration::from_millis(10),
        proxy_poll_interval: Duration::from_millis(10),
        proxy_timeout_override: None,
    };

    fn item(path: &Path, id: u128, video: bool, audio: bool) -> MediaItem {
        let mut streams = Vec::new();
        if video {
            streams.push(MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                0,
                Some("h264".to_owned()),
                NonZeroU32::new(16).unwrap(),
                NonZeroU32::new(9).unwrap(),
                None,
                None,
                None,
            )));
        }
        if audio {
            streams.push(MediaStreamMetadata::Audio(AudioStreamMetadata::from_probe(
                u32::try_from(streams.len()).unwrap(),
                Some("pcm_s16le".to_owned()),
                NonZeroU32::new(48_000),
                NonZeroU32::new(2),
                Some("stereo".to_owned()),
                None,
            )));
        }
        let source =
            MediaSourceRef::local_file(url::Url::from_file_path(path).unwrap().as_str()).unwrap();
        MediaItem::new(
            MediaId::from_str(&format!("00000000-0000-4000-8000-{id:012x}")).unwrap(),
            source,
            MediaMetadata::from_probe(vec!["matroska".to_owned()], None, 0, streams),
        )
        .unwrap()
    }

    fn wait_for_state(service: &MediaArtifactService, job_id: JobId, expected: JobState) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while service.job_snapshot(job_id).map(|snapshot| snapshot.state) != Some(expected) {
            assert!(
                Instant::now() < deadline,
                "artifact job did not reach {expected:?}"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_for_process_count(directory: &TestDirectory, expected: usize) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while directory.process_count() < expected {
            assert!(
                Instant::now() < deadline,
                "fake ffmpeg process did not start"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_event(receiver: &mpsc::Receiver<MediaArtifactEvent>) -> MediaArtifactEvent {
        receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("artifact event was not published")
    }

    #[test]
    fn source_fingerprint_is_stable_and_small_file_content_is_hashed() {
        let directory = TestDirectory::new();
        let path = directory.media("source.bin", b"same-length-a");
        let first = fingerprint_media_source(&path).unwrap();
        assert_eq!(first, fingerprint_media_source(&path).unwrap());
        let mut file = File::open(&path).unwrap();
        let content_only_before = fingerprint_open_file(&mut file, 13, None).unwrap();
        fs::write(&path, b"same-length-b").unwrap();
        let mut file = File::open(&path).unwrap();
        let content_only_after = fingerprint_open_file(&mut file, 13, None).unwrap();
        assert_ne!(content_only_before, content_only_after);
        assert_ne!(first, fingerprint_media_source(&path).unwrap());
    }

    #[test]
    fn source_fingerprint_samples_a_changed_large_file_region() {
        let directory = TestDirectory::new();
        let path = directory.media(
            "large.bin",
            &vec![0_u8; SOURCE_FINGERPRINT_MAX_TOTAL_SAMPLE_BYTES + 1024],
        );
        let size = fs::metadata(&path).unwrap().len();
        let mut file = File::open(&path).unwrap();
        let before = fingerprint_open_file(&mut file, size, None).unwrap();
        let offset = size / 3 + 17;
        let mut file = OpenOptions::new().write(true).open(&path).unwrap();
        file.seek(SeekFrom::Start(offset)).unwrap();
        file.write_all(&[1]).unwrap();
        let mut file = File::open(&path).unwrap();
        let after = fingerprint_open_file(&mut file, size, None).unwrap();
        assert_ne!(before, after);
    }

    #[test]
    fn source_fingerprint_sampling_stays_bounded_for_large_files() {
        let size = 16 * 1024 * 1024 * 1024_u64;
        let windows = sample_windows(size);
        assert!(windows.len() <= 4);
        assert_eq!(windows[0].0, 0);
        assert_eq!(windows[1].0, size / 3);
        assert_eq!(windows[2].0, (size / 3) * 2);
        assert_eq!(
            windows[3].0,
            size - SOURCE_FINGERPRINT_SAMPLE_WINDOW_BYTES as u64
        );
        assert!(
            windows
                .iter()
                .all(|(_, length)| *length <= SOURCE_FINGERPRINT_SAMPLE_WINDOW_BYTES)
        );
        assert!(
            windows.iter().map(|(_, length)| length).sum::<usize>()
                <= SOURCE_FINGERPRINT_MAX_TOTAL_SAMPLE_BYTES
        );
        assert_eq!(
            sample_windows(SOURCE_FINGERPRINT_MAX_TOTAL_SAMPLE_BYTES as u64)
                .iter()
                .map(|(_, length)| length)
                .sum::<usize>(),
            SOURCE_FINGERPRINT_MAX_TOTAL_SAMPLE_BYTES
        );
    }

    #[test]
    fn source_fingerprint_rejects_directories_with_a_structured_error() {
        let directory = TestDirectory::new();
        assert_eq!(
            fingerprint_media_source(&directory.0),
            Err(SourceFingerprintError::NotRegularFile)
        );
        assert_eq!(
            fingerprint_media_source(&directory.0.join("missing media file.mkv")),
            Err(SourceFingerprintError::NotFound)
        );
    }

    #[test]
    fn profile_keys_use_distinct_locked_descriptors() {
        let source = SourceFingerprint::from_bytes(b"same source");
        let thumbnail = CacheKey::new(
            CacheArtifactKind::Thumbnail,
            source,
            parameters_fingerprint(CacheArtifactKind::Thumbnail),
        );
        let waveform = CacheKey::new(
            CacheArtifactKind::Waveform,
            source,
            parameters_fingerprint(CacheArtifactKind::Waveform),
        );
        let proxy = CacheKey::new(
            CacheArtifactKind::Proxy,
            source,
            parameters_fingerprint(CacheArtifactKind::Proxy),
        );
        assert_ne!(thumbnail, waveform);
        assert_ne!(thumbnail, proxy);
        assert_ne!(waveform, proxy);
        assert!(THUMBNAIL_PROFILE.starts_with("opencut-reinforced-thumbnail-v1\n"));
        assert!(WAVEFORM_PROFILE.starts_with("opencut-reinforced-waveform-v1\n"));
        assert_eq!(
            thumbnail.to_hex(),
            "139fa366d3dd700c0f5707e0a5c2c1fbf0bf8863eaf7055957777286eac35a4f"
        );
        assert_eq!(
            waveform.to_hex(),
            "a12589b4ad1b69143d795c1c5dfa8b85ffd5e76934b8b5a52ca2ccc128cef8d8"
        );
        assert_eq!(
            PROXY_PROFILE,
            "opencut-reinforced-proxy-v1\ncontainer=matroska\nvideo_codec=mpeg4\nmax_width=960\nmax_height=540\nupscale=false\nsquare_pixels=true\npixel_format=yuv420p\nqscale=6\ngop=12\nbframes=0\nfps_mode=passthrough\npts=start_at_zero\naudio=none\nmetadata=none"
        );
        assert_eq!(
            proxy.to_hex(),
            "efcb593b6ef519f3d23e26f38059e72158f87e08268c95baf3ba096df455b7d3"
        );
        assert_eq!(
            CacheKey::new(
                CacheArtifactKind::Proxy,
                source,
                parameters_fingerprint(CacheArtifactKind::Proxy),
            ),
            CacheKey::new(
                CacheArtifactKind::Proxy,
                source,
                ParametersFingerprint::from_bytes(PROXY_PROFILE.as_bytes()),
            )
        );
        let arguments = proxy_ffmpeg_arguments(Path::new("source.mkv"), Path::new("stage.mkv"));
        let argument_text = arguments
            .iter()
            .map(|argument| argument.to_string_lossy())
            .collect::<Vec<_>>();
        for pair in [
            ["-map", "0:v:0"],
            ["-c:v", "mpeg4"],
            ["-q:v", "6"],
            ["-g", "12"],
            ["-bf", "0"],
            ["-fps_mode", "passthrough"],
            ["-f", "matroska"],
            ["-map_metadata", "-1"],
            ["-map_chapters", "-1"],
        ] {
            assert!(argument_text.windows(2).any(|args| args == pair));
        }
        assert!(!argument_text.iter().any(|argument| *argument == "-r"));
        assert_eq!(proxy_timeout(None), Duration::from_secs(1800));
        assert_eq!(
            proxy_timeout(Some(RationalTime::ZERO)),
            Duration::from_secs(120)
        );
        assert_eq!(
            proxy_timeout(Some(RationalTime::new(50, 1).unwrap())),
            Duration::from_secs(150)
        );
        assert_eq!(
            proxy_timeout(Some(RationalTime::new(3000, 1).unwrap())),
            Duration::from_secs(7200)
        );
        assert_eq!(
            proxy_timeout(Some(RationalTime::new(i64::MAX, 1).unwrap())),
            Duration::from_secs(7200)
        );
    }

    #[test]
    fn thumbnail_and_waveform_generators_cache_pngs_and_emit_ordered_events() {
        let directory = TestDirectory::new();
        let video_path = directory.media("video path with spaces-媒体.mkv", b"video");
        let audio_path = directory.media("audio path with spaces-媒体.mkv", b"audio");
        directory.output("thumbnail-output", &png(320, 180));
        directory.output("waveform-output", &png(512, 96));
        let service = service(
            &directory,
            TEST_LIMITS,
            2,
            4,
            8,
            1024 * 1024,
            2 * 1024 * 1024,
        );
        let events = service.subscribe_events();
        let thumbnail = service
            .request_thumbnail(&item(&video_path, 1, true, false))
            .unwrap();
        let waveform = service
            .request_waveform(&item(&audio_path, 2, false, true))
            .unwrap();
        let thumbnail_key = thumbnail.cache_key.unwrap();
        let waveform_key = waveform.cache_key.unwrap();
        let mut terminal = [wait_event(&events), wait_event(&events)];
        terminal.sort_by_key(|event| event.sequence);
        assert_eq!(
            terminal
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert!(
            terminal
                .iter()
                .all(|event| event.state == MediaArtifactEventState::Succeeded)
        );
        let thumbnail_bytes = service
            .read_artifact(CacheArtifactKind::Thumbnail, thumbnail_key)
            .unwrap()
            .unwrap();
        let waveform_bytes = service
            .read_artifact(CacheArtifactKind::Waveform, waveform_key)
            .unwrap()
            .unwrap();
        assert!(validate_png(CacheArtifactKind::Thumbnail, &thumbnail_bytes));
        assert!(validate_png(CacheArtifactKind::Waveform, &waveform_bytes));
        assert_eq!(
            u32::from_be_bytes(waveform_bytes[16..20].try_into().unwrap()),
            512
        );
        assert_eq!(
            u32::from_be_bytes(waveform_bytes[20..24].try_into().unwrap()),
            96
        );
    }

    #[test]
    fn cache_hit_does_not_start_a_second_generation_job() {
        let directory = TestDirectory::new();
        let path = directory.media("cache path with spaces-媒体.mkv", b"cache");
        directory.output("thumbnail-output", &png(32, 18));
        let service = service(
            &directory,
            TEST_LIMITS,
            1,
            2,
            4,
            1024 * 1024,
            2 * 1024 * 1024,
        );
        let item = item(&path, 3, true, false);
        let events = service.subscribe_events();
        let first = service.request_thumbnail(&item).unwrap();
        wait_event(&events);
        let second = service.request_thumbnail(&item).unwrap();
        assert_eq!(first.cache_key, second.cache_key);
        assert_eq!(second.state, MediaArtifactRequestState::Ready);
        assert_eq!(second.job_id, None);
        assert_eq!(directory.process_count(), 1);
    }

    #[test]
    fn an_evicted_preview_is_generated_again_on_the_next_request() {
        let directory = TestDirectory::new();
        let first_path = directory.media("first path with spaces-媒体.mkv", b"source one");
        let second_path = directory.media(
            "second path with spaces-媒体.mkv",
            b"source two has a different length",
        );
        let output = png(32, 18);
        directory.output("thumbnail-output", &output);
        let service = service(
            &directory,
            TEST_LIMITS,
            1,
            4,
            8,
            1024 * 1024,
            output.len() as u64,
        );
        let events = service.subscribe_events();
        let first_item = item(&first_path, 21, true, false);
        let second_item = item(&second_path, 22, true, false);

        let first = service.request_thumbnail(&first_item).unwrap();
        assert_eq!(
            wait_event(&events).state,
            MediaArtifactEventState::Succeeded
        );
        let second = service.request_thumbnail(&second_item).unwrap();
        assert_eq!(
            wait_event(&events).state,
            MediaArtifactEventState::Succeeded
        );
        assert!(
            service
                .read_artifact(CacheArtifactKind::Thumbnail, first.cache_key.unwrap())
                .unwrap()
                .is_none()
        );
        assert!(
            service
                .read_artifact(CacheArtifactKind::Thumbnail, second.cache_key.unwrap())
                .unwrap()
                .is_some()
        );

        let regenerated = service.request_thumbnail(&first_item).unwrap();
        assert!(matches!(
            regenerated.state,
            MediaArtifactRequestState::Queued | MediaArtifactRequestState::Running
        ));
        assert_eq!(
            wait_event(&events).state,
            MediaArtifactEventState::Succeeded
        );
        assert!(
            service
                .read_artifact(CacheArtifactKind::Thumbnail, first.cache_key.unwrap())
                .unwrap()
                .is_some()
        );
        assert!(
            service
                .read_artifact(CacheArtifactKind::Thumbnail, second.cache_key.unwrap())
                .unwrap()
                .is_none()
        );
        assert_eq!(directory.process_count(), 3);
    }

    #[test]
    fn in_flight_requests_share_one_job_and_running_cancel_cleans_child() {
        let directory = TestDirectory::new();
        let path = directory.media("block- path with spaces-媒体.mkv", b"block");
        let queued_path = directory.media("queued path with spaces-媒体.mkv", b"queued");
        let service = service(
            &directory,
            TEST_LIMITS,
            1,
            2,
            4,
            1024 * 1024,
            2 * 1024 * 1024,
        );
        let media_item = item(&path, 4, true, false);
        let events = service.subscribe_events();
        let first = service.request_thumbnail(&media_item).unwrap();
        let job_id = first.job_id.unwrap();
        wait_for_state(&service, job_id, JobState::Running);
        let second = service
            .request_thumbnail(&item(&path, 19, true, false))
            .unwrap();
        assert_eq!(second.job_id, Some(job_id));
        wait_for_process_count(&directory, 1);
        let queued = service
            .request_thumbnail(&item(&queued_path, 17, true, false))
            .unwrap();
        assert_eq!(queued.state, MediaArtifactRequestState::Queued);
        let queued_job_id = queued.job_id.unwrap();
        service.cancel(queued_job_id).unwrap();
        let queued_event = wait_event(&events);
        assert_eq!(queued_event.job_id, queued_job_id);
        assert_eq!(queued_event.sequence, 1);
        assert_eq!(queued_event.state, MediaArtifactEventState::Cancelled);
        service.cancel(job_id).unwrap();
        let event = wait_event(&events);
        assert_eq!(event.sequence, 2);
        assert_eq!(event.job_id, job_id);
        assert_eq!(event.state, MediaArtifactEventState::Cancelled);
        assert_eq!(event.error_code, Some(MediaArtifactErrorCode::Cancelled));
        wait_for_state(&service, job_id, JobState::Cancelled);
        assert!(
            service
                .read_artifact(CacheArtifactKind::Thumbnail, first.cache_key.unwrap())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn distinct_artifact_profiles_schedule_distinct_jobs() {
        let directory = TestDirectory::new();
        let path = directory.media("dual path with spaces-媒体.mkv", b"dual");
        directory.output("thumbnail-output", &png(32, 18));
        directory.output("waveform-output", &png(512, 96));
        let service = service(
            &directory,
            TEST_LIMITS,
            2,
            4,
            8,
            1024 * 1024,
            2 * 1024 * 1024,
        );
        let item = item(&path, 5, true, true);
        let events = service.subscribe_events();
        let thumbnail = service.request_thumbnail(&item).unwrap();
        let waveform = service.request_waveform(&item).unwrap();
        assert_ne!(thumbnail.cache_key, waveform.cache_key);
        assert_ne!(thumbnail.job_id, waveform.job_id);
        wait_for_state(&service, thumbnail.job_id.unwrap(), JobState::Succeeded);
        wait_for_state(&service, waveform.job_id.unwrap(), JobState::Succeeded);
        assert_eq!(wait_event(&events).sequence, 1);
        assert_eq!(wait_event(&events).sequence, 2);
    }

    #[test]
    fn unsupported_stream_requests_do_not_create_jobs_or_spawn_ffmpeg() {
        let directory = TestDirectory::new();
        let audio_path = directory.media("audio-only path with spaces-媒体.mkv", b"audio");
        let video_path = directory.media("video-only path with spaces-媒体.mkv", b"video");
        let service = service(
            &directory,
            TEST_LIMITS,
            1,
            2,
            4,
            1024 * 1024,
            2 * 1024 * 1024,
        );
        let no_video = service
            .request_thumbnail(&item(&audio_path, 6, false, true))
            .unwrap();
        let no_audio = service
            .request_waveform(&item(&video_path, 7, true, false))
            .unwrap();
        let no_proxy_video = service
            .request_proxy(&item(&audio_path, 23, false, true))
            .unwrap();
        assert_eq!(
            no_video.not_applicable_reason,
            Some(MediaArtifactNotApplicableReason::NoVideoStream)
        );
        assert_eq!(
            no_audio.not_applicable_reason,
            Some(MediaArtifactNotApplicableReason::NoAudioStream)
        );
        assert_eq!(no_proxy_video.kind, CacheArtifactKind::Proxy);
        assert_eq!(
            no_proxy_video.state,
            MediaArtifactRequestState::NotApplicable
        );
        assert_eq!(
            no_proxy_video.not_applicable_reason,
            Some(MediaArtifactNotApplicableReason::NoVideoStream)
        );
        assert_eq!(service.jobs.record_count(), 0);
        assert_eq!(directory.process_count(), 0);
    }

    #[test]
    fn proxy_generation_is_file_backed_and_a_hit_starts_no_new_job() {
        let directory = TestDirectory::new();
        let path = directory.media("proxy cache path with spaces-媒体.mkv", b"source");
        directory.output("proxy-output", b"\x1a\x45\xdf\xa3synthetic Matroska proxy");
        let service = service(
            &directory,
            TEST_LIMITS,
            1,
            2,
            4,
            8 * 1024 * 1024,
            16 * 1024 * 1024,
        );
        let events = service.subscribe_events();
        let media_item = item(&path, 24, true, true);
        let first = service.request_proxy(&media_item).unwrap();
        assert_eq!(first.kind, CacheArtifactKind::Proxy);
        assert_eq!(first.state, MediaArtifactRequestState::Queued);
        let job_id = first.job_id.unwrap();
        let event = wait_event(&events);
        assert_eq!(event.kind, CacheArtifactKind::Proxy);
        assert_eq!(event.job_id, job_id);
        assert_eq!(event.state, MediaArtifactEventState::Succeeded);
        assert_eq!(event.error_code, None);
        wait_for_state(&service, job_id, JobState::Succeeded);

        let key = first.cache_key.unwrap();
        let hex = key.to_hex();
        let final_path = service
            .cache
            .root()
            .join("proxy")
            .join(&hex[..2])
            .join(format!("{hex}.mkv"));
        assert_eq!(final_path.extension().unwrap(), "mkv");
        assert!(final_path.is_file());
        assert_eq!(
            service.cache.proxy_path_if_present(key).unwrap(),
            Some(final_path)
        );
        assert_eq!(
            service.read_artifact(CacheArtifactKind::Proxy, key),
            Err(MediaArtifactRequestError::ArtifactNotReadable)
        );
        let second = service.request_proxy(&media_item).unwrap();
        assert_eq!(second.state, MediaArtifactRequestState::Ready);
        assert_eq!(second.cache_key, Some(key));
        assert_eq!(second.job_id, None);
        assert_eq!(directory.process_count(), 1);
    }

    #[test]
    fn proxy_in_flight_requests_share_one_job_and_cancel_removes_staging() {
        let directory = TestDirectory::new();
        let path = directory.media("block-proxy path with spaces-媒体.mkv", b"blocking source");
        let service = service(&directory, TEST_LIMITS, 1, 2, 4, 1024, 2048);
        let events = service.subscribe_events();
        let first = service
            .request_proxy(&item(&path, 25, true, false))
            .unwrap();
        let job_id = first.job_id.unwrap();
        wait_for_state(&service, job_id, JobState::Running);
        wait_for_process_count(&directory, 1);
        let second = service
            .request_proxy(&item(&path, 26, true, false))
            .unwrap();
        assert_eq!(second.job_id, Some(job_id));
        assert_eq!(second.cache_key, first.cache_key);
        assert_eq!(directory.process_count(), 1);

        service.cancel(job_id).unwrap();
        let event = wait_event(&events);
        assert_eq!(event.state, MediaArtifactEventState::Cancelled);
        assert_eq!(event.error_code, Some(MediaArtifactErrorCode::Cancelled));
        wait_for_state(&service, job_id, JobState::Cancelled);
        assert_eq!(directory.proxy_staging_count(), 0);
        assert_eq!(
            service
                .cache
                .proxy_path_if_present(first.cache_key.unwrap())
                .unwrap(),
            None
        );
    }

    #[test]
    fn proxy_timeout_oversize_failure_and_invalid_container_clean_staging() {
        let directory = TestDirectory::new();
        let timeout_path = directory.media("timeout-proxy path with spaces-媒体.mkv", b"timeout");
        let oversized_path = directory.media("proxy-grow- path with spaces-媒体.mkv", b"large");
        let failed_path = directory.media("failure-proxy path with spaces-媒体.mkv", b"failure");
        let malformed_path = directory.media("malformed-proxy path with spaces-媒体.mkv", b"bad");
        directory.output("proxy-output", b"\x1a\x45\xdf\xa3valid synthetic proxy");
        directory.output("proxy-malformed-output", b"not a Matroska container");
        let limits = GenerationLimits {
            proxy_timeout_override: Some(Duration::from_secs(1)),
            ..TEST_LIMITS
        };
        let service = service(&directory, limits, 4, 8, 8, 1024 * 1024, 128);
        let events = service.subscribe_events();
        let requests = [
            service
                .request_proxy(&item(&timeout_path, 27, true, false))
                .unwrap(),
            service
                .request_proxy(&item(&oversized_path, 28, true, false))
                .unwrap(),
            service
                .request_proxy(&item(&failed_path, 29, true, false))
                .unwrap(),
            service
                .request_proxy(&item(&malformed_path, 30, true, false))
                .unwrap(),
        ];
        let mut errors = HashMap::new();
        for _ in 0..requests.len() {
            let event = wait_event(&events);
            errors.insert(event.job_id, event.error_code.unwrap());
        }
        assert_eq!(
            errors[&requests[0].job_id.unwrap()],
            MediaArtifactErrorCode::GenerationTimeout
        );
        assert_eq!(
            errors[&requests[1].job_id.unwrap()],
            MediaArtifactErrorCode::OutputTooLarge
        );
        assert_eq!(
            errors[&requests[2].job_id.unwrap()],
            MediaArtifactErrorCode::GenerationFailed
        );
        assert_eq!(
            errors[&requests[3].job_id.unwrap()],
            MediaArtifactErrorCode::InvalidGeneratedArtifact
        );
        for request in requests {
            wait_for_state(&service, request.job_id.unwrap(), JobState::Failed);
            assert_eq!(
                service
                    .cache
                    .proxy_path_if_present(request.cache_key.unwrap())
                    .unwrap(),
                None
            );
        }
        assert_eq!(directory.proxy_staging_count(), 0);
    }

    #[test]
    fn proxy_requests_share_the_existing_job_backpressure() {
        let directory = TestDirectory::new();
        for name in ["block-proxy-one", "proxy-two", "proxy-three"] {
            directory.media(
                &format!("{name} path with spaces-媒体.mkv"),
                name.as_bytes(),
            );
        }
        let service = service(&directory, TEST_LIMITS, 1, 1, 4, 1024, 4096);
        let first_path = directory
            .0
            .join("block-proxy-one path with spaces-媒体.mkv");
        let first = service
            .request_proxy(&item(&first_path, 31, true, false))
            .unwrap();
        wait_for_state(&service, first.job_id.unwrap(), JobState::Running);
        let second = service
            .request_proxy(&item(
                &directory.0.join("proxy-two path with spaces-媒体.mkv"),
                32,
                true,
                false,
            ))
            .unwrap();
        assert_eq!(second.state, MediaArtifactRequestState::Queued);
        assert_eq!(
            service.request_proxy(&item(
                &directory.0.join("proxy-three path with spaces-媒体.mkv"),
                33,
                true,
                false,
            )),
            Err(MediaArtifactRequestError::QueueFull)
        );
        service.cancel(first.job_id.unwrap()).unwrap();
        service.cancel(second.job_id.unwrap()).unwrap();
    }

    #[test]
    fn queue_and_record_backpressure_are_returned_without_blocking() {
        let directory = TestDirectory::new();
        for name in ["block-one", "block-two", "block-three"] {
            directory.media(
                &format!("{name} path with spaces-媒体.mkv"),
                name.as_bytes(),
            );
        }
        let queue_limited = service(
            &directory,
            TEST_LIMITS,
            1,
            1,
            8,
            1024 * 1024,
            2 * 1024 * 1024,
        );
        let one = item(
            &directory.0.join("block-one path with spaces-媒体.mkv"),
            8,
            true,
            false,
        );
        let two = item(
            &directory.0.join("block-two path with spaces-媒体.mkv"),
            9,
            true,
            false,
        );
        let three = item(
            &directory.0.join("block-three path with spaces-媒体.mkv"),
            10,
            true,
            false,
        );
        let first = queue_limited.request_thumbnail(&one).unwrap();
        wait_for_state(&queue_limited, first.job_id.unwrap(), JobState::Running);
        assert_eq!(
            queue_limited.request_thumbnail(&two).unwrap().state,
            MediaArtifactRequestState::Queued
        );
        assert_eq!(
            queue_limited.request_thumbnail(&three),
            Err(MediaArtifactRequestError::QueueFull)
        );

        let record_limited = service(
            &directory,
            TEST_LIMITS,
            1,
            8,
            2,
            1024 * 1024,
            2 * 1024 * 1024,
        );
        let first = record_limited.request_thumbnail(&one).unwrap();
        wait_for_state(&record_limited, first.job_id.unwrap(), JobState::Running);
        record_limited.request_thumbnail(&two).unwrap();
        assert_eq!(
            record_limited.request_thumbnail(&three),
            Err(MediaArtifactRequestError::RecordCapacityExceeded)
        );
    }

    #[test]
    fn timeout_kills_child_and_does_not_create_a_cache_entry() {
        let directory = TestDirectory::new();
        let timeout_path = directory.media("timeout- path with spaces-媒体.mkv", b"timeout");
        let limits = GenerationLimits {
            thumbnail_timeout: Duration::from_millis(200),
            ..TEST_LIMITS
        };
        let service = service(&directory, limits, 1, 2, 4, 1024 * 1024, 2 * 1024 * 1024);
        let events = service.subscribe_events();
        let timeout = service
            .request_thumbnail(&item(&timeout_path, 11, true, false))
            .unwrap();
        let event = wait_event(&events);
        assert_eq!(
            event.error_code,
            Some(MediaArtifactErrorCode::GenerationTimeout)
        );
        wait_for_state(&service, timeout.job_id.unwrap(), JobState::Failed);
        assert!(
            service
                .read_artifact(CacheArtifactKind::Thumbnail, timeout.cache_key.unwrap())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn oversized_stdout_stderr_and_malformed_png_fail_without_cache_entries() {
        let directory = TestDirectory::new();
        let oversized_path = directory.media("oversized- path with spaces-媒体.mkv", b"large");
        let malformed_path = directory.media("malformed- path with spaces-媒体.mkv", b"bad");
        let stderr_path = directory.media("stderr-oversized- path with spaces-媒体.mkv", b"stderr");
        directory.output("oversized-output", &vec![7_u8; 4096]);
        directory.output("malformed-output", b"not a png");
        let limits = GenerationLimits {
            stdout_limit: 512,
            stderr_limit: 128,
            ..TEST_LIMITS
        };
        let service = service(&directory, limits, 3, 4, 8, 1024 * 1024, 2 * 1024 * 1024);
        let events = service.subscribe_events();
        let oversized = service
            .request_thumbnail(&item(&oversized_path, 12, true, false))
            .unwrap();
        let malformed = service
            .request_thumbnail(&item(&malformed_path, 13, true, false))
            .unwrap();
        let stderr = service
            .request_thumbnail(&item(&stderr_path, 18, true, false))
            .unwrap();
        let mut results = [
            wait_event(&events),
            wait_event(&events),
            wait_event(&events),
        ];
        let errors = results
            .iter_mut()
            .map(|event| event.error_code.unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            errors
                .iter()
                .filter(|error| **error == MediaArtifactErrorCode::OutputTooLarge)
                .count(),
            2
        );
        assert!(errors.contains(&MediaArtifactErrorCode::InvalidGeneratedArtifact));
        for request in [oversized, malformed, stderr] {
            assert!(
                service
                    .read_artifact(CacheArtifactKind::Thumbnail, request.cache_key.unwrap())
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[test]
    fn cache_budget_failure_preserves_other_entries_and_shutdown_cancels_work() {
        let directory = TestDirectory::new();
        let path = directory.media("valid path with spaces-媒体.mkv", b"valid");
        let blocking_path = directory.media("block- shutdown path with spaces-媒体.mkv", b"block");
        let queued_path = directory.media("queued shutdown path with spaces-媒体.mkv", b"queued");
        directory.output("thumbnail-output", &png(32, 18));
        let config = MediaArtifactServiceConfig::new(
            directory.0.join("cache"),
            JobManagerConfig::new(1, 2, 4).unwrap(),
            CacheStoreConfig::new(1024 * 1024, 32).unwrap(),
            fake_executable(),
        );
        let service = MediaArtifactService::new_with_limits(config, TEST_LIMITS).unwrap();
        let old_key = CacheKey::new(
            CacheArtifactKind::Waveform,
            SourceFingerprint::from_bytes(b"old"),
            ParametersFingerprint::from_bytes(b"old profile"),
        );
        service
            .cache
            .put(CacheArtifactKind::Waveform, old_key, b"keep me")
            .unwrap();
        let events = service.subscribe_events();
        let request = service
            .request_thumbnail(&item(&path, 14, true, false))
            .unwrap();
        let event = wait_event(&events);
        assert_eq!(event.error_code, Some(MediaArtifactErrorCode::CacheError));
        assert_eq!(
            service
                .read_artifact(CacheArtifactKind::Waveform, old_key)
                .unwrap()
                .unwrap(),
            b"keep me"
        );
        assert!(
            service
                .read_artifact(CacheArtifactKind::Thumbnail, request.cache_key.unwrap())
                .unwrap()
                .is_none()
        );

        let block_request = service
            .request_thumbnail(&item(&blocking_path, 15, true, false))
            .unwrap();
        wait_for_state(&service, block_request.job_id.unwrap(), JobState::Running);
        let queued_request = service
            .request_thumbnail(&item(&queued_path, 20, true, false))
            .unwrap();
        assert_eq!(queued_request.state, MediaArtifactRequestState::Queued);
        service.shutdown();
        wait_for_state(&service, block_request.job_id.unwrap(), JobState::Cancelled);
        let queued_event = wait_event(&events);
        let running_event = wait_event(&events);
        assert_eq!(queued_event.job_id, queued_request.job_id.unwrap());
        assert_eq!(queued_event.state, MediaArtifactEventState::Cancelled);
        assert_eq!(queued_event.sequence, 2);
        assert_eq!(running_event.job_id, block_request.job_id.unwrap());
        assert_eq!(running_event.state, MediaArtifactEventState::Cancelled);
        assert_eq!(running_event.sequence, 3);
        assert!(
            service
                .read_artifact(
                    CacheArtifactKind::Thumbnail,
                    block_request.cache_key.unwrap()
                )
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn offline_source_returns_source_unavailable_without_project_mutation() {
        let directory = TestDirectory::new();
        let path = directory.media("offline path with spaces-媒体.mkv", b"offline");
        let item = item(&path, 16, true, false);
        fs::remove_file(&path).unwrap();
        let service = service(
            &directory,
            TEST_LIMITS,
            1,
            2,
            4,
            1024 * 1024,
            2 * 1024 * 1024,
        );
        assert_eq!(
            service.request_thumbnail(&item),
            Err(MediaArtifactRequestError::SourceUnavailable)
        );
        assert_eq!(service.jobs.record_count(), 0);
    }

    #[test]
    fn fingerprint_modified_time_encodes_available_and_unavailable_markers() {
        let mut unavailable = Sha256::new();
        update_modified_time(&mut unavailable, None);
        let mut available = Sha256::new();
        update_modified_time(&mut available, Some(SystemTime::UNIX_EPOCH));
        assert_ne!(unavailable.finalize(), available.finalize());
    }
}
