use or_runtime::PlaybackSnapshot;
use std::{error::Error, fmt};

#[derive(Clone, Debug)]
pub(crate) struct PreviewSnapshot {
    pub playback: PlaybackSnapshot,
    pub width: u32,
    pub height: u32,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug)]
pub(crate) struct PreviewError {
    pub code: String,
    pub message: String,
}

impl PreviewError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for PreviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl Error for PreviewError {}

pub(crate) enum PreviewPreparationAction {
    Seek(or_core::RationalTime),
    Step(or_runtime::PreviewFrameStep),
    Play,
    Tick,
}

#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "android"
))]
pub(crate) use desktop::render_adapter_info;
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "android"
))]
pub(crate) use desktop::{graphics_preference, set_graphics_preference};

pub(crate) struct PreviewPreparation {
    pub request_id: Option<u64>,
    pub sources: Vec<String>,
    pub snapshot: PreviewSnapshot,
}

#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "android"
))]
mod desktop {
    use super::{PreviewError, PreviewPreparation, PreviewPreparationAction, PreviewSnapshot};
    use crate::viewer_texture;
    use or_audio::{
        AudioClockMessage, AudioProducer, AvSynchronizer, NoiseGate, VideoSyncAction,
        process_audio_clip,
    };
    use or_audio::{AudioDeviceOutput, AudioDeviceOutputError};
    use or_core::{
        ApplicationRequest, ApplicationResponse, AudioSettings, ClipAnimation, ClipContent,
        ClipSettings, Crop, EffectReference, ExportProfile, ExportQuality, ExportRequest,
        ExportResponse, JobCancelOutcome, JobContext, JobFailure, JobKind, JobManager,
        JobManagerConfig, JobState, MAX_TIMELINE_CLIP_PAGE_SIZE, MediaId, MediaSourceRef,
        MediaStreamMetadata, MotionMaterializeRequest, MotionMaterializeResponse, Opacity,
        ProjectId, ProjectInstanceId, ProjectRevision, ProjectSession, QueryEnvelope, QueryResult,
        RationalRate, RationalTime, SequenceCanvas, TextFormatting, TimeRange, TimelineClipState,
        TimelineTrackSummaryV2, TrackKind, Transform, TransitionReference,
    };
    use or_ipc::{
        ExportRequestHandler, LiveProjectHost, MotionMaterializeHandler, ProjectSessionIdentity,
    };
    use or_media::{DecodeOutput, SnapshotQueue, SoftwareMediaDecoder, VideoDecodeSession};
    use or_render::{
        CompositorLayer, Nv12VideoLayer, RenderDevice, RenderSize, RgbaVideoLayer, TextRasterizer,
        VisualTransition, process_visual_rgba,
    };
    use or_runtime::{
        BudgetLimits, CancellationToken, FramePixelFormat, PreviewFrameRequest, PreviewFrameStep,
        PreviewTransport, RenderSnapshot, RuntimeBudgets,
    };
    use std::{
        collections::{HashMap, HashSet, VecDeque},
        future::Future,
        num::NonZeroUsize,
        pin::Pin,
        sync::{
            Arc, Mutex, MutexGuard, OnceLock,
            atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
            mpsc,
        },
        task::{Context, Poll, Wake, Waker},
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    const PAGE_SIZE: usize = 100;
    const MAX_VIEWER_WIDTH: u32 = 1920;
    const MAX_VIEWER_HEIGHT: u32 = 1080;
    const MAX_BLANK_WIDTH: u32 = 640;
    const MAX_BLANK_HEIGHT: u32 = 360;
    const MAX_TEXT_OVERLAY_BYTES: usize = 64 * 1024 * 1024;
    const MAX_VISUAL_PROCESS_BYTES: usize = 256 * 1024 * 1024;
    const AUDIO_SAMPLE_RATE: u32 = 48_000;
    const AUDIO_BUFFER_FRAMES: usize = 12_000;
    const AUDIO_DECODE_QUEUE_CAPACITY: usize = 256;
    /// How many rendered frames may wait for the writer during an export. Two is
    /// enough to keep the GPU and the encoder busy at once without holding a long
    /// run of full-size frames in memory.
    const EXPORT_RENDER_AHEAD: usize = 2;

    /// Experiment knob for the render-ahead depth; defaults to the constant.
    fn export_render_ahead() -> usize {
        std::env::var("OR_EXPORT_RENDER_AHEAD")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(EXPORT_RENDER_AHEAD)
    }
    /// Export decodes each audio clip once into memory before rendering. The cap
    /// keeps a very long timeline from exhausting the process; above it the export
    /// keeps the streaming path instead of swapping.
    const MAX_EXPORT_AUDIO_CACHE_BYTES: usize = 1024 * 1024 * 1024;
    /// Ceiling for the parallel audio pre-decode. The clips are independent files,
    /// so the cache scales with the machine instead of stopping at four workers;
    /// the memory reservation below, not the worker count, is what keeps it bounded.
    const MAX_EXPORT_AUDIO_WORKERS: usize = 16;

    /// The graphics backend chosen in Settings, read once when the device opens.
    static GRAPHICS_PREFERENCE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

    /// Records the backend preference before the renderer opens.
    ///
    /// Accepts `auto` or a compiler backend name; an unknown name is a caller
    /// error rather than a silent fallback, so the UI can say the truth.
    pub(crate) fn set_graphics_preference(backend: &str) -> Result<(), String> {
        let name = backend.trim().to_ascii_lowercase();
        if name != "auto" && or_render::RenderDevice::backend_flag(&name).is_none() {
            return Err(format!(
                "graphics backend must be auto, metal, vulkan, dx12 or gles; got {backend:?}"
            ));
        }
        let mut guard = GRAPHICS_PREFERENCE
            .lock()
            .map_err(|_| "graphics preference lock is poisoned".to_owned())?;
        *guard = Some(name);
        Ok(())
    }

    pub(crate) fn graphics_preference() -> String {
        GRAPHICS_PREFERENCE
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
            .unwrap_or_else(|| "auto".to_owned())
    }

    static RENDER_DEVICE: OnceLock<Result<RenderDevice, String>> = OnceLock::new();
    static FFMPEG_LICENSE_OK: OnceLock<bool> = OnceLock::new();
    static FFMPEG_LICENSE_PROBLEM: OnceLock<String> = OnceLock::new();

    fn media_decoder(
        source: &MediaSourceRef,
        budgets: RuntimeBudgets,
    ) -> Result<SoftwareMediaDecoder, or_media::DecodeError> {
        #[cfg(target_os = "android")]
        if let Some(capability) = crate::android_media_io::capability_for(source) {
            return SoftwareMediaDecoder::new_with_seekable_io(capability, budgets);
        }
        SoftwareMediaDecoder::new(source, budgets)
    }

    pub struct PreviewRuntime {
        state: Mutex<SessionState>,
        render_lock: Mutex<()>,
        cancellation: Mutex<CancellationToken>,
        generation: AtomicU64,
        render_resources: Arc<RenderResources>,
        export_jobs: JobManager,
        pending: Mutex<Option<PreparedFrame>>,
        preparation_sequence: AtomicU64,
        preparation_lock: Mutex<()>,
        closed: AtomicBool,
        completing_prepared: AtomicBool,
        completion_lock: Mutex<()>,
    }

    #[derive(Clone, Copy)]
    enum RenderMode {
        Immediate,
        Prepare(u64),
    }

    struct PreparedFrame {
        operation: u64,
        program: Arc<PreviewProgram>,
        request: PreviewFrameRequest,
        generation: u64,
        cancellation: CancellationToken,
        sources: Vec<String>,
        start_audio_after_registration: bool,
    }

    struct PreparedCompletion<'a>(&'a AtomicBool);
    impl Drop for PreparedCompletion<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }

    impl PreviewRuntime {
        pub fn new() -> Self {
            let budgets = RuntimeBudgets::new(
                BudgetLimits::new(4, 256 * 1024 * 1024).expect("nonzero render budget"),
                BudgetLimits::new(AUDIO_DECODE_QUEUE_CAPACITY, 32 * 1024 * 1024)
                    .expect("nonzero audio budget"),
                BudgetLimits::new(16, 256 * 1024 * 1024).expect("nonzero decode budget"),
            );
            Self {
                state: Mutex::new(SessionState::default()),
                render_lock: Mutex::new(()),
                cancellation: Mutex::new(CancellationToken::new()),
                generation: AtomicU64::new(0),
                render_resources: Arc::new(RenderResources {
                    budgets,
                    text_rasterizer: Mutex::new(TextRasterizer::new()),
                    video_sessions: Mutex::new(VecDeque::new()),
                }),
                export_jobs: JobManager::new(
                    JobManagerConfig::new(1, 1, 32).expect("valid bounded export jobs"),
                ),
                pending: Mutex::new(None),
                preparation_sequence: AtomicU64::new(0),
                preparation_lock: Mutex::new(()),
                closed: AtomicBool::new(false),
                completing_prepared: AtomicBool::new(false),
                completion_lock: Mutex::new(()),
            }
        }

        fn handle_export_request(
            &self,
            session: &ProjectSession,
            request: ExportRequest,
        ) -> ExportResponse {
            match request {
                ExportRequest::Start {
                    project_id,
                    project_instance_id,
                    expected_project_revision,
                    destination,
                    profile,
                    quality,
                    short_side,
                    performance,
                } => {
                    if project_id != session.project_id()
                        || project_instance_id != session.project_instance_id()
                    {
                        return ExportResponse::failure(
                            "PROJECT_INSTANCE_MISMATCH",
                            "export request belongs to a different project session",
                        );
                    }
                    if expected_project_revision != session.project_revision() {
                        return ExportResponse::failure(
                            "REVISION_CONFLICT",
                            "the project changed before export started; refresh and retry",
                        );
                    }
                    self.quiesce_for_export();
                    let destination = std::path::PathBuf::from(destination);
                    let (extension, format_name) = match profile {
                        ExportProfile::MatroskaFfv1PcmS16le => ("mkv", "Matroska"),
                        ExportProfile::WebmVp9Opus => ("webm", "WebM"),
                        ExportProfile::Mp4H264Aac => ("mp4", "MP4"),
                    };
                    if !destination.is_absolute()
                        || !destination
                            .extension()
                            .and_then(std::ffi::OsStr::to_str)
                            .is_some_and(|actual| actual.eq_ignore_ascii_case(extension))
                    {
                        return ExportResponse::failure(
                            "INVALID_EXPORT_DESTINATION",
                            format!(
                                "choose an absolute {format_name} destination with a .{extension} extension"
                            ),
                        );
                    }
                    if !*FFMPEG_LICENSE_OK.get_or_init(or_media::verify_ffmpeg_runtime) {
                        let reason = FFMPEG_LICENSE_PROBLEM
                            .get_or_init(|| {
                                or_media::ffmpeg_runtime_problem()
                                    .unwrap_or_else(|| "the runtime check did not pass".to_owned())
                            })
                            .clone();
                        return ExportResponse::failure(
                            "FFMPEG_UNAVAILABLE",
                            format!(
                                "the approved LGPL FFmpeg runtime could not be loaded: {reason}"
                            ),
                        );
                    }
                    let program = match load_program_from_session(session) {
                        Ok(program) => Arc::new(program),
                        Err(error) => {
                            return ExportResponse::failure(error.code, error.message);
                        }
                    };
                    let Some(frame_rate) = program.frame_rate else {
                        return ExportResponse::failure(
                            "SEQUENCE_FRAME_RATE_REQUIRED",
                            "set the sequence frame rate before exporting",
                        );
                    };
                    let Some(content_end) = program.content_end.filter(|end| end.is_positive())
                    else {
                        return ExportResponse::failure(
                            "EXPORT_EMPTY_TIMELINE",
                            "add timed timeline content before exporting",
                        );
                    };
                    let frame_count = match frame_rate.frame_index_ceil(content_end) {
                        Ok(count) if count > 0 => count,
                        Ok(_) => {
                            return ExportResponse::failure(
                                "EXPORT_EMPTY_TIMELINE",
                                "the timeline contains no output frames",
                            );
                        }
                        Err(error) => {
                            return ExportResponse::failure(
                                "EXPORT_DURATION_INVALID",
                                error.to_string(),
                            );
                        }
                    };
                    let output_size = export_size(
                        program.canvas,
                        short_side,
                        program
                            .video_clips
                            .iter()
                            .find_map(|clip| clip.source_dimensions),
                    );
                    let render_resources = Arc::clone(&self.render_resources);
                    let submit = self.export_jobs.submit(JobKind::Export, move |job| {
                        let cancellation = CancellationToken::new();
                        let watcher_cancellation = cancellation.clone();
                        let watcher_job = job.clone();
                        let watcher_stopped = Arc::new(AtomicBool::new(false));
                        let stop_watcher = Arc::clone(&watcher_stopped);
                        let watcher = thread::Builder::new()
                            .name("or-export-cancel-watch".to_owned())
                            .spawn(move || {
                                while !stop_watcher.load(Ordering::SeqCst) {
                                    if watcher_job.is_cancelled() {
                                        watcher_cancellation.cancel();
                                        return;
                                    }
                                    thread::sleep(Duration::from_millis(5));
                                }
                            })
                            .map_err(|error| JobFailure::with_message(error.to_string()))?;
                        let result = export_program(
                            program,
                            destination,
                            profile,
                            quality,
                            performance,
                            output_size,
                            frame_count,
                            frame_rate,
                            render_resources,
                            job,
                            &cancellation,
                        );
                        watcher_stopped.store(true, Ordering::SeqCst);
                        let _ = watcher.join();
                        result.map_err(JobFailure::with_message)
                    });
                    let job_id = match submit {
                        Ok(job_id) => job_id,
                        Err(error) => {
                            let (code, message) = match error {
                                or_core::JobSubmitError::QueueFull => {
                                    ("EXPORT_QUEUE_FULL", "another export is queued or running")
                                }
                                or_core::JobSubmitError::RecordCapacityExceeded => (
                                    "EXPORT_JOB_CAPACITY",
                                    "export job history is full; wait for active work to finish",
                                ),
                                or_core::JobSubmitError::Shutdown => (
                                    "EXPORT_RUNTIME_UNAVAILABLE",
                                    "the export runtime is shutting down",
                                ),
                            };
                            return ExportResponse::failure(code, message);
                        }
                    };
                    self.export_jobs.snapshot(job_id).map_or_else(
                        || {
                            ExportResponse::failure(
                                "EXPORT_JOB_UNAVAILABLE",
                                "the new export job could not be inspected",
                            )
                        },
                        |snapshot| ExportResponse::success("Export queued.", snapshot),
                    )
                }
                ExportRequest::Status {
                    project_id,
                    project_instance_id,
                    job_id,
                } => {
                    if !export_request_matches_session(project_id, project_instance_id, session) {
                        return ExportResponse::failure(
                            "PROJECT_INSTANCE_MISMATCH",
                            "export job belongs to a different project session",
                        );
                    }
                    self.export_jobs.snapshot(job_id).map_or_else(
                        || {
                            ExportResponse::failure(
                                "EXPORT_JOB_NOT_FOUND",
                                "the export job is no longer available",
                            )
                        },
                        |snapshot| ExportResponse::success("Export status.", snapshot),
                    )
                }
                ExportRequest::Cancel {
                    project_id,
                    project_instance_id,
                    job_id,
                } => {
                    if !export_request_matches_session(project_id, project_instance_id, session) {
                        return ExportResponse::failure(
                            "PROJECT_INSTANCE_MISMATCH",
                            "export job belongs to a different project session",
                        );
                    }
                    let outcome = match self.export_jobs.cancel(job_id) {
                        Ok(outcome) => outcome,
                        Err(_) => {
                            return ExportResponse::failure(
                                "EXPORT_JOB_NOT_FOUND",
                                "the export job is no longer available",
                            );
                        }
                    };
                    let message = match outcome {
                        JobCancelOutcome::CancelledQueued => "Export cancelled.",
                        JobCancelOutcome::CancellationRequested => "Cancellation requested.",
                        JobCancelOutcome::AlreadyTerminal(JobState::Succeeded) => {
                            "Export already completed."
                        }
                        JobCancelOutcome::AlreadyTerminal(JobState::Failed) => {
                            "Export already failed."
                        }
                        JobCancelOutcome::AlreadyTerminal(JobState::Cancelled) => {
                            "Export already cancelled."
                        }
                        JobCancelOutcome::AlreadyTerminal(JobState::Queued | JobState::Running) => {
                            "Cancellation was requested."
                        }
                    };
                    self.export_jobs.snapshot(job_id).map_or_else(
                        || {
                            ExportResponse::failure(
                                "EXPORT_JOB_NOT_FOUND",
                                "the export job is no longer available",
                            )
                        },
                        |snapshot| ExportResponse::success(message, snapshot),
                    )
                }
            }
        }

        pub fn status(&self, host: &LiveProjectHost) -> Result<PreviewSnapshot, PreviewError> {
            self.ensure_program(host)?;
            Ok(self.snapshot())
        }

        pub fn seek(
            &self,
            host: &LiveProjectHost,
            time: RationalTime,
        ) -> Result<PreviewSnapshot, PreviewError> {
            self.seek_with_mode(host, time, RenderMode::Immediate)
        }

        fn seek_with_mode(
            &self,
            host: &LiveProjectHost,
            time: RationalTime,
            mode: RenderMode,
        ) -> Result<PreviewSnapshot, PreviewError> {
            if time.is_negative() {
                return Err(PreviewError::new(
                    "NEGATIVE_PREVIEW_TIME",
                    "preview time must be nonnegative",
                ));
            }
            let program = self.ensure_program(host)?;
            let (request, external_generation, cancellation, stopped_audio) = {
                let mut state = lock(&self.state);
                let playback = state.transport.seek(time).map_err(transport_error)?;
                state.error_code = None;
                state.error_message = None;
                state.audio_error = None;
                let (external_generation, cancellation) = self.invalidate()?;
                let stopped_audio = state.audio_playback.take();
                (
                    PreviewFrameRequest {
                        time,
                        frame_index: None,
                        generation: playback.generation,
                    },
                    external_generation,
                    cancellation,
                    stopped_audio,
                )
            };
            drop(stopped_audio);
            self.dispatch_request(
                host,
                program,
                request,
                external_generation,
                cancellation,
                mode,
            )?;
            Ok(self.snapshot())
        }

        pub fn step(
            &self,
            host: &LiveProjectHost,
            direction: PreviewFrameStep,
        ) -> Result<PreviewSnapshot, PreviewError> {
            self.step_with_mode(host, direction, RenderMode::Immediate)
        }

        fn step_with_mode(
            &self,
            host: &LiveProjectHost,
            direction: PreviewFrameStep,
            mode: RenderMode,
        ) -> Result<PreviewSnapshot, PreviewError> {
            let program = self.ensure_program(host)?;
            let (action, stopped_audio) = {
                let mut state = lock(&self.state);
                let Some(request) = state.transport.step(direction).map_err(transport_error)?
                else {
                    return Ok(state.snapshot());
                };
                state.error_code = None;
                state.error_message = None;
                state.audio_error = None;
                let (external_generation, cancellation) = self.invalidate()?;
                (
                    Some((request, external_generation, cancellation)),
                    state.audio_playback.take(),
                )
            };
            drop(stopped_audio);
            if let Some((request, external_generation, cancellation)) = action {
                self.dispatch_request(
                    host,
                    program,
                    request,
                    external_generation,
                    cancellation,
                    mode,
                )?;
            }
            Ok(self.snapshot())
        }

        pub fn play(&self, host: &LiveProjectHost) -> Result<PreviewSnapshot, PreviewError> {
            self.play_with_audio(host, true)
        }

        fn play_with_audio(
            &self,
            host: &LiveProjectHost,
            start_audio: bool,
        ) -> Result<PreviewSnapshot, PreviewError> {
            let program = self.ensure_program(host)?;
            let (origin, expected_generation, speed) = {
                let mut state = lock(&self.state);
                let playback = state.transport.snapshot();
                if playback.playing {
                    return Ok(state.snapshot());
                }
                if playback.frame_rate.is_none() {
                    return Err(transport_error(
                        or_runtime::PreviewTransportError::FrameRateUnavailable,
                    ));
                }
                if let Some(content_end) = playback
                    .content_end
                    .filter(|content_end| playback.position >= *content_end)
                {
                    if content_end <= RationalTime::ZERO {
                        return Ok(state.snapshot());
                    }
                    // Play on the last frame replays from the start, the way
                    // every familiar player behaves.
                    state
                        .transport
                        .seek(RationalTime::ZERO)
                        .map_err(transport_error)?;
                }
                let playback = state.transport.snapshot();
                (
                    playback.position,
                    playback.generation,
                    state.transport.speed(),
                )
            };
            let (audio_playback, audio_error) =
                if !start_audio || program.audio_clips.is_empty() || speed != RationalRate::ONE {
                    (None, None)
                } else {
                    start_audio_playback(
                        Arc::clone(&program),
                        origin,
                        self.render_resources.budgets.clone(),
                    )
                };
            let mut state = lock(&self.state);
            let current = state.transport.snapshot();
            if current.generation != expected_generation || current.position != origin {
                let snapshot = state.snapshot();
                drop(state);
                drop(audio_playback);
                return Ok(snapshot);
            }
            let previous_generation = state.transport.snapshot().generation;
            let playback = state
                .transport
                .play(Instant::now())
                .map_err(transport_error)?;
            if playback.generation != previous_generation {
                self.invalidate()?;
            }
            state.audio_playback = audio_playback;
            state.audio_error = audio_error;
            state.error_code = None;
            state.error_message = None;
            Ok(state.snapshot())
        }

        /// Stops preview playback and releases the audio decoder leases before an
        /// export prepares its own audio cache. Exporting while the preview plays
        /// otherwise exhausts the shared playback audio budget.
        fn quiesce_for_export(&self) {
            let mut state = lock(&self.state);
            let audio_clock = state.audio_playback.as_ref().map(AudioPlayback::clock);
            if let Some(clock) = audio_clock {
                if let Ok(position) = clock.timeline_time() {
                    let _ = state.transport.pause_at_master_clock(position);
                }
            } else {
                let _ = state.transport.pause(Instant::now());
            }
            let stopped_audio = state.audio_playback.take();
            drop(state);
            drop(stopped_audio);
        }

        pub fn pause(&self, host: &LiveProjectHost) -> Result<PreviewSnapshot, PreviewError> {
            let _ = self.ensure_program(host)?;
            let mut state = lock(&self.state);
            let audio_clock = state.audio_playback.as_ref().map(AudioPlayback::clock);
            if let Some(clock) = audio_clock {
                let position = clock
                    .timeline_time()
                    .map_err(|error| PreviewError::new("AUDIO_CLOCK_FAILED", error.to_string()))?;
                state
                    .transport
                    .pause_at_master_clock(position)
                    .map_err(transport_error)?;
            } else {
                state
                    .transport
                    .pause(Instant::now())
                    .map_err(transport_error)?;
            }
            let stopped_audio = state.audio_playback.take();
            let snapshot = state.snapshot();
            drop(state);
            drop(stopped_audio);
            Ok(snapshot)
        }

        /// Changes preview speed, keeping the exact position.
        ///
        /// Audio is turned off because the mixer renders at 1x; a speed other
        /// than 1x is therefore silent rather than pitched or drifting.
        pub fn set_speed(
            &self,
            host: &LiveProjectHost,
            speed: RationalRate,
        ) -> Result<PreviewSnapshot, PreviewError> {
            let _ = self.ensure_program(host)?;
            let mut state = lock(&self.state);
            let was_playing = state.transport.snapshot().playing;
            let audio_clock = state.audio_playback.as_ref().map(AudioPlayback::clock);
            if let Some(clock) = audio_clock {
                if let Ok(position) = clock.timeline_time() {
                    let _ = state.transport.pause_at_master_clock(position);
                }
            } else if was_playing {
                let _ = state.transport.pause(Instant::now());
            }
            let now = Instant::now();
            state
                .transport
                .set_speed(speed, now)
                .map_err(transport_error)?;
            let stopped_audio = state.audio_playback.take();
            if was_playing {
                let _ = state.transport.play(now).map_err(transport_error)?;
                self.invalidate()?;
            }
            let snapshot = state.snapshot();
            drop(state);
            drop(stopped_audio);
            Ok(snapshot)
        }

        pub fn tick(&self, host: &LiveProjectHost) -> Result<PreviewSnapshot, PreviewError> {
            self.tick_with_mode(host, RenderMode::Immediate)
        }

        fn tick_with_mode(
            &self,
            host: &LiveProjectHost,
            mode: RenderMode,
        ) -> Result<PreviewSnapshot, PreviewError> {
            let program = self.ensure_program(host)?;
            let (action, audio_clock, stopped_audio) = {
                let mut state = lock(&self.state);
                let audio_clock = state.audio_playback.as_ref().map(AudioPlayback::clock);
                let next = if let Some(clock) = audio_clock {
                    let position = clock.timeline_time().map_err(|error| {
                        PreviewError::new("AUDIO_CLOCK_FAILED", error.to_string())
                    })?;
                    state.transport.tick_from_master_clock(position)
                } else {
                    state.transport.tick(Instant::now())
                };
                let Some(request) = next.map_err(transport_error)? else {
                    if let Some(audio) = state.audio_playback.as_ref() {
                        state.audio_error = audio.error().or_else(|| {
                            audio
                                .failed()
                                .then(|| "The audio output device stopped.".to_owned())
                        });
                    }
                    return Ok(state.snapshot());
                };
                if let Some(audio) = state.audio_playback.as_ref() {
                    state.audio_error = audio.error().or_else(|| {
                        audio
                            .failed()
                            .then(|| "The audio output device stopped.".to_owned())
                    });
                }
                let generation = self.generation.load(Ordering::SeqCst);
                let cancellation = lock(&self.cancellation).clone();
                let stopped_audio = if state.transport.snapshot().playing {
                    None
                } else {
                    state.audio_playback.take()
                };
                (
                    Some((request, generation, cancellation)),
                    audio_clock,
                    stopped_audio,
                )
            };
            drop(stopped_audio);
            if let Some((request, generation, cancellation)) = action {
                if let Some(clock) = audio_clock {
                    let max_drift = program
                        .frame_rate
                        .and_then(|rate| rate.frame_time(1).ok())
                        .unwrap_or(RationalTime::new(1, 24).expect("positive drift"));
                    let synchronizer = AvSynchronizer::new(max_drift).map_err(|error| {
                        PreviewError::new("AUDIO_SYNC_FAILED", error.to_string())
                    })?;
                    let video_snapshot = RenderSnapshot::new(
                        program.key.project_id,
                        program.key.revision,
                        TimeRange::new(request.time, RationalTime::ZERO).map_err(|error| {
                            PreviewError::new("AUDIO_SYNC_FAILED", error.to_string())
                        })?,
                    );
                    if synchronizer
                        .video_action(video_snapshot, request.time, clock)
                        .map_err(|error| {
                            PreviewError::new("AUDIO_SYNC_FAILED", error.to_string())
                        })?
                        != VideoSyncAction::Present
                    {
                        return Ok(self.snapshot());
                    }
                }
                self.dispatch_request(host, program, request, generation, cancellation, mode)?;
            }
            Ok(self.snapshot())
        }

        fn ensure_program(
            &self,
            host: &LiveProjectHost,
        ) -> Result<Arc<PreviewProgram>, PreviewError> {
            let summary = host
                .describe()
                .map_err(|error| PreviewError::new("PREVIEW_QUERY_FAILED", error.to_string()))?
                .summary;
            let key = ProgramKey {
                project_id: summary.project_id,
                project_instance_id: summary.project_instance_id,
                revision: summary.project_revision,
            };
            if let Some(program) = lock(&self.state)
                .program
                .as_ref()
                .filter(|program| program.key == key)
                .cloned()
            {
                return Ok(program);
            }

            let program = Arc::new(load_program(host, key)?);
            let current_summary = host
                .describe()
                .map_err(|error| PreviewError::new("PREVIEW_QUERY_FAILED", error.to_string()))?
                .summary;
            if current_summary.project_id != key.project_id
                || current_summary.project_instance_id != key.project_instance_id
                || current_summary.project_revision != key.revision
            {
                return Err(PreviewError::new(
                    "STALE_PREVIEW_SNAPSHOT",
                    "the project changed while preview state was loading; refresh and retry",
                ));
            }

            let mut state = lock(&self.state);
            let mut stopped_audio = None;
            if state
                .program
                .as_ref()
                .is_none_or(|current| current.key != key)
            {
                let _ = self.invalidate()?;
                state
                    .transport
                    .configure(program.frame_rate, program.content_end)
                    .map_err(transport_error)?;
                state.program = Some(Arc::clone(&program));
                state.key = Some(key);
                state.width = 0;
                state.height = 0;
                state.error_code = None;
                state.error_message = None;
                state.audio_error = None;
                stopped_audio = state.audio_playback.take();
            }
            let program = state.program.as_ref().cloned().unwrap_or(program);
            drop(state);
            drop(stopped_audio);
            Ok(program)
        }

        fn invalidate(&self) -> Result<(u64, CancellationToken), PreviewError> {
            lock(&self.pending).take();
            let generation = viewer_texture::next_generation().ok_or_else(|| {
                PreviewError::new(
                    "PREVIEW_GENERATION_OVERFLOW",
                    "preview generation overflowed",
                )
            })?;
            let mut current = lock(&self.cancellation);
            current.cancel();
            let next = CancellationToken::new();
            *current = next.clone();
            if !viewer_texture::advance_generation(generation) {
                return Err(PreviewError::new(
                    "PREVIEW_GENERATION_STALE",
                    "a newer preview generation has already replaced this request",
                ));
            }
            self.generation.store(generation, Ordering::SeqCst);
            Ok((generation, next))
        }

        /// Android binds only the sources required by this exact shared preview
        /// request. One pending request owns no native media handles or pixels.
        pub fn prepare(
            &self,
            host: &LiveProjectHost,
            action: PreviewPreparationAction,
        ) -> Result<PreviewPreparation, PreviewError> {
            let _prepare = lock(&self.preparation_lock);
            if self.closed.load(Ordering::SeqCst) {
                return Err(stale_preparation());
            }
            // A timer tick cannot supersede a frame still opening or decoding.
            // The next completed tick chooses the newest due transport frame.
            if matches!(action, PreviewPreparationAction::Tick)
                && (self.completing_prepared.load(Ordering::SeqCst)
                    || lock(&self.pending).is_some())
            {
                return Ok(PreviewPreparation {
                    request_id: None,
                    sources: Vec::new(),
                    snapshot: self.snapshot(),
                });
            }
            let operation = self.preparation_sequence.fetch_add(1, Ordering::SeqCst) + 1;
            let mode = RenderMode::Prepare(operation);
            lock(&self.pending).take();
            let snapshot = match action {
                PreviewPreparationAction::Seek(time) => self.seek_with_mode(host, time, mode)?,
                PreviewPreparationAction::Step(direction) => {
                    self.step_with_mode(host, direction, mode)?
                }
                PreviewPreparationAction::Play => {
                    self.play_with_audio(host, false)?;
                    self.tick_with_mode(host, mode)?
                }
                PreviewPreparationAction::Tick => self.tick_with_mode(host, mode)?,
            };
            if self.preparation_sequence.load(Ordering::SeqCst) != operation {
                return Err(stale_preparation());
            }
            let pending = lock(&self.pending);
            Ok(PreviewPreparation {
                request_id: pending
                    .as_ref()
                    .filter(|frame| frame.operation == operation)
                    .map(|frame| frame.generation),
                sources: pending
                    .as_ref()
                    .filter(|frame| frame.operation == operation)
                    .map_or_else(Vec::new, |frame| frame.sources.clone()),
                snapshot,
            })
        }

        pub fn complete_prepared(
            &self,
            host: &LiveProjectHost,
            request_id: u64,
        ) -> Result<PreviewSnapshot, PreviewError> {
            let _serial = lock(&self.completion_lock);
            self.completing_prepared.store(true, Ordering::SeqCst);
            let _completion = PreparedCompletion(&self.completing_prepared);
            let frame = {
                let mut pending = lock(&self.pending);
                if !pending
                    .as_ref()
                    .is_some_and(|frame| frame.generation == request_id)
                {
                    return Err(stale_preparation());
                }
                pending.take().expect("checked pending request")
            };
            if self.closed.load(Ordering::SeqCst)
                || !self.request_is_current(frame.request, frame.generation, &frame.cancellation)
            {
                return Err(stale_preparation());
            }
            let current = host
                .describe()
                .map_err(|error| PreviewError::new("PREVIEW_QUERY_FAILED", error.to_string()))?
                .summary;
            if current.project_id != frame.program.key.project_id
                || current.project_instance_id != frame.program.key.project_instance_id
                || current.project_revision != frame.program.key.revision
            {
                return Err(stale_preparation());
            }
            if frame.start_audio_after_registration {
                let origin = lock(&self.state).transport.snapshot().position;
                let (audio_playback, audio_error) = start_audio_playback(
                    Arc::clone(&frame.program),
                    origin,
                    self.render_resources.budgets.clone(),
                );
                let mut state = lock(&self.state);
                if state.transport.snapshot().playing && state.audio_playback.is_none() {
                    state.audio_playback = audio_playback;
                    state.audio_error = audio_error;
                }
            }
            self.render_request(
                host,
                frame.program,
                frame.request,
                frame.generation,
                frame.cancellation,
            );
            Ok(self.snapshot())
        }

        pub fn abort_prepared(&self, request_id: u64) -> Result<(), PreviewError> {
            let _prepare = lock(&self.preparation_lock);
            if self.generation.load(Ordering::SeqCst) == request_id {
                self.cancel()?;
                let _render = lock(&self.render_lock);
                lock(&self.render_resources.video_sessions).clear();
            }
            Ok(())
        }

        pub fn shutdown(&self) -> Result<(), PreviewError> {
            let _prepare = lock(&self.preparation_lock);
            self.closed.store(true, Ordering::SeqCst);
            self.cancel()?;
            // Cancellation invalidates publication first. Drain any decoder
            // holding a cloned capability before the platform clears its FDs.
            let _render = lock(&self.render_lock);
            lock(&self.render_resources.video_sessions).clear();
            let mut state = lock(&self.state);
            state.program = None;
            state.key = None;
            Ok(())
        }

        fn cancel(&self) -> Result<(), PreviewError> {
            self.invalidate()?;
            let mut state = lock(&self.state);
            state
                .transport
                .pause(Instant::now())
                .map_err(transport_error)?;
            let audio = state.audio_playback.take();
            drop(state);
            drop(audio);
            Ok(())
        }

        #[allow(clippy::too_many_arguments)]
        fn dispatch_request(
            &self,
            host: &LiveProjectHost,
            program: Arc<PreviewProgram>,
            request: PreviewFrameRequest,
            generation: u64,
            cancellation: CancellationToken,
            mode: RenderMode,
        ) -> Result<(), PreviewError> {
            match mode {
                RenderMode::Immediate => {
                    self.render_request(host, program, request, generation, cancellation)
                }
                RenderMode::Prepare(operation) => {
                    if self.closed.load(Ordering::SeqCst) {
                        return Err(stale_preparation());
                    }
                    let (playing, start_audio_after_registration) = {
                        let state = lock(&self.state);
                        let playing = state.transport.snapshot().playing;
                        (
                            playing,
                            playing
                                && state.audio_playback.is_none()
                                && state.audio_error.is_none()
                                && !program.audio_clips.is_empty(),
                        )
                    };
                    let sources = if playing {
                        program.playback_saf_sources(request.time)?
                    } else {
                        program.active_saf_sources(request.time)?
                    };
                    // Superseding a prepared tick also cancels already decoding
                    // work; transport timing remains the original exact request.
                    let (generation, cancellation) = self.invalidate()?;
                    if self.preparation_sequence.load(Ordering::SeqCst) != operation {
                        return Err(stale_preparation());
                    }
                    *lock(&self.pending) = Some(PreparedFrame {
                        operation,
                        program,
                        request,
                        generation,
                        cancellation,
                        sources,
                        start_audio_after_registration,
                    });
                }
            }
            Ok(())
        }

        fn render_request(
            &self,
            host: &LiveProjectHost,
            program: Arc<PreviewProgram>,
            request: PreviewFrameRequest,
            external_generation: u64,
            cancellation: CancellationToken,
        ) {
            let _render = lock(&self.render_lock);
            if !self.request_is_current(request, external_generation, &cancellation) {
                return;
            }
            match self.render_at(&program, request.time, external_generation, &cancellation) {
                Ok(presentation) => {
                    let current = host.describe().ok().map(|result| result.summary);
                    let is_current_project = current.is_some_and(|summary| {
                        summary.project_id == program.key.project_id
                            && summary.project_instance_id == program.key.project_instance_id
                            && summary.project_revision == program.key.revision
                    });
                    if !is_current_project
                        || !self.request_is_current(request, external_generation, &cancellation)
                    {
                        return;
                    }
                    match viewer_texture::publish_rgba_frame(
                        external_generation,
                        presentation.width,
                        presentation.height,
                        request.time,
                        &presentation.pixels,
                    ) {
                        Ok(()) => {
                            let mut state = lock(&self.state);
                            if state
                                .transport
                                .record_presented(request.generation, request.time)
                            {
                                state.width = presentation.width;
                                state.height = presentation.height;
                                state.error_code = presentation.error_code;
                                state.error_message = presentation.error_message;
                            }
                        }
                        Err(or_runtime::ViewerTextureError::StaleGeneration) => {}
                        Err(error) => self.set_error(
                            request.generation,
                            "VIEWER_FRAME_REJECTED",
                            error.to_string(),
                        ),
                    }
                }
                Err(error) => self.set_error(request.generation, &error.code, error.message),
            }
        }

        fn render_at(
            &self,
            program: &PreviewProgram,
            time: RationalTime,
            generation: u64,
            cancellation: &CancellationToken,
        ) -> Result<Presentation, PreviewError> {
            let fixed_size = program.canvas.map(preview_canvas_size);
            self.render_resources.render_with_size(
                program,
                time,
                fixed_size,
                Some((generation, &self.generation)),
                false,
                RenderOutput::Packed,
                cancellation,
            )
        }
    }

    struct RenderResources {
        budgets: RuntimeBudgets,
        text_rasterizer: Mutex<TextRasterizer>,
        video_sessions: Mutex<VecDeque<VideoSessionEntry>>,
    }

    const VIDEO_SESSION_CACHE_CAPACITY: usize = 4;

    #[derive(Eq, PartialEq)]
    struct VideoSessionKey {
        program: ProgramKey,
        generation: Option<u64>,
        source: String,
        output: DecodeOutput,
    }

    struct VideoSessionEntry {
        key: VideoSessionKey,
        session: VideoDecodeSession,
    }

    impl RenderResources {
        fn decode_video_frame(
            &self,
            program: ProgramKey,
            generation: Option<u64>,
            source: &MediaSourceRef,
            source_time: RationalTime,
            cancellation: &CancellationToken,
        ) -> Result<Option<or_media::VideoFrame>, or_media::DecodeError> {
            self.decode_video_frame_for(
                program,
                generation,
                source,
                source_time,
                DecodeOutput::Rgba,
                cancellation,
            )
        }

        /// Decodes one frame as NV12 already scaled to `size`, which skips the
        /// RGBA conversion, the GPU upload and the readback entirely.
        fn decode_video_frame_nv12(
            &self,
            program: ProgramKey,
            generation: Option<u64>,
            source: &MediaSourceRef,
            source_time: RationalTime,
            size: (u32, u32),
            cancellation: &CancellationToken,
        ) -> Result<Option<or_media::VideoFrame>, or_media::DecodeError> {
            self.decode_video_frame_for(
                program,
                generation,
                source,
                source_time,
                DecodeOutput::Nv12 {
                    width: size.0,
                    height: size.1,
                },
                cancellation,
            )
        }

        #[allow(clippy::too_many_arguments)]
        fn decode_video_frame_for(
            &self,
            program: ProgramKey,
            generation: Option<u64>,
            source: &MediaSourceRef,
            source_time: RationalTime,
            output: DecodeOutput,
            cancellation: &CancellationToken,
        ) -> Result<Option<or_media::VideoFrame>, or_media::DecodeError> {
            let key = VideoSessionKey {
                program,
                generation,
                source: source.uri().to_owned(),
                output,
            };
            let entry = {
                let mut sessions = lock(&self.video_sessions);
                sessions.retain(|entry| {
                    entry.key.program == program && entry.key.generation == generation
                });
                sessions
                    .iter()
                    .position(|entry| entry.key == key)
                    .and_then(|index| sessions.remove(index))
            };
            let mut entry = match entry {
                Some(entry) => entry,
                None => VideoSessionEntry {
                    key,
                    session: media_decoder(source, self.budgets.clone())?
                        .open_video_session_with_output(output, cancellation)?,
                },
            };
            let result = entry.session.frame_at(source_time, cancellation);
            if result.is_ok() {
                let mut sessions = lock(&self.video_sessions);
                sessions.push_back(entry);
                while sessions.len() > VIDEO_SESSION_CACHE_CAPACITY {
                    sessions.pop_front();
                }
            }
            result
        }

        #[allow(clippy::too_many_arguments)]
        fn render_with_size(
            &self,
            program: &PreviewProgram,
            time: RationalTime,
            fixed_size: Option<(u32, u32)>,
            expected_generation: Option<(u64, &AtomicU64)>,
            fail_on_decode_error: bool,
            output: RenderOutput,
            cancellation: &CancellationToken,
        ) -> Result<Presentation, PreviewError> {
            if cancellation.is_cancelled()
                || expected_generation.is_some_and(|(generation, current)| {
                    current.load(Ordering::SeqCst) != generation
                })
            {
                return Err(PreviewError::new(
                    "PREVIEW_CANCELLED",
                    "preview request was superseded",
                ));
            }
            let trace_decode = trace_stage();
            let range = TimeRange::new(time, RationalTime::ZERO)
                .map_err(|error| PreviewError::new("INVALID_PREVIEW_TIME", error.to_string()))?;
            let snapshot = RenderSnapshot::new(program.key.project_id, program.key.revision, range);
            let mut frames = Vec::new();
            let mut decode_error = None;
            let has_active_video = program.video_clips.iter().any(|clip| {
                clip.timeline_start
                    .checked_add(clip.duration)
                    .is_ok_and(|end| time >= clip.timeline_start && time < end)
            });
            if has_active_video && !*FFMPEG_LICENSE_OK.get_or_init(or_media::verify_ffmpeg_runtime)
            {
                return Err(PreviewError::new(
                    "FFMPEG_UNAVAILABLE",
                    "the approved LGPL FFmpeg runtime could not be loaded",
                ));
            }
            if let Some((width, height)) = fixed_size
                && let Some(clip) =
                    direct_video_clip_at(program, time, (width, height)).or_else(|| {
                        matches!(output, RenderOutput::Nv12)
                            .then(|| covers_canvas_clip_at(program, time))
                            .flatten()
                    })
            {
                let offset = time.checked_sub(clip.timeline_start).map_err(|error| {
                    PreviewError::new("INVALID_TIMELINE_TIME", error.to_string())
                })?;
                // Timeline time maps to source time through the clip's rate: a clip at
                // 2x consumes two units of source per unit of timeline, and a reversed
                // clip walks that same span backwards.
                let source_time = clip_source_time(clip, offset)?;
                let runtime_generation = expected_generation.map(|(generation, _)| generation);
                if matches!(output, RenderOutput::Nv12) {
                    let decoded = self.decode_video_frame_nv12(
                        program.key,
                        runtime_generation,
                        &clip.source,
                        source_time,
                        (width, height),
                        cancellation,
                    );
                    match decoded {
                        Ok(Some(frame))
                            if frame.descriptor().pixel_format() == FramePixelFormat::Nv12
                                && frame.descriptor().width() == width
                                && frame.descriptor().height() == height
                                && let Some(planes) = frame.shared_bytes().and_then(|bytes| {
                                    or_render::Nv12Planes::from_shared(width, height, bytes)
                                }) =>
                        {
                            if cancellation.is_cancelled()
                                || expected_generation.is_some_and(|(generation, current)| {
                                    current.load(Ordering::SeqCst) != generation
                                })
                            {
                                return Err(PreviewError::new(
                                    "PREVIEW_CANCELLED",
                                    "preview request was superseded",
                                ));
                            }
                            return Ok(Presentation {
                                width,
                                height,
                                pixels: Vec::new(),
                                planes: Some(planes),
                                pending_planes: None,
                                error_code: None,
                                error_message: None,
                            });
                        }
                        Err(error) if matches!(error, or_media::DecodeError::Cancelled) => {
                            return Err(PreviewError::new("PREVIEW_CANCELLED", error.to_string()));
                        }
                        Err(error) if fail_on_decode_error => {
                            return Err(PreviewError::new(
                                "EXPORT_MEDIA_DECODE_FAILED",
                                error.to_string(),
                            ));
                        }
                        Ok(Some(_)) | Ok(None) | Err(_) => {}
                    }
                }
                // A rotated source refuses the NV12 decode above; the rgba path
                // still turns it into a frame.
                if clip.source_dimensions == Some((width, height)) {
                    match self.decode_video_frame(
                        program.key,
                        runtime_generation,
                        &clip.source,
                        source_time,
                        cancellation,
                    ) {
                        Ok(Some(frame))
                            if frame.descriptor().width() == width
                                && frame.descriptor().height() == height
                                && frame
                                    .pixels()
                                    .as_chunks::<4>()
                                    .0
                                    .iter()
                                    .all(|pixel| pixel[3] == u8::MAX) =>
                        {
                            if cancellation.is_cancelled()
                                || expected_generation.is_some_and(|(generation, current)| {
                                    current.load(Ordering::SeqCst) != generation
                                })
                            {
                                return Err(PreviewError::new(
                                    "PREVIEW_CANCELLED",
                                    "preview request was superseded",
                                ));
                            }
                            if matches!(output, RenderOutput::Nv12) {
                                // The direct path has a full-frame opaque picture and no
                                // layers to composite. When the caller wants NV12 (the
                                // hardware H.264 export), convert it on the GPU and read
                                // back the two planes instead of paying a CPU YUV->NV12
                                // swscale on the writer thread for every frame.
                                let gpu_planes =
                                    RenderSize::new(width, height).ok().and_then(|size| {
                                        let layer =
                                            RgbaVideoLayer::new(width, height, frame.pixels())
                                                .ok()?;
                                        renderer().ok().and_then(|renderer| {
                                            renderer
                                                .render_rgba_layers_nv12(snapshot, size, &[layer])
                                                .ok()
                                        })
                                    });
                                if let Some(planes) = gpu_planes {
                                    return Ok(Presentation {
                                        width,
                                        height,
                                        pixels: Vec::new(),
                                        planes: Some(planes),
                                        pending_planes: None,
                                        error_code: None,
                                        error_message: None,
                                    });
                                }
                            }
                            return Ok(Presentation {
                                width,
                                height,
                                pixels: frame.pixels().to_vec(),
                                planes: None,
                                pending_planes: None,
                                error_code: None,
                                error_message: None,
                            });
                        }
                        Err(error) if matches!(error, or_media::DecodeError::Cancelled) => {
                            return Err(PreviewError::new("PREVIEW_CANCELLED", error.to_string()));
                        }
                        Err(error) if fail_on_decode_error => {
                            return Err(PreviewError::new(
                                "EXPORT_MEDIA_DECODE_FAILED",
                                error.to_string(),
                            ));
                        }
                        Ok(Some(_)) | Ok(None) | Err(_) => {}
                    }
                }
            }
            for clip in &program.video_clips {
                let clip_end = clip
                    .timeline_start
                    .checked_add(clip.duration)
                    .map_err(|error| {
                        PreviewError::new("INVALID_TIMELINE_TIME", error.to_string())
                    })?;
                if time < clip.timeline_start || time >= clip_end {
                    continue;
                }
                let offset = time.checked_sub(clip.timeline_start).map_err(|error| {
                    PreviewError::new("INVALID_TIMELINE_TIME", error.to_string())
                })?;
                let source_time = clip_source_time(clip, offset)?;
                let runtime_generation = expected_generation.map(|(generation, _)| generation);
                // A clip that fills the frame needs no RGBA round trip: decoding
                // it straight to NV12 at the output size lets the layer shader
                // convert to RGB on the GPU, and the export wants NV12 anyway.
                let nv12_size = fixed_size.filter(|_| clip_covers_output(clip));
                let decoded = if let Some(size) = nv12_size {
                    self.decode_video_frame_nv12(
                        program.key,
                        runtime_generation,
                        &clip.source,
                        source_time,
                        size,
                        cancellation,
                    )
                    .map(|frame| frame.map(DecodedVideoFrame::Nv12))
                } else {
                    self.decode_video_frame(
                        program.key,
                        runtime_generation,
                        &clip.source,
                        source_time,
                        cancellation,
                    )
                    .map(|frame| frame.map(DecodedVideoFrame::Rgba))
                };
                match decoded {
                    Ok(Some(frame)) => {
                        frames.push((frame, clip));
                    }
                    Ok(None) => {}
                    Err(error) if matches!(error, or_media::DecodeError::Cancelled) => {
                        return Err(PreviewError::new("PREVIEW_CANCELLED", error.to_string()));
                    }
                    Err(error) => {
                        decode_error.get_or_insert_with(|| error.to_string());
                    }
                }
            }
            if cancellation.is_cancelled()
                || expected_generation.is_some_and(|(generation, current)| {
                    current.load(Ordering::SeqCst) != generation
                })
            {
                return Err(PreviewError::new(
                    "PREVIEW_CANCELLED",
                    "preview request was superseded",
                ));
            }
            if fail_on_decode_error && let Some(message) = &decode_error {
                return Err(PreviewError::new(
                    "EXPORT_MEDIA_DECODE_FAILED",
                    message.clone(),
                ));
            }

            let (width, height) = fixed_size.unwrap_or_else(|| {
                frames
                    .first()
                    .map(|(frame, _)| {
                        fit_size(frame.descriptor().width(), frame.descriptor().height())
                    })
                    .unwrap_or((MAX_BLANK_WIDTH, MAX_BLANK_HEIGHT))
            });
            let size = RenderSize::new(width, height)
                .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?;
            let mut processed_video_pixels = Vec::with_capacity(frames.len());
            let mut processed_bytes = 0usize;
            for (frame, clip) in &frames {
                let transitions = clip_transitions(
                    time,
                    clip.timeline_start,
                    clip.duration,
                    clip.transition_in,
                    clip.transition_out,
                )?;
                let processed = match frame {
                    DecodedVideoFrame::Rgba(rgba) => {
                        if clip.effects.is_empty() && transitions.is_empty() {
                            None
                        } else {
                            let pixels = process_visual_rgba(
                                rgba.descriptor().width(),
                                rgba.descriptor().height(),
                                rgba.pixels(),
                                &clip.effects,
                                &transitions,
                            )
                            .map_err(|error| {
                                PreviewError::new("VISUAL_PROCESS_FAILED", error.to_string())
                            })?;
                            processed_bytes = processed_bytes
                                .checked_add(pixels.len())
                                .filter(|bytes| *bytes <= MAX_VISUAL_PROCESS_BYTES)
                                .ok_or_else(|| {
                                    PreviewError::new(
                                        "VISUAL_PROCESS_LIMIT",
                                        "active video effects exceed the preview memory limit",
                                    )
                                })?;
                            Some(pixels)
                        }
                    }
                    // Only plain covering clips are decoded this way, so there is
                    // never an effect or transition to apply here.
                    DecodedVideoFrame::Nv12(_) => None,
                };
                processed_video_pixels.push(processed);
            }
            let mut layers: Vec<CompositorLayer<'_>> = frames
                .iter()
                .zip(&processed_video_pixels)
                .map(|((frame, clip), processed)| match frame {
                    DecodedVideoFrame::Rgba(rgba) => {
                        let pixels = processed.as_deref().unwrap_or_else(|| rgba.pixels());
                        let (transform, opacity) = animated_settings(
                            &clip.animation,
                            clip.transform,
                            clip.opacity,
                            clip.timeline_start,
                            time,
                        );
                        Ok(CompositorLayer::Rgba(
                            RgbaVideoLayer::new(
                                rgba.descriptor().width(),
                                rgba.descriptor().height(),
                                pixels,
                            )?
                            .with_visual_settings(transform, clip.crop, opacity)?,
                        ))
                    }
                    DecodedVideoFrame::Nv12(nv12) => {
                        let (transform, opacity) = animated_settings(
                            &clip.animation,
                            clip.transform,
                            clip.opacity,
                            clip.timeline_start,
                            time,
                        );
                        Ok(CompositorLayer::Nv12(
                            Nv12VideoLayer::from_contiguous(width, height, nv12.bytes())?
                                .with_visual_settings(transform, clip.crop, opacity)?,
                        ))
                    }
                })
                .collect::<Result<Vec<_>, or_render::RenderError>>()
                .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?;
            trace_record(|trace| &trace.decode, trace_decode);
            let trace_compose = trace_stage();
            let mut active_text_clips = Vec::new();
            for clip in &program.text_clips {
                let end = clip
                    .timeline_start
                    .checked_add(clip.duration)
                    .map_err(|error| {
                        PreviewError::new("INVALID_TIMELINE_TIME", error.to_string())
                    })?;
                if time >= clip.timeline_start && time < end {
                    active_text_clips.push(clip);
                }
            }
            let bytes_per_text_layer = (width as usize)
                .checked_mul(height as usize)
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or_else(|| {
                    PreviewError::new("TEXT_RENDER_LIMIT", "text canvas is too large")
                })?;
            let text_bytes = active_text_clips
                .len()
                .checked_mul(bytes_per_text_layer)
                .filter(|bytes| *bytes <= MAX_TEXT_OVERLAY_BYTES)
                .ok_or_else(|| {
                    PreviewError::new(
                        "TEXT_RENDER_LIMIT",
                        "active text layers exceed the preview memory limit",
                    )
                })?;
            let mut text_pixels: Vec<(
                std::sync::Arc<[u8]>,
                Option<u64>,
                or_render::TextInkBounds,
            )> = Vec::with_capacity(active_text_clips.len());
            for clip in &active_text_clips {
                let rasterized = lock(&self.text_rasterizer)
                    .rasterize_cached(&clip.text, clip.formatting, width, height)
                    .map_err(|error| PreviewError::new("TEXT_RENDER_FAILED", error.to_string()))?;
                let pixels = rasterized.pixels;
                let transitions = clip_transitions(
                    time,
                    clip.timeline_start,
                    clip.duration,
                    clip.transition_in,
                    clip.transition_out,
                )?;
                let ink = rasterized.bounds;
                let processed = if clip.effects.is_empty() && transitions.is_empty() {
                    (pixels, Some(rasterized.id), ink)
                } else {
                    let processed =
                        process_visual_rgba(width, height, &pixels, &clip.effects, &transitions)
                            .map_err(|error| {
                                PreviewError::new("VISUAL_PROCESS_FAILED", error.to_string())
                            })?;
                    processed_bytes = processed_bytes
                        .checked_add(processed.len())
                        .filter(|bytes| *bytes <= MAX_VISUAL_PROCESS_BYTES)
                        .ok_or_else(|| {
                            PreviewError::new(
                                "VISUAL_PROCESS_LIMIT",
                                "active video and text effects exceed the preview memory limit",
                            )
                        })?;
                    (Arc::from(processed), None, ink)
                };
                text_pixels.push(processed);
            }
            debug_assert!(
                text_pixels
                    .iter()
                    .map(|(pixels, _, _)| pixels.len())
                    .sum::<usize>()
                    <= text_bytes
            );
            for (clip, (pixels, cache_key, ink)) in active_text_clips.iter().zip(&text_pixels) {
                // Effects rewrite the pixels and can spread them past the glyph
                // rectangle, so only an untouched rasterisation may be clipped.
                let untouched = cache_key.is_some();
                if untouched && (ink.width == 0 || ink.height == 0) {
                    // Nothing was drawn (an empty string, or text placed entirely
                    // off the canvas), so there is no frame to blend.
                    continue;
                }
                let (transform, opacity) = animated_settings(
                    &clip.animation,
                    clip.transform,
                    clip.opacity,
                    clip.timeline_start,
                    time,
                );
                let mut layer = RgbaVideoLayer::new(width, height, pixels)
                    .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?
                    .with_visual_settings(transform, clip.crop, opacity)
                    .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?;
                if let Some(key) = cache_key {
                    layer = layer.with_cache_key(*key);
                }
                if let Some((x, y, width, height)) = text_scissor(clip, untouched, *ink) {
                    layer = layer.with_scissor(x, y, width, height);
                }
                layers.push(CompositorLayer::Rgba(layer));
            }
            let renderer = renderer()?;
            let (pixels, planes, pending_planes) = match output {
                RenderOutput::Packed => {
                    let rendered = renderer
                        .composite_layers(size, &layers)
                        .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?;
                    (rendered.pixels().to_vec(), None, None)
                }
                RenderOutput::Nv12 => {
                    // Queued, not waited for: the export resolves this once the
                    // frame reaches the writer, which is a frame later, so the
                    // readback overlaps the next frame's decode and compositing.
                    let pending = renderer
                        .composite_layers_nv12_pending(size, &layers)
                        .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?;
                    (Vec::new(), None, Some(pending))
                }
            };
            trace_record(|trace| &trace.compose, trace_compose);
            if cancellation.is_cancelled() {
                return Err(PreviewError::new(
                    "PREVIEW_CANCELLED",
                    "preview request was superseded",
                ));
            }
            Ok(Presentation {
                width,
                height,
                pixels,
                planes,
                pending_planes,
                error_code: decode_error
                    .as_ref()
                    .map(|_| "MEDIA_DECODE_FAILED".to_owned()),
                error_message: decode_error,
            })
        }
    }

    impl PreviewRuntime {
        fn request_is_current(
            &self,
            request: PreviewFrameRequest,
            external_generation: u64,
            cancellation: &CancellationToken,
        ) -> bool {
            if cancellation.is_cancelled()
                || self.generation.load(Ordering::SeqCst) != external_generation
            {
                return false;
            }
            lock(&self.state).transport.snapshot().generation == request.generation
        }

        fn set_error(&self, generation: u64, code: &str, message: String) {
            let mut state = lock(&self.state);
            if state.transport.snapshot().generation == generation {
                state.error_code = Some(code.to_owned());
                state.error_message = Some(message);
            }
        }

        fn snapshot(&self) -> PreviewSnapshot {
            lock(&self.state).snapshot()
        }
    }

    impl Default for PreviewRuntime {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ExportRequestHandler for PreviewRuntime {
        fn handle_export_request(
            &self,
            session: &ProjectSession,
            request: ExportRequest,
        ) -> ExportResponse {
            PreviewRuntime::handle_export_request(self, session, request)
        }
    }

    impl MotionMaterializeHandler for PreviewRuntime {
        #[allow(clippy::let_and_return)]
        fn handle_motion_materialize_request(
            &self,
            identity: ProjectSessionIdentity,
            request: MotionMaterializeRequest,
        ) -> MotionMaterializeResponse {
            let view = match request {
                MotionMaterializeRequest::FromCode {
                    project_id,
                    project_instance_id,
                    source_path,
                    destination,
                    frame_rate_numerator,
                    frame_rate_denominator,
                    width,
                    height,
                    title,
                } => {
                    if project_id != identity.project_id
                        || project_instance_id != identity.project_instance_id
                    {
                        return MotionMaterializeResponse::failure(
                            "PROJECT_INSTANCE_MISMATCH",
                            "motion request belongs to a different project session",
                        );
                    }
                    if !std::path::Path::new(&destination).is_absolute() {
                        return MotionMaterializeResponse::failure(
                            "INVALID_MOTION_DESTINATION",
                            "choose an absolute destination path for the rendered video",
                        );
                    }
                    let view = crate::api::motion::materialize_motion_scene_from_code(
                        source_path,
                        destination,
                        frame_rate_numerator,
                        frame_rate_denominator,
                        width,
                        height,
                        title,
                    );
                    view
                }
                MotionMaterializeRequest::FromScene {
                    project_id,
                    project_instance_id,
                    scene_path,
                    destination,
                    frame_rate_numerator,
                    frame_rate_denominator,
                    width,
                    height,
                } => {
                    if project_id != identity.project_id
                        || project_instance_id != identity.project_instance_id
                    {
                        return MotionMaterializeResponse::failure(
                            "PROJECT_INSTANCE_MISMATCH",
                            "motion request belongs to a different project session",
                        );
                    }
                    if !std::path::Path::new(&destination).is_absolute() {
                        return MotionMaterializeResponse::failure(
                            "INVALID_MOTION_DESTINATION",
                            "choose an absolute destination path for the rendered video",
                        );
                    }
                    let view = crate::api::motion::materialize_motion_scene(
                        scene_path,
                        destination,
                        frame_rate_numerator,
                        frame_rate_denominator,
                        width,
                        height,
                    );
                    view
                }
            };
            let duration =
                RationalTime::new(view.duration_numerator, view.duration_denominator as u32)
                    .unwrap_or(RationalTime::ZERO);
            MotionMaterializeResponse {
                succeeded: view.succeeded,
                error_code: view.code,
                message: view.message,
                output_path: view.output_path,
                scene_id: view.scene_id,
                scene_name: view.scene_name,
                width: view.width,
                height: view.height,
                frame_rate_numerator: view.frame_rate_numerator,
                frame_rate_denominator: view.frame_rate_denominator,
                frame_count: view.frame_count,
                duration,
                bytes_written: view.bytes_written,
            }
        }
    }

    impl Drop for PreviewRuntime {
        fn drop(&mut self) {
            lock(&self.cancellation).cancel();
        }
    }

    struct SessionState {
        key: Option<ProgramKey>,
        program: Option<Arc<PreviewProgram>>,
        audio_playback: Option<AudioPlayback>,
        transport: PreviewTransport,
        width: u32,
        height: u32,
        error_code: Option<String>,
        error_message: Option<String>,
        audio_error: Option<String>,
    }

    impl Default for SessionState {
        fn default() -> Self {
            Self {
                key: None,
                program: None,
                audio_playback: None,
                transport: PreviewTransport::new(None, None),
                width: 0,
                height: 0,
                error_code: None,
                error_message: None,
                audio_error: None,
            }
        }
    }

    impl SessionState {
        fn snapshot(&self) -> PreviewSnapshot {
            PreviewSnapshot {
                playback: self.transport.snapshot(),
                width: self.width,
                height: self.height,
                error_code: self.error_code.clone().or_else(|| {
                    self.audio_error
                        .as_ref()
                        .map(|_| "AUDIO_PLAYBACK_FAILED".to_owned())
                }),
                error_message: self
                    .error_message
                    .clone()
                    .or_else(|| self.audio_error.clone()),
            }
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ProgramKey {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        revision: ProjectRevision,
    }

    struct PreviewProgram {
        key: ProgramKey,
        frame_rate: Option<RationalRate>,
        content_end: Option<RationalTime>,
        canvas: Option<SequenceCanvas>,
        video_clips: Vec<VideoClip>,
        text_clips: Vec<TextClip>,
        audio_clips: Vec<AudioClip>,
    }

    /// Source time for a timeline offset inside one clip.
    ///
    /// A forward clip walks its source from `source_start`; a reversed clip walks the
    /// same span from its end, so the offset mirrors inside the span the clip covers
    /// rather than inside the source file.
    fn clip_source_time(
        clip: &VideoClip,
        offset: RationalTime,
    ) -> Result<RationalTime, PreviewError> {
        mapped_source_time(
            clip.source_start,
            clip.duration,
            clip.playback_rate,
            offset,
            clip.reversed,
            clip.frozen,
        )
    }

    /// The mapping itself, separated from `VideoClip` so it can be exercised without a
    /// whole clip: frozen holds the first frame, reversed mirrors inside the clip span.
    pub(crate) fn mapped_source_time(
        source_start: RationalTime,
        duration: RationalTime,
        playback_rate: RationalRate,
        offset: RationalTime,
        reversed: bool,
        frozen: bool,
    ) -> Result<RationalTime, PreviewError> {
        let invalid =
            |error: or_core::TimeError| PreviewError::new("INVALID_SOURCE_TIME", error.to_string());
        if frozen {
            // A frozen clip shows the frame at its source start for the whole duration.
            return Ok(source_start);
        }
        let scaled = playback_rate.apply(offset).map_err(invalid)?;
        let span = playback_rate.apply(duration).map_err(invalid)?;
        let walked = if reversed {
            span.checked_sub(scaled).map_err(invalid)?
        } else {
            scaled
        };
        source_start.checked_add(walked).map_err(invalid)
    }

    fn direct_video_clip_at(
        program: &PreviewProgram,
        time: RationalTime,
        output_size: (u32, u32),
    ) -> Option<&VideoClip> {
        let clip = covers_canvas_clip_at(program, time)?;
        if clip.source_dimensions != Some(output_size) {
            return None;
        }
        Some(clip)
    }

    /// One decoded video frame, tagged with the layout it was decoded to.
    enum DecodedVideoFrame {
        Rgba(or_media::VideoFrame),
        Nv12(or_media::VideoFrame),
    }

    impl DecodedVideoFrame {
        const fn descriptor(&self) -> &or_runtime::FrameDescriptor {
            match self {
                Self::Rgba(frame) | Self::Nv12(frame) => frame.descriptor(),
            }
        }
    }

    /// True when the clip fills the output frame with no visual remapping.
    ///
    /// Such a clip loses nothing by being decoded straight to NV12 at the output
    /// size: the planes are what the shader wants and what the encoder ends with,
    /// so the software RGBA conversion and the 8 MB upload disappear.
    fn clip_covers_output(clip: &VideoClip) -> bool {
        clip.transform == Transform::IDENTITY
            && clip.crop == Crop::NONE
            && clip.opacity == Opacity::OPAQUE
            && clip.effects.is_empty()
            && clip.transition_in.is_none()
            && clip.transition_out.is_none()
    }

    /// The clip that fills the whole canvas with no compositing work, ignoring
    /// whether its source pixels already match the output size. The NV12 direct
    /// path scales while decoding, so it can serve any source size.
    fn covers_canvas_clip_at(program: &PreviewProgram, time: RationalTime) -> Option<&VideoClip> {
        let active = |start: RationalTime, duration: RationalTime| {
            start
                .checked_add(duration)
                .is_ok_and(|end| time >= start && time < end)
        };
        if program
            .text_clips
            .iter()
            .any(|clip| active(clip.timeline_start, clip.duration))
        {
            return None;
        }
        let mut video_clips = program
            .video_clips
            .iter()
            .filter(|clip| active(clip.timeline_start, clip.duration));
        let clip = video_clips.next()?;
        if video_clips.next().is_some() || !clip_covers_output(clip) {
            return None;
        }
        Some(clip)
    }

    #[cfg(test)]
    mod export_size_tests {
        use super::*;

        fn canvas(width: u32, height: u32) -> SequenceCanvas {
            SequenceCanvas::new(width, height).expect("even, bounded canvas")
        }

        #[test]
        fn a_canvas_exports_at_its_exact_size() {
            assert_eq!(
                export_size(Some(canvas(1080, 1920)), None, None),
                (1080, 1920)
            );
            assert_eq!(
                export_size(Some(canvas(1920, 1080)), None, Some((64, 48))),
                (1920, 1080)
            );
        }

        #[test]
        fn a_short_side_keeps_the_canvas_shape_for_portrait_and_landscape() {
            assert_eq!(
                export_size(Some(canvas(1080, 1920)), Some(720), None),
                (720, 1280)
            );
            // 4:5 at a 1080 short side is already 1080 wide, so the height
            // follows the shape instead of scaling up.
            assert_eq!(
                export_size(Some(canvas(1080, 1350)), Some(1080), None),
                (1080, 1350)
            );
            assert_eq!(
                export_size(Some(canvas(1920, 1080)), Some(480), None),
                (852, 480)
            );
        }

        #[test]
        fn a_project_without_a_canvas_keeps_the_legacy_fit() {
            assert_eq!(export_size(None, None, Some((32, 24))), (32, 24));
            assert_eq!(
                export_size(None, Some(720), Some((3840, 2160))),
                (1280, 720)
            );
            assert_eq!(export_size(None, None, Some((3840, 2160))), (1920, 1080));
        }
    }

    #[cfg(test)]
    mod direct_frame_tests {
        use super::*;

        fn local_media_uri(name: &str) -> String {
            if cfg!(windows) {
                format!("file:///C:/{name}")
            } else {
                format!("file:///{name}")
            }
        }

        fn program() -> PreviewProgram {
            let project = or_core::ProjectDocument::new("identity export");
            PreviewProgram {
                canvas: None,
                key: ProgramKey {
                    project_id: project.id(),
                    project_instance_id: ProjectInstanceId::generate(),
                    revision: project.revision(),
                },
                frame_rate: Some(RationalRate::new(24, 1).unwrap()),
                content_end: Some(RationalTime::new(10, 1).unwrap()),
                video_clips: vec![VideoClip {
                    playback_rate: RationalRate::ONE,
                    reversed: false,
                    frozen: false,
                    source: MediaSourceRef::local_file(local_media_uri("identity.mp4")).unwrap(),
                    source_dimensions: Some((1920, 1080)),
                    timeline_start: RationalTime::ZERO,
                    source_start: RationalTime::ZERO,
                    duration: RationalTime::new(10, 1).unwrap(),
                    transform: Transform::IDENTITY,
                    crop: Crop::NONE,
                    opacity: Opacity::OPAQUE,
                    animation: ClipAnimation::default(),
                    effects: Vec::new(),
                    transition_in: None,
                    transition_out: None,
                }],
                text_clips: Vec::new(),
                audio_clips: Vec::new(),
            }
        }

        #[test]
        fn direct_frame_requires_an_identity_full_size_uncomposited_video() {
            let mut program = program();
            let middle = RationalTime::new(5, 1).unwrap();
            assert!(direct_video_clip_at(&program, middle, (1920, 1080)).is_some());
            assert!(direct_video_clip_at(&program, middle, (1280, 720)).is_none());
            assert!(
                direct_video_clip_at(&program, RationalTime::new(10, 1).unwrap(), (1920, 1080),)
                    .is_none()
            );

            program.video_clips[0].transform.rotation_milli_degrees = 1_000;
            assert!(direct_video_clip_at(&program, middle, (1920, 1080)).is_none());
            program.video_clips[0].transform = Transform::IDENTITY;
            program.video_clips[0].crop.left_basis_points = 1_000;
            assert!(direct_video_clip_at(&program, middle, (1920, 1080)).is_none());
            program.video_clips[0].crop = Crop::NONE;
            program.video_clips[0].opacity.basis_points = 5_000;
            assert!(direct_video_clip_at(&program, middle, (1920, 1080)).is_none());
            program.video_clips[0].opacity = Opacity::OPAQUE;

            program.video_clips.push(VideoClip {
                playback_rate: RationalRate::ONE,
                reversed: false,
                frozen: false,
                source: MediaSourceRef::local_file(local_media_uri("second.mp4")).unwrap(),
                source_dimensions: Some((1920, 1080)),
                timeline_start: RationalTime::ZERO,
                source_start: RationalTime::ZERO,
                duration: RationalTime::new(10, 1).unwrap(),
                transform: Transform::IDENTITY,
                crop: Crop::NONE,
                opacity: Opacity::OPAQUE,
                animation: ClipAnimation::default(),
                effects: Vec::new(),
                transition_in: None,
                transition_out: None,
            });
            assert!(direct_video_clip_at(&program, middle, (1920, 1080)).is_none());
            program.video_clips.pop();
            program.text_clips.push(TextClip {
                text: "title".to_owned(),
                formatting: TextFormatting::default(),
                timeline_start: RationalTime::new(4, 1).unwrap(),
                duration: RationalTime::new(2, 1).unwrap(),
                transform: Transform::IDENTITY,
                crop: Crop::NONE,
                opacity: Opacity::OPAQUE,
                animation: ClipAnimation::default(),
                effects: Vec::new(),
                transition_in: None,
                transition_out: None,
            });
            assert!(direct_video_clip_at(&program, middle, (1920, 1080)).is_none());
        }
    }

    impl PreviewProgram {
        fn active_saf_sources(&self, time: RationalTime) -> Result<Vec<String>, PreviewError> {
            let mut sources = self
                .video_clips
                .iter()
                .filter(|clip| {
                    clip.timeline_start
                        .checked_add(clip.duration)
                        .is_ok_and(|end| time >= clip.timeline_start && time < end)
                })
                .filter_map(|clip| match &clip.source {
                    MediaSourceRef::AndroidSafDocumentUri { uri } => Some(uri.as_str().to_owned()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            sources.sort_unstable();
            sources.dedup();
            if sources.len() > 64 {
                return Err(PreviewError::new(
                    "MEDIA_SOURCE_BUDGET_EXCEEDED",
                    "This preview frame requires more than 64 Android media sources.",
                ));
            }
            Ok(sources)
        }

        fn playback_saf_sources(&self, time: RationalTime) -> Result<Vec<String>, PreviewError> {
            let mut sources = self.active_saf_sources(time)?;
            sources.extend(
                self.audio_clips
                    .iter()
                    .filter(|clip| {
                        clip.timeline_start
                            .checked_add(clip.duration)
                            .is_ok_and(|end| end > time)
                    })
                    .filter_map(|clip| match &clip.source {
                        MediaSourceRef::AndroidSafDocumentUri { uri } => {
                            Some(uri.as_str().to_owned())
                        }
                        _ => None,
                    }),
            );
            sources.sort_unstable();
            sources.dedup();
            if sources.len() > 64 {
                return Err(PreviewError::new(
                    "MEDIA_SOURCE_BUDGET_EXCEEDED",
                    "This playback requires more than 64 Android media sources.",
                ));
            }
            Ok(sources)
        }
    }

    fn stale_preparation() -> PreviewError {
        PreviewError::new(
            "STALE_PREVIEW_REQUEST",
            "A newer preview request replaced this frame.",
        )
    }

    struct VideoClip {
        source: MediaSourceRef,
        source_dimensions: Option<(u32, u32)>,
        timeline_start: RationalTime,
        source_start: RationalTime,
        duration: RationalTime,
        playback_rate: RationalRate,
        /// Set when the clip walks its source backwards; the source-time mapping
        /// mirrors inside the clip's own source range.
        reversed: bool,
        /// Set when the clip holds one frame: every offset maps to the source start,
        /// so the picture stops while the clip keeps its duration.
        frozen: bool,
        transform: Transform,
        crop: Crop,
        opacity: Opacity,
        animation: ClipAnimation,
        effects: Vec<EffectReference>,
        transition_in: Option<TransitionReference>,
        transition_out: Option<TransitionReference>,
    }

    /// The part of the canvas a text layer can paint, when clipping it is safe.
    ///
    /// A rasterisation is a full-canvas buffer that is transparent almost
    /// everywhere, so blending the rest of the frame is wasted fill. Only an
    /// untouched rasterisation of an untransformed, uncropped clip qualifies:
    /// effects can spread pixels past the glyph rectangle, and a moved or scaled
    /// layer paints outside it.
    pub(super) fn text_scissor(
        clip: &TextClip,
        untouched: bool,
        ink: or_render::TextInkBounds,
    ) -> Option<(u32, u32, u32, u32)> {
        (untouched
            && clip.transform == or_core::Transform::IDENTITY
            && clip.crop == or_core::Crop::NONE
            && ink.width > 0
            && ink.height > 0)
            .then_some((ink.x, ink.y, ink.width, ink.height))
    }

    pub(super) struct TextClip {
        pub(super) text: String,
        pub(super) formatting: TextFormatting,
        pub(super) timeline_start: RationalTime,
        pub(super) duration: RationalTime,
        pub(super) transform: Transform,
        pub(super) crop: Crop,
        pub(super) opacity: Opacity,
        pub(super) animation: ClipAnimation,
        pub(super) effects: Vec<EffectReference>,
        pub(super) transition_in: Option<TransitionReference>,
        pub(super) transition_out: Option<TransitionReference>,
    }

    /// The transform and opacity a clip has at `time`, after animation.
    ///
    /// A clip with no keyframes returns its static values, so the common case
    /// costs one empty-vector check per layer.
    fn animated_settings(
        animation: &ClipAnimation,
        transform: Transform,
        opacity: Opacity,
        timeline_start: RationalTime,
        time: RationalTime,
    ) -> (Transform, Opacity) {
        if animation.is_empty() {
            return (transform, opacity);
        }
        let elapsed = time
            .checked_sub(timeline_start)
            .unwrap_or(RationalTime::zero());
        animation.sample(transform, opacity, elapsed)
    }

    #[derive(Clone)]
    struct AudioClip {
        /// Stable identity so a clip's noise gate can keep its state from one
        /// render block to the next.
        id: String,
        #[allow(dead_code)]
        playback_rate: RationalRate,
        source: MediaSourceRef,
        timeline_start: RationalTime,
        source_start: RationalTime,
        duration: RationalTime,
        settings: AudioSettings,
    }

    struct Presentation {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
        /// Set when the caller asked for NV12 output: the same composition in the
        /// layout the hardware encoder takes, converted on the GPU.
        planes: Option<or_render::Nv12Planes>,
        /// Set instead of `planes` when the conversion was queued but not waited
        /// for. The export resolves it on the writer thread, so this frame's
        /// colour conversion runs while the renderer decodes the next one.
        pending_planes: Option<or_render::PendingNv12Planes>,
        error_code: Option<String>,
        error_message: Option<String>,
    }

    impl Presentation {
        /// Waits for a queued readback, if any, and leaves the planes in place.
        ///
        /// Every consumer that wants pixels has to call this: the export does it
        /// on the writer thread, and a measurement of the render stage does it
        /// right after the frame, so a queued conversion is never mistaken for a
        /// finished one.
        fn resolve_planes(&mut self) -> Result<(), PreviewError> {
            if let Some(pending) = self.pending_planes.take() {
                self.planes = Some(
                    pending
                        .resolve()
                        .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?,
                );
            }
            Ok(())
        }
    }

    /// Which CPU layout a render request wants back.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum RenderOutput {
        /// Packed RGBA, for the viewer texture and the lossless encoder.
        Packed,
        /// NV12 planes, for the hardware H.264 encoder.
        Nv12,
    }

    fn load_program(
        host: &LiveProjectHost,
        key: ProgramKey,
    ) -> Result<PreviewProgram, PreviewError> {
        load_program_with_query(key, |request| query(host, request))
    }

    fn load_program_from_session(session: &ProjectSession) -> Result<PreviewProgram, PreviewError> {
        let key = ProgramKey {
            project_id: session.project_id(),
            project_instance_id: session.project_instance_id(),
            revision: session.project_revision(),
        };
        load_program_with_query(key, |request| {
            session
                .execute_query(request)
                .map_err(|error| PreviewError::new("PREVIEW_QUERY_FAILED", error.to_string()))
        })
    }

    fn load_program_with_query(
        key: ProgramKey,
        mut query_request: impl FnMut(QueryEnvelope) -> Result<QueryResult, PreviewError>,
    ) -> Result<PreviewProgram, PreviewError> {
        let mut media_sources = HashMap::new();
        let mut media_dimensions = HashMap::new();
        let mut media_with_audio = HashSet::new();
        let mut offset = 0usize;
        loop {
            let result = query_request(QueryEnvelope::media_list(
                key.project_id,
                key.project_instance_id,
                offset,
                PAGE_SIZE,
            ))?;
            ensure_key(&result, key)?;
            let page = result.media_page.ok_or_else(|| {
                PreviewError::new("PREVIEW_QUERY_FAILED", "media page was missing")
            })?;
            for item in page.items {
                let dimensions = item
                    .metadata()
                    .streams()
                    .iter()
                    .find_map(|stream| match stream {
                        MediaStreamMetadata::Video(video) => Some((video.width(), video.height())),
                        MediaStreamMetadata::Audio(_) | MediaStreamMetadata::Other(_) => None,
                    });
                if let Some(dimensions) = dimensions {
                    media_dimensions.insert(item.id(), dimensions);
                }
                if item
                    .metadata()
                    .streams()
                    .iter()
                    .any(|stream| matches!(stream, MediaStreamMetadata::Audio(_)))
                {
                    media_with_audio.insert(item.id());
                }
                media_sources.insert(item.id(), item.source().clone());
            }
            let Some(next) = page.next_offset else { break };
            if next <= offset {
                return Err(PreviewError::new(
                    "PREVIEW_QUERY_FAILED",
                    "media pagination did not advance",
                ));
            }
            offset = next;
        }

        let result = query_request(QueryEnvelope::timeline_tracks_v2(
            key.project_id,
            key.project_instance_id,
        ))?;
        ensure_key(&result, key)?;
        let tracks = result.timeline_tracks_v2.ok_or_else(|| {
            PreviewError::new("PREVIEW_QUERY_FAILED", "timeline tracks were missing")
        })?;
        let mut content_end = None;
        let mut video_clips = Vec::new();
        let mut text_clips = Vec::new();
        let mut audio_clips = Vec::new();
        let has_visual_solo = tracks.iter().any(|track| {
            matches!(
                track.kind,
                TrackKind::Video | TrackKind::Text | TrackKind::Caption
            ) && track.state.solo()
        });
        let has_audio_solo = tracks
            .iter()
            .any(|track| track.kind == TrackKind::Audio && track.state.solo());
        for track in tracks {
            let render_track = match track.kind {
                TrackKind::Audio => !track.state.muted() && (!has_audio_solo || track.state.solo()),
                TrackKind::Video | TrackKind::Text | TrackKind::Caption => {
                    track.state.visible() && (!has_visual_solo || track.state.solo())
                }
            };
            for clip in list_clips(key, &track, &mut query_request)? {
                add_clip(
                    &mut content_end,
                    &mut video_clips,
                    &mut text_clips,
                    &mut audio_clips,
                    track.kind,
                    render_track,
                    clip,
                    &media_sources,
                    &media_dimensions,
                    &media_with_audio,
                )?;
            }
        }

        let result = query_request(QueryEnvelope::timeline_sequence_settings(
            key.project_id,
            key.project_instance_id,
        ))?;
        ensure_key(&result, key)?;
        let settings = result.timeline_sequence_settings.ok_or_else(|| {
            PreviewError::new("PREVIEW_QUERY_FAILED", "sequence settings were missing")
        })?;
        Ok(PreviewProgram {
            key,
            frame_rate: settings.sequence_frame_rate,
            content_end,
            canvas: settings.sequence_canvas,
            video_clips,
            text_clips,
            audio_clips,
        })
    }

    fn list_clips(
        key: ProgramKey,
        track: &TimelineTrackSummaryV2,
        query_request: &mut impl FnMut(QueryEnvelope) -> Result<QueryResult, PreviewError>,
    ) -> Result<Vec<TimelineClipState>, PreviewError> {
        let mut clips = Vec::new();
        let mut offset = 0usize;
        loop {
            let result = query_request(QueryEnvelope::timeline_clips_v2(
                key.project_id,
                key.project_instance_id,
                track.track_id,
                offset,
                MAX_TIMELINE_CLIP_PAGE_SIZE,
            ))?;
            ensure_key(&result, key)?;
            let page = result.timeline_clip_page_v2.ok_or_else(|| {
                PreviewError::new("PREVIEW_QUERY_FAILED", "timeline clip page was missing")
            })?;
            clips.extend(page.items);
            let Some(next) = page.next_offset else { break };
            if next <= offset {
                return Err(PreviewError::new(
                    "PREVIEW_QUERY_FAILED",
                    "timeline pagination did not advance",
                ));
            }
            offset = next;
        }
        Ok(clips)
    }

    #[allow(clippy::too_many_arguments)]
    fn add_clip(
        content_end: &mut Option<RationalTime>,
        video_clips: &mut Vec<VideoClip>,
        text_clips: &mut Vec<TextClip>,
        audio_clips: &mut Vec<AudioClip>,
        track_kind: TrackKind,
        render_track: bool,
        clip: TimelineClipState,
        sources: &HashMap<MediaId, MediaSourceRef>,
        media_dimensions: &HashMap<MediaId, (u32, u32)>,
        media_with_audio: &HashSet<MediaId>,
    ) -> Result<(), PreviewError> {
        let end = clip
            .timeline_start
            .checked_add(clip.timeline_duration)
            .map_err(|error| PreviewError::new("INVALID_TIMELINE_TIME", error.to_string()))?;
        *content_end = Some(content_end.map_or(end, |current| current.max(end)));
        if !render_track {
            return Ok(());
        }
        match (track_kind, clip.content, clip.settings) {
            (
                TrackKind::Video,
                ClipContent::Media {
                    media_id,
                    source_range,
                    playback_rate,
                    reversed,
                    frozen,
                },
                ClipSettings::Visual(settings),
            ) => {
                let source = sources.get(&media_id).cloned().ok_or_else(|| {
                    PreviewError::new("PREVIEW_QUERY_FAILED", "timeline media source was missing")
                })?;
                video_clips.push(VideoClip {
                    source: source.clone(),
                    source_dimensions: media_dimensions.get(&media_id).copied(),
                    timeline_start: clip.timeline_start,
                    source_start: source_range.start(),
                    duration: clip.timeline_duration,
                    playback_rate,
                    reversed,
                    frozen,
                    transform: settings.transform,
                    crop: settings.crop,
                    opacity: settings.opacity,
                    animation: settings.animation,
                    effects: settings.effects,
                    transition_in: settings.transition_in,
                    transition_out: settings.transition_out,
                });
                // A video clip's own audio is mixed like an audio clip's, using
                // the settings stored on the visual clip.
                if media_with_audio.contains(&media_id) {
                    audio_clips.push(AudioClip {
                        id: clip.clip_id.to_string(),
                        source,
                        timeline_start: clip.timeline_start,
                        source_start: source_range.start(),
                        duration: clip.timeline_duration,
                        playback_rate,
                        settings: settings.audio,
                    });
                }
            }
            (
                TrackKind::Text,
                ClipContent::Text { text, formatting },
                ClipSettings::Visual(settings),
            )
            | (
                TrackKind::Caption,
                ClipContent::Caption { text, formatting },
                ClipSettings::Visual(settings),
            ) => {
                text_clips.push(TextClip {
                    text,
                    formatting,
                    timeline_start: clip.timeline_start,
                    duration: clip.timeline_duration,
                    transform: settings.transform,
                    crop: settings.crop,
                    opacity: settings.opacity,
                    animation: settings.animation,
                    effects: settings.effects,
                    transition_in: settings.transition_in,
                    transition_out: settings.transition_out,
                });
            }
            (
                TrackKind::Audio,
                ClipContent::Media {
                    media_id,
                    source_range,
                    playback_rate,
                    reversed: _,
                    frozen: _,
                },
                ClipSettings::Audio(settings),
            ) => {
                let source = sources.get(&media_id).cloned().ok_or_else(|| {
                    PreviewError::new("PREVIEW_QUERY_FAILED", "timeline media source was missing")
                })?;
                audio_clips.push(AudioClip {
                    id: clip.clip_id.to_string(),
                    source,
                    timeline_start: clip.timeline_start,
                    source_start: source_range.start(),
                    duration: clip.timeline_duration,
                    playback_rate,
                    settings,
                });
            }
            _ => {
                return Err(PreviewError::new(
                    "PREVIEW_QUERY_FAILED",
                    "timeline track contained unsupported clip content",
                ));
            }
        }
        Ok(())
    }

    fn query(host: &LiveProjectHost, request: QueryEnvelope) -> Result<QueryResult, PreviewError> {
        match host
            .handle_application_request(ApplicationRequest::Query(request))
            .map_err(|error| PreviewError::new("PREVIEW_QUERY_FAILED", error.to_string()))?
        {
            ApplicationResponse::Query(result) => Ok(result),
            ApplicationResponse::Error(error) => {
                Err(PreviewError::new("PREVIEW_QUERY_FAILED", error.to_string()))
            }
            _ => Err(PreviewError::new(
                "PREVIEW_QUERY_FAILED",
                "project host returned an unexpected preview query response",
            )),
        }
    }

    fn ensure_key(result: &QueryResult, key: ProgramKey) -> Result<(), PreviewError> {
        if result.summary.project_id != key.project_id
            || result.summary.project_instance_id != key.project_instance_id
            || result.summary.project_revision != key.revision
        {
            return Err(PreviewError::new(
                "STALE_PREVIEW_SNAPSHOT",
                "the project changed while preview state was loading; refresh and retry",
            ));
        }
        Ok(())
    }

    /// The adapter the shared renderer actually opened.
    ///
    /// Opening it here is intentional: the report describes the device the
    /// preview and the export will use, not a list of devices that might work.
    pub(crate) fn render_adapter_info() -> Result<or_render::RenderAdapterInfo, String> {
        renderer()
            .map(|device| device.adapter_info().clone())
            .map_err(|error| error.to_string())
    }

    fn renderer() -> Result<&'static RenderDevice, PreviewError> {
        RENDER_DEVICE
            .get_or_init(|| {
                let preferred = preferred_backend();
                let device = match preferred
                    .as_deref()
                    .and_then(or_render::RenderDevice::backend_flag)
                {
                    Some(backends) => block_on(RenderDevice::new_with_backends(backends))
                        .or_else(|_| block_on(RenderDevice::new())),
                    None => block_on(RenderDevice::new()),
                };
                device.map_err(|error| error.to_string())
            })
            .as_ref()
            .map_err(|message| PreviewError::new("RENDER_UNAVAILABLE", message.clone()))
    }

    /// The backend the person chose, or `None` for automatic selection.
    ///
    /// The device is process-global and opened once. Reading the preference
    /// here keeps the choice out of every frame: a change made in Settings is
    /// stored for the next start instead of rebuilding a live device.
    fn preferred_backend() -> Option<String> {
        let value = GRAPHICS_PREFERENCE
            .lock()
            .ok()
            .and_then(|guard| guard.clone())?;
        (value != "auto").then_some(value)
    }

    /// Preview renders a portrait or landscape canvas at its own aspect, capped
    /// on the long edge so the texture stays inside the viewer budget.
    fn preview_canvas_size(canvas: SequenceCanvas) -> (u32, u32) {
        let width = f64::from(canvas.width());
        let height = f64::from(canvas.height());
        let long_edge = width.max(height);
        let scale = (f64::from(MAX_VIEWER_WIDTH) / long_edge).min(1.0);
        (
            even_dimension(width * scale),
            even_dimension(height * scale),
        )
    }

    /// Resolves the export frame size: the canvas when the project sets one,
    /// otherwise the first video clip fit inside the viewer budget (historical
    /// behaviour). `short_side` scales the natural aspect to a delivery height.
    fn export_size(
        canvas: Option<SequenceCanvas>,
        short_side: Option<u32>,
        clip_dimensions: Option<(u32, u32)>,
    ) -> (u32, u32) {
        let (natural_width, natural_height) = match (canvas, clip_dimensions) {
            (Some(canvas), _) => (canvas.width(), canvas.height()),
            (None, Some((width, height))) => (width, height),
            (None, None) => (MAX_BLANK_WIDTH, MAX_BLANK_HEIGHT),
        };
        let Some(short_side) = short_side.map(|side| even_dimension(f64::from(side))) else {
            // A project canvas is an exact composition size; a legacy project
            // without one keeps the long-standing fit inside 1920x1080.
            return if canvas.is_some() {
                (natural_width, natural_height)
            } else {
                fit_size(natural_width, natural_height)
            };
        };
        let (width, height) = if natural_width >= natural_height {
            (
                even_dimension(
                    f64::from(short_side) * f64::from(natural_width) / f64::from(natural_height),
                ),
                short_side,
            )
        } else {
            (
                short_side,
                even_dimension(
                    f64::from(short_side) * f64::from(natural_height) / f64::from(natural_width),
                ),
            )
        };
        (
            even_dimension(f64::from(width)),
            even_dimension(f64::from(height)),
        )
    }

    fn even_dimension(value: f64) -> u32 {
        let rounded = value.round().max(f64::from(SequenceCanvas::MIN_SIDE));
        let even = (rounded as u32) & !1;
        even.max(SequenceCanvas::MIN_SIDE)
    }

    fn fit_size(width: u32, height: u32) -> (u32, u32) {
        let scale = (MAX_VIEWER_WIDTH as f64 / f64::from(width))
            .min(MAX_VIEWER_HEIGHT as f64 / f64::from(height))
            .min(1.0);
        (
            (f64::from(width) * scale).round().max(1.0) as u32,
            (f64::from(height) * scale).round().max(1.0) as u32,
        )
    }

    fn clip_transitions(
        time: RationalTime,
        start: RationalTime,
        duration: RationalTime,
        transition_in: Option<TransitionReference>,
        transition_out: Option<TransitionReference>,
    ) -> Result<Vec<VisualTransition>, PreviewError> {
        let mut transitions = Vec::with_capacity(2);
        if let Some(transition) = transition_in {
            let elapsed = time
                .checked_sub(start)
                .map_err(|error| PreviewError::new("INVALID_TIMELINE_TIME", error.to_string()))?;
            if elapsed <= transition.duration {
                transitions.push(VisualTransition {
                    kind: transition.kind,
                    visibility_basis_points: transition_visibility(elapsed, transition.duration)?,
                    entering: true,
                });
            }
        }
        if let Some(transition) = transition_out {
            let end = start
                .checked_add(duration)
                .map_err(|error| PreviewError::new("INVALID_TIMELINE_TIME", error.to_string()))?;
            let remaining = end
                .checked_sub(time)
                .map_err(|error| PreviewError::new("INVALID_TIMELINE_TIME", error.to_string()))?;
            if remaining <= transition.duration {
                transitions.push(VisualTransition {
                    kind: transition.kind,
                    visibility_basis_points: transition_visibility(remaining, transition.duration)?,
                    entering: false,
                });
            }
        }
        Ok(transitions)
    }

    fn transition_visibility(
        elapsed: RationalTime,
        duration: RationalTime,
    ) -> Result<u16, PreviewError> {
        let numerator = i128::from(elapsed.numerator()) * i128::from(duration.denominator());
        let denominator = i128::from(elapsed.denominator()) * i128::from(duration.numerator());
        if denominator <= 0 {
            return Err(PreviewError::new(
                "INVALID_TRANSITION_DURATION",
                "transition duration must be positive",
            ));
        }
        let scaled = numerator.max(0) * 10_000 / denominator;
        Ok(scaled.clamp(0, 10_000) as u16)
    }

    fn transport_error(error: or_runtime::PreviewTransportError) -> PreviewError {
        let code = match error {
            or_runtime::PreviewTransportError::NegativeTime => "NEGATIVE_PREVIEW_TIME",
            or_runtime::PreviewTransportError::FrameRateUnavailable => {
                "SEQUENCE_FRAME_RATE_REQUIRED"
            }
            or_runtime::PreviewTransportError::GenerationOverflow => "PREVIEW_GENERATION_OVERFLOW",
            or_runtime::PreviewTransportError::ClockOverflow => "PREVIEW_CLOCK_OVERFLOW",
            or_runtime::PreviewTransportError::Time(_) => "INVALID_PREVIEW_TIME",
        };
        PreviewError::new(code, error.to_string())
    }

    fn export_request_matches_session(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        session: &ProjectSession,
    ) -> bool {
        project_id == session.project_id() && project_instance_id == session.project_instance_id()
    }

    /// Per-stage cost of one export frame, collected when `OR_EXPORT_TRACE=1`.
    ///
    /// The export overlaps decode, compositing, the NV12 readback and the
    /// encoder, so a single frames-per-second number cannot say which of them
    /// is the bound. This records each stage separately and reports median and
    /// p95 when the job finishes, which is what a measurement run needs and
    /// what a normal run must not pay for.
    #[derive(Default)]
    struct ExportStageTrace {
        decode: Mutex<Vec<f64>>,
        compose: Mutex<Vec<f64>>,
        resolve: Mutex<Vec<f64>>,
        encode: Mutex<Vec<f64>>,
    }

    impl ExportStageTrace {
        fn record(stage: &Mutex<Vec<f64>>, started: std::time::Instant) {
            let mut samples = stage
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        }

        fn report(&self, label: &str) {
            println!("OR_EXPORT_TRACE job={label}");
            for (name, stage) in [
                ("decode", &self.decode),
                ("compose", &self.compose),
                ("resolve", &self.resolve),
                ("encode", &self.encode),
            ] {
                let samples = stage
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if samples.is_empty() {
                    continue;
                }
                let mut sorted = samples.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let median = sorted[sorted.len() / 2];
                let p95 = sorted[(sorted.len() * 95 / 100).min(sorted.len() - 1)];
                println!(
                    "OR_EXPORT_TRACE {name}_median_ms={median:.3} {name}_p95_ms={p95:.3} {name}_min_ms={:.3} n={}",
                    sorted[0],
                    sorted.len()
                );
            }
        }
    }

    thread_local! {
        /// Set on the export's render thread so `render_with_size` can time its
        /// own decode and compose halves without changing its signature.
        static ACTIVE_EXPORT_TRACE: std::cell::RefCell<Option<Arc<ExportStageTrace>>> =
            const { std::cell::RefCell::new(None) };
    }

    fn export_trace() -> Option<Arc<ExportStageTrace>> {
        matches!(std::env::var("OR_EXPORT_TRACE").as_deref(), Ok("1"))
            .then(|| Arc::new(ExportStageTrace::default()))
    }

    /// Starts a stage clock when tracing is active on this thread.
    fn trace_stage() -> Option<std::time::Instant> {
        ACTIVE_EXPORT_TRACE
            .with(|trace| trace.borrow().is_some())
            .then(std::time::Instant::now)
    }

    fn trace_record(
        stage: fn(&ExportStageTrace) -> &Mutex<Vec<f64>>,
        started: Option<std::time::Instant>,
    ) {
        if let Some(started) = started {
            ACTIVE_EXPORT_TRACE.with(|trace| {
                if let Some(trace) = trace.borrow().as_ref() {
                    ExportStageTrace::record(stage(trace), started);
                }
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn export_program(
        program: Arc<PreviewProgram>,
        destination: std::path::PathBuf,
        profile: ExportProfile,
        quality: ExportQuality,
        performance: or_core::ExportPerformance,
        output_size: (u32, u32),
        frame_count: u64,
        frame_rate: RationalRate,
        render_resources: Arc<RenderResources>,
        job: &JobContext,
        cancellation: &CancellationToken,
    ) -> Result<(), String> {
        let content_end = program
            .content_end
            .ok_or_else(|| "timeline duration is unavailable".to_owned())?;
        let total_audio_frames = audio_frames_ceil(content_end)?;
        i64::try_from(total_audio_frames)
            .map_err(|_| "audio duration exceeds the output sample clock".to_owned())?;
        let audio_cache = if estimated_export_audio_bytes(&program)? > MAX_EXPORT_AUDIO_CACHE_BYTES
        {
            None
        } else {
            Some(prepare_export_audio_cache(
                &program,
                &render_resources.budgets,
                cancellation,
            )?)
        };
        let media_profile = match profile {
            ExportProfile::MatroskaFfv1PcmS16le => {
                or_media::FfmpegExportProfile::MatroskaFfv1PcmS16le
            }
            ExportProfile::WebmVp9Opus => or_media::FfmpegExportProfile::WebmVp9Opus,
            ExportProfile::Mp4H264Aac => or_media::FfmpegExportProfile::Mp4H264Aac,
        };
        let mut writer = or_media::FfmpegExportWriter::create_with_options(
            destination,
            or_media::ExportWriterOptions {
                profile: media_profile,
                quality,
                performance: performance.profile,
                preferred_h264_encoder: performance.preferred_h264_encoder.as_deref(),
                width: output_size.0,
                height: output_size.1,
                frame_rate_numerator: frame_rate.numerator(),
                frame_rate_denominator: frame_rate.denominator(),
            },
        )
        .map_err(|error| error.to_string())?;
        if let Some(plan) = writer.encoding_plan() {
            // The receipt names the encoder this job actually opened, not the
            // one the dialog previewed, so a fallback is visible after the fact.
            let mut detail = format!("Encoding with {}", plan.summary());
            if let Some(reason) = &plan.fallback {
                detail.push_str(&format!(" — {reason}"));
            } else if let Some(note) = plan.notes.first() {
                detail.push_str(&format!(" — {note}"));
            }
            job.report_detail(format!("{detail}."));
        }
        let mut audio_cursor = 0_u64;
        job.report_progress(0, frame_count);

        // Rendering a frame and writing one share no state, so the renderer runs
        // a bounded number of frames ahead of the writer: the encoder turns frame
        // N into packets while the renderer decodes and composites N+1, instead of
        // the two halves taking turns. Frames still reach the writer in index
        // order, and audio blocks are still written by this thread between them,
        // so the file is the one a frame-at-a-time loop produces.
        let render_output = if media_profile == or_media::FfmpegExportProfile::Mp4H264Aac {
            RenderOutput::Nv12
        } else {
            RenderOutput::Packed
        };
        let trace = export_trace();
        let written = std::thread::scope(|scope| -> Result<(), String> {
            let (sender, receiver) = std::sync::mpsc::sync_channel::<(
                u64,
                Result<Presentation, String>,
            )>(export_render_ahead());
            let render_program = Arc::clone(&program);
            let render_resources_for_render = Arc::clone(&render_resources);
            let trace_for_render = trace.clone();
            scope.spawn(move || {
                ACTIVE_EXPORT_TRACE.with(|slot| *slot.borrow_mut() = trace_for_render);
                for frame_index in 0..frame_count {
                    if job.is_cancelled() || cancellation.is_cancelled() {
                        return;
                    }
                    let produced = frame_rate
                        .frame_time(frame_index)
                        .map_err(|error| error.to_string())
                        .and_then(|time| {
                            render_resources_for_render
                                .render_with_size(
                                    &render_program,
                                    time,
                                    Some(output_size),
                                    None,
                                    true,
                                    render_output,
                                    cancellation,
                                )
                                .map_err(|error| error.to_string())
                        });
                    // A closed channel means the writer stopped; there is no one
                    // left to hand this frame to.
                    if sender.send((frame_index, produced)).is_err() {
                        return;
                    }
                }
            });

            for frame_index in 0..frame_count {
                if job.is_cancelled() || cancellation.is_cancelled() {
                    return Err("export was cancelled".to_owned());
                }
                let (rendered_index, frame) = receiver
                    .recv()
                    .map_err(|_| "the renderer stopped before the last frame".to_owned())?;
                if rendered_index != frame_index {
                    return Err("the renderer produced frames out of order".to_owned());
                }
                let mut presentation = frame?;
                // The conversion was queued a frame ago; waiting for it here
                // keeps the render thread decoding instead of blocking on the GPU.
                let resolve_started = std::time::Instant::now();
                presentation
                    .resolve_planes()
                    .map_err(|error| error.to_string())?;
                if let Some(trace) = &trace {
                    ExportStageTrace::record(&trace.resolve, resolve_started);
                }
                let frame = ExportFrame::from(presentation);
                let encode_started = std::time::Instant::now();
                match &frame.planes {
                    Some(planes) => writer.write_video_frame_nv12(
                        planes.luma(),
                        planes.chroma(),
                        frame.width,
                        frame.height,
                        frame_index,
                    ),
                    None => writer.write_video_frame(
                        &frame.pixels,
                        frame.width,
                        frame.height,
                        frame_index,
                    ),
                }
                .map_err(|error| error.to_string())?;
                if let Some(trace) = &trace {
                    ExportStageTrace::record(&trace.encode, encode_started);
                }
                let next_time = frame_rate
                    .frame_time(frame_index.saturating_add(1))
                    .map_err(|error| error.to_string())?;
                let audio_until = audio_frames_floor(next_time)
                    .map_err(|_| "audio output time exceeds the sample clock".to_owned())?
                    .max(0) as u64;
                write_export_audio_until(
                    &mut writer,
                    &program,
                    audio_cache.as_deref(),
                    &render_resources.budgets,
                    &mut audio_cursor,
                    audio_until.min(total_audio_frames),
                    job,
                    cancellation,
                )?;
                job.report_progress(frame_index + 1, frame_count);
            }
            write_export_audio_until(
                &mut writer,
                &program,
                audio_cache.as_deref(),
                &render_resources.budgets,
                &mut audio_cursor,
                total_audio_frames,
                job,
                cancellation,
            )?;
            Ok(())
        });
        written?;
        if let Some(trace) = &trace {
            trace.report("export");
        }
        if job.is_cancelled() || cancellation.is_cancelled() {
            return Err("export was cancelled".to_owned());
        }
        writer.finish().map_err(|error| error.to_string())
    }

    /// One finished export frame: what the writer encodes, without a render
    /// handle, so it can cross the render thread's boundary.
    struct ExportFrame {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
        planes: Option<or_render::Nv12Planes>,
    }

    impl From<Presentation> for ExportFrame {
        fn from(presentation: Presentation) -> Self {
            Self {
                width: presentation.width,
                height: presentation.height,
                pixels: presentation.pixels,
                planes: presentation.planes,
            }
        }
    }

    /// One decoded audio chunk and the source time of its first sample.
    struct CachedAudioChunk {
        timestamp: RationalTime,
        samples: Vec<f32>,
    }

    /// Decoded PCM for one audio clip. Chunks keep their own timestamps so a
    /// decoder gap or a non-contiguous source stays aligned exactly as the
    /// streaming path mixed it, instead of assuming the samples are contiguous.
    #[derive(Default)]
    struct CachedAudioClip {
        chunks: Vec<CachedAudioChunk>,
    }

    impl CachedAudioClip {
        fn bytes(&self) -> usize {
            self.chunks.iter().fold(0_usize, |total, chunk| {
                total.saturating_add(
                    chunk
                        .samples
                        .len()
                        .saturating_mul(std::mem::size_of::<f32>()),
                )
            })
        }
    }

    /// Estimates the decoded size of one audio clip, in bytes.
    fn estimated_audio_clip_bytes(clip: &AudioClip) -> Result<usize, String> {
        let source_duration = clip
            .playback_rate
            .apply(clip.duration)
            .map_err(|_| "audio source duration overflowed".to_owned())?;
        let frames = audio_frames_ceil(source_duration)?;
        let samples = frames
            .checked_mul(2)
            .ok_or_else(|| "audio sample count overflowed".to_owned())?;
        Ok(usize::try_from(samples)
            .unwrap_or(usize::MAX)
            .saturating_mul(std::mem::size_of::<f32>()))
    }

    /// Estimates the decoded size of every audio clip. Timelines larger than the
    /// cache budget keep the streaming path instead of failing the export.
    fn estimated_export_audio_bytes(program: &PreviewProgram) -> Result<usize, String> {
        let mut bytes = 0_usize;
        for clip in &program.audio_clips {
            bytes = bytes.saturating_add(estimated_audio_clip_bytes(clip)?);
        }
        Ok(bytes)
    }

    /// Decodes every audio clip once, before the frame loop. The old path opened a
    /// decoder for each clip on every video frame; on a timeline with many short
    /// clips that turns a hardware encode into a file-open benchmark. The clips are
    /// independent, so they decode in parallel, and the writer later only mixes
    /// samples that are already in memory.
    fn prepare_export_audio_cache(
        program: &PreviewProgram,
        _budgets: &RuntimeBudgets,
        cancellation: &CancellationToken,
    ) -> Result<Vec<CachedAudioClip>, String> {
        let clips = &program.audio_clips;
        if clips.is_empty() {
            return Ok(Vec::new());
        }
        let key = program.key;
        let requested = std::env::var("OR_EXPORT_AUDIO_WORKERS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|count| count.get())
                    .unwrap_or(1)
            });
        // A knob exists for measurement; production takes every usable core.
        let worker_count = requested
            .clamp(1, MAX_EXPORT_AUDIO_WORKERS)
            .min(clips.len());
        // The cache runs while the viewer and its audio clock may still hold
        // leases on the shared playback budget. Give the export cache its own
        // budget so a playing preview cannot starve export preparation, and size
        // its in-flight allowance from the worker count: each decoder holds a
        // small queue, so a fixed 64 would cap the cache at four workers.
        let in_flight = worker_count
            .saturating_mul(AUDIO_DECODE_QUEUE_CAPACITY.min(8))
            .saturating_mul(2)
            .max(64);
        let cache_budgets = RuntimeBudgets::new(
            BudgetLimits::new(4, 256 * 1024 * 1024).expect("nonzero render budget"),
            BudgetLimits::new(in_flight, 64 * 1024 * 1024)
                .expect("nonzero export audio cache budget"),
            BudgetLimits::new(in_flight, 256 * 1024 * 1024).expect("nonzero decode budget"),
        );
        let next = AtomicUsize::new(0);
        let slots: Mutex<Vec<Option<CachedAudioClip>>> =
            Mutex::new((0..clips.len()).map(|_| None).collect());
        let failure: Mutex<Option<String>> = Mutex::new(None);
        let bytes = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..worker_count {
                scope.spawn(|| {
                    loop {
                        if cancellation.is_cancelled() || lock(&failure).is_some() {
                            return;
                        }
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(clip) = clips.get(index) else {
                            return;
                        };
                        // Reserve the estimated footprint *before* decoding, so N
                        // workers cannot decode N large clips and only then notice
                        // the cache is over budget. The post-decode check stays as
                        // a guard for clips whose real size beats the estimate.
                        let estimate = match estimated_audio_clip_bytes(clip) {
                            Ok(estimate) => estimate,
                            Err(error) => {
                                let mut failure = lock(&failure);
                                if failure.is_none() {
                                    *failure = Some(error);
                                }
                                return;
                            }
                        };
                        let reserved = bytes.fetch_add(estimate, Ordering::Relaxed);
                        if reserved.saturating_add(estimate) > MAX_EXPORT_AUDIO_CACHE_BYTES {
                            let mut failure = lock(&failure);
                            if failure.is_none() {
                                *failure = Some(
                                    "the export audio cache exceeded its memory budget".to_owned(),
                                );
                            }
                            return;
                        }
                        match decode_audio_clip(clip, key, cache_budgets.clone(), cancellation) {
                            Ok(decoded) => {
                                if decoded.bytes() > estimate {
                                    let extra = decoded.bytes() - estimate;
                                    let before = bytes.fetch_add(extra, Ordering::Relaxed);
                                    if before.saturating_add(extra) > MAX_EXPORT_AUDIO_CACHE_BYTES {
                                        let mut failure = lock(&failure);
                                        if failure.is_none() {
                                            *failure = Some(
                                                "the export audio cache exceeded its memory budget"
                                                    .to_owned(),
                                            );
                                        }
                                        return;
                                    }
                                }
                                if let Some(slot) = lock(&slots).get_mut(index) {
                                    *slot = Some(decoded);
                                }
                            }
                            Err(error) => {
                                let mut failure = lock(&failure);
                                if failure.is_none() {
                                    *failure = Some(error);
                                }
                                return;
                            }
                        }
                    }
                });
            }
        });
        if cancellation.is_cancelled() {
            return Err("export was cancelled".to_owned());
        }
        if let Some(error) = lock(&failure).take() {
            return Err(error);
        }
        Ok(lock(&slots)
            .drain(..)
            .map(|slot| slot.unwrap_or_default())
            .collect())
    }

    fn decode_audio_clip(
        clip: &AudioClip,
        key: ProgramKey,
        budgets: RuntimeBudgets,
        cancellation: &CancellationToken,
    ) -> Result<CachedAudioClip, String> {
        if clip.duration.numerator() == 0 {
            return Ok(CachedAudioClip::default());
        }
        let source_duration = clip
            .playback_rate
            .apply(clip.duration)
            .map_err(|_| "audio source duration overflowed".to_owned())?;
        let range = TimeRange::new(clip.source_start, source_duration)
            .map_err(|_| "audio source range is invalid".to_owned())?;
        let snapshot = RenderSnapshot::new(key.project_id, key.revision, range);
        // The cache decodes several clips at once, so each queue holds fewer
        // frames: the runtime's in-flight budget is shared with playback.
        let queue = SnapshotQueue::new(snapshot, AUDIO_DECODE_QUEUE_CAPACITY.min(8))
            .map_err(|error| error.to_string())?;
        let decoder = media_decoder(&clip.source, budgets).map_err(|error| error.to_string())?;
        let decoder_queue = queue.clone();
        let mut cached = CachedAudioClip::default();
        std::thread::scope(|scope| -> Result<(), String> {
            let decode =
                scope.spawn(move || decoder.decode_audio(snapshot, &decoder_queue, cancellation));
            loop {
                if cancellation.is_cancelled() {
                    break;
                }
                if let Some(item) = queue.try_pop_current().map_err(|error| error.to_string())? {
                    let chunk = item.into_value();
                    cached.chunks.push(CachedAudioChunk {
                        timestamp: chunk.timestamp(),
                        samples: chunk.samples().to_vec(),
                    });
                } else if decode.is_finished() {
                    break;
                } else {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            if cancellation.is_cancelled() {
                return Ok(());
            }
            match decode.join() {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(error)) => Err(error.to_string()),
                Err(_) => Err("audio decoder worker panicked".to_owned()),
            }
        })?;
        // The cached path holds the whole clip, so its gate runs once, start to
        // end, and reaches the same steady state the streaming path would.
        if clip.settings.noise_reduction_decibels > 0 {
            let mut gate = NoiseGate::default();
            for chunk in &mut cached.chunks {
                gate.process(&mut chunk.samples, clip.settings.noise_reduction_decibels);
            }
        }
        Ok(cached)
    }

    /// Mixes one block from decoded clips. No decoder, thread or sleep is created
    /// here; the only work left is sample-rate mapping and the clip's audio
    /// settings.
    fn mix_audio_window_cached(
        output: &mut [f32],
        clips: &[AudioClip],
        cache: &[CachedAudioClip],
        block_start: RationalTime,
    ) -> Result<(), String> {
        if output.is_empty() {
            return Ok(());
        }
        let block_end = block_start
            .checked_add(
                audio_time_for_frames(u64::try_from(output.len() / 2).unwrap_or(0))
                    .map_err(|_| "audio block duration exceeds the sample clock".to_owned())?,
            )
            .map_err(|_| "audio block end overflowed".to_owned())?;
        for (clip, cached) in clips.iter().zip(cache.iter()) {
            if cached.chunks.is_empty() {
                continue;
            }
            let clip_end = clip
                .timeline_start
                .checked_add(clip.duration)
                .map_err(|_| "audio clip end overflowed".to_owned())?;
            if block_end <= clip.timeline_start || block_start >= clip_end {
                continue;
            }
            for chunk in &cached.chunks {
                mix_audio_samples(output, clip, block_start, chunk.timestamp, &chunk.samples)?;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn write_export_audio_until(
        writer: &mut or_media::MatroskaFfv1PcmS16leWriter,
        program: &PreviewProgram,
        cache: Option<&[CachedAudioClip]>,
        budgets: &RuntimeBudgets,
        audio_cursor: &mut u64,
        audio_until: u64,
        job: &JobContext,
        cancellation: &CancellationToken,
    ) -> Result<(), String> {
        let mut noise_gates: HashMap<String, NoiseGate> = HashMap::new();
        while *audio_cursor < audio_until {
            if job.is_cancelled() || cancellation.is_cancelled() {
                return Err("export was cancelled".to_owned());
            }
            let frame_count = (audio_until - *audio_cursor).min(AUDIO_BUFFER_FRAMES as u64);
            let sample_count = usize::try_from(frame_count)
                .ok()
                .and_then(|frames| frames.checked_mul(2))
                .ok_or_else(|| "audio block exceeds the memory limit".to_owned())?;
            let mut mixed = vec![0.0_f32; sample_count];
            let block_start = audio_time_for_frames(*audio_cursor)
                .map_err(|_| "audio block time exceeds the sample clock".to_owned())?;
            match cache {
                Some(cache) => {
                    mix_audio_window_cached(&mut mixed, &program.audio_clips, cache, block_start)?
                }
                None => {
                    let error = Mutex::new(None);
                    mix_audio_window(
                        &mut mixed,
                        &program.audio_clips,
                        &mut noise_gates,
                        program.key,
                        block_start,
                        budgets.clone(),
                        cancellation,
                        &error,
                        true,
                    )?;
                    if let Some(message) = error.into_inner().unwrap_or_default() {
                        return Err(message);
                    }
                }
            }
            if job.is_cancelled() || cancellation.is_cancelled() {
                return Err("export was cancelled".to_owned());
            }
            let pcm: Vec<i16> = mixed.into_iter().map(float_to_s16).collect();
            writer
                .write_audio_frames(&pcm, *audio_cursor)
                .map_err(|error| error.to_string())?;
            *audio_cursor += frame_count;
        }
        Ok(())
    }

    fn audio_frames_ceil(time: RationalTime) -> Result<u64, String> {
        if time.is_negative() {
            return Err("audio duration cannot be negative".to_owned());
        }
        let numerator = i128::from(time.numerator()) * i128::from(AUDIO_SAMPLE_RATE);
        let denominator = i128::from(time.denominator());
        let quotient = numerator.div_euclid(denominator);
        let rounded = quotient + i128::from(numerator.rem_euclid(denominator) != 0);
        u64::try_from(rounded).map_err(|_| "audio sample count overflowed".to_owned())
    }

    fn float_to_s16(sample: f32) -> i16 {
        if !sample.is_finite() {
            return 0;
        }
        let sample = sample.clamp(-1.0, 1.0);
        (sample * if sample < 0.0 { 32_768.0 } else { 32_767.0 }).round() as i16
    }

    struct AudioPlayback {
        output: AudioDeviceOutput,
        cancellation: CancellationToken,
        worker: Option<JoinHandle<()>>,
        error: Arc<Mutex<Option<String>>>,
    }

    impl AudioPlayback {
        fn start(
            program: Arc<PreviewProgram>,
            origin: RationalTime,
            budgets: RuntimeBudgets,
        ) -> Result<Self, PreviewError> {
            if !*FFMPEG_LICENSE_OK.get_or_init(or_media::verify_ffmpeg_runtime) {
                return Err(PreviewError::new(
                    "FFMPEG_UNAVAILABLE",
                    "the approved LGPL FFmpeg runtime could not be loaded",
                ));
            }
            let snapshot = RenderSnapshot::new(
                program.key.project_id,
                program.key.revision,
                TimeRange::new(origin, RationalTime::ZERO)
                    .map_err(|error| PreviewError::new("AUDIO_CLOCK_FAILED", error.to_string()))?,
            );
            let (mut producer, output) = AudioDeviceOutput::open(
                snapshot,
                NonZeroUsize::new(AUDIO_BUFFER_FRAMES).expect("audio buffer is nonzero"),
            )
            .map_err(audio_output_error)?;
            let cancellation = CancellationToken::new();
            let worker_cancellation = cancellation.clone();
            let worker_error = Arc::new(Mutex::new(None));
            let thread_error = Arc::clone(&worker_error);
            let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
            let clips = program.audio_clips.clone();
            let key = program.key;
            let content_end = program.content_end;
            let worker = thread::Builder::new()
                .name("or-preview-audio".to_owned())
                .spawn(move || {
                    produce_audio(
                        &mut producer,
                        &clips,
                        key,
                        origin,
                        content_end,
                        budgets,
                        &worker_cancellation,
                        &thread_error,
                        ready_sender,
                    )
                })
                .map_err(|error| PreviewError::new("AUDIO_WORKER_FAILED", error.to_string()))?;
            if ready_receiver.recv_timeout(Duration::from_secs(5)).is_err() {
                cancellation.cancel();
                let _ = worker.join();
                return Err(PreviewError::new(
                    "AUDIO_STARTUP_TIMEOUT",
                    "audio did not prepare its first bounded output block",
                ));
            }
            if let Err(error) = output.start() {
                cancellation.cancel();
                let _ = worker.join();
                return Err(audio_output_error(error));
            }
            Ok(Self {
                output,
                cancellation,
                worker: Some(worker),
                error: worker_error,
            })
        }

        fn clock(&self) -> AudioClockMessage {
            self.output.clock()
        }

        fn failed(&self) -> bool {
            self.output.failed()
        }

        fn error(&self) -> Option<String> {
            lock(&self.error).clone()
        }
    }

    impl Drop for AudioPlayback {
        fn drop(&mut self) {
            self.cancellation.cancel();
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    fn audio_output_error(error: AudioDeviceOutputError) -> PreviewError {
        PreviewError::new("AUDIO_OUTPUT_UNAVAILABLE", error.to_string())
    }

    fn start_audio_playback(
        program: Arc<PreviewProgram>,
        origin: RationalTime,
        budgets: RuntimeBudgets,
    ) -> (Option<AudioPlayback>, Option<String>) {
        match AudioPlayback::start(program, origin, budgets) {
            Ok(audio) => (Some(audio), None),
            Err(error) => (None, Some(error.message)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn produce_audio(
        producer: &mut AudioProducer,
        clips: &[AudioClip],
        key: ProgramKey,
        origin: RationalTime,
        content_end: Option<RationalTime>,
        budgets: RuntimeBudgets,
        cancellation: &CancellationToken,
        error: &Mutex<Option<String>>,
        ready: mpsc::SyncSender<()>,
    ) {
        let Some(content_end) = content_end else {
            let _ = ready.send(());
            return;
        };
        let Ok(remaining) = content_end.checked_sub(origin) else {
            set_audio_error(error, "audio playback range overflowed");
            let _ = ready.send(());
            return;
        };
        let Ok(end_frame) = audio_frames_floor(remaining) else {
            set_audio_error(error, "audio playback range exceeds the sample clock");
            let _ = ready.send(());
            return;
        };
        if end_frame <= 0 {
            let _ = ready.send(());
            return;
        }

        let mut cursor = 0_u64;
        let mut noise_gates: HashMap<String, NoiseGate> = HashMap::new();
        let mut samples = Vec::new();
        let mut ready = Some(ready);
        while cursor < end_frame as u64 && !cancellation.is_cancelled() {
            let frame_count = (end_frame as u64 - cursor).min(AUDIO_BUFFER_FRAMES as u64) as usize;
            samples.clear();
            samples.resize(frame_count.saturating_mul(2), 0.0);
            let block_start = match audio_time_for_frames(cursor) {
                Ok(offset) => origin.checked_add(offset),
                Err(_) => Err(or_core::TimeError::ArithmeticOverflow),
            };
            let block_start = match block_start {
                Ok(value) => value,
                Err(_) => {
                    set_audio_error(error, "audio block time overflowed");
                    break;
                }
            };
            if let Err(message) = mix_audio_window(
                &mut samples,
                clips,
                &mut noise_gates,
                key,
                block_start,
                budgets.clone(),
                cancellation,
                error,
                false,
            ) {
                set_audio_error(error, &message);
            }
            for sample in &mut samples {
                *sample = if sample.is_finite() {
                    sample.clamp(-1.0, 1.0)
                } else {
                    0.0
                };
            }
            loop {
                match producer.try_push(&samples, cancellation) {
                    Ok(()) => {
                        if let Some(ready) = ready.take() {
                            let _ = ready.send(());
                        }
                        break;
                    }
                    Err(or_audio::AudioPushError::Full) => {
                        thread::sleep(Duration::from_millis(2));
                        if cancellation.is_cancelled() {
                            return;
                        }
                    }
                    Err(or_audio::AudioPushError::Cancelled) => return,
                    Err(or_audio::AudioPushError::IncompleteFrame) => {
                        set_audio_error(error, "audio mixer produced an incomplete frame");
                        if let Some(ready) = ready.take() {
                            let _ = ready.send(());
                        }
                        return;
                    }
                }
            }
            cursor += frame_count as u64;
        }
        if let Some(ready) = ready {
            let _ = ready.send(());
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn mix_audio_window(
        output: &mut [f32],
        clips: &[AudioClip],
        gates: &mut HashMap<String, NoiseGate>,
        key: ProgramKey,
        block_start: RationalTime,
        budgets: RuntimeBudgets,
        cancellation: &CancellationToken,
        error: &Mutex<Option<String>>,
        fail_on_decode_error: bool,
    ) -> Result<(), String> {
        let block_duration = audio_time_for_frames((output.len() / 2) as u64)
            .map_err(|_| "audio block duration overflowed".to_owned())?;
        let block_end = block_start
            .checked_add(block_duration)
            .map_err(|_| "audio block end overflowed".to_owned())?;
        for clip in clips {
            if cancellation.is_cancelled() {
                break;
            }
            let clip_end = clip
                .timeline_start
                .checked_add(clip.duration)
                .map_err(|_| "audio clip end overflowed".to_owned())?;
            let overlap_start = block_start.max(clip.timeline_start);
            let overlap_end = block_end.min(clip_end);
            if overlap_start >= overlap_end {
                continue;
            }
            let timeline_offset = overlap_start
                .checked_sub(clip.timeline_start)
                .map_err(|_| "audio source offset overflowed".to_owned())?;
            let source_start = clip
                .source_start
                .checked_add(
                    clip.playback_rate
                        .apply(timeline_offset)
                        .map_err(|_| "audio source offset overflowed".to_owned())?,
                )
                .map_err(|_| "audio source time overflowed".to_owned())?;
            let source_duration = clip
                .playback_rate
                .apply(
                    overlap_end
                        .checked_sub(overlap_start)
                        .map_err(|_| "audio overlap duration overflowed".to_owned())?,
                )
                .map_err(|_| "audio source duration overflowed".to_owned())?;
            let range = TimeRange::new(source_start, source_duration)
                .map_err(|_| "audio source range is invalid".to_owned())?;
            let snapshot = RenderSnapshot::new(key.project_id, key.revision, range);
            let queue = SnapshotQueue::new(snapshot, AUDIO_DECODE_QUEUE_CAPACITY)
                .map_err(|error| error.to_string())?;
            let decoder = match media_decoder(&clip.source, budgets.clone()) {
                Ok(decoder) => decoder,
                Err(decode_error) => {
                    set_audio_error(error, &decode_error.to_string());
                    continue;
                }
            };
            let decoder_queue = queue.clone();
            let decode_result = thread::scope(|scope| {
                let decode = scope
                    .spawn(move || decoder.decode_audio(snapshot, &decoder_queue, cancellation));
                let mut mix_error = None;
                loop {
                    if let Some(item) =
                        queue.try_pop_current().map_err(|error| error.to_string())?
                    {
                        if !cancellation.is_cancelled()
                            && let Err(message) =
                                mix_audio_chunk(output, clip, gates, block_start, item.into_value())
                        {
                            mix_error.get_or_insert(message);
                        }
                    } else if decode.is_finished() {
                        break;
                    } else {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
                let decoded = decode
                    .join()
                    .map_err(|_| "audio decoder worker panicked".to_owned())?;
                if let Err(decode_error) = decoded
                    && !cancellation.is_cancelled()
                {
                    mix_error.get_or_insert_with(|| decode_error.to_string());
                }
                if let Some(message) = mix_error {
                    set_audio_error(error, &message);
                }
                Ok::<(), String>(())
            });
            if let Err(message) = decode_result {
                if cancellation.is_cancelled() {
                    break;
                }
                set_audio_error(error, &message);
            }
        }
        if fail_on_decode_error && let Some(message) = lock(error).clone() {
            return Err(message);
        }
        Ok(())
    }

    /// Applies one clip's noise gate to a decoded block, keeping gate state per
    /// clip so a hiss tail is not re-opened at every block boundary.
    fn gate_clip_audio(
        clip: &AudioClip,
        gates: &mut HashMap<String, NoiseGate>,
        samples: &mut [f32],
    ) {
        let gate = gates.entry(clip.id.clone()).or_default();
        gate.process(samples, clip.settings.noise_reduction_decibels);
    }

    fn mix_audio_chunk(
        output: &mut [f32],
        clip: &AudioClip,
        gates: &mut HashMap<String, NoiseGate>,
        block_start: RationalTime,
        mut chunk: or_media::AudioChunk,
    ) -> Result<(), String> {
        gate_clip_audio(clip, gates, chunk.samples_mut());
        mix_audio_samples(
            output,
            clip,
            block_start,
            chunk.timestamp(),
            chunk.samples(),
        )
    }

    fn mix_audio_samples(
        output: &mut [f32],
        clip: &AudioClip,
        block_start: RationalTime,
        source_timestamp: RationalTime,
        source_samples: &[f32],
    ) -> Result<(), String> {
        if !output.len().is_multiple_of(2) || !source_samples.len().is_multiple_of(2) {
            return Err("audio mixer received an incomplete stereo frame".to_owned());
        }
        let output_frames = output.len() / 2;
        let source_frames = source_samples.len() / 2;
        if output_frames == 0 || source_frames == 0 {
            return Ok(());
        }

        let inverse_rate = RationalRate::new(
            clip.playback_rate.denominator(),
            clip.playback_rate.numerator(),
        )
        .map_err(|_| "audio playback rate could not be inverted".to_owned())?;
        let source_offset = source_timestamp
            .checked_sub(clip.source_start)
            .map_err(|_| "audio source offset overflowed".to_owned())?;
        let chunk_timeline_start = clip
            .timeline_start
            .checked_add(
                inverse_rate
                    .apply(source_offset)
                    .map_err(|_| "audio chunk timeline offset overflowed".to_owned())?,
            )
            .map_err(|_| "audio chunk timeline time overflowed".to_owned())?;
        let source_chunk_duration = audio_time_for_frames(
            u64::try_from(source_frames)
                .map_err(|_| "audio chunk exceeds the sample clock".to_owned())?,
        )
        .map_err(|_| "audio chunk duration exceeds the sample clock".to_owned())?;
        let chunk_timeline_end = clip
            .timeline_start
            .checked_add(
                inverse_rate
                    .apply(
                        source_offset
                            .checked_add(source_chunk_duration)
                            .map_err(|_| "audio chunk source end overflowed".to_owned())?,
                    )
                    .map_err(|_| "audio chunk timeline duration overflowed".to_owned())?,
            )
            .map_err(|_| "audio chunk timeline end overflowed".to_owned())?;
        let block_end = block_start
            .checked_add(
                audio_time_for_frames(
                    u64::try_from(output_frames)
                        .map_err(|_| "audio block exceeds the sample clock".to_owned())?,
                )
                .map_err(|_| "audio block duration exceeds the sample clock".to_owned())?,
            )
            .map_err(|_| "audio block end overflowed".to_owned())?;
        let clip_end = clip
            .timeline_start
            .checked_add(clip.duration)
            .map_err(|_| "audio clip end overflowed".to_owned())?;
        let overlap_start = chunk_timeline_start
            .max(block_start)
            .max(clip.timeline_start);
        let overlap_end = chunk_timeline_end.min(block_end).min(clip_end);
        if overlap_start >= overlap_end {
            return Ok(());
        }

        let first_frame = usize::try_from(
            audio_frame_difference_ceil(overlap_start, block_start)
                .map_err(|_| "audio chunk offset exceeds the device clock".to_owned())?
                .max(0),
        )
        .map_err(|_| "audio chunk offset exceeds the device clock".to_owned())?;
        let end_frame = usize::try_from(
            audio_frame_difference_ceil(overlap_end, block_start)
                .map_err(|_| "audio chunk end exceeds the device clock".to_owned())?
                .max(0),
        )
        .map_err(|_| "audio chunk end exceeds the device clock".to_owned())?
        .min(output_frames);
        if first_frame >= end_frame {
            return Ok(());
        }

        let first_time = block_start
            .checked_add(
                audio_time_for_frames(
                    u64::try_from(first_frame)
                        .map_err(|_| "audio chunk offset exceeds the sample clock".to_owned())?,
                )
                .map_err(|_| "audio chunk offset exceeds the sample clock".to_owned())?,
            )
            .map_err(|_| "audio output time overflowed".to_owned())?;
        let clip_offset = first_time
            .checked_sub(clip.timeline_start)
            .map_err(|_| "audio clip offset overflowed".to_owned())?;
        let first_source_time = clip
            .source_start
            .checked_add(
                clip.playback_rate
                    .apply(clip_offset)
                    .map_err(|_| "audio source time overflowed".to_owned())?,
            )
            .map_err(|_| "audio source time overflowed".to_owned())?;
        let source_delta = first_source_time
            .checked_sub(source_timestamp)
            .map_err(|_| "audio sample offset overflowed".to_owned())?;
        let position_denominator =
            i128::from(source_delta.denominator()) * i128::from(clip.playback_rate.denominator());
        let mut position_numerator = i128::from(source_delta.numerator())
            * i128::from(AUDIO_SAMPLE_RATE)
            * i128::from(clip.playback_rate.denominator());
        let position_step =
            i128::from(clip.playback_rate.numerator()) * i128::from(source_delta.denominator());
        let mut resampled = vec![0.0_f32; (end_frame - first_frame) * 2];
        for frame in 0..resampled.len() / 2 {
            let source_frame = usize::try_from(position_numerator.div_euclid(position_denominator))
                .map_err(|_| "audio sample position is invalid".to_owned())?;
            if source_frame >= source_frames {
                break;
            }
            let next_source_frame = (source_frame + 1).min(source_frames - 1);
            let fraction = position_numerator.rem_euclid(position_denominator) as f64
                / position_denominator as f64;
            let source_index = source_frame * 2;
            let next_source_index = next_source_frame * 2;
            for channel in 0..2 {
                let first = f64::from(source_samples[source_index + channel]);
                let second = f64::from(source_samples[next_source_index + channel]);
                resampled[frame * 2 + channel] = (first + (second - first) * fraction) as f32;
            }
            position_numerator += position_step;
        }
        process_audio_clip(&mut resampled, clip_offset, clip.duration, clip.settings)
            .map_err(|error| error.to_string())?;
        for (frame, stereo) in resampled.as_chunks::<2>().0.iter().enumerate() {
            let destination = (first_frame + frame) * 2;
            output[destination] += stereo[0];
            output[destination + 1] += stereo[1];
        }
        Ok(())
    }

    fn audio_time_for_frames(frames: u64) -> Result<RationalTime, or_core::TimeError> {
        let frames = i64::try_from(frames).map_err(|_| or_core::TimeError::ArithmeticOverflow)?;
        RationalTime::new(frames, AUDIO_SAMPLE_RATE)
    }

    fn audio_frames_floor(time: RationalTime) -> Result<i64, ()> {
        let frames = (i128::from(time.numerator()) * i128::from(AUDIO_SAMPLE_RATE))
            .div_euclid(i128::from(time.denominator()));
        i64::try_from(frames).map_err(|_| ())
    }

    fn audio_frame_difference_ceil(time: RationalTime, origin: RationalTime) -> Result<i64, ()> {
        let numerator = (i128::from(time.numerator()) * i128::from(origin.denominator())
            - i128::from(origin.numerator()) * i128::from(time.denominator()))
            * i128::from(AUDIO_SAMPLE_RATE);
        let denominator = i128::from(time.denominator()) * i128::from(origin.denominator());
        let quotient = numerator.div_euclid(denominator);
        let rounded = quotient + i128::from(numerator.rem_euclid(denominator) != 0);
        i64::try_from(rounded).map_err(|_| ())
    }

    fn set_audio_error(error: &Mutex<Option<String>>, message: &str) {
        let mut current = lock(error);
        if current.is_none() {
            *current = Some(message.to_owned());
        }
    }

    struct ThreadWaker(thread::Thread);

    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut future = Box::pin(future);
        let current = thread::current();
        let waker: Waker = Waker::from(Arc::new(ThreadWaker(current.clone())));
        let mut context = Context::from_waker(&waker);
        loop {
            match Pin::as_mut(&mut future).poll(&mut context) {
                Poll::Ready(value) => return value,
                Poll::Pending => thread::park_timeout(Duration::from_millis(10)),
            }
        }
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[cfg(test)]
    mod android_preparation_tests {
        use super::*;
        use or_core::{ProjectFileSession, decode_project, save_project_file_atomic};
        use std::{
            fs,
            path::PathBuf,
            time::{SystemTime, UNIX_EPOCH},
        };
        static TEST_LOCK: Mutex<()> = Mutex::new(());

        #[test]
        fn audio_chunk_bounds_use_sample_clock_when_exact_rational_difference_overflows() {
            let chunk_timeline = RationalTime::new(741, 500_000_000).unwrap();
            let block_start = RationalTime::new(-221, 8_820).unwrap();

            assert!(chunk_timeline.checked_sub(block_start).is_err());
            assert_eq!(
                audio_frame_difference_ceil(chunk_timeline, block_start),
                Ok(1_203)
            );
        }

        #[test]
        fn audio_mixer_resamples_at_the_clip_playback_rate() {
            let source =
                MediaSourceRef::local_file("file:///tmp/or-preview-audio-speed.wav").unwrap();
            let fast_clip = AudioClip {
                id: "fast".to_owned(),
                source: source.clone(),
                timeline_start: RationalTime::ZERO,
                source_start: RationalTime::ZERO,
                duration: RationalTime::new(4, AUDIO_SAMPLE_RATE).unwrap(),
                playback_rate: RationalRate::new(2, 1).unwrap(),
                settings: AudioSettings::DEFAULT,
            };
            let source_samples = (0..8)
                .flat_map(|sample| [sample as f32, sample as f32])
                .collect::<Vec<_>>();
            let mut fast_output = vec![0.0; 8];
            mix_audio_samples(
                &mut fast_output,
                &fast_clip,
                RationalTime::ZERO,
                RationalTime::ZERO,
                &source_samples,
            )
            .unwrap();
            assert_eq!(fast_output, [0.0, 0.0, 2.0, 2.0, 4.0, 4.0, 6.0, 6.0]);

            let slow_clip = AudioClip {
                id: "slow".to_owned(),
                source,
                timeline_start: RationalTime::ZERO,
                source_start: RationalTime::ZERO,
                duration: RationalTime::new(8, AUDIO_SAMPLE_RATE).unwrap(),
                playback_rate: RationalRate::new(1, 2).unwrap(),
                settings: AudioSettings::DEFAULT,
            };
            let slow_source_samples = (0..4)
                .flat_map(|sample| [sample as f32, sample as f32])
                .collect::<Vec<_>>();
            let mut slow_output = vec![0.0; 16];
            mix_audio_samples(
                &mut slow_output,
                &slow_clip,
                RationalTime::ZERO,
                RationalTime::ZERO,
                &slow_source_samples,
            )
            .unwrap();
            assert_eq!(
                slow_output,
                [
                    0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.5, 1.5, 2.0, 2.0, 2.5, 2.5, 3.0, 3.0, 3.0, 3.0,
                ]
            );
        }

        #[test]
        fn a_clip_gate_quiets_hiss_after_a_burst_and_keeps_state_across_blocks() {
            // The gate must not be re-created per block: a hiss tail would then be
            // measured against a cold peak and stay open.
            let source =
                MediaSourceRef::local_file("file:///tmp/or-preview-audio-noise.wav").unwrap();
            let clip = AudioClip {
                id: "noisy".to_owned(),
                source,
                timeline_start: RationalTime::ZERO,
                source_start: RationalTime::ZERO,
                duration: RationalTime::new(2, 1).unwrap(),
                playback_rate: RationalRate::new(1, 1).unwrap(),
                settings: AudioSettings {
                    noise_reduction_decibels: 18,
                    ..AudioSettings::DEFAULT
                },
            };
            let frames = AUDIO_SAMPLE_RATE as usize;
            let mut burst = vec![0.0_f32; frames * 2];
            for frame in 0..frames / 4 {
                let value = 0.5 * (frame as f32 * 0.2).sin();
                burst[frame * 2] = value;
                burst[frame * 2 + 1] = value;
            }
            let mut block = burst.clone();
            let mut gates = HashMap::new();
            gate_clip_audio(&clip, &mut gates, &mut block);
            let mut hiss = vec![0.0_f32; frames * 2];
            for (index, sample) in hiss.iter_mut().enumerate() {
                *sample = if index % 7 == 0 { 0.004 } else { -0.004 };
            }
            gate_clip_audio(&clip, &mut gates, &mut hiss);
            let quiet = hiss.iter().fold(0.0_f32, |sum, value| sum + value * value);
            assert!(
                quiet < burst.len() as f32 * 0.000_01,
                "hiss stayed loud: energy {quiet}"
            );
            // A steady tone in the same clip keeps its level.
            let mut tone = vec![0.0_f32; frames * 2];
            for frame in 0..frames {
                let value = 0.2 * (frame as f32 * 0.05).sin();
                tone[frame * 2] = value;
                tone[frame * 2 + 1] = value;
            }
            let before = tone.iter().fold(0.0_f32, |sum, value| sum + value * value);
            gate_clip_audio(&clip, &mut gates, &mut tone);
            let after = tone.iter().fold(0.0_f32, |sum, value| sum + value * value);
            assert!(
                after > before * 0.25,
                "the tone was gated: {after} vs {before}"
            );
        }

        #[test]
        fn cached_export_mixer_matches_the_direct_mixer() {
            let source =
                MediaSourceRef::local_file("file:///tmp/or-preview-audio-cache.wav").unwrap();
            let clip = AudioClip {
                id: "cached".to_owned(),
                source,
                timeline_start: RationalTime::ZERO,
                source_start: RationalTime::ZERO,
                duration: RationalTime::new(8, AUDIO_SAMPLE_RATE).unwrap(),
                playback_rate: RationalRate::new(1, 1).unwrap(),
                settings: AudioSettings::DEFAULT,
            };
            let source_samples = (0..8)
                .flat_map(|sample| [sample as f32, sample as f32])
                .collect::<Vec<_>>();
            let mut direct = vec![0.0; 8];
            mix_audio_samples(
                &mut direct,
                &clip,
                RationalTime::ZERO,
                RationalTime::ZERO,
                &source_samples,
            )
            .unwrap();
            let cache = vec![CachedAudioClip {
                chunks: vec![CachedAudioChunk {
                    timestamp: RationalTime::ZERO,
                    samples: source_samples,
                }],
            }];
            let mut cached = vec![0.0; 8];
            mix_audio_window_cached(&mut cached, &[clip], &cache, RationalTime::ZERO).unwrap();
            assert_eq!(cached, direct);
        }

        fn late_source_project() -> or_core::ProjectDocument {
            let video_media = (1..=65)
                .map(|index| format!(r#"{{"id":"{index:08x}-2222-4222-8222-222222222222","source":{{"kind":"android_saf_document_uri","uri":"content://dev.opencut.fixture/document/clip-{index:02}"}},"metadata":{{"format_names":["matroska"],"duration":{{"numerator":1,"denominator":1}},"file_size_bytes":1024,"streams":[{{"kind":"video","metadata":{{"index":0,"codec_name":"ffv1","width":16,"height":16,"pixel_format":"bgra","average_frame_rate":{{"numerator":4,"denominator":1}},"duration":{{"numerator":1,"denominator":1}}}}}}]}}}}"#))
                .collect::<Vec<_>>()
                .join(",");
            let media = format!(
                r#"{video_media},{{"id":"00000042-2222-4222-8222-222222222222","source":{{"kind":"android_saf_document_uri","uri":"content://dev.opencut.fixture/document/media-audio"}},"metadata":{{"format_names":["wav"],"duration":{{"numerator":1,"denominator":1}},"file_size_bytes":2048,"streams":[{{"kind":"audio","metadata":{{"index":0,"codec_name":"pcm_s16le","sample_rate":48000,"channels":2,"channel_layout":"stereo","duration":{{"numerator":1,"denominator":1}}}}}}]}}}}"#
            );
            let project = decode_project(&format!(r#"{{"format":"opencut-reinforced-project","schema_version":7,"project":{{"id":"01234567-89ab-4def-8123-456789abcdef","revision":0,"name":"Late SAF source","media":[{media}],"timeline":{{"tracks":[],"markers":[],"sequence_frame_rate":{{"numerator":4,"denominator":1}}}}}}}}"#)).unwrap();
            let mut session = ProjectSession::open(project);
            let track = or_core::TrackId::generate();
            session
                .execute_command(or_core::CommandEnvelope::add_timeline_track(
                    session.project_id(),
                    session.project_instance_id(),
                    session.project_revision(),
                    track,
                    TrackKind::Video,
                ))
                .unwrap();
            session
                .execute_command(or_core::CommandEnvelope::insert_timeline_clip(
                    session.project_id(),
                    session.project_instance_id(),
                    session.project_revision(),
                    or_core::ClipId::generate(),
                    track,
                    session.project().media_items()[64].id(),
                    RationalTime::ZERO,
                    TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap()).unwrap(),
                ))
                .unwrap();
            let audio_track = or_core::TrackId::generate();
            session
                .execute_command(or_core::CommandEnvelope::add_timeline_track(
                    session.project_id(),
                    session.project_instance_id(),
                    session.project_revision(),
                    audio_track,
                    TrackKind::Audio,
                ))
                .unwrap();
            session
                .execute_command(or_core::CommandEnvelope::insert_timeline_clip(
                    session.project_id(),
                    session.project_instance_id(),
                    session.project_revision(),
                    or_core::ClipId::generate(),
                    audio_track,
                    session.project().media_items()[65].id(),
                    RationalTime::ZERO,
                    TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 2).unwrap()).unwrap(),
                ))
                .unwrap();
            session.project().clone()
        }

        #[test]
        fn evaluated_late_library_source_is_active_and_half_open() {
            let project = late_source_project();
            assert_eq!(project.media_items().len(), 66);
            let session = ProjectSession::open(project);
            let mut program = load_program_from_session(&session).unwrap();
            assert_eq!(
                program.active_saf_sources(RationalTime::ZERO).unwrap(),
                ["content://dev.opencut.fixture/document/clip-65"]
            );
            assert_eq!(
                program.playback_saf_sources(RationalTime::ZERO).unwrap(),
                [
                    "content://dev.opencut.fixture/document/clip-65",
                    "content://dev.opencut.fixture/document/media-audio"
                ]
            );
            assert_eq!(
                program
                    .playback_saf_sources(RationalTime::new(1, 2).unwrap())
                    .unwrap(),
                ["content://dev.opencut.fixture/document/clip-65"]
            );
            assert!(
                program
                    .active_saf_sources(RationalTime::new(1, 1).unwrap())
                    .unwrap()
                    .is_empty()
            );
            let mut second = load_program_from_session(&session)
                .unwrap()
                .video_clips
                .remove(0);
            second.source = MediaSourceRef::android_saf_document_uri(
                "content://dev.opencut.fixture/document/aaa",
            )
            .unwrap();
            program.video_clips.push(second);
            program
                .video_clips
                .extend(load_program_from_session(&session).unwrap().video_clips);
            assert_eq!(
                program.active_saf_sources(RationalTime::ZERO).unwrap(),
                [
                    "content://dev.opencut.fixture/document/aaa",
                    "content://dev.opencut.fixture/document/clip-65"
                ]
            );
            assert_eq!(session.project_revision().value(), 4);
        }

        #[test]
        fn source_budget_counts_simultaneously_active_sources() {
            let session = ProjectSession::open(late_source_project());
            let mut program = load_program_from_session(&session).unwrap();
            for index in 0..64 {
                let mut clip = load_program_from_session(&session)
                    .unwrap()
                    .video_clips
                    .remove(0);
                clip.source = MediaSourceRef::android_saf_document_uri(format!(
                    "content://dev.opencut.fixture/document/extra-{index}"
                ))
                .unwrap();
                program.video_clips.push(clip);
            }
            assert_eq!(
                program
                    .active_saf_sources(RationalTime::ZERO)
                    .unwrap_err()
                    .code,
                "MEDIA_SOURCE_BUDGET_EXCEEDED"
            );
            assert_eq!(
                program
                    .playback_saf_sources(RationalTime::ZERO)
                    .unwrap_err()
                    .code,
                "MEDIA_SOURCE_BUDGET_EXCEEDED"
            );
            assert!(
                program
                    .active_saf_sources(RationalTime::new(1, 1).unwrap())
                    .unwrap()
                    .is_empty()
            );
        }

        struct TempProject(PathBuf);
        impl Drop for TempProject {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        fn test_host() -> (TempProject, Arc<PreviewRuntime>, LiveProjectHost) {
            let directory = TempProject(std::env::temp_dir().join(format!(
                    "or-preview-preparation-{}-{}",
                    std::process::id(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                )));
            fs::create_dir(&directory.0).unwrap();
            let path = directory.0.join("late.orproj");
            save_project_file_atomic(&path, &late_source_project()).unwrap();
            let runtime = Arc::new(PreviewRuntime::new());
            let handler: Arc<dyn ExportRequestHandler> = runtime.clone();
            let host = LiveProjectHost::in_process_with_export_handler(
                ProjectFileSession::open(path).unwrap(),
                handler,
            );
            (directory, runtime, host)
        }

        #[test]
        fn ticks_do_not_cancel_slow_binding_or_completion() {
            let _test = lock(&TEST_LOCK);
            let (_directory, runtime, mut host) = test_host();
            let first = runtime
                .prepare(&host, PreviewPreparationAction::Play)
                .unwrap()
                .request_id
                .unwrap();
            let cancellation = lock(&runtime.cancellation).clone();
            for _ in 0..100 {
                assert!(
                    runtime
                        .prepare(&host, PreviewPreparationAction::Tick)
                        .unwrap()
                        .request_id
                        .is_none()
                );
                assert_eq!(lock(&runtime.pending).as_ref().unwrap().generation, first);
                assert!(!cancellation.is_cancelled());
            }
            let frame = lock(&runtime.pending).take().unwrap();
            runtime.completing_prepared.store(true, Ordering::SeqCst);
            for _ in 0..100 {
                assert!(
                    runtime
                        .prepare(&host, PreviewPreparationAction::Tick)
                        .unwrap()
                        .request_id
                        .is_none()
                );
                assert_eq!(runtime.generation.load(Ordering::SeqCst), first);
                assert!(!cancellation.is_cancelled());
            }
            drop(frame);
            runtime.completing_prepared.store(false, Ordering::SeqCst);
            runtime.shutdown().unwrap();
            host.shutdown(false).unwrap();
        }

        #[test]
        fn current_seek_waits_for_old_completion_and_rejects_edit_during_binding() {
            let _test = lock(&TEST_LOCK);
            let (_directory, runtime, mut host) = test_host();
            runtime
                .prepare(&host, PreviewPreparationAction::Play)
                .unwrap();
            let draining = lock(&runtime.completion_lock);
            runtime.completing_prepared.store(true, Ordering::SeqCst);
            let seek = runtime
                .prepare(
                    &host,
                    PreviewPreparationAction::Seek(RationalTime::new(1, 4).unwrap()),
                )
                .unwrap()
                .request_id
                .unwrap();
            let summary = host.describe().unwrap().summary;
            let changed = host
                .handle_application_request(ApplicationRequest::Command(
                    or_core::CommandEnvelope::set_timeline_sequence_frame_rate(
                        summary.project_id,
                        summary.project_instance_id,
                        summary.project_revision,
                        Some(RationalRate::new(24, 1).unwrap()),
                    ),
                ))
                .unwrap();
            assert!(matches!(changed, ApplicationResponse::Command(_)));
            thread::scope(|scope| {
                let (started_send, started_receive) = mpsc::channel();
                let (done_send, done_receive) = mpsc::channel();
                let runtime = &runtime;
                let host = &host;
                scope.spawn(move || {
                    started_send.send(()).unwrap();
                    done_send
                        .send(runtime.complete_prepared(host, seek))
                        .unwrap();
                });
                started_receive.recv().unwrap();
                assert!(
                    matches!(
                        done_receive.recv_timeout(Duration::from_millis(100)),
                        Err(mpsc::RecvTimeoutError::Timeout)
                    ),
                    "a current seek waits instead of being rejected by the old completion flag"
                );
                assert_eq!(lock(&runtime.pending).as_ref().unwrap().generation, seek);
                drop(draining);
                assert_eq!(
                    done_receive.recv().unwrap().unwrap_err().code,
                    "STALE_PREVIEW_REQUEST",
                    "canonical edit prevents old-program rendering"
                );
            });
            assert!(lock(&runtime.pending).is_none());
            assert_eq!(runtime.snapshot().playback.frame_sequence, 0);
            runtime.shutdown().unwrap();
            host.shutdown(true).unwrap();
        }

        #[test]
        fn play_prepares_without_seek_and_replaced_or_closed_requests_cannot_render() {
            let _test = lock(&TEST_LOCK);
            let directory = TempProject(std::env::temp_dir().join(format!(
                    "or-preview-preparation-{}-{}",
                    std::process::id(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                )));
            fs::create_dir(&directory.0).unwrap();
            let path = directory.0.join("late.orproj");
            save_project_file_atomic(&path, &late_source_project()).unwrap();
            let runtime = Arc::new(PreviewRuntime::new());
            let handler: Arc<dyn ExportRequestHandler> = runtime.clone();
            let mut host = LiveProjectHost::in_process_with_export_handler(
                ProjectFileSession::open(path).unwrap(),
                handler,
            );
            let play = runtime
                .prepare(&host, PreviewPreparationAction::Play)
                .unwrap();
            assert!(play.snapshot.playback.playing);
            assert_eq!(
                play.sources,
                [
                    "content://dev.opencut.fixture/document/clip-65",
                    "content://dev.opencut.fixture/document/media-audio"
                ]
            );
            let old = play
                .request_id
                .expect("play has a first frame without prior seek");
            assert!(
                lock(&runtime.pending)
                    .as_ref()
                    .unwrap()
                    .start_audio_after_registration
            );
            assert!(lock(&runtime.state).audio_playback.is_none());
            let seek = runtime
                .prepare(
                    &host,
                    PreviewPreparationAction::Seek(RationalTime::new(1, 4).unwrap()),
                )
                .unwrap();
            assert_eq!(
                seek.snapshot.playback.position,
                RationalTime::new(1, 4).unwrap()
            );
            assert_eq!(
                runtime.complete_prepared(&host, old).unwrap_err().code,
                "STALE_PREVIEW_REQUEST"
            );
            assert_eq!(
                lock(&runtime.pending).as_ref().unwrap().generation,
                seek.request_id.unwrap()
            );
            runtime.abort_prepared(seek.request_id.unwrap()).unwrap();
            assert!(lock(&runtime.pending).is_none());
            assert!(!runtime.snapshot().playback.playing);
            let recovered = runtime
                .prepare(&host, PreviewPreparationAction::Seek(RationalTime::ZERO))
                .unwrap();
            assert!(
                recovered.request_id.is_some(),
                "failed registration is retryable"
            );
            runtime.shutdown().unwrap();
            assert!(lock(&runtime.pending).is_none());
            assert_eq!(
                runtime
                    .complete_prepared(&host, recovered.request_id.unwrap())
                    .unwrap_err()
                    .code,
                "STALE_PREVIEW_REQUEST"
            );
            assert_eq!(
                runtime
                    .prepare(&host, PreviewPreparationAction::Play)
                    .err()
                    .unwrap()
                    .code,
                "STALE_PREVIEW_REQUEST"
            );
            assert_eq!(host.describe().unwrap().summary.project_revision.value(), 4);
            host.shutdown(false).unwrap();
        }
    }
    #[cfg(test)]
    mod export_pipeline_tests {
        use super::*;

        /// Exports a blank 1080p30 three-second timeline through the render-ahead
        /// loop, probes nothing itself, and prints the throughput so the lane records
        /// the pipeline cost of one real export. It leaves the file behind so an
        /// independent probe can read the container, codec, frame count and duration.
        /// Exports a blank timeline to the hardware H.264 profile, which is the
        /// path that takes NV12 planes from the GPU, and leaves the file behind for
        /// an independent probe. The picture should decode as limited-range black:
        /// luma 16, chroma 128.
        #[test]
        #[ignore = "encodes with the codec-enabled FFmpeg profile; run with the lane's helper"]
        fn a_hardware_export_writes_from_gpu_nv12_planes() {
            let directory =
                std::env::temp_dir().join(format!("or-export-nv12-{}", std::process::id()));
            std::fs::create_dir_all(&directory).expect("scratch directory");
            let destination = directory.join("hardware.mp4");
            let _ = std::fs::remove_file(&destination);

            let project = or_core::ProjectDocument::new("nv12 export");
            let rate = RationalRate::new(30, 1).expect("rate");
            let program = Arc::new(PreviewProgram {
                canvas: None,
                key: ProgramKey {
                    project_id: project.id(),
                    project_instance_id: ProjectInstanceId::generate(),
                    revision: project.revision(),
                },
                frame_rate: Some(rate),
                content_end: Some(RationalTime::new(4, 1).expect("duration")),
                video_clips: Vec::new(),
                text_clips: Vec::new(),
                audio_clips: Vec::new(),
            });
            let render_resources = Arc::new(RenderResources {
                budgets: RuntimeBudgets::new(
                    BudgetLimits::new(4, 256 * 1024 * 1024).expect("render budget"),
                    BudgetLimits::new(AUDIO_DECODE_QUEUE_CAPACITY, 32 * 1024 * 1024)
                        .expect("audio budget"),
                    BudgetLimits::new(16, 256 * 1024 * 1024).expect("decode budget"),
                ),
                text_rasterizer: Mutex::new(TextRasterizer::new()),
                video_sessions: Mutex::new(VecDeque::new()),
            });

            let jobs = JobManager::new(JobManagerConfig::new(1, 1, 8).expect("job manager"));
            let cancellation = CancellationToken::new();
            let outcome: Arc<Mutex<Option<Result<(), String>>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&outcome);
            let target = destination.clone();
            let started = Instant::now();
            jobs.submit(JobKind::Export, move |job| {
                let result = export_program(
                    program,
                    target,
                    ExportProfile::Mp4H264Aac,
                    ExportQuality::Quality,
                    or_core::ExportPerformance::default(),
                    (1920, 1080),
                    120,
                    rate,
                    render_resources,
                    job,
                    &cancellation,
                );
                *lock(&slot) = Some(result);
                Ok(())
            })
            .expect("export job submits");

            let deadline = Instant::now() + Duration::from_secs(120);
            while lock(&outcome).is_none() {
                assert!(Instant::now() < deadline, "the export did not finish");
                thread::sleep(Duration::from_millis(10));
            }
            lock(&outcome)
                .take()
                .expect("the export ran")
                .expect("the export succeeded");
            jobs.shutdown();

            let bytes = std::fs::metadata(&destination)
                .expect("output exists")
                .len();
            assert!(bytes > 0, "the export produced no bytes");
            println!(
                "OR_EXPORT_NV12 path={} bytes={bytes} frames=120 size=1920x1080 seconds={:.3}",
                destination.display(),
                started.elapsed().as_secs_f64()
            );
        }

        /// Exports the same blank timeline at both delivery qualities and prints the
        /// sizes, so the preference is proven by two files rather than by the option
        /// mapping alone. A blank frame is the hardest case for a size difference; the
        /// assertion is that both exports succeed and that the quality pass is not
        /// smaller, which is what a rate-control setting that never reached the encoder
        /// would produce.
        #[test]
        #[ignore = "encodes with the codec-enabled FFmpeg profile; run with the lane's helper"]
        fn a_delivery_export_honours_the_quality_preference() {
            let mut sizes = Vec::new();
            for quality in [ExportQuality::Speed, ExportQuality::Quality] {
                let directory = std::env::temp_dir().join(format!(
                    "or-export-quality-{}-{quality:?}",
                    std::process::id()
                ));
                std::fs::create_dir_all(&directory).expect("scratch directory");
                let destination = directory.join("quality.webm");
                let _ = std::fs::remove_file(&destination);

                let project = or_core::ProjectDocument::new("export quality");
                let rate = RationalRate::new(30, 1).expect("rate");
                let program = Arc::new(PreviewProgram {
                    canvas: None,
                    key: ProgramKey {
                        project_id: project.id(),
                        project_instance_id: ProjectInstanceId::generate(),
                        revision: project.revision(),
                    },
                    frame_rate: Some(rate),
                    content_end: Some(RationalTime::new(2, 1).expect("duration")),
                    video_clips: Vec::new(),
                    text_clips: Vec::new(),
                    audio_clips: Vec::new(),
                });
                let render_resources = Arc::new(RenderResources {
                    budgets: RuntimeBudgets::new(
                        BudgetLimits::new(4, 256 * 1024 * 1024).expect("render budget"),
                        BudgetLimits::new(AUDIO_DECODE_QUEUE_CAPACITY, 32 * 1024 * 1024)
                            .expect("audio budget"),
                        BudgetLimits::new(16, 256 * 1024 * 1024).expect("decode budget"),
                    ),
                    text_rasterizer: Mutex::new(TextRasterizer::new()),
                    video_sessions: Mutex::new(VecDeque::new()),
                });

                let jobs = JobManager::new(JobManagerConfig::new(1, 1, 8).expect("job manager"));
                let cancellation = CancellationToken::new();
                let outcome: Arc<Mutex<Option<Result<(), String>>>> = Arc::new(Mutex::new(None));
                let slot = Arc::clone(&outcome);
                let target = destination.clone();
                jobs.submit(JobKind::Export, move |job| {
                    let result = export_program(
                        program,
                        target,
                        ExportProfile::WebmVp9Opus,
                        quality,
                        or_core::ExportPerformance::default(),
                        (640, 360),
                        60,
                        rate,
                        render_resources,
                        job,
                        &cancellation,
                    );
                    *lock(&slot) = Some(result);
                    Ok(())
                })
                .expect("export job submits");

                let deadline = Instant::now() + Duration::from_secs(120);
                while lock(&outcome).is_none() {
                    assert!(Instant::now() < deadline, "the export did not finish");
                    thread::sleep(Duration::from_millis(10));
                }
                lock(&outcome)
                    .take()
                    .expect("the export ran")
                    .expect("the export succeeded");
                jobs.shutdown();

                let bytes = std::fs::metadata(&destination)
                    .expect("output exists")
                    .len();
                println!(
                    "OR_EXPORT_QUALITY quality={quality:?} bytes={bytes} path={}",
                    destination.display()
                );
                sizes.push((quality, bytes));
            }
            assert!(
                sizes[1].1 >= sizes[0].1,
                "the quality export produced fewer bytes than the speed export"
            );
        }

        #[test]
        #[ignore = "encodes with the codec-enabled FFmpeg profile; run with the lane's helper"]
        fn a_render_ahead_export_writes_a_probeable_file() {
            const FRAMES: u64 = 90;
            let directory =
                std::env::temp_dir().join(format!("or-export-pipeline-{}", std::process::id()));
            std::fs::create_dir_all(&directory).expect("scratch directory");
            let destination = directory.join("pipeline.mkv");
            let _ = std::fs::remove_file(&destination);

            let project = or_core::ProjectDocument::new("export pipeline");
            let rate = RationalRate::new(30, 1).expect("rate");
            let program = Arc::new(PreviewProgram {
                canvas: None,
                key: ProgramKey {
                    project_id: project.id(),
                    project_instance_id: ProjectInstanceId::generate(),
                    revision: project.revision(),
                },
                frame_rate: Some(rate),
                content_end: Some(RationalTime::new(3, 1).expect("duration")),
                video_clips: Vec::new(),
                text_clips: Vec::new(),
                audio_clips: Vec::new(),
            });
            let render_resources = Arc::new(RenderResources {
                budgets: RuntimeBudgets::new(
                    BudgetLimits::new(4, 256 * 1024 * 1024).expect("render budget"),
                    BudgetLimits::new(AUDIO_DECODE_QUEUE_CAPACITY, 32 * 1024 * 1024)
                        .expect("audio budget"),
                    BudgetLimits::new(16, 256 * 1024 * 1024).expect("decode budget"),
                ),
                text_rasterizer: Mutex::new(TextRasterizer::new()),
                video_sessions: Mutex::new(VecDeque::new()),
            });

            let jobs = JobManager::new(JobManagerConfig::new(1, 1, 8).expect("job manager"));
            let cancellation = CancellationToken::new();
            let outcome: Arc<Mutex<Option<Result<(), String>>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&outcome);
            let target = destination.clone();
            let started = Instant::now();
            jobs.submit(JobKind::Export, move |job| {
                let result = export_program(
                    program,
                    target,
                    ExportProfile::MatroskaFfv1PcmS16le,
                    ExportQuality::Quality,
                    or_core::ExportPerformance::default(),
                    (1920, 1080),
                    FRAMES,
                    rate,
                    render_resources,
                    job,
                    &cancellation,
                );
                *lock(&slot) = Some(result);
                Ok(())
            })
            .expect("export job submits");

            // The manager owns the worker thread; wait for the body to publish its
            // result instead of keeping a handle to it.
            let deadline = Instant::now() + Duration::from_secs(300);
            while lock(&outcome).is_none() {
                assert!(Instant::now() < deadline, "the export did not finish");
                thread::sleep(Duration::from_millis(10));
            }
            let result = lock(&outcome).take().expect("the export ran");
            result.expect("the export succeeded");
            jobs.shutdown();

            let elapsed = started.elapsed().as_secs_f64();
            let bytes = std::fs::metadata(&destination)
                .expect("output exists")
                .len();
            assert!(bytes > 0, "the export produced no bytes");
            println!(
                "OR_EXPORT_PIPELINE path={} bytes={bytes} frames={FRAMES} size=1920x1080 rate=30/1 seconds={elapsed:.3} fps={:.1}",
                destination.display(),
                FRAMES as f64 / elapsed.max(f64::EPSILON)
            );
        }

        /// Local export benchmark: decodes a real file through the same program
        /// shape the editor builds. Env knobs let one ignored test measure the
        /// direct path, the scaled path and the text path on the same machine.
        #[test]
        #[ignore = "benchmark; run explicitly with OR_BENCH_* set"]
        fn export_media_benchmark() {
            fn env_number(name: &str, fallback: u32) -> u32 {
                std::env::var(name)
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(fallback)
            }
            let source = std::env::var("OR_BENCH_SOURCE")
                .unwrap_or_else(|_| "/tmp/or-bench/bench1080.mp4".to_owned());
            // `encode` feeds the writer a blank frame and never opens the
            // fixture, so asking for one there only made a row look skipped.
            let needs_source = std::env::var("OR_BENCH_MODE")
                .map(|mode| mode != "encode")
                .unwrap_or(true);
            if needs_source && !std::path::Path::new(&source).exists() {
                println!(
                    "OR_EXPORT_BENCH skipped: fixture {source} is missing (set OR_BENCH_SOURCE)"
                );
                return;
            }
            let width = env_number("OR_BENCH_OUT_W", 1920);
            let height = env_number("OR_BENCH_OUT_H", 1080);
            let source_width = env_number("OR_BENCH_SRC_W", width);
            let source_height = env_number("OR_BENCH_SRC_H", height);
            let frames = u64::from(env_number("OR_BENCH_FRAMES", 300));
            let with_text = std::env::var("OR_BENCH_TEXT").is_ok_and(|value| value == "1");
            let with_animation = std::env::var("OR_BENCH_ANIM").is_ok_and(|value| value == "1");
            let animation = if with_animation {
                or_core::ClipAnimation {
                    scale_x_milli: or_core::KeyframeTrack {
                        keyframes: vec![
                            or_core::Keyframe {
                                at: RationalTime::ZERO,
                                value: 1_000,
                                easing: or_core::KeyframeEasing::Linear,
                            },
                            or_core::Keyframe {
                                at: RationalTime::new(5, 1).expect("keyframe time"),
                                value: 1_120,
                                easing: or_core::KeyframeEasing::EaseInOut,
                            },
                            or_core::Keyframe {
                                at: RationalTime::new(10, 1).expect("keyframe time"),
                                value: 1_000,
                                easing: or_core::KeyframeEasing::Linear,
                            },
                        ],
                    },
                    ..or_core::ClipAnimation::default()
                }
            } else {
                or_core::ClipAnimation::default()
            };
            let profile = std::env::var("OR_BENCH_PROFILE").unwrap_or_else(|_| "mp4".to_owned());
            let quality = match std::env::var("OR_BENCH_QUALITY").as_deref() {
                Ok("speed") => ExportQuality::Speed,
                _ => ExportQuality::Quality,
            };
            let rate = RationalRate::new(30, 1).expect("rate");
            let duration =
                RationalTime::new(i64::try_from(frames).expect("frames"), 30).expect("duration");

            if std::env::var("OR_BENCH_MODE").is_ok_and(|value| value == "encode") {
                // This branch opens no fixture and builds no program, so it also
                // builds its own destination: an encode row must be measurable
                // on a host that has no benchmark media at all.
                let extension = match profile.as_str() {
                    "webm" => "webm",
                    "mkv" => "mkv",
                    _ => "mp4",
                };
                let destination = std::env::temp_dir()
                    .join("or-export-bench")
                    .join(format!("encode-row.{extension}"));
                std::fs::create_dir_all(destination.parent().expect("parent")).expect("scratch");
                let _ = std::fs::remove_file(&destination);
                let export_profile = match profile.as_str() {
                    "webm" => ExportProfile::WebmVp9Opus,
                    "mkv" => ExportProfile::MatroskaFfv1PcmS16le,
                    _ => ExportProfile::Mp4H264Aac,
                };
                let media_profile = match export_profile {
                    ExportProfile::MatroskaFfv1PcmS16le => {
                        or_media::FfmpegExportProfile::MatroskaFfv1PcmS16le
                    }
                    ExportProfile::WebmVp9Opus => or_media::FfmpegExportProfile::WebmVp9Opus,
                    ExportProfile::Mp4H264Aac => or_media::FfmpegExportProfile::Mp4H264Aac,
                };
                let mut writer = or_media::FfmpegExportWriter::create_with_profile_and_quality(
                    &destination,
                    media_profile,
                    quality,
                    width,
                    height,
                    rate.numerator(),
                    rate.denominator(),
                )
                .expect("writer opens");
                let accepted_nv12 = writer.accepts_nv12();
                let luma = vec![16_u8; width as usize * height as usize];
                let chroma = vec![128_u8; width as usize * height as usize / 2];
                // A profile whose encoder takes RGBA (VP9, FFV1) is measured
                // with the frame shape it really receives; handing it NV12 would
                // measure a path the product never takes.
                let rgba = vec![0x80_u8; width as usize * height as usize * 4];
                let started = Instant::now();
                for frame_index in 0..frames {
                    if accepted_nv12 {
                        writer
                            .write_video_frame_nv12(&luma, &chroma, width, height, frame_index)
                            .expect("frame writes");
                    } else {
                        writer
                            .write_video_frame(&rgba, width, height, frame_index)
                            .expect("frame writes");
                    }
                }
                writer.finish().expect("writer finishes");
                let elapsed = started.elapsed().as_secs_f64();
                println!(
                    "OR_EXPORT_BENCH_ENCODE profile={profile} out={width}x{height} frames={frames} seconds={elapsed:.3} fps={:.1}",
                    frames as f64 / elapsed.max(f64::EPSILON)
                );
                return;
            }

            let project = or_core::ProjectDocument::new("export benchmark");
            let program = Arc::new(PreviewProgram {
                canvas: None,
                key: ProgramKey {
                    project_id: project.id(),
                    project_instance_id: ProjectInstanceId::generate(),
                    revision: project.revision(),
                },
                frame_rate: Some(rate),
                content_end: Some(duration),
                video_clips: vec![VideoClip {
                    source: MediaSourceRef::from_file_path(&source).expect("source path"),
                    source_dimensions: Some((source_width, source_height)),
                    timeline_start: RationalTime::ZERO,
                    source_start: RationalTime::ZERO,
                    duration,
                    playback_rate: RationalRate::new(1, 1).expect("playback rate"),
                    reversed: false,
                    frozen: false,
                    // `OR_BENCH_SCALED=1` shrinks the clip by one thousandth so
                    // the direct decode path declines and the compositor runs.
                    // Without it the no-text render row measures a passthrough,
                    // not the composited stage, and comparing it with a text row
                    // overstates what text costs.
                    transform: if std::env::var("OR_BENCH_SCALED").is_ok_and(|value| value == "1") {
                        Transform {
                            scale_x_milli: 999,
                            scale_y_milli: 999,
                            ..Transform::IDENTITY
                        }
                    } else {
                        Transform::IDENTITY
                    },
                    crop: Crop::NONE,
                    opacity: Opacity::OPAQUE,
                    animation: animation.clone(),
                    effects: Vec::new(),
                    transition_in: None,
                    transition_out: None,
                }],
                text_clips: if with_text {
                    vec![TextClip {
                        text: "Export benchmark".to_owned(),
                        formatting: TextFormatting::DEFAULT,
                        timeline_start: RationalTime::ZERO,
                        duration,
                        transform: Transform::IDENTITY,
                        crop: Crop::NONE,
                        opacity: Opacity::OPAQUE,
                        animation: ClipAnimation::default(),
                        effects: Vec::new(),
                        transition_in: None,
                        transition_out: None,
                    }]
                } else {
                    Vec::new()
                },
                audio_clips: Vec::new(),
            });
            let render_resources = Arc::new(RenderResources {
                budgets: RuntimeBudgets::new(
                    BudgetLimits::new(4, 256 * 1024 * 1024).expect("render budget"),
                    BudgetLimits::new(AUDIO_DECODE_QUEUE_CAPACITY, 32 * 1024 * 1024)
                        .expect("audio budget"),
                    BudgetLimits::new(16, 256 * 1024 * 1024).expect("decode budget"),
                ),
                text_rasterizer: Mutex::new(TextRasterizer::new()),
                video_sessions: Mutex::new(VecDeque::new()),
            });
            let directory = std::env::temp_dir().join("or-export-bench");
            std::fs::create_dir_all(&directory).expect("scratch directory");
            let extension = match profile.as_str() {
                "webm" => "webm",
                "mkv" => "mkv",
                _ => "mp4",
            };
            let destination = directory.join(format!("bench.{extension}"));
            let _ = std::fs::remove_file(&destination);
            let export_profile = match profile.as_str() {
                "webm" => ExportProfile::WebmVp9Opus,
                "mkv" => ExportProfile::MatroskaFfv1PcmS16le,
                _ => ExportProfile::Mp4H264Aac,
            };

            if std::env::var("OR_BENCH_MODE").is_ok_and(|value| value == "decode") {
                let nv12 = std::env::var("OR_BENCH_NV12").is_ok_and(|value| value == "1");
                let cancellation = CancellationToken::new();
                let clip = &program.video_clips[0];
                let started = Instant::now();
                for frame_index in 0..frames {
                    let time = rate.frame_time(frame_index).expect("frame time");
                    let frame = if nv12 {
                        render_resources
                            .decode_video_frame_nv12(
                                program.key,
                                None,
                                &clip.source,
                                time,
                                (width, height),
                                &cancellation,
                            )
                            .expect("decode succeeds")
                    } else {
                        render_resources
                            .decode_video_frame(
                                program.key,
                                None,
                                &clip.source,
                                time,
                                &cancellation,
                            )
                            .expect("decode succeeds")
                    };
                    assert!(frame.is_some(), "the fixture has no frame at {time:?}");
                }
                let elapsed = started.elapsed().as_secs_f64();
                println!(
                    "OR_EXPORT_BENCH_DECODE source={source} frames={frames} seconds={elapsed:.3} fps={:.1}",
                    frames as f64 / elapsed.max(f64::EPSILON)
                );
                return;
            }

            if std::env::var("OR_BENCH_MODE").is_ok_and(|value| value == "render") {
                let output = if export_profile == ExportProfile::Mp4H264Aac {
                    RenderOutput::Nv12
                } else {
                    RenderOutput::Packed
                };
                let started = Instant::now();
                let cancellation = CancellationToken::new();
                for frame_index in 0..frames {
                    let time = rate.frame_time(frame_index).expect("frame time");
                    render_resources
                        .render_with_size(
                            &program,
                            time,
                            Some((width, height)),
                            None,
                            true,
                            output,
                            &cancellation,
                        )
                        .expect("render succeeds")
                        .resolve_planes()
                        .expect("the frame's readback resolves");
                }
                let elapsed = started.elapsed().as_secs_f64();
                println!(
                    "OR_EXPORT_BENCH_RENDER source={source} out={width}x{height} frames={frames} text={with_text} seconds={elapsed:.3} fps={:.1}",
                    frames as f64 / elapsed.max(f64::EPSILON)
                );
                return;
            }

            let jobs = JobManager::new(JobManagerConfig::new(1, 1, 32).expect("job manager"));
            let cancellation = CancellationToken::new();
            let outcome: Arc<Mutex<Option<Result<(), String>>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&outcome);
            let target = destination.clone();
            let started = Instant::now();
            jobs.submit(JobKind::Export, move |job| {
                let result = export_program(
                    program,
                    target,
                    export_profile,
                    quality,
                    or_core::ExportPerformance::default(),
                    (width, height),
                    frames,
                    rate,
                    render_resources,
                    job,
                    &cancellation,
                );
                *lock(&slot) = Some(result);
                Ok(())
            })
            .expect("export job submits");
            let deadline = Instant::now() + Duration::from_secs(1800);
            while lock(&outcome).is_none() {
                assert!(Instant::now() < deadline, "the export did not finish");
                thread::sleep(Duration::from_millis(20));
            }
            lock(&outcome)
                .take()
                .expect("the export ran")
                .expect("the export succeeded");
            jobs.shutdown();
            let elapsed = started.elapsed().as_secs_f64();
            let bytes = std::fs::metadata(&destination)
                .expect("output exists")
                .len();
            let adapter = renderer()
                .map(|renderer| {
                    let info = renderer.adapter_info();
                    format!("{} ({}, {:?})", info.name(), info.driver(), info.backend())
                })
                .unwrap_or_else(|_| "unavailable".to_owned());
            println!("OR_EXPORT_BENCH_ADAPTER {adapter}");
            println!(
                "OR_EXPORT_BENCH profile={profile} source={source} out={width}x{height} frames={frames} text={with_text} bytes={bytes} seconds={elapsed:.3} fps={:.1}",
                frames as f64 / elapsed.max(f64::EPSILON)
            );
        }

        /// Exports one real `.orproj` through the path the Export dialog starts:
        /// the same program load from the session, the same output-size
        /// resolution, and the same render/encode pipeline.
        ///
        /// A synthetic fixture row answers "how fast is the pipeline on this
        /// host". This row answers "how long does this project take", which is
        /// the number a person compares between builds, so it names the project,
        /// its clip counts and the profile it measured.
        ///
        /// `OR_PROJECT_BENCH` is the project path. `OR_PROJECT_BENCH_PROFILE`
        /// takes `mp4`, `webm` or `mkv`; `OR_PROJECT_BENCH_QUALITY` takes
        /// `quality` or `speed`; `OR_PROJECT_BENCH_SHORT_SIDE` overrides the
        /// dialog's resolution choice. The file is written to the scratch
        /// directory and left there for an independent `ffprobe`.
        #[test]
        #[ignore = "benchmark; set OR_PROJECT_BENCH to a .orproj path"]
        fn export_project_benchmark() {
            let project_path = std::env::var("OR_PROJECT_BENCH")
                .unwrap_or_else(|_| "/tmp/or-bench/project.orproj".to_owned());
            if !std::path::Path::new(&project_path).exists() {
                println!("OR_EXPORT_PROJECT skipped: {project_path} is missing");
                return;
            }
            // Opening reads the file and probes nothing; the session is never
            // saved, so a benchmark cannot touch the project it measures.
            let file_session =
                or_core::ProjectFileSession::open(&project_path).expect("project opens");
            let program =
                Arc::new(load_program_from_session(file_session.session()).expect("program loads"));
            let rate = program.frame_rate.expect("sequence frame rate");
            let content_end = program.content_end.expect("content end");
            let frames = rate.frame_index_ceil(content_end).expect("frame count");
            let short_side = std::env::var("OR_PROJECT_BENCH_SHORT_SIDE")
                .ok()
                .and_then(|value| value.parse::<u32>().ok());
            let (width, height) = export_size(
                program.canvas,
                short_side,
                program
                    .video_clips
                    .iter()
                    .find_map(|clip| clip.source_dimensions),
            );
            let profile = match std::env::var("OR_PROJECT_BENCH_PROFILE").as_deref() {
                Ok("webm") => ExportProfile::WebmVp9Opus,
                Ok("mkv") => ExportProfile::MatroskaFfv1PcmS16le,
                _ => ExportProfile::Mp4H264Aac,
            };
            let quality = match std::env::var("OR_PROJECT_BENCH_QUALITY").as_deref() {
                Ok("speed") => ExportQuality::Speed,
                _ => ExportQuality::Quality,
            };
            let extension = match profile {
                ExportProfile::WebmVp9Opus => "webm",
                ExportProfile::MatroskaFfv1PcmS16le => "mkv",
                ExportProfile::Mp4H264Aac => "mp4",
            };
            let directory = std::env::temp_dir().join("or-export-bench");
            std::fs::create_dir_all(&directory).expect("scratch directory");
            if std::env::var("OR_PROJECT_BENCH_MODE").as_deref() == Ok("audio") {
                // Times the pre-decode stage on its own, so the cache can be
                // compared across worker counts without a full export.
                let budgets = RuntimeBudgets::new(
                    BudgetLimits::new(4, 256 * 1024 * 1024).expect("render budget"),
                    BudgetLimits::new(AUDIO_DECODE_QUEUE_CAPACITY, 32 * 1024 * 1024)
                        .expect("audio budget"),
                    BudgetLimits::new(16, 256 * 1024 * 1024).expect("decode budget"),
                );
                let cancellation = CancellationToken::new();
                let started = Instant::now();
                let cache = prepare_export_audio_cache(&program, &budgets, &cancellation)
                    .expect("the audio cache decodes");
                let elapsed = started.elapsed().as_secs_f64();
                let bytes: usize = cache.iter().map(CachedAudioClip::bytes).sum();
                println!(
                    "OR_EXPORT_PROJECT_AUDIO clips={} bytes={bytes} seconds={elapsed:.3}",
                    cache.len(),
                );
                return;
            }
            let destination = directory.join(format!("project.{extension}"));
            let _ = std::fs::remove_file(&destination);
            let render_resources = Arc::new(RenderResources {
                budgets: RuntimeBudgets::new(
                    BudgetLimits::new(4, 256 * 1024 * 1024).expect("render budget"),
                    BudgetLimits::new(AUDIO_DECODE_QUEUE_CAPACITY, 32 * 1024 * 1024)
                        .expect("audio budget"),
                    BudgetLimits::new(16, 256 * 1024 * 1024).expect("decode budget"),
                ),
                text_rasterizer: Mutex::new(TextRasterizer::new()),
                video_sessions: Mutex::new(VecDeque::new()),
            });

            let (video_clips, text_clips, audio_clips) = (
                program.video_clips.len(),
                program.text_clips.len(),
                program.audio_clips.len(),
            );
            // The direct NV12 path needs the source dimensions the program
            // carries, so a row that measured the composited path names them.
            let source_dimensions = program
                .video_clips
                .first()
                .and_then(|clip| clip.source_dimensions);
            if let Some(clip) = program.video_clips.first() {
                let resolved = clip.source.to_file_path();
                println!(
                    "OR_EXPORT_PROJECT_SOURCE uri={} path={:?} exists={:?}",
                    clip.source.uri(),
                    resolved.as_ref().map(|path| path.display().to_string()),
                    resolved.as_ref().map(|path| path.exists()).ok(),
                );
            }
            let jobs = JobManager::new(JobManagerConfig::new(1, 1, 32).expect("job manager"));
            let cancellation = CancellationToken::new();
            let outcome: Arc<Mutex<Option<Result<(), String>>>> = Arc::new(Mutex::new(None));
            let slot = Arc::clone(&outcome);
            let target = destination.clone();
            let started = Instant::now();
            jobs.submit(JobKind::Export, move |job| {
                let result = export_program(
                    program,
                    target,
                    profile,
                    quality,
                    or_core::ExportPerformance::default(),
                    (width, height),
                    frames,
                    rate,
                    render_resources,
                    job,
                    &cancellation,
                );
                *lock(&slot) = Some(result);
                Ok(())
            })
            .expect("export job submits");
            let deadline = Instant::now() + Duration::from_secs(3600);
            while lock(&outcome).is_none() {
                assert!(Instant::now() < deadline, "the export did not finish");
                thread::sleep(Duration::from_millis(20));
            }
            lock(&outcome)
                .take()
                .expect("the export ran")
                .expect("the export succeeded");
            jobs.shutdown();
            let elapsed = started.elapsed().as_secs_f64();
            let bytes = std::fs::metadata(&destination)
                .expect("output exists")
                .len();
            println!(
                "OR_EXPORT_PROJECT project={project_path} out={width}x{height} source={source_dimensions:?} rate={}/{} frames={frames} video={video_clips} text={text_clips} audio={audio_clips} profile={profile:?} quality={quality:?} bytes={bytes} seconds={elapsed:.3} fps={:.1}",
                rate.numerator(),
                rate.denominator(),
                frames as f64 / elapsed.max(f64::EPSILON)
            );
        }
    }
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "android"
)))]
mod desktop {
    use super::{PreviewError, PreviewPreparation, PreviewPreparationAction, PreviewSnapshot};
    use or_core::{
        ExportRequest, ExportResponse, MotionMaterializeRequest, MotionMaterializeResponse,
        ProjectSession, RationalTime,
    };
    use or_ipc::{
        ExportRequestHandler, LiveProjectHost, MotionMaterializeHandler, ProjectSessionIdentity,
    };
    use or_runtime::PreviewFrameStep;

    pub struct PreviewRuntime;

    impl PreviewRuntime {
        pub fn new() -> Self {
            Self
        }

        fn unavailable() -> PreviewError {
            PreviewError {
                code: "PREVIEW_UNAVAILABLE".to_owned(),
                message: "video preview is not available on this platform".to_owned(),
            }
        }

        pub fn status(&self, _host: &LiveProjectHost) -> Result<PreviewSnapshot, PreviewError> {
            Err(Self::unavailable())
        }
        pub fn seek(
            &self,
            _host: &LiveProjectHost,
            _time: RationalTime,
        ) -> Result<PreviewSnapshot, PreviewError> {
            Err(Self::unavailable())
        }
        pub fn step(
            &self,
            _host: &LiveProjectHost,
            _direction: PreviewFrameStep,
        ) -> Result<PreviewSnapshot, PreviewError> {
            Err(Self::unavailable())
        }
        pub fn play(&self, _host: &LiveProjectHost) -> Result<PreviewSnapshot, PreviewError> {
            Err(Self::unavailable())
        }
        pub fn pause(&self, _host: &LiveProjectHost) -> Result<PreviewSnapshot, PreviewError> {
            Err(Self::unavailable())
        }
        pub fn tick(&self, _host: &LiveProjectHost) -> Result<PreviewSnapshot, PreviewError> {
            Err(Self::unavailable())
        }
        pub fn prepare(
            &self,
            _host: &LiveProjectHost,
            _action: PreviewPreparationAction,
        ) -> Result<PreviewPreparation, PreviewError> {
            Err(Self::unavailable())
        }
        pub fn complete_prepared(
            &self,
            _host: &LiveProjectHost,
            _request_id: u64,
        ) -> Result<PreviewSnapshot, PreviewError> {
            Err(Self::unavailable())
        }
        pub fn abort_prepared(&self, _request_id: u64) -> Result<(), PreviewError> {
            Err(Self::unavailable())
        }
        pub fn shutdown(&self) -> Result<(), PreviewError> {
            Ok(())
        }
    }

    impl ExportRequestHandler for PreviewRuntime {
        fn handle_export_request(
            &self,
            _session: &ProjectSession,
            _request: ExportRequest,
        ) -> ExportResponse {
            ExportResponse::failure(
                "EXPORT_UNAVAILABLE",
                "software export is currently supported on desktop only",
            )
        }
    }

    impl MotionMaterializeHandler for PreviewRuntime {
        fn handle_motion_materialize_request(
            &self,
            _identity: ProjectSessionIdentity,
            _request: MotionMaterializeRequest,
        ) -> MotionMaterializeResponse {
            MotionMaterializeResponse::failure(
                "MOTION_UNAVAILABLE",
                "motion materialisation is currently supported on desktop only",
            )
        }
    }
}

pub(crate) use desktop::PreviewRuntime;

#[cfg(test)]
mod text_scissor_tests {
    use super::desktop::{TextClip, text_scissor};
    use or_core::{
        ClipAnimation, Crop, EffectReference, Opacity, RationalTime, TextFormatting, Transform,
        TransitionReference,
    };
    use or_render::TextInkBounds;

    fn clip(transform: Transform) -> TextClip {
        TextClip {
            text: "Title".into(),
            formatting: TextFormatting::DEFAULT,
            timeline_start: RationalTime::new(0, 1).expect("valid"),
            duration: RationalTime::new(1, 1).expect("valid"),
            transform,
            crop: Crop::NONE,
            opacity: Opacity::OPAQUE,
            animation: ClipAnimation::default(),
            effects: Vec::<EffectReference>::new(),
            transition_in: None::<TransitionReference>,
            transition_out: None::<TransitionReference>,
        }
    }

    fn ink() -> TextInkBounds {
        TextInkBounds {
            x: 10,
            y: 20,
            width: 100,
            height: 40,
        }
    }

    #[test]
    fn an_untouched_still_title_clips_to_its_glyph_rectangle() {
        let scissor = text_scissor(&clip(Transform::IDENTITY), true, ink());
        assert_eq!(scissor, Some((10, 20, 100, 40)));
    }

    #[test]
    fn an_effected_or_transformed_title_paints_the_whole_layer() {
        assert_eq!(text_scissor(&clip(Transform::IDENTITY), false, ink()), None);
        let moved = Transform {
            x_milli_canvas: 10_000,
            ..Transform::IDENTITY
        };
        assert_eq!(text_scissor(&clip(moved), true, ink()), None);
    }

    #[test]
    fn a_title_with_no_ink_is_skipped_rather_than_drawn() {
        assert_eq!(
            text_scissor(&clip(Transform::IDENTITY), true, TextInkBounds::EMPTY),
            None
        );
    }
}

#[cfg(test)]
mod source_time_tests {
    use super::desktop::mapped_source_time;
    use or_core::{RationalRate, RationalTime};

    fn seconds(value: i64) -> RationalTime {
        RationalTime::new(value, 1).expect("valid time")
    }

    #[test]
    fn a_frozen_clip_holds_its_first_frame() {
        let start = seconds(3);
        for offset in [seconds(0), seconds(1), seconds(5)] {
            let mapped =
                mapped_source_time(start, seconds(4), RationalRate::ONE, offset, false, true)
                    .expect("frozen mapping");
            assert_eq!(
                mapped, start,
                "a frozen clip must not advance at {offset:?}"
            );
        }
    }

    #[test]
    fn a_forward_clip_walks_past_its_source_start() {
        let mapped = mapped_source_time(
            seconds(1),
            seconds(2),
            RationalRate::ONE,
            seconds(2),
            false,
            false,
        )
        .expect("forward mapping");
        assert_eq!(mapped, seconds(3));
    }

    #[test]
    fn a_reversed_clip_mirrors_inside_its_own_span() {
        let mapped = mapped_source_time(
            seconds(1),
            seconds(2),
            RationalRate::ONE,
            seconds(2),
            true,
            false,
        )
        .expect("reversed mapping");
        assert_eq!(mapped, seconds(1), "the last offset maps back to the start");
    }
}
