use crate::{SnapshotQueue, SnapshotQueueSendError};
use ffmpeg::{
    ChannelLayout, ChannelLayoutMask, Error as FfmpegError, codec, format, frame,
    software::{resampling, scaling},
    util::{
        format::{
            Pixel,
            sample::{Sample, Type},
        },
        frame::Video as VideoFrameBuffer,
    },
};
use ffmpeg_the_third as ffmpeg;
use or_core::{
    MediaSourceRef, MediaSourceUriError, RationalRate, RationalTime, TimeError, TimeRange,
};
use or_runtime::{
    BudgetAcquireError, BudgetLease, FrameDescriptor, FrameDescriptorError, FrameLease,
    FrameLeaseError, FramePixelFormat, RenderSnapshot, RuntimeBudgets,
};
#[cfg(not(windows))]
use std::io::ErrorKind;
use std::{
    error::Error,
    fmt,
    io::{self, Seek, SeekFrom, Write},
    path::PathBuf,
};
#[cfg(unix)]
use std::{fs::File, sync::Arc};

const OUTPUT_AUDIO_RATE: u32 = 48_000;
const OUTPUT_AUDIO_CHANNELS: usize = 2;
const TRANSCRIPTION_AUDIO_RATE: u32 = 16_000;
const MAX_TRANSCRIPTION_AUDIO_DURATION_MS: u64 = 4 * 60 * 60 * 1_000;
const MAX_TRANSCRIPTION_WAV_BYTES: u64 = 512 * 1024 * 1024;
const MAX_TRANSCRIPTION_FRAME_BYTES: u128 = 64 * 1024 * 1024;
const MAX_SOFTWARE_FRAME_BYTES: usize = 256 * 1024 * 1024;
const MAX_AUDIO_INPUT_SAMPLES: usize = 1_048_576;
const MAX_AUDIO_CHUNK_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum DisplayRotation {
    #[default]
    None,
    Clockwise90,
    HalfTurn,
    CounterClockwise90,
}

#[derive(Clone, Copy)]
struct VideoStreamPresentation {
    time_base: TimestampBase,
    display_rotation: DisplayRotation,
}

impl DisplayRotation {
    fn from_degrees(degrees: f64) -> Result<Self, DecodeError> {
        if !degrees.is_finite() {
            return Err(DecodeError::InvalidDisplayMatrix);
        }
        let degrees = (degrees.round() as i32).rem_euclid(360);
        match degrees {
            0 => Ok(Self::None),
            90 => Ok(Self::CounterClockwise90),
            180 => Ok(Self::HalfTurn),
            270 => Ok(Self::Clockwise90),
            _ => Err(DecodeError::UnsupportedDisplayRotation(degrees)),
        }
    }

    const fn output_size(self, width: u32, height: u32) -> (u32, u32) {
        match self {
            Self::Clockwise90 | Self::CounterClockwise90 => (height, width),
            Self::None | Self::HalfTurn => (width, height),
        }
    }

    const fn output_coordinate(
        self,
        width: usize,
        height: usize,
        column: usize,
        row: usize,
    ) -> (usize, usize) {
        match self {
            Self::None => (column, row),
            Self::Clockwise90 => (height - row - 1, column),
            Self::HalfTurn => (width - column - 1, height - row - 1),
            Self::CounterClockwise90 => (row, width - column - 1),
        }
    }
}

/// A packed RGBA software frame. The runtime budget stays held until the frame
/// is dropped, including while it waits in a bounded queue.
#[derive(Debug)]
pub struct VideoFrame {
    lease: FrameLease,
    _budget: BudgetLease,
}

impl VideoFrame {
    pub const fn descriptor(&self) -> &FrameDescriptor {
        self.lease.descriptor()
    }

    pub fn pixels(&self) -> &[u8] {
        self.lease
            .software_bytes()
            .expect("software decoder frames own packed pixels")
    }

    fn duplicate(&self, budgets: &RuntimeBudgets) -> Result<Self, DecodeError> {
        let descriptor = *self.descriptor();
        let pixels = self.pixels();
        let budget = budgets
            .decode()
            .try_acquire(pixels.len() as u64)
            .map_err(DecodeError::Budget)?;
        let descriptor = FrameDescriptor::software(
            descriptor.width(),
            descriptor.height(),
            FramePixelFormat::Rgba8,
            descriptor.timing().timestamp(),
        )?;
        let lease = FrameLease::from_software(descriptor, pixels.to_vec())?;
        Ok(Self {
            lease,
            _budget: budget,
        })
    }
}

/// Interleaved stereo F32 audio at 48 kHz.
#[derive(Debug)]
pub struct AudioChunk {
    timestamp: RationalTime,
    samples: Vec<f32>,
    _budget: BudgetLease,
}

/// Opaque, runtime-only seekable media access backed by an already-open file
/// descriptor. FFmpeg reads it directly through custom AVIO; no inode is
/// reopened and the source URI is never mapped to a filesystem path.
#[cfg(unix)]
#[derive(Clone)]
pub struct SeekableMediaIoCapability {
    pub(super) file: Arc<File>,
    pub(super) length: i64,
}

#[cfg(unix)]
impl SeekableMediaIoCapability {
    /// Takes ownership of an open descriptor after confirming it supports seek.
    /// Android SAF adapters should pass a duplicated descriptor so this value
    /// owns its lifetime independently of the provider callback.
    pub fn from_file(mut file: File) -> Result<Self, io::Error> {
        use io::{Seek, SeekFrom};

        let position = file.stream_position()?;
        let length = i64::try_from(file.seek(SeekFrom::End(0))?).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "seekable source length exceeds i64",
            )
        })?;
        file.seek(SeekFrom::Start(position))?;
        Ok(Self {
            file: Arc::new(file),
            length,
        })
    }

    pub const fn len(&self) -> u64 {
        self.length as u64
    }

    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl AudioChunk {
    pub const fn timestamp(&self) -> RationalTime {
        self.timestamp
    }

    pub const fn sample_rate(&self) -> u32 {
        OUTPUT_AUDIO_RATE
    }

    pub const fn channels(&self) -> usize {
        OUTPUT_AUDIO_CHANNELS
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn sample_frames(&self) -> usize {
        self.samples.len() / OUTPUT_AUDIO_CHANNELS
    }
}

/// FFmpeg-backed software demux and decode for local files or owned seekable I/O.
#[derive(Clone)]
pub struct SoftwareMediaDecoder {
    path: PathBuf,
    #[cfg(unix)]
    _media_io: Option<SeekableMediaIoCapability>,
    budgets: RuntimeBudgets,
}

// Field order closes the format context before its custom AVIO owner.
struct MediaInput {
    input: format::context::Input,
    #[cfg(unix)]
    _io: Option<crate::seekable_io::CapabilityIo>,
}
impl std::ops::Deref for MediaInput {
    type Target = format::context::Input;
    fn deref(&self) -> &Self::Target {
        &self.input
    }
}
impl std::ops::DerefMut for MediaInput {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.input
    }
}

fn is_still_image_format(name: &str) -> bool {
    name.split(',').map(str::trim).any(|format| {
        matches!(
            format,
            "image2" | "image2pipe" | "jpeg_pipe" | "png_pipe" | "webp_pipe"
        )
    })
}

impl SoftwareMediaDecoder {
    pub fn new(source: &MediaSourceRef, budgets: RuntimeBudgets) -> Result<Self, DecodeError> {
        ffmpeg::init().map_err(DecodeError::Ffmpeg)?;
        Ok(Self {
            path: source.to_file_path().map_err(DecodeError::SourceUri)?,
            #[cfg(unix)]
            _media_io: None,
            budgets,
        })
    }

    /// Creates a decoder from transient seekable provider I/O without
    /// materializing the media into an app-private file.
    #[cfg(unix)]
    pub fn new_with_seekable_io(
        capability: SeekableMediaIoCapability,
        budgets: RuntimeBudgets,
    ) -> Result<Self, DecodeError> {
        ffmpeg::init().map_err(DecodeError::Ffmpeg)?;
        Ok(Self {
            path: PathBuf::new(),
            _media_io: Some(capability),
            budgets,
        })
    }

    fn open_input(
        &self,
        cancellation: &or_runtime::CancellationToken,
    ) -> Result<MediaInput, DecodeError> {
        #[cfg(not(unix))]
        let _ = cancellation;
        #[cfg(unix)]
        if let Some(capability) = &self._media_io {
            let (input, io) = crate::seekable_io::CapabilityIo::open(
                Arc::clone(&capability.file),
                capability.length,
                cancellation,
            )
            .map_err(|error| {
                if cancellation.is_cancelled() {
                    DecodeError::Cancelled
                } else {
                    DecodeError::Ffmpeg(error)
                }
            })?;
            return Ok(MediaInput {
                input,
                _io: Some(io),
            });
        }
        Ok(MediaInput {
            input: format::input(&self.path).map_err(DecodeError::Ffmpeg)?,
            #[cfg(unix)]
            _io: None,
        })
    }

