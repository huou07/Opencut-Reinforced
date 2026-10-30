//! Noise reduction over a media file, through the packaged FFmpeg helper.
//!
//! This is the audio half of Phase 4: the deliberately enabled `afftdn` filter,
//! driven by a bounded helper process. It is not a project mutation — it reads one
//! media file and writes another, exactly like an export — so it produces an
//! ordinary audio file the user can import, compare against the original, and
//! discard. That keeps noise reduction out of the project format, which does not
//! need a field for it.
//!
//! The noise floor is a *parameter*, not a constant: FFmpeg's FFT denoiser does
//! nothing when it is told the floor is below the recording's real one, and eats the
//! audio when it is told the floor is above it. Callers therefore pass the figure
//! they measured for the material, and [`audio_cleanup_arguments`] is a pure
//! function so the exact chain can be asserted without running a process.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::media_helpers::ffmpeg_executable;

/// How long a clean-up may run before it is killed. Reducing noise on a long
/// recording is slower than real time, so this is generous; it exists so a wedged
/// helper cannot hang a caller forever.
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(900);

/// How often the bounded wait checks the child.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// The noisiest floor this accepts, and the quietest. The range is wide enough for
/// real material and narrow enough that a typo is refused rather than run.
const MIN_NOISE_FLOOR_DB: i32 = -120;
const MAX_NOISE_FLOOR_DB: i32 = -1;

/// FFmpeg's own reduction range for `afftdn`.
const MAX_REDUCTION_DB: u32 = 97;

/// The statistics window the floor estimate averages over. Short enough that a
/// pause in speech is several windows, long enough that one sample is not noise.
/// Mono sample rate of the temporary decode, and the window it is cut into.
const SAMPLE_RATE: u32 = 48_000;
/// 100 ms at [`SAMPLE_RATE`], the same window the removed filter chain used.
const WINDOW_SAMPLES: usize = (SAMPLE_RATE as usize) / 10;

/// The key `ametadata` prints for each window's RMS level, without the separator its
/// How far above the quietest measured windows the filter's floor is set.
const NOISE_FLOOR_MARGIN_DB: i32 = 2;

/// The percentile of window levels used as the floor estimate: the quietest tenth.
const NOISE_FLOOR_PERCENTILE: f64 = 0.1;

/// A request to reduce noise in one media file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioCleanupRequest {
    /// The file to read. It is never modified.
    pub source: PathBuf,
    /// Where the cleaned audio is written.
    pub destination: PathBuf,
    /// The measured noise floor of the source, in dBFS. This is the parameter a
    /// hard-coded value gets wrong in both directions.
    pub noise_floor_db: i32,
    /// How much the filter may reduce, in dB.
    pub reduction_db: u32,
}

/// What a completed clean-up produced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioCleanupResult {
    pub destination: PathBuf,
    pub noise_floor_db: i32,
    pub reduction_db: u32,
    pub bytes_written: u64,
}

/// Why a clean-up did not complete.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioCleanupErrorCode {
    /// The request cannot be run as given.
    InvalidArguments,
    /// The packaged helper could not be started.
    BackendUnavailable,
    /// The helper ran and failed, or wrote nothing usable.
    GenerationFailed,
    /// The helper exceeded its bound and was killed.
    TimedOut,
    /// The source has no measurable audio, so no floor can be derived from it.
    ///
    /// Distinct from [`Self::GenerationFailed`] because it is a fact about the
    /// material rather than a failure of the run: a silent file has no noise to
    /// remove, and the caller can act on that (say so, or pass an explicit floor)
    /// instead of retrying.
    SilentSource,
}

impl AudioCleanupErrorCode {
    /// The stable wire spelling a client sees.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArguments => "INVALID_ARGUMENTS",
            Self::BackendUnavailable => "CLEANUP_BACKEND_UNAVAILABLE",
            Self::GenerationFailed => "CLEANUP_GENERATION_FAILED",
            Self::TimedOut => "CLEANUP_TIMED_OUT",
            Self::SilentSource => "CLEANUP_SILENT_SOURCE",
        }
    }
}

/// A clean-up failure with its stable code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioCleanupError {
    code: AudioCleanupErrorCode,
}

