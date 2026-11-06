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

/// H.264 encoders to try, best first.
///
/// The desktop packages enable a different subset on each platform — Apple
/// VideoToolbox on macOS, Media Foundation on Windows — so an export profile
/// must never name one encoder and hope every build carries it. The list is a
/// preference order over what is actually compiled in, which keeps the MP4
/// profile working on every platform the product ships.
/// The most encoder threads a WebM export asks libvpx for. The encoder is
/// memory hungry, so a wide host is served by row threading rather than by
/// spinning up every core.
const VP9_MAX_THREADS: usize = 16;

/// Hardware first, then the operating system's own encoder, then software.
///
/// A package can carry an encoder the machine cannot drive — NVENC in a Linux
/// build on an AMD box — so presence alone is not evidence that an export will
/// start. Every candidate is opened on a tiny frame before the picker commits,
/// which is the same call the export makes, so the choice is a working encoder
/// rather than a name in a list.
///
/// VAAPI is the one candidate that reads GPU surfaces instead of a plain NV12
/// frame, so it is probed by building its frame pool and running one frame
/// through the encoder. QSV and AMF stay out: QSV needs the oneVPL dispatcher
/// on top of a VAAPI device, AMF needs AMD's headers, and on Windows Media
/// Foundation already drives the same hardware for both vendors.
const H264_ENCODER_CANDIDATES: &[&str] = &[
    "h264_videotoolbox",
    "h264_nvenc",
    "h264_mf",
    "h264_vaapi",
    "h264_open",
    "libx264",
];

/// Surfaces the VAAPI pool holds before it has to grow. One frame is in
/// flight per rendered frame and the encoder may hold a few, so a short runway
/// keeps the upload from stalling on allocation.
const HARDWARE_POOL_SURFACES: i32 = 20;

/// The MP4 encoder the picker settled on, and whether it reads GPU surfaces.
#[derive(Clone, Copy)]
struct H264Pick {
    encoder: codec::Codec,
    vaapi: bool,
}

/// A VAAPI device and the NV12 surface pool built on it.
///
/// The encoder reads from this pool, so the writer uploads every rendered frame
/// into it. The handles are owned by this value and released on drop; the
/// encoder keeps its own reference to the pool.
struct HardwareFrames {
    device: *mut ffmpeg::sys::AVBufferRef,
    frames: *mut ffmpeg::sys::AVBufferRef,
}

impl HardwareFrames {
    /// Opens the driver and allocates the surface pool.
    fn vaapi(width: u32, height: u32) -> Result<Self, ExportEncodeError> {
        if !cfg!(target_os = "linux") {
            return Err(ExportEncodeError(
                "VAAPI is only available in the Linux package".to_owned(),
            ));
        }
        let width = i32::try_from(width)
            .map_err(|_| ExportEncodeError("export width is out of range".to_owned()))?;
        let height = i32::try_from(height)
            .map_err(|_| ExportEncodeError("export height is out of range".to_owned()))?;
        // SAFETY: the two references are created here and released together in
        // `Drop`; FFmpeg owns the underlying contexts.
        unsafe {
            let mut device: *mut ffmpeg::sys::AVBufferRef = std::ptr::null_mut();
            let status = ffmpeg::sys::av_hwdevice_ctx_create(
                &mut device,
                ffmpeg::sys::AVHWDeviceType::VAAPI,
                std::ptr::null(),
                std::ptr::null_mut(),
                0,
            );
            if status < 0 || device.is_null() {
                return Err(ExportEncodeError(
                    "this machine has no VAAPI device to encode with".to_owned(),
                ));
            }
            let frames = ffmpeg::sys::av_hwframe_ctx_alloc(device);
            if frames.is_null() {
                ffmpeg::sys::av_buffer_unref(&mut device);
                return Err(ExportEncodeError(
                    "VAAPI surface pool allocation failed".to_owned(),
                ));
            }
            let context = (*frames).data.cast::<ffmpeg::sys::AVHWFramesContext>();
            (*context).format = format::Pixel::VAAPI.into();
            (*context).sw_format = format::Pixel::NV12.into();
            (*context).width = width;
            (*context).height = height;
            (*context).initial_pool_size = HARDWARE_POOL_SURFACES;
            let status = ffmpeg::sys::av_hwframe_ctx_init(frames);
            if status < 0 {
                let mut frames = frames;
                let mut device = device;
                ffmpeg::sys::av_buffer_unref(&mut frames);
                ffmpeg::sys::av_buffer_unref(&mut device);
                return Err(ExportEncodeError(
                    "the VAAPI driver refused the surface pool".to_owned(),
                ));
            }
            Ok(Self { device, frames })
        }
    }

