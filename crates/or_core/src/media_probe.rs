use crate::{
    AudioStreamMetadata, MAX_MEDIA_STREAMS, MediaMetadata, MediaStreamMetadata,
    OtherStreamMetadata, RationalRate, RationalTime, VideoStreamMetadata,
    media::parse_decimal_duration,
};
use serde::Serialize;
use serde_json::{Map, Value};
use std::{
    error::Error,
    fmt, fs,
    io::{self, Read},
    num::NonZeroU32,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_STDOUT_BYTES: usize = 1024 * 1024;
const MAX_STDERR_BYTES: usize = 64 * 1024;
const MAX_DIAGNOSTIC_CHARS: usize = 512;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const SHOW_ENTRIES: &str = "format=format_name,duration:stream=index,codec_type,codec_name,width,height,pix_fmt,avg_frame_rate,sample_rate,channels,channel_layout,duration:stream_disposition=attached_pic";

/// Stable machine-readable classification for a local media-probe failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum MediaProbeErrorCode {
    #[serde(rename = "MEDIA_NOT_FOUND")]
    MediaNotFound,
    #[serde(rename = "MEDIA_NOT_REGULAR_FILE")]
    MediaNotRegularFile,
    #[serde(rename = "PROBE_BACKEND_UNAVAILABLE")]
    ProbeBackendUnavailable,
    #[serde(rename = "UNSUPPORTED_CONTAINER")]
    UnsupportedContainer,
    #[serde(rename = "UNSUPPORTED_CODEC")]
    UnsupportedCodec,
    #[serde(rename = "PROBE_TIMEOUT")]
    ProbeTimeout,
    #[serde(rename = "PROBE_OUTPUT_TOO_LARGE")]
    ProbeOutputTooLarge,
    #[serde(rename = "PROBE_FAILED")]
    ProbeFailed,
    #[serde(rename = "INVALID_PROBE_OUTPUT")]
    InvalidProbeOutput,
    #[serde(rename = "INVALID_MEDIA_METADATA")]
    InvalidMediaMetadata,
}

impl MediaProbeErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MediaNotFound => "MEDIA_NOT_FOUND",
            Self::MediaNotRegularFile => "MEDIA_NOT_REGULAR_FILE",
            Self::ProbeBackendUnavailable => "PROBE_BACKEND_UNAVAILABLE",
            Self::UnsupportedContainer => "UNSUPPORTED_CONTAINER",
            Self::UnsupportedCodec => "UNSUPPORTED_CODEC",
            Self::ProbeTimeout => "PROBE_TIMEOUT",
            Self::ProbeOutputTooLarge => "PROBE_OUTPUT_TOO_LARGE",
            Self::ProbeFailed => "PROBE_FAILED",
            Self::InvalidProbeOutput => "INVALID_PROBE_OUTPUT",
            Self::InvalidMediaMetadata => "INVALID_MEDIA_METADATA",
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::MediaNotFound => "media file was not found",
            Self::MediaNotRegularFile => "media path is not a regular file",
            Self::ProbeBackendUnavailable => "ffprobe backend is not available",
            Self::UnsupportedContainer => "the media container is not in the import matrix",
            Self::UnsupportedCodec => "a media stream codec is not in the import matrix",
            Self::ProbeTimeout => "media probing timed out",
            Self::ProbeOutputTooLarge => "ffprobe output exceeded the configured size limit",
            Self::ProbeFailed => "ffprobe failed to inspect the media file",
            Self::InvalidProbeOutput => "ffprobe returned invalid output",
            Self::InvalidMediaMetadata => "ffprobe metadata contains invalid media values",
        }
    }
}

/// Structured failure from a bounded read-only media probe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaProbeError {
    code: MediaProbeErrorCode,
    diagnostic: Option<String>,
}

impl MediaProbeError {
    const fn new(code: MediaProbeErrorCode) -> Self {
        Self {
            code,
            diagnostic: None,
        }
    }

    fn with_diagnostic(code: MediaProbeErrorCode, diagnostic: Option<String>) -> Self {
        Self { code, diagnostic }
    }

    pub const fn code(&self) -> MediaProbeErrorCode {
        self.code
    }

    pub const fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    pub const fn message(&self) -> &'static str {
        self.code.message()
    }

    pub fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
}

impl fmt::Display for MediaProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())?;
        // The probe backend's own bounded, path-sanitised diagnostic is the only
        // evidence a user or an agent has for *why* ffprobe refused a file. A
        // generic "ffprobe failed" sentence hides the difference between an
        // unfinished recording, a truncated download and a missing demuxer.
        if let Some(diagnostic) = &self.diagnostic {
            write!(formatter, ": {diagnostic}")?;
        }
        Ok(())
    }
}

impl Error for MediaProbeError {}

/// Inspects one local filesystem file through the packaged-first `ffprobe`.
///
/// The call is synchronous and read-only. It does not create a `MediaId`, open
/// a project, or mutate project revision/history. Files outside the supported
/// import profiles (Matroska/WebM with FFV1/PCM S16LE, H.264/AAC, or VP9/Opus,
/// PNG/JPEG/WebP still images, audio-only PCM S16LE WAV or MP3, or MOV-family
/// H.264/AAC) are rejected as unsupported formats.
pub fn probe_media_file(path: &Path) -> Result<MediaMetadata, MediaProbeError> {
    let metadata = FfprobeBackend::from_environment().probe(path)?;
    enforce_import_matrix(&metadata)?;
    Ok(metadata)
}

/// Validates probe output produced by an in-process media backend against the
/// same metadata limits and minimum container/codec import matrix as ffprobe.
pub fn parse_media_probe_output(
    bytes: &[u8],
    file_size_bytes: u64,
) -> Result<MediaMetadata, MediaProbeError> {
    let metadata = parse_probe_json(bytes, file_size_bytes)?;
    enforce_import_matrix(&metadata)?;
    Ok(metadata)
}

struct FfprobeBackend {
    executable: PathBuf,
    timeout: Duration,
    stdout_limit: usize,
    stderr_limit: usize,
}

impl FfprobeBackend {
    fn from_environment() -> Self {
        Self::new(crate::media_helpers::ffprobe_executable())
    }

    fn new(executable: PathBuf) -> Self {
        Self {
            executable,
            timeout: DEFAULT_TIMEOUT,
            stdout_limit: MAX_STDOUT_BYTES,
            stderr_limit: MAX_STDERR_BYTES,
        }
    }