impl AudioCleanupError {
    pub fn new(code: AudioCleanupErrorCode) -> Self {
        Self { code }
    }

    pub fn code(self) -> AudioCleanupErrorCode {
        self.code
    }
}

/// The exact helper arguments a request produces.
///
/// Pure, so a test can assert the chain without a helper process: the floor and the
/// reduction come from the request, the video is dropped (`-vn`), and the output is
/// written in the PCM container the rest of the product reads.
pub fn audio_cleanup_arguments(request: &AudioCleanupRequest) -> Vec<OsString> {
    [
        "-hide_banner".to_owned(),
        "-nostdin".to_owned(),
        "-loglevel".to_owned(),
        "error".to_owned(),
        "-y".to_owned(),
        "-i".to_owned(),
        request.source.display().to_string(),
        "-vn".to_owned(),
        "-af".to_owned(),
        format!(
            "afftdn=nf={}:nr={}",
            request.noise_floor_db, request.reduction_db
        ),
        "-c:a".to_owned(),
        "pcm_s16le".to_owned(),
        request.destination.display().to_string(),
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}

/// Reduces noise in `request.source`, writing `request.destination`.
///
/// The process is bounded: a helper that does not finish inside [`CLEANUP_TIMEOUT`]
/// is killed and reported as [`AudioCleanupErrorCode::TimedOut`] rather than left
/// running. An existing destination is only replaced by a successful run, because
/// the filter writes it directly.
pub fn clean_audio(request: &AudioCleanupRequest) -> Result<AudioCleanupResult, AudioCleanupError> {
    if !request.source.is_file() {
        return Err(AudioCleanupError::new(
            AudioCleanupErrorCode::InvalidArguments,
        ));
    }
    if request.destination.as_os_str().is_empty()
        || request.destination == request.source
        || request.destination.is_dir()
    {
        return Err(AudioCleanupError::new(
            AudioCleanupErrorCode::InvalidArguments,
        ));
    }
    if !(MIN_NOISE_FLOOR_DB..=MAX_NOISE_FLOOR_DB).contains(&request.noise_floor_db)
        || request.reduction_db == 0
        || request.reduction_db > MAX_REDUCTION_DB
    {
        return Err(AudioCleanupError::new(
            AudioCleanupErrorCode::InvalidArguments,
        ));
    }
    if let Some(parent) = request.destination.parent()
        && !parent.as_os_str().is_empty()
        && !parent.is_dir()
    {
        return Err(AudioCleanupError::new(
            AudioCleanupErrorCode::InvalidArguments,
        ));
    }

    let executable = ffmpeg_executable();
    let mut child = Command::new(&executable)
        .args(audio_cleanup_arguments(request))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| AudioCleanupError::new(AudioCleanupErrorCode::BackendUnavailable))?;

    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let _ = std::fs::remove_file(&request.destination);
                    return Err(AudioCleanupError::new(
                        AudioCleanupErrorCode::GenerationFailed,
                    ));
                }
                break;
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(&request.destination);
                    return Err(AudioCleanupError::new(AudioCleanupErrorCode::TimedOut));
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = std::fs::remove_file(&request.destination);
                return Err(AudioCleanupError::new(
                    AudioCleanupErrorCode::GenerationFailed,
                ));
            }
        }
    }

    let bytes_written = std::fs::metadata(&request.destination)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if bytes_written == 0 {
        let _ = std::fs::remove_file(&request.destination);
        return Err(AudioCleanupError::new(
            AudioCleanupErrorCode::GenerationFailed,
        ));
    }

    Ok(AudioCleanupResult {
        destination: request.destination.clone(),
        noise_floor_db: request.noise_floor_db,
        reduction_db: request.reduction_db,
        bytes_written,
    })
}

