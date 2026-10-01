use std::cell::{Cell, UnsafeCell};
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use or_core::{
    AudioSettings, MAX_AUDIO_GAIN_MILLIDECIBELS, MIN_AUDIO_GAIN_MILLIDECIBELS, RationalRate,
    RationalTime, TimeError,
};
use or_runtime::{CancellationToken, RenderSnapshot};

/// Fixed-capacity interleaved audio storage shared by one producer and one
/// device callback consumer.
pub struct AudioBuffer {
    inner: Arc<AudioBufferInner>,
}

struct AudioBufferInner {
    samples: Vec<UnsafeCell<f32>>,
    channels: usize,
    read_index: AtomicUsize,
    write_index: AtomicUsize,
}

// SAFETY: split consumes the buffer and creates one non-Sync producer and one
// non-Sync consumer. The producer writes only free slots and publishes them
// with Release; the consumer reads only published slots and releases them with
// Release before the producer can reuse them.
unsafe impl Sync for AudioBufferInner {}

impl AudioBuffer {
    pub fn new(
        capacity_frames: NonZeroUsize,
        channels: NonZeroUsize,
    ) -> Result<Self, AudioBufferError> {
        let capacity_samples = capacity_frames
            .get()
            .checked_mul(channels.get())
            .ok_or(AudioBufferError::CapacityOverflow)?;
        let mut samples = Vec::new();
        samples
            .try_reserve_exact(capacity_samples)
            .map_err(|_| AudioBufferError::AllocationFailed)?;
        samples.resize_with(capacity_samples, || UnsafeCell::new(0.0));

        Ok(Self {
            inner: Arc::new(AudioBufferInner {
                samples,
                channels: channels.get(),
                read_index: AtomicUsize::new(0),
                write_index: AtomicUsize::new(0),
            }),
        })
    }

    pub fn capacity_frames(&self) -> usize {
        self.inner.samples.len() / self.inner.channels
    }

    pub fn channels(&self) -> usize {
        self.inner.channels
    }

    /// Consumes the buffer and creates its sole producer and callback consumer.
    pub fn split(self, clock: AudioClockMessage) -> (AudioProducer, AudioConsumer) {
        self.inner.read_index.store(0, Ordering::Relaxed);
        self.inner.write_index.store(0, Ordering::Relaxed);
        let consumer_buffer = Arc::clone(&self.inner);
        (
            AudioProducer {
                buffer: self.inner,
                write_index: 0,
                _not_sync: PhantomData,
            },
            AudioConsumer {
                buffer: consumer_buffer,
                read_index: 0,
                clock,
                _not_sync: PhantomData,
            },
        )
    }
}

/// The decode/resample side of the bounded audio buffer.
pub struct AudioProducer {
    buffer: Arc<AudioBufferInner>,
    write_index: usize,
    _not_sync: PhantomData<Cell<()>>,
}

impl AudioProducer {
    /// Copies a complete interleaved block without waiting or growing storage.
    pub fn try_push(
        &mut self,
        samples: &[f32],
        cancellation: &CancellationToken,
    ) -> Result<(), AudioPushError> {
        if !samples.len().is_multiple_of(self.buffer.channels) {
            return Err(AudioPushError::IncompleteFrame);
        }
        if cancellation.is_cancelled() {
            return Err(AudioPushError::Cancelled);
        }
        if samples.is_empty() {
            return Ok(());
        }

        let capacity = self.buffer.samples.len();
        let read_index = self.buffer.read_index.load(Ordering::Acquire);
        let used = self.write_index.wrapping_sub(read_index);
        if used > capacity || samples.len() > capacity - used {
            return Err(AudioPushError::Full);
        }

        for (offset, sample) in samples.iter().enumerate() {
            let index = self.write_index.wrapping_add(offset) % capacity;
            // SAFETY: this producer exclusively owns every free slot until
            // write_index is published below. The consumer cannot read these
            // slots before the matching Acquire load observes that publication.
            unsafe { *self.buffer.samples[index].get() = *sample };
        }

        self.write_index = self.write_index.wrapping_add(samples.len());
        self.buffer
            .write_index
            .store(self.write_index, Ordering::Release);
        Ok(())
    }
}