    /// Points the encoder at this pool. Called before the encoder opens.
    ///
    /// SAFETY: `context` must be an encoder context that has not been opened
    /// yet, and the pool must outlive it.
    unsafe fn attach(&self, context: *mut ffmpeg::sys::AVCodecContext) {
        unsafe {
            (*context).pix_fmt = format::Pixel::VAAPI.into();
            (*context).hw_frames_ctx = ffmpeg::sys::av_buffer_ref(self.frames);
        }
    }

    /// Uploads one software frame into the pool and sends it to the encoder.
    ///
    /// SAFETY: `context` must be the opened encoder that owns this pool, and
    /// `software` must be a live NV12 frame.
    unsafe fn send(
        &self,
        context: *mut ffmpeg::sys::AVCodecContext,
        software: *const ffmpeg::sys::AVFrame,
    ) -> Result<(), ExportEncodeError> {
        unsafe {
            let hardware = ffmpeg::sys::av_frame_alloc();
            if hardware.is_null() {
                return Err(ExportEncodeError(
                    "VAAPI frame allocation failed".to_owned(),
                ));
            }
            (*hardware).format = ffmpeg::sys::AVPixelFormat::VAAPI.0;
            (*hardware).width = (*software).width;
            (*hardware).height = (*software).height;
            let mut status = ffmpeg::sys::av_hwframe_get_buffer(self.frames, hardware, 0);
            if status < 0 {
                let mut hardware = hardware;
                ffmpeg::sys::av_frame_free(&mut hardware);
                return Err(ExportEncodeError::from(ffmpeg::Error::from(status)));
            }
            (*hardware).pts = (*software).pts;
            status = ffmpeg::sys::av_hwframe_transfer_data(hardware, software as *mut _, 0);
            if status < 0 {
                let mut hardware = hardware;
                ffmpeg::sys::av_frame_free(&mut hardware);
                return Err(ExportEncodeError::from(ffmpeg::Error::from(status)));
            }
            status = ffmpeg::sys::avcodec_send_frame(context, hardware);
            let mut hardware = hardware;
            ffmpeg::sys::av_frame_free(&mut hardware);
            if status < 0 {
                return Err(ExportEncodeError::from(ffmpeg::Error::from(status)));
            }
            Ok(())
        }
    }
}

impl Drop for HardwareFrames {
    fn drop(&mut self) {
        // SAFETY: both handles were created by FFmpeg for this value and are
        // released exactly once.
        unsafe {
            ffmpeg::sys::av_buffer_unref(&mut self.frames);
            ffmpeg::sys::av_buffer_unref(&mut self.device);
        }
    }
}

// SAFETY: the handles are plain pointers to FFmpeg reference-counted buffers;
// moving the owning value between threads does not share mutable access.
unsafe impl Send for HardwareFrames {}

/// The encoders that need a device driver the build cannot see. NVENC loads the
/// driver's library at run time, so a build always carries it and only the
/// machine decides whether it works.
fn hardware_device_type(name: &str) -> Option<ffmpeg::sys::AVHWDeviceType> {
    use ffmpeg::sys::AVHWDeviceType;
    match name {
        "h264_nvenc" => Some(AVHWDeviceType::CUDA),
        _ => None,
    }
}

/// Whether this machine can drive a hardware encoder. Creating the device
/// context is the same step the encoder does at open, so a refusal here is the
/// refusal the export would hit, and the picker moves on instead of failing.
fn hardware_device_available(name: &str) -> bool {
    let Some(device_type) = hardware_device_type(name) else {
        return true;
    };
    unsafe {
        let mut device: *mut ffmpeg::sys::AVBufferRef = std::ptr::null_mut();
        let status = ffmpeg::sys::av_hwdevice_ctx_create(
            &mut device,
            device_type,
            std::ptr::null(),
            std::ptr::null_mut(),
            0,
        );
        let available = status >= 0 && !device.is_null();
        if !device.is_null() {
            ffmpeg::sys::av_buffer_unref(&mut device);
        }
        available
    }
}

