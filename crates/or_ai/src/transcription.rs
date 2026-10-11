use serde::{Deserialize, Serialize};
use uuid::{Uuid, Version};

pub const MAX_TRANSCRIPT_SEGMENTS: usize = 10_000;
pub const MAX_TRANSCRIPT_TEXT_BYTES: usize = 16 * 1024;
pub const MAX_TRANSCRIPT_PROPOSAL_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    /// Mean token probability reported by the provider; not a calibrated
    /// probability that the complete transcript segment is correct.
    pub confidence: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptProposal {
    pub project_id: Uuid,
    pub project_revision: u64,
    pub media_id: Uuid,
    pub media_sha256: String,
    pub provider_id: String,
    pub model_id: String,
    pub model_version: String,
    pub model_license: String,
    pub language: String,
    pub segments: Vec<TranscriptSegment>,
}

/// Identity supplied by the trusted project host before a provider is started.
/// Provider output must match it exactly before it can enter the review path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptIdentity {
    pub project_id: Uuid,
    pub project_revision: u64,
    pub media_id: Uuid,
    pub media_sha256: String,
    pub provider_id: String,
    pub model_id: String,
    pub model_version: String,
    pub model_license: String,
}

impl TranscriptIdentity {
    pub fn validate(&self) -> Result<(), TranscriptValidationError> {
        validate_identity_fields(
            self.project_id,
            self.media_id,
            &self.media_sha256,
            &self.provider_id,
            &self.model_id,
            &self.model_version,
            &self.model_license,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranscriptValidationError {
    ProposalTooLarge,
    NoSegments,
    TooManySegments,
    InvalidMediaDigest,
    InvalidIdentity,
    InvalidProvenance,
    IdentityMismatch,
    InvalidSegment,
    SegmentOutOfRange,
}

impl TranscriptProposal {
    pub fn decode(
        bytes: &[u8],
        expected: &TranscriptIdentity,
        media_duration_ms: u64,
    ) -> Result<Self, TranscriptValidationError> {
        if bytes.len() > MAX_TRANSCRIPT_PROPOSAL_BYTES {
            return Err(TranscriptValidationError::ProposalTooLarge);
        }
        let proposal: Self =
            serde_json::from_slice(bytes).map_err(|_| TranscriptValidationError::InvalidSegment)?;
        proposal.validate(expected, media_duration_ms)?;
        Ok(proposal)
    }

    pub fn validate(
        &self,
        expected: &TranscriptIdentity,
        media_duration_ms: u64,
    ) -> Result<(), TranscriptValidationError> {
        if self.segments.is_empty() {
            return Err(TranscriptValidationError::NoSegments);
        }
        if self.segments.len() > MAX_TRANSCRIPT_SEGMENTS {
            return Err(TranscriptValidationError::TooManySegments);
        }
        expected.validate()?;
        validate_identity_fields(
            self.project_id,
            self.media_id,
            &self.media_sha256,
            &self.provider_id,
            &self.model_id,
            &self.model_version,
            &self.model_license,
        )?;
        if !valid_label(&self.language, 64) {
            return Err(TranscriptValidationError::InvalidProvenance);
        }
        if self.project_id != expected.project_id
            || self.project_revision != expected.project_revision
            || self.media_id != expected.media_id
            || self.media_sha256 != expected.media_sha256
            || self.provider_id != expected.provider_id
            || self.model_id != expected.model_id
            || self.model_version != expected.model_version
            || self.model_license != expected.model_license
        {
            return Err(TranscriptValidationError::IdentityMismatch);
        }
        if estimated_json_size(self)? > MAX_TRANSCRIPT_PROPOSAL_BYTES {
            return Err(TranscriptValidationError::ProposalTooLarge);
        }
        let mut previous_start = None;
        for segment in &self.segments {
            if segment.start_ms >= segment.end_ms
                || segment.text.trim().is_empty()
                || segment.text.len() > MAX_TRANSCRIPT_TEXT_BYTES
                || segment.text.contains('\0')
                || !segment.confidence.is_finite()
                || !(0.0..=1.0).contains(&segment.confidence)
            {
                return Err(TranscriptValidationError::InvalidSegment);
            }
            if segment.end_ms > media_duration_ms
                || previous_start.is_some_and(|start| segment.start_ms < start)
            {
                return Err(TranscriptValidationError::SegmentOutOfRange);
            }
            previous_start = Some(segment.start_ms);
        }
        Ok(())
    }
}

// JSON string escaping is counted exactly; fixed JSON punctuation, integer and
// float encodings use a conservative per-record allowance to avoid allocating
// a serialized copy just to enforce the output boundary.
fn estimated_json_size(proposal: &TranscriptProposal) -> Result<usize, TranscriptValidationError> {
    let mut size = 1024usize;
    for value in [
        proposal.media_sha256.as_str(),
        proposal.provider_id.as_str(),
        proposal.model_id.as_str(),
        proposal.model_version.as_str(),
        proposal.model_license.as_str(),
        proposal.language.as_str(),
    ] {
        size = size
            .checked_add(json_string_len(value)?)
            .ok_or(TranscriptValidationError::ProposalTooLarge)?;
    }
    for segment in &proposal.segments {
        size = size
            .checked_add(json_string_len(&segment.text)?)
            .and_then(|size| size.checked_add(128))
            .ok_or(TranscriptValidationError::ProposalTooLarge)?;
        if size > MAX_TRANSCRIPT_PROPOSAL_BYTES {
            return Err(TranscriptValidationError::ProposalTooLarge);
        }
    }
    Ok(size)
}

fn json_string_len(value: &str) -> Result<usize, TranscriptValidationError> {
    value.chars().try_fold(2usize, |size, character| {
        let escaped_bytes = match character {
            '"' | '\\' | '\u{08}' | '\u{0c}' | '\n' | '\r' | '\t' => 2,
            character if character.is_control() => 6,
            character => character.len_utf8(),
        };
        size.checked_add(escaped_bytes)
            .ok_or(TranscriptValidationError::ProposalTooLarge)
    })
}

fn valid_label(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && !value.contains('\0')
        && !value.contains('\n')
        && !value.contains('\r')
}

fn validate_identity_fields(
    project_id: Uuid,
    media_id: Uuid,
    media_sha256: &str,
    provider_id: &str,
    model_id: &str,
    model_version: &str,
    model_license: &str,
) -> Result<(), TranscriptValidationError> {
    if project_id.get_version() != Some(Version::Random)
        || media_id.get_version() != Some(Version::Random)
    {
        return Err(TranscriptValidationError::InvalidIdentity);
    }
    if media_sha256.len() != 64 || !media_sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(TranscriptValidationError::InvalidMediaDigest);
    }
    if !valid_label(provider_id, 128)
        || !valid_label(model_id, 128)
        || !valid_label(model_version, 128)
        || !valid_label(model_license, 256)
    {
        return Err(TranscriptValidationError::InvalidProvenance);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal() -> TranscriptProposal {
        TranscriptProposal {
            project_id: test_uuid(1),
            project_revision: 4,
            media_id: test_uuid(2),
            media_sha256: "a".repeat(64),
            provider_id: "whisper.cpp".to_owned(),
            model_id: "whisper-tiny-en".to_owned(),
            model_version: "1".to_owned(),
            model_license: "MIT".to_owned(),
            language: "en".to_owned(),
            segments: vec![TranscriptSegment {
                start_ms: 0,
                end_ms: 1200,
                text: "A reviewed proposal.".to_owned(),
                confidence: 0.91,
            }],
        }
    }

    fn identity() -> TranscriptIdentity {
        TranscriptIdentity {
            project_id: test_uuid(1),
            project_revision: 4,
            media_id: test_uuid(2),
            media_sha256: "a".repeat(64),
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
    fn validates_provenance_exact_millisecond_times_and_confidence() {
        let proposal = proposal();
        proposal.validate(&identity(), 1200).unwrap();
        let decoded =
            TranscriptProposal::decode(&serde_json::to_vec(&proposal).unwrap(), &identity(), 1200)
                .unwrap();
        assert_eq!(decoded, proposal);
    }

    #[test]
    fn rejects_bad_times_confidence_provenance_and_oversized_payloads() {
        let mut candidate = proposal();
        candidate.segments[0].end_ms = 1201;
        assert_eq!(
            candidate.validate(&identity(), 1200),
            Err(TranscriptValidationError::SegmentOutOfRange)
        );

        let mut candidate = proposal();
        candidate.segments[0].confidence = f32::NAN;
        assert_eq!(
            candidate.validate(&identity(), 1200),
            Err(TranscriptValidationError::InvalidSegment)
        );

        let mut candidate = proposal();
        candidate.media_sha256 = "bad".to_owned();
        assert_eq!(
            candidate.validate(&identity(), 1200),
            Err(TranscriptValidationError::InvalidMediaDigest)
        );

        assert_eq!(
            TranscriptProposal::decode(
                &vec![b'x'; MAX_TRANSCRIPT_PROPOSAL_BYTES + 1],
                &identity(),
                1200,
            ),
            Err(TranscriptValidationError::ProposalTooLarge)
        );
    }

    #[test]
    fn rejects_unsorted_or_empty_segments() {
        let mut candidate = proposal();
        candidate.segments.push(TranscriptSegment {
            start_ms: 0,
            end_ms: 1500,
            text: "Out of order".to_owned(),
            confidence: 0.5,
        });
        candidate.segments[0].start_ms = 600;
        assert_eq!(
            candidate.validate(&identity(), 1500),
            Err(TranscriptValidationError::SegmentOutOfRange)
        );

        let mut candidate = proposal();
        candidate.segments[0].text = " \n ".to_owned();
        assert_eq!(
            candidate.validate(&identity(), 1200),
            Err(TranscriptValidationError::InvalidSegment)
        );
    }

    #[test]
    fn rejects_proposals_for_another_project_media_revision_or_model() {
        let mutations: [fn(&mut TranscriptIdentity); 8] = [
            |expected: &mut TranscriptIdentity| expected.project_id = test_uuid(3),
            |expected: &mut TranscriptIdentity| expected.project_revision += 1,
            |expected: &mut TranscriptIdentity| expected.media_id = test_uuid(3),
            |expected: &mut TranscriptIdentity| expected.media_sha256 = "b".repeat(64),
            |expected: &mut TranscriptIdentity| expected.provider_id = "other".to_owned(),
            |expected: &mut TranscriptIdentity| expected.model_id = "other".to_owned(),
            |expected: &mut TranscriptIdentity| expected.model_version = "2".to_owned(),
            |expected: &mut TranscriptIdentity| expected.model_license = "other".to_owned(),
        ];
        for mutate_expected in mutations {
            let mut expected = identity();
            mutate_expected(&mut expected);
            assert_eq!(
                proposal().validate(&expected, 1200),
                Err(TranscriptValidationError::IdentityMismatch)
            );
        }
    }

    #[test]
    fn rejects_oversized_directly_constructed_proposals() {
        let mut candidate = proposal();
        candidate.segments = (0..MAX_TRANSCRIPT_PROPOSAL_BYTES / MAX_TRANSCRIPT_TEXT_BYTES + 1)
            .map(|index| TranscriptSegment {
                start_ms: index as u64,
                end_ms: index as u64 + 1,
                text: "x".repeat(MAX_TRANSCRIPT_TEXT_BYTES),
                confidence: 0.5,
            })
            .collect();
        assert_eq!(
            candidate.validate(&identity(), u64::MAX),
            Err(TranscriptValidationError::ProposalTooLarge)
        );
    }

    #[test]
    fn size_estimate_covers_json_escaping_and_numeric_encodings() {
        let mut candidate = proposal();
        candidate.project_revision = u64::MAX;
        candidate.segments[0].start_ms = u64::MAX - 1;
        candidate.segments[0].end_ms = u64::MAX;
        candidate.segments[0].confidence = 1.0;
        candidate.segments[0].text = "quotes \"\\ and controls \u{01} with café".to_owned();
        let estimate = estimated_json_size(&candidate).unwrap();
        let encoded = serde_json::to_vec(&candidate).unwrap();
        assert!(estimate >= encoded.len());
        let mut expected = identity();
        expected.project_revision = u64::MAX;
        candidate.validate(&expected, u64::MAX).unwrap();
    }

    #[test]
    fn rejects_non_v4_project_or_media_identity_even_when_proposal_matches() {
        let mut expected = identity();
        expected.project_id = Uuid::nil();
        let mut candidate = proposal();
        candidate.project_id = Uuid::nil();
        assert_eq!(
            candidate.validate(&expected, 1200),
            Err(TranscriptValidationError::InvalidIdentity)
        );

        let mut expected = identity();
        expected.media_id = Uuid::from_u128(2);
        let mut candidate = proposal();
        candidate.media_id = Uuid::from_u128(2);
        assert_eq!(
            candidate.validate(&expected, 1200),
            Err(TranscriptValidationError::InvalidIdentity)
        );
    }

    #[test]
    fn rejects_oversized_provenance_without_copying_into_validation_identity() {
        let mut candidate = proposal();
        candidate.provider_id = "x".repeat(1024 * 1024);
        assert_eq!(
            candidate.validate(&identity(), 1200),
            Err(TranscriptValidationError::InvalidProvenance)
        );
    }
}