/// The sole callback-side consumer of an audio buffer.
pub struct AudioConsumer {
    buffer: Arc<AudioBufferInner>,
    read_index: usize,
    clock: AudioClockMessage,
    _not_sync: PhantomData<Cell<()>>,
}

impl AudioConsumer {
    /// Fills one device callback block. Missing samples become silence; this
    /// method performs no allocation, locking, project access, or waiting.
    pub fn render_into(
        &mut self,
        output: &mut [f32],
        cancellation: &CancellationToken,
    ) -> Result<AudioRenderReport, AudioRenderError> {
        if !output.len().is_multiple_of(self.buffer.channels) {
            output.fill(0.0);
            return Err(AudioRenderError::IncompleteFrame);
        }

        let frames = output.len() / self.buffer.channels;
        if cancellation.is_cancelled() {
            output.fill(0.0);
            let published = self.buffer.write_index.load(Ordering::Acquire);
            self.read_index = published;
            self.buffer
                .read_index
                .store(self.read_index, Ordering::Release);
            self.advance_clock(frames);
            return Ok(AudioRenderReport {
                clock: self.clock,
                underrun_frames: 0,
                cancelled: true,
            });
        }

        let published = self.buffer.write_index.load(Ordering::Acquire);
        let available = published
            .wrapping_sub(self.read_index)
            .min(self.buffer.samples.len());
        let copied = available.min(output.len());
        for (offset, target) in output[..copied].iter_mut().enumerate() {
            let index = self.read_index.wrapping_add(offset) % self.buffer.samples.len();
            // SAFETY: Acquire observed the producer's Release publication;
            // this consumer exclusively owns each published slot until it
            // releases the updated read index below.
            *target = unsafe { *self.buffer.samples[index].get() };
        }
        output[copied..].fill(0.0);
        self.read_index = self.read_index.wrapping_add(copied);
        self.buffer
            .read_index
            .store(self.read_index, Ordering::Release);

        self.advance_clock(frames);
        Ok(AudioRenderReport {
            clock: self.clock,
            underrun_frames: ((output.len() - copied) / self.buffer.channels) as u64,
            cancelled: false,
        })
    }

    fn advance_clock(&mut self, frames: usize) {
        let frames = u64::try_from(frames).unwrap_or(u64::MAX);
        self.clock.device_frames = self.clock.device_frames.saturating_add(frames);
    }
}

/// Exact snapshot of the audio device's master playback clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioClockMessage {
    sample_rate: NonZeroU32,
    snapshot: RenderSnapshot,
    device_frames: u64,
}

impl AudioClockMessage {
    /// Starts a device clock at a timeline seek position.
    pub const fn new(sample_rate: NonZeroU32, snapshot: RenderSnapshot) -> Self {
        Self {
            sample_rate,
            snapshot,
            device_frames: 0,
        }
    }

    pub const fn sample_rate(self) -> NonZeroU32 {
        self.sample_rate
    }

    pub const fn snapshot(self) -> RenderSnapshot {
        self.snapshot
    }

    pub const fn timeline_origin(self) -> RationalTime {
        self.snapshot.requested_range().start()
    }

    /// Device frames include frames rendered as silence during underruns.
    pub const fn device_frames(self) -> u64 {
        self.device_frames
    }

    pub(crate) const fn with_device_frames(mut self, device_frames: u64) -> Self {
        self.device_frames = device_frames;
        self
    }

    /// Converts the device position to exact timeline seconds.
    pub fn timeline_time(self) -> Result<RationalTime, AudioClockError> {
        let frames =
            i64::try_from(self.device_frames).map_err(|_| AudioClockError::FrameCountOverflow)?;
        let rate = RationalRate::new(self.sample_rate.get(), 1).map_err(AudioClockError::Time)?;
        let elapsed = RationalTime::from_units(frames, rate).map_err(AudioClockError::Time)?;
        self.timeline_origin()
            .checked_add(elapsed)
            .map_err(AudioClockError::Time)
    }

