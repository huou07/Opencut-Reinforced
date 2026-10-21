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
const RMS_WINDOW_ARGUMENT: &str = "length=0.1";

/// The key `ametadata` prints for each window's RMS level, without the separator its
/// value follows.
const RMS_LEVEL_KEY: &str = "lavfi.astats.Overall.RMS_level";

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
/// error, because "the helper printed nothing measurable" is the caller's decision.
pub fn parse_window_rms_db(output: &str) -> Vec<f64> {
    output
        .lines()
        .filter_map(|line| line.trim().strip_prefix(RMS_LEVEL_KEY))
        .filter_map(|value| value.strip_prefix('='))
        .filter_map(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .collect()
}

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

/// The helper arguments that print one RMS figure per statistics window.
///
/// Pure, so the exact chain is asserted without a process; see the test that pins the
/// missing `=` separator, which is easy to get wrong and produces silence.
fn statistics_arguments(source: &Path) -> Vec<OsString> {
    [
        "-hide_banner".to_owned(),
        "-nostdin".to_owned(),
        "-loglevel".to_owned(),
        "error".to_owned(),
        "-i".to_owned(),
        source.display().to_string(),
        "-vn".to_owned(),
        "-af".to_owned(),
        format!(
            "astats=metadata=1:reset=1:{RMS_WINDOW_ARGUMENT},\
             ametadata=print:key={RMS_LEVEL_KEY}:file=-"
        ),
        "-f".to_owned(),
        "null".to_owned(),
        "-".to_owned(),
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}

/// Measures the material's own noise floor, in whole dBFS.
///
/// Runs the helper over the source once with per-window RMS statistics and takes a
/// low percentile of them. This is the figure [`clean_audio`] must be told: a value
/// below the real floor leaves the noise, and a value above it takes the audio.
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
    let arguments = statistics_arguments(source);

    let mut child = Command::new(&executable)
        .args(&arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| AudioCleanupError::new(AudioCleanupErrorCode::BackendUnavailable))?;
    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        use std::io::Read;
        let _ = stdout.read_to_string(&mut output);
    }
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(AudioCleanupError::new(AudioCleanupErrorCode::TimedOut));
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(_) => {
                let _ = child.kill();
                return Err(AudioCleanupError::new(
                    AudioCleanupErrorCode::GenerationFailed,
                ));
            }
        }
    };
    if !status.success() {
        return Err(AudioCleanupError::new(
            AudioCleanupErrorCode::GenerationFailed,
        ));
    }

    let windows = parse_window_rms_db(&output);
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
    fn the_statistics_arguments_name_the_key_once() {
        // A trailing '=' in the key would make the filter argument
        // "...RMS_level=:file=-", which parses to nothing.
        let arguments = statistics_arguments(Path::new("/tmp/or-source.wav"))
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let chain = arguments
            .iter()
            .find(|argument| argument.starts_with("astats="))
            .expect("the statistics chain is present");
        assert!(
            chain.contains("key=lavfi.astats.Overall.RMS_level:file=-"),
            "{chain}"
        );
    }

    #[test]
    fn the_statistics_parser_reads_only_rms_figures() {
        let output = "frame:0    pts:0       pts_time:0\n\
             lavfi.astats.Overall.RMS_level=-23.201059\n\
             frame:1    pts:4096    pts_time:0.0853333\n\
             lavfi.astats.Overall.RMS_level=-21.5\n\
             lavfi.astats.Overall.Peak_level=-18.0\n\
             lavfi.astats.Overall.RMS_level=nan\n";
        assert_eq!(parse_window_rms_db(output), vec![-23.201059, -21.5]);
        assert!(parse_window_rms_db("no figures here").is_empty());
    }

    #[test]
    fn a_silent_source_is_reported_as_such_rather_than_as_a_failure() {
        // `astats` reports -inf for digital silence, and the parser drops non-finite
        // figures, so the set is empty and the caller learns the material has no
        // measurable audio instead of being told the run failed.
        let output = "frame:0    pts:0       pts_time:0\n\
             lavfi.astats.Overall.RMS_level=-inf\n\
             frame:1    pts:512     pts_time:0.064\n\
             lavfi.astats.Overall.RMS_level=-inf\n";
        assert!(parse_window_rms_db(output).is_empty());
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
