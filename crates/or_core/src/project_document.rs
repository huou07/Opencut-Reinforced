use crate::{
    AudioStreamMetadata, ClipId, MAX_MEDIA_FORMAT_NAME_BYTES, MAX_MEDIA_FORMAT_NAMES,
    MAX_MEDIA_SOURCE_URI_BYTES, MAX_MEDIA_STREAMS, MAX_TIMELINE_CLIPS_PER_TRACK,
    MAX_TIMELINE_TRACKS, MediaId, MediaItem, MediaMetadata, MediaSourceRef, MediaStreamMetadata,
    OtherStreamMetadata, ProjectId, ProjectRevision, ProjectTimeline, RationalRate, RationalTime,
    TimeRange, TimelineClip, TimelineTrack, TrackId, TrackKind, VideoStreamMetadata,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{collections::HashSet, error::Error, fmt, num::NonZeroU32};

const PROJECT_FORMAT_MARKER: &str = "opencut-reinforced-project";
pub const CURRENT_PROJECT_SCHEMA_VERSION: u32 = 3;

/// Canonical persistent state for a project.
///
/// Runtime identity and editor state are intentionally not part of this value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectDocument {
    id: ProjectId,
    revision: ProjectRevision,
    name: String,
    media: Vec<MediaItem>,
    timeline: ProjectTimeline,
}

impl ProjectDocument {
    /// Creates a project with a new persistent ID and its initial revision.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ProjectId::generate(),
            revision: ProjectRevision::INITIAL,
            name: name.into(),
            media: Vec::new(),
            timeline: ProjectTimeline::default(),
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

    pub fn timeline(&self) -> &ProjectTimeline {
        &self.timeline
    }

    #[cfg(test)]
    pub(crate) fn set_timeline_for_test(&mut self, timeline: ProjectTimeline) {
        self.timeline = timeline;
    }

    /// Applies the fully validated rename mutation from the application command path.
    pub(crate) fn rename_for_command(&mut self, name: String, revision: ProjectRevision) {
        self.name = name;
        self.revision = revision;
    }

    pub(crate) fn try_reserve_media_items(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.media.try_reserve(additional)
    }

    /// Inserts a validated library item at an index checked by the command path.
    pub(crate) fn insert_media_for_command(
        &mut self,
        item: MediaItem,
        index: usize,
        revision: ProjectRevision,
    ) {
        self.media.insert(index, item);
        self.revision = revision;
    }

    /// Removes an item at an index checked by the command path.
    pub(crate) fn remove_media_for_command(
        &mut self,
        index: usize,
        revision: ProjectRevision,
    ) -> MediaItem {
        let item = self.media.remove(index);
        self.revision = revision;
        item
    }

    pub(crate) fn try_reserve_timeline_tracks(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.timeline.try_reserve_tracks(additional)
    }

    pub(crate) fn try_reserve_timeline_track_clips(
        &mut self,
        track_index: usize,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.timeline
            .try_reserve_track_clips(track_index, additional)
    }

    pub(crate) fn insert_timeline_track_for_command(
        &mut self,
        index: usize,
        track: TimelineTrack,
        revision: ProjectRevision,
    ) {
        self.timeline.insert_track_for_command(index, track);
        self.revision = revision;
    }

    pub(crate) fn remove_timeline_track_for_command(
        &mut self,
        index: usize,
        revision: ProjectRevision,
    ) -> TimelineTrack {
        let track = self.timeline.remove_track_for_command(index);
        self.revision = revision;
        track
    }

    pub(crate) fn insert_timeline_clip_for_command(
        &mut self,
        track_index: usize,
        index: usize,
        clip: TimelineClip,
        revision: ProjectRevision,
    ) {
        self.timeline
            .insert_clip_for_command(track_index, index, clip);
        self.revision = revision;
    }

    pub(crate) fn remove_timeline_clip_for_command(
        &mut self,
        track_index: usize,
        index: usize,
        revision: ProjectRevision,
    ) -> TimelineClip {
        let clip = self.timeline.remove_clip_for_command(track_index, index);
        self.revision = revision;
        clip
    }

    pub(crate) fn move_timeline_clip_for_command(
        &mut self,
        from_track_index: usize,
        from_index: usize,
        to_track_index: usize,
        to_index: usize,
        timeline_start: RationalTime,
        revision: ProjectRevision,
    ) {
        let clip = self
            .timeline
            .remove_clip_for_command(from_track_index, from_index)
            .with_timeline_start_for_command(timeline_start);
        self.timeline
            .insert_clip_for_command(to_track_index, to_index, clip);
        self.revision = revision;
    }