    /// Maps timeline seconds to the device frame at or immediately before that
    /// time. The exact rational product is floored; negative relative times are
    /// rejected rather than rounded into the device stream.
    pub fn device_frame_at_or_before(
        self,
        timeline_time: RationalTime,
    ) -> Result<u64, AudioClockError> {
        let elapsed = timeline_time
            .checked_sub(self.timeline_origin())
            .map_err(AudioClockError::Time)?;
        if elapsed.is_negative() {
            return Err(AudioClockError::BeforeTimelineOrigin);
        }

        let numerator = i128::from(elapsed.numerator()) * i128::from(self.sample_rate.get());
        let frames = numerator.div_euclid(i128::from(elapsed.denominator()));
        u64::try_from(frames).map_err(|_| AudioClockError::FrameCountOverflow)
    }
}

/// Deterministic video correction against the audio device master clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AvSynchronizer {
    max_video_drift: RationalTime,
}

impl AvSynchronizer {
    pub fn new(max_video_drift: RationalTime) -> Result<Self, AvSynchronizerError> {
        if max_video_drift.is_negative() {
            return Err(AvSynchronizerError::NegativeDriftTolerance);
        }
        Ok(Self { max_video_drift })
    }

    pub const fn max_video_drift(self) -> RationalTime {
        self.max_video_drift
    }

    pub fn video_action(
        self,
        video_snapshot: RenderSnapshot,
        video_time: RationalTime,
        audio_clock: AudioClockMessage,
    ) -> Result<VideoSyncAction, AudioClockError> {
        if !audio_clock.snapshot().matches(
            video_snapshot.project_id(),
            video_snapshot.project_revision(),
        ) {
            return Err(AudioClockError::SnapshotMismatch);
        }
        let audio_time = audio_clock.timeline_time()?;
        let oldest_presentable = audio_time
            .checked_sub(self.max_video_drift)
            .map_err(AudioClockError::Time)?;
        let newest_presentable = audio_time
            .checked_add(self.max_video_drift)
            .map_err(AudioClockError::Time)?;
        Ok(if video_time < oldest_presentable {
            VideoSyncAction::DropStaleFrame
        } else if video_time > newest_presentable {
            VideoSyncAction::WaitForAudio
        } else {
            VideoSyncAction::Present
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioRenderReport {
    pub clock: AudioClockMessage,
    pub underrun_frames: u64,
    pub cancelled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VideoSyncAction {
    DropStaleFrame,
    Present,
    WaitForAudio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioBufferError {
    CapacityOverflow,
    AllocationFailed,
}

impl fmt::Display for AudioBufferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityOverflow => formatter.write_str("audio buffer capacity overflow"),
            Self::AllocationFailed => formatter.write_str("audio buffer allocation failed"),
        }
    }
}

impl Error for AudioBufferError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioPushError {
    IncompleteFrame,
    Full,
    Cancelled,
}

impl fmt::Display for AudioPushError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompleteFrame => formatter.write_str("audio samples do not form whole frames"),
            Self::Full => formatter.write_str("audio buffer is full"),
            Self::Cancelled => formatter.write_str("audio playback was cancelled"),
        }
    }
}

impl Error for AudioPushError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioRenderError {
    IncompleteFrame,
}

impl fmt::Display for AudioRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("device callback samples do not form whole frames")
    }
}

impl Error for AudioRenderError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioClockError {
    BeforeTimelineOrigin,
    FrameCountOverflow,
    SnapshotMismatch,
    Time(TimeError),
}

impl fmt::Display for AudioClockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeforeTimelineOrigin => {
                formatter.write_str("timeline time precedes the audio clock origin")
            }
            Self::FrameCountOverflow => formatter.write_str("audio clock frame count overflow"),
            Self::SnapshotMismatch => formatter.write_str("audio and video snapshots do not match"),
            Self::Time(error) => error.fmt(formatter),
        }
    }
}

impl Error for AudioClockError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Time(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AvSynchronizerError {
    NegativeDriftTolerance,
}

impl fmt::Display for AvSynchronizerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("A/V drift tolerance cannot be negative")
    }
}

impl Error for AvSynchronizerError {}

