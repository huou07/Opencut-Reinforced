use ffmpeg::{
    codec::{
        self,
        encoder::{audio::Encoder as AudioEncoder, video::Encoder as VideoEncoder},
    },
    format::{self, Sample},
    frame,
    software::scaling,
    util::dictionary::Dictionary,
    util::format::sample::Type,
};
use ffmpeg_the_third as ffmpeg;
use std::{
    collections::VecDeque,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const AUDIO_RATE: u32 = 48_000;
const WEBM_AUDIO_QUEUE_FRAMES: usize = 4_096;
static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Failure while preparing or writing a software export profile.
#[derive(Debug)]
pub struct ExportEncodeError(String);

impl std::fmt::Display for ExportEncodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ExportEncodeError {}

impl From<ffmpeg::Error> for ExportEncodeError {
    fn from(error: ffmpeg::Error) -> Self {
        Self(error.to_string())
    }
}

/// Stable software export profiles implemented by OR's linked FFmpeg runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoftwareExportProfile {
    /// Matroska with lossless FFV1 video and stereo PCM S16LE audio.
    MatroskaFfv1PcmS16le,
    /// WebM with VP9 video and stereo Opus audio.
    WebmVp9Opus,
}

/// Bounded-memory FFmpeg writer for the supported software export profiles.
///
/// Output is encoded in a private sibling staging directory and atomically moved
/// into place only after both streams and the container trailer are complete.
pub struct FfmpegSoftwareExportWriter {
    destination: PathBuf,
    staging_directory: PathBuf,
    staging_file: PathBuf,
    output: Option<format::context::Output>,
    video_encoder: VideoEncoder,
    audio_encoder: AudioEncoder,
    profile: SoftwareExportProfile,
    video_scaler: Option<scaling::Context>,
    video_stream: usize,
    audio_stream: usize,
    video_time_base: ffmpeg::Rational,
    audio_time_base: ffmpeg::Rational,
    audio_frame_size: usize,
    pending_audio: VecDeque<i16>,
    audio_frames_submitted: u64,
    audio_frames_encoded: u64,
    committed: bool,
}

/// Backwards-compatible name for callers that need the lossless profile.
pub type MatroskaFfv1PcmS16leWriter = FfmpegSoftwareExportWriter;

impl FfmpegSoftwareExportWriter {
    /// Opens a Matroska destination using FFV1 video and stereo PCM S16LE audio.
    pub fn create(
        destination: impl AsRef<Path>,
        width: u32,
        height: u32,
        frame_rate_numerator: u32,
        frame_rate_denominator: u32,
    ) -> Result<Self, ExportEncodeError> {
        Self::create_with_profile(
            destination,
            SoftwareExportProfile::MatroskaFfv1PcmS16le,
            width,
            height,
            frame_rate_numerator,
            frame_rate_denominator,
        )
    }

