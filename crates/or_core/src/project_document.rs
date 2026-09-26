use crate::{
    AudioStreamMetadata, MAX_MEDIA_FORMAT_NAME_BYTES, MAX_MEDIA_FORMAT_NAMES,
    MAX_MEDIA_SOURCE_URI_BYTES, MAX_MEDIA_STREAMS, MediaId, MediaItem, MediaMetadata,
    MediaSourceRef, MediaStreamMetadata, OtherStreamMetadata, ProjectId, ProjectRevision,
    RationalRate, RationalTime, VideoStreamMetadata,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashSet, error::Error, fmt, num::NonZeroU32};

const PROJECT_FORMAT_MARKER: &str = "opencut-reinforced-project";
pub const CURRENT_PROJECT_SCHEMA_VERSION: u32 = 2;

/// Canonical persistent state for a project.
///
/// Runtime identity and editor state are intentionally not part of this value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectDocument {
    id: ProjectId,
    revision: ProjectRevision,
    name: String,
    media: Vec<MediaItem>,
}

impl ProjectDocument {
    /// Creates a project with a new persistent ID and its initial revision.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ProjectId::generate(),
            revision: ProjectRevision::INITIAL,
            name: name.into(),
            media: Vec::new(),
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

    pub fn media_items(&self) -> &[MediaItem] {
        &self.media
    }

    /// Applies the fully validated rename mutation from the application command path.
    pub(crate) fn rename_for_command(&mut self, name: String, revision: ProjectRevision) {
        self.name = name;
        self.revision = revision;
    }

    fn from_v1(project: ProjectStateV1) -> Self {
        Self {
            id: project.id,
            revision: project.revision,
            name: project.name,
            media: Vec::new(),
        }
    }