    /// Opens one bounded demux/decode session for sequential preview requests.
    pub fn open_video_session(
        &self,
        cancellation: &or_runtime::CancellationToken,
    ) -> Result<VideoDecodeSession, DecodeError> {
        if cancellation.is_cancelled() {
            return Err(DecodeError::Cancelled);
        }
        let input = self.open_input(cancellation)?;
        let still_image = is_still_image_format(input.format().name());
        let (stream_index, time_base, context, display_rotation) = {
            let stream = input
                .streams()
                .best(ffmpeg::media::Type::Video)
                .ok_or(DecodeError::MissingVideoStream)?;
            let parameters = stream.parameters();
            let display_rotation = stream_display_rotation(&parameters)?;
            let context = codec::context::Context::from_parameters(parameters)
                .map_err(DecodeError::Ffmpeg)?;
            (
                stream.index(),
                TimestampBase::new(stream.time_base(), stream.start_time())?,
                context,
                display_rotation,
            )
        };
        let decoder = context.decoder().video().map_err(DecodeError::Ffmpeg)?;
        Ok(VideoDecodeSession {
            input,
            decoder,
            stream_index,
            time_base,
            display_rotation,
            still_image,
            budgets: self.budgets.clone(),
            scaler: None,
            previous_frame: None,
            next_frame: None,
            last_request: None,
            sent_eof: false,
            ended: false,
            metrics: VideoDecodeSessionMetrics::default(),
        })
    }

    /// Seeks and decodes the first video stream into packed RGBA software frames.
    /// A zero-duration snapshot returns the first frame at or after its time.
    pub fn decode_video(
        &self,
        snapshot: RenderSnapshot,
        queue: &SnapshotQueue<VideoFrame>,
        cancellation: &or_runtime::CancellationToken,
    ) -> Result<usize, DecodeError> {
        check_request(snapshot, queue, cancellation)?;
        let requested_range = snapshot.requested_range();
        validate_range(requested_range)?;
        let mut input = self.open_input(cancellation)?;
        let still_image = is_still_image_format(input.format().name());
        let range = if still_image {
            TimeRange::new(RationalTime::ZERO, requested_range.duration())
                .map_err(DecodeError::Time)?
        } else {
            requested_range
        };
        let (stream_index, time_base, context, display_rotation) = {
            let stream = input
                .streams()
                .best(ffmpeg::media::Type::Video)
                .ok_or(DecodeError::MissingVideoStream)?;
            let parameters = stream.parameters();
            let display_rotation = stream_display_rotation(&parameters)?;
            let context = codec::context::Context::from_parameters(parameters)
                .map_err(DecodeError::Ffmpeg)?;
            (
                stream.index(),
                TimestampBase::new(stream.time_base(), stream.start_time())?,
                context,
                display_rotation,
            )
        };
        let mut decoder = context.decoder().video().map_err(DecodeError::Ffmpeg)?;
        if !still_image {
            seek_to_range(&mut input, stream_index, range, time_base)?;
        }

        let job = DecodeJob {
            snapshot,
            range,
            queue,
            cancellation,
            budgets: &self.budgets,
        };
        let mut scaler: Option<(Pixel, u32, u32, scaling::Context)> = None;
        let mut emitted = 0;
        for packet in input.packets() {
            check_request(snapshot, job.queue, job.cancellation)?;
            let (_, packet) = packet.map_err(DecodeError::Ffmpeg)?;
            if packet.stream() != stream_index {
                continue;
            }
            decoder.send_packet(&packet).map_err(DecodeError::Ffmpeg)?;
            if drain_video(
                &mut decoder,
                &mut scaler,
                display_rotation,
                time_base,
                &job,
                &mut emitted,
            )? {
                return Ok(emitted);
            }
        }

        decoder.send_eof().map_err(DecodeError::Ffmpeg)?;
        drain_video(
            &mut decoder,
            &mut scaler,
            display_rotation,
            time_base,
            &job,
            &mut emitted,
        )?;
        Ok(emitted)
    }

    /// Decodes the source frame with the greatest presentation timestamp at or
    /// before `source_time`, falling forward to the first frame when seeking
    /// before a stream's first timestamp.
    pub fn decode_video_frame_at(
        &self,
        source_time: RationalTime,
        cancellation: &or_runtime::CancellationToken,
    ) -> Result<Option<VideoFrame>, DecodeError> {
        if cancellation.is_cancelled() {
            return Err(DecodeError::Cancelled);
        }
        let range = TimeRange::new(source_time, RationalTime::ZERO).map_err(DecodeError::Time)?;
        validate_range(range)?;
        let mut input = self.open_input(cancellation)?;
        let still_image = is_still_image_format(input.format().name());
        let (stream_index, time_base, context, display_rotation) = {
            let stream = input
                .streams()
                .best(ffmpeg::media::Type::Video)
                .ok_or(DecodeError::MissingVideoStream)?;
            let parameters = stream.parameters();
            let display_rotation = stream_display_rotation(&parameters)?;
            let context = codec::context::Context::from_parameters(parameters)
                .map_err(DecodeError::Ffmpeg)?;
            (
                stream.index(),
                TimestampBase::new(stream.time_base(), stream.start_time())?,
                context,
                display_rotation,
            )
        };
        let mut decoder = context.decoder().video().map_err(DecodeError::Ffmpeg)?;
        let presentation = VideoStreamPresentation {
            time_base,
            display_rotation,
        };
        let target = if still_image {
            RationalTime::ZERO
        } else {
            source_time
        };
        if !still_image {
            seek_to_range(&mut input, stream_index, range, time_base)?;
        }
        let mut scaler = None;
        let mut candidate = None;
        for packet in input.packets() {
            if cancellation.is_cancelled() {
                return Err(DecodeError::Cancelled);
            }
            let (_, packet) = packet.map_err(DecodeError::Ffmpeg)?;
            if packet.stream() != stream_index {
                continue;
            }
            decoder.send_packet(&packet).map_err(DecodeError::Ffmpeg)?;
            if drain_video_frame_at(
                &mut decoder,
                &mut scaler,
                presentation,
                target,
                &self.budgets,
                cancellation,
                &mut candidate,
            )? {
                return Ok(candidate);
            }
        }

        decoder.send_eof().map_err(DecodeError::Ffmpeg)?;
        drain_video_frame_at(
            &mut decoder,
            &mut scaler,
            presentation,
            target,
            &self.budgets,
            cancellation,
            &mut candidate,
        )?;
        Ok(candidate)
    }

    /// Seeks and decodes the first audio stream, converting it to bounded,
    /// interleaved stereo F32 audio at 48 kHz.
    pub fn decode_audio(
        &self,
        snapshot: RenderSnapshot,
        queue: &SnapshotQueue<AudioChunk>,
        cancellation: &or_runtime::CancellationToken,
    ) -> Result<usize, DecodeError> {
        check_request(snapshot, queue, cancellation)?;
        let range = snapshot.requested_range();
        validate_range(range)?;
        let mut input = self.open_input(cancellation)?;
        let (stream_index, time_base, context, preroll_samples) = {
            let stream = input
                .streams()
                .best(ffmpeg::media::Type::Audio)
                .ok_or(DecodeError::MissingAudioStream)?;
            let parameters = stream.parameters();
            let preroll = parameters.initial_padding().max(parameters.seek_preroll());
            let context = codec::context::Context::from_parameters(parameters)
                .map_err(DecodeError::Ffmpeg)?;
            (
                stream.index(),
                TimestampBase::new(stream.time_base(), stream.start_time())?,
                context,
                preroll,
            )
        };
        let mut decoder = context.decoder().audio().map_err(DecodeError::Ffmpeg)?;
        if decoder.rate() == 0 {
            return Err(DecodeError::InvalidAudioRate);
        }
        let mut source_layout = decoder.ch_layout().clone();
        if source_layout.mask().is_none() {
            source_layout = ChannelLayout::default_for_channels(source_layout.channels().max(1));
        }
        if source_layout.mask().is_none() {
            return Err(DecodeError::InvalidAudioFrame);
        }
        let mut resampler = resampling::Context::get2(
            decoder.format(),
            source_layout,
            decoder.rate(),
            Sample::F32(Type::Packed),
            ChannelLayout::STEREO,
            OUTPUT_AUDIO_RATE,
        )
        .map_err(DecodeError::Ffmpeg)?;
        let preroll = RationalTime::new(i64::from(preroll_samples), decoder.rate())
            .map_err(DecodeError::Time)?;
        let seek_start = range
            .start()
            .checked_sub(preroll)
            .map_err(DecodeError::Time)?;
        seek_to_time(&mut input, stream_index, seek_start, time_base)?;

        let job = DecodeJob {
            snapshot,
            range,
            queue,
            cancellation,
            budgets: &self.budgets,
        };
        let mut emitted = 0;
        let mut audio_cursor = None;
        for packet in input.packets() {
            check_request(snapshot, job.queue, job.cancellation)?;
            let (_, packet) = packet.map_err(DecodeError::Ffmpeg)?;
            if packet.stream() != stream_index {
                continue;
            }
            decoder.send_packet(&packet).map_err(DecodeError::Ffmpeg)?;
            if drain_audio(
                &mut decoder,
                &mut resampler,
                time_base,
                &job,
                &mut audio_cursor,
                &mut emitted,
            )? {
                break;
            }
        }

        if range.duration().is_zero() {
            return Ok(emitted);
        }
        decoder.send_eof().map_err(DecodeError::Ffmpeg)?;
        let _ = drain_audio(
            &mut decoder,
            &mut resampler,
            time_base,
            &job,
            &mut audio_cursor,
            &mut emitted,
        )?;
        loop {
            check_request(snapshot, queue, cancellation)?;
            if pending_output_samples(&resampler)? == 0 {
                break;
            }
            let mut output = resample_output(&resampler, 0)?;
            resampler.flush(&mut output).map_err(DecodeError::Ffmpeg)?;
            if output.samples() == 0 {
                break;
            }
            let timestamp = audio_cursor.unwrap_or(range.start());
            if emit_audio_frame(&output, timestamp, &job, &mut audio_cursor, &mut emitted)? {
                break;
            }
        }
        Ok(emitted)
    }

