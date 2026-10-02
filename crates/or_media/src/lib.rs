mod decoder;
mod encoder;
mod snapshot_queue;

#[cfg(unix)]
pub use decoder::SeekableMediaIoCapability;
pub use decoder::{AudioChunk, DecodeError, SoftwareMediaDecoder, VideoFrame};
pub use encoder::{ExportEncodeError, MatroskaFfv1PcmS16leWriter};
pub use snapshot_queue::{SnapshotItem, SnapshotQueue, SnapshotQueueSendError};

/// Initializes the dynamically linked FFmpeg runtime and verifies its license.
pub fn verify_ffmpeg_runtime() -> bool {
    ffmpeg_the_third::init().is_ok()
        && ffmpeg_the_third::util::license() == "LGPL version 2.1 or later"
}