    fn probe(&self, path: &Path) -> Result<MediaMetadata, MediaProbeError> {
        let metadata = fs::metadata(path).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                MediaProbeError::new(MediaProbeErrorCode::MediaNotFound)
            } else {
                MediaProbeError::new(MediaProbeErrorCode::ProbeFailed)
            }
        })?;
        if !metadata.is_file() {
            return Err(MediaProbeError::new(
                MediaProbeErrorCode::MediaNotRegularFile,
            ));
        }

        let resolved_path = fs::canonicalize(path).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                MediaProbeError::new(MediaProbeErrorCode::MediaNotFound)
            } else {
                MediaProbeError::new(MediaProbeErrorCode::ProbeFailed)
            }
        })?;
        reject_unsupported_container(&resolved_path)?;
        let stdout = self.run(&resolved_path)?;
        parse_probe_json(&stdout, metadata.len())
    }

    fn run(&self, path: &Path) -> Result<Vec<u8>, MediaProbeError> {
        let mut child = Command::new(&self.executable)
            .arg("-v")
            .arg("error")
            .arg("-show_entries")
            .arg(SHOW_ENTRIES)
            .arg("-of")
            .arg("json")
            .arg("-i")
            .arg(path.as_os_str())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| MediaProbeError::new(MediaProbeErrorCode::ProbeBackendUnavailable))?;

        let Some(stdout) = child.stdout.take() else {
            stop_child(&mut child);
            return Err(MediaProbeError::with_diagnostic(
                MediaProbeErrorCode::ProbeFailed,
                Some("ffprobe stdout pipe was unavailable".to_owned()),
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            stop_child(&mut child);
            return Err(MediaProbeError::with_diagnostic(
                MediaProbeErrorCode::ProbeFailed,
                Some("ffprobe stderr pipe was unavailable".to_owned()),
            ));
        };

        let (sender, receiver) = mpsc::channel();
        let stdout_reader = spawn_reader(
            StreamKind::Stdout,
            stdout,
            self.stdout_limit,
            sender.clone(),
        );
        let stderr_reader = spawn_reader(StreamKind::Stderr, stderr, self.stderr_limit, sender);
        let started = Instant::now();
        let mut status = None;
        let mut stdout_bytes = None;
        let mut stderr_bytes = None;

        while status.is_none() || stdout_bytes.is_none() || stderr_bytes.is_none() {
            if status.is_none() {
                match child.try_wait() {
                    Ok(Some(exited)) => status = Some(exited),
                    Ok(None) => {}
                    Err(_) => {
                        stop_child(&mut child);
                        join_readers(stdout_reader, stderr_reader);
                        return Err(MediaProbeError::with_diagnostic(
                            MediaProbeErrorCode::ProbeFailed,
                            Some("could not wait for ffprobe to exit".to_owned()),
                        ));
                    }
                }
            }

            if status.is_some() && stdout_bytes.is_some() && stderr_bytes.is_some() {
                break;
            }
            if started.elapsed() >= self.timeout {
                stop_child(&mut child);
                join_readers(stdout_reader, stderr_reader);
                return Err(MediaProbeError::new(MediaProbeErrorCode::ProbeTimeout));
            }

            match receiver.recv_timeout(POLL_INTERVAL) {
                Ok(ReaderEvent::Complete(StreamKind::Stdout, Ok(bytes))) => {
                    stdout_bytes = Some(bytes);
                }
                Ok(ReaderEvent::Complete(StreamKind::Stderr, Ok(bytes))) => {
                    stderr_bytes = Some(bytes);
                }
                Ok(ReaderEvent::Complete(_, Err(ReadFailure::TooLarge)))
                | Ok(ReaderEvent::TooLarge) => {
                    stop_child(&mut child);
                    join_readers(stdout_reader, stderr_reader);
                    return Err(MediaProbeError::new(
                        MediaProbeErrorCode::ProbeOutputTooLarge,
                    ));
                }
                Ok(ReaderEvent::Complete(_, Err(ReadFailure::Io))) => {
                    stop_child(&mut child);
                    join_readers(stdout_reader, stderr_reader);
                    return Err(MediaProbeError::with_diagnostic(
                        MediaProbeErrorCode::ProbeFailed,
                        Some("could not read ffprobe output".to_owned()),
                    ));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    if stdout_bytes.is_some() && stderr_bytes.is_some() {
                        // The child may close both pipes just before it exits.
                        // Keep polling it instead of treating that short race
                        // as a reader failure.
                        thread::sleep(POLL_INTERVAL);
                        continue;
                    }
                    stop_child(&mut child);
                    join_readers(stdout_reader, stderr_reader);
                    return Err(MediaProbeError::with_diagnostic(
                        MediaProbeErrorCode::ProbeFailed,
                        Some("ffprobe output readers stopped unexpectedly".to_owned()),
                    ));
                }
            }
        }

        let reader_threads_ok = join_readers(stdout_reader, stderr_reader);
        let stdout = stdout_bytes.expect("reader output is collected before loop exit");
        let stderr = stderr_bytes.expect("reader output is collected before loop exit");
        if !reader_threads_ok {
            return Err(MediaProbeError::with_diagnostic(
                MediaProbeErrorCode::ProbeFailed,
                Some("an ffprobe output reader stopped unexpectedly".to_owned()),
            ));
        }

        let status = status.expect("child status is collected before loop exit");
        if !status.success() {
            return Err(MediaProbeError::with_diagnostic(
                MediaProbeErrorCode::ProbeFailed,
                sanitize_diagnostic(&stderr, path).or_else(|| {
                    Some("ffprobe exited unsuccessfully without a diagnostic".to_owned())
                }),
            ));
        }
        Ok(stdout)
    }
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[derive(Clone, Copy, Debug)]
enum StreamKind {
    Stdout,
    Stderr,
}

enum ReaderEvent {
    Complete(StreamKind, Result<Vec<u8>, ReadFailure>),
    TooLarge,
}

#[derive(Clone, Copy, Debug)]
enum ReadFailure {
    TooLarge,
    Io,
}

fn spawn_reader<R: Read + Send + 'static>(
    kind: StreamKind,
    mut reader: R,
    limit: usize,
    sender: mpsc::Sender<ReaderEvent>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || match read_bounded(&mut reader, limit) {
        Err(ReadFailure::TooLarge) => {
            let _ = sender.send(ReaderEvent::TooLarge);
        }
        result => {
            let _ = sender.send(ReaderEvent::Complete(kind, result));
        }
    })
}