    /// Streams timeline-aligned source audio to mono 16 kHz PCM WAV for an
    /// isolated local transcription provider. Positive stream starts and
    /// forward timestamp gaps become silence; material timestamp overlaps fail.
    /// `writer` must be an empty stream at offset zero.
    /// Write to a private temporary file and discard it on error; output is
    /// bounded by `max_duration_ms` and four hours.
    pub fn write_transcription_wav<W: Write + Seek>(
        &self,
        writer: &mut W,
        max_duration_ms: u64,
        cancellation: &or_runtime::CancellationToken,
    ) -> Result<u64, DecodeError> {
        if cancellation.is_cancelled() {
            return Err(DecodeError::Cancelled);
        }
        if max_duration_ms == 0 || max_duration_ms > MAX_TRANSCRIPTION_AUDIO_DURATION_MS {
            return Err(DecodeError::InvalidTranscriptionDuration);
        }
        let max_sample_frames = max_duration_ms
            .checked_mul(u64::from(TRANSCRIPTION_AUDIO_RATE) / 1_000)
            .ok_or(DecodeError::TranscriptionAudioTooLarge)?;
        if max_sample_frames
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(44))
            .is_none_or(|bytes| bytes > MAX_TRANSCRIPTION_WAV_BYTES)
        {
            return Err(DecodeError::TranscriptionAudioTooLarge);
        }
        let position = writer.stream_position().map_err(DecodeError::Io)?;
        let length = writer.seek(SeekFrom::End(0)).map_err(DecodeError::Io)?;
        if position != 0 || length != 0 {
            return Err(DecodeError::InvalidTranscriptionWavDestination);
        }
        writer.seek(SeekFrom::Start(0)).map_err(DecodeError::Io)?;

        write_pcm_wav_header(writer)?;
        let mut input = self.open_input(cancellation)?;
        let (stream_index, time_base, media_start, context) = {
            let media_start = unsafe { (*input.as_ptr()).start_time };
            let media_start = if media_start == ffmpeg::ffi::AV_NOPTS_VALUE {
                RationalTime::ZERO
            } else {
                RationalTime::new(media_start, ffmpeg::ffi::AV_TIME_BASE as u32)
                    .map_err(DecodeError::Time)?
            };
            let stream = input
                .streams()
                .best(ffmpeg::media::Type::Audio)
                .ok_or(DecodeError::MissingAudioStream)?;
            (
                stream.index(),
                // Keep stream PTS relative to the media timeline. Normalizing
                // to stream.start_time would move late-starting audio to zero.
                TimestampBase::new(stream.time_base(), 0)?,
                media_start,
                codec::context::Context::from_parameters(stream.parameters())
                    .map_err(DecodeError::Ffmpeg)?,
            )
        };
        let mut decoder = context.decoder().audio().map_err(DecodeError::Ffmpeg)?;
        if decoder.rate() == 0 {
            return Err(DecodeError::InvalidAudioRate);
        }
        let source_rate = decoder.rate();
        let mut source_layout = decoder.ch_layout().clone();
        if source_layout.mask().is_none() {
            source_layout = ChannelLayout::default_for_channels(source_layout.channels().max(1));
        }
        if source_layout.mask().is_none() {
            return Err(DecodeError::InvalidAudioFrame);
        }
        let mut resampler = resampling::Context::get2(
            decoder.format(),
            source_layout,
            decoder.rate(),
            Sample::I16(Type::Packed),
            ChannelLayout::MONO,
            TRANSCRIPTION_AUDIO_RATE,
        )
        .map_err(DecodeError::Ffmpeg)?;
        let mut progress = TranscriptionAudioProgress {
            sample_frames: 0,
            max_sample_frames,
            source_end: None,
        };
        let timeline = TranscriptionAudioTimeline {
            time_base,
            media_start,
            source_rate,
        };
        for packet in input.packets() {
            if cancellation.is_cancelled() {
                return Err(DecodeError::Cancelled);
            }
            let (_, packet) = packet.map_err(DecodeError::Ffmpeg)?;
            if packet.stream() != stream_index {
                continue;
            }
            decoder.send_packet(&packet).map_err(DecodeError::Ffmpeg)?;
            drain_transcription_audio(
                &mut decoder,
                &mut resampler,
                writer,
                timeline,
                &mut progress,
                cancellation,
            )?;
        }
        decoder.send_eof().map_err(DecodeError::Ffmpeg)?;
        drain_transcription_audio(
            &mut decoder,
            &mut resampler,
            writer,
            timeline,
            &mut progress,
            cancellation,
        )?;
        flush_transcription_resampler(
            &mut resampler,
            writer,
            &mut progress.sample_frames,
            progress.max_sample_frames,
            cancellation,
        )?;
        finish_pcm_wav(writer, progress.sample_frames)?;
        Ok(progress.sample_frames)
    }
}

#[derive(Clone, Copy)]
struct TranscriptionAudioTimeline {
    time_base: TimestampBase,
    media_start: RationalTime,
    source_rate: u32,
}

struct TranscriptionAudioProgress {
    sample_frames: u64,
    max_sample_frames: u64,
    source_end: Option<RationalTime>,
}

fn drain_transcription_audio<W: Write + Seek>(
    decoder: &mut codec::decoder::Audio,
    resampler: &mut resampling::Context,
    writer: &mut W,
    timeline: TranscriptionAudioTimeline,
    progress: &mut TranscriptionAudioProgress,
    cancellation: &or_runtime::CancellationToken,
) -> Result<(), DecodeError> {
    loop {
        if cancellation.is_cancelled() {
            return Err(DecodeError::Cancelled);
        }
        let mut decoded = frame::Audio::empty();
        match decoder.receive_frame(&mut decoded) {
            Ok(()) => {
                if decoded.samples() > MAX_AUDIO_INPUT_SAMPLES {
                    return Err(DecodeError::AudioFrameTooLarge);
                }
                let timestamp = decoded
                    .timestamp()
                    .ok_or(DecodeError::MissingTimestamp)
                    .and_then(|pts| timeline.time_base.to_time(pts))?
                    .checked_sub(timeline.media_start)
                    .map_err(DecodeError::Time)?;
                if let Some(end) = progress.source_end {
                    // Container time bases may quantize adjacent packet times
                    // apart by one tick. Treat that as rounding, not a real gap.
                    let tolerance = RationalTime::from_units(1, timeline.time_base.rate)
                        .map_err(DecodeError::Time)?;
                    if transcription_timestamp_is_gap(timestamp, end, tolerance)? {
                        flush_transcription_resampler(
                            resampler,
                            writer,
                            &mut progress.sample_frames,
                            progress.max_sample_frames,
                            cancellation,
                        )?;
                        write_transcription_silence_until(
                            writer,
                            &mut progress.sample_frames,
                            timestamp,
                            progress.max_sample_frames,
                            cancellation,
                        )?;
                    }
                } else {
                    write_transcription_silence_until(
                        writer,
                        &mut progress.sample_frames,
                        timestamp,
                        progress.max_sample_frames,
                        cancellation,
                    )?;
                }
                let duration = RationalTime::new(
                    i64::try_from(decoded.samples()).map_err(|_| DecodeError::TimestampOverflow)?,
                    timeline.source_rate,
                )
                .map_err(DecodeError::Time)?;
                let frame_end = timestamp.checked_add(duration).map_err(DecodeError::Time)?;
                progress.source_end = Some(
                    progress
                        .source_end
                        .map_or(frame_end, |end| end.max(frame_end)),
                );
                if decoded.ch_layout().mask().is_none() {
                    let channels = decoded.ch_layout().channels().max(1);
                    decoded.set_ch_layout(ChannelLayout::default_for_channels(channels));
                }
                let mut output = transcription_resample_output(resampler, decoded.samples())?;
                resampler
                    .run(&decoded, &mut output)
                    .map_err(DecodeError::Ffmpeg)?;
                write_transcription_samples(
                    &output,
                    writer,
                    &mut progress.sample_frames,
                    progress.max_sample_frames,
                    cancellation,
                )?;
            }
            Err(FfmpegError::Eof) => return Ok(()),
            Err(error) if is_again(error) => return Ok(()),
            Err(error) => return Err(DecodeError::Ffmpeg(error)),
        }
    }
}

fn transcription_resample_output(
    resampler: &resampling::Context,
    input_samples: usize,
) -> Result<frame::Audio, DecodeError> {
    let input_rate = u128::from(resampler.input().rate);
    let output_rate = u128::from(TRANSCRIPTION_AUDIO_RATE);
    let converted = (input_samples as u128)
        .checked_mul(output_rate)
        .and_then(|samples| samples.checked_add(input_rate.saturating_sub(1)))
        .ok_or(DecodeError::AudioFrameTooLarge)?
        / input_rate;
    let delay = pending_output_samples_at(resampler, TRANSCRIPTION_AUDIO_RATE)? as u128;
    let capacity = converted
        .checked_add(delay)
        .and_then(|samples| samples.checked_add(32))
        .filter(|samples| samples * 2 <= MAX_TRANSCRIPTION_FRAME_BYTES)
        .and_then(|samples| usize::try_from(samples).ok())
        .ok_or(DecodeError::AudioFrameTooLarge)?;
    let mut output =
        frame::Audio::new(Sample::I16(Type::Packed), capacity, ChannelLayoutMask::MONO);
    output.set_rate(TRANSCRIPTION_AUDIO_RATE);
    Ok(output)
}