    fn from_v1(project: ProjectStateV1) -> Self {
        Self {
            id: project.id,
            revision: project.revision,
            name: project.name,
            media: Vec::new(),
            timeline: ProjectTimeline::default(),
        }
    }

    fn from_v2(project: ProjectStateV2) -> Result<Self, ProjectCodecError> {
        let media = decode_media_library(project.media, ProjectCodecError::InvalidV2Data)?;
        Ok(Self {
            id: project.id,
            revision: project.revision,
            name: project.name,
            media,
            timeline: ProjectTimeline::default(),
        })
    }

    fn from_v3(project: ProjectStateV3) -> Result<Self, ProjectCodecError> {
        let media = decode_media_library(project.media, ProjectCodecError::InvalidV3Data)?;
        let timeline = project
            .timeline
            .into_domain(&media)
            .map_err(|_| ProjectCodecError::InvalidV3Data)?;
        Ok(Self {
            id: project.id,
            revision: project.revision,
            name: project.name,
            media,
            timeline,
        })
    }
}

fn decode_media_library(
    items: Vec<MediaItemV2>,
    invalid_data: ProjectCodecError,
) -> Result<Vec<MediaItem>, ProjectCodecError> {
    let mut media = Vec::with_capacity(items.len());
    for item in items {
        let source = match item.source {
            MediaSourceRefV2::LocalFile { uri } => {
                MediaSourceRef::local_file(uri).map_err(|_| invalid_data.clone())?
            }
        };
        media.push(
            MediaItem::new(item.id, source, item.metadata.into_domain())
                .map_err(|_| invalid_data.clone())?,
        );
    }
    validate_media_library(&media, invalid_data)?;
    Ok(media)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectFileV1 {
    format: String,
    schema_version: u32,
    project: ProjectStateV1,
}

#[derive(Deserialize)]
struct ProjectEnvelopeProbe<'a> {
    #[serde(borrow)]
    format: Option<&'a RawValue>,
    #[serde(borrow)]
    schema_version: Option<&'a RawValue>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectStateV1 {
    id: ProjectId,
    revision: ProjectRevision,
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectFileV2 {
    format: String,
    schema_version: u32,
    project: ProjectStateV2,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectStateV2 {
    id: ProjectId,
    revision: ProjectRevision,
    name: String,
    media: Vec<MediaItemV2>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectFileV3 {
    format: String,
    schema_version: u32,
    project: ProjectStateV3,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectStateV3 {
    id: ProjectId,
    revision: ProjectRevision,
    name: String,
    media: Vec<MediaItemV2>,
    timeline: ProjectTimelineV3,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectTimelineV3 {
    #[serde(deserialize_with = "deserialize_v3_tracks")]
    tracks: Vec<TimelineTrackV3>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineTrackV3 {
    id: TrackId,
    kind: TrackKindV3,
    #[serde(deserialize_with = "deserialize_v3_clips")]
    clips: Vec<TimelineClipV3>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TrackKindV3 {
    Video,
    Audio,
}

impl From<TrackKindV3> for TrackKind {
    fn from(kind: TrackKindV3) -> Self {
        match kind {
            TrackKindV3::Video => Self::Video,
            TrackKindV3::Audio => Self::Audio,
        }
    }
}

impl From<TrackKind> for TrackKindV3 {
    fn from(kind: TrackKind) -> Self {
        match kind {
            TrackKind::Video => Self::Video,
            TrackKind::Audio => Self::Audio,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipV3 {
    id: ClipId,
    media_id: MediaId,
    timeline_start: RationalTimeV3,
    source_range: TimeRangeV3,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RationalTimeV3 {
    numerator: i64,
    denominator: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimeRangeV3 {
    start: RationalTimeV3,
    duration: RationalTimeV3,
}

impl RationalTimeV3 {
    fn into_domain(self) -> Result<RationalTime, crate::TimeError> {
        RationalTime::new(self.numerator, self.denominator)
    }
}

impl TimeRangeV3 {
    fn into_domain(self) -> Result<TimeRange, crate::TimeError> {
        TimeRange::new(self.start.into_domain()?, self.duration.into_domain()?)
    }
}

impl ProjectTimelineV3 {
    fn into_domain(
        self,
        media: &[MediaItem],
    ) -> Result<ProjectTimeline, crate::timeline::TimelineValidationError> {
        let tracks = self
            .tracks
            .into_iter()
            .map(|track| {
                let clips = track
                    .clips
                    .into_iter()
                    .map(|clip| {
                        Ok(TimelineClip::from_parts_for_codec(
                            clip.id,
                            clip.media_id,
                            clip.timeline_start
                                .into_domain()
                                .map_err(|_| crate::timeline::TimelineValidationError)?,
                            clip.source_range
                                .into_domain()
                                .map_err(|_| crate::timeline::TimelineValidationError)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, crate::timeline::TimelineValidationError>>()?;
                Ok(TimelineTrack::from_parts_for_codec(
                    track.id,
                    track.kind.into(),
                    clips,
                ))
            })
            .collect::<Result<Vec<_>, crate::timeline::TimelineValidationError>>()?;
        let timeline = ProjectTimeline::from_tracks_for_codec(tracks);
        timeline.validate(media)?;
        Ok(timeline)
    }
}

fn deserialize_v3_tracks<'de, D>(deserializer: D) -> Result<Vec<TimelineTrackV3>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_v3_tracks_with_limits(deserializer, MAX_TIMELINE_TRACKS, crate::MAX_TIMELINE_CLIPS)
}

fn deserialize_v3_tracks_with_limits<'de, D>(
    deserializer: D,
    max_tracks: usize,
    max_total_clips: usize,
) -> Result<Vec<TimelineTrackV3>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct TracksVisitor {
        max_tracks: usize,
        max_total_clips: usize,
    }

    impl<'de> serde::de::Visitor<'de> for TracksVisitor {
        type Value = Vec<TimelineTrackV3>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                formatter,
                "at most {} bounded timeline tracks",
                self.max_tracks
            )
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let capacity = sequence.size_hint().unwrap_or(0).min(self.max_tracks);
            let mut tracks = Vec::with_capacity(capacity);
            let mut total_clips = 0usize;
            while tracks.len() < self.max_tracks {
                let Some(track) = sequence.next_element::<TimelineTrackV3>()? else {
                    return Ok(tracks);
                };
                total_clips = total_clips
                    .checked_add(track.clips.len())
                    .filter(|count| *count <= self.max_total_clips)
                    .ok_or_else(|| {
                        serde::de::Error::custom("timeline clips exceed configured limit")
                    })?;
                tracks.push(track);
            }
            if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(serde::de::Error::custom(
                    "timeline tracks exceed configured limit",
                ));
            }
            Ok(tracks)
        }
    }

    deserializer.deserialize_seq(TracksVisitor {
        max_tracks,
        max_total_clips,
    })
}

fn deserialize_v3_clips<'de, D>(deserializer: D) -> Result<Vec<TimelineClipV3>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_v3_clips_with_limit(deserializer, MAX_TIMELINE_CLIPS_PER_TRACK)
}

fn deserialize_v3_clips_with_limit<'de, D>(
    deserializer: D,
    max_clips: usize,
) -> Result<Vec<TimelineClipV3>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct ClipsVisitor {
        max_clips: usize,
    }

    impl<'de> serde::de::Visitor<'de> for ClipsVisitor {
        type Value = Vec<TimelineClipV3>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                formatter,
                "at most {} timeline clips per track",
                self.max_clips
            )
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let capacity = sequence.size_hint().unwrap_or(0).min(self.max_clips);
            let mut clips = Vec::with_capacity(capacity);
            while clips.len() < self.max_clips {
                let Some(clip) = sequence.next_element::<TimelineClipV3>()? else {
                    return Ok(clips);
                };
                clips.push(clip);
            }
            if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(serde::de::Error::custom(
                    "timeline clips per track exceed configured limit",
                ));
            }
            Ok(clips)
        }
    }

    deserializer.deserialize_seq(ClipsVisitor { max_clips })
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

impl From<&ProjectDocument> for ProjectFileV3 {
    fn from(document: &ProjectDocument) -> Self {
        Self {
            format: PROJECT_FORMAT_MARKER.to_owned(),
            schema_version: CURRENT_PROJECT_SCHEMA_VERSION,
            project: ProjectStateV3 {
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
                timeline: ProjectTimelineV3::from(&document.timeline),
            },
        }
    }
}

impl From<&ProjectTimeline> for ProjectTimelineV3 {
    fn from(timeline: &ProjectTimeline) -> Self {
        Self {
            tracks: timeline
                .tracks()
                .iter()
                .map(|track| TimelineTrackV3 {
                    id: track.id(),
                    kind: track.kind().into(),
                    clips: track
                        .clips()
                        .iter()
                        .map(|clip| TimelineClipV3 {
                            id: clip.id(),
                            media_id: clip.media_id(),
                            timeline_start: RationalTimeV3::from(clip.timeline_start()),
                            source_range: TimeRangeV3::from(clip.source_range()),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

impl From<RationalTime> for RationalTimeV3 {
    fn from(time: RationalTime) -> Self {
        Self {
            numerator: time.numerator(),
            denominator: time.denominator(),
        }
    }
}

impl From<TimeRange> for TimeRangeV3 {
    fn from(range: TimeRange) -> Self {
        Self {
            start: RationalTimeV3::from(range.start()),
            duration: RationalTimeV3::from(range.duration()),
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
    InvalidV3Data,
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
            Self::InvalidV3Data => formatter.write_str("project schema version 3 data is invalid"),
            Self::SerializationFailure => {
                formatter.write_str("project document could not be serialized")
            }
        }
    }
}

impl Error for ProjectCodecError {}

/// Encodes canonical project state as readable UTF-8 JSON with a trailing newline.
pub fn encode_project(document: &ProjectDocument) -> Result<String, ProjectCodecError> {
    validate_media_library(&document.media, ProjectCodecError::InvalidV3Data)?;
    document
        .timeline
        .validate(&document.media)
        .map_err(|_| ProjectCodecError::InvalidV3Data)?;
    let mut encoded = serde_json::to_string_pretty(&ProjectFileV3::from(document))
        .map_err(|_| ProjectCodecError::SerializationFailure)?;
    encoded.push('\n');
    Ok(encoded)
}

/// Decodes a versioned `.orproj` JSON document into validated canonical state.
pub fn decode_project(encoded: &str) -> Result<ProjectDocument, ProjectCodecError> {
    let raw: &RawValue =
        serde_json::from_str(encoded).map_err(|_| ProjectCodecError::InvalidJson)?;
    if !raw.get().trim_start().starts_with('{') {
        return Err(ProjectCodecError::InvalidEnvelope);
    }
    let envelope: ProjectEnvelopeProbe<'_> =
        serde_json::from_str(raw.get()).map_err(|_| ProjectCodecError::InvalidEnvelope)?;
    let format: String = envelope
        .format
        .ok_or(ProjectCodecError::InvalidEnvelope)
        .and_then(|value| {
            serde_json::from_str(value.get()).map_err(|_| ProjectCodecError::InvalidEnvelope)
        })?;
    if format != PROJECT_FORMAT_MARKER {
        return Err(ProjectCodecError::WrongFormatMarker);
    }
    let schema_version: u64 = envelope
        .schema_version
        .ok_or(ProjectCodecError::InvalidSchemaVersion)
        .and_then(|value| {
            serde_json::from_str(value.get()).map_err(|_| ProjectCodecError::InvalidSchemaVersion)
        })?;
    match schema_version {
        1 => {
            let file: ProjectFileV1 =
                serde_json::from_str(encoded).map_err(|_| ProjectCodecError::InvalidV1Data)?;
            if file.format != PROJECT_FORMAT_MARKER || file.schema_version != 1 {
                return Err(ProjectCodecError::InvalidV1Data);
            }
            Ok(ProjectDocument::from_v1(file.project))
        }
        2 => {
            let file: ProjectFileV2 =
                serde_json::from_str(encoded).map_err(|_| ProjectCodecError::InvalidV2Data)?;
            if file.format != PROJECT_FORMAT_MARKER || file.schema_version != 2 {
                return Err(ProjectCodecError::InvalidV2Data);
            }
            ProjectDocument::from_v2(file.project)
        }
        3 => {
            let file: ProjectFileV3 =
                serde_json::from_str(encoded).map_err(|_| ProjectCodecError::InvalidV3Data)?;
            if file.format != PROJECT_FORMAT_MARKER || file.schema_version != 3 {
                return Err(ProjectCodecError::InvalidV3Data);
            }
            ProjectDocument::from_v3(file.project)
        }
        _ => Err(ProjectCodecError::UnsupportedSchemaVersion(schema_version)),
    }
}

fn validate_media_library(
    media: &[MediaItem],
    invalid_data: ProjectCodecError,
) -> Result<(), ProjectCodecError> {
    let mut ids = HashSet::with_capacity(media.len());
    let mut sources = HashSet::with_capacity(media.len());
    for item in media {
        item.metadata()
            .validate()
            .map_err(|_| invalid_data.clone())?;
        if !ids.insert(item.id()) || !sources.insert(item.source().uri()) {
            return Err(invalid_data);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CURRENT_PROJECT_SCHEMA_VERSION, ProjectCodecError, ProjectDocument, decode_project,
        deserialize_v3_clips_with_limit, deserialize_v3_tracks_with_limits, encode_project,
    };
    use crate::{
        AudioStreamMetadata, MediaId, MediaItem, MediaMetadata, MediaSourceRef,
        MediaStreamMetadata, ProjectId, ProjectInstanceId, ProjectRevision, ProjectTimeline,
        RationalRate, RationalTime, TimeRange, TimelineClip, TimelineTrack, TrackId, TrackKind,
        VideoStreamMetadata,
    };
    use serde_json::{Value, json};
    use std::{num::NonZeroU32, str::FromStr};

    const PROJECT_ID: &str = "01234567-89ab-4def-8123-456789abcdef";
    const INSTANCE_ID: &str = "fedcba98-7654-4cba-8fed-cba987654321";

    fn fixed_project() -> ProjectDocument {
        ProjectDocument {
            id: ProjectId::from_str(PROJECT_ID).unwrap(),
            revision: ProjectRevision::INITIAL,
            name: "Example".to_owned(),
            media: Vec::new(),
            timeline: ProjectTimeline::default(),
        }
    }

    fn video_media_item(
        id: &str,
        uri: &str,
        container_duration: Option<RationalTime>,
        stream_duration: Option<RationalTime>,
    ) -> MediaItem {
        MediaItem::new(
            MediaId::from_str(id).unwrap(),
            MediaSourceRef::local_file(uri).unwrap(),
            MediaMetadata::from_probe(
                vec!["mov".to_owned()],
                container_duration,
                2048,
                vec![MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                    0,
                    Some("h264".to_owned()),
                    NonZeroU32::new(1920).unwrap(),
                    NonZeroU32::new(1080).unwrap(),
                    Some("yuv420p".to_owned()),
                    Some(RationalRate::new(24, 1).unwrap()),
                    stream_duration,
                ))],
            ),
        )
        .unwrap()
    }

    fn clip_json(
        id: &str,
        media_id: &str,
        timeline_start: (i64, u32),
        source: (i64, u32),
        duration: (i64, u32),
    ) -> Value {
        json!({
            "id": id,
            "media_id": media_id,
            "timeline_start": {"numerator": timeline_start.0, "denominator": timeline_start.1},
            "source_range": {
                "start": {"numerator": source.0, "denominator": source.1},
                "duration": {"numerator": duration.0, "denominator": duration.1}
            }
        })
    }

    fn v3_value(media: Vec<Value>, timeline: Value) -> Value {
        json!({
            "format": "opencut-reinforced-project",
            "schema_version": 3,
            "project": {
                "id": PROJECT_ID,
                "revision": 7,
                "name": "Example",
                "media": media,
                "timeline": timeline
            }
        })
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
    fn project_round_trips_through_v3_json_without_changing_revision() {
        let project = ProjectDocument::new("Example");
        let encoded = encode_project(&project).unwrap();
        let decoded = decode_project(&encoded).unwrap();

        assert_eq!(decoded.id(), project.id());
        assert_eq!(decoded.revision(), project.revision());
        assert_eq!(decoded.name(), project.name());
    }

    #[test]
    fn encoding_uses_the_v3_envelope_and_a_trailing_newline() {
        let project = ProjectDocument::new("Example");
        let encoded = encode_project(&project).unwrap();
        let value: Value = serde_json::from_str(&encoded).unwrap();

        assert_eq!(
            value,
            json!({
                "format": "opencut-reinforced-project",
                "schema_version": 3,
                "project": {
                    "id": project.id().to_string(),
                    "revision": 0,
                    "name": "Example",
                    "media": [],
                    "timeline": { "tracks": [] }
                }
            })
        );
        assert!(encoded.ends_with('\n'));
        assert_eq!(CURRENT_PROJECT_SCHEMA_VERSION, 3);
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
            .replace("\"schema_version\":1", "\"schema_version\":4");

        assert_eq!(
            decode_project(&input),
            Err(ProjectCodecError::UnsupportedSchemaVersion(4))
        );
    }

    #[test]
    fn v1_migration_preserves_identity_revision_and_name_then_encodes_v3() {
        let migrated = decode_project(&encoded_project_with_revision("7")).unwrap();
        assert_eq!(migrated.id().to_string(), PROJECT_ID);
        assert_eq!(migrated.revision(), ProjectRevision::new(7));
        assert_eq!(migrated.name(), "Example");
        assert!(migrated.media_items().is_empty());
        assert!(migrated.timeline().tracks().is_empty());

        let encoded = encode_project(&migrated).unwrap();
        let value: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(value["schema_version"], 3);
        assert_eq!(value["project"]["revision"], 7);
    }

    #[test]
    fn v2_migration_preserves_project_and_media_then_encodes_v3() {
        let media_json = v2_item_json(
            "22222222-2222-4222-8222-222222222222",
            "file:///offline/legacy.mov",
        );
        let migrated = decode_project(&v2_json(&media_json, "")).unwrap();
        let expected_media = MediaItem::new(
            MediaId::from_str("22222222-2222-4222-8222-222222222222").unwrap(),
            MediaSourceRef::local_file("file:///offline/legacy.mov").unwrap(),
            MediaMetadata::from_probe(Vec::new(), None, 0, Vec::new()),
        )
        .unwrap();

        assert_eq!(migrated.id().to_string(), PROJECT_ID);
        assert_eq!(migrated.revision(), ProjectRevision::new(7));
        assert_eq!(migrated.name(), "Example");
        assert_eq!(migrated.media_items(), &[expected_media]);
        assert!(migrated.timeline().tracks().is_empty());
        let encoded: Value = serde_json::from_str(&encode_project(&migrated).unwrap()).unwrap();
        assert_eq!(encoded["schema_version"], 3);
        assert_eq!(encoded["project"]["revision"], 7);
    }

    #[test]
    fn v3_round_trip_preserves_video_timeline_order_ids_ranges_and_media_reuse() {
        let mut project = fixed_project();
        let media = video_media_item(
            "22222222-2222-4222-8222-222222222222",
            "file:///missing/offline-video.mov",
            Some(RationalTime::new(10, 1).unwrap()),
            Some(RationalTime::new(8, 1).unwrap()),
        );
        let media_id = media.id();
        let audio_media = MediaItem::new(
            MediaId::from_str("66666666-6666-4666-8666-666666666666").unwrap(),
            MediaSourceRef::local_file("file:///missing/offline-audio.wav").unwrap(),
            MediaMetadata::from_probe(
                vec!["wav".to_owned()],
                Some(RationalTime::new(3, 1).unwrap()),
                512,
                vec![MediaStreamMetadata::Audio(AudioStreamMetadata::from_probe(
                    0,
                    Some("pcm_s16le".to_owned()),
                    NonZeroU32::new(48_000),
                    NonZeroU32::new(2),
                    Some("stereo".to_owned()),
                    Some(RationalTime::new(3, 1).unwrap()),
                ))],
            ),
        )
        .unwrap();
        project.media = vec![media, audio_media];
        project.timeline = ProjectTimeline::from_tracks_for_codec(vec![
            TimelineTrack::from_parts_for_codec(
                TrackId::from_str("33333333-3333-4333-8333-333333333333").unwrap(),
                TrackKind::Video,
                vec![
                    TimelineClip::from_parts_for_codec(
                        crate::ClipId::from_str("44444444-4444-4444-8444-444444444444").unwrap(),
                        media_id,
                        RationalTime::ZERO,
                        TimeRange::new(RationalTime::ZERO, RationalTime::new(2, 1).unwrap())
                            .unwrap(),
                    ),
                    TimelineClip::from_parts_for_codec(
                        crate::ClipId::from_str("55555555-5555-4555-8555-555555555555").unwrap(),
                        media_id,
                        RationalTime::new(2, 1).unwrap(),
                        TimeRange::new(
                            RationalTime::new(2, 1).unwrap(),
                            RationalTime::new(2, 1).unwrap(),
                        )
                        .unwrap(),
                    ),
                ],
            ),
            TimelineTrack::from_parts_for_codec(
                TrackId::from_str("77777777-7777-4777-8777-777777777777").unwrap(),
                TrackKind::Audio,
                vec![TimelineClip::from_parts_for_codec(
                    crate::ClipId::from_str("88888888-8888-4888-8888-888888888888").unwrap(),
                    MediaId::from_str("66666666-6666-4666-8666-666666666666").unwrap(),
                    RationalTime::ZERO,
                    TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap()).unwrap(),
                )],
            ),
        ]);

        let encoded = encode_project(&project).unwrap();
        let value: Value = serde_json::from_str(&encoded).unwrap();
        let decoded = decode_project(&encoded).unwrap();

        assert_eq!(value["schema_version"], 3);
        assert_eq!(value["project"]["timeline"]["tracks"][0]["kind"], "video");
        assert_eq!(value["project"]["timeline"]["tracks"][1]["kind"], "audio");
        assert_eq!(decoded, project);
        assert_eq!(decoded.timeline().tracks().len(), 2);
        assert_eq!(decoded.timeline().tracks()[0].clips().len(), 2);
        assert_eq!(
            decoded.timeline().tracks()[0].clips()[1].media_id(),
            media_id
        );
        assert_eq!(decoded.timeline().tracks()[1].kind(), TrackKind::Audio);
    }

    #[test]
    fn v3_rejects_unknown_timeline_track_clip_and_time_fields_and_kinds() {
        let track_id = "33333333-3333-4333-8333-333333333333";
        let clip_id = "44444444-4444-4444-8444-444444444444";
        let media_id = "22222222-2222-4222-8222-222222222222";
        let cases = [
            v3_value(Vec::new(), json!({"tracks": [], "unexpected": true})),
            v3_value(
                Vec::new(),
                json!({"tracks": [{"id": track_id, "kind": "video", "clips": [], "unexpected": true}]}),
            ),
            v3_value(
                Vec::new(),
                json!({"tracks": [{"id": track_id, "kind": "video", "clips": [{"id": clip_id, "media_id": media_id, "timeline_start": {"numerator": 0, "denominator": 1}, "source_range": {"start": {"numerator": 0, "denominator": 1}, "duration": {"numerator": 1, "denominator": 1}}, "unexpected": true}]}]}),
            ),
            v3_value(
                Vec::new(),
                json!({"tracks": [{"id": track_id, "kind": "subtitle", "clips": []}]}),
            ),
            v3_value(
                Vec::new(),
                json!({"tracks": [{"id": track_id, "kind": "mixed", "clips": []}]}),
            ),
            v3_value(
                Vec::new(),
                json!({"tracks": [{"id": track_id, "kind": "video", "clips": [{"id": clip_id, "media_id": media_id, "timeline_start": {"numerator": 0, "denominator": 1, "unexpected": true}, "source_range": {"start": {"numerator": 0, "denominator": 1}, "duration": {"numerator": 1, "denominator": 1}}}]}]}),
            ),
            v3_value(
                Vec::new(),
                json!({"tracks": [{"id": track_id, "kind": "video", "clips": [{"id": clip_id, "media_id": media_id, "timeline_start": {"numerator": 0, "denominator": 1}, "source_range": {"start": {"numerator": 0, "denominator": 1}, "duration": {"numerator": 1, "denominator": 1, "unexpected": true}}}]}]}),
            ),
        ];
        for value in cases {
            assert_eq!(
                decode_project(&value.to_string()),
                Err(ProjectCodecError::InvalidV3Data)
            );
        }
    }

    #[test]
    fn v3_codec_applies_timeline_reference_range_order_and_overflow_validation() {
        let media = video_media_item(
            "22222222-2222-4222-8222-222222222222",
            "file:///missing/offline-video.mov",
            Some(RationalTime::new(10, 1).unwrap()),
            Some(RationalTime::new(8, 1).unwrap()),
        );
        let media_value = serde_json::to_value(&media).unwrap();
        let track_id = "33333333-3333-4333-8333-333333333333";
        let clip_id = "44444444-4444-4444-8444-444444444444";
        let media_id = "22222222-2222-4222-8222-222222222222";
        let valid_clip = clip_json(clip_id, media_id, (0, 1), (0, 1), (1, 1));
        let make_value = |clips: Vec<Value>, kind: &str| {
            v3_value(
                vec![media_value.clone()],
                json!({"tracks": [{"id": track_id, "kind": kind, "clips": clips}]}),
            )
        };
        let mut unknown_media = valid_clip.clone();
        unknown_media["media_id"] = json!("66666666-6666-4666-8666-666666666666");
        let mut negative_timeline = valid_clip.clone();
        negative_timeline["timeline_start"]["numerator"] = json!(-1);
        let mut negative_source = valid_clip.clone();
        negative_source["source_range"]["start"]["numerator"] = json!(-1);
        let mut zero_duration = valid_clip.clone();
        zero_duration["source_range"]["duration"]["numerator"] = json!(0);
        let mut past_stream_end = valid_clip.clone();
        past_stream_end["source_range"]["start"]["numerator"] = json!(7);
        past_stream_end["source_range"]["duration"]["numerator"] = json!(2);
        let mut timeline_overflow = valid_clip.clone();
        timeline_overflow["timeline_start"]["numerator"] = json!(i64::MAX);
        let mut source_overflow = valid_clip.clone();
        source_overflow["source_range"]["start"]["numerator"] = json!(i64::MAX);
        let out_of_order = vec![
            clip_json(
                "55555555-5555-4555-8555-555555555555",
                media_id,
                (2, 1),
                (0, 1),
                (1, 1),
            ),
            clip_json(
                "66666666-6666-4666-8666-666666666666",
                media_id,
                (0, 1),
                (1, 1),
                (1, 1),
            ),
        ];
        let duplicate_track = v3_value(
            vec![media_value.clone()],
            json!({"tracks": [
                {"id": track_id, "kind": "video", "clips": []},
                {"id": track_id, "kind": "video", "clips": []}
            ]}),
        );
        let duplicate_clip_same_track = make_value(
            vec![
                valid_clip.clone(),
                clip_json(clip_id, media_id, (1, 1), (1, 1), (1, 1)),
            ],
            "video",
        );
        let duplicate_clip_across_tracks = v3_value(
            vec![media_value.clone()],
            json!({"tracks": [
                {"id": track_id, "kind": "video", "clips": [valid_clip.clone()]},
                {"id": "77777777-7777-4777-8777-777777777777", "kind": "video", "clips": [
                    clip_json(clip_id, media_id, (4, 1), (1, 1), (1, 1))
                ]}
            ]}),
        );
        let cases = [
            make_value(vec![unknown_media], "video"),
            make_value(vec![valid_clip.clone()], "audio"),
            make_value(vec![negative_timeline], "video"),
            make_value(vec![negative_source], "video"),
            make_value(vec![zero_duration], "video"),
            make_value(vec![past_stream_end], "video"),
            make_value(vec![timeline_overflow], "video"),
            make_value(vec![source_overflow], "video"),
            make_value(out_of_order, "video"),
            duplicate_track,
            duplicate_clip_same_track,
            duplicate_clip_across_tracks,
        ];
        for value in cases {
            assert_eq!(
                decode_project(&value.to_string()),
                Err(ProjectCodecError::InvalidV3Data)
            );
        }

        let exact_end = clip_json(clip_id, media_id, (0, 1), (7, 1), (1, 1));
        assert!(decode_project(&make_value(vec![exact_end], "video").to_string()).is_ok());
    }

    #[test]
    fn bounded_timeline_track_deserialization_checks_track_and_total_clip_limits() {
        let clip_a = clip_json(
            "44444444-4444-4444-8444-444444444444",
            "22222222-2222-4222-8222-222222222222",
            (0, 1),
            (0, 1),
            (1, 1),
        );
        let clip_b = clip_json(
            "55555555-5555-4555-8555-555555555555",
            "22222222-2222-4222-8222-222222222222",
            (1, 1),
            (1, 1),
            (1, 1),
        );
        let clips = json!([clip_a, clip_b]);
        let tracks = json!([
            {"id": "33333333-3333-4333-8333-333333333333", "kind": "video", "clips": [clips[0].clone()]},
            {"id": "66666666-6666-4666-8666-666666666666", "kind": "video", "clips": [clips[1].clone()]}
        ])
        .to_string();
        let mut deserializer = serde_json::Deserializer::from_str(&tracks);
        assert!(deserialize_v3_tracks_with_limits(&mut deserializer, 2, 2).is_ok());
        let mut deserializer = serde_json::Deserializer::from_str(&tracks);
        assert!(deserialize_v3_tracks_with_limits(&mut deserializer, 1, 2).is_err());
        let mut deserializer = serde_json::Deserializer::from_str(&tracks);
        assert!(deserialize_v3_tracks_with_limits(&mut deserializer, 2, 1).is_err());

        let clips = clips.to_string();
        let mut deserializer = serde_json::Deserializer::from_str(&clips);
        assert!(deserialize_v3_clips_with_limit(&mut deserializer, 2).is_ok());
        let mut deserializer = serde_json::Deserializer::from_str(&clips);
        assert!(deserialize_v3_clips_with_limit(&mut deserializer, 1).is_err());
    }

    #[test]
    fn v3_round_trip_preserves_media_identity_metadata_and_insertion_order() {
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
