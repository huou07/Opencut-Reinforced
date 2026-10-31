use crate::{
    AudioBuffer, AudioBufferError, AudioClockMessage, AudioConsumer, AudioProducer, LinearResampler,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, Stream, SupportedStreamConfigRange};
use or_runtime::{CancellationToken, RenderSnapshot};
use std::error::Error;
use std::fmt;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

/// The project audio pipeline always renders at this rate and channel count.
const PROJECT_SAMPLE_RATE: u32 = 48_000;
const PROJECT_CHANNELS: usize = 2;
/// Upper bound on source frames converted in one device callback.
const MAX_CALLBACK_FRAMES: usize = 32_768;
const PREFERRED_SAMPLE_RATE: u32 = 48_000;

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
        let selection = select_output_configuration(&device)?;
        let device_config = selection.config.clone();
        let stream_config = device_config.config();
        let device_rate = device_config.sample_rate();
        let device_channels = usize::from(device_config.channels());

        let clock = AudioClockMessage::new(
            NonZeroU32::new(PROJECT_SAMPLE_RATE).expect("fixed rate is nonzero"),
            snapshot,
        );
        let audio_buffer = AudioBuffer::new(
            capacity_frames,
            NonZeroUsize::new(PROJECT_CHANNELS).expect("fixed channel count is nonzero"),
        )?;
        let (producer, consumer): (AudioProducer, AudioConsumer) = audio_buffer.split(clock);
        let device_frames = Arc::new(AtomicU64::new(0));
        let failed = Arc::new(AtomicBool::new(false));
        let callback_cancellation = CancellationToken::new();
        let state = CallbackState::new(device_rate);

        let build_failed = Arc::clone(&failed);
        let build_frames = Arc::clone(&device_frames);
        let build_cancellation = callback_cancellation.clone();
        let stream = match selection.sample_format {
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                &stream_config,
                consumer,
                state,
                device_channels,
                build_cancellation,
                build_frames,
                build_failed,
            ),
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                &stream_config,
                consumer,
                state,
                device_channels,
                build_cancellation,
                build_frames,
                build_failed,
            ),
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                &stream_config,
                consumer,
                state,
                device_channels,
                build_cancellation,
                build_frames,
                build_failed,
            ),
            _ => return Err(AudioDeviceOutputError::NoSupportedConfiguration),
        }?;

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

struct OutputSelection {
    config: cpal::SupportedStreamConfig,
    sample_format: SampleFormat,
}

/// Picks the device configuration that needs the least adaptation.
///
/// The historical code demanded stereo F32 at exactly 48 kHz, which excludes
/// Bluetooth headsets that expose 44.1 kHz, mono, or 16-bit output. Any
/// configuration whose sample format can be converted is accepted; the mixer
/// stays at 48 kHz stereo and the callback resamples and maps channels.
fn select_output_configuration(
    device: &cpal::Device,
) -> Result<OutputSelection, AudioDeviceOutputError> {
    let ranges: Vec<SupportedStreamConfigRange> = device
        .supported_output_configs()
        .map_err(|error| AudioDeviceOutputError::Device(error.to_string()))?
        .collect();

    let mut best: Option<(u32, OutputSelection)> = None;
    for range in ranges {
        let Some(format_rank) = sample_format_rank(range.sample_format()) else {
            continue;
        };
        let channels = range.channels();
        if channels == 0 || channels > 8 {
            continue;
        }
        let rate = preferred_rate(&range);
        let Some(config) = range.try_with_sample_rate(rate) else {
            continue;
        };
        let score = u32::from(format_rank) * 1_000
            + u32::from(channel_rank(channels)) * 100
            + u32::from(rate_rank(rate));
        if best.as_ref().is_none_or(|(current, _)| score < *current) {
            best = Some((
                score,
                OutputSelection {
                    config,
                    sample_format: range.sample_format(),
                },
            ));
        }
    }

    best.map(|(_, selection)| selection)
        .ok_or(AudioDeviceOutputError::NoSupportedConfiguration)
}

fn sample_format_rank(format: SampleFormat) -> Option<u8> {
    match format {
        SampleFormat::F32 => Some(0),
        SampleFormat::I16 => Some(1),
        SampleFormat::U16 => Some(2),
        _ => None,
    }
}

fn channel_rank(channels: u16) -> u8 {
    match channels {
        2 => 0,
        1 => 1,
        _ => 2,
    }
}

fn rate_rank(rate: u32) -> u8 {
    match rate.abs_diff(PREFERRED_SAMPLE_RATE) {
        0 => 0,
        drift if drift <= 4_800 => 1,
        _ => 2,
    }
}