/// Whether this machine can encode H.264 through VAAPI.
///
/// A driver can advertise the profile and still refuse the encode, so the probe
/// builds the real pool and pushes one frame through the real encoder. Nothing
/// is written: the frame never reaches a muxer.
fn vaapi_answers() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    let Some(encoder) = codec::encoder::find_by_name("h264_vaapi") else {
        return false;
    };
    let Ok(pool) = HardwareFrames::vaapi(64, 64) else {
        return false;
    };
    let Ok(mut context) = codec::context::Context::new_with_codec(encoder)
        .encoder()
        .video()
    else {
        return false;
    };
    context.set_width(64);
    context.set_height(64);
    context.set_time_base(ffmpeg::Rational::new(1, 30));
    context.set_frame_rate(Some(ffmpeg::Rational::new(30, 1)));
    // SAFETY: the context is not open yet and the pool outlives it here.
    unsafe { pool.attach(context.as_mut_ptr()) };
    let Ok(mut encoder) = context.open_as_with(encoder, Dictionary::new()) else {
        return false;
    };
    let mut software = frame::Video::new(format::Pixel::NV12, 64, 64);
    software.set_pts(Some(0));
    // SAFETY: the encoder is open on this pool and the frame is a live NV12
    // frame of the same size.
    unsafe { pool.send(encoder.as_mut_ptr(), software.as_ptr()).is_ok() }
}

/// Whether an encoder opens on a plain frame. Media Foundation, VideoToolbox
/// and NVENC all take an ordinary software frame, so a test open is the honest
/// check that this process can encode with them.
fn encoder_answers(encoder: codec::Codec) -> bool {
    let Ok(mut context) = codec::context::Context::new_with_codec(encoder)
        .encoder()
        .video()
    else {
        return false;
    };
    context.set_width(64);
    context.set_height(64);
    context.set_format(format::Pixel::NV12);
    context.set_time_base(ffmpeg::Rational::new(1, 30));
    context.set_frame_rate(Some(ffmpeg::Rational::new(30, 1)));
    context.open().is_ok()
}

/// Opens the encoder a plan selected, or fails with the gap the plan found.
fn pick_h264_encoder(
    plan: &or_core::ExportEncodingPlan,
) -> Result<H264Pick, ExportEncodeError> {
    let Some(name) = plan.h264_encoder.as_deref() else {
        let mut message = format!(
            "this machine has no usable H.264 encoder (looked for {}), so MP4 \
             cannot be encoded here. Export WebM instead, or install a package \
             that links a hardware H.264 encoder for this platform.",
            H264_ENCODER_CANDIDATES.join(", ")
        );
        if !plan.notes.is_empty() {
            message.push_str(" Probes: ");
            message.push_str(&plan.notes.join("; "));
        }
        return Err(ExportEncodeError(message));
    };
    let encoder = codec::encoder::find_by_name(name).ok_or_else(|| {
        ExportEncodeError(format!(
            "internal: the plan chose {name}, which this build does not carry"
        ))
    })?;
    Ok(H264Pick {
        encoder,
        vaapi: name == "h264_vaapi",
    })
}

/// Turns an H.264 open failure into a sentence the choosing writer can act on.
///
/// A build that carries no usable H.264 encoder answers "Invalid argument" from
/// the codec context. That string names neither the format nor the way out, and
/// the format choice is exactly what the person in front of the dialog owns.
fn h264_open_error(encoder: &str, error: ffmpeg::Error) -> ExportEncodeError {
    ExportEncodeError(format!(
        "MP4 could not be encoded here: the {encoder} encoder did not open \
         ({error}). This machine has no working H.264 encoder; export WebM or \
         Matroska instead."
    ))
}

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
    /// MP4 with H.264 video and AAC audio, encoded by the best encoder this
    /// build carries on this host (VideoToolbox on macOS, Media Foundation on
    /// Windows, a vendor encoder when one is linked).
    Mp4H264Aac,
}