    pub fn create_with_profile(
        destination: impl AsRef<Path>,
        profile: SoftwareExportProfile,
        width: u32,
        height: u32,
        frame_rate_numerator: u32,
        frame_rate_denominator: u32,
    ) -> Result<Self, ExportEncodeError> {
        ffmpeg::init().map_err(ExportEncodeError::from)?;
        if width == 0 || height == 0 || frame_rate_numerator == 0 || frame_rate_denominator == 0 {
            return Err(ExportEncodeError(
                "export dimensions and frame rate must be nonzero".to_owned(),
            ));
        }
        if profile == SoftwareExportProfile::WebmVp9Opus
            && (!width.is_multiple_of(2) || !height.is_multiple_of(2))
        {
            return Err(ExportEncodeError(
                "WebM export requires even video dimensions".to_owned(),
            ));
        }
        let destination = destination.as_ref().to_path_buf();
        let staging_directory = create_staging_directory(&destination)?;
        let mut staging_guard = StagingDirectoryGuard::new(staging_directory.clone());
        let (container, staging_name, video_codec, audio_codec, pixel_format, video_scaler) =
            match profile {
                SoftwareExportProfile::MatroskaFfv1PcmS16le => (
                    "matroska",
                    "output.mkv",
                    codec::encoder::find(codec::Id::FFV1).ok_or_else(|| {
                        ExportEncodeError("linked FFmpeg has no FFV1 encoder".to_owned())
                    })?,
                    codec::encoder::find(codec::Id::PCM_S16LE).ok_or_else(|| {
                        ExportEncodeError("linked FFmpeg has no PCM S16LE encoder".to_owned())
                    })?,
                    format::Pixel::BGRA,
                    None,
                ),
                SoftwareExportProfile::WebmVp9Opus => (
                    "webm",
                    "output.webm",
                    codec::encoder::find_by_name("libvpx-vp9").ok_or_else(|| {
                        ExportEncodeError("linked FFmpeg has no libvpx VP9 encoder".to_owned())
                    })?,
                    codec::encoder::find_by_name("libopus").ok_or_else(|| {
                        ExportEncodeError("linked FFmpeg has no libopus encoder".to_owned())
                    })?,
                    format::Pixel::YUV420P,
                    Some(yuv420p_scaler(width, height)?),
                ),
            };
        let staging_file = staging_directory.join(staging_name);
        let mut output =
            format::output_as(&staging_file, container).map_err(ExportEncodeError::from)?;
        let global_header = output
            .format()
            .flags()
            .contains(format::Flags::GLOBAL_HEADER);
        let rate_num = i32::try_from(frame_rate_numerator)
            .map_err(|_| ExportEncodeError("sequence frame rate is out of range".to_owned()))?;
        let rate_den = i32::try_from(frame_rate_denominator)
            .map_err(|_| ExportEncodeError("sequence frame rate is out of range".to_owned()))?;
        let video_time_base = ffmpeg::Rational::new(rate_den, rate_num);
        let audio_time_base = ffmpeg::Rational::new(1, AUDIO_RATE as i32);

        let mut video_context = codec::context::Context::new_with_codec(video_codec)
            .encoder()
            .video()
            .map_err(ExportEncodeError::from)?;
        video_context.set_width(width);
        video_context.set_height(height);
        video_context.set_format(pixel_format);
        video_context.set_time_base(video_time_base);
        video_context.set_frame_rate(Some(ffmpeg::Rational::new(rate_num, rate_den)));
        if global_header {
            video_context.set_flags(codec::Flags::GLOBAL_HEADER);
        }
        let video_encoder = if profile == SoftwareExportProfile::WebmVp9Opus {
            video_context.set_colorspace(ffmpeg::color::Space::BT709);
            video_context.set_color_range(ffmpeg::color::Range::MPEG);
            unsafe {
                let context = video_context.as_mut_ptr();
                (*context).color_primaries = ffmpeg::color::Primaries::BT709.into();
                (*context).color_trc = ffmpeg::color::TransferCharacteristic::BT709.into();
            }
            let mut options = Dictionary::new();
            options.set("deadline", "realtime");
            options.set("cpu-used", "8");
            options.set("crf", "36");
            options.set("b", "0");
            options.set("row-mt", "1");
            video_context
                .open_as_with(video_codec, options)
                .map_err(ExportEncodeError::from)?
        } else {
            video_context
                .open_as(video_codec)
                .map_err(ExportEncodeError::from)?
        };

        let mut audio_context = codec::context::Context::new_with_codec(audio_codec)
            .encoder()
            .audio()
            .map_err(ExportEncodeError::from)?;
        audio_context.set_rate(AUDIO_RATE as i32);
        audio_context.set_format(Sample::I16(Type::Packed));
        audio_context.set_ch_layout(ffmpeg::ChannelLayout::STEREO);
        audio_context.set_time_base(audio_time_base);
        if profile == SoftwareExportProfile::WebmVp9Opus {
            audio_context.set_bit_rate(128_000);
        }
        if global_header {
            audio_context.set_flags(codec::Flags::GLOBAL_HEADER);
        }
        let audio_encoder = audio_context
            .open_as(audio_codec)
            .map_err(ExportEncodeError::from)?;
        let audio_frame_size = usize::try_from(audio_encoder.frame_size())
            .map_err(|_| ExportEncodeError("audio encoder frame size is invalid".to_owned()))?;
        if profile == SoftwareExportProfile::WebmVp9Opus
            && (audio_frame_size == 0 || audio_frame_size > AUDIO_RATE as usize)
        {
            return Err(ExportEncodeError(
                "libopus frame size is outside the bounded export limit".to_owned(),
            ));
        }

        let video_stream = {
            let mut stream = output
                .add_stream(video_codec)
                .map_err(ExportEncodeError::from)?;
            let index = stream.index();
            stream.set_time_base(video_time_base);
            stream.set_rate(ffmpeg::Rational::new(rate_num, rate_den));
            stream.set_avg_frame_rate(ffmpeg::Rational::new(rate_num, rate_den));
            stream.set_parameters(codec::Parameters::from(&video_encoder));
            index
        };

        let audio_stream = {
            let mut stream = output
                .add_stream(audio_codec)
                .map_err(ExportEncodeError::from)?;
            let index = stream.index();
            stream.set_time_base(audio_time_base);
            stream.set_parameters(codec::Parameters::from(&audio_encoder));
            index
        };
        output.write_header().map_err(ExportEncodeError::from)?;
        staging_guard.keep();

        Ok(Self {
            destination,
            staging_directory,
            staging_file,
            output: Some(output),
            video_encoder,
            audio_encoder,
            profile,
            video_scaler,
            video_stream,
            audio_stream,
            video_time_base,
            audio_time_base,
            audio_frame_size,
            pending_audio: VecDeque::new(),
            audio_frames_submitted: 0,
            audio_frames_encoded: 0,
            committed: false,
        })
    }