/// Chooses 48 kHz when the range carries it, otherwise the range's maximum.
fn preferred_rate(range: &SupportedStreamConfigRange) -> u32 {
    let min = range.min_sample_rate();
    let max = range.max_sample_rate();
    if (min..=max).contains(&PREFERRED_SAMPLE_RATE) {
        PREFERRED_SAMPLE_RATE
    } else {
        max
    }
}

struct CallbackState {
    resampler: LinearResampler,
    scratch: Vec<f32>,
    stereo: Vec<f32>,
}

impl CallbackState {
    fn new(device_rate: u32) -> Self {
        Self {
            resampler: LinearResampler::new(PROJECT_SAMPLE_RATE, device_rate)
                .expect("both sample rates are nonzero"),
            scratch: vec![0.0; MAX_CALLBACK_FRAMES * PROJECT_CHANNELS],
            stereo: Vec::new(),
        }
    }
}

fn build_stream<S>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut consumer: AudioConsumer,
    mut state: CallbackState,
    device_channels: usize,
    cancellation: CancellationToken,
    device_frames: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
) -> Result<Stream, AudioDeviceOutputError>
where
    S: SizedSample + FromSample<f32> + Send + 'static,
{
    let callback_failed = Arc::clone(&failed);
    let error_failed = Arc::clone(&failed);
    device
        .build_output_stream::<S, _, _>(
            *config,
            move |output: &mut [S], _| match render_block::<S>(
                &mut consumer,
                &mut state,
                device_channels,
                output,
                &cancellation,
            ) {
                Ok(frames) => device_frames.store(frames, Ordering::Relaxed),
                Err(_) => callback_failed.store(true, Ordering::Relaxed),
            },
            move |_| error_failed.store(true, Ordering::Relaxed),
            Some(Duration::from_secs(5)),
        )
        .map_err(|error| AudioDeviceOutputError::Stream(error.to_string()))
}

fn render_block<S>(
    consumer: &mut AudioConsumer,
    state: &mut CallbackState,
    device_channels: usize,
    output: &mut [S],
    cancellation: &CancellationToken,
) -> Result<u64, crate::AudioRenderError>
where
    S: SizedSample + FromSample<f32>,
{
    let device_frames = output.len() / device_channels;
    if device_frames == 0 {
        return Ok(0);
    }

    let required = state.resampler.required_input_frames(device_frames);
    let retained = state.resampler.retained_input_frames();
    let new_frames = required.saturating_sub(retained).min(MAX_CALLBACK_FRAMES);
    let new_samples = new_frames * PROJECT_CHANNELS;
    state.scratch[..new_samples].fill(0.0);
    let report = consumer.render_into(&mut state.scratch[..new_samples], cancellation)?;
    state.resampler.push(&state.scratch, new_frames);

    state.stereo.resize(device_frames * PROJECT_CHANNELS, 0.0);
    state.resampler.pull(&mut state.stereo, device_frames);
    write_channels(&state.stereo, device_frames, device_channels, output);
    Ok(report.clock.device_frames())
}

/// Writes stereo into the device layout: mono downmix, stereo copy, and
/// silence for any extra surround channels.
fn write_channels<S>(stereo: &[f32], frames: usize, channels: usize, output: &mut [S])
where
    S: SizedSample + FromSample<f32>,
{
    for frame in 0..frames {
        let left = stereo[frame * PROJECT_CHANNELS];
        let right = stereo[frame * PROJECT_CHANNELS + 1];
        let base = frame * channels;
        if channels == 1 {
            output[base] = S::from_sample((left + right) * 0.5);
            continue;
        }
        output[base] = S::from_sample(left);
        output[base + 1] = S::from_sample(right);
        for channel in 2..channels {
            output[base + channel] = S::from_sample(0.0);
        }
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
            Self::NoSupportedConfiguration => formatter.write_str(
                "the default output device exposes no usable audio output configuration",
            ),
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

#[cfg(test)]
mod tests {
    use super::write_channels;
    use cpal::Sample;

    #[test]
    fn stereo_is_copied_and_mono_is_a_downmix() {
        let stereo = [0.8f32, 0.4];
        let mut mono = [0i16; 1];
        write_channels(&stereo, 1, 1, &mut mono);
        assert_eq!(mono, [i16::from_sample(0.6)]);
    }

    #[test]
    fn surround_layouts_fill_the_first_two_channels_and_silence_the_rest() {
        let stereo = [0.5f32, -0.5];
        let mut surround = [1f32; 4];
        write_channels(&stereo, 1, 4, &mut surround);
        assert_eq!(surround, [0.5, -0.5, 0.0, 0.0]);
    }
}