fn write_transcription_samples<W: Write + Seek>(
    output: &frame::Audio,
    writer: &mut W,
    sample_frames: &mut u64,
    max_sample_frames: u64,
    cancellation: &or_runtime::CancellationToken,
) -> Result<(), DecodeError> {
    if output.samples() == 0 {
        return Ok(());
    }
    if cancellation.is_cancelled() {
        return Err(DecodeError::Cancelled);
    }
    if output.format() != Sample::I16(Type::Packed)
        || output.ch_layout().channels() != 1
        || output.rate() != TRANSCRIPTION_AUDIO_RATE
    {
        return Err(DecodeError::InvalidAudioFrame);
    }
    let byte_len = output
        .samples()
        .checked_mul(2)
        .filter(|bytes| *bytes as u128 <= MAX_TRANSCRIPTION_FRAME_BYTES)
        .ok_or(DecodeError::AudioFrameTooLarge)?;
    let data = output.data(0);
    if data.len() < byte_len {
        return Err(DecodeError::InvalidAudioFrame);
    }
    let next_sample_frames = sample_frames
        .checked_add(output.samples() as u64)
        .filter(|frames| *frames <= max_sample_frames)
        .ok_or(DecodeError::TranscriptionAudioTooLarge)?;
    if cfg!(target_endian = "little") {
        writer
            .write_all(&data[..byte_len])
            .map_err(DecodeError::Io)?;
    } else {
        for sample in data[..byte_len].as_chunks::<2>().0 {
            writer
                .write_all(&i16::from_ne_bytes([sample[0], sample[1]]).to_le_bytes())
                .map_err(DecodeError::Io)?;
        }
    }
    *sample_frames = next_sample_frames;
    Ok(())
}

fn write_transcription_silence_until<W: Write + Seek>(
    writer: &mut W,
    sample_frames: &mut u64,
    timestamp: RationalTime,
    max_sample_frames: u64,
    cancellation: &or_runtime::CancellationToken,
) -> Result<(), DecodeError> {
    let target = transcription_sample_index_ceil(timestamp)?;
    let target = u64::try_from(target.max(0)).map_err(|_| DecodeError::TimestampOverflow)?;
    if target > *sample_frames {
        write_transcription_silence(
            writer,
            sample_frames,
            target - *sample_frames,
            max_sample_frames,
            cancellation,
        )?;
    }
    Ok(())
}

fn flush_transcription_resampler<W: Write + Seek>(
    resampler: &mut resampling::Context,
    writer: &mut W,
    sample_frames: &mut u64,
    max_sample_frames: u64,
    cancellation: &or_runtime::CancellationToken,
) -> Result<(), DecodeError> {
    while pending_output_samples_at(resampler, TRANSCRIPTION_AUDIO_RATE)? > 0 {
        if cancellation.is_cancelled() {
            return Err(DecodeError::Cancelled);
        }
        let mut output = transcription_resample_output(resampler, 0)?;
        resampler.flush(&mut output).map_err(DecodeError::Ffmpeg)?;
        if output.samples() == 0 {
            break;
        }
        write_transcription_samples(
            &output,
            writer,
            sample_frames,
            max_sample_frames,
            cancellation,
        )?;
    }
    Ok(())
}

fn transcription_sample_index_ceil(timestamp: RationalTime) -> Result<i64, DecodeError> {
    let numerator = i128::from(timestamp.numerator())
        .checked_mul(i128::from(TRANSCRIPTION_AUDIO_RATE))
        .ok_or(DecodeError::TimestampOverflow)?;
    let denominator = i128::from(timestamp.denominator());
    let mut samples = numerator.div_euclid(denominator);
    if numerator.rem_euclid(denominator) != 0 {
        samples = samples
            .checked_add(1)
            .ok_or(DecodeError::TimestampOverflow)?;
    }
    i64::try_from(samples).map_err(|_| DecodeError::TimestampOverflow)
}

fn transcription_timestamp_is_gap(
    timestamp: RationalTime,
    source_end: RationalTime,
    tolerance: RationalTime,
) -> Result<bool, DecodeError> {
    let contiguous_end = source_end
        .checked_add(tolerance)
        .map_err(DecodeError::Time)?;
    if timestamp > contiguous_end {
        return Ok(true);
    }
    let contiguous_start = source_end
        .checked_sub(tolerance)
        .map_err(DecodeError::Time)?;
    if timestamp < contiguous_start {
        return Err(DecodeError::TranscriptionTimestampOverlap);
    }
    Ok(false)
}

fn write_transcription_silence<W: Write + Seek>(
    writer: &mut W,
    sample_frames: &mut u64,
    silence_frames: u64,
    max_sample_frames: u64,
    cancellation: &or_runtime::CancellationToken,
) -> Result<(), DecodeError> {
    let next_sample_frames = sample_frames
        .checked_add(silence_frames)
        .filter(|frames| *frames <= max_sample_frames)
        .ok_or(DecodeError::TranscriptionAudioTooLarge)?;
    const SILENCE: [u8; 8192] = [0; 8192];
    let mut remaining_bytes = silence_frames
        .checked_mul(2)
        .ok_or(DecodeError::TranscriptionAudioTooLarge)?;
    while remaining_bytes > 0 {
        if cancellation.is_cancelled() {
            return Err(DecodeError::Cancelled);
        }
        let count = usize::try_from(remaining_bytes.min(SILENCE.len() as u64))
            .map_err(|_| DecodeError::TranscriptionAudioTooLarge)?;
        writer
            .write_all(&SILENCE[..count])
            .map_err(DecodeError::Io)?;
        remaining_bytes -= count as u64;
    }
    *sample_frames = next_sample_frames;
    Ok(())
}

fn pending_output_samples_at(
    resampler: &resampling::Context,
    output_rate: u32,
) -> Result<usize, DecodeError> {
    // The wrapper's delay helper loses sub-second delay; query in output samples.
    let delay =
        unsafe { ffmpeg::ffi::swr_get_delay(resampler.as_ptr() as *mut _, i64::from(output_rate)) };
    usize::try_from(delay).map_err(|_| DecodeError::AudioFrameTooLarge)
}

fn write_pcm_wav_header<W: Write>(writer: &mut W) -> Result<(), DecodeError> {
    writer
        .write_all(b"RIFF\0\0\0\0WAVEfmt ")
        .map_err(DecodeError::Io)?;
    writer
        .write_all(&16u32.to_le_bytes())
        .map_err(DecodeError::Io)?;
    writer
        .write_all(&1u16.to_le_bytes())
        .map_err(DecodeError::Io)?;
    writer
        .write_all(&1u16.to_le_bytes())
        .map_err(DecodeError::Io)?;
    writer
        .write_all(&TRANSCRIPTION_AUDIO_RATE.to_le_bytes())
        .map_err(DecodeError::Io)?;
    writer
        .write_all(&(TRANSCRIPTION_AUDIO_RATE * 2).to_le_bytes())
        .map_err(DecodeError::Io)?;
    writer
        .write_all(&2u16.to_le_bytes())
        .map_err(DecodeError::Io)?;
    writer
        .write_all(&16u16.to_le_bytes())
        .map_err(DecodeError::Io)?;
    writer.write_all(b"data\0\0\0\0").map_err(DecodeError::Io)
}

fn finish_pcm_wav<W: Write + Seek>(writer: &mut W, sample_frames: u64) -> Result<(), DecodeError> {
    let data_bytes = sample_frames
        .checked_mul(2)
        .filter(|bytes| *bytes <= u64::from(u32::MAX) - 36)
        .ok_or(DecodeError::TranscriptionAudioTooLarge)? as u32;
    writer.seek(SeekFrom::Start(4)).map_err(DecodeError::Io)?;
    writer
        .write_all(&(data_bytes + 36).to_le_bytes())
        .map_err(DecodeError::Io)?;
    writer.seek(SeekFrom::Start(40)).map_err(DecodeError::Io)?;
    writer
        .write_all(&data_bytes.to_le_bytes())
        .map_err(DecodeError::Io)?;
    writer.seek(SeekFrom::End(0)).map_err(DecodeError::Io)?;
    writer.flush().map_err(DecodeError::Io)
}

fn stream_display_rotation(
    parameters: &codec::ParametersRef<'_>,
) -> Result<DisplayRotation, DecodeError> {
    let parameters = parameters.as_ptr();
    // FFmpeg 8 stores stream-wide display transforms on AVCodecParameters.
    // Copy the small matrix while the stream-owned parameter object is borrowed.
    unsafe {
        let parameters = &*parameters;
        if parameters.nb_coded_side_data < 0 {
            return Err(DecodeError::InvalidDisplayMatrix);
        }
        let side_data = ffmpeg::ffi::av_packet_side_data_get(
            parameters.coded_side_data,
            parameters.nb_coded_side_data,
            ffmpeg::ffi::AVPacketSideDataType::DISPLAYMATRIX,
        );
        if side_data.is_null() {
            return Ok(DisplayRotation::None);
        }
        let side_data = &*side_data;
        let matrix_bytes = 9 * std::mem::size_of::<i32>();
        if side_data.data.is_null() || side_data.size < matrix_bytes {
            return Err(DecodeError::InvalidDisplayMatrix);
        }
        let matrix: [i32; 9] = std::array::from_fn(|index| {
            std::ptr::read_unaligned(
                side_data
                    .data
                    .add(index * std::mem::size_of::<i32>())
                    .cast(),
            )
        });
        DisplayRotation::from_degrees(ffmpeg::ffi::av_display_rotation_get(matrix.as_ptr()))
    }
}