    /// Encodes one packed RGBA frame at its exact sequence frame index.
    pub fn write_video_frame(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        frame_index: u64,
    ) -> Result<(), ExportEncodeError> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| ExportEncodeError("video frame size overflowed".to_owned()))?;
        if rgba.len() != expected {
            return Err(ExportEncodeError(
                "rendered video frame has an invalid byte length".to_owned(),
            ));
        }
        let mut encoded = frame::Video::new(format::Pixel::BGRA, width, height);
        let stride = encoded.stride(0);
        let row_bytes = width as usize * 4;
        let destination = encoded.data_mut(0);
        for (source_row, target_row) in rgba
            .chunks_exact(row_bytes)
            .zip(destination.chunks_mut(stride))
            .take(height as usize)
        {
            let (source_pixels, _) = source_row.as_chunks::<4>();
            let (target_pixels, _) = target_row.as_chunks_mut::<4>();
            for (source, target) in source_pixels.iter().zip(target_pixels) {
                target.copy_from_slice(&[source[2], source[1], source[0], source[3]]);
            }
        }
        let mut encoded = if let Some(scaler) = &mut self.video_scaler {
            let mut converted = frame::Video::new(format::Pixel::YUV420P, width, height);
            scaler
                .run(&encoded, &mut converted)
                .map_err(ExportEncodeError::from)?;
            converted.set_color_space(ffmpeg::color::Space::BT709);
            converted.set_color_range(ffmpeg::color::Range::MPEG);
            converted.set_color_primaries(ffmpeg::color::Primaries::BT709);
            converted
                .set_color_transfer_characteristic(ffmpeg::color::TransferCharacteristic::BT709);
            converted
        } else {
            encoded
        };
        encoded.set_pts(Some(i64::try_from(frame_index).map_err(|_| {
            ExportEncodeError("video frame index exceeds the FFmpeg time base".to_owned())
        })?));
        self.video_encoder
            .send_frame(&encoded)
            .map_err(ExportEncodeError::from)?;
        self.drain_video_packets()
    }

    /// Encodes interleaved stereo samples at an exact 48 kHz sample-frame offset.
    pub fn write_audio_frames(
        &mut self,
        samples: &[i16],
        sample_frame_offset: u64,
    ) -> Result<(), ExportEncodeError> {
        if !samples.len().is_multiple_of(2) {
            return Err(ExportEncodeError(
                "stereo audio requires pairs of samples".to_owned(),
            ));
        }
        if samples.is_empty() {
            return Ok(());
        }
        let frame_count = samples.len() / 2;
        if self.profile == SoftwareExportProfile::WebmVp9Opus {
            if sample_frame_offset != self.audio_frames_submitted {
                return Err(ExportEncodeError(
                    "WebM audio samples must use consecutive sample-frame offsets".to_owned(),
                ));
            }
            self.audio_frames_submitted = self
                .audio_frames_submitted
                .checked_add(frame_count as u64)
                .ok_or_else(|| ExportEncodeError("audio sample count overflowed".to_owned()))?;
            for bounded_chunk in samples.chunks(WEBM_AUDIO_QUEUE_FRAMES * 2) {
                self.pending_audio.extend(bounded_chunk.iter().copied());
                while self.pending_audio.len() / 2 >= self.audio_frame_size {
                    self.encode_opus_frame(self.audio_frame_size)?;
                }
            }
            return Ok(());
        }
        let mut encoded = frame::Audio::new(
            Sample::I16(Type::Packed),
            frame_count,
            ffmpeg::ChannelLayoutMask::STEREO,
        );
        encoded.set_rate(AUDIO_RATE);
        let byte_count = std::mem::size_of_val(samples);
        let data = encoded.data_mut(0);
        if data.len() < byte_count {
            return Err(ExportEncodeError(
                "FFmpeg allocated a short PCM frame".to_owned(),
            ));
        }
        let (sample_bytes, _) = data[..byte_count].as_chunks_mut::<2>();
        for (sample, bytes) in samples.iter().zip(sample_bytes) {
            bytes.copy_from_slice(&sample.to_le_bytes());
        }
        encoded.set_pts(Some(i64::try_from(sample_frame_offset).map_err(|_| {
            ExportEncodeError("audio sample offset exceeds the FFmpeg time base".to_owned())
        })?));
        self.audio_encoder
            .send_frame(&encoded)
            .map_err(ExportEncodeError::from)?;
        self.drain_audio_packets()
    }

    fn encode_opus_frame(&mut self, frame_count: usize) -> Result<(), ExportEncodeError> {
        if frame_count == 0 || frame_count > self.audio_frame_size {
            return Err(ExportEncodeError(
                "Opus frame sample count is outside the encoder limit".to_owned(),
            ));
        }
        let sample_count = frame_count
            .checked_mul(2)
            .ok_or_else(|| ExportEncodeError("audio frame size overflowed".to_owned()))?;
        let mut encoded = frame::Audio::new(
            Sample::I16(Type::Packed),
            frame_count,
            ffmpeg::ChannelLayoutMask::STEREO,
        );
        encoded.set_rate(AUDIO_RATE);
        let data = encoded.data_mut(0);
        let byte_count = sample_count
            .checked_mul(std::mem::size_of::<i16>())
            .ok_or_else(|| ExportEncodeError("audio byte count overflowed".to_owned()))?;
        if data.len() < byte_count {
            return Err(ExportEncodeError(
                "FFmpeg allocated a short Opus frame".to_owned(),
            ));
        }
        let (sample_bytes, _) = data[..byte_count].as_chunks_mut::<2>();
        for bytes in sample_bytes {
            let sample = self.pending_audio.pop_front().ok_or_else(|| {
                ExportEncodeError("Opus input queue ended inside an audio frame".to_owned())
            })?;
            bytes.copy_from_slice(&sample.to_le_bytes());
        }
        encoded.set_pts(Some(i64::try_from(self.audio_frames_encoded).map_err(
            |_| ExportEncodeError("audio sample offset exceeds the output time base".to_owned()),
        )?));
        self.audio_frames_encoded = self
            .audio_frames_encoded
            .checked_add(frame_count as u64)
            .ok_or_else(|| ExportEncodeError("encoded audio sample count overflowed".to_owned()))?;
        self.audio_encoder
            .send_frame(&encoded)
            .map_err(ExportEncodeError::from)?;
        self.drain_audio_packets()
    }

    /// Flushes both codecs and atomically replaces the chosen destination.
    pub fn finish(mut self) -> Result<(), ExportEncodeError> {
        if self.profile == SoftwareExportProfile::WebmVp9Opus && !self.pending_audio.is_empty() {
            self.encode_opus_frame(self.pending_audio.len() / 2)?;
        }
        if self.profile == SoftwareExportProfile::WebmVp9Opus
            && self.audio_frames_encoded != self.audio_frames_submitted
        {
            return Err(ExportEncodeError(
                "Opus encoder sample count does not match submitted audio".to_owned(),
            ));
        }
        self.video_encoder
            .send_eof()
            .map_err(ExportEncodeError::from)?;
        self.drain_video_packets()?;
        self.audio_encoder
            .send_eof()
            .map_err(ExportEncodeError::from)?;
        self.drain_audio_packets()?;
        let mut output = self.output.take().expect("export output is open");
        output.write_trailer().map_err(ExportEncodeError::from)?;
        drop(output);
        or_core::replace_published_file(&self.staging_file, &self.destination).map_err(
            |error| ExportEncodeError(format!("could not publish the completed export: {error}")),
        )?;
        self.committed = true;
        let _ = fs::remove_dir(&self.staging_directory);
        Ok(())
    }

    fn drain_video_packets(&mut self) -> Result<(), ExportEncodeError> {
        loop {
            let mut packet = ffmpeg::Packet::empty();
            match self.video_encoder.receive_packet(&mut packet) {
                Ok(()) => {
                    packet.set_stream(self.video_stream);
                    packet.rescale_ts(
                        self.video_time_base,
                        stream_time_base(self.output.as_ref(), self.video_stream)?,
                    );
                    packet
                        .write_interleaved(self.output.as_mut().expect("export output is open"))
                        .map_err(ExportEncodeError::from)?;
                }
                Err(ffmpeg::Error::Eof) => break,
                Err(error) if is_again(error) => break,
                Err(error) => return Err(ExportEncodeError::from(error)),
            }
        }
        Ok(())
    }

    fn drain_audio_packets(&mut self) -> Result<(), ExportEncodeError> {
        loop {
            let mut packet = ffmpeg::Packet::empty();
            match self.audio_encoder.receive_packet(&mut packet) {
                Ok(()) => {
                    packet.set_stream(self.audio_stream);
                    packet.rescale_ts(
                        self.audio_time_base,
                        stream_time_base(self.output.as_ref(), self.audio_stream)?,
                    );
                    packet
                        .write_interleaved(self.output.as_mut().expect("export output is open"))
                        .map_err(ExportEncodeError::from)?;
                }
                Err(ffmpeg::Error::Eof) => break,
                Err(error) if is_again(error) => break,
                Err(error) => return Err(ExportEncodeError::from(error)),
            }
        }
        Ok(())
    }
}

