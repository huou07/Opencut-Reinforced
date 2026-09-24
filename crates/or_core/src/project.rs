use serde::{Deserialize, Deserializer, Serialize};
use std::{error::Error, fmt, str::FromStr};
use uuid::{Uuid, Version};

/// Persistent identity for one logical project.
///
/// A project name or path is mutable metadata and is never part of this ID.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ProjectId(Uuid);

impl ProjectId {
    /// Generates a new UUID version 4 project identity.
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

impl FromStr for ProjectId {
    type Err = UuidV4ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(value).map_err(UuidV4ParseError::InvalidUuid)?;
        validate_v4(uuid).map(Self)
    }
}

impl<'de> Deserialize<'de> for ProjectId {
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

/// Ephemeral identity for one loaded runtime instance of a project.
///
/// A fresh value is generated for each runtime open. It is not canonical project
/// identity and must not be stored in a future project document.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ProjectInstanceId(Uuid);

impl ProjectInstanceId {
    /// Generates a new UUID version 4 runtime-instance identity.
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for ProjectInstanceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

impl FromStr for ProjectInstanceId {
    type Err = UuidV4ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(value).map_err(UuidV4ParseError::InvalidUuid)?;
        validate_v4(uuid).map(Self)
    }
}

impl<'de> Deserialize<'de> for ProjectInstanceId {
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

/// Failure while parsing an ID that must contain a UUID version 4 value.
#[derive(Debug)]
pub enum UuidV4ParseError {
    InvalidUuid(uuid::Error),
    NotVersion4,
}

impl fmt::Display for UuidV4ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUuid(error) => fmt::Display::fmt(error, formatter),
            Self::NotVersion4 => formatter.write_str("UUID must be version 4"),
        }
    }
}

impl Error for UuidV4ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidUuid(error) => Some(error),
            Self::NotVersion4 => None,
        }
    }
}

/// Persistent monotonically increasing revision of canonical project state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectRevision(u64);

impl ProjectRevision {
    pub const INITIAL: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    /// Returns the next revision, failing rather than wrapping at u64::MAX.
    pub const fn checked_next(self) -> Result<Self, ProjectRevisionOverflow> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(ProjectRevisionOverflow),
        }
    }
}

impl fmt::Display for ProjectRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectRevisionOverflow;

impl fmt::Display for ProjectRevisionOverflow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("project revision overflow")
    }
}

impl Error for ProjectRevisionOverflow {}

#[cfg(test)]
mod tests {
    use super::{ProjectId, ProjectInstanceId, ProjectRevision, ProjectRevisionOverflow};
    use std::str::FromStr;
    use uuid::Version;

    #[test]
    fn project_id_generates_and_formats_uuid_v4() {
        let id = ProjectId::generate();
        assert_eq!(id.0.get_version(), Some(Version::Random));

        let text = id.to_string();
        assert_eq!(text.len(), 36);
        assert_eq!(text.bytes().filter(|byte| *byte == b'-').count(), 4);
        assert!(text.bytes().all(|byte| {
            byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte) || byte == b'-'
        }));
        assert_eq!(ProjectId::from_str(&text).unwrap(), id);
    }

    #[test]
    fn project_id_serde_round_trips_as_canonical_text() {
        let id = ProjectId::generate();
        let serialized = serde_json::to_string(&id).unwrap();

        assert_eq!(
            serde_json::from_str::<String>(&serialized).unwrap(),
            id.to_string()
        );
        assert_eq!(serde_json::from_str::<ProjectId>(&serialized).unwrap(), id);
    }

    #[test]
    fn project_id_rejects_invalid_text_and_non_v4_uuids() {
        assert!(ProjectId::from_str("not-a-uuid").is_err());
        assert!(ProjectId::from_str("00000000-0000-0000-0000-000000000000").is_err());
        let version_one_uuid = "00000000-0000-1000-8000-000000000000";
        assert!(ProjectId::from_str(version_one_uuid).is_err());
        assert!(
            serde_json::from_str::<ProjectId>(r#""00000000-0000-0000-0000-000000000000""#).is_err()
        );
        let serialized_version_one_uuid = serde_json::to_string(version_one_uuid).unwrap();
        assert!(serde_json::from_str::<ProjectId>(&serialized_version_one_uuid).is_err());
    }

    #[test]
    fn project_instance_id_generates_and_round_trips() {
        let id = ProjectInstanceId::generate();
        assert_eq!(id.0.get_version(), Some(Version::Random));

        let text = id.to_string();
        assert_eq!(ProjectInstanceId::from_str(&text).unwrap(), id);
        let serialized = serde_json::to_string(&id).unwrap();
        assert_eq!(
            serde_json::from_str::<ProjectInstanceId>(&serialized).unwrap(),
            id
        );
    }

    #[test]
    fn project_revision_starts_at_zero_and_increments_checked() {
        assert_eq!(ProjectRevision::INITIAL.value(), 0);
        assert_eq!(ProjectRevision::INITIAL.checked_next().unwrap().value(), 1);
        assert_eq!(ProjectRevision::new(41).checked_next().unwrap().value(), 42);
        assert_eq!(
            ProjectRevision::new(u64::MAX).checked_next(),
            Err(ProjectRevisionOverflow)
        );
    }

    #[test]
    fn project_revision_serde_round_trips() {
        let revision = ProjectRevision::new(42);
        let serialized = serde_json::to_string(&revision).unwrap();

        assert_eq!(serialized, "42");
        assert_eq!(
            serde_json::from_str::<ProjectRevision>(&serialized).unwrap(),
            revision
        );
    }
}