fn read_bounded(reader: &mut impl Read, limit: usize) -> Result<Vec<u8>, ReadFailure> {
    let mut output = Vec::with_capacity(limit.min(8192));
    let mut buffer = [0_u8; 8192];
    loop {
        let remaining = limit
            .checked_add(1)
            .and_then(|bound| bound.checked_sub(output.len()))
            .ok_or(ReadFailure::TooLarge)?;
        let read_limit = remaining.min(buffer.len());
        let count = match reader.read(&mut buffer[..read_limit]) {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(ReadFailure::Io),
        };
        if count == 0 {
            return Ok(output);
        }
        output.extend_from_slice(&buffer[..count]);
        if output.len() > limit {
            return Err(ReadFailure::TooLarge);
        }
    }
}

fn join_readers(
    stdout_reader: thread::JoinHandle<()>,
    stderr_reader: thread::JoinHandle<()>,
) -> bool {
    stdout_reader.join().is_ok() && stderr_reader.join().is_ok()
}

fn sanitize_diagnostic(stderr: &[u8], path: &Path) -> Option<String> {
    let mut text = String::from_utf8_lossy(stderr).into_owned();
    let path = path.to_string_lossy();
    if !path.is_empty() {
        text = text.replace(path.as_ref(), "<media file>");
    }
    let text: String = text
        .chars()
        .filter(|character| !character.is_control() || character.is_whitespace())
        .take(MAX_DIAGNOSTIC_CHARS)
        .collect();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn parse_probe_json(bytes: &[u8], file_size_bytes: u64) -> Result<MediaMetadata, MediaProbeError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| MediaProbeError::new(MediaProbeErrorCode::InvalidProbeOutput))?;
    let root = value
        .as_object()
        .ok_or_else(|| MediaProbeError::new(MediaProbeErrorCode::InvalidProbeOutput))?;
    let streams = root
        .get("streams")
        .and_then(Value::as_array)
        .ok_or_else(|| MediaProbeError::new(MediaProbeErrorCode::InvalidProbeOutput))?;
    if streams.len() > MAX_MEDIA_STREAMS {
        return Err(MediaProbeError::new(
            MediaProbeErrorCode::InvalidMediaMetadata,
        ));
    }
    let format = match root.get("format") {
        None | Some(Value::Null) => None,
        Some(Value::Object(format)) => Some(format),
        Some(_) => {
            return Err(MediaProbeError::new(
                MediaProbeErrorCode::InvalidProbeOutput,
            ));
        }
    };

    let format_names = match format.and_then(|fields| fields.get("format_name")) {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(names)) => names
            .split(',')
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect(),
        Some(_) => {
            return Err(MediaProbeError::new(
                MediaProbeErrorCode::InvalidMediaMetadata,
            ));
        }
    };
    let is_still_image = is_still_image_format(&format_names);
    // Image demuxers report one synthetic frame as a 1/25-second stream. A
    // still image has no finite source range; the timeline supplies its own
    // duration when the user inserts it.
    let duration = if is_still_image {
        None
    } else {
        parse_optional_duration(format.and_then(|fields| fields.get("duration")))?
    };
    let mut parsed_streams = Vec::with_capacity(streams.len());

    for stream in streams {
        let fields = stream
            .as_object()
            .ok_or_else(|| MediaProbeError::new(MediaProbeErrorCode::InvalidProbeOutput))?;
        if format_names.iter().any(|name| name == "mp3") && attached_picture(fields)? {
            continue;
        }
        let index = required_u32(fields, "index")?;
        let codec_type = optional_string(fields, "codec_type")?;
        let codec_name = optional_string(fields, "codec_name")?;

        let stream = match codec_type.as_deref() {
            Some("video") => {
                let width = required_nonzero_u32(fields, "width")?;
                let height = required_nonzero_u32(fields, "height")?;
                let pixel_format = optional_string(fields, "pix_fmt")?;
                let average_frame_rate = optional_rate(fields.get("avg_frame_rate"));
                let stream_duration = if is_still_image {
                    None
                } else {
                    parse_optional_duration(fields.get("duration"))?
                };
                MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                    index,
                    codec_name,
                    width,
                    height,
                    pixel_format,
                    average_frame_rate,
                    stream_duration,
                ))
            }
            Some("audio") => {
                let sample_rate = optional_nonzero_u32(fields.get("sample_rate"));
                let channels = optional_nonzero_u32(fields.get("channels"));
                let channel_layout = optional_string(fields, "channel_layout")?;
                let stream_duration = if is_still_image {
                    None
                } else {
                    parse_optional_duration(fields.get("duration"))?
                };
                MediaStreamMetadata::Audio(AudioStreamMetadata::from_probe(
                    index,
                    codec_name,
                    sample_rate,
                    channels,
                    channel_layout,
                    stream_duration,
                ))
            }
            _ => MediaStreamMetadata::Other(OtherStreamMetadata::from_probe(
                index, codec_type, codec_name,
            )),
        };
        parsed_streams.push(stream);
    }

    let metadata =
        MediaMetadata::from_probe(format_names, duration, file_size_bytes, parsed_streams);
    metadata
        .validate()
        .map_err(|_| MediaProbeError::new(MediaProbeErrorCode::InvalidMediaMetadata))?;
    Ok(metadata)
}

fn attached_picture(fields: &Map<String, Value>) -> Result<bool, MediaProbeError> {
    let Some(disposition) = fields.get("disposition") else {
        return Ok(false);
    };
    let Some(disposition) = disposition.as_object() else {
        return Err(MediaProbeError::new(
            MediaProbeErrorCode::InvalidMediaMetadata,
        ));
    };
    match disposition.get("attached_pic") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(Value::Number(value)) if value.as_u64().is_some() => {
            Ok(value.as_u64().is_some_and(|value| value != 0))
        }
        Some(Value::String(value)) if value == "0" || value == "1" => Ok(value == "1"),
        _ => Err(MediaProbeError::new(
            MediaProbeErrorCode::InvalidMediaMetadata,
        )),
    }
}