/// What this build can actually offer from the Export dialog.
///
/// Presence is measured the same way the writer measures it — `avcodec` lookup
/// by the very names the profile code uses — so the dialog and the writer can
/// never disagree about which profiles exist. Presence is still not a promise
/// that a hardware encoder will open on this machine; the writer opens every
/// candidate on a frame before it commits, and a failure there is reported by
/// the export itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportCapability {
    /// The H.264 encoder this build would pick first, by name.
    pub h264_encoder: Option<String>,
    /// True when that encoder reads GPU surfaces rather than host memory.
    pub h264_hardware: bool,
    /// Whether the MP4 profile's H.264 *and* AAC encoders are both present.
    pub mp4_available: bool,
    /// Whether the WebM profile's VP9 and Opus encoders are both present.
    pub webm_available: bool,
    /// Whether the lossless profile's FFV1 and PCM S16LE encoders are present.
    pub lossless_available: bool,
}

impl ExportCapability {
    /// The profile a new project should default to on this build.
    pub fn preferred_profile(&self) -> FfmpegExportProfile {
        if self.mp4_available {
            FfmpegExportProfile::Mp4H264Aac
        } else if self.webm_available {
            FfmpegExportProfile::WebmVp9Opus
        } else {
            FfmpegExportProfile::MatroskaFfv1PcmS16le
        }
    }
}

/// Measures which export profiles this build carries.
pub fn describe_export_capability() -> ExportCapability {
    let h264 = H264_ENCODER_CANDIDATES
        .iter()
        .copied()
        .find(|name| codec::encoder::find_by_name(name).is_some());
    let aac = codec::encoder::find(codec::Id::AAC).is_some();
    let vp9 = codec::encoder::find_by_name("libvpx-vp9").is_some()
        && codec::encoder::find_by_name("libopus").is_some();
    let lossless = codec::encoder::find(codec::Id::FFV1).is_some()
        && codec::encoder::find(codec::Id::PCM_S16LE).is_some();
    ExportCapability {
        h264_encoder: h264.map(str::to_owned),
        h264_hardware: matches!(
            h264,
            Some("h264_videotoolbox" | "h264_nvenc" | "h264_mf" | "h264_vaapi" | "h264_open")
        ),
        mp4_available: h264.is_some() && aac,
        webm_available: vp9,
        lossless_available: lossless,
    }
}

/// The H.264 encoders this build carries, with the result of the exact open
/// probe an MP4 export performs.
///
/// Presence alone is not capability: a Linux package can carry NVENC on a
/// machine with no NVIDIA driver, and Media Foundation can exist without a
/// hardware MFT. Reporting `present` and `opens` separately lets the settings
/// surface say "carried, but this machine cannot open it" instead of offering
/// an encoder that would fail on its first frame.
pub fn probe_h264_encoders() -> Vec<or_core::EncoderCandidate> {
    H264_ENCODER_CANDIDATES
        .iter()
        .map(|name| {
            let encoder = codec::encoder::find_by_name(name);
            let present = encoder.is_some();
            let hardware = matches!(
                *name,
                "h264_videotoolbox" | "h264_nvenc" | "h264_mf" | "h264_vaapi" | "h264_open"
            );
            let (opens, note) = if let Some(encoder) = encoder {
                if *name == "h264_vaapi" {
                    let opens = vaapi_answers();
                    (
                        opens,
                        (!opens).then(|| {
                            "the VAAPI device or its frame pool did not open".to_owned()
                        }),
                    )
                } else if !hardware_device_available(name) {
                    (
                        false,
                        Some("the device this encoder needs did not open".to_owned()),
                    )
                } else {
                    let opens = encoder_answers(encoder);
                    (
                        opens,
                        (!opens).then(|| "the encoder did not open on this machine".to_owned()),
                    )
                }
            } else {
                (false, None)
            };
            or_core::EncoderCandidate {
                name: (*name).to_owned(),
                present,
                opens,
                hardware,
                note,
            }
        })
        .collect()
}

