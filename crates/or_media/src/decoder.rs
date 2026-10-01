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
use std::io::{self, ErrorKind};
use std::{error::Error, fmt, path::PathBuf};

const OUTPUT_AUDIO_RATE: u32 = 48_000;
const OUTPUT_AUDIO_CHANNELS: usize = 2;
const MAX_SOFTWARE_FRAME_BYTES: usize = 256 * 1024 * 1024;
const MAX_AUDIO_INPUT_SAMPLES: usize = 1_048_576;
const MAX_AUDIO_CHUNK_BYTES: usize = 64 * 1024 * 1024;

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
}

/// Interleaved stereo F32 audio at 48 kHz.
#[derive(Debug)]
pub struct AudioChunk {
    timestamp: RationalTime,
    samples: Vec<f32>,
    _budget: BudgetLease,
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

/// FFmpeg-backed software demux and decode for a validated local media source.
#[derive(Clone)]
pub struct SoftwareMediaDecoder {
    path: PathBuf,
    budgets: RuntimeBudgets,
}

impl SoftwareMediaDecoder {
    pub fn new(source: &MediaSourceRef, budgets: RuntimeBudgets) -> Result<Self, DecodeError> {
        ffmpeg::init().map_err(DecodeError::Ffmpeg)?;
        Ok(Self {
            path: source.to_file_path().map_err(DecodeError::SourceUri)?,
            budgets,
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
        let range = snapshot.requested_range();
        validate_range(range)?;
        let mut input = format::input(&self.path).map_err(DecodeError::Ffmpeg)?;
        let (stream_index, time_base, context) = {
            let stream = input
                .streams()
                .best(ffmpeg::media::Type::Video)
                .ok_or(DecodeError::MissingVideoStream)?;
            (
                stream.index(),
                TimestampBase::new(stream.time_base(), stream.start_time())?,
                codec::context::Context::from_parameters(stream.parameters())
                    .map_err(DecodeError::Ffmpeg)?,
            )
        };
        let mut decoder = context.decoder().video().map_err(DecodeError::Ffmpeg)?;
        seek_to_range(&mut input, stream_index, range, time_base)?;

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
            if drain_video(&mut decoder, &mut scaler, time_base, &job, &mut emitted)? {
                return Ok(emitted);
            }
        }

        decoder.send_eof().map_err(DecodeError::Ffmpeg)?;
        drain_video(&mut decoder, &mut scaler, time_base, &job, &mut emitted)?;
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
        let mut input = format::input(&self.path).map_err(DecodeError::Ffmpeg)?;
        let (stream_index, time_base, context) = {
            let stream = input
                .streams()
                .best(ffmpeg::media::Type::Video)
                .ok_or(DecodeError::MissingVideoStream)?;
            (
                stream.index(),
                TimestampBase::new(stream.time_base(), stream.start_time())?,
                codec::context::Context::from_parameters(stream.parameters())
                    .map_err(DecodeError::Ffmpeg)?,
            )
        };
        let mut decoder = context.decoder().video().map_err(DecodeError::Ffmpeg)?;
        seek_to_range(&mut input, stream_index, range, time_base)?;
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
                time_base,
                source_time,
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
            time_base,
            source_time,
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
        let mut input = format::input(&self.path).map_err(DecodeError::Ffmpeg)?;
        let (stream_index, time_base, context) = {
            let stream = input
                .streams()
                .best(ffmpeg::media::Type::Audio)
                .ok_or(DecodeError::MissingAudioStream)?;
            (
                stream.index(),
                TimestampBase::new(stream.time_base(), stream.start_time())?,
                codec::context::Context::from_parameters(stream.parameters())
                    .map_err(DecodeError::Ffmpeg)?,
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
        seek_to_range(&mut input, stream_index, range, time_base)?;

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
    let seek_pts = time_base
        .origin_pts
        .checked_add(time_to_rate_units_floor(relative_start, time_base.rate)?)
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
        RationalTime::from_units(relative_pts, self.rate).map_err(DecodeError::Time)
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
                let done = emit_video_frame(&decoded, timestamp, scaler, job)?;
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
    scaler: &mut Option<(Pixel, u32, u32, scaling::Context)>,
    job: &DecodeJob<'_, VideoFrame>,
) -> Result<bool, DecodeError> {
    let frame = make_video_frame(decoded, timestamp, scaler, job.budgets)?;
    queue_video(job.snapshot, frame, job.queue, job.cancellation)?;
    Ok(job.range.duration().is_zero())
}

fn make_video_frame(
    decoded: &VideoFrameBuffer,
    timestamp: RationalTime,
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
    let mut pixels = Vec::with_capacity(packed_len);
    for row in 0..height as usize {
        let start = row
            .checked_mul(stride)
            .ok_or(DecodeError::InvalidFrameData)?;
        let end = start
            .checked_add(row_bytes)
            .ok_or(DecodeError::InvalidFrameData)?;
        pixels.extend_from_slice(plane.get(start..end).ok_or(DecodeError::InvalidFrameData)?);
    }

    let descriptor = FrameDescriptor::software(width, height, FramePixelFormat::Rgba8, timestamp)?;
    let lease = FrameLease::from_software(descriptor, pixels)?;
    Ok(VideoFrame {
        lease,
        _budget: budget,
    })
}

fn drain_video_frame_at(
    decoder: &mut codec::decoder::Video,
    scaler: &mut Option<(Pixel, u32, u32, scaling::Context)>,
    time_base: TimestampBase,
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
                    .and_then(|pts| time_base.to_time(pts))?;
                if timestamp > target {
                    if candidate.is_none() {
                        *candidate = Some(make_video_frame(&decoded, timestamp, scaler, budgets)?);
                    }
                    return Ok(true);
                }
                candidate.take();
                *candidate = Some(make_video_frame(&decoded, timestamp, scaler, budgets)?);
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
    *audio_cursor = Some(
        timestamp
            .checked_add(sample_offset_time(sample_frames)?)
            .map_err(DecodeError::Time)?,
    );
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
    // The wrapper's delay helper queries in seconds and reports None for normal
    // sub-second audio delay; sample units preserve the pending resampler tail.
    let delay = unsafe {
        ffmpeg::ffi::swr_get_delay(resampler.as_ptr() as *mut _, i64::from(OUTPUT_AUDIO_RATE))
    };
    usize::try_from(delay).map_err(|_| DecodeError::AudioFrameTooLarge)
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
        ceil_sample_offset(start.checked_sub(timestamp).map_err(DecodeError::Time)?)?
    } else {
        0
    }
    .min(source_frames);
    let end_frame = if range.duration().is_zero() {
        start_frame.saturating_add(1).min(source_frames)
    } else if timestamp < end {
        floor_sample_offset(end.checked_sub(timestamp).map_err(DecodeError::Time)?)?
            .min(source_frames)
    } else {
        0
    };
    if end_frame <= start_frame {
        return Ok(None);
    }
    let timestamp = timestamp
        .checked_add(sample_offset_time(start_frame)?)
        .map_err(DecodeError::Time)?;
    samples.drain(..start_frame * OUTPUT_AUDIO_CHANNELS);
    samples.truncate((end_frame - start_frame) * OUTPUT_AUDIO_CHANNELS);
    Ok(Some((timestamp, samples)))
}

fn ceil_sample_offset(time: RationalTime) -> Result<usize, DecodeError> {
    let product = i128::from(time.numerator()) * i128::from(OUTPUT_AUDIO_RATE);
    let denominator = i128::from(time.denominator());
    let samples = if product <= 0 {
        0
    } else {
        (product + denominator - 1) / denominator
    };
    usize::try_from(samples).map_err(|_| DecodeError::TimestampOverflow)
}

fn floor_sample_offset(time: RationalTime) -> Result<usize, DecodeError> {
    let product = i128::from(time.numerator()) * i128::from(OUTPUT_AUDIO_RATE);
    let denominator = i128::from(time.denominator());
    if product <= 0 {
        return Ok(0);
    }
    usize::try_from(product / denominator).map_err(|_| DecodeError::TimestampOverflow)
}

fn sample_offset_time(samples: usize) -> Result<RationalTime, DecodeError> {
    let samples = i64::try_from(samples).map_err(|_| DecodeError::TimestampOverflow)?;
    let rate = RationalRate::new(OUTPUT_AUDIO_RATE, 1).map_err(DecodeError::Time)?;
    RationalTime::from_units(samples, rate).map_err(DecodeError::Time)
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
    Time(TimeError),
    Budget(BudgetAcquireError),
    FrameDescriptor(FrameDescriptorError),
    FrameLease(FrameLeaseError),
    MissingVideoStream,
    MissingAudioStream,
    MissingTimestamp,
    InvalidTimeBase,
    InvalidAudioRate,
    InvalidAudioFrame,
    InvalidFrameData,
    VideoFrameTooLarge,
    AudioFrameTooLarge,
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
            Self::InvalidAudioRate => {
                formatter.write_str("audio stream has an invalid sample rate")
            }
            Self::InvalidAudioFrame => {
                formatter.write_str("decoded audio frame has an unsupported layout")
            }
            Self::InvalidFrameData => formatter.write_str("decoded video frame data is incomplete"),
            Self::VideoFrameTooLarge => {
                formatter.write_str("decoded video frame exceeds the configured bound")
            }
            Self::AudioFrameTooLarge => {
                formatter.write_str("decoded audio frame exceeds the configured bound")
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
            Self::Time(error) => Some(error),
            Self::Budget(error) => Some(error),
            Self::FrameDescriptor(error) => Some(error),
            Self::FrameLease(error) => Some(error),
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

    fn time(numerator: i64, denominator: u32) -> RationalTime {
        RationalTime::new(numerator, denominator).unwrap()
    }

    #[test]
    fn ffmpeg_timebase_and_output_sample_offsets_stay_rational() {
        let base = TimestampBase::new(ffmpeg::Rational::new(1, 24_000), 0).unwrap();
        assert_eq!(base.to_time(1001).unwrap(), time(1001, 24_000));
        assert_eq!(ceil_sample_offset(time(1, 44_100)).unwrap(), 2);
        assert_eq!(floor_sample_offset(time(1, 44_100)).unwrap(), 1);
        assert_eq!(sample_offset_time(48_000).unwrap(), time(1, 1));
    }

    #[test]
    fn ffmpeg_eagain_is_a_normal_decoder_drain_result() {
        // FFmpeg exposes POSIX errno: Darwin uses 35; Windows/Linux use 11.
        let eagain_errno = if cfg!(target_os = "macos") { 35 } else { 11 };
        assert!(is_again(FfmpegError::Other {
            errno: eagain_errno,
        }));
    }
}