/// EBML header shared by Matroska and WebM.
const EBML_HEADER: &[u8; 4] = b"\x1a\x45\xdf\xa3";
const RIFF_HEADER: &[u8; 4] = b"RIFF";
const RF64_HEADER: &[u8; 4] = b"RF64";
const WAVE_HEADER: &[u8; 4] = b"WAVE";
const WEBP_HEADER: &[u8; 4] = b"WEBP";
const ID3_HEADER: &[u8; 3] = b"ID3";
const JPEG_HEADER: &[u8; 3] = b"\xff\xd8\xff";
const PNG_HEADER: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// Video codecs the import profile carries. The linked FFmpeg build decides
/// whether a stream can actually be decoded; this list bounds what the project
/// core accepts before any decoder is opened.
const IMPORT_VIDEO_CODECS: &[&str] = &[
    "av1",
    "dnxhd",
    "ffv1",
    "h263",
    "h264",
    "hevc",
    "mjpeg",
    "mpeg2video",
    "mpeg4",
    "prores",
    "theora",
    "vc1",
    "vp6",
    "vp8",
    "vp9",
    "wmv3",
];
/// Audio codecs the import profile carries, including the common lossless and
/// uncompressed families users get from cameras, phones and audio editors.
const IMPORT_AUDIO_CODECS: &[&str] = &[
    "aac",
    "ac3",
    "alac",
    "dts",
    "eac3",
    "flac",
    "mp1",
    "mp2",
    "mp3",
    "opus",
    "pcm_f32le",
    "pcm_f64le",
    "pcm_s16be",
    "pcm_s16le",
    "pcm_s24be",
    "pcm_s24le",
    "pcm_s32be",
    "pcm_s32le",
    "pcm_u8",
    "truehd",
    "vorbis",
    "wmav2",
];
const MOV_FORMAT_NAMES: &[&str] = &["mov", "mp4", "m4a", "3gp", "3g2", "mj2"];
const SINGLE_CONTAINER_FORMAT_NAMES: &[&str] = &["wav", "mp3", "ogg", "flac", "aiff", "avi"];
const STILL_IMAGE_FORMAT_NAMES: &[&str] =
    &["image2", "image2pipe", "jpeg_pipe", "png_pipe", "webp_pipe"];
const OGG_HEADER: &[u8; 4] = b"OggS";
const FLAC_HEADER: &[u8; 4] = b"fLaC";
const FORM_HEADER: &[u8; 4] = b"FORM";
const AIFF_HEADER: &[u8; 4] = b"AIFF";
const AIFC_HEADER: &[u8; 4] = b"AIFC";
const AVI_HEADER: &[u8; 4] = b"AVI ";

fn is_still_image_format(format_names: &[String]) -> bool {
    format_names
        .iter()
        .any(|name| STILL_IMAGE_FORMAT_NAMES.contains(&name.as_str()))
}

/// Rejects containers outside the shipped Matroska/WebM, WAV, MP3, ISO BMFF,
/// and still-image profiles before spawning the probe backend.
fn reject_unsupported_container(path: &Path) -> Result<(), MediaProbeError> {
    use std::io::Read as _;
    let mut header = [0_u8; 12];
    let mut file =
        fs::File::open(path).map_err(|_| MediaProbeError::new(MediaProbeErrorCode::ProbeFailed))?;
    file.read_exact(&mut header[..4])
        .map_err(|_| MediaProbeError::new(MediaProbeErrorCode::UnsupportedContainer))?;
    if &header[..4] == EBML_HEADER
        || &header[..4] == OGG_HEADER
        || &header[..4] == FLAC_HEADER
        || &header[..3] == ID3_HEADER
        || (header[0] == 0xff && header[1] & 0xe0 == 0xe0)
        || &header[..3] == JPEG_HEADER
    {
        return Ok(());
    }
    file.read_exact(&mut header[4..])
        .map_err(|_| MediaProbeError::new(MediaProbeErrorCode::UnsupportedContainer))?;
    if &header[..4] == RIFF_HEADER || &header[..4] == RF64_HEADER {
        if &header[8..12] == WAVE_HEADER
            || (&header[..4] == RIFF_HEADER
                && (&header[8..12] == WEBP_HEADER || &header[8..12] == AVI_HEADER))
        {
            return Ok(());
        }
        return Err(MediaProbeError::new(
            MediaProbeErrorCode::UnsupportedContainer,
        ));
    }
    if &header[..4] == FORM_HEADER
        && (&header[8..12] == AIFF_HEADER || &header[8..12] == AIFC_HEADER)
    {
        return Ok(());
    }
    if &header[..8] == PNG_HEADER {
        return Ok(());
    }
    let declared_size = u32::from_be_bytes(header[..4].try_into().expect("four-byte size"));
    if &header[4..8] == b"ftyp" && declared_size >= 16 {
        return Ok(());
    }
    Err(MediaProbeError::new(
        MediaProbeErrorCode::UnsupportedContainer,
    ))
}

/// Rejects probed streams outside the shipped import profile.
///
/// Non-audio/video streams (subtitles, attachments) are inert to OR and pass.
/// Containers are bounded to Matroska/WebM, the ISO BMFF family, WAV, MP3, OGG,
/// FLAC, AIFF, AVI and still images. Every audio and video stream must name a
/// codec in the import profile; the application additionally checks that the
/// linked FFmpeg build carries a decoder for it, so a file is never admitted and
/// then found unplayable during preview.
fn enforce_import_matrix(metadata: &MediaMetadata) -> Result<(), MediaProbeError> {
    if is_still_image_format(metadata.format_names()) {
        return match metadata.streams() {
            [MediaStreamMetadata::Video(video)]
                if still_image_codec_supported(metadata.format_names(), video.codec_name()) =>
            {
                Ok(())
            }
            _ => Err(MediaProbeError::new(MediaProbeErrorCode::UnsupportedCodec)),
        };
    }

    let names = metadata.format_names();
    let is_matroska = names.iter().any(|name| name == "matroska");
    let is_mov = names
        .iter()
        .any(|name| MOV_FORMAT_NAMES.contains(&name.as_str()));
    let is_audio_only_container = names
        .iter()
        .any(|name| matches!(name.as_str(), "wav" | "mp3" | "ogg" | "flac" | "aiff"));
    let is_avi = names.iter().any(|name| name == "avi");
    if !is_matroska && !is_mov && !is_audio_only_container && !is_avi {
        // An AVI file whose format names do not include `avi` is not ours.
        if !names
            .iter()
            .any(|name| SINGLE_CONTAINER_FORMAT_NAMES.contains(&name.as_str()))
        {
            return Err(MediaProbeError::new(
                MediaProbeErrorCode::UnsupportedContainer,
            ));
        }
    }

    let mut has_audio = false;
    let mut has_video = false;
    for stream in metadata.streams() {
        match stream {
            MediaStreamMetadata::Video(video) => {
                has_video = true;
                if !video
                    .codec_name()
                    .is_some_and(|codec| IMPORT_VIDEO_CODECS.contains(&codec))
                {
                    return Err(MediaProbeError::new(MediaProbeErrorCode::UnsupportedCodec));
                }
            }
            MediaStreamMetadata::Audio(audio) => {
                has_audio = true;
                if !audio
                    .codec_name()
                    .is_some_and(|codec| IMPORT_AUDIO_CODECS.contains(&codec))
                {
                    return Err(MediaProbeError::new(MediaProbeErrorCode::UnsupportedCodec));
                }
            }
            MediaStreamMetadata::Other(_) => {}
        }
    }
    if is_audio_only_container && (!has_audio || has_video) {
        return Err(MediaProbeError::new(MediaProbeErrorCode::UnsupportedCodec));
    }
    if !has_audio && !has_video {
        return Err(MediaProbeError::new(MediaProbeErrorCode::UnsupportedCodec));
    }
    Ok(())
}