/// Every measured window's RMS level in dBFS, in the order the helper printed them.
///
/// Pure, so the shape of the helper's output is tested without running one. Lines
/// that are not an RMS figure — the `frame:`/`pts:` headers `ametadata` also prints —
/// are ignored, and a run with no figures at all is an empty vector rather than an
/// error. This keeps a helper failure distinguishable from a silent recording.
///
/// The value at `percentile` (0.0..=1.0) of a sample, using nearest-rank.
///
/// The floor estimate is a low percentile rather than the mean: a recording's mean
/// level is dominated by whatever is being said, while its quietest windows are the
/// noise. A low percentile of the window levels is therefore the closest thing to the
/// floor that needs no model.
pub fn percentile(samples: &[f64], percentile: f64) -> Option<f64> {
    if samples.is_empty() || !(0.0..=1.0).contains(&percentile) {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let rank = ((percentile * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    sorted.get(rank - 1).copied()
}

/// The helper arguments that decode the source to one mono WAV.
///
/// The measurement cannot be delegated to an FFmpeg filter chain: the FFmpeg the
/// app ships is built for media interchange only, without the `null` muxer, the
/// raw PCM muxers or the `astats`/`ametadata` filters. A WAV is the one container
/// every build writes, and the windows are then computed here.
fn statistics_arguments(source: &Path, decoded: &Path) -> Vec<OsString> {
    [
        "-hide_banner".to_owned(),
        "-nostdin".to_owned(),
        "-loglevel".to_owned(),
        "error".to_owned(),
        "-y".to_owned(),
        "-i".to_owned(),
        source.display().to_string(),
        "-vn".to_owned(),
        "-ac".to_owned(),
        "1".to_owned(),
        "-ar".to_owned(),
        SAMPLE_RATE.to_string(),
        "-f".to_owned(),
        "wav".to_owned(),
        decoded.display().to_string(),
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}

/// Reads the little-endian samples of a 16-bit mono WAV file.
///
/// The chunks are walked rather than assumed: `LIST` and `fact` chunks legitimately
/// precede `data`, and reading from a fixed offset would decode header bytes as
/// audio.
fn read_wav_pcm16(path: &Path) -> Option<Vec<i16>> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        let body = offset + 8;
        if id == b"data" {
            let end = body.saturating_add(size).min(bytes.len());
            return Some(
                bytes[body..end]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| i16::from_le_bytes(*pair))
                    .collect(),
            );
        }
        // Chunks are word aligned, so an odd size is followed by one pad byte.
        offset = body.saturating_add(size).saturating_add(size % 2);
    }
    None
}

/// Root-mean-square level of each non-overlapping window, in dBFS.
///
/// The windows are the measurement: a percentile over them answers "how quiet is
/// this recording when nobody is speaking", which a single average cannot. A window
/// of pure digital silence has no finite level and is skipped rather than reported
/// as negative infinity.
pub fn window_rms_dbfs(samples: &[i16], window_len: usize) -> Vec<f64> {
    if window_len == 0 {
        return Vec::new();
    }
    samples
        .chunks(window_len)
        .filter_map(|window| {
            if window.is_empty() {
                return None;
            }
            let sum = window
                .iter()
                .map(|sample| {
                    let value = f64::from(*sample) / 32_768.0;
                    value * value
                })
                .sum::<f64>();
            let mean = sum / window.len() as f64;
            if mean <= 0.0 {
                return None;
            }
            Some(10.0 * mean.log10())
        })
        .collect()
}

pub fn measure_noise_floor_db(
    source: &Path,
    percentile_rank: f64,
) -> Result<i32, AudioCleanupError> {
    if !source.is_file() {
        return Err(AudioCleanupError::new(
            AudioCleanupErrorCode::InvalidArguments,
        ));
    }
    let executable = ffmpeg_executable();
    let decoded = std::env::temp_dir().join(format!(
        "or-noise-floor-{}-{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default()
    ));
    let arguments = statistics_arguments(source, &decoded);

    let mut child = Command::new(&executable)
        .args(&arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| AudioCleanupError::new(AudioCleanupErrorCode::BackendUnavailable))?;
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(&decoded);
                    return Err(AudioCleanupError::new(AudioCleanupErrorCode::TimedOut));
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = std::fs::remove_file(&decoded);
                return Err(AudioCleanupError::new(
                    AudioCleanupErrorCode::GenerationFailed,
                ));
            }
        }
    };
    if !status.success() {
        let _ = std::fs::remove_file(&decoded);
        return Err(AudioCleanupError::new(
            AudioCleanupErrorCode::GenerationFailed,
        ));
    }

    let samples = read_wav_pcm16(&decoded);
    let _ = std::fs::remove_file(&decoded);
    let samples =
        samples.ok_or_else(|| AudioCleanupError::new(AudioCleanupErrorCode::GenerationFailed))?;
    let windows = window_rms_dbfs(&samples, WINDOW_SAMPLES);
    // A recording with no measurable window is silent, which is a fact about the
    // material rather than a failed run; the caller can say so.
    if windows.is_empty() {
        return Err(AudioCleanupError::new(AudioCleanupErrorCode::SilentSource));
    }
    let floor = percentile(&windows, percentile_rank)
        .ok_or_else(|| AudioCleanupError::new(AudioCleanupErrorCode::GenerationFailed))?;
    // The floor is passed to the filter two dB above the quietest windows: the
    // estimate is a lower bound on the noise, and a filter set exactly at it leaves
    // the noise's own peak.
    let floor = (floor.round() as i32 + NOISE_FLOOR_MARGIN_DB)
        .clamp(MIN_NOISE_FLOOR_DB, MAX_NOISE_FLOOR_DB);
    Ok(floor)
}

