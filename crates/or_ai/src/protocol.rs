use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::TranscriptIdentity;

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_CONTROL_MESSAGE_BYTES: usize = 1024 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4096;
const MAX_STAGE_BYTES: usize = 128;
const MAX_LANGUAGE_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiTask {
    Transcribe,
    Translate,
    TextToSpeech,
    Segment,
    DetectScene,
    PlanEdit,
    GenerateImage,
    GenerateVideo,
    GenerateMotionScene,
    GenerateAudio,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedFileRef {
    pub id: Uuid,
    pub size_bytes: u64,
    pub sha256: String,
}

impl ManagedFileRef {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.sha256.len() != 64
            || !self.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || self.size_bytes == 0
        {
            return Err(ProtocolError::InvalidMessage);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranscribeRequest {
    pub input: ManagedFileRef,
    pub identity: TranscriptIdentity,
    pub language: Option<String>,
    pub max_duration_ms: u64,
}

impl TranscribeRequest {
    fn validate(&self) -> Result<(), ProtocolError> {
        self.input.validate()?;
        if self.identity.validate().is_err()
            || self.language.as_ref().is_some_and(|language| {
                language.is_empty()
                    || language.len() > MAX_LANGUAGE_BYTES
                    || language.contains('\n')
                    || language.contains('\r')
            })
            || self.max_duration_ms == 0
        {
            return Err(ProtocolError::InvalidMessage);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderErrorCode {
    Unavailable,
    InvalidRequest,
    UnsupportedTask,
    InputUnavailable,
    ModelUnavailable,
    ModelInvalid,
    ResourceLimit,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProviderMessage {
    Ready {
        capabilities: Vec<AiTask>,
    },
    StartTranscription {
        request: TranscribeRequest,
    },
    Progress {
        fraction: f32,
        stage: String,
    },
    Result {
        output: ManagedFileRef,
    },
    Error {
        code: ProviderErrorCode,
        message: String,
    },
    Cancel,
    Cancelled,
    Shutdown,
    ShutdownComplete,
}

impl ProviderMessage {
    fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::Ready { capabilities } if capabilities.len() > 32 => {
                Err(ProtocolError::InvalidMessage)
            }
            Self::StartTranscription { request } => request.validate(),
            Self::Progress { fraction, stage }
                if !fraction.is_finite()
                    || !(0.0..=1.0).contains(fraction)
                    || stage.is_empty()
                    || stage.len() > MAX_STAGE_BYTES
                    || stage.contains('\n')
                    || stage.contains('\r') =>
            {
                Err(ProtocolError::InvalidMessage)
            }
            Self::Result { output }
                if output.size_bytes > crate::MAX_TRANSCRIPT_PROPOSAL_BYTES as u64 =>
            {
                Err(ProtocolError::InvalidMessage)
            }
            Self::Result { output } => output.validate(),
            Self::Error { message, .. }
                if message.is_empty()
                    || message.len() > MAX_ERROR_MESSAGE_BYTES
                    || message.contains('\0') =>
            {
                Err(ProtocolError::InvalidMessage)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolEnvelope {
    pub protocol_version: u16,
    pub request_id: Uuid,
    pub message: ProviderMessage,
}

impl ProtocolEnvelope {
    pub fn new(request_id: Uuid, message: ProviderMessage) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            request_id,
            message,
        }
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion);
        }
        self.message.validate()
    }
}

#[derive(Debug)]
pub enum ProtocolError {
    Io(io::Error),
    MessageTooLarge,
    InvalidUtf8,
    InvalidMessage,
    UnsupportedVersion,
    UnterminatedLine,
}

impl From<io::Error> for ProtocolError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Reads one bounded UTF-8 JSON object terminated by a single newline.
///
/// A malformed or over-limit line is terminal to the caller: it must not skip
/// the frame and continue trusting bytes from the same provider stream.
pub fn read_message(reader: impl BufRead) -> Result<ProtocolEnvelope, ProtocolError> {
    let mut frame = Vec::new();
    reader
        .take((MAX_CONTROL_MESSAGE_BYTES + 1) as u64)
        .read_until(b'\n', &mut frame)?;
    if frame.len() > MAX_CONTROL_MESSAGE_BYTES {
        return Err(ProtocolError::MessageTooLarge);
    }
    if !frame.ends_with(b"\n") {
        return Err(ProtocolError::UnterminatedLine);
    }
    frame.pop();
    if frame.is_empty() {
        return Err(ProtocolError::InvalidMessage);
    }
    std::str::from_utf8(&frame).map_err(|_| ProtocolError::InvalidUtf8)?;
    let envelope: ProtocolEnvelope =
        serde_json::from_slice(&frame).map_err(|_| ProtocolError::InvalidMessage)?;
    envelope.validate()?;
    Ok(envelope)
}

/// Writes one bounded UTF-8 JSON object and flushes its newline-delimited frame.
pub fn write_message(
    mut writer: impl Write,
    message: &ProtocolEnvelope,
) -> Result<(), ProtocolError> {
    message.validate()?;
    let frame = serde_json::to_vec(message).map_err(|_| ProtocolError::InvalidMessage)?;
    if frame.len() >= MAX_CONTROL_MESSAGE_BYTES {
        return Err(ProtocolError::MessageTooLarge);
    }
    writer.write_all(&frame)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn request(message: ProviderMessage) -> ProtocolEnvelope {
        ProtocolEnvelope::new(Uuid::from_u128(1), message)
    }

    fn identity() -> TranscriptIdentity {
        TranscriptIdentity {
            project_id: test_uuid(3),
            project_revision: 7,
            media_id: test_uuid(4),
            media_sha256: "b".repeat(64),
            provider_id: "whisper.cpp".to_owned(),
            model_id: "whisper-tiny-en".to_owned(),
            model_version: "1".to_owned(),
            model_license: "MIT".to_owned(),
        }
    }

    fn test_uuid(id: u128) -> Uuid {
        Uuid::from_u128(0x0000_0000_0000_4000_8000_0000_0000_0000 | id)
    }

    #[test]
    fn transcribe_messages_roundtrip_in_one_newline_delimited_frame() {
        let message = request(ProviderMessage::StartTranscription {
            request: TranscribeRequest {
                input: ManagedFileRef {
                    id: Uuid::from_u128(2),
                    size_bytes: 32,
                    sha256: "a".repeat(64),
                },
                identity: identity(),
                language: Some("en".to_owned()),
                max_duration_ms: 60_000,
            },
        });
        let mut frame = Vec::new();
        write_message(&mut frame, &message).unwrap();

        assert_eq!(frame.last(), Some(&b'\n'));
        assert_eq!(read_message(Cursor::new(frame)).unwrap(), message);
    }

    #[test]
    fn rejects_unterminated_malformed_oversized_and_unknown_fields() {
        let mut unknown_field = br#"{"protocol_version":1,"request_id":"00000000-0000-0000-0000-000000000001","message":{"kind":"cancel"},"extra":true}"#.to_vec();
        unknown_field.push(b'\n');
        assert!(matches!(
            read_message(Cursor::new(b"{}")),
            Err(ProtocolError::UnterminatedLine)
        ));
        assert!(matches!(
            read_message(Cursor::new(b"{broken}\n")),
            Err(ProtocolError::InvalidMessage)
        ));
        assert!(matches!(
            read_message(Cursor::new(vec![b'x'; MAX_CONTROL_MESSAGE_BYTES + 1])),
            Err(ProtocolError::MessageTooLarge)
        ));
        assert!(matches!(
            read_message(Cursor::new(unknown_field)),
            Err(ProtocolError::InvalidMessage)
        ));
    }

    #[test]
    fn validates_versions_digests_and_progress_at_the_boundary() {
        let mut message = request(ProviderMessage::Cancel);
        message.protocol_version = PROTOCOL_VERSION + 1;
        assert!(matches!(
            message.validate(),
            Err(ProtocolError::UnsupportedVersion)
        ));

        assert!(matches!(
            request(ProviderMessage::Progress {
                fraction: f32::NAN,
                stage: "inference".to_owned(),
            })
            .validate(),
            Err(ProtocolError::InvalidMessage)
        ));

        assert!(matches!(
            request(ProviderMessage::Result {
                output: ManagedFileRef {
                    id: Uuid::from_u128(2),
                    size_bytes: 1,
                    sha256: "../invalid".to_owned(),
                },
            })
            .validate(),
            Err(ProtocolError::InvalidMessage)
        ));

        assert!(matches!(
            request(ProviderMessage::Result {
                output: ManagedFileRef {
                    id: Uuid::from_u128(2),
                    size_bytes: crate::MAX_TRANSCRIPT_PROPOSAL_BYTES as u64 + 1,
                    sha256: "a".repeat(64),
                },
            })
            .validate(),
            Err(ProtocolError::InvalidMessage)
        ));

        let mut invalid_identity = identity();
        invalid_identity.provider_id = "whisper\nother".to_owned();
        assert!(matches!(
            request(ProviderMessage::StartTranscription {
                request: TranscribeRequest {
                    input: ManagedFileRef {
                        id: Uuid::from_u128(2),
                        size_bytes: 1,
                        sha256: "a".repeat(64),
                    },
                    identity: invalid_identity,
                    language: None,
                    max_duration_ms: 1_000,
                },
            })
            .validate(),
            Err(ProtocolError::InvalidMessage)
        ));
    }

    #[test]
    fn rejects_non_v4_project_and_media_ids_in_transcription_requests() {
        for mutate in [
            |identity: &mut TranscriptIdentity| identity.project_id = Uuid::nil(),
            |identity: &mut TranscriptIdentity| identity.media_id = Uuid::from_u128(4),
        ] {
            let mut invalid = identity();
            mutate(&mut invalid);
            let message = request(ProviderMessage::StartTranscription {
                request: TranscribeRequest {
                    input: ManagedFileRef {
                        id: Uuid::from_u128(2),
                        size_bytes: 1,
                        sha256: "a".repeat(64),
                    },
                    identity: invalid,
                    language: None,
                    max_duration_ms: 1_000,
                },
            });
            assert!(matches!(
                message.validate(),
                Err(ProtocolError::InvalidMessage)
            ));
        }
    }
}