/// What an export writer needs, in one value.
pub struct ExportWriterOptions<'a> {
    pub profile: FfmpegExportProfile,
    pub quality: ExportQuality,
    /// The machine's chosen performance profile; decides which encoder opens
    /// when no explicit one is requested.
    pub performance: or_core::PerformanceProfile,
    /// An encoder the person picked explicitly. It wins whenever it opens.
    pub preferred_h264_encoder: Option<&'a str>,
    pub width: u32,
    pub height: u32,
    pub frame_rate_numerator: u32,
    pub frame_rate_denominator: u32,
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
    /// The encoder this writer settled on, so a caller or a test can tell which
    /// host path an export took.
    video_encoder_name: String,
    /// Set when the video encoder reads GPU surfaces; the writer uploads every
    /// rendered frame into this pool before sending it.
    hardware_frames: Option<HardwareFrames>,
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
    pub fn create_with_profile_and_quality(
        destination: impl AsRef<Path>,
        profile: FfmpegExportProfile,
        quality: ExportQuality,
        width: u32,
        height: u32,
        frame_rate_numerator: u32,
        frame_rate_denominator: u32,
    ) -> Result<Self, ExportEncodeError> {
        Self::create_with_options(
            destination,
            ExportWriterOptions {
                profile,
                quality,
                performance: or_core::PerformanceProfile::Recommended,
                preferred_h264_encoder: None,
                width,
                height,
                frame_rate_numerator,
                frame_rate_denominator,
            },
        )
    }

    /// Opens a destination with the machine's performance profile and an
    /// optional encoder the person chose.
    ///
    /// The encoder decision happens here, once per job. Nothing in the frame
    /// loop asks the hardware again, so a slow probe never turns into a
    /// per-frame tax.
    pub fn create_with_options(
        destination: impl AsRef<Path>,
        options: ExportWriterOptions<'_>,
    ) -> Result<Self, ExportEncodeError> {
        let ExportWriterOptions {
            profile,
            quality,
            performance,
            preferred_h264_encoder,
            width,
            height,
            frame_rate_numerator,
            frame_rate_denominator,
        } = options;
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
        // The MP4 arm needs two things from one pick: the encoder to open and
        // whether it wants a device surface pool. Resolving it here keeps the
        // probe out of the tuple below.
        let encoding_plan = if profile == FfmpegExportProfile::Mp4H264Aac {
            let plan = or_core::plan_h264_encoding(
                performance,
                preferred_h264_encoder,
                &probe_h264_encoders(),
            );
            Some(plan)
        } else {
            None
        };
        let h264_pick = match &encoding_plan {
            Some(plan) => Some(pick_h264_encoder(plan)?),
            None => None,
        };
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
                    h264_pick
                        .ok_or_else(|| {
                            ExportEncodeError("internal: no H.264 encoder was picked".to_owned())
                        })?
                        .encoder,
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
                // libvpx defaults to one thread; row-mt only pays off once the
                // encoder is told how many rows may run at once, so a WebM export
                // was leaving every other core idle.
                let threads = std::thread::available_parallelism()
                    .map_or(1, |count| count.get())
                    .clamp(1, VP9_MAX_THREADS);
                options.set("threads", threads.to_string());
                options.set("row-mt", "1");
            } else {
                options.set("b", video_bitrate(width, height, quality).to_string());
                if video_codec.name() == "h264_nvenc" {
                    // NVENC's own speed ladder is its preset. Leaving it at the
                    // default spends the same time on a Speed export as on a
                    // Quality one, which is the opposite of what the preference
                    // asks for.
                    options.set("preset", nvenc_preset(quality));
                } else if video_codec.name() == "h264_vaapi" {
                    // VAAPI answers "No quality level set; using default (20)"
                    // without this, and the driver's default is the same for a
                    // Speed export and a Quality one.
                    options.set("quality", vaapi_quality(quality).to_string());
                }
            }
            // A VAAPI encoder reads surfaces from its own pool, so the pool is
            // attached before the encoder opens and every frame is uploaded
            // into it on the way in.
            let hardware_frames = if h264_pick.is_some_and(|pick| pick.vaapi) {
                let pool = HardwareFrames::vaapi(width, height)?;
                // SAFETY: the encoder has not opened, and the pool is moved
                // into the writer below, so it outlives the encoder.
                unsafe { pool.attach(video_context.as_mut_ptr()) };
                Some(pool)
            } else {
                None
            };
            video_context
                .open_as_with(video_codec, options)
                .map_err(|error| h264_open_error(video_codec.name(), error))
                .map(|encoder| (encoder, hardware_frames))?
        } else {
            (
                video_context
                    .open_as(video_codec)
                    .map_err(ExportEncodeError::from)?,
                None,
            )
        };
        let (video_encoder, hardware_frames) = video_encoder;

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
            video_encoder_name: video_codec.name().to_owned(),
            hardware_frames,
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

    /// The H.264/VP9/FFV1 encoder this writer opened.
    pub fn video_encoder_name(&self) -> &str {
        &self.video_encoder_name
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
        match &self.hardware_frames {
            // A VAAPI encoder only reads its own surfaces, so the rendered
            // NV12 frame is uploaded into the device pool first.
            Some(pool) => unsafe {
                // SAFETY: the pool belongs to this writer and the encoder is
                // open on it; `encoded` is a live NV12 frame.
                pool.send(self.video_encoder.as_mut_ptr(), encoded.as_ptr())?
            },
            None => self
                .video_encoder
                .send_frame(&encoded)
                .map_err(ExportEncodeError::from)?,
        }
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

/// NVENC's speed ladder: higher is slower and cleaner.
///
/// `p2` still uses the GPU for every stage, so a Speed export stays hardware
/// bound rather than falling back to software.
const fn nvenc_preset(quality: ExportQuality) -> &'static str {
    match quality {
        ExportQuality::Quality => "p5",
        ExportQuality::Speed => "p2",
    }
}

