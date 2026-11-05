mod manager;

pub use manager::{
    JobCancelError, JobCancelOutcome, JobContext, JobFailure, JobManager, JobManagerConfig,
    JobManagerConfigError, JobProgress, JobSnapshot, JobSubmitError,
};

/// How many media artifact jobs may run at once on this host.
///
/// Every caller that builds an artifact pool uses this one policy, so the app, the
/// throughput lane and the playback-starvation lane cannot drift apart. Measured on
/// the calibration host with `crates/or_core/tests/artifact_throughput.rs` (16
/// artifacts): 34.4 ms each at one worker, 15.3 ms at two, 8.7 ms at four, 5.9 ms at
/// eight — four is where a small machine still keeps playback responsive and a wide
/// machine stops leaving cores idle.
pub fn artifact_worker_count() -> usize {
    let parallelism = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(2);
    artifact_worker_count_for(parallelism)
}

/// The same policy for a caller that already knows the host's parallelism.
pub fn artifact_worker_count_for(parallelism: usize) -> usize {
    if parallelism >= 6 { 4 } else { 2 }
}

use crate::UuidV4ParseError;
use serde::{Deserialize, Deserializer, Serialize};
use std::{fmt, str::FromStr};
use uuid::{Uuid, Version};

/// Opaque identity for one unit of work.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct JobId(Uuid);

impl JobId {
    /// Generates a new UUID version 4 job identity.
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

impl FromStr for JobId {
    type Err = UuidV4ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(value).map_err(UuidV4ParseError::InvalidUuid)?;
        validate_v4(uuid).map(Self)
    }
}

impl<'de> Deserialize<'de> for JobId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let uuid = Uuid::deserialize(deserializer)?;
        validate_v4(uuid)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

fn validate_v4(uuid: Uuid) -> Result<Uuid, UuidV4ParseError> {
    if uuid.get_version() == Some(Version::Random) {
        Ok(uuid)
    } else {
        Err(UuidV4ParseError::NotVersion4)
    }
}

/// Work category at the shared job boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    MediaProbe,
    ThumbnailGenerate,
    WaveformGenerate,
    ProxyGenerate,
    Transcription,
    Export,
}

/// Lifecycle state shared by future bounded background jobs.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::{JobId, JobKind, JobState};
    use std::str::FromStr;
    use uuid::Uuid;

    #[test]
    fn job_id_generates_formats_parses_and_round_trips() {
        let id = JobId::generate();
        let text = id.to_string();
        assert_eq!(
            Uuid::parse_str(&text).unwrap().get_version(),
            Some(uuid::Version::Random)
        );
        assert_eq!(JobId::from_str(&text).unwrap(), id);
        assert_eq!(
            serde_json::from_str::<JobId>(&serde_json::to_string(&id).unwrap()).unwrap(),
            id
        );
    }

    #[test]
    fn job_id_rejects_malformed_and_non_v4_values() {
        assert!(JobId::from_str("not-a-uuid").is_err());
        assert!(serde_json::from_str::<JobId>("\"00000000-0000-0000-0000-000000000001\"").is_err());
    }

    #[test]
    fn job_kinds_keep_the_media_probe_name_and_use_snake_case_for_previews() {
        assert_eq!(
            serde_json::to_string(&JobKind::MediaProbe).unwrap(),
            "\"media_probe\""
        );
        assert_eq!(
            serde_json::to_string(&JobKind::ThumbnailGenerate).unwrap(),
            "\"thumbnail_generate\""
        );
        assert_eq!(
            serde_json::to_string(&JobKind::WaveformGenerate).unwrap(),
            "\"waveform_generate\""
        );
        assert_eq!(
            serde_json::to_string(&JobKind::Export).unwrap(),
            "\"export\""
        );
        assert_eq!(
            serde_json::to_string(&JobKind::ProxyGenerate).unwrap(),
            "\"proxy_generate\""
        );
        assert_eq!(
            serde_json::to_string(&JobKind::Transcription).unwrap(),
            "\"transcription\""
        );
        assert_eq!(
            serde_json::to_string(&JobState::Queued).unwrap(),
            "\"queued\""
        );
        assert_eq!(
            serde_json::from_str::<JobState>("\"cancelled\"").unwrap(),
            JobState::Cancelled
        );
    }
}
