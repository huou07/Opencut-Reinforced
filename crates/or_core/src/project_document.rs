use crate::{ProjectId, ProjectRevision};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, fmt};

const PROJECT_FORMAT_MARKER: &str = "opencut-reinforced-project";
pub const CURRENT_PROJECT_SCHEMA_VERSION: u32 = 1;

/// Canonical persistent state for a project.
///
/// Runtime identity and editor state are intentionally not part of this value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectDocument {
    id: ProjectId,
    revision: ProjectRevision,
    name: String,
}

impl ProjectDocument {
    /// Creates a project with a new persistent ID and its initial revision.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ProjectId::generate(),
            revision: ProjectRevision::INITIAL,
            name: name.into(),
        }
    }

    pub const fn id(&self) -> ProjectId {
        self.id
    }

    pub const fn revision(&self) -> ProjectRevision {
        self.revision
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn from_v1(project: ProjectStateV1) -> Self {
        Self {
            id: project.id,
            revision: project.revision,
            name: project.name,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectFileV1 {
    format: String,
    schema_version: u32,
    project: ProjectStateV1,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectStateV1 {
    id: ProjectId,
    revision: ProjectRevision,
    name: String,
}

impl From<&ProjectDocument> for ProjectFileV1 {
    fn from(document: &ProjectDocument) -> Self {
        Self {
            format: PROJECT_FORMAT_MARKER.to_owned(),
            schema_version: CURRENT_PROJECT_SCHEMA_VERSION,
            project: ProjectStateV1 {
                id: document.id,
                revision: document.revision,
                name: document.name.clone(),
            },
        }
    }
}

/// Failures from encoding or validating an `.orproj` document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectCodecError {
    InvalidJson,
    InvalidEnvelope,
    WrongFormatMarker,
    InvalidSchemaVersion,
    UnsupportedSchemaVersion(u64),
    InvalidV1Data,
    SerializationFailure,
}

impl fmt::Display for ProjectCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson => formatter.write_str("project document is not valid JSON"),
            Self::InvalidEnvelope => {
                formatter.write_str("project document has an invalid envelope")
            }
            Self::WrongFormatMarker => {
                formatter.write_str("project document format marker is not recognized")
            }
            Self::InvalidSchemaVersion => {
                formatter.write_str("project schema_version must be an unsigned integer")
            }
            Self::UnsupportedSchemaVersion(version) => {
                write!(
                    formatter,
                    "project schema version {version} is not supported"
                )
            }
            Self::InvalidV1Data => formatter.write_str("project schema version 1 data is invalid"),
            Self::SerializationFailure => {
                formatter.write_str("project document could not be serialized")
            }
        }
    }
}

impl Error for ProjectCodecError {}

/// Encodes canonical project state as readable UTF-8 JSON with a trailing newline.
pub fn encode_project(document: &ProjectDocument) -> Result<String, ProjectCodecError> {
    let mut encoded = serde_json::to_string_pretty(&ProjectFileV1::from(document))
        .map_err(|_| ProjectCodecError::SerializationFailure)?;
    encoded.push('\n');
    Ok(encoded)
}

/// Decodes a versioned `.orproj` JSON document into validated canonical state.
pub fn decode_project(encoded: &str) -> Result<ProjectDocument, ProjectCodecError> {
    let envelope: Value =
        serde_json::from_str(encoded).map_err(|_| ProjectCodecError::InvalidJson)?;
    let envelope = envelope
        .as_object()
        .ok_or(ProjectCodecError::InvalidEnvelope)?;

    let format = envelope
        .get("format")
        .and_then(Value::as_str)
        .ok_or(ProjectCodecError::InvalidEnvelope)?;
    if format != PROJECT_FORMAT_MARKER {
        return Err(ProjectCodecError::WrongFormatMarker);
    }

    let schema_version = envelope
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or(ProjectCodecError::InvalidSchemaVersion)?;
    if schema_version != u64::from(CURRENT_PROJECT_SCHEMA_VERSION) {
        return Err(ProjectCodecError::UnsupportedSchemaVersion(schema_version));
    }

    let file: ProjectFileV1 =
        serde_json::from_str(encoded).map_err(|_| ProjectCodecError::InvalidV1Data)?;
    Ok(ProjectDocument::from_v1(file.project))
}