/// Applies the persisted basic clip gain, pan, and linear fades to one
/// interleaved stereo block. The block's first sample offset is exact timeline
/// time relative to the clip start.
pub fn process_audio_clip(
    samples: &mut [f32],
    clip_offset: RationalTime,
    clip_duration: RationalTime,
    settings: AudioSettings,
) -> Result<(), AudioProcessError> {
    if !samples.len().is_multiple_of(2) {
        return Err(AudioProcessError::IncompleteStereoFrame);
    }
    if clip_offset.is_negative()
        || clip_duration.is_negative()
        || settings.gain_millidecibels < MIN_AUDIO_GAIN_MILLIDECIBELS
        || settings.gain_millidecibels > MAX_AUDIO_GAIN_MILLIDECIBELS
        || !(-10_000..=10_000).contains(&settings.pan_basis_points)
        || settings.fade_in.is_negative()
        || settings.fade_out.is_negative()
        || settings.fade_in > clip_duration
        || settings.fade_out > clip_duration
    {
        return Err(AudioProcessError::InvalidSettings);
    }

    const SAMPLE_RATE: f64 = 48_000.0;
    let gain = 10.0_f64.powf(f64::from(settings.gain_millidecibels) / 20_000.0) as f32;
    let pan = f32::from(settings.pan_basis_points) / 10_000.0;
    let left_pan = (1.0 - pan).min(1.0);
    let right_pan = (1.0 + pan).min(1.0);
    let offset = seconds(clip_offset);
    let duration = seconds(clip_duration);
    let fade_in = seconds(settings.fade_in);
    let fade_out = seconds(settings.fade_out);

    let (stereo_frames, _) = samples.as_chunks_mut::<2>();
    for (frame, stereo) in stereo_frames.iter_mut().enumerate() {
        let time = offset + frame as f64 / SAMPLE_RATE;
        let fade_in_gain = if fade_in == 0.0 {
            1.0
        } else {
            (time / fade_in).clamp(0.0, 1.0) as f32
        };
        let fade_out_gain = if fade_out == 0.0 {
            1.0
        } else {
            ((duration - time) / fade_out).clamp(0.0, 1.0) as f32
        };
        let level = gain * fade_in_gain * fade_out_gain;
        stereo[0] *= level * left_pan;
        stereo[1] *= level * right_pan;
    }
    Ok(())
}

fn seconds(time: RationalTime) -> f64 {
    time.numerator() as f64 / f64::from(time.denominator())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioProcessError {
    IncompleteStereoFrame,
    InvalidSettings,
}

impl fmt::Display for AudioProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompleteStereoFrame => {
                formatter.write_str("audio samples do not form whole stereo frames")
            }
            Self::InvalidSettings => formatter.write_str("audio settings or clip time are invalid"),
        }
    }
}

impl Error for AudioProcessError {}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
mod device;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub use device::{DesktopAudioOutput, DesktopAudioOutputError};

#[cfg(test)]
mod tests {
    use super::{
        AudioBuffer, AudioClockError, AudioClockMessage, AudioProcessError, AudioPushError,
        AudioRenderError, AvSynchronizer, AvSynchronizerError, VideoSyncAction, process_audio_clip,
    };
    use or_core::{AudioSettings, ProjectDocument, RationalTime};
    use or_runtime::{CancellationToken, RenderSnapshot};
    use std::num::{NonZeroU32, NonZeroUsize};

    fn time(numerator: i64, denominator: u32) -> RationalTime {
        RationalTime::new(numerator, denominator).unwrap()
    }

    fn buffer(capacity_frames: usize, channels: usize) -> AudioBuffer {
        AudioBuffer::new(
            NonZeroUsize::new(capacity_frames).unwrap(),
            NonZeroUsize::new(channels).unwrap(),
        )
        .unwrap()
    }

    fn clock(rate: u32, origin: RationalTime) -> AudioClockMessage {
        AudioClockMessage::new(NonZeroU32::new(rate).unwrap(), snapshot_at(origin))
    }

    fn snapshot_at(origin: RationalTime) -> RenderSnapshot {
        let project = ProjectDocument::new("audio test");
        RenderSnapshot::at_time(&project, origin).unwrap()
    }

    #[test]
    fn timeline_conversion_floors_exact_rationals_and_keeps_seek_origin() {
        let clock = clock(48_000, time(2, 1));
        assert_eq!(clock.device_frame_at_or_before(time(2, 1)), Ok(0));
        assert_eq!(
            clock.device_frame_at_or_before(time(2, 1).checked_add(time(1, 3)).unwrap()),
            Ok(16_000)
        );
        assert_eq!(
            clock.device_frame_at_or_before(time(2, 1).checked_add(time(1, 7)).unwrap()),
            Ok(6_857)
        );
        assert_eq!(
            clock.device_frame_at_or_before(time(1, 1)),
            Err(AudioClockError::BeforeTimelineOrigin)
        );
    }