/// Counters used to verify that playback reuses its demuxer and seeks only on
/// discontinuities.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VideoDecodeSessionMetrics {
    pub seeks: u64,
    pub decoded_frames: u64,
}

/// Stateful video decode for playback. It retains one preceding and one
/// lookahead frame; both count against the shared decode budget.
pub struct VideoDecodeSession {
    input: MediaInput,
    decoder: codec::decoder::Video,
    stream_index: usize,
    time_base: TimestampBase,
    display_rotation: DisplayRotation,
    still_image: bool,
    budgets: RuntimeBudgets,
    scaler: Option<(Pixel, u32, u32, scaling::Context)>,
    previous_frame: Option<VideoFrame>,
    next_frame: Option<VideoFrame>,
    last_request: Option<RationalTime>,
    sent_eof: bool,
    ended: bool,
    metrics: VideoDecodeSessionMetrics,
}

// SAFETY: FFmpeg's demux, decode, and scaling contexts are exclusively owned
// by this value and every operation requires `&mut self`. Moving those native
// contexts between threads does not expose concurrent access; the custom AVIO
// cursor is heap-owned and only called while this input context is mutably used.
unsafe impl Send for VideoDecodeSession {}

impl VideoDecodeSession {
    pub fn metrics(&self) -> VideoDecodeSessionMetrics {
        self.metrics
    }

    pub fn frame_at(
        &mut self,
        target: RationalTime,
        cancellation: &or_runtime::CancellationToken,
    ) -> Result<Option<VideoFrame>, DecodeError> {
        if cancellation.is_cancelled() {
            return Err(DecodeError::Cancelled);
        }
        let discontinuity = self.last_request.is_some_and(|previous| target < previous);
        if !self.still_image && (self.last_request.is_none() || discontinuity) {
            self.seek(target)?;
        }
        self.last_request = Some(target);

        while self
            .next_frame
            .as_ref()
            .is_some_and(|frame| frame.descriptor().timing().timestamp() <= target)
        {
            self.previous_frame = self.next_frame.take();
        }

        loop {
            if self
                .next_frame
                .as_ref()
                .is_some_and(|frame| frame.descriptor().timing().timestamp() > target)
                || self.ended
            {
                break;
            }
            if cancellation.is_cancelled() {
                return Err(DecodeError::Cancelled);
            }
            let mut decoded = VideoFrameBuffer::empty();
            match self.decoder.receive_frame(&mut decoded) {
                Ok(()) => {
                    let timestamp = decoded
                        .timestamp()
                        .ok_or(DecodeError::MissingTimestamp)
                        .and_then(|pts| self.time_base.to_time(pts))?;
                    let frame = make_video_frame(
                        &decoded,
                        timestamp,
                        self.display_rotation,
                        &mut self.scaler,
                        &self.budgets,
                    )?;
                    self.metrics.decoded_frames = self.metrics.decoded_frames.saturating_add(1);
                    if timestamp <= target {
                        self.previous_frame = Some(frame);
                    } else {
                        self.next_frame = Some(frame);
                    }
                }
                Err(FfmpegError::Eof) => self.ended = true,
                Err(error) if is_again(error) => self.read_packet(cancellation)?,
                Err(error) => return Err(DecodeError::Ffmpeg(error)),
            }
        }

        self.previous_frame
            .as_ref()
            .or(self.next_frame.as_ref())
            .map(|frame| frame.duplicate(&self.budgets))
            .transpose()
    }

    fn seek(&mut self, target: RationalTime) -> Result<(), DecodeError> {
        let range = TimeRange::new(target, RationalTime::ZERO).map_err(DecodeError::Time)?;
        seek_to_range(&mut self.input, self.stream_index, range, self.time_base)?;
        self.decoder.flush();
        self.scaler = None;
        self.previous_frame = None;
        self.next_frame = None;
        self.sent_eof = false;
        self.ended = false;
        self.metrics.seeks = self.metrics.seeks.saturating_add(1);
        Ok(())
    }

    fn read_packet(
        &mut self,
        cancellation: &or_runtime::CancellationToken,
    ) -> Result<(), DecodeError> {
        loop {
            if cancellation.is_cancelled() {
                return Err(DecodeError::Cancelled);
            }
            if self.sent_eof {
                self.ended = true;
                return Ok(());
            }
            let mut packet = ffmpeg::Packet::empty();
            match packet.read(&mut self.input) {
                Ok(()) if packet.stream() == self.stream_index => {
                    self.decoder
                        .send_packet(&packet)
                        .map_err(DecodeError::Ffmpeg)?;
                    return Ok(());
                }
                Ok(()) => continue,
                Err(FfmpegError::Eof) => {
                    self.decoder.send_eof().map_err(DecodeError::Ffmpeg)?;
                    self.sent_eof = true;
                    return Ok(());
                }
                Err(error) if is_again(error) => continue,
                Err(error) => return Err(DecodeError::Ffmpeg(error)),
            }
        }
    }
}

fn check_request<T>(
    snapshot: RenderSnapshot,
    queue: &SnapshotQueue<T>,
    cancellation: &or_runtime::CancellationToken,
) -> Result<(), DecodeError> {
    if cancellation.is_cancelled() {
        return Err(DecodeError::Cancelled);
    }
    if !queue.is_current(snapshot) {
        return Err(DecodeError::StaleSnapshot);
    }
    Ok(())
}

fn seek_to_range(
    input: &mut format::context::Input,
    stream_index: usize,
    range: TimeRange,
    time_base: TimestampBase,
) -> Result<(), DecodeError> {
    let relative_start = if range.start().is_negative() {
        RationalTime::ZERO
    } else {
        range.start()
    };
    seek_to_time(input, stream_index, relative_start, time_base)
}