#[cfg(test)]
mod tests {
    use super::{
        CURRENT_PROJECT_SCHEMA_VERSION, ProjectCodecError, ProjectDocument, decode_project,
        encode_project,
    };
    use crate::{ProjectId, ProjectInstanceId, ProjectRevision};
    use serde_json::{Value, json};
    use std::str::FromStr;

    const PROJECT_ID: &str = "01234567-89ab-4def-8123-456789abcdef";
    const INSTANCE_ID: &str = "fedcba98-7654-4cba-8fed-cba987654321";

    fn fixed_project() -> ProjectDocument {
        ProjectDocument {
            id: ProjectId::from_str(PROJECT_ID).unwrap(),
            revision: ProjectRevision::INITIAL,
            name: "Example".to_owned(),
        }
    }

    fn encoded_project_with_revision(revision: &str) -> String {
        format!(
            r#"{{"format":"opencut-reinforced-project","schema_version":1,"project":{{"id":"{PROJECT_ID}","revision":{revision},"name":"Example"}}}}"#
        )
    }

    #[test]
    fn new_project_has_a_v4_id_initial_revision_and_exact_name() {
        let project = ProjectDocument::new("Example");

        assert!(ProjectId::from_str(&project.id().to_string()).is_ok());
        assert_eq!(project.revision(), ProjectRevision::INITIAL);
        assert_eq!(project.name(), "Example");
    }

    #[test]
    fn project_round_trips_through_v1_json_without_changing_revision() {
        let project = ProjectDocument::new("Example");
        let encoded = encode_project(&project).unwrap();
        let decoded = decode_project(&encoded).unwrap();

        assert_eq!(decoded.id(), project.id());
        assert_eq!(decoded.revision(), project.revision());
        assert_eq!(decoded.name(), project.name());
    }

    #[test]
    fn encoding_uses_the_v1_envelope_and_a_trailing_newline() {
        let project = ProjectDocument::new("Example");
        let encoded = encode_project(&project).unwrap();
        let value: Value = serde_json::from_str(&encoded).unwrap();

        assert_eq!(
            value,
            json!({
                "format": "opencut-reinforced-project",
                "schema_version": 1,
                "project": {
                    "id": project.id().to_string(),
                    "revision": 0,
                    "name": "Example"
                }
            })
        );
        assert!(encoded.ends_with('\n'));
        assert_eq!(CURRENT_PROJECT_SCHEMA_VERSION, 1);
    }

    #[test]
    fn encoding_is_deterministic_for_the_same_document() {
        let project = ProjectDocument::new("Example");

        assert_eq!(
            encode_project(&project).unwrap(),
            encode_project(&project).unwrap()
        );
    }

    #[test]
    fn unicode_name_is_preserved_exactly_without_trimming_or_normalization() {
        let name = "  Tiếng Việt – cà phê 🎬  ";
        let project = ProjectDocument::new(name);
        let decoded = decode_project(&encode_project(&project).unwrap()).unwrap();

        assert_eq!(decoded.name(), name);
    }

    #[test]
    fn wrong_format_marker_is_rejected() {
        let input = encoded_project_with_revision("0")
            .replace("opencut-reinforced-project", "another-format");

        assert_eq!(
            decode_project(&input),
            Err(ProjectCodecError::WrongFormatMarker)
        );
    }

    #[test]
    fn unsupported_schema_version_is_rejected_before_v1_parsing() {
        let input = encoded_project_with_revision("0")
            .replace("\"schema_version\":1", "\"schema_version\":2");

        assert_eq!(
            decode_project(&input),
            Err(ProjectCodecError::UnsupportedSchemaVersion(2))
        );
    }

    #[test]
    fn missing_or_non_integer_schema_version_is_rejected() {
        for version in [None, Some("\"1\""), Some("1.0"), Some("null")] {
            let input = match version {
                Some(version) => encoded_project_with_revision("0").replace(
                    "\"schema_version\":1",
                    &format!("\"schema_version\":{version}"),
                ),
                None => encoded_project_with_revision("0").replace("\"schema_version\":1,", ""),
            };
            assert_eq!(
                decode_project(&input),
                Err(ProjectCodecError::InvalidSchemaVersion)
            );
        }
    }