fn still_image_codec_supported(format_names: &[String], codec: Option<&str>) -> bool {
    let generic_image_demuxer = format_names
        .iter()
        .any(|name| matches!(name.as_str(), "image2" | "image2pipe"));
    match codec {
        Some("png") => generic_image_demuxer || format_names.iter().any(|name| name == "png_pipe"),
        Some("mjpeg") => {
            generic_image_demuxer || format_names.iter().any(|name| name == "jpeg_pipe")
        }
        Some("webp") => {
            generic_image_demuxer || format_names.iter().any(|name| name == "webp_pipe")
        }
        _ => false,
    }
}

fn optional_string(
    fields: &Map<String, Value>,
    name: &str,
) -> Result<Option<String>, MediaProbeError> {
    match fields.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if value.is_empty() || value == "N/A" => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(MediaProbeError::new(
            MediaProbeErrorCode::InvalidMediaMetadata,
        )),
    }
}

fn required_u32(fields: &Map<String, Value>, name: &str) -> Result<u32, MediaProbeError> {
    fields
        .get(name)
        .and_then(parse_u64_value)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| MediaProbeError::new(MediaProbeErrorCode::InvalidMediaMetadata))
}

fn required_nonzero_u32(
    fields: &Map<String, Value>,
    name: &str,
) -> Result<NonZeroU32, MediaProbeError> {
    required_u32(fields, name).and_then(|value| {
        NonZeroU32::new(value)
            .ok_or_else(|| MediaProbeError::new(MediaProbeErrorCode::InvalidMediaMetadata))
    })
}

fn optional_nonzero_u32(value: Option<&Value>) -> Option<NonZeroU32> {
    value
        .and_then(parse_u64_value)
        .and_then(|value| u32::try_from(value).ok())
        .and_then(NonZeroU32::new)
}