    #[test]
    fn callback_uses_audio_as_master_and_silences_underruns() {
        let token = CancellationToken::new();
        let buffer = buffer(2, 2);
        let (mut producer, mut consumer) = buffer.split(clock(48_000, time(3, 2)));
        producer.try_push(&[0.25, -0.25], &token).unwrap();

        let mut output = [9.0; 4];
        let report = consumer.render_into(&mut output, &token).unwrap();
        assert_eq!(output, [0.25, -0.25, 0.0, 0.0]);
        assert_eq!(report.underrun_frames, 1);
        assert!(!report.cancelled);
        assert_eq!(report.clock.device_frames(), 2);
        assert_eq!(
            report.clock.timeline_time(),
            Ok(time(3, 2).checked_add(time(1, 24_000)).unwrap())
        );
    }

    #[test]
    fn bounded_buffer_reports_full_and_wraps_without_growing() {
        let token = CancellationToken::new();
        let buffer = buffer(2, 2);
        let capacity_frames = buffer.capacity_frames();
        let (mut producer, mut consumer) = buffer.split(clock(48_000, RationalTime::ZERO));
        producer.try_push(&[1.0, 2.0, 3.0, 4.0], &token).unwrap();
        assert_eq!(
            producer.try_push(&[5.0, 6.0], &token),
            Err(AudioPushError::Full)
        );

        let mut first = [0.0; 4];
        consumer.render_into(&mut first, &token).unwrap();
        assert_eq!(first, [1.0, 2.0, 3.0, 4.0]);
        producer.try_push(&[5.0, 6.0, 7.0, 8.0], &token).unwrap();

        let mut wrapped = [0.0; 4];
        consumer.render_into(&mut wrapped, &token).unwrap();
        assert_eq!(wrapped, [5.0, 6.0, 7.0, 8.0]);
        assert_eq!(capacity_frames, 2);
    }

    #[test]
    fn single_producer_and_callback_transfer_ordered_frames_across_threads() {
        const FRAME_COUNT: u32 = 4_096;

        let token = CancellationToken::new();
        let producer_token = token.clone();
        let buffer = buffer(64, 2);
        let (mut producer, mut consumer) = buffer.split(clock(48_000, RationalTime::ZERO));
        std::thread::scope(|scope| {
            let producer_thread = scope.spawn(move || {
                for frame in 1..=FRAME_COUNT {
                    let samples = [frame as f32, frame as f32];
                    loop {
                        match producer.try_push(&samples, &producer_token) {
                            Ok(()) => break,
                            Err(AudioPushError::Full) => std::thread::yield_now(),
                            Err(error) => return Err(error),
                        }
                    }
                }
                Ok(())
            });

            let mut output = [0.0; 128];
            let mut expected = 1.0;
            let mut out_of_order = false;
            for attempt in 0..10_000 {
                if expected > FRAME_COUNT as f32 {
                    break;
                }
                consumer.render_into(&mut output, &token).unwrap();
                for frame in output.as_chunks::<2>().0 {
                    if *frame == [0.0, 0.0] {
                        continue;
                    }
                    if *frame != [expected, expected] {
                        out_of_order = true;
                        break;
                    }
                    expected += 1.0;
                }
                if out_of_order {
                    break;
                }
                if attempt % 16 == 0 {
                    std::thread::yield_now();
                }
            }

            let complete = expected > FRAME_COUNT as f32;
            if !complete || out_of_order {
                token.cancel();
            }
            let producer_result = producer_thread.join().unwrap();
            assert!(
                complete,
                "producer and callback did not transfer every frame"
            );
            assert!(
                !out_of_order,
                "callback observed reordered or duplicated audio"
            );
            assert_eq!(producer_result, Ok(()));
        });
    }

