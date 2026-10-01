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

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
mod desktop {
    use super::{PreviewError, PreviewSnapshot};
    use crate::viewer_texture;
    use or_core::{
        ApplicationRequest, ApplicationResponse, ClipContent, ClipSettings, Crop,
        MAX_TIMELINE_CLIP_PAGE_SIZE, MediaId, MediaSourceRef, Opacity, ProjectId,
        ProjectInstanceId, ProjectRevision, QueryEnvelope, QueryResult, RationalRate, RationalTime,
        TimeRange, TimelineClipState, TimelineTrackSummaryV2, TrackKind, Transform,
    };
    use or_ipc::LiveProjectHost;
    use or_media::SoftwareMediaDecoder;
    use or_render::{RenderDevice, RenderSize, RgbaVideoLayer};
    use or_runtime::{
        BudgetLimits, CancellationToken, PreviewFrameRequest, PreviewFrameStep, PreviewTransport,
        RenderSnapshot, RuntimeBudgets,
    };
    use std::{
        collections::HashMap,
        future::Future,
        pin::Pin,
        sync::{
            Arc, Mutex, MutexGuard, OnceLock,
            atomic::{AtomicU64, Ordering},
        },
        task::{Context, Poll, Wake, Waker},
        thread,
        time::{Duration, Instant},
    };

    const PAGE_SIZE: usize = 100;
    const MAX_VIEWER_WIDTH: u32 = 1920;
    const MAX_VIEWER_HEIGHT: u32 = 1080;
    const MAX_BLANK_WIDTH: u32 = 640;
    const MAX_BLANK_HEIGHT: u32 = 360;

    static RENDER_DEVICE: OnceLock<Result<RenderDevice, String>> = OnceLock::new();
    static FFMPEG_LICENSE_OK: OnceLock<bool> = OnceLock::new();

    pub struct PreviewRuntime {
        state: Mutex<SessionState>,
        render_lock: Mutex<()>,
        cancellation: Mutex<CancellationToken>,
        generation: AtomicU64,
        budgets: RuntimeBudgets,
    }