    fn from_v2(project: ProjectStateV2) -> Result<Self, ProjectCodecError> {
        let mut media = Vec::with_capacity(project.media.len());
        for item in project.media {
            let source = match item.source {
                MediaSourceRefV2::LocalFile { uri } => {
                    MediaSourceRef::local_file(uri).map_err(|_| ProjectCodecError::InvalidV2Data)?
                }
            };
            media.push(
                MediaItem::new(item.id, source, item.metadata.into_domain())
                    .map_err(|_| ProjectCodecError::InvalidV2Data)?,
            );
        }
        validate_media_library(&media)?;
        Ok(Self {
            id: project.id,
            revision: project.revision,
            name: project.name,
            media,
        })
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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectFileV2 {
    format: String,
    schema_version: u32,
    project: ProjectStateV2,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectStateV2 {
    id: ProjectId,
    revision: ProjectRevision,
    name: String,
    media: Vec<MediaItemV2>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaItemV2 {
    id: MediaId,
    source: MediaSourceRefV2,
    metadata: MediaMetadataV2,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum MediaSourceRefV2 {
    LocalFile {
        #[serde(deserialize_with = "deserialize_media_uri")]
        uri: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaMetadataV2 {
    #[serde(deserialize_with = "deserialize_v2_format_names")]
    format_names: Vec<String>,
    duration: Option<RationalTime>,
    file_size_bytes: u64,
    #[serde(deserialize_with = "deserialize_v2_streams")]
    streams: Vec<MediaStreamMetadataV2>,
}

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "metadata",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum MediaStreamMetadataV2 {
    Video(VideoStreamMetadataV2),
    Audio(AudioStreamMetadataV2),
    Other(OtherStreamMetadataV2),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VideoStreamMetadataV2 {
    index: u32,
    #[serde(deserialize_with = "crate::media::deserialize_codec_name")]
    codec_name: Option<String>,
    width: NonZeroU32,
    height: NonZeroU32,
    #[serde(deserialize_with = "crate::media::deserialize_pixel_format")]
    pixel_format: Option<String>,
    average_frame_rate: Option<RationalRate>,
    duration: Option<RationalTime>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AudioStreamMetadataV2 {
    index: u32,
    #[serde(deserialize_with = "crate::media::deserialize_codec_name")]
    codec_name: Option<String>,
    sample_rate: Option<NonZeroU32>,
    channels: Option<NonZeroU32>,
    #[serde(deserialize_with = "crate::media::deserialize_channel_layout")]
    channel_layout: Option<String>,
    duration: Option<RationalTime>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OtherStreamMetadataV2 {
    index: u32,
    #[serde(deserialize_with = "crate::media::deserialize_codec_type")]
    codec_type: Option<String>,
    #[serde(deserialize_with = "crate::media::deserialize_codec_name")]
    codec_name: Option<String>,
}

fn deserialize_media_uri<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let uri = String::deserialize(deserializer)?;
    if uri.len() > MAX_MEDIA_SOURCE_URI_BYTES {
        return Err(serde::de::Error::custom(
            "media source URI exceeds byte limit",
        ));
    }
    Ok(uri)
}

fn deserialize_v2_format_names<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = crate::media::deserialize_limited_vec(
        deserializer,
        MAX_MEDIA_FORMAT_NAMES,
        "format names",
    )?;
    if values
        .iter()
        .any(|value: &String| value.len() > MAX_MEDIA_FORMAT_NAME_BYTES)
    {
        return Err(serde::de::Error::custom("format name exceeds byte limit"));
    }
    Ok(values)
}

fn deserialize_v2_streams<'de, D>(deserializer: D) -> Result<Vec<MediaStreamMetadataV2>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::media::deserialize_limited_vec(deserializer, MAX_MEDIA_STREAMS, "media streams")
}

impl MediaMetadataV2 {
    fn into_domain(self) -> MediaMetadata {
        let streams = self
            .streams
            .into_iter()
            .map(|stream| match stream {
                MediaStreamMetadataV2::Video(stream) => {
                    MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                        stream.index,
                        stream.codec_name,
                        stream.width,
                        stream.height,
                        stream.pixel_format,
                        stream.average_frame_rate,
                        stream.duration,
                    ))
                }
                MediaStreamMetadataV2::Audio(stream) => {
                    MediaStreamMetadata::Audio(AudioStreamMetadata::from_probe(
                        stream.index,
                        stream.codec_name,
                        stream.sample_rate,
                        stream.channels,
                        stream.channel_layout,
                        stream.duration,
                    ))
                }
                MediaStreamMetadataV2::Other(stream) => {
                    MediaStreamMetadata::Other(OtherStreamMetadata::from_probe(
                        stream.index,
                        stream.codec_type,
                        stream.codec_name,
                    ))
                }
            })
            .collect();
        MediaMetadata::from_probe(
            self.format_names,
            self.duration,
            self.file_size_bytes,
            streams,
        )
    }
}

impl From<&MediaMetadata> for MediaMetadataV2 {
    fn from(metadata: &MediaMetadata) -> Self {
        let streams = metadata
            .streams()
            .iter()
            .map(|stream| match stream {
                MediaStreamMetadata::Video(stream) => {
                    MediaStreamMetadataV2::Video(VideoStreamMetadataV2 {
                        index: stream.index(),
                        codec_name: stream.codec_name().map(str::to_owned),
                        width: NonZeroU32::new(stream.width()).expect("validated video width"),
                        height: NonZeroU32::new(stream.height()).expect("validated video height"),
                        pixel_format: stream.pixel_format().map(str::to_owned),
                        average_frame_rate: stream.average_frame_rate(),
                        duration: stream.duration(),
                    })
                }
                MediaStreamMetadata::Audio(stream) => {
                    MediaStreamMetadataV2::Audio(AudioStreamMetadataV2 {
                        index: stream.index(),
                        codec_name: stream.codec_name().map(str::to_owned),
                        sample_rate: stream.sample_rate().and_then(NonZeroU32::new),
                        channels: stream.channels().and_then(NonZeroU32::new),
                        channel_layout: stream.channel_layout().map(str::to_owned),
                        duration: stream.duration(),
                    })
                }
                MediaStreamMetadata::Other(stream) => {
                    MediaStreamMetadataV2::Other(OtherStreamMetadataV2 {
                        index: stream.index(),
                        codec_type: stream.codec_type().map(str::to_owned),
                        codec_name: stream.codec_name().map(str::to_owned),
                    })
                }
            })
            .collect();
        Self {
            format_names: metadata.format_names().to_vec(),
            duration: metadata.duration(),
            file_size_bytes: metadata.file_size_bytes(),
            streams,
        }
    }
}

impl From<&ProjectDocument> for ProjectFileV2 {
    fn from(document: &ProjectDocument) -> Self {
        Self {
            format: PROJECT_FORMAT_MARKER.to_owned(),
            schema_version: CURRENT_PROJECT_SCHEMA_VERSION,
            project: ProjectStateV2 {
                id: document.id,
                revision: document.revision,
                name: document.name.clone(),
                media: document
                    .media
                    .iter()
                    .map(|item| MediaItemV2 {
                        id: item.id(),
                        source: match item.source() {
                            MediaSourceRef::LocalFile { uri } => MediaSourceRefV2::LocalFile {
                                uri: uri.as_str().to_owned(),
                            },
                        },
                        metadata: MediaMetadataV2::from(item.metadata()),
                    })
                    .collect(),
            },
        }
    }
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
    InvalidV2Data,
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
            Self::InvalidV2Data => formatter.write_str("project schema version 2 data is invalid"),
            Self::SerializationFailure => {
                formatter.write_str("project document could not be serialized")
            }
        }
    }
}

impl Error for ProjectCodecError {}

/// Encodes canonical project state as readable UTF-8 JSON with a trailing newline.
pub fn encode_project(document: &ProjectDocument) -> Result<String, ProjectCodecError> {
    validate_media_library(&document.media)?;
    let mut encoded = serde_json::to_string_pretty(&ProjectFileV2::from(document))
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
    match schema_version {
        1 => {
            let file: ProjectFileV1 =
                serde_json::from_str(encoded).map_err(|_| ProjectCodecError::InvalidV1Data)?;
            Ok(ProjectDocument::from_v1(file.project))
        }
        2 => {
            let file: ProjectFileV2 =
                serde_json::from_str(encoded).map_err(|_| ProjectCodecError::InvalidV2Data)?;
            ProjectDocument::from_v2(file.project)
        }
        _ => Err(ProjectCodecError::UnsupportedSchemaVersion(schema_version)),
    }
}

fn validate_media_library(media: &[MediaItem]) -> Result<(), ProjectCodecError> {
    let mut ids = HashSet::with_capacity(media.len());
    let mut sources = HashSet::with_capacity(media.len());
    for item in media {
        item.metadata()
            .validate()
            .map_err(|_| ProjectCodecError::InvalidV2Data)?;
        if !ids.insert(item.id()) || !sources.insert(item.source().uri()) {
            return Err(ProjectCodecError::InvalidV2Data);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CURRENT_PROJECT_SCHEMA_VERSION, ProjectCodecError, ProjectDocument, decode_project,
        encode_project,
    };
    use crate::{
        MediaId, MediaItem, MediaMetadata, MediaSourceRef, ProjectId, ProjectInstanceId,
        ProjectRevision, RationalTime,
    };
    use serde_json::{Value, json};
    use std::str::FromStr;

    const PROJECT_ID: &str = "01234567-89ab-4def-8123-456789abcdef";
    const INSTANCE_ID: &str = "fedcba98-7654-4cba-8fed-cba987654321";

    fn fixed_project() -> ProjectDocument {
        ProjectDocument {
            id: ProjectId::from_str(PROJECT_ID).unwrap(),
            revision: ProjectRevision::INITIAL,
            name: "Example".to_owned(),
            media: Vec::new(),
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
    fn project_round_trips_through_v2_json_without_changing_revision() {
        let project = ProjectDocument::new("Example");
        let encoded = encode_project(&project).unwrap();
        let decoded = decode_project(&encoded).unwrap();

        assert_eq!(decoded.id(), project.id());
        assert_eq!(decoded.revision(), project.revision());
        assert_eq!(decoded.name(), project.name());
    }

    #[test]
    fn encoding_uses_the_v2_envelope_and_a_trailing_newline() {
        let project = ProjectDocument::new("Example");
        let encoded = encode_project(&project).unwrap();
        let value: Value = serde_json::from_str(&encoded).unwrap();

        assert_eq!(
            value,
            json!({
                "format": "opencut-reinforced-project",
                "schema_version": 2,
                "project": {
                    "id": project.id().to_string(),
                    "revision": 0,
                    "name": "Example",
                    "media": []
                }
            })
        );
        assert!(encoded.ends_with('\n'));
        assert_eq!(CURRENT_PROJECT_SCHEMA_VERSION, 2);
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
    fn unsupported_schema_version_is_rejected() {
        let input = encoded_project_with_revision("0")
            .replace("\"schema_version\":1", "\"schema_version\":3");

        assert_eq!(
            decode_project(&input),
            Err(ProjectCodecError::UnsupportedSchemaVersion(3))
        );
    }

    #[test]
    fn v1_migration_preserves_identity_revision_and_name_then_encodes_v2() {
        let migrated = decode_project(&encoded_project_with_revision("7")).unwrap();
        assert_eq!(migrated.id().to_string(), PROJECT_ID);
        assert_eq!(migrated.revision(), ProjectRevision::new(7));
        assert_eq!(migrated.name(), "Example");
        assert!(migrated.media_items().is_empty());

        let encoded = encode_project(&migrated).unwrap();
        let value: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(value["schema_version"], 2);
        assert_eq!(value["project"]["revision"], 7);
    }

    #[test]
    fn v2_round_trip_preserves_media_identity_metadata_and_insertion_order() {
        let mut project = fixed_project();
        let first = test_media_item(
            "22222222-2222-4222-8222-222222222222",
            "file:///tmp/a%20clip.mkv",
        );
        let second = test_media_item("33333333-3333-4333-8333-333333333333", "file:///tmp/b.mkv");
        project.media = vec![first.clone(), second.clone()];

        let decoded = decode_project(&encode_project(&project).unwrap()).unwrap();
        assert_eq!(decoded, project);
        assert_eq!(decoded.media_items(), &[first, second]);
    }

    #[test]
    fn v2_rejects_unknown_fields_bad_ids_duplicate_ids_and_duplicate_sources() {
        let item = v2_item_json("22222222-2222-4222-8222-222222222222", "file:///tmp/a.mkv");
        let duplicate_id =
            v2_item_json("22222222-2222-4222-8222-222222222222", "file:///tmp/b.mkv");
        let duplicate_source =
            v2_item_json("33333333-3333-4333-8333-333333333333", "file:///tmp/a.mkv");
        let cases = [
            v2_json(&item, "\"unexpected\":true,"),
            v2_json(&item, "").replace(
                "\"name\":\"Example\"",
                "\"name\":\"Example\",\"unexpected\":true",
            ),
            v2_json(&item, "").replace("\"media\":[{", "\"media\":[{\"unexpected\":true,"),
            v2_json(&item, "").replace("file:///tmp/a.mkv", "https://example.com/a.mkv"),
            v2_json(&item, "").replace(
                "22222222-2222-4222-8222-222222222222",
                "00000000-0000-0000-0000-000000000000",
            ),
            v2_json(&format!("{item},{duplicate_id}"), ""),
            v2_json(&format!("{item},{duplicate_source}"), ""),
            v2_json(&item, "").replace(
                "\"uri\":\"file:///tmp/a.mkv\"",
                "\"uri\":\"file:///tmp/a.mkv\",\"unexpected\":true",
            ),
            v2_json(&item, "").replace(
                "\"format_names\":[]",
                &format!("\"format_names\":[\"{}\"]", "x".repeat(257)),
            ),
        ];
        for input in cases {
            assert_eq!(
                decode_project(&input),
                Err(ProjectCodecError::InvalidV2Data)
            );
        }
    }

    #[test]
    fn v2_rejects_oversized_uri_and_excessive_metadata_collections() {
        let oversized_uri = format!("file:///{}", "a".repeat(crate::MAX_MEDIA_SOURCE_URI_BYTES));
        let input = v2_json(
            &v2_item_json("22222222-2222-4222-8222-222222222222", &oversized_uri),
            "",
        );
        assert_eq!(
            decode_project(&input),
            Err(ProjectCodecError::InvalidV2Data)
        );

        let too_many_formats =
            serde_json::to_string(&vec!["matroska"; crate::MAX_MEDIA_FORMAT_NAMES + 1]).unwrap();
        let input = v2_json(
            &v2_item_json("22222222-2222-4222-8222-222222222222", "file:///tmp/a.mkv").replace(
                "\"format_names\":[]",
                &format!("\"format_names\":{too_many_formats}"),
            ),
            "",
        );
        assert_eq!(
            decode_project(&input),
            Err(ProjectCodecError::InvalidV2Data)
        );

        let stream = json!({
            "kind": "other",
            "metadata": { "index": 0, "codec_type": null, "codec_name": null }
        });
        let too_many_streams =
            serde_json::to_string(&vec![stream; crate::MAX_MEDIA_STREAMS + 1]).unwrap();
        let input = v2_json(
            &v2_item_json("22222222-2222-4222-8222-222222222222", "file:///tmp/a.mkv")
                .replace("\"streams\":[]", &format!("\"streams\":{too_many_streams}")),
            "",
        );
        assert_eq!(
            decode_project(&input),
            Err(ProjectCodecError::InvalidV2Data)
        );
    }

    fn test_media_item(id: &str, uri: &str) -> MediaItem {
        let id = MediaId::from_str(id).unwrap();
        let source = MediaSourceRef::local_file(uri).unwrap();
        let metadata = MediaMetadata::from_probe(
            vec!["matroska".to_owned()],
            Some(RationalTime::new(3, 2).unwrap()),
            1024,
            Vec::new(),
        );
        MediaItem::new(id, source, metadata).unwrap()
    }

    fn v2_item_json(id: &str, uri: &str) -> String {
        serde_json::to_string(&json!({
            "id": id,
            "source": { "kind": "local_file", "uri": uri },
            "metadata": { "format_names": [], "duration": null, "file_size_bytes": 0, "streams": [] }
        })).unwrap()
    }

    fn v2_json(media_items: &str, extra_root: &str) -> String {
        format!(
            r#"{{"format":"opencut-reinforced-project","schema_version":2,{extra_root}"project":{{"id":"{PROJECT_ID}","revision":7,"name":"Example","media":[{media_items}]}}}}"#
        )
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
