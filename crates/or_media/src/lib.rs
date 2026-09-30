mod decoder;
mod snapshot_queue;

pub use decoder::{AudioChunk, DecodeError, SoftwareMediaDecoder, VideoFrame};
pub use snapshot_queue::{SnapshotItem, SnapshotQueue, SnapshotQueueSendError};

/// Initializes the dynamically linked FFmpeg runtime and verifies its license.
pub fn verify_ffmpeg_runtime() -> bool {
    ffmpeg_the_third::init().is_ok()
        && ffmpeg_the_third::util::license() == "LGPL version 2.1 or later"
}