fn parse_u64_value(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

fn parse_optional_duration(value: Option<&Value>) -> Result<Option<RationalTime>, MediaProbeError> {
    let Some(value) = value else {
        return Ok(None);
    };
    match value {
        Value::Null => Ok(None),
        Value::String(text) if text == "N/A" => Ok(None),
        Value::String(text) => parse_decimal_duration(text)
            .map(Some)
            .map_err(|_| MediaProbeError::new(MediaProbeErrorCode::InvalidMediaMetadata)),
        _ => Err(MediaProbeError::new(
            MediaProbeErrorCode::InvalidMediaMetadata,
        )),
    }
}

fn optional_rate(value: Option<&Value>) -> Option<RationalRate> {
    let Value::String(value) = value? else {
        return None;
    };
    parse_rational_rate(value).ok()
}

fn parse_rational_rate(value: &str) -> Result<RationalRate, RationalRateParseError> {
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or(RationalRateParseError::Invalid)?;
    let numerator = numerator
        .parse::<u64>()
        .map_err(|_| RationalRateParseError::Invalid)?;
    let denominator = denominator
        .parse::<u64>()
        .map_err(|_| RationalRateParseError::Invalid)?;
    if numerator == 0 || denominator == 0 {
        return Err(RationalRateParseError::Invalid);
    }
    let divisor = gcd(numerator, denominator);
    let numerator =
        u32::try_from(numerator / divisor).map_err(|_| RationalRateParseError::Overflow)?;
    let denominator =
        u32::try_from(denominator / divisor).map_err(|_| RationalRateParseError::Overflow)?;
    RationalRate::new(numerator, denominator).map_err(|_| RationalRateParseError::Invalid)
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RationalRateParseError {
    Invalid,
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::{
        FfprobeBackend, MAX_DIAGNOSTIC_CHARS, MediaProbeErrorCode, RationalRateParseError,
        enforce_import_matrix, parse_probe_json, parse_rational_rate, probe_media_file,
        reject_unsupported_container,
    };
    use crate::{MediaStreamMetadata, RationalRate};
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant},
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "or-media-probe-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn file(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            // Backend-plumbing fixtures carry the EBML header so they reach
            // the stub executable instead of the container-matrix rejection.
            let mut contents = b"\x1a\x45\xdf\xa3".to_vec();
            contents.extend_from_slice(b"generated test input");
            fs::write(&path, contents).unwrap();
            path
        }

        fn executable(&self) -> PathBuf {
            let executable = self.0.join(if cfg!(windows) {
                "probe-helper.exe"
            } else {
                "probe-helper"
            });
            let source = self.0.join("probe-helper.rs");
            fs::write(
                &source,
                include_str!("../tests/support/media_probe_stub.rs"),
            )
            .unwrap();
            let output = Command::new("rustc")
                .arg("--edition=2024")
                .arg(source)
                .arg("-o")
                .arg(&executable)
                .output()
                .expect("run rustc to build a test-only probe helper");
            assert!(
                output.status.success(),
                "test helper compilation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            executable
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn backend(executable: &Path) -> FfprobeBackend {
        FfprobeBackend::new(executable.to_owned())
    }

    #[test]
    fn ffprobe_json_parses_video_audio_and_other_streams() {
        let json = br#"{
            "format": {"format_name":"matroska,,webm", "duration":"1.5", "size":"1024", "future_format_field":"ignored"},
            "streams": [
                {"index":0,"codec_type":"video","codec_name":"ffv1","width":16,"height":16,"pix_fmt":"yuv420p","avg_frame_rate":"24000/1001","duration":"1.5","future_stream_field":true},
                {"index":1,"codec_type":"audio","codec_name":"pcm_s16le","sample_rate":"48000","channels":2,"channel_layout":"stereo","duration":"1.5"},
                {"index":2,"codec_type":"subtitle","codec_name":"subrip"}
            ],
            "future_root_field": {"safe": true}
        }"#;
        let metadata = parse_probe_json(json, 2048).unwrap();
        assert_eq!(metadata.format_names(), ["matroska", "webm"]);
        assert_eq!(metadata.duration().unwrap().numerator(), 3);
        assert_eq!(metadata.duration().unwrap().denominator(), 2);
        assert_eq!(metadata.file_size_bytes(), 2048);
        assert_eq!(metadata.streams().len(), 3);
        match &metadata.streams()[0] {
            MediaStreamMetadata::Video(stream) => {
                assert_eq!(stream.width(), 16);
                assert_eq!(stream.height(), 16);
                assert_eq!(
                    stream.average_frame_rate(),
                    Some(RationalRate::new(24_000, 1001).unwrap())
                );
            }
            _ => panic!("expected video stream"),
        }
        match &metadata.streams()[1] {
            MediaStreamMetadata::Audio(stream) => {
                assert_eq!(stream.sample_rate(), Some(48_000));
                assert_eq!(stream.channels(), Some(2));
            }
            _ => panic!("expected audio stream"),
        }
        match &metadata.streams()[2] {
            MediaStreamMetadata::Other(stream) => {
                assert_eq!(stream.codec_type(), Some("subtitle"));
                assert_eq!(stream.codec_name(), Some("subrip"));
            }
            _ => panic!("expected other stream"),
        }
    }

    #[test]
    fn ffprobe_json_accepts_video_only_audio_only_and_missing_optional_fields() {
        let video =
            br#"{"streams":[{"index":0,"codec_type":"video","width":1,"height":1}],"format":{}}"#;
        let video = parse_probe_json(video, 10).unwrap();
        assert_eq!(video.streams().len(), 1);
        assert!(video.format_names().is_empty());

        let audio = br#"{"streams":[{"index":0,"codec_type":"audio","sample_rate":"0","channels":0}],"format":{"duration":"N/A"}}"#;
        let audio = parse_probe_json(audio, 10).unwrap();
        assert_eq!(audio.duration(), None);
        match &audio.streams()[0] {
            MediaStreamMetadata::Audio(stream) => {
                assert_eq!(stream.sample_rate(), None);
                assert_eq!(stream.channels(), None);
            }
            _ => panic!("expected audio stream"),
        }

        let invalid_rate = br#"{"streams":[{"index":0,"codec_type":"video","width":1,"height":1,"avg_frame_rate":"0/0"}]}"#;
        let invalid_rate = parse_probe_json(invalid_rate, 10).unwrap();
        match &invalid_rate.streams()[0] {
            MediaStreamMetadata::Video(stream) => assert_eq!(stream.average_frame_rate(), None),
            _ => panic!("expected video stream"),
        }
    }

    #[test]
    fn ffprobe_json_rejects_invalid_dimensions_and_durations() {
        for json in [
            r#"{"streams":[{"index":0,"codec_type":"video","width":0,"height":1}]}"#,
            r#"{"streams":[{"index":0,"codec_type":"video","width":1,"height":-1}]}"#,
            r#"{"streams":[],"format":{"duration":"NaN"}}"#,
            r#"{"streams":[],"format":{"duration":"-0.5"}}"#,
            r#"{"streams":[],"format":{"duration":"1e3"}}"#,
        ] {
            assert_eq!(
                parse_probe_json(json.as_bytes(), 0).unwrap_err().code(),
                MediaProbeErrorCode::InvalidMediaMetadata
            );
        }
    }

    #[test]
    fn malformed_ffprobe_documents_have_a_stable_output_error() {
        for json in [
            br#""#.as_slice(),
            br#"[]"#,
            br#"{"format":{}}"#,
            br#"{"streams":[],"format":[]}"#,
        ] {
            assert_eq!(
                parse_probe_json(json, 0).unwrap_err().code(),
                MediaProbeErrorCode::InvalidProbeOutput
            );
        }
    }

    #[test]
    fn unsupported_file_signatures_are_rejected_without_spawning() {
        let directory = TestDirectory::new();
        for (name, bytes) in [
            ("empty.mkv", b"".as_slice()),
            ("text.mp4", b"not a media container".as_slice()),
            ("not-wave.wav", b"RIFF1234NOPE".as_slice()),
            ("short-riff.wav", b"RIFF".as_slice()),
        ] {
            let path = directory.0.join(name);
            fs::write(&path, bytes).unwrap();
            assert_eq!(
                probe_media_file(&path).unwrap_err().code(),
                MediaProbeErrorCode::UnsupportedContainer,
                "{name} must not reach the probe backend",
            );
        }
    }

    #[test]
    fn common_still_image_signatures_pass_container_preflight() {
        let directory = TestDirectory::new();
        for (name, signature) in [
            ("tiny.png", b"\x89PNG\r\n\x1a\n\0\0\0\0".as_slice()),
            ("tiny.jpg", b"\xff\xd8\xff\xd9".as_slice()),
            ("tiny.webp", b"RIFF\x04\0\0\0WEBP".as_slice()),
        ] {
            let path = directory.0.join(name);
            fs::write(&path, signature).unwrap();
            assert!(reject_unsupported_container(&path).is_ok(), "{name}");
        }
    }

    #[test]
    fn import_matrix_accepts_ffv1_pcm_and_ignores_other_streams() {
        let json = br#"{
            "format": {"format_name":"matroska,webm"},
            "streams": [
                {"index":0,"codec_type":"video","codec_name":"ffv1","width":16,"height":16},
                {"index":1,"codec_type":"audio","codec_name":"pcm_s16le","sample_rate":"48000","channels":2},
                {"index":2,"codec_type":"subtitle","codec_name":"subrip"}
            ]
        }"#;
        let metadata = parse_probe_json(json, 2048).unwrap();
        enforce_import_matrix(&metadata).unwrap();
    }

    #[test]
    fn import_matrix_accepts_vp9_opus_ebml_media() {
        let json = br#"{
            "format": {"format_name":"matroska,webm"},
            "streams": [
                {"index":0,"codec_type":"video","codec_name":"vp9","width":64,"height":48},
                {"index":1,"codec_type":"audio","codec_name":"opus","sample_rate":"48000","channels":2}
            ]
        }"#;

        enforce_import_matrix(&parse_probe_json(json, 2048).unwrap()).unwrap();
    }

    #[test]
    fn import_matrix_accepts_h264_aac_matroska_media() {
        for json in [
            br#"{"format":{"format_name":"matroska,webm"},"streams":[{"index":0,"codec_type":"video","codec_name":"h264","width":1920,"height":1080},{"index":1,"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":6}]}"#.as_slice(),
            br#"{"format":{"format_name":"matroska,webm"},"streams":[{"index":0,"codec_type":"video","codec_name":"h264","width":1920,"height":1080}]}"#.as_slice(),
            br#"{"format":{"format_name":"matroska,webm"},"streams":[{"index":0,"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2}]}"#.as_slice(),
        ] {
            enforce_import_matrix(&parse_probe_json(json, 4096).unwrap()).unwrap();
        }
    }

    #[test]
    fn import_matrix_accepts_audio_only_pcm_wav_and_rejects_non_audio_wav() {
        let audio = br#"{"format":{"format_name":"wav"},"streams":[{"index":0,"codec_type":"audio","codec_name":"pcm_s16le","sample_rate":"48000","channels":2}]}"#;
        enforce_import_matrix(&parse_probe_json(audio, 1024).unwrap()).unwrap();

        for json in [
            br#"{"format":{"format_name":"wav"},"streams":[{"index":0,"codec_type":"audio","codec_name":"speex","sample_rate":"48000","channels":2}]}"#.as_slice(),
            br#"{"format":{"format_name":"wav"},"streams":[{"index":0,"codec_type":"video","codec_name":"ffv1","width":16,"height":16}]}"#.as_slice(),
            br#"{"format":{"format_name":"wav"},"streams":[]}"#.as_slice(),
        ] {
            assert_eq!(
                enforce_import_matrix(&parse_probe_json(json, 1024).unwrap())
                    .unwrap_err()
                    .code(),
                MediaProbeErrorCode::UnsupportedCodec,
            );
        }
    }

    #[test]
    fn riff_and_rf64_wave_headers_are_accepted_but_other_riff_is_not() {
        let directory = TestDirectory::new();
        for (name, signature, supported) in [
            ("pcm.wav", b"RIFF0000WAVE".as_slice(), true),
            ("large.wav", b"RF640000WAVE".as_slice(), true),
            ("riff.avi", b"RIFF0000AVI ".as_slice(), true),
            ("riff.wma", b"RIFF0000WMA ".as_slice(), false),
        ] {
            let path = directory.0.join(name);
            fs::write(&path, signature).unwrap();
            assert_eq!(reject_unsupported_container(&path).is_ok(), supported);
        }
    }

    #[test]
    fn iso_bmff_ftyp_header_is_accepted_but_truncated_or_invalid_boxes_are_not() {
        let directory = TestDirectory::new();
        for (name, header, supported) in [
            ("phone.mp4", b"\0\0\0\x10ftypisom0000".as_slice(), true),
            ("short.mp4", b"\0\0\0\x10ftyp".as_slice(), false),
            ("not-mp4.bin", b"\0\0\0\x18freeisom0000".as_slice(), false),
        ] {
            let path = directory.0.join(name);
            fs::write(&path, header).unwrap();
            assert_eq!(reject_unsupported_container(&path).is_ok(), supported);
        }
    }

    #[test]
    fn import_matrix_accepts_h264_aac_iso_bmff_and_audio_only_aac() {
        for json in [
            br#"{"format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2"},"streams":[{"index":0,"codec_type":"video","codec_name":"h264","width":32,"height":24},{"index":1,"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":1}]}"#.as_slice(),
            br#"{"format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2"},"streams":[{"index":0,"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2}]}"#.as_slice(),
            br#"{"format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2"},"streams":[{"index":0,"codec_type":"video","codec_name":"hevc","width":32,"height":24},{"index":1,"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2}]}"#.as_slice(),
            br#"{"format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2"},"streams":[{"index":0,"codec_type":"audio","codec_name":"alac","sample_rate":"44100","channels":2}]}"#.as_slice(),
        ] {
            enforce_import_matrix(&parse_probe_json(json, 2048).unwrap()).unwrap();
        }

        for json in [
            br#"{"format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2"},"streams":[{"index":0,"codec_type":"audio","codec_name":"speex","sample_rate":"48000","channels":2}]}"#.as_slice(),
            br#"{"format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2"},"streams":[]}"#.as_slice(),
        ] {
            assert_eq!(
                enforce_import_matrix(&parse_probe_json(json, 2048).unwrap())
                    .unwrap_err()
                    .code(),
                MediaProbeErrorCode::UnsupportedCodec,
            );
        }
    }

    #[test]
    fn import_matrix_accepts_only_supported_still_image_codecs() {
        for (format, codec) in [
            ("png_pipe", "png"),
            ("jpeg_pipe", "mjpeg"),
            ("webp_pipe", "webp"),
            ("image2", "png"),
            ("image2", "mjpeg"),
            ("image2", "webp"),
        ] {
            let json = format!(
                r#"{{"format":{{"format_name":"{format}"}},"streams":[{{"index":0,"codec_type":"video","codec_name":"{codec}","width":16,"height":16}}]}}"#
            );
            let metadata = parse_probe_json(json.as_bytes(), 2048).unwrap();
            enforce_import_matrix(&metadata).unwrap();
        }

        for json in [
            br#"{"format":{"format_name":"png_pipe"},"streams":[{"index":0,"codec_type":"video","codec_name":"mjpeg","width":16,"height":16}]}"#.as_slice(),
            br#"{"format":{"format_name":"jpeg_pipe"},"streams":[{"index":0,"codec_type":"video","codec_name":"bmp","width":16,"height":16}]}"#.as_slice(),
            br#"{"format":{"format_name":"webp_pipe"},"streams":[{"index":0,"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2}]}"#.as_slice(),
        ] {
            assert_eq!(
                enforce_import_matrix(&parse_probe_json(json, 2048).unwrap())
                    .unwrap_err()
                    .code(),
                MediaProbeErrorCode::UnsupportedCodec,
            );
        }
    }

    #[test]
    fn still_image_probe_durations_do_not_limit_timeline_source_range() {
        let json = br#"{
            "format": {"format_name":"image2", "duration":"0.040000"},
            "streams": [{
                "index":0,
                "codec_type":"video",
                "codec_name":"png",
                "width":16,
                "height":16,
                "avg_frame_rate":"25/1",
                "duration":"0.040000"
            }]
        }"#;

        let metadata = parse_probe_json(json, 2048).unwrap();
        enforce_import_matrix(&metadata).unwrap();

        assert!(metadata.duration().is_none());
        match &metadata.streams()[0] {
            MediaStreamMetadata::Video(video) => assert!(video.duration().is_none()),
            _ => panic!("still images must probe as one video stream"),
        }
    }

    #[test]
    fn import_matrix_rejects_foreign_codecs_and_containers() {
        let unnamed = br#"{"format":{"format_name":"matroska,webm"},"streams":[{"index":0,"codec_type":"video","width":16,"height":16}]}"#;
        let hap = br#"{"format":{"format_name":"matroska,webm"},"streams":[{"index":0,"codec_type":"video","codec_name":"hap","width":16,"height":16}]}"#;
        let speex = br#"{"format":{"format_name":"matroska,webm"},"streams":[{"index":0,"codec_type":"audio","codec_name":"speex","sample_rate":"48000","channels":2}]}"#;
        for json in [unnamed as &[u8], hap as &[u8], speex as &[u8]] {
            let metadata = parse_probe_json(json, 64).unwrap();
            assert_eq!(
                enforce_import_matrix(&metadata).unwrap_err().code(),
                MediaProbeErrorCode::UnsupportedCodec,
            );
        }

        for json in [
            br#"{"format":{"format_name":"matroska,webm"},"streams":[{"index":0,"codec_type":"video","codec_name":"hevc","width":16,"height":16}]}"#.as_slice(),
            br#"{"format":{"format_name":"matroska,webm"},"streams":[{"index":0,"codec_type":"video","codec_name":"av1","width":16,"height":16}]}"#.as_slice(),
            br#"{"format":{"format_name":"matroska,webm"},"streams":[{"index":0,"codec_type":"audio","codec_name":"pcm_f32le","sample_rate":"48000","channels":2}]}"#.as_slice(),
            br#"{"format":{"format_name":"matroska,webm"},"streams":[{"index":0,"codec_type":"audio","codec_name":"flac","sample_rate":"48000","channels":2}]}"#.as_slice(),
            br#"{"format":{"format_name":"mov,mp4"},"streams":[{"index":0,"codec_type":"video","codec_name":"prores","width":1920,"height":1080}]}"#.as_slice(),
        ] {
            enforce_import_matrix(&parse_probe_json(json, 4096).unwrap()).unwrap();
        }

        let container =
            br#"{"format":{"format_name":"asf"},"streams":[{"index":0,"codec_type":"video","codec_name":"h264","width":16,"height":16}]}"#;
        let metadata = parse_probe_json(container, 64).unwrap();
        assert_eq!(
            enforce_import_matrix(&metadata).unwrap_err().code(),
            MediaProbeErrorCode::UnsupportedContainer,
        );
    }

    #[test]
    fn rational_rate_parser_keeps_common_rates_exact_and_rejects_invalid_values() {
        for (text, numerator, denominator) in [
            ("24/1", 24, 1),
            ("24000/1001", 24_000, 1001),
            ("30000/1001", 30_000, 1001),
        ] {
            assert_eq!(
                parse_rational_rate(text).unwrap(),
                RationalRate::new(numerator, denominator).unwrap()
            );
        }
        for text in ["0/0", "0/1", "N/A", "abc", "24", "24/", "/1"] {
            assert_eq!(
                parse_rational_rate(text),
                Err(RationalRateParseError::Invalid)
            );
        }
        assert_eq!(
            parse_rational_rate("4294967296/1"),
            Err(RationalRateParseError::Overflow)
        );
    }

    #[test]
    fn missing_backend_is_a_structured_error() {
        let directory = TestDirectory::new();
        let media = directory.file("input.mkv");
        let error = backend(&directory.0.join("does-not-exist"))
            .probe(&media)
            .unwrap_err();
        assert_eq!(error.code(), MediaProbeErrorCode::ProbeBackendUnavailable);
    }

    #[test]
    fn directories_are_rejected_before_spawning_the_probe() {
        let directory = TestDirectory::new();
        let error = backend(&directory.executable())
            .probe(&directory.0)
            .unwrap_err();
        assert_eq!(error.code(), MediaProbeErrorCode::MediaNotRegularFile);
    }

    #[test]
    fn timeout_kills_the_probe_process_and_returns_promptly() {
        let directory = TestDirectory::new();
        let media = directory.file("sleep.mkv");
        let executable = directory.executable();
        let mut backend = backend(&executable);
        backend.timeout = Duration::from_millis(100);
        let started = Instant::now();
        let error = backend.probe(&media).unwrap_err();
        assert_eq!(error.code(), MediaProbeErrorCode::ProbeTimeout);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn oversized_stdout_is_bounded_and_kills_the_probe_process() {
        let directory = TestDirectory::new();
        let media = directory.file("oversized.mkv");
        let executable = directory.executable();
        let mut backend = backend(&executable);
        backend.stdout_limit = 32;
        let error = backend.probe(&media).unwrap_err();
        assert_eq!(error.code(), MediaProbeErrorCode::ProbeOutputTooLarge);
    }

    #[test]
    fn oversized_stderr_is_bounded_and_kills_the_probe_process() {
        let directory = TestDirectory::new();
        let media = directory.file("oversized-stderr.mkv");
        let executable = directory.executable();
        let mut backend = backend(&executable);
        backend.stderr_limit = 32;
        let error = backend.probe(&media).unwrap_err();
        assert_eq!(error.code(), MediaProbeErrorCode::ProbeOutputTooLarge);
    }

    #[test]
    fn nonzero_exit_has_a_bounded_sanitized_diagnostic() {
        let directory = TestDirectory::new();
        let media = directory.file("failure.mkv");
        let executable = directory.executable();
        let error = backend(&executable).probe(&media).unwrap_err();
        assert_eq!(error.code(), MediaProbeErrorCode::ProbeFailed);
        let diagnostic = error.diagnostic().unwrap();
        assert!(diagnostic.chars().count() <= MAX_DIAGNOSTIC_CHARS);
        assert!(!diagnostic.contains(&directory.0.to_string_lossy().to_string()));
        // The user-facing sentence carries the backend diagnostic; a generic
        // failure sentence would leave an agent unable to tell why a file failed.
        assert_eq!(
            error.to_string(),
            format!("{}: {diagnostic}", error.message())
        );
    }

    #[test]
    fn media_path_with_spaces_and_unicode_is_one_native_argument() {
        let directory = TestDirectory::new();
        let media = directory.file("path with spaces-媒体.mkv");
        let executable = directory.executable();
        let metadata = backend(&executable).probe(&media).unwrap();
        assert!(metadata.format_names().contains(&"matroska".to_owned()));
    }
}