fn seek_to_time(
    input: &mut format::context::Input,
    stream_index: usize,
    time: RationalTime,
    time_base: TimestampBase,
) -> Result<(), DecodeError> {
    let seek_pts = time_base
        .origin_pts
        .checked_add(time_to_rate_units_floor(time, time_base.rate)?)
        .ok_or(DecodeError::TimestampOverflow)?;
    let stream_index = i32::try_from(stream_index).map_err(|_| DecodeError::TimestampOverflow)?;
    // The wrapper's seek method only targets the global time base. Stream PTS
    // seeking is needed to select the preceding audio packet/keyframe exactly.
    let result = unsafe {
        ffmpeg::ffi::av_seek_frame(
            input.as_mut_ptr(),
            stream_index,
            seek_pts,
            ffmpeg::ffi::AVSEEK_FLAG_BACKWARD,
        )
    };
    if result < 0 {
        Err(DecodeError::Ffmpeg(FfmpegError::from(result)))
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct TimestampBase {
    rate: RationalRate,
    origin_pts: i64,
}

impl TimestampBase {
    fn new(time_base: ffmpeg::Rational, origin_pts: i64) -> Result<Self, DecodeError> {
        let numerator = time_base.numerator();
        let denominator = time_base.denominator();
        if numerator <= 0 || denominator <= 0 {
            return Err(DecodeError::InvalidTimeBase);
        }
        let rate = RationalRate::new(
            u32::try_from(denominator).map_err(|_| DecodeError::InvalidTimeBase)?,
            u32::try_from(numerator).map_err(|_| DecodeError::InvalidTimeBase)?,
        )
        .map_err(DecodeError::Time)?;
        Ok(Self {
            rate,
            origin_pts: if origin_pts == ffmpeg::ffi::AV_NOPTS_VALUE {
                0
            } else {
                origin_pts
            },
        })
    }

    fn to_time(self, pts: i64) -> Result<RationalTime, DecodeError> {
        let relative_pts = pts
            .checked_sub(self.origin_pts)
            .ok_or(DecodeError::TimestampOverflow)?;
        RationalTime::from_units(relative_pts, self.rate).map_err(|source| {
            DecodeError::TimestampConversion {
                pts,
                origin_pts: self.origin_pts,
                units_per_second: self.rate,
                source,
            }
        })
    }
}

fn time_to_rate_units_floor(time: RationalTime, rate: RationalRate) -> Result<i64, DecodeError> {
    let numerator = i128::from(time.numerator()) * i128::from(rate.numerator());
    let denominator = i128::from(time.denominator()) * i128::from(rate.denominator());
    let units = numerator.div_euclid(denominator);
    i64::try_from(units).map_err(|_| DecodeError::TimestampOverflow)
}

struct DecodeJob<'a, T> {
    snapshot: RenderSnapshot,
    range: TimeRange,
    queue: &'a SnapshotQueue<T>,
    cancellation: &'a or_runtime::CancellationToken,
    budgets: &'a RuntimeBudgets,
}

fn drain_video(
    decoder: &mut codec::decoder::Video,
    scaler: &mut Option<(Pixel, u32, u32, scaling::Context)>,
    display_rotation: DisplayRotation,
    time_base: TimestampBase,
    job: &DecodeJob<'_, VideoFrame>,
    emitted: &mut usize,
) -> Result<bool, DecodeError> {
    loop {
        check_request(job.snapshot, job.queue, job.cancellation)?;
        let mut decoded = VideoFrameBuffer::empty();
        match decoder.receive_frame(&mut decoded) {
            Ok(()) => {
                let timestamp = decoded
                    .timestamp()
                    .ok_or(DecodeError::MissingTimestamp)
                    .and_then(|pts| time_base.to_time(pts))?;
                if past_range(timestamp, job.range) {
                    return Ok(true);
                }
                if timestamp < job.range.start() {
                    continue;
                }
                let done = emit_video_frame(&decoded, timestamp, display_rotation, scaler, job)?;
                *emitted += 1;
                if done {
                    return Ok(true);
                }
            }
            Err(FfmpegError::Eof) => return Ok(true),
            Err(error) if is_again(error) => return Ok(false),
            Err(error) => return Err(DecodeError::Ffmpeg(error)),
        }
    }
}

fn emit_video_frame(
    decoded: &VideoFrameBuffer,
    timestamp: RationalTime,
    display_rotation: DisplayRotation,
    scaler: &mut Option<(Pixel, u32, u32, scaling::Context)>,
    job: &DecodeJob<'_, VideoFrame>,
) -> Result<bool, DecodeError> {
    let frame = make_video_frame(decoded, timestamp, display_rotation, scaler, job.budgets)?;
    queue_video(job.snapshot, frame, job.queue, job.cancellation)?;
    Ok(job.range.duration().is_zero())
}

fn make_video_frame(
    decoded: &VideoFrameBuffer,
    timestamp: RationalTime,
    display_rotation: DisplayRotation,
    scaler: &mut Option<(Pixel, u32, u32, scaling::Context)>,
    budgets: &RuntimeBudgets,
) -> Result<VideoFrame, DecodeError> {
    let (width, height, input_format) = (decoded.width(), decoded.height(), decoded.format());
    let packed_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|bytes| *bytes <= MAX_SOFTWARE_FRAME_BYTES)
        .ok_or(DecodeError::VideoFrameTooLarge)?;
    let budget = budgets
        .decode()
        .try_acquire(packed_len as u64)
        .map_err(DecodeError::Budget)?;
    if !matches!(scaler, Some((format, w, h, _)) if *format == input_format && *w == width && *h == height)
    {
        *scaler = Some((
            input_format,
            width,
            height,
            scaling::Context::get(
                input_format,
                width,
                height,
                Pixel::RGBA,
                width,
                height,
                scaling::Flags::BILINEAR,
            )
            .map_err(DecodeError::Ffmpeg)?,
        ));
    }

    let Some((_, _, _, scaler)) = scaler.as_mut() else {
        return Err(DecodeError::InvalidFrameData);
    };
    let mut rgba = VideoFrameBuffer::empty();
    scaler
        .run(decoded, &mut rgba)
        .map_err(DecodeError::Ffmpeg)?;
    let row_bytes = (width as usize)
        .checked_mul(4)
        .ok_or(DecodeError::VideoFrameTooLarge)?;
    let stride = rgba.stride(0);
    let plane = rgba.data(0);
    if stride < row_bytes {
        return Err(DecodeError::InvalidFrameData);
    }
    let (display_width, display_height) = display_rotation.output_size(width, height);
    let mut pixels = vec![0; packed_len];
    for row in 0..height as usize {
        let start = row
            .checked_mul(stride)
            .ok_or(DecodeError::InvalidFrameData)?;
        let end = start
            .checked_add(row_bytes)
            .ok_or(DecodeError::InvalidFrameData)?;
        let source_row = plane.get(start..end).ok_or(DecodeError::InvalidFrameData)?;
        for column in 0..width as usize {
            let (display_x, display_y) =
                display_rotation.output_coordinate(width as usize, height as usize, column, row);
            let source_offset = column * 4;
            let destination_offset = (display_y * display_width as usize + display_x) * 4;
            pixels[destination_offset..destination_offset + 4]
                .copy_from_slice(&source_row[source_offset..source_offset + 4]);
        }
    }

    let descriptor = FrameDescriptor::software(
        display_width,
        display_height,
        FramePixelFormat::Rgba8,
        timestamp,
    )?;
    let lease = FrameLease::from_software(descriptor, pixels)?;
    Ok(VideoFrame {
        lease,
        _budget: budget,
    })
}

fn drain_video_frame_at(
    decoder: &mut codec::decoder::Video,
    scaler: &mut Option<(Pixel, u32, u32, scaling::Context)>,
    presentation: VideoStreamPresentation,
    target: RationalTime,
    budgets: &RuntimeBudgets,
    cancellation: &or_runtime::CancellationToken,
    candidate: &mut Option<VideoFrame>,
) -> Result<bool, DecodeError> {
    loop {
        if cancellation.is_cancelled() {
            return Err(DecodeError::Cancelled);
        }
        let mut decoded = VideoFrameBuffer::empty();
        match decoder.receive_frame(&mut decoded) {
            Ok(()) => {
                let timestamp = decoded
                    .timestamp()
                    .ok_or(DecodeError::MissingTimestamp)
                    .and_then(|pts| presentation.time_base.to_time(pts))?;
                if timestamp > target {
                    if candidate.is_none() {
                        *candidate = Some(make_video_frame(
                            &decoded,
                            timestamp,
                            presentation.display_rotation,
                            scaler,
                            budgets,
                        )?);
                    }
                    return Ok(true);
                }
                candidate.take();
                *candidate = Some(make_video_frame(
                    &decoded,
                    timestamp,
                    presentation.display_rotation,
                    scaler,
                    budgets,
                )?);
            }
            Err(FfmpegError::Eof) => return Ok(true),
            Err(error) if is_again(error) => return Ok(false),
            Err(error) => return Err(DecodeError::Ffmpeg(error)),
        }
    }
}

fn drain_audio(
    decoder: &mut codec::decoder::Audio,
    resampler: &mut resampling::Context,
    time_base: TimestampBase,
    job: &DecodeJob<'_, AudioChunk>,
    audio_cursor: &mut Option<RationalTime>,
    emitted: &mut usize,
) -> Result<bool, DecodeError> {
    loop {
        check_request(job.snapshot, job.queue, job.cancellation)?;
        let mut decoded = frame::Audio::empty();
        match decoder.receive_frame(&mut decoded) {
            Ok(()) => {
                if decoded.samples() > MAX_AUDIO_INPUT_SAMPLES {
                    return Err(DecodeError::AudioFrameTooLarge);
                }
                let timestamp = decoded
                    .timestamp()
                    .ok_or(DecodeError::MissingTimestamp)
                    .and_then(|pts| time_base.to_time(pts))?;
                if past_range(timestamp, job.range) {
                    return Ok(true);
                }
                if decoded.ch_layout().mask().is_none() {
                    let channels = decoded.ch_layout().channels().max(1);
                    decoded.set_ch_layout(ChannelLayout::default_for_channels(channels));
                }
                let mut output = resample_output(resampler, decoded.samples())?;
                resampler
                    .run(&decoded, &mut output)
                    .map_err(DecodeError::Ffmpeg)?;
                if emit_audio_frame(
                    &output,
                    audio_cursor.unwrap_or(timestamp),
                    job,
                    audio_cursor,
                    emitted,
                )? {
                    return Ok(true);
                }
            }
            Err(FfmpegError::Eof) => return Ok(true),
            Err(error) if is_again(error) => return Ok(false),
            Err(error) => return Err(DecodeError::Ffmpeg(error)),
        }
    }
}

fn emit_audio_frame(
    output: &frame::Audio,
    timestamp: RationalTime,
    job: &DecodeJob<'_, AudioChunk>,
    audio_cursor: &mut Option<RationalTime>,
    emitted: &mut usize,
) -> Result<bool, DecodeError> {
    if output.samples() == 0 {
        return Ok(false);
    }
    let byte_len = output
        .samples()
        .checked_mul(OUTPUT_AUDIO_CHANNELS)
        .and_then(|samples| samples.checked_mul(std::mem::size_of::<f32>()))
        .filter(|bytes| *bytes <= MAX_AUDIO_CHUNK_BYTES)
        .ok_or(DecodeError::AudioFrameTooLarge)?;
    let budget = job
        .budgets
        .audio()
        .try_acquire(byte_len as u64)
        .map_err(DecodeError::Budget)?;
    if output.format() != Sample::F32(Type::Packed)
        || output.ch_layout().channels() as usize != OUTPUT_AUDIO_CHANNELS
    {
        return Err(DecodeError::InvalidAudioFrame);
    }
    let data = output.data(0);
    if data.len() < byte_len {
        return Err(DecodeError::InvalidAudioFrame);
    }
    let mut samples = Vec::with_capacity(byte_len / std::mem::size_of::<f32>());
    for bytes in data[..byte_len]
        .as_chunks::<{ std::mem::size_of::<f32>() }>()
        .0
    {
        samples.push(f32::from_ne_bytes(*bytes));
    }
    let Some((timestamp, samples)) = clip_audio(timestamp, job.range, samples)? else {
        return Ok(past_range(timestamp, job.range));
    };
    let sample_frames = samples.len() / OUTPUT_AUDIO_CHANNELS;
    let frame = AudioChunk {
        timestamp,
        samples,
        _budget: budget,
    };
    queue_audio(job.snapshot, frame, job.queue, job.cancellation)?;
    let sample_offset = sample_offset_time(sample_frames)?;
    *audio_cursor = Some(audio_time_arithmetic(
        "next audio chunk cursor",
        timestamp,
        sample_offset,
        timestamp.checked_add(sample_offset),
    )?);
    *emitted += 1;
    Ok(job.range.duration().is_zero())
}