fn yuv420p_scaler(width: u32, height: u32) -> Result<scaling::Context, ExportEncodeError> {
    let mut scaler = scaling::Context::get(
        format::Pixel::BGRA,
        width,
        height,
        format::Pixel::YUV420P,
        width,
        height,
        scaling::flag::Flags::BILINEAR,
    )
    .map_err(ExportEncodeError::from)?;
    let coefficients = unsafe { ffmpeg::ffi::sws_getCoefficients(ffmpeg::ffi::SWS_CS_ITU709) };
    if coefficients.is_null() {
        return Err(ExportEncodeError(
            "FFmpeg has no BT.709 conversion coefficients".to_owned(),
        ));
    }
    // Renderer output is full-range RGB; WebM SDR output uses BT.709 studio-range YUV.
    let status = unsafe {
        ffmpeg::ffi::sws_setColorspaceDetails(
            scaler.as_mut_ptr(),
            coefficients,
            1,
            coefficients,
            0,
            0,
            1 << 16,
            1 << 16,
        )
    };
    if status < 0 {
        return Err(ExportEncodeError(format!(
            "FFmpeg could not configure BT.709 conversion ({status})"
        )));
    }
    Ok(scaler)
}

impl Drop for FfmpegSoftwareExportWriter {
    fn drop(&mut self) {
        if !self.committed {
            self.output.take();
            let _ = fs::remove_file(&self.staging_file);
            let _ = fs::remove_dir(&self.staging_directory);
        }
    }
}