    #[test]
    fn cancellation_silences_output_advances_device_clock_and_rejects_input() {
        let token = CancellationToken::new();
        let buffer = buffer(2, 2);
        let (mut producer, mut consumer) = buffer.split(clock(48_000, RationalTime::ZERO));
        producer.try_push(&[1.0, 2.0], &token).unwrap();
        token.cancel();
        assert_eq!(
            producer.try_push(&[3.0, 4.0], &token),
            Err(AudioPushError::Cancelled)
        );

        let mut output = [9.0; 4];
        let report = consumer.render_into(&mut output, &token).unwrap();
        assert_eq!(output, [0.0; 4]);
        assert_eq!(report.underrun_frames, 0);
        assert!(report.cancelled);
        assert_eq!(report.clock.device_frames(), 2);
    }

    #[test]
    fn callback_rejects_partial_device_frames_as_silence() {
        let token = CancellationToken::new();
        let buffer = buffer(1, 2);
        let (_, mut consumer) = buffer.split(clock(48_000, RationalTime::ZERO));
        let mut output = [9.0; 3];
        assert_eq!(
            consumer.render_into(&mut output, &token),
            Err(AudioRenderError::IncompleteFrame)
        );
        assert_eq!(output, [0.0; 3]);
    }

    #[test]
    fn video_waits_drops_or_presents_using_exact_audio_drift() {
        let project = ProjectDocument::new("audio sync test");
        let snapshot = RenderSnapshot::at_time(&project, RationalTime::ZERO).unwrap();
        let video_snapshot = RenderSnapshot::at_time(&project, time(1, 500)).unwrap();
        let audio_clock = AudioClockMessage::new(NonZeroU32::new(48_000).unwrap(), snapshot);
        let buffer = buffer(1, 2);
        let (_, mut consumer) = buffer.split(audio_clock);
        let token = CancellationToken::new();
        let report = consumer.render_into(&mut [0.0; 192], &token).unwrap();
        let sync = AvSynchronizer::new(time(1, 1_000)).unwrap();

        assert_eq!(
            sync.video_action(video_snapshot, time(0, 1), report.clock),
            Ok(VideoSyncAction::DropStaleFrame)
        );
        assert_eq!(
            sync.video_action(video_snapshot, time(1, 500), report.clock),
            Ok(VideoSyncAction::Present)
        );
        assert_eq!(
            sync.video_action(video_snapshot, time(1, 250), report.clock),
            Ok(VideoSyncAction::WaitForAudio)
        );
        assert_eq!(
            sync.video_action(snapshot_at(time(0, 1)), time(0, 1), report.clock),
            Err(AudioClockError::SnapshotMismatch)
        );
        let other_revision = RenderSnapshot::new(
            snapshot.project_id(),
            or_core::ProjectRevision::new(snapshot.project_revision().value() + 1),
            snapshot.requested_range(),
        );
        assert_eq!(
            sync.video_action(other_revision, time(0, 1), report.clock),
            Err(AudioClockError::SnapshotMismatch)
        );
        assert_eq!(
            AvSynchronizer::new(time(-1, 1)),
            Err(AvSynchronizerError::NegativeDriftTolerance)
        );
    }

    #[test]
    fn clip_gain_pan_and_fades_process_interleaved_stereo_samples() {
        let mut samples = [1.0; 8];
        process_audio_clip(
            &mut samples,
            RationalTime::ZERO,
            time(4, 48_000),
            AudioSettings {
                pan_basis_points: 10_000,
                fade_in: time(1, 48_000),
                fade_out: time(2, 48_000),
                ..AudioSettings::DEFAULT
            },
        )
        .unwrap();
        assert_eq!(samples[0], 0.0);
        assert_eq!(samples[1], 0.0);
        assert_eq!(samples[2], 0.0);
        assert_eq!(samples[3], 1.0);
        assert_eq!(samples[5], 1.0);
        assert_eq!(samples[7], 0.5);
    }

    #[test]
    fn clip_processor_rejects_invalid_blocks_and_ranges() {
        assert_eq!(
            process_audio_clip(
                &mut [0.0; 3],
                RationalTime::ZERO,
                time(1, 1),
                AudioSettings::DEFAULT
            ),
            Err(AudioProcessError::IncompleteStereoFrame)
        );
        assert_eq!(
            process_audio_clip(
                &mut [0.0; 2],
                RationalTime::ZERO,
                time(1, 1),
                AudioSettings {
                    gain_millidecibels: 24_001,
                    ..AudioSettings::DEFAULT
                },
            ),
            Err(AudioProcessError::InvalidSettings)
        );
    }
}
