//! Bounded, provider-independent AI task and proposal contracts.
//!
//! This crate does not load models, access the network, or mutate projects.
//! Provider execution and user consent live above it; accepted proposals are
//! applied only through the normal project command path.

mod model;
mod protocol;
mod transcription;

pub use model::{
    InstalledModel, MAX_MODEL_ARTIFACT_BYTES, ModelManifest, ModelValidationError,
    cleanup_abandoned_model_staging, install_verified_model, remove_installed_model,
    verify_installed_model,
};
pub use protocol::{
    AiTask, MAX_CONTROL_MESSAGE_BYTES, ManagedFileRef, ProtocolEnvelope, ProtocolError,
    ProviderErrorCode, ProviderMessage, TranscribeRequest, read_message, write_message,
};
pub use transcription::{
    MAX_TRANSCRIPT_PROPOSAL_BYTES, MAX_TRANSCRIPT_SEGMENTS, MAX_TRANSCRIPT_TEXT_BYTES,
    TranscriptIdentity, TranscriptProposal, TranscriptSegment, TranscriptValidationError,
};