/// VAAPI's speed ladder: higher is faster and softer.
///
/// The driver takes 1..=7 as target usage; 7 is its fastest mode and 2 its
/// slower, cleaner one, which is the same direction as the other ladders in
/// this file.
const fn vaapi_quality(quality: ExportQuality) -> u8 {
    match quality {
        ExportQuality::Quality => 2,
        ExportQuality::Speed => 7,
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
    use super::{
        ExportQuality, FfmpegExportProfile, H264_ENCODER_CANDIDATES, codec,
        describe_export_capability, nvenc_preset, vaapi_quality, video_bitrate, vp9_settings,
    };

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
    fn vaapi_spends_more_time_on_a_quality_export_than_on_a_speed_export() {
        // VAAPI's `quality` is target usage: higher is faster and softer.
        assert_eq!(vaapi_quality(ExportQuality::Speed), 7);
        assert_eq!(vaapi_quality(ExportQuality::Quality), 2);
        assert!(vaapi_quality(ExportQuality::Speed) > vaapi_quality(ExportQuality::Quality));
    }

    #[test]
    fn nvenc_spends_more_time_on_a_quality_export_than_on_a_speed_export() {
        assert_eq!(nvenc_preset(ExportQuality::Speed), "p2");
        assert_eq!(nvenc_preset(ExportQuality::Quality), "p5");
        // NVENC orders its presets from p1 (fastest) to p7 (slowest).
        assert!(nvenc_preset(ExportQuality::Speed) < nvenc_preset(ExportQuality::Quality));
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

    #[test]
    fn the_described_profiles_match_the_encoders_this_build_links() {
        let capability = describe_export_capability();
        // Every shipped build links the lossless pair, so an empty row is a
        // packaging regression rather than a host difference.
        assert!(
            capability.lossless_available,
            "the linked FFmpeg has no FFV1/PCM_S16LE pair"
        );
        let h264_present = H264_ENCODER_CANDIDATES
            .iter()
            .any(|name| codec::encoder::find_by_name(name).is_some());
        assert_eq!(capability.h264_encoder.is_some(), h264_present);
        // A named encoder is one of the candidates, never an invention.
        if let Some(name) = capability.h264_encoder.as_deref() {
            assert!(H264_ENCODER_CANDIDATES.contains(&name), "{name}");
        }
        // The preferred profile is one the description says is available.
        let preferred = capability.preferred_profile();
        let preferred_available = match preferred {
            FfmpegExportProfile::Mp4H264Aac => capability.mp4_available,
            FfmpegExportProfile::WebmVp9Opus => capability.webm_available,
            FfmpegExportProfile::MatroskaFfv1PcmS16le => capability.lossless_available,
        };
        assert!(preferred_available, "{preferred:?} is not available");
    }
}