fn resample_output(
    resampler: &resampling::Context,
    input_samples: usize,
) -> Result<frame::Audio, DecodeError> {
    let input_rate = u128::from(resampler.input().rate);
    let output_rate = u128::from(resampler.output().rate);
    let converted = (input_samples as u128)
        .checked_mul(output_rate)
        .and_then(|samples| samples.checked_add(input_rate.saturating_sub(1)))
        .ok_or(DecodeError::AudioFrameTooLarge)?
        / input_rate;
    let delay = pending_output_samples(resampler)? as u128;
    let capacity = converted
        .checked_add(delay)
        .and_then(|samples| samples.checked_add(32))
        .filter(|samples| {
            *samples * OUTPUT_AUDIO_CHANNELS as u128 * std::mem::size_of::<f32>() as u128
                <= MAX_AUDIO_CHUNK_BYTES as u128
        })
        .and_then(|samples| usize::try_from(samples).ok())
        .ok_or(DecodeError::AudioFrameTooLarge)?;
    let mut output = frame::Audio::new(
        Sample::F32(Type::Packed),
        capacity,
        ChannelLayoutMask::STEREO,
    );
    output.set_rate(OUTPUT_AUDIO_RATE);
    Ok(output)
}

fn pending_output_samples(resampler: &resampling::Context) -> Result<usize, DecodeError> {
    pending_output_samples_at(resampler, OUTPUT_AUDIO_RATE)
}

fn clip_audio(
    timestamp: RationalTime,
    range: TimeRange,
    mut samples: Vec<f32>,
) -> Result<Option<(RationalTime, Vec<f32>)>, DecodeError> {
    let source_frames = samples.len() / OUTPUT_AUDIO_CHANNELS;
    if source_frames == 0 {
        return Ok(None);
    }
    let start = range.start();
    let end = start
        .checked_add(range.duration())
        .map_err(DecodeError::Time)?;
    if !range.duration().is_zero() && timestamp >= end {
        return Ok(None);
    }

    let start_frame = if timestamp < start {
        audio_sample_offset(start, timestamp, true)?
    } else {
        0
    }
    .min(source_frames);
    let end_frame = if range.duration().is_zero() {
        start_frame.saturating_add(1).min(source_frames)
    } else if timestamp < end {
        audio_sample_offset(end, timestamp, false)?.min(source_frames)
    } else {
        0
    };
    if end_frame <= start_frame {
        return Ok(None);
    }
    let sample_offset = sample_offset_time(start_frame)?;
    let timestamp = match timestamp.checked_add(sample_offset) {
        Ok(timestamp) => timestamp,
        Err(TimeError::ArithmeticOverflow) => {
            let timestamp_frame = audio_sample_frame_ceil(timestamp)?
                .checked_add(
                    i64::try_from(start_frame).map_err(|_| DecodeError::TimestampOverflow)?,
                )
                .ok_or(DecodeError::TimestampOverflow)?;
            RationalTime::new(timestamp_frame, OUTPUT_AUDIO_RATE).map_err(DecodeError::Time)?
        }
        Err(error) => return Err(DecodeError::Time(error)),
    };
    samples.drain(..start_frame * OUTPUT_AUDIO_CHANNELS);
    samples.truncate((end_frame - start_frame) * OUTPUT_AUDIO_CHANNELS);
    Ok(Some((timestamp, samples)))
}

fn sample_offset_time(samples: usize) -> Result<RationalTime, DecodeError> {
    let samples = i64::try_from(samples).map_err(|_| DecodeError::TimestampOverflow)?;
    let rate = RationalRate::new(OUTPUT_AUDIO_RATE, 1).map_err(DecodeError::Time)?;
    RationalTime::from_units(samples, rate).map_err(DecodeError::Time)
}

fn audio_time_arithmetic(
    operation: &'static str,
    left: RationalTime,
    right: RationalTime,
    result: Result<RationalTime, TimeError>,
) -> Result<RationalTime, DecodeError> {
    result.map_err(|source| DecodeError::AudioTimeArithmetic {
        operation,
        left,
        right,
        source,
    })
}

fn audio_sample_offset(
    later: RationalTime,
    earlier: RationalTime,
    round_up: bool,
) -> Result<usize, DecodeError> {
    let numerator = (i128::from(later.numerator()) * i128::from(earlier.denominator())
        - i128::from(earlier.numerator()) * i128::from(later.denominator()))
        * i128::from(OUTPUT_AUDIO_RATE);
    if numerator <= 0 {
        return Ok(0);
    }
    let denominator = i128::from(later.denominator()) * i128::from(earlier.denominator());
    let mut samples = numerator.div_euclid(denominator);
    if round_up && numerator.rem_euclid(denominator) != 0 {
        samples += 1;
    }
    usize::try_from(samples).map_err(|_| DecodeError::TimestampOverflow)
}

fn audio_sample_frame_ceil(time: RationalTime) -> Result<i64, DecodeError> {
    let numerator = i128::from(time.numerator()) * i128::from(OUTPUT_AUDIO_RATE);
    let denominator = i128::from(time.denominator());
    let mut frames = numerator.div_euclid(denominator);
    if numerator.rem_euclid(denominator) != 0 {
        frames += 1;
    }
    i64::try_from(frames).map_err(|_| DecodeError::TimestampOverflow)
}

fn past_range(timestamp: RationalTime, range: TimeRange) -> bool {
    !range.duration().is_zero()
        && range
            .start()
            .checked_add(range.duration())
            .is_ok_and(|end| timestamp >= end)
}

fn validate_range(range: TimeRange) -> Result<(), DecodeError> {
    range
        .start()
        .checked_add(range.duration())
        .map_err(DecodeError::Time)?;
    Ok(())
}

fn queue_video(
    snapshot: RenderSnapshot,
    frame: VideoFrame,
    queue: &SnapshotQueue<VideoFrame>,
    cancellation: &or_runtime::CancellationToken,
) -> Result<(), DecodeError> {
    queue
        .push(snapshot, frame, cancellation)
        .map_err(map_snapshot_send_error)
}

fn queue_audio(
    snapshot: RenderSnapshot,
    frame: AudioChunk,
    queue: &SnapshotQueue<AudioChunk>,
    cancellation: &or_runtime::CancellationToken,
) -> Result<(), DecodeError> {
    queue
        .push(snapshot, frame, cancellation)
        .map_err(map_snapshot_send_error)
}

fn map_snapshot_send_error<T>(error: SnapshotQueueSendError<T>) -> DecodeError {
    match error {
        SnapshotQueueSendError::StaleSnapshot(_) => DecodeError::StaleSnapshot,
        SnapshotQueueSendError::Cancelled(_) => DecodeError::Cancelled,
        SnapshotQueueSendError::Closed(_) | SnapshotQueueSendError::Backpressure(_) => {
            DecodeError::QueueClosed
        }
    }
}

fn is_again(error: FfmpegError) -> bool {
    let FfmpegError::Other { errno } = error else {
        return false;
    };

    #[cfg(windows)]
    {
        // FFmpeg stores POSIX errno, while std::io expects a Win32 error code.
        errno == 11
    }
    #[cfg(not(windows))]
    {
        io::Error::from_raw_os_error(errno).kind() == ErrorKind::WouldBlock
    }
}