fn stream_time_base(
    output: Option<&format::context::Output>,
    stream_index: usize,
) -> Result<ffmpeg::Rational, ExportEncodeError> {
    output
        .and_then(|output| output.stream(stream_index))
        .map(|stream| stream.time_base())
        .ok_or_else(|| ExportEncodeError("FFmpeg output stream disappeared".to_owned()))
}

fn create_staging_directory(destination: &Path) -> Result<PathBuf, ExportEncodeError> {
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = destination
        .file_name()
        .ok_or_else(|| ExportEncodeError("export destination has no file name".to_owned()))?;
    for _ in 0..100 {
        let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            ".{}.or-export-{}-{sequence}",
            file_name.to_string_lossy(),
            std::process::id()
        );
        let directory = parent.join(name);
        match create_private_directory(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(ExportEncodeError(format!(
                    "could not create export staging directory: {error}"
                )));
            }
        }
    }
    Err(ExportEncodeError(
        "could not allocate a unique export staging directory".to_owned(),
    ))
}

struct StagingDirectoryGuard {
    directory: Option<PathBuf>,
}

impl StagingDirectoryGuard {
    fn new(directory: PathBuf) -> Self {
        Self {
            directory: Some(directory),
        }
    }

    fn keep(&mut self) {
        self.directory = None;
    }
}

impl Drop for StagingDirectoryGuard {
    fn drop(&mut self) {
        if let Some(directory) = self.directory.take() {
            let _ = fs::remove_dir_all(directory);
        }
    }
}

#[cfg(unix)]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    fs::create_dir(path)
}

fn is_again(error: ffmpeg::Error) -> bool {
    let ffmpeg::Error::Other { errno } = error else {
        return false;
    };
    #[cfg(windows)]
    {
        errno == 11
    }
    #[cfg(not(windows))]
    {
        std::io::Error::from_raw_os_error(errno).kind() == ErrorKind::WouldBlock
    }
}
