use ffmpeg::{
    codec::{
        self,
        encoder::{audio::Encoder as AudioEncoder, video::Encoder as VideoEncoder},
    },
    format::{self, Sample},
    frame,
    util::format::sample::Type,
};
use ffmpeg_the_third as ffmpeg;
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const AUDIO_RATE: u32 = 48_000;
static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Failure while preparing or writing the fixed Matroska/FFV1/PCM S16LE profile.
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

/// Bounded-memory FFmpeg writer for the mandatory software export profile.
///
/// Output is encoded in a private sibling staging directory and atomically moved
/// into place only after both streams and the Matroska trailer are complete.
pub struct MatroskaFfv1PcmS16leWriter {
    destination: PathBuf,
    staging_directory: PathBuf,
    staging_file: PathBuf,
    output: Option<format::context::Output>,
    video_encoder: VideoEncoder,
    audio_encoder: AudioEncoder,
    video_stream: usize,
    audio_stream: usize,
    video_time_base: ffmpeg::Rational,
    audio_time_base: ffmpeg::Rational,
    committed: bool,
}

impl MatroskaFfv1PcmS16leWriter {
    /// Opens a Matroska destination using FFV1 video and stereo PCM S16LE audio.
    pub fn create(
        destination: impl AsRef<Path>,
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
        let destination = destination.as_ref().to_path_buf();
        let staging_directory = create_staging_directory(&destination)?;
        let mut staging_guard = StagingDirectoryGuard::new(staging_directory.clone());
        let staging_file = staging_directory.join("output.mkv");
        let mut output =
            format::output_as(&staging_file, "matroska").map_err(ExportEncodeError::from)?;
        let global_header = output
            .format()
            .flags()
            .contains(format::Flags::GLOBAL_HEADER);
        let video_codec = codec::encoder::find(codec::Id::FFV1)
            .ok_or_else(|| ExportEncodeError("linked FFmpeg has no FFV1 encoder".to_owned()))?;
        let audio_codec = codec::encoder::find(codec::Id::PCM_S16LE).ok_or_else(|| {
            ExportEncodeError("linked FFmpeg has no PCM S16LE encoder".to_owned())
        })?;
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
        video_context.set_format(format::Pixel::BGRA);
        video_context.set_time_base(video_time_base);
        video_context.set_frame_rate(Some(ffmpeg::Rational::new(rate_num, rate_den)));
        if global_header {
            video_context.set_flags(codec::Flags::GLOBAL_HEADER);
        }
        let video_encoder = video_context
            .open_as(video_codec)
            .map_err(ExportEncodeError::from)?;

        let mut audio_context = codec::context::Context::new_with_codec(audio_codec)
            .encoder()
            .audio()
            .map_err(ExportEncodeError::from)?;
        audio_context.set_rate(AUDIO_RATE as i32);
        audio_context.set_format(Sample::I16(Type::Packed));
        audio_context.set_ch_layout(ffmpeg::ChannelLayout::STEREO);
        audio_context.set_time_base(audio_time_base);
        if global_header {
            audio_context.set_flags(codec::Flags::GLOBAL_HEADER);
        }
        let audio_encoder = audio_context
            .open_as(audio_codec)
            .map_err(ExportEncodeError::from)?;

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
            video_stream,
            audio_stream,
            video_time_base,
            audio_time_base,
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

    /// Flushes both codecs and atomically replaces the chosen destination.
    pub fn finish(mut self) -> Result<(), ExportEncodeError> {
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
        fs::rename(&self.staging_file, &self.destination).map_err(|error| {
            ExportEncodeError(format!("could not publish the completed export: {error}"))
        })?;
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

impl Drop for MatroskaFfv1PcmS16leWriter {
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