#[derive(Debug)]
pub enum DecodeError {
    SourceUri(MediaSourceUriError),
    Ffmpeg(FfmpegError),
    Io(io::Error),
    Time(TimeError),
    Budget(BudgetAcquireError),
    FrameDescriptor(FrameDescriptorError),
    FrameLease(FrameLeaseError),
    MissingVideoStream,
    MissingAudioStream,
    MissingTimestamp,
    InvalidTimeBase,
    AudioTimeArithmetic {
        operation: &'static str,
        left: RationalTime,
        right: RationalTime,
        source: TimeError,
    },
    TimestampConversion {
        pts: i64,
        origin_pts: i64,
        units_per_second: RationalRate,
        source: TimeError,
    },
    InvalidAudioRate,
    InvalidAudioFrame,
    InvalidFrameData,
    InvalidDisplayMatrix,
    UnsupportedDisplayRotation(i32),
    VideoFrameTooLarge,
    AudioFrameTooLarge,
    InvalidTranscriptionDuration,
    InvalidTranscriptionWavDestination,
    TranscriptionAudioTooLarge,
    TranscriptionTimestampOverlap,
    TimestampOverflow,
    Cancelled,
    StaleSnapshot,
    QueueClosed,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceUri(error) => write!(formatter, "invalid media source: {error}"),
            Self::Ffmpeg(error) => write!(formatter, "FFmpeg operation failed: {error}"),
            Self::Io(error) => write!(formatter, "audio output failed: {error}"),
            Self::Time(error) => write!(formatter, "invalid media timestamp: {error}"),
            Self::Budget(error) => write!(formatter, "runtime media budget exceeded: {error}"),
            Self::FrameDescriptor(error) => write!(formatter, "invalid decoded frame: {error}"),
            Self::FrameLease(error) => write!(formatter, "invalid decoded frame storage: {error}"),
            Self::MissingVideoStream => formatter.write_str("media source has no video stream"),
            Self::MissingAudioStream => formatter.write_str("media source has no audio stream"),
            Self::MissingTimestamp => {
                formatter.write_str("decoded frame has no presentation timestamp")
            }
            Self::InvalidTimeBase => formatter.write_str("media stream has an invalid time base"),
            Self::AudioTimeArithmetic {
                operation,
                left,
                right,
                source,
            } => write!(
                formatter,
                "audio timestamp arithmetic failed during {operation} (left={left:?}, right={right:?}): {source}"
            ),
            Self::TimestampConversion {
                pts,
                origin_pts,
                units_per_second,
                source,
            } => write!(
                formatter,
                "media timestamp conversion failed (pts={pts}, origin_pts={origin_pts}, time_base_rate={}/{}): {source}",
                units_per_second.numerator(),
                units_per_second.denominator()
            ),
            Self::InvalidAudioRate => {
                formatter.write_str("audio stream has an invalid sample rate")
            }
            Self::InvalidAudioFrame => {
                formatter.write_str("decoded audio frame has an unsupported layout")
            }
            Self::InvalidFrameData => formatter.write_str("decoded video frame data is incomplete"),
            Self::InvalidDisplayMatrix => {
                formatter.write_str("video stream has an invalid display matrix")
            }
            Self::UnsupportedDisplayRotation(degrees) => {
                write!(
                    formatter,
                    "video stream has unsupported display rotation: {degrees} degrees"
                )
            }
            Self::VideoFrameTooLarge => {
                formatter.write_str("decoded video frame exceeds the configured bound")
            }
            Self::AudioFrameTooLarge => {
                formatter.write_str("decoded audio frame exceeds the configured bound")
            }
            Self::InvalidTranscriptionDuration => {
                formatter.write_str("transcription audio duration must be within four hours")
            }
            Self::InvalidTranscriptionWavDestination => {
                formatter.write_str("transcription WAV output must be an empty stream")
            }
            Self::TranscriptionAudioTooLarge => {
                formatter.write_str("transcription audio exceeds the configured bound")
            }
            Self::TranscriptionTimestampOverlap => {
                formatter.write_str("transcription audio contains overlapping frame timestamps")
            }
            Self::TimestampOverflow => formatter.write_str("media timestamp conversion overflowed"),
            Self::Cancelled => formatter.write_str("media decode was cancelled"),
            Self::StaleSnapshot => formatter.write_str("media decode request is stale"),
            Self::QueueClosed => formatter.write_str("media output queue is closed"),
        }
    }
}

impl Error for DecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceUri(error) => Some(error),
            Self::Ffmpeg(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Time(error) => Some(error),
            Self::Budget(error) => Some(error),
            Self::FrameDescriptor(error) => Some(error),
            Self::FrameLease(error) => Some(error),
            Self::AudioTimeArithmetic { source, .. } => Some(source),
            Self::TimestampConversion { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<FrameDescriptorError> for DecodeError {
    fn from(error: FrameDescriptorError) -> Self {
        Self::FrameDescriptor(error)
    }
}

impl From<FrameLeaseError> for DecodeError {
    fn from(error: FrameLeaseError) -> Self {
        Self::FrameLease(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct CancelOnWrite {
        inner: Cursor<Vec<u8>>,
        cancellation: or_runtime::CancellationToken,
    }

    impl Write for CancelOnWrite {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let written = self.inner.write(bytes)?;
            self.cancellation.cancel();
            Ok(written)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.inner.flush()
        }
    }

    impl Seek for CancelOnWrite {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            self.inner.seek(position)
        }
    }

    fn time(numerator: i64, denominator: u32) -> RationalTime {
        RationalTime::new(numerator, denominator).unwrap()
    }

    #[test]
    fn ffmpeg_timebase_and_output_sample_offsets_stay_rational() {
        let base = TimestampBase::new(ffmpeg::Rational::new(1, 24_000), 0).unwrap();
        assert_eq!(base.to_time(1001).unwrap(), time(1001, 24_000));
        assert_eq!(
            audio_sample_offset(time(1, 44_100), RationalTime::ZERO, true).unwrap(),
            2
        );
        assert_eq!(
            audio_sample_offset(time(1, 44_100), RationalTime::ZERO, false).unwrap(),
            1
        );
        assert_eq!(sample_offset_time(48_000).unwrap(), time(1, 1));
    }

    #[test]
    fn transcription_timing_distinguishes_quantization_gaps_and_overlaps() {
        let end = time(1, 2);
        let tick = time(1, 1_000);
        assert!(!transcription_timestamp_is_gap(time(501, 1_000), end, tick).unwrap());
        assert!(transcription_timestamp_is_gap(time(503, 1_000), end, tick).unwrap());
        assert!(matches!(
            transcription_timestamp_is_gap(time(497, 1_000), end, tick),
            Err(DecodeError::TranscriptionTimestampOverlap)
        ));
    }

    #[test]
    fn transcription_silence_padding_checks_cancellation_between_writes() {
        let cancellation = or_runtime::CancellationToken::new();
        let mut writer = CancelOnWrite {
            inner: Cursor::new(Vec::new()),
            cancellation: cancellation.clone(),
        };
        let mut sample_frames = 0;

        assert!(matches!(
            write_transcription_silence(
                &mut writer,
                &mut sample_frames,
                5_000,
                10_000,
                &cancellation,
            ),
            Err(DecodeError::Cancelled)
        ));
    }

    #[test]
    fn invalid_stream_timestamp_reports_the_pts_and_time_base() {
        let base = TimestampBase {
            rate: RationalRate::new(1, 2).unwrap(),
            origin_pts: 17,
        };
        let error = base.to_time(i64::MAX).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("pts=9223372036854775807"));
        assert!(message.contains("origin_pts=17"));
        assert!(message.contains("time_base_rate=1/2"));
        assert!(message.contains("outside the supported range"));
    }

    #[test]
    fn invalid_audio_time_arithmetic_reports_the_operation_and_operands() {
        let left = time(i64::MAX, 1);
        let right = time(1, 1);
        let error =
            audio_time_arithmetic("test cursor", left, right, left.checked_add(right)).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("test cursor"));
        assert!(message.contains("left=RationalTime"));
        assert!(message.contains("right=RationalTime"));
        assert!(message.contains("outside the supported range"));
    }

    #[test]
    fn audio_clipping_uses_sample_offsets_without_materializing_time_differences() {
        let clip_start = time(741, 500_000_000);
        let decoded_timestamp = time(-221, 8_820);
        let offset = audio_sample_offset(clip_start, decoded_timestamp, true).unwrap();
        let first_output_frame = audio_sample_frame_ceil(decoded_timestamp)
            .unwrap()
            .checked_add(i64::try_from(offset).unwrap())
            .unwrap();

        assert_eq!(offset, 1_203);
        assert_eq!(
            RationalTime::new(first_output_frame, OUTPUT_AUDIO_RATE).unwrap(),
            time(1, 48_000)
        );
    }

    #[test]
    fn audio_clipping_handles_mp3_preroll_against_a_fine_grained_range() {
        let clip_start = time(741, 500_000_000);
        let decoded_timestamp = time(-221, 8_820);
        let range = TimeRange::new(clip_start, time(1, 8)).unwrap();
        let samples = vec![0.0; 16_000 * OUTPUT_AUDIO_CHANNELS];
        let (timestamp, samples) = clip_audio(decoded_timestamp, range, samples)
            .unwrap()
            .unwrap();

        assert!(timestamp >= clip_start);
        assert!(timestamp < clip_start.checked_add(time(1, 48_000)).unwrap());
        assert!(samples.len() / OUTPUT_AUDIO_CHANNELS <= 6_000);
    }

    #[test]
    fn ffmpeg_eagain_is_a_normal_decoder_drain_result() {
        // FFmpeg exposes POSIX errno: Darwin uses 35; Windows/Linux use 11.
        let eagain_errno = if cfg!(target_os = "macos") { 35 } else { 11 };
        assert!(is_again(FfmpegError::Other {
            errno: eagain_errno,
        }));
    }

    #[test]
    fn display_rotation_maps_pixels_in_the_expected_direction() {
        let source = *b"ABCDEF";
        let rotated = |rotation: DisplayRotation| {
            let (width, height) = rotation.output_size(2, 3);
            let mut output = vec![0; (width * height) as usize];
            for row in 0..3 {
                for column in 0..2 {
                    let (x, y) = rotation.output_coordinate(2, 3, column, row);
                    output[(y * width as usize) + x] = source[row * 2 + column];
                }
            }
            output
        };

        assert_eq!(rotated(DisplayRotation::Clockwise90), b"ECAFDB");
        assert_eq!(rotated(DisplayRotation::CounterClockwise90), b"BDFACE");
        assert_eq!(rotated(DisplayRotation::HalfTurn), b"FEDCBA");
    }

    #[test]
    fn display_matrix_angles_are_normalized_and_reject_arbitrary_rotation() {
        assert_eq!(
            DisplayRotation::from_degrees(-90.0).unwrap(),
            DisplayRotation::Clockwise90
        );
        assert_eq!(
            DisplayRotation::from_degrees(180.0).unwrap(),
            DisplayRotation::HalfTurn
        );
        assert!(matches!(
            DisplayRotation::from_degrees(45.0),
            Err(DecodeError::UnsupportedDisplayRotation(45))
        ));
    }
}
