use crate::{AudioBuffer, AudioBufferError, AudioClockMessage, AudioConsumer, AudioProducer};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use or_runtime::{CancellationToken, RenderSnapshot};
use std::error::Error;
use std::fmt;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

const DEVICE_SAMPLE_RATE: u32 = 48_000;
const DEVICE_CHANNELS: usize = 2;

/// CPAL output. The stream starts paused; callers can prefill the
/// returned bounded producer before starting device callbacks.
pub struct AudioDeviceOutput {
    stream: Stream,
    clock: AudioClockMessage,
    device_frames: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
    callback_cancellation: CancellationToken,
}

impl AudioDeviceOutput {
    pub fn open(
        snapshot: RenderSnapshot,
        capacity_frames: NonZeroUsize,
    ) -> Result<(AudioProducer, Self), AudioDeviceOutputError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioDeviceOutputError::NoOutputDevice)?;
        let config = device
            .supported_output_configs()
            .map_err(|error| AudioDeviceOutputError::Device(error.to_string()))?
            .find_map(|range| {
                (range.channels() as usize == DEVICE_CHANNELS
                    && range.sample_format() == SampleFormat::F32)
                    .then(|| range.try_with_sample_rate(DEVICE_SAMPLE_RATE))
                    .flatten()
            })
            .ok_or(AudioDeviceOutputError::NoSupportedConfiguration)?;

        let clock = AudioClockMessage::new(
            NonZeroU32::new(DEVICE_SAMPLE_RATE).expect("fixed rate is nonzero"),
            snapshot,
        );
        let audio_buffer = AudioBuffer::new(
            capacity_frames,
            NonZeroUsize::new(DEVICE_CHANNELS).expect("fixed channel count is nonzero"),
        )?;
        let (producer, mut consumer): (AudioProducer, AudioConsumer) = audio_buffer.split(clock);
        let device_frames = Arc::new(AtomicU64::new(0));
        let callback_frames = Arc::clone(&device_frames);
        let failed = Arc::new(AtomicBool::new(false));
        let callback_failed = Arc::clone(&failed);
        let error_failed = Arc::clone(&failed);
        let callback_cancellation = CancellationToken::new();
        let render_cancellation = callback_cancellation.clone();
        let stream = device
            .build_output_stream::<f32, _, _>(
                config.config(),
                move |output, _| match consumer.render_into(output, &render_cancellation) {
                    Ok(report) => {
                        callback_frames.store(report.clock.device_frames(), Ordering::Relaxed)
                    }
                    Err(_) => callback_failed.store(true, Ordering::Relaxed),
                },
                move |_| error_failed.store(true, Ordering::Relaxed),
                Some(Duration::from_secs(5)),
            )
            .map_err(|error| AudioDeviceOutputError::Stream(error.to_string()))?;

        Ok((
            producer,
            Self {
                stream,
                clock,
                device_frames,
                failed,
                callback_cancellation,
            },
        ))
    }

    pub fn start(&self) -> Result<(), AudioDeviceOutputError> {
        self.stream
            .play()
            .map_err(|error| AudioDeviceOutputError::Stream(error.to_string()))
    }

    pub fn clock(&self) -> AudioClockMessage {
        self.clock
            .with_device_frames(self.device_frames.load(Ordering::Relaxed))
    }

    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }
}

impl Drop for AudioDeviceOutput {
    fn drop(&mut self) {
        self.callback_cancellation.cancel();
        let _ = self.stream.pause();
    }
}

#[derive(Debug)]
pub enum AudioDeviceOutputError {
    NoOutputDevice,
    NoSupportedConfiguration,
    Device(String),
    Stream(String),
    Buffer(AudioBufferError),
}

impl fmt::Display for AudioDeviceOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoOutputDevice => formatter.write_str("no default audio output device"),
            Self::NoSupportedConfiguration => formatter
                .write_str("the default output device does not support stereo F32 audio at 48 kHz"),
            Self::Device(message) => write!(
                formatter,
                "could not inspect audio output device: {message}"
            ),
            Self::Stream(message) => write!(
                formatter,
                "could not create or start audio output: {message}"
            ),
            Self::Buffer(error) => error.fmt(formatter),
        }
    }
}

impl Error for AudioDeviceOutputError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Buffer(error) => Some(error),
            _ => None,
        }
    }
}

impl From<AudioBufferError> for AudioDeviceOutputError {
    fn from(error: AudioBufferError) -> Self {
        Self::Buffer(error)
    }
}
