mod decoder;
mod encoder;
mod probe;
#[cfg(unix)]
mod seekable_io;
mod snapshot_queue;

#[cfg(unix)]
pub use decoder::SeekableMediaIoCapability;
pub use decoder::{
    AudioChunk, DecodeError, SoftwareMediaDecoder, VideoDecodeSession, VideoDecodeSessionMetrics,
    VideoFrame,
};
pub use encoder::{
    ExportEncodeError, FfmpegExportProfile, FfmpegExportWriter, FfmpegSoftwareExportWriter,
    MatroskaFfv1PcmS16leWriter, SoftwareExportProfile,
};
pub use snapshot_queue::{SnapshotItem, SnapshotQueue, SnapshotQueueSendError};

pub use probe::probe_media_path;

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
    ffmpeg_the_third::init().is_ok()
        && ffmpeg_the_third::util::license() == "LGPL version 2.1 or later"
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_linked_ffmpeg_runtime_is_the_approved_lgpl_build() {
        assert!(
            super::verify_ffmpeg_runtime(),
            "the linked FFmpeg runtime must report LGPL version 2.1 or later"
        );
    }
}