/// Measures a file's own noise floor at the default percentile.
pub fn measure_noise_floor(source: &Path) -> Result<i32, AudioCleanupError> {
    measure_noise_floor_db(source, NOISE_FLOOR_PERCENTILE)
}

/// Convenience for a caller that only has paths.
pub fn clean_audio_file(
    source: &Path,
    destination: &Path,
    noise_floor_db: i32,
    reduction_db: u32,
) -> Result<AudioCleanupResult, AudioCleanupError> {
    clean_audio(&AudioCleanupRequest {
        source: source.to_path_buf(),
        destination: destination.to_path_buf(),
        noise_floor_db,
        reduction_db,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> AudioCleanupRequest {
        AudioCleanupRequest {
            source: PathBuf::from("/tmp/or-cleanup-source.wav"),
            destination: PathBuf::from("/tmp/or-cleanup-out.wav"),
            noise_floor_db: -27,
            reduction_db: 18,
        }
    }

    #[test]
    fn the_chain_carries_the_measured_floor_and_the_reduction() {
        let arguments = audio_cleanup_arguments(&request());
        let rendered = arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(rendered.contains(&"afftdn=nf=-27:nr=18".to_owned()));
        // Video is dropped: this is an audio clean-up, not a re-encode.
        assert!(rendered.contains(&"-vn".to_owned()));
        assert!(rendered.iter().any(|argument| argument == "pcm_s16le"));
        assert_eq!(
            rendered.last().map(String::as_str),
            Some("/tmp/or-cleanup-out.wav")
        );
    }

    #[test]
    fn a_missing_source_is_refused_before_the_helper_runs() {
        let error = clean_audio(&AudioCleanupRequest {
            source: PathBuf::from("/tmp/or-no-such-source-4e2.wav"),
            ..request()
        })
        .unwrap_err();
        assert_eq!(error.code(), AudioCleanupErrorCode::InvalidArguments);
    }

    #[test]
    fn implausible_parameters_are_refused() {
        for noise_floor_db in [-200, 0, 12] {
            let error = clean_audio(&AudioCleanupRequest {
                source: std::env::current_exe().unwrap(),
                noise_floor_db,
                ..request()
            })
            .unwrap_err();
            assert_eq!(error.code(), AudioCleanupErrorCode::InvalidArguments);
        }
        let error = clean_audio(&AudioCleanupRequest {
            source: std::env::current_exe().unwrap(),
            reduction_db: 0,
            ..request()
        })
        .unwrap_err();
        assert_eq!(error.code(), AudioCleanupErrorCode::InvalidArguments);
    }

    #[test]
    fn the_statistics_arguments_decode_one_mono_wav() {
        // The bundle's FFmpeg has no null muxer and no astats filter, so the
        // measurement can only be a WAV the crate reads itself.
        let decoded = Path::new("/tmp/or-decoded.wav");
        let arguments = statistics_arguments(Path::new("/tmp/or-source.wav"), decoded)
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(
            arguments.windows(2).any(|pair| pair == ["-ac", "1"]),
            "{arguments:?}"
        );
        assert!(
            arguments.windows(2).any(|pair| pair == ["-f", "wav"]),
            "{arguments:?}"
        );
        assert_eq!(
            arguments.last().map(String::as_str),
            Some("/tmp/or-decoded.wav")
        );
        assert!(
            !arguments.iter().any(|argument| argument.contains("astats")),
            "{arguments:?}"
        );
    }

    #[test]
    fn window_rms_reports_levels_and_skips_digital_silence() {
        // Full-scale halving is -6.02 dBFS; a silent window has no finite level.
        let windows = window_rms_dbfs(&[16_384, -16_384, 16_384, -16_384, 0, 0], 2);
        assert_eq!(windows.len(), 2);
        assert!((windows[0] + 6.0206).abs() < 0.01, "{windows:?}");
        assert_eq!(window_rms_dbfs(&[0, 0, 0, 0], 2), Vec::<f64>::new());
    }

    #[test]
    fn a_wav_written_by_the_helper_is_read_back_as_samples() {
        let path = std::env::temp_dir().join(format!("or-wav-read-{}.wav", std::process::id()));
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36u32 + 8).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"LIST");
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(b"INFO");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&[1, 0, 1, 0, 0x80, 0xbb, 0, 0]);
        bytes.extend_from_slice(&[0, 0xee, 2, 0, 2, 0, 16, 0]);
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&1i16.to_le_bytes());
        bytes.extend_from_slice(&(-2i16).to_le_bytes());
        std::fs::write(&path, &bytes).unwrap();
        let samples = read_wav_pcm16(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(samples, Some(vec![1, -2]));
    }

    #[test]
    fn a_silent_source_has_no_measurable_window() {
        // Digital silence has no finite level, so the window set is empty and the
        // caller learns the material has no audio instead of being told the run failed.
        assert!(window_rms_dbfs(&[0i16; 4_800], WINDOW_SAMPLES).is_empty());
        assert_eq!(
            AudioCleanupErrorCode::SilentSource.as_str(),
            "CLEANUP_SILENT_SOURCE"
        );
    }

    #[test]
    fn the_percentile_picks_the_quietest_windows() {
        let windows = vec![
            -30.0, -29.0, -28.0, -12.0, -11.0, -10.0, -9.0, -8.0, -7.0, -6.0,
        ];
        // The quietest tenth of ten windows is the first one.
        assert_eq!(percentile(&windows, 0.1), Some(-30.0));
        assert_eq!(percentile(&windows, 0.5), Some(-11.0));
        assert_eq!(percentile(&windows, 1.0), Some(-6.0));
        assert_eq!(percentile(&[], 0.1), None);
        assert_eq!(percentile(&windows, 1.5), None);
    }

    #[test]
    fn writing_over_the_source_is_refused() {
        let error = clean_audio(&AudioCleanupRequest {
            source: std::env::current_exe().unwrap(),
            destination: std::env::current_exe().unwrap(),
            ..request()
        })
        .unwrap_err();
        assert_eq!(error.code(), AudioCleanupErrorCode::InvalidArguments);
    }

    #[test]
    fn a_missing_helper_is_reported_as_a_backend_failure() {
        // The resolver takes the override first, so a path that cannot exist makes
        // the spawn fail without touching the real helper.
        let previous = std::env::var_os(crate::media_helpers::FFMPEG_PATH_ENVIRONMENT_VARIABLE);
        unsafe {
            std::env::set_var(
                crate::media_helpers::FFMPEG_PATH_ENVIRONMENT_VARIABLE,
                "/tmp/or-no-such-helper-7c1/ffmpeg",
            );
        }
        let error = clean_audio_file(
            &std::env::current_exe().unwrap(),
            Path::new("/tmp/or-out.wav"),
            -30,
            12,
        )
        .unwrap_err();
        match previous {
            Some(value) => unsafe {
                std::env::set_var(
                    crate::media_helpers::FFMPEG_PATH_ENVIRONMENT_VARIABLE,
                    value,
                )
            },
            None => unsafe {
                std::env::remove_var(crate::media_helpers::FFMPEG_PATH_ENVIRONMENT_VARIABLE)
            },
        }
        assert_eq!(error.code(), AudioCleanupErrorCode::BackendUnavailable);
    }
}
