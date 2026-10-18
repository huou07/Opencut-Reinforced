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
}

impl AudioCleanupErrorCode {
    /// The stable wire spelling a client sees.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArguments => "INVALID_ARGUMENTS",
            Self::BackendUnavailable => "CLEANUP_BACKEND_UNAVAILABLE",
            Self::GenerationFailed => "CLEANUP_GENERATION_FAILED",
            Self::TimedOut => "CLEANUP_TIMED_OUT",
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
                source: PathBuf::from("/etc/hostname"),
                noise_floor_db,
                ..request()
            })
            .unwrap_err();
            assert_eq!(error.code(), AudioCleanupErrorCode::InvalidArguments);
        }
        let error = clean_audio(&AudioCleanupRequest {
            source: PathBuf::from("/etc/hostname"),
            reduction_db: 0,
            ..request()
        })
        .unwrap_err();
        assert_eq!(error.code(), AudioCleanupErrorCode::InvalidArguments);
    }

    #[test]
    fn writing_over_the_source_is_refused() {
        let error = clean_audio(&AudioCleanupRequest {
            source: PathBuf::from("/etc/hostname"),
            destination: PathBuf::from("/etc/hostname"),
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
            Path::new("/etc/hostname"),
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
