mod decoder;
mod encoder;
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
    ExportEncodeError, FfmpegSoftwareExportWriter, MatroskaFfv1PcmS16leWriter,
    SoftwareExportProfile,
};
pub use snapshot_queue::{SnapshotItem, SnapshotQueue, SnapshotQueueSendError};

#[cfg(unix)]
pub fn probe_seekable_media(
    capability: &SeekableMediaIoCapability,
) -> Result<Vec<u8>, ffmpeg_the_third::Error> {
    seekable_io::probe(capability)
}

/// Initializes the dynamically linked FFmpeg runtime and verifies its license.
pub fn verify_ffmpeg_runtime() -> bool {
    ffmpeg_the_third::init().is_ok()
        && ffmpeg_the_third::util::license() == "LGPL version 2.1 or later"
}
