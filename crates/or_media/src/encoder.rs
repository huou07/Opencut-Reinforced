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
use or_core::ExportQuality;
use std::{
    collections::VecDeque,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const AUDIO_RATE: u32 = 48_000;
const COMPRESSED_AUDIO_QUEUE_FRAMES: usize = 4_096;
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

/// Export profiles implemented by OR's linked FFmpeg runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FfmpegExportProfile {
    /// Matroska with lossless FFV1 video and stereo PCM S16LE audio.
    MatroskaFfv1PcmS16le,
    /// WebM with VP9 video and stereo Opus audio.
    WebmVp9Opus,
    /// MP4 with Apple VideoToolbox H.264 video and AAC audio.
    Mp4H264Aac,
}

/// Compatibility name for the previous software-only profile type.
pub type SoftwareExportProfile = FfmpegExportProfile;

/// Bounded-memory FFmpeg writer for the supported export profiles.
///
/// Output is encoded in a private sibling staging directory and atomically moved
/// into place only after both streams and the container trailer are complete.
pub struct FfmpegExportWriter {
    destination: PathBuf,
    staging_directory: PathBuf,
    staging_file: PathBuf,
    output: Option<format::context::Output>,
    video_encoder: VideoEncoder,
    audio_encoder: AudioEncoder,
    profile: FfmpegExportProfile,
    pixel_format: format::Pixel,
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

/// Compatibility names retained for existing callers.
pub type FfmpegSoftwareExportWriter = FfmpegExportWriter;
pub type MatroskaFfv1PcmS16leWriter = FfmpegExportWriter;

impl FfmpegExportWriter {
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
            FfmpegExportProfile::MatroskaFfv1PcmS16le,
            width,
            height,
            frame_rate_numerator,
            frame_rate_denominator,
        )
    }

    pub fn create_with_profile(
        destination: impl AsRef<Path>,
        profile: FfmpegExportProfile,
        width: u32,
        height: u32,
        frame_rate_numerator: u32,
        frame_rate_denominator: u32,
    ) -> Result<Self, ExportEncodeError> {
        Self::create_with_profile_and_quality(
            destination,
            profile,
            ExportQuality::Quality,
            width,
            height,
            frame_rate_numerator,
            frame_rate_denominator,
        )
    }

    /// Opens a destination with an explicit quality preference.
    ///
    /// `Quality` spends encoding time and bytes on a cleaner picture; `Speed`
    /// spends as little as possible. A lossless profile ignores the preference
    /// because it has no rate control to trade.
    #[allow(clippy::too_many_arguments)]
    pub fn create_with_profile_and_quality(
        destination: impl AsRef<Path>,
        profile: FfmpegExportProfile,
        quality: ExportQuality,
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
        if matches!(
            profile,
            FfmpegExportProfile::WebmVp9Opus | FfmpegExportProfile::Mp4H264Aac
        ) && (!width.is_multiple_of(2) || !height.is_multiple_of(2))
        {
            return Err(ExportEncodeError(
                "WebM and MP4 export require even video dimensions".to_owned(),
            ));
        }
        let destination = destination.as_ref().to_path_buf();
        if destination.is_dir() {
            return Err(ExportEncodeError(
                "Export destination is a folder. Choose a file name and try again.".to_owned(),
            ));
        }
        let staging_directory = create_staging_directory(&destination)?;
        let mut staging_guard = StagingDirectoryGuard::new(staging_directory.clone());
        let (container, staging_name, video_codec, audio_codec, pixel_format, video_scaler) =
            match profile {
                FfmpegExportProfile::MatroskaFfv1PcmS16le => (
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
                FfmpegExportProfile::WebmVp9Opus => (
                    "webm",
                    "output.webm",
                    codec::encoder::find_by_name("libvpx-vp9").ok_or_else(|| {
                        ExportEncodeError("linked FFmpeg has no libvpx VP9 encoder".to_owned())
                    })?,
                    codec::encoder::find_by_name("libopus").ok_or_else(|| {
                        ExportEncodeError("linked FFmpeg has no libopus encoder".to_owned())
                    })?,
                    format::Pixel::YUV420P,
                    Some(yuv_scaler(width, height, format::Pixel::YUV420P)?),
                ),
                FfmpegExportProfile::Mp4H264Aac => (
                    "mov",
                    "output.mp4",
                    codec::encoder::find_by_name("h264_videotoolbox").ok_or_else(|| {
                        ExportEncodeError(
                            "linked FFmpeg has no VideoToolbox H.264 encoder".to_owned(),
                        )
                    })?,
                    codec::encoder::find(codec::Id::AAC).ok_or_else(|| {
                        ExportEncodeError("linked FFmpeg has no AAC encoder".to_owned())
                    })?,
                    // NV12 is what the hardware encoder takes, so a caller with
                    // planes from the GPU needs no conversion at all.
                    format::Pixel::NV12,
                    Some(yuv_scaler(width, height, format::Pixel::NV12)?),
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
        let video_encoder = if matches!(
            profile,
            FfmpegExportProfile::WebmVp9Opus | FfmpegExportProfile::Mp4H264Aac
        ) {
            video_context.set_colorspace(ffmpeg::color::Space::BT709);
            video_context.set_color_range(ffmpeg::color::Range::MPEG);
            unsafe {
                let context = video_context.as_mut_ptr();
                (*context).color_primaries = ffmpeg::color::Primaries::BT709.into();
                (*context).color_trc = ffmpeg::color::TransferCharacteristic::BT709.into();
            }
            let mut options = Dictionary::new();
            if profile == FfmpegExportProfile::WebmVp9Opus {
                let (deadline, cpu_used, crf) = vp9_settings(quality);
                options.set("deadline", deadline);
                options.set("cpu-used", cpu_used);
                options.set("crf", crf);
                options.set("b", "0");
                options.set("row-mt", "1");
            } else {
                options.set("b", video_bitrate(width, height, quality).to_string());
            }
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
        let audio_format = if profile == FfmpegExportProfile::Mp4H264Aac {
            Sample::F32(Type::Planar)
        } else {
            Sample::I16(Type::Packed)
        };
        audio_context.set_format(audio_format);
        audio_context.set_ch_layout(ffmpeg::ChannelLayout::STEREO);
        audio_context.set_time_base(audio_time_base);
        if profile == FfmpegExportProfile::WebmVp9Opus {
            audio_context.set_bit_rate(128_000);
        } else if profile == FfmpegExportProfile::Mp4H264Aac {
            audio_context.set_bit_rate(192_000);
        }
        if global_header {
            audio_context.set_flags(codec::Flags::GLOBAL_HEADER);
        }
        let audio_encoder = audio_context
            .open_as(audio_codec)
            .map_err(ExportEncodeError::from)?;
        let audio_frame_size = usize::try_from(audio_encoder.frame_size())
            .map_err(|_| ExportEncodeError("audio encoder frame size is invalid".to_owned()))?;
        if matches!(
            profile,
            FfmpegExportProfile::WebmVp9Opus | FfmpegExportProfile::Mp4H264Aac
        ) && (audio_frame_size == 0 || audio_frame_size > AUDIO_RATE as usize)
        {
            return Err(ExportEncodeError(
                "compressed audio frame size is outside the bounded export limit".to_owned(),
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
            pixel_format,
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
            let mut converted = frame::Video::new(self.pixel_format, width, height);
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

    /// Encodes one NV12 picture from GPU-produced planes at its exact index.
    ///
    /// Only a profile whose encoder takes NV12 accepts this, and then no colour
    /// conversion runs on the CPU: the composited frame arrives as the luma and
    /// interleaved chroma planes the hardware encoder wants.
    pub fn write_video_frame_nv12(
        &mut self,
        luma: &[u8],
        chroma: &[u8],
        width: u32,
        height: u32,
        frame_index: u64,
    ) -> Result<(), ExportEncodeError> {
        if self.pixel_format != format::Pixel::NV12 {
            return Err(ExportEncodeError(
                "this export profile does not take NV12 frames".to_owned(),
            ));
        }
        if !width.is_multiple_of(2) || !height.is_multiple_of(2) {
            return Err(ExportEncodeError(
                "NV12 frames need even dimensions".to_owned(),
            ));
        }
        let luma_len = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| ExportEncodeError("video frame size overflowed".to_owned()))?;
        let chroma_len = (width as usize / 2)
            .checked_mul(height as usize / 2)
            .and_then(|pairs| pairs.checked_mul(2))
            .ok_or_else(|| ExportEncodeError("video frame size overflowed".to_owned()))?;
        if luma.len() != luma_len || chroma.len() != chroma_len {
            return Err(ExportEncodeError(
                "rendered NV12 frame has an invalid byte length".to_owned(),
            ));
        }
        let mut encoded = frame::Video::new(format::Pixel::NV12, width, height);
        copy_plane(&mut encoded, 0, luma, width as usize, height);
        copy_plane(&mut encoded, 1, chroma, width as usize, height / 2);
        encoded.set_color_space(ffmpeg::color::Space::BT709);
        encoded.set_color_range(ffmpeg::color::Range::MPEG);
        encoded.set_color_primaries(ffmpeg::color::Primaries::BT709);
        encoded.set_color_transfer_characteristic(ffmpeg::color::TransferCharacteristic::BT709);
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
        if matches!(
            self.profile,
            FfmpegExportProfile::WebmVp9Opus | FfmpegExportProfile::Mp4H264Aac
        ) {
            if sample_frame_offset != self.audio_frames_submitted {
                return Err(ExportEncodeError(
                    "compressed audio samples must use consecutive sample-frame offsets".to_owned(),
                ));
            }
            self.audio_frames_submitted = self
                .audio_frames_submitted
                .checked_add(frame_count as u64)
                .ok_or_else(|| ExportEncodeError("audio sample count overflowed".to_owned()))?;
            for bounded_chunk in samples.chunks(COMPRESSED_AUDIO_QUEUE_FRAMES * 2) {
                self.pending_audio.extend(bounded_chunk.iter().copied());
                while self.pending_audio.len() / 2 >= self.audio_frame_size {
                    if self.profile == FfmpegExportProfile::WebmVp9Opus {
                        self.encode_opus_frame(self.audio_frame_size)?;
                    } else {
                        self.encode_aac_frame(self.audio_frame_size)?;
                    }
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

    fn encode_aac_frame(&mut self, frame_count: usize) -> Result<(), ExportEncodeError> {
        if frame_count == 0 || frame_count != self.audio_frame_size {
            return Err(ExportEncodeError(
                "AAC input must contain one complete encoder frame".to_owned(),
            ));
        }
        let plane_bytes = frame_count
            .checked_mul(std::mem::size_of::<f32>())
            .ok_or_else(|| ExportEncodeError("AAC audio frame size overflowed".to_owned()))?;
        let mut encoded = frame::Audio::new(
            Sample::F32(Type::Planar),
            frame_count,
            ffmpeg::ChannelLayoutMask::STEREO,
        );
        encoded.set_rate(AUDIO_RATE);
        if encoded.data(0).len() < plane_bytes {
            return Err(ExportEncodeError(
                "FFmpeg allocated a short AAC frame".to_owned(),
            ));
        }
        let mut right_plane = Vec::with_capacity(frame_count);
        for sample in encoded.plane_mut::<f32>(0) {
            let left_sample = self.pending_audio.pop_front().ok_or_else(|| {
                ExportEncodeError("AAC input queue ended inside an encoder frame".to_owned())
            })?;
            let right_sample = self.pending_audio.pop_front().ok_or_else(|| {
                ExportEncodeError("AAC input queue ended inside an encoder frame".to_owned())
            })?;
            *sample = f32::from(left_sample) / 32768.0;
            right_plane.push(f32::from(right_sample) / 32768.0);
        }
        encoded.plane_mut::<f32>(1).copy_from_slice(&right_plane);
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
        if self.profile == FfmpegExportProfile::WebmVp9Opus && !self.pending_audio.is_empty() {
            self.encode_opus_frame(self.pending_audio.len() / 2)?;
        }
        if self.profile == FfmpegExportProfile::Mp4H264Aac && !self.pending_audio.is_empty() {
            let missing_samples = self.audio_frame_size - self.pending_audio.len() / 2;
            for _ in 0..missing_samples * 2 {
                self.pending_audio.push_back(0);
            }
            self.encode_aac_frame(self.audio_frame_size)?;
        }
        if self.profile == FfmpegExportProfile::WebmVp9Opus
            && self.audio_frames_encoded != self.audio_frames_submitted
        {
            return Err(ExportEncodeError(
                "Opus encoder sample count does not match submitted audio".to_owned(),
            ));
        }
        if self.profile == FfmpegExportProfile::Mp4H264Aac
            && (self.audio_frames_encoded < self.audio_frames_submitted
                || self.audio_frames_encoded - self.audio_frames_submitted
                    >= self.audio_frame_size as u64)
        {
            return Err(ExportEncodeError(
                "AAC encoder sample count does not cover the submitted audio".to_owned(),
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

/// VP9 rate control for a quality preference: `(deadline, cpu-used, crf)`.
///
/// `cpu-used` 8 with `deadline=realtime` is libvpx's cheapest mode and visibly
/// softer; `cpu-used` 4 with `deadline=good` costs several times the encoding
/// time and measurably better pictures at a similar size.
const fn vp9_settings(quality: ExportQuality) -> (&'static str, &'static str, &'static str) {
    match quality {
        ExportQuality::Quality => ("good", "4", "32"),
        ExportQuality::Speed => ("realtime", "8", "36"),
    }
}

/// Video bitrate for the hardware H.264 profile, scaled by the quality preference.
///
/// The base rate stays inside the bounds the profile always had; `Quality` spends
/// half again as much on the picture without leaving those bounds.
fn video_bitrate(width: u32, height: u32, quality: ExportQuality) -> u64 {
    let base = (u64::from(width) * u64::from(height) * 3).clamp(500_000, 12_000_000);
    match quality {
        ExportQuality::Quality => (base + base / 2).clamp(500_000, 12_000_000),
        ExportQuality::Speed => base,
    }
}

/// Copies one tightly-packed plane into an FFmpeg frame plane, honouring the
/// destination stride.
fn copy_plane(frame: &mut frame::Video, plane: usize, source: &[u8], row_bytes: usize, rows: u32) {
    let stride = frame.stride(plane);
    let data = frame.data_mut(plane);
    for (source_row, target_row) in source
        .chunks_exact(row_bytes)
        .zip(data.chunks_mut(stride))
        .take(rows as usize)
    {
        target_row[..row_bytes].copy_from_slice(source_row);
    }
}

fn yuv_scaler(
    width: u32,
    height: u32,
    destination: format::Pixel,
) -> Result<scaling::Context, ExportEncodeError> {
    let mut scaler = scaling::Context::get(
        format::Pixel::BGRA,
        width,
        height,
        destination,
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

#[cfg(test)]
mod quality_tests {
    use super::{ExportQuality, video_bitrate, vp9_settings};

    #[test]
    fn quality_spends_more_encoding_time_and_bytes_than_speed() {
        let (deadline, cpu_used, crf) = vp9_settings(ExportQuality::Speed);
        assert_eq!((deadline, cpu_used, crf), ("realtime", "8", "36"));
        let (deadline, cpu_used, crf) = vp9_settings(ExportQuality::Quality);
        assert_eq!((deadline, cpu_used, crf), ("good", "4", "32"));
        // Lower `cpu-used` is slower and better; lower `crf` is better and larger.
        assert!(cpu_used.parse::<u8>().unwrap() < "8".parse::<u8>().unwrap());
        assert!(crf.parse::<u32>().unwrap() < "36".parse::<u32>().unwrap());
    }

    #[test]
    fn the_hardware_profile_spends_half_again_as_much_on_quality() {
        let speed = video_bitrate(1920, 1080, ExportQuality::Speed);
        let quality = video_bitrate(1920, 1080, ExportQuality::Quality);
        assert!(quality > speed);
        assert_eq!(quality, (speed + speed / 2).min(12_000_000));
        // The bounds the profile always had still hold at both extremes.
        assert!((500_000..=12_000_000).contains(&video_bitrate(16, 16, ExportQuality::Speed)));
        assert!((500_000..=12_000_000).contains(&video_bitrate(
            7680,
            4320,
            ExportQuality::Quality
        )));
    }
}
