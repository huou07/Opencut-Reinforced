mod decoder;
mod encoder;
mod probe;
#[cfg(unix)]
mod seekable_io;
mod snapshot_queue;

#[cfg(unix)]
pub use decoder::SeekableMediaIoCapability;
pub use decoder::{
    AudioChunk, DecodeError, DecodeOutput, SoftwareMediaDecoder, VideoDecodeSession,
    VideoDecodeSessionMetrics, VideoFrame,
};
pub use encoder::{
    ExportEncodeError, FfmpegExportProfile, FfmpegExportWriter, FfmpegSoftwareExportWriter,
    MatroskaFfv1PcmS16leWriter, SoftwareExportProfile,
};
pub use snapshot_queue::{SnapshotItem, SnapshotQueue, SnapshotQueueSendError};

pub use probe::probe_media_path;

/// Probes a seekable Android document through the file descriptor the SAF
/// capability hands over. It is Unix-only because the capability itself is.
#[cfg(unix)]
pub fn probe_seekable_media(
    capability: &SeekableMediaIoCapability,
) -> Result<Vec<u8>, ffmpeg_the_third::Error> {
    seekable_io::probe(capability)
}

/// Whether this build carries a decoder for `codec_name` (an FFmpeg codec name
/// such as `h264` or `hevc`).
///
/// The release bundle and a local debug build link different FFmpeg builds, so
/// the import matrix cannot know what this particular process can actually
/// play. The bridge asks this question before admitting a file.
pub fn decoder_available(codec_name: &str) -> bool {
    ffmpeg_the_third::codec::decoder::find_by_name(codec_name).is_some()
}

/// Initializes the dynamically linked FFmpeg runtime and verifies its license.
pub fn verify_ffmpeg_runtime() -> bool {
    ffmpeg_runtime_problem().is_none()
}

/// Explains why the linked runtime is not the approved LGPL build. A caller
/// that refuses a request because of this returns the reason, so a packaged
/// failure says what actually went wrong instead of only that it did.
pub fn ffmpeg_runtime_problem() -> Option<String> {
    if let Err(error) = ffmpeg_the_third::init() {
        return Some(format!("the library refused to initialise: {error}"));
    }
    let license = ffmpeg_the_third::util::license();
    if approved_license(license) {
        None
    } else {
        Some(format!(
            "the library reports {license:?} instead of the approved LGPL build (loaded {})",
            loaded_avutil_path().unwrap_or_else(|| "an unknown library".to_owned())
        ))
    }
}

/// Whether the runtime reports a licence this build may ship. Every platform
/// ships LGPL; the Linux delivery build links x264, so a GPL-2 build is the
/// approved runtime there (ADR 0011). Version 3 and nonfree stay refused
/// everywhere, because they report a different string.
fn approved_license(license: &str) -> bool {
    if license == "LGPL version 2.1 or later" {
        return true;
    }
    cfg!(target_os = "linux") && license == "GPL version 2 or later"
}

/// The libavutil this process actually mapped, so a licence refusal names the
/// file instead of only the verdict. Linux only; other targets report unknown.
fn loaded_avutil_path() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let maps = std::fs::read_to_string("/proc/self/maps").ok()?;
        maps.lines()
            .find(|line| line.contains("libavutil.so"))
            .and_then(|line| line.split_whitespace().last())
            .map(std::string::ToString::to_string)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_linked_ffmpeg_runtime_is_an_approved_build() {
        assert!(
            super::verify_ffmpeg_runtime(),
            "the linked FFmpeg runtime must report an approved licence: {:?}",
            ffmpeg_the_third::util::license()
        );
    }
}