    #[test]
    fn malformed_json_and_missing_document_fields_are_rejected() {
        assert_eq!(
            decode_project(r#"{"format":"opencut-reinforced-project""#),
            Err(ProjectCodecError::InvalidJson)
        );
        assert_eq!(
            decode_project("not JSON"),
            Err(ProjectCodecError::InvalidJson)
        );

        for input in [
            r#"{"format":"opencut-reinforced-project","schema_version":1}"#,
            r#"{"format":"opencut-reinforced-project","schema_version":1,"project":{}}"#,
            r#"{"format":"opencut-reinforced-project","schema_version":1,"project":{"revision":0,"name":"Example"}}"#,
            r#"{"format":"opencut-reinforced-project","schema_version":1,"project":{"id":"01234567-89ab-4def-8123-456789abcdef","name":"Example"}}"#,
            r#"{"format":"opencut-reinforced-project","schema_version":1,"project":{"id":"01234567-89ab-4def-8123-456789abcdef","revision":0}}"#,
        ] {
            assert_eq!(decode_project(input), Err(ProjectCodecError::InvalidV1Data));
        }
    }

    #[test]
    fn unknown_v1_fields_are_rejected_at_root_and_project_levels() {
        for input in [
            r#"{"format":"opencut-reinforced-project","schema_version":1,"unexpected":true,"project":{"id":"01234567-89ab-4def-8123-456789abcdef","revision":0,"name":"Example"}}"#,
            r#"{"format":"opencut-reinforced-project","schema_version":1,"project":{"id":"01234567-89ab-4def-8123-456789abcdef","revision":0,"name":"Example","unexpected":true}}"#,
            r#"{"format":"opencut-reinforced-project","schema_version":1,"project":{"id":"01234567-89ab-4def-8123-456789abcdef","revision":0,"name":"Example","project_instance_id":"01234567-89ab-4def-8123-456789abcdef"}}"#,
        ] {
            assert_eq!(decode_project(input), Err(ProjectCodecError::InvalidV1Data));
        }
    }

    #[test]
    fn invalid_project_ids_are_rejected_by_the_existing_id_invariant() {
        for id in [
            "not-a-uuid",
            "00000000-0000-0000-0000-000000000000",
            "00000000-0000-1000-8000-000000000000",
        ] {
            let input = format!(
                r#"{{"format":"opencut-reinforced-project","schema_version":1,"project":{{"id":"{id}","revision":0,"name":"Example"}}}}"#
            );
            assert_eq!(
                decode_project(&input),
                Err(ProjectCodecError::InvalidV1Data)
            );
        }
    }

    #[test]
    fn nonzero_and_maximum_revisions_round_trip_unchanged() {
        for revision in ["42", "18446744073709551615"] {
            let decoded = decode_project(&encoded_project_with_revision(revision)).unwrap();
            assert_eq!(decoded.revision().to_string(), revision);

            let encoded = encode_project(&decoded).unwrap();
            let reloaded = decode_project(&encoded).unwrap();
            assert_eq!(reloaded.revision(), decoded.revision());
        }
    }

    #[test]
    fn runtime_instance_identity_never_enters_persisted_project_json() {
        let project = fixed_project();
        let instance_id = ProjectInstanceId::from_str(INSTANCE_ID).unwrap();
        let encoded = encode_project(&project).unwrap();

        assert!(!encoded.contains(&instance_id.to_string()));
        for forbidden in ["instance", "instance_id", "project_instance_id"] {
            assert!(!encoded.contains(forbidden));
        }
    }

    #[test]
    fn decoding_returns_only_canonical_project_state() {
        let project = ProjectDocument::new("Example");
        let decoded = decode_project(&encode_project(&project).unwrap()).unwrap();
        let reencoded = encode_project(&decoded).unwrap();

        assert_eq!(decoded.id(), project.id());
        assert_eq!(decoded.revision(), ProjectRevision::INITIAL);
        assert_eq!(decoded.name(), project.name());
        assert!(!reencoded.contains("instance"));
    }
}
