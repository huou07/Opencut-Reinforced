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
        AudioClockMessage, AudioProducer, AvSynchronizer, VideoSyncAction, process_audio_clip,
    };
    use or_audio::{AudioDeviceOutput, AudioDeviceOutputError};
    use or_core::{
        ApplicationRequest, ApplicationResponse, AudioSettings, ClipContent, ClipSettings, Crop,
        EffectReference, ExportRequest, ExportResponse, JobCancelOutcome, JobContext, JobFailure,
        JobKind, JobManager, JobManagerConfig, JobState, MAX_TIMELINE_CLIP_PAGE_SIZE, MediaId,
        MediaSourceRef, MediaStreamMetadata, Opacity, ProjectId, ProjectInstanceId,
        ProjectRevision, ProjectSession, QueryEnvelope, QueryResult, RationalRate, RationalTime,
        TextFormatting, TimeRange, TimelineClipState, TimelineTrackSummaryV2, TrackKind, Transform,
        TransitionReference,
    };
    use or_ipc::{ExportRequestHandler, LiveProjectHost};
    use or_media::{SnapshotQueue, SoftwareMediaDecoder, VideoDecodeSession};
    use or_render::{
        RenderDevice, RenderSize, RgbaVideoLayer, TextRasterizer, VisualTransition,
        process_visual_rgba,
    };
    use or_runtime::{
        BudgetLimits, CancellationToken, PreviewFrameRequest, PreviewFrameStep, PreviewTransport,
        RenderSnapshot, RuntimeBudgets,
    };
    use std::{
        collections::{HashMap, VecDeque},
        future::Future,
        num::NonZeroUsize,
        pin::Pin,
        sync::{
            Arc, Mutex, MutexGuard, OnceLock,
            atomic::{AtomicBool, AtomicU64, Ordering},
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

    static RENDER_DEVICE: OnceLock<Result<RenderDevice, String>> = OnceLock::new();
    static FFMPEG_LICENSE_OK: OnceLock<bool> = OnceLock::new();

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
                    let destination = std::path::PathBuf::from(destination);
                    if !destination.is_absolute()
                        || !destination
                            .extension()
                            .and_then(std::ffi::OsStr::to_str)
                            .is_some_and(|extension| extension.eq_ignore_ascii_case("mkv"))
                    {
                        return ExportResponse::failure(
                            "INVALID_EXPORT_DESTINATION",
                            "choose an absolute Matroska destination with an .mkv extension",
                        );
                    }
                    if !*FFMPEG_LICENSE_OK.get_or_init(or_media::verify_ffmpeg_runtime) {
                        return ExportResponse::failure(
                            "FFMPEG_UNAVAILABLE",
                            "the approved LGPL FFmpeg runtime could not be loaded",
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
                    let output_size = program
                        .video_clips
                        .iter()
                        .find_map(|clip| clip.source_dimensions)
                        .map(|(width, height)| fit_size(width, height))
                        .unwrap_or((MAX_BLANK_WIDTH, MAX_BLANK_HEIGHT));
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
            let program = self.ensure_program(host)?;
            let (origin, expected_generation) = {
                let state = lock(&self.state);
                let playback = state.transport.snapshot();
                if playback.playing {
                    return Ok(state.snapshot());
                }
                if playback.frame_rate.is_none() {
                    return Err(transport_error(
                        or_runtime::PreviewTransportError::FrameRateUnavailable,
                    ));
                }
                if playback
                    .content_end
                    .is_none_or(|content_end| playback.position >= content_end)
                {
                    return Ok(state.snapshot());
                }
                (playback.position, playback.generation)
            };
            let (audio_playback, audio_error) = if program.audio_clips.is_empty() {
                (None, None)
            } else {
                match AudioPlayback::start(
                    Arc::clone(&program),
                    origin,
                    self.render_resources.budgets.clone(),
                ) {
                    Ok(audio) => (Some(audio), None),
                    Err(error) => (None, Some(error.message)),
                }
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
                    self.play(host)?;
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
                    let sources = program.active_saf_sources(request.time)?;
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
            self.render_resources.render_with_size(
                program,
                time,
                None,
                Some((generation, &self.generation)),
                false,
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
            let key = VideoSessionKey {
                program,
                generation,
                source: source.uri().to_owned(),
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
                        .open_video_session(cancellation)?,
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

        fn render_with_size(
            &self,
            program: &PreviewProgram,
            time: RationalTime,
            fixed_size: Option<(u32, u32)>,
            expected_generation: Option<(u64, &AtomicU64)>,
            fail_on_decode_error: bool,
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
                let source_time = clip
                    .source_start
                    .checked_add(offset)
                    .map_err(|error| PreviewError::new("INVALID_SOURCE_TIME", error.to_string()))?;
                let runtime_generation = expected_generation.map(|(generation, _)| generation);
                match self.decode_video_frame(
                    program.key,
                    runtime_generation,
                    &clip.source,
                    source_time,
                    cancellation,
                ) {
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
                let processed = if clip.effects.is_empty() && transitions.is_empty() {
                    None
                } else {
                    let pixels = process_visual_rgba(
                        frame.descriptor().width(),
                        frame.descriptor().height(),
                        frame.pixels(),
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
                };
                processed_video_pixels.push(processed);
            }
            let mut layers = frames
                .iter()
                .zip(&processed_video_pixels)
                .map(|((frame, clip), processed)| {
                    let pixels = processed.as_deref().unwrap_or_else(|| frame.pixels());
                    RgbaVideoLayer::new(
                        frame.descriptor().width(),
                        frame.descriptor().height(),
                        pixels,
                    )?
                    .with_visual_settings(
                        clip.transform,
                        clip.crop,
                        clip.opacity,
                    )
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?;
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
            let mut text_pixels = Vec::with_capacity(active_text_clips.len());
            for clip in &active_text_clips {
                let pixels = lock(&self.text_rasterizer)
                    .rasterize(&clip.text, clip.formatting, width, height)
                    .map_err(|error| PreviewError::new("TEXT_RENDER_FAILED", error.to_string()))?;
                let transitions = clip_transitions(
                    time,
                    clip.timeline_start,
                    clip.duration,
                    clip.transition_in,
                    clip.transition_out,
                )?;
                let processed = if clip.effects.is_empty() && transitions.is_empty() {
                    pixels
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
                    processed
                };
                text_pixels.push(processed);
            }
            debug_assert!(text_pixels.iter().map(Vec::len).sum::<usize>() <= text_bytes);
            for (clip, pixels) in active_text_clips.iter().zip(&text_pixels) {
                layers.push(
                    RgbaVideoLayer::new(width, height, pixels)
                        .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?
                        .with_visual_settings(clip.transform, clip.crop, clip.opacity)
                        .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?,
                );
            }
            let renderer = renderer()?;
            let rendered = renderer
                .render_rgba_layers(snapshot, size, &layers)
                .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?;
            if cancellation.is_cancelled() {
                return Err(PreviewError::new(
                    "PREVIEW_CANCELLED",
                    "preview request was superseded",
                ));
            }
            Ok(Presentation {
                width,
                height,
                pixels: rendered.pixels().to_vec(),
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
        video_clips: Vec<VideoClip>,
        text_clips: Vec<TextClip>,
        audio_clips: Vec<AudioClip>,
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
        transform: Transform,
        crop: Crop,
        opacity: Opacity,
        effects: Vec<EffectReference>,
        transition_in: Option<TransitionReference>,
        transition_out: Option<TransitionReference>,
    }

    struct TextClip {
        text: String,
        formatting: TextFormatting,
        timeline_start: RationalTime,
        duration: RationalTime,
        transform: Transform,
        crop: Crop,
        opacity: Opacity,
        effects: Vec<EffectReference>,
        transition_in: Option<TransitionReference>,
        transition_out: Option<TransitionReference>,
    }

    #[derive(Clone)]
    struct AudioClip {
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
        error_code: Option<String>,
        error_message: Option<String>,
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
                )?;
            }
        }

        let result = query_request(QueryEnvelope::timeline_sequence_settings(
            key.project_id,
            key.project_instance_id,
        ))?;
        ensure_key(&result, key)?;
        let frame_rate = result
            .timeline_sequence_settings
            .ok_or_else(|| {
                PreviewError::new("PREVIEW_QUERY_FAILED", "sequence settings were missing")
            })?
            .sequence_frame_rate;
        Ok(PreviewProgram {
            key,
            frame_rate,
            content_end,
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
                },
                ClipSettings::Visual(settings),
            ) => {
                let source = sources.get(&media_id).cloned().ok_or_else(|| {
                    PreviewError::new("PREVIEW_QUERY_FAILED", "timeline media source was missing")
                })?;
                video_clips.push(VideoClip {
                    source,
                    source_dimensions: media_dimensions.get(&media_id).copied(),
                    timeline_start: clip.timeline_start,
                    source_start: source_range.start(),
                    duration: clip.timeline_duration,
                    transform: settings.transform,
                    crop: settings.crop,
                    opacity: settings.opacity,
                    effects: settings.effects,
                    transition_in: settings.transition_in,
                    transition_out: settings.transition_out,
                });
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
                },
                ClipSettings::Audio(settings),
            ) => {
                let source = sources.get(&media_id).cloned().ok_or_else(|| {
                    PreviewError::new("PREVIEW_QUERY_FAILED", "timeline media source was missing")
                })?;
                audio_clips.push(AudioClip {
                    source,
                    timeline_start: clip.timeline_start,
                    source_start: source_range.start(),
                    duration: clip.timeline_duration,
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

    fn renderer() -> Result<&'static RenderDevice, PreviewError> {
        RENDER_DEVICE
            .get_or_init(|| block_on(RenderDevice::new()).map_err(|error| error.to_string()))
            .as_ref()
            .map_err(|message| PreviewError::new("RENDER_UNAVAILABLE", message.clone()))
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

    #[allow(clippy::too_many_arguments)]
    fn export_program(
        program: Arc<PreviewProgram>,
        destination: std::path::PathBuf,
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
        let mut writer = or_media::MatroskaFfv1PcmS16leWriter::create(
            destination,
            output_size.0,
            output_size.1,
            frame_rate.numerator(),
            frame_rate.denominator(),
        )
        .map_err(|error| error.to_string())?;
        let mut audio_cursor = 0_u64;
        job.report_progress(0, frame_count);
        for frame_index in 0..frame_count {
            if job.is_cancelled() || cancellation.is_cancelled() {
                return Err("export was cancelled".to_owned());
            }
            let time = frame_rate
                .frame_time(frame_index)
                .map_err(|error| error.to_string())?;
            let presentation = render_resources
                .render_with_size(&program, time, Some(output_size), None, true, cancellation)
                .map_err(|error| error.to_string())?;
            writer
                .write_video_frame(
                    &presentation.pixels,
                    presentation.width,
                    presentation.height,
                    frame_index,
                )
                .map_err(|error| error.to_string())?;
            let next_time = frame_rate
                .frame_time(frame_index.saturating_add(1))
                .map_err(|error| error.to_string())?;
            let audio_until = audio_frames_floor(next_time)
                .map_err(|_| "audio output time exceeds the sample clock".to_owned())?
                .max(0) as u64;
            write_export_audio_until(
                &mut writer,
                &program,
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
            &render_resources.budgets,
            &mut audio_cursor,
            total_audio_frames,
            job,
            cancellation,
        )?;
        if job.is_cancelled() || cancellation.is_cancelled() {
            return Err("export was cancelled".to_owned());
        }
        writer.finish().map_err(|error| error.to_string())
    }

    fn write_export_audio_until(
        writer: &mut or_media::MatroskaFfv1PcmS16leWriter,
        program: &PreviewProgram,
        budgets: &RuntimeBudgets,
        audio_cursor: &mut u64,
        audio_until: u64,
        job: &JobContext,
        cancellation: &CancellationToken,
    ) -> Result<(), String> {
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
            let error = Mutex::new(None);
            mix_audio_window(
                &mut mixed,
                &program.audio_clips,
                program.key,
                block_start,
                budgets.clone(),
                cancellation,
                &error,
                true,
            )?;
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
            let source_start = clip
                .source_start
                .checked_add(
                    overlap_start
                        .checked_sub(clip.timeline_start)
                        .map_err(|_| "audio source offset overflowed".to_owned())?,
                )
                .map_err(|_| "audio source time overflowed".to_owned())?;
            let range = TimeRange::new(
                source_start,
                overlap_end
                    .checked_sub(overlap_start)
                    .map_err(|_| "audio overlap duration overflowed".to_owned())?,
            )
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
                                mix_audio_chunk(output, clip, block_start, item.into_value())
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

    fn mix_audio_chunk(
        output: &mut [f32],
        clip: &AudioClip,
        block_start: RationalTime,
        chunk: or_media::AudioChunk,
    ) -> Result<(), String> {
        let clip_offset = chunk
            .timestamp()
            .checked_sub(clip.source_start)
            .map_err(|_| "audio chunk starts before its clip source range".to_owned())?;
        let chunk_timeline = clip
            .timeline_start
            .checked_add(clip_offset)
            .map_err(|_| "audio chunk timeline time overflowed".to_owned())?;
        let output_offset = audio_frames_floor(
            chunk_timeline
                .checked_sub(block_start)
                .map_err(|_| "audio chunk offset overflowed".to_owned())?,
        )
        .map_err(|_| "audio chunk offset exceeds the device clock".to_owned())?;
        let mut samples = chunk.samples().to_vec();
        process_audio_clip(&mut samples, clip_offset, clip.duration, clip.settings)
            .map_err(|error| error.to_string())?;
        let source_frame = if output_offset < 0 {
            usize::try_from(output_offset.unsigned_abs()).unwrap_or(usize::MAX)
        } else {
            0
        };
        let destination_frame = usize::try_from(output_offset.max(0)).unwrap_or(usize::MAX);
        let available_source_frames = samples.len() / 2;
        let output_frames = output.len() / 2;
        if source_frame >= available_source_frames || destination_frame >= output_frames {
            return Ok(());
        }
        let mix_frames =
            (available_source_frames - source_frame).min(output_frames - destination_frame);
        for frame in 0..mix_frames {
            let destination = (destination_frame + frame) * 2;
            let source = (source_frame + frame) * 2;
            output[destination] += samples[source];
            output[destination + 1] += samples[source + 1];
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

        fn late_source_project() -> or_core::ProjectDocument {
            let media = (1..=65).map(|index| format!(r#"{{"id":"{index:08x}-2222-4222-8222-222222222222","source":{{"kind":"android_saf_document_uri","uri":"content://dev.opencut.fixture/document/clip-{index:02}"}},"metadata":{{"format_names":["matroska"],"duration":{{"numerator":1,"denominator":1}},"file_size_bytes":1024,"streams":[{{"kind":"video","metadata":{{"index":0,"codec_name":"ffv1","width":16,"height":16,"pixel_format":"bgra","average_frame_rate":{{"numerator":4,"denominator":1}},"duration":{{"numerator":1,"denominator":1}}}}}}]}}}}"#)).collect::<Vec<_>>().join(",");
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
            session.project().clone()
        }

        #[test]
        fn evaluated_late_library_source_is_active_and_half_open() {
            let project = late_source_project();
            assert_eq!(project.media_items().len(), 65);
            let session = ProjectSession::open(project);
            let mut program = load_program_from_session(&session).unwrap();
            assert_eq!(
                program.active_saf_sources(RationalTime::ZERO).unwrap(),
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
            assert_eq!(session.project_revision().value(), 2);
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
                ["content://dev.opencut.fixture/document/clip-65"]
            );
            let old = play
                .request_id
                .expect("play has a first frame without prior seek");
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
            assert_eq!(host.describe().unwrap().summary.project_revision.value(), 2);
            host.shutdown(false).unwrap();
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
    use or_core::{ExportRequest, ExportResponse, ProjectSession, RationalTime};
    use or_ipc::{ExportRequestHandler, LiveProjectHost};
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
}

pub(crate) use desktop::PreviewRuntime;