    impl PreviewRuntime {
        pub fn new() -> Self {
            let budgets = RuntimeBudgets::new(
                BudgetLimits::new(4, 256 * 1024 * 1024).expect("nonzero render budget"),
                BudgetLimits::new(2, 32 * 1024 * 1024).expect("nonzero audio budget"),
                BudgetLimits::new(16, 256 * 1024 * 1024).expect("nonzero decode budget"),
            );
            Self {
                state: Mutex::new(SessionState::default()),
                render_lock: Mutex::new(()),
                cancellation: Mutex::new(CancellationToken::new()),
                generation: AtomicU64::new(0),
                budgets,
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
            if time.is_negative() {
                return Err(PreviewError::new(
                    "NEGATIVE_PREVIEW_TIME",
                    "preview time must be nonnegative",
                ));
            }
            let program = self.ensure_program(host)?;
            let (request, external_generation, cancellation) = {
                let mut state = lock(&self.state);
                let playback = state.transport.seek(time).map_err(transport_error)?;
                state.error_code = None;
                state.error_message = None;
                let (external_generation, cancellation) = self.invalidate()?;
                (
                    PreviewFrameRequest {
                        time,
                        frame_index: None,
                        generation: playback.generation,
                    },
                    external_generation,
                    cancellation,
                )
            };
            self.render_request(host, program, request, external_generation, cancellation);
            Ok(self.snapshot())
        }

        pub fn step(
            &self,
            host: &LiveProjectHost,
            direction: PreviewFrameStep,
        ) -> Result<PreviewSnapshot, PreviewError> {
            let program = self.ensure_program(host)?;
            let action = {
                let mut state = lock(&self.state);
                let Some(request) = state.transport.step(direction).map_err(transport_error)?
                else {
                    return Ok(state.snapshot());
                };
                state.error_code = None;
                state.error_message = None;
                let (external_generation, cancellation) = self.invalidate()?;
                Some((request, external_generation, cancellation))
            };
            if let Some((request, external_generation, cancellation)) = action {
                self.render_request(host, program, request, external_generation, cancellation);
            }
            Ok(self.snapshot())
        }

        pub fn play(&self, host: &LiveProjectHost) -> Result<PreviewSnapshot, PreviewError> {
            let _ = self.ensure_program(host)?;
            let mut state = lock(&self.state);
            let previous_generation = state.transport.snapshot().generation;
            let playback = state
                .transport
                .play(Instant::now())
                .map_err(transport_error)?;
            if playback.generation != previous_generation {
                self.invalidate()?;
            }
            state.error_code = None;
            state.error_message = None;
            Ok(state.snapshot())
        }

        pub fn pause(&self, host: &LiveProjectHost) -> Result<PreviewSnapshot, PreviewError> {
            let _ = self.ensure_program(host)?;
            let mut state = lock(&self.state);
            state
                .transport
                .pause(Instant::now())
                .map_err(transport_error)?;
            Ok(state.snapshot())
        }

        pub fn tick(&self, host: &LiveProjectHost) -> Result<PreviewSnapshot, PreviewError> {
            let program = self.ensure_program(host)?;
            let action = {
                let mut state = lock(&self.state);
                let Some(request) = state
                    .transport
                    .tick(Instant::now())
                    .map_err(transport_error)?
                else {
                    return Ok(state.snapshot());
                };
                let generation = self.generation.load(Ordering::SeqCst);
                let cancellation = lock(&self.cancellation).clone();
                Some((request, generation, cancellation))
            };
            if let Some((request, generation, cancellation)) = action {
                self.render_request(host, program, request, generation, cancellation);
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
            }
            Ok(state.program.as_ref().cloned().unwrap_or(program))
        }

        fn invalidate(&self) -> Result<(u64, CancellationToken), PreviewError> {
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
            if cancellation.is_cancelled() {
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
                let decoder = match SoftwareMediaDecoder::new(&clip.source, self.budgets.clone()) {
                    Ok(decoder) => decoder,
                    Err(error) => {
                        decode_error.get_or_insert_with(|| error.to_string());
                        continue;
                    }
                };
                match decoder.decode_video_frame_at(source_time, cancellation) {
                    Ok(Some(frame)) => {
                        frames.push((frame, clip.transform, clip.crop, clip.opacity));
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
            if cancellation.is_cancelled() || self.generation.load(Ordering::SeqCst) != generation {
                return Err(PreviewError::new(
                    "PREVIEW_CANCELLED",
                    "preview request was superseded",
                ));
            }

            let (width, height) = frames
                .first()
                .map(|(frame, _, _, _)| {
                    fit_size(frame.descriptor().width(), frame.descriptor().height())
                })
                .unwrap_or((MAX_BLANK_WIDTH, MAX_BLANK_HEIGHT));
            let size = RenderSize::new(width, height)
                .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?;
            let layers = frames
                .iter()
                .map(|(frame, transform, crop, opacity)| {
                    RgbaVideoLayer::new(
                        frame.descriptor().width(),
                        frame.descriptor().height(),
                        frame.pixels(),
                    )?
                    .with_visual_settings(*transform, *crop, *opacity)
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| PreviewError::new("RENDER_FAILED", error.to_string()))?;
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

    impl Drop for PreviewRuntime {
        fn drop(&mut self) {
            lock(&self.cancellation).cancel();
        }
    }

    struct SessionState {
        key: Option<ProgramKey>,
        program: Option<Arc<PreviewProgram>>,
        transport: PreviewTransport,
        width: u32,
        height: u32,
        error_code: Option<String>,
        error_message: Option<String>,
    }

    impl Default for SessionState {
        fn default() -> Self {
            Self {
                key: None,
                program: None,
                transport: PreviewTransport::new(None, None),
                width: 0,
                height: 0,
                error_code: None,
                error_message: None,
            }
        }
    }

    impl SessionState {
        fn snapshot(&self) -> PreviewSnapshot {
            PreviewSnapshot {
                playback: self.transport.snapshot(),
                width: self.width,
                height: self.height,
                error_code: self.error_code.clone(),
                error_message: self.error_message.clone(),
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
    }

    struct VideoClip {
        source: MediaSourceRef,
        timeline_start: RationalTime,
        source_start: RationalTime,
        duration: RationalTime,
        transform: Transform,
        crop: Crop,
        opacity: Opacity,
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
        let mut media_sources = HashMap::new();
        let mut offset = 0usize;
        loop {
            let result = query(
                host,
                QueryEnvelope::media_list(
                    key.project_id,
                    key.project_instance_id,
                    offset,
                    PAGE_SIZE,
                ),
            )?;
            ensure_key(&result, key)?;
            let page = result.media_page.ok_or_else(|| {
                PreviewError::new("PREVIEW_QUERY_FAILED", "media page was missing")
            })?;
            for item in page.items {
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

        let result = query(
            host,
            QueryEnvelope::timeline_tracks_v2(key.project_id, key.project_instance_id),
        )?;
        ensure_key(&result, key)?;
        let tracks = result.timeline_tracks_v2.ok_or_else(|| {
            PreviewError::new("PREVIEW_QUERY_FAILED", "timeline tracks were missing")
        })?;
        let mut content_end = None;
        let mut video_clips = Vec::new();
        let has_video_solo = tracks
            .iter()
            .any(|track| track.kind == TrackKind::Video && track.state.solo());
        for track in tracks {
            let render_track = track.kind == TrackKind::Video
                && track.state.visible()
                && (!has_video_solo || track.state.solo());
            for clip in list_clips(host, key, &track)? {
                add_clip(
                    &mut content_end,
                    &mut video_clips,
                    track.kind,
                    render_track,
                    clip,
                    &media_sources,
                )?;
            }
        }

        let result = query(
            host,
            QueryEnvelope::timeline_sequence_settings(key.project_id, key.project_instance_id),
        )?;
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
        })
    }

    fn list_clips(
        host: &LiveProjectHost,
        key: ProgramKey,
        track: &TimelineTrackSummaryV2,
    ) -> Result<Vec<TimelineClipState>, PreviewError> {
        let mut clips = Vec::new();
        let mut offset = 0usize;
        loop {
            let result = query(
                host,
                QueryEnvelope::timeline_clips_v2(
                    key.project_id,
                    key.project_instance_id,
                    track.track_id,
                    offset,
                    MAX_TIMELINE_CLIP_PAGE_SIZE,
                ),
            )?;
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

    fn add_clip(
        content_end: &mut Option<RationalTime>,
        video_clips: &mut Vec<VideoClip>,
        track_kind: TrackKind,
        render_track: bool,
        clip: TimelineClipState,
        sources: &HashMap<MediaId, MediaSourceRef>,
    ) -> Result<(), PreviewError> {
        let end = clip
            .timeline_start
            .checked_add(clip.timeline_duration)
            .map_err(|error| PreviewError::new("INVALID_TIMELINE_TIME", error.to_string()))?;
        *content_end = Some(content_end.map_or(end, |current| current.max(end)));
        if track_kind == TrackKind::Video && render_track {
            let ClipContent::Media {
                media_id,
                source_range,
            } = clip.content
            else {
                return Err(PreviewError::new(
                    "PREVIEW_QUERY_FAILED",
                    "video track contained unsupported clip content",
                ));
            };
            let source = sources.get(&media_id).cloned().ok_or_else(|| {
                PreviewError::new("PREVIEW_QUERY_FAILED", "timeline media source was missing")
            })?;
            let ClipSettings::Visual(settings) = clip.settings else {
                return Err(PreviewError::new(
                    "PREVIEW_QUERY_FAILED",
                    "video clip visual settings were missing",
                ));
            };
            video_clips.push(VideoClip {
                source,
                timeline_start: clip.timeline_start,
                source_start: source_range.start(),
                duration: clip.timeline_duration,
                transform: settings.transform,
                crop: settings.crop,
                opacity: settings.opacity,
            });
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
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
mod desktop {
    use super::{PreviewError, PreviewSnapshot};
    use or_core::RationalTime;
    use or_ipc::LiveProjectHost;
    use or_runtime::PreviewFrameStep;

    pub struct PreviewRuntime;

    impl PreviewRuntime {
        pub fn new() -> Self {
            Self
        }

        fn unavailable() -> PreviewError {
            PreviewError {
                code: "PREVIEW_UNAVAILABLE".to_owned(),
                message: "video preview is currently supported on desktop only".to_owned(),
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
    }
}

pub(crate) use desktop::PreviewRuntime;
