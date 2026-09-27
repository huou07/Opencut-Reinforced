use crate::{MediaId, MediaItem, MediaStreamMetadata, RationalTime, TimeRange};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    str::FromStr,
};
use uuid::{Uuid, Version};

pub const MAX_TIMELINE_TRACKS: usize = 256;
pub const MAX_TIMELINE_CLIPS: usize = 100_000;
pub const MAX_TIMELINE_CLIPS_PER_TRACK: usize = 100_000;

macro_rules! timeline_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn generate() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, formatter)
            }
        }

        impl FromStr for $name {
            type Err = TimelineIdParseError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let uuid = Uuid::parse_str(value).map_err(TimelineIdParseError::InvalidUuid)?;
                if uuid.to_string() != value {
                    return Err(TimelineIdParseError::NotCanonical);
                }
                if uuid.get_version() != Some(Version::Random) {
                    return Err(TimelineIdParseError::NotVersion4);
                }
                Ok(Self(uuid))
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                value.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

timeline_id!(TrackId);
timeline_id!(ClipId);

#[derive(Debug)]
pub enum TimelineIdParseError {
    InvalidUuid(uuid::Error),
    NotCanonical,
    NotVersion4,
}

impl fmt::Display for TimelineIdParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidUuid(_) => "timeline ID must be a UUID",
            Self::NotCanonical => "timeline ID must use canonical lowercase UUID text",
            Self::NotVersion4 => "timeline ID must be UUID version 4",
        })
    }
}

impl Error for TimelineIdParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidUuid(error) => Some(error),
            Self::NotCanonical | Self::NotVersion4 => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrackKind {
    Video,
    Audio,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineClip {
    id: ClipId,
    media_id: MediaId,
    timeline_start: RationalTime,
    source_range: TimeRange,
}

impl TimelineClip {
    pub const fn id(&self) -> ClipId {
        self.id
    }

    pub const fn media_id(&self) -> MediaId {
        self.media_id
    }

    pub const fn timeline_start(&self) -> RationalTime {
        self.timeline_start
    }

    pub const fn source_range(&self) -> TimeRange {
        self.source_range
    }

    pub(crate) const fn from_parts_for_codec(
        id: ClipId,
        media_id: MediaId,
        timeline_start: RationalTime,
        source_range: TimeRange,
    ) -> Self {
        Self {
            id,
            media_id,
            timeline_start,
            source_range,
        }
    }

    pub(crate) const fn from_parts_for_command(
        id: ClipId,
        media_id: MediaId,
        timeline_start: RationalTime,
        source_range: TimeRange,
    ) -> Self {
        Self {
            id,
            media_id,
            timeline_start,
            source_range,
        }
    }

    pub(crate) const fn with_timeline_start_for_command(
        self,
        timeline_start: RationalTime,
    ) -> Self {
        Self {
            timeline_start,
            ..self
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineTrack {
    id: TrackId,
    kind: TrackKind,
    clips: Vec<TimelineClip>,
}

impl TimelineTrack {
    pub const fn id(&self) -> TrackId {
        self.id
    }

    pub const fn kind(&self) -> TrackKind {
        self.kind
    }

    pub fn clips(&self) -> &[TimelineClip] {
        &self.clips
    }

    pub(crate) fn from_parts_for_codec(
        id: TrackId,
        kind: TrackKind,
        clips: Vec<TimelineClip>,
    ) -> Self {
        Self { id, kind, clips }
    }

    pub(crate) fn empty_for_command(id: TrackId, kind: TrackKind) -> Self {
        Self {
            id,
            kind,
            clips: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectTimeline {
    tracks: Vec<TimelineTrack>,
}

impl ProjectTimeline {
    pub fn tracks(&self) -> &[TimelineTrack] {
        &self.tracks
    }

    pub(crate) fn from_tracks_for_codec(tracks: Vec<TimelineTrack>) -> Self {
        Self { tracks }
    }

    pub(crate) fn validate(&self, media: &[MediaItem]) -> Result<(), TimelineValidationError> {
        self.validate_with_limits(
            media,
            MAX_TIMELINE_TRACKS,
            MAX_TIMELINE_CLIPS,
            MAX_TIMELINE_CLIPS_PER_TRACK,
        )
    }

    fn validate_with_limits(
        &self,
        media: &[MediaItem],
        max_tracks: usize,
        max_clips: usize,
        max_clips_per_track: usize,
    ) -> Result<(), TimelineValidationError> {
        if self.tracks.len() > max_tracks {
            return Err(TimelineValidationError);
        }

        let mut media_by_id = HashMap::with_capacity(media.len());
        for item in media {
            media_by_id.insert(item.id(), item);
        }

        let mut track_ids = HashSet::with_capacity(self.tracks.len());
        let mut clip_ids = HashSet::new();
        let mut total_clips = 0usize;
        for track in &self.tracks {
            if !track_ids.insert(track.id) || track.clips.len() > max_clips_per_track {
                return Err(TimelineValidationError);
            }
            total_clips = total_clips
                .checked_add(track.clips.len())
                .filter(|count| *count <= max_clips)
                .ok_or(TimelineValidationError)?;

            let mut previous_start = None;
            let mut previous_end = None;
            for clip in &track.clips {
                if !clip_ids.insert(clip.id) || clip.timeline_start.is_negative() {
                    return Err(TimelineValidationError);
                }

                let source_start = clip.source_range.start();
                let duration = clip.source_range.duration();
                if source_start.is_negative() || !duration.is_positive() {
                    return Err(TimelineValidationError);
                }

                let timeline_end = clip
                    .timeline_start
                    .checked_add(duration)
                    .map_err(|_| TimelineValidationError)?;
                let source_end = source_start
                    .checked_add(duration)
                    .map_err(|_| TimelineValidationError)?;

                if previous_start.is_some_and(|start| clip.timeline_start <= start)
                    || previous_end.is_some_and(|end| clip.timeline_start < end)
                {
                    return Err(TimelineValidationError);
                }

                let item = media_by_id
                    .get(&clip.media_id)
                    .ok_or(TimelineValidationError)?;
                let (has_compatible_stream, stream_duration) = matching_stream(track.kind, item);
                if !has_compatible_stream {
                    return Err(TimelineValidationError);
                }
                let known_duration = stream_duration.or(item.metadata().duration());
                if known_duration.is_some_and(|end| source_end > end) {
                    return Err(TimelineValidationError);
                }

                previous_start = Some(clip.timeline_start);
                previous_end = Some(timeline_end);
            }
        }
        Ok(())
    }

    pub(crate) fn references_media(&self, media_id: MediaId) -> bool {
        self.tracks
            .iter()
            .flat_map(|track| &track.clips)
            .any(|clip| clip.media_id == media_id)
    }

    pub(crate) fn try_reserve_tracks(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.tracks.try_reserve(additional)
    }

    pub(crate) fn try_reserve_track_clips(
        &mut self,
        track_index: usize,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.tracks[track_index].clips.try_reserve(additional)
    }

    pub(crate) fn insert_track_for_command(&mut self, index: usize, track: TimelineTrack) {
        self.tracks.insert(index, track);
    }

    pub(crate) fn remove_track_for_command(&mut self, index: usize) -> TimelineTrack {
        self.tracks.remove(index)
    }

    pub(crate) fn insert_clip_for_command(
        &mut self,
        track_index: usize,
        index: usize,
        clip: TimelineClip,
    ) {
        self.tracks[track_index].clips.insert(index, clip);
    }

    pub(crate) fn remove_clip_for_command(
        &mut self,
        track_index: usize,
        index: usize,
    ) -> TimelineClip {
        self.tracks[track_index].clips.remove(index)
    }
}

pub(crate) fn matching_stream(kind: TrackKind, item: &MediaItem) -> (bool, Option<RationalTime>) {
    let stream = item.metadata().streams().iter().find(|stream| {
        matches!(
            (kind, stream),
            (TrackKind::Video, MediaStreamMetadata::Video(_))
                | (TrackKind::Audio, MediaStreamMetadata::Audio(_))
        )
    });
    let duration = stream.and_then(|stream| match stream {
        MediaStreamMetadata::Video(stream) => stream.duration(),
        MediaStreamMetadata::Audio(stream) => stream.duration(),
        MediaStreamMetadata::Other(_) => None,
    });
    (stream.is_some(), duration)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TimelineValidationError;

impl fmt::Display for TimelineValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("timeline data is invalid or exceeds a configured bound")
    }
}

impl Error for TimelineValidationError {}

#[cfg(test)]
mod tests {
    use super::{
        ClipId, MAX_TIMELINE_CLIPS, MAX_TIMELINE_CLIPS_PER_TRACK, MAX_TIMELINE_TRACKS,
        ProjectTimeline, TimelineClip, TimelineTrack, TrackId, TrackKind,
    };
    use crate::{
        AudioStreamMetadata, MediaId, MediaItem, MediaMetadata, MediaSourceRef,
        MediaStreamMetadata, OtherStreamMetadata, RationalRate, RationalTime, TimeRange,
        VideoStreamMetadata,
    };
    use std::{num::NonZeroU32, str::FromStr};

    const TRACK_ID: &str = "22222222-2222-4222-8222-222222222222";
    const CLIP_ID: &str = "33333333-3333-4333-8333-333333333333";
    const MEDIA_ID: &str = "44444444-4444-4444-8444-444444444444";

    fn time(numerator: i64, denominator: u32) -> RationalTime {
        RationalTime::new(numerator, denominator).unwrap()
    }

    fn media(duration: Option<RationalTime>, streams: Vec<MediaStreamMetadata>) -> MediaItem {
        MediaItem::new(
            MediaId::from_str(MEDIA_ID).unwrap(),
            MediaSourceRef::local_file("file:///missing/offline.mov").unwrap(),
            MediaMetadata::from_probe(vec!["mov".to_owned()], duration, 0, streams),
        )
        .unwrap()
    }

    fn video_stream(duration: Option<RationalTime>) -> MediaStreamMetadata {
        MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
            0,
            None,
            NonZeroU32::new(1920).unwrap(),
            NonZeroU32::new(1080).unwrap(),
            None,
            Some(RationalRate::new(24, 1).unwrap()),
            duration,
        ))
    }

    fn audio_stream(duration: Option<RationalTime>) -> MediaStreamMetadata {
        MediaStreamMetadata::Audio(AudioStreamMetadata::from_probe(
            1,
            None,
            NonZeroU32::new(48_000),
            NonZeroU32::new(2),
            None,
            duration,
        ))
    }

    fn clip(
        id: &str,
        start: RationalTime,
        source_start: RationalTime,
        duration: RationalTime,
    ) -> TimelineClip {
        TimelineClip::from_parts_for_codec(
            ClipId::from_str(id).unwrap(),
            MediaId::from_str(MEDIA_ID).unwrap(),
            start,
            TimeRange::new(source_start, duration).unwrap(),
        )
    }

    fn track(id: &str, kind: TrackKind, clips: Vec<TimelineClip>) -> TimelineTrack {
        TimelineTrack::from_parts_for_codec(TrackId::from_str(id).unwrap(), kind, clips)
    }

    #[test]
    fn timeline_ids_generate_and_round_trip_as_canonical_uuid_v4_text() {
        let generated_track = TrackId::generate();
        let generated_clip = ClipId::generate();
        assert_eq!(
            uuid::Uuid::parse_str(&generated_track.to_string())
                .unwrap()
                .get_version(),
            Some(uuid::Version::Random)
        );
        assert_eq!(
            uuid::Uuid::parse_str(&generated_clip.to_string())
                .unwrap()
                .get_version(),
            Some(uuid::Version::Random)
        );
        for (generated, parsed, serialized) in [
            (
                generated_track.to_string(),
                TrackId::from_str(TRACK_ID).unwrap().to_string(),
                serde_json::to_string(&TrackId::from_str(TRACK_ID).unwrap()).unwrap(),
            ),
            (
                generated_clip.to_string(),
                ClipId::from_str(CLIP_ID).unwrap().to_string(),
                serde_json::to_string(&ClipId::from_str(CLIP_ID).unwrap()).unwrap(),
            ),
        ] {
            assert_eq!(generated.len(), 36);
            assert_eq!(parsed.len(), 36);
            assert_eq!(serde_json::from_str::<String>(&serialized).unwrap(), parsed);
        }
        assert_eq!(
            serde_json::from_str::<TrackId>(
                &serde_json::to_string(&TrackId::from_str(TRACK_ID).unwrap()).unwrap()
            )
            .unwrap(),
            TrackId::from_str(TRACK_ID).unwrap()
        );
        assert_eq!(
            serde_json::from_str::<ClipId>(
                &serde_json::to_string(&ClipId::from_str(CLIP_ID).unwrap()).unwrap()
            )
            .unwrap(),
            ClipId::from_str(CLIP_ID).unwrap()
        );
    }

    #[test]
    fn timeline_ids_reject_malformed_noncanonical_and_non_v4_values() {
        assert!(TrackId::from_str("not-a-uuid").is_err());
        assert!(ClipId::from_str("not-a-uuid").is_err());
        assert!(TrackId::from_str("22222222-2222-1222-8222-222222222222").is_err());
        assert!(ClipId::from_str("33333333-3333-1333-8333-333333333333").is_err());
        assert!(TrackId::from_str("22222222-2222-4222-8222-22222222222A").is_err());
        assert!(
            serde_json::from_str::<ClipId>("\"33333333-3333-1333-8333-333333333333\"").is_err()
        );
    }

    #[test]
    fn default_timeline_is_empty_and_locked_bounds_are_stable() {
        assert!(ProjectTimeline::default().tracks().is_empty());
        assert_eq!(MAX_TIMELINE_TRACKS, 256);
        assert_eq!(MAX_TIMELINE_CLIPS, 100_000);
        assert_eq!(MAX_TIMELINE_CLIPS_PER_TRACK, 100_000);
    }

    #[test]
    fn validation_accepts_compatible_offline_media_and_same_media_reuse() {
        let item = media(Some(time(12, 1)), vec![video_stream(None)]);
        let timeline = ProjectTimeline::from_tracks_for_codec(vec![track(
            TRACK_ID,
            TrackKind::Video,
            vec![
                clip(CLIP_ID, time(0, 1), time(0, 1), time(3, 1)),
                clip(
                    "55555555-5555-4555-8555-555555555555",
                    time(3, 1),
                    time(3, 1),
                    time(3, 1),
                ),
            ],
        )]);
        assert!(timeline.validate(&[item]).is_ok());
    }

    #[test]
    fn unknown_source_duration_does_not_invent_an_upper_bound() {
        let item = media(None, vec![video_stream(None)]);
        let timeline = ProjectTimeline::from_tracks_for_codec(vec![track(
            TRACK_ID,
            TrackKind::Video,
            vec![clip(CLIP_ID, time(0, 1), time(1_000_000, 1), time(3, 1))],
        )]);
        assert!(timeline.validate(&[item]).is_ok());
    }

    #[test]
    fn validation_uses_first_matching_stream_then_container_duration_fallback() {
        let item = media(
            Some(time(10, 1)),
            vec![
                MediaStreamMetadata::Other(OtherStreamMetadata::from_probe(0, None, None)),
                video_stream(None),
                video_stream(Some(time(1, 1))),
            ],
        );
        let timeline = ProjectTimeline::from_tracks_for_codec(vec![track(
            TRACK_ID,
            TrackKind::Video,
            vec![clip(CLIP_ID, time(0, 1), time(8, 1), time(2, 1))],
        )]);
        assert!(timeline.validate(&[item]).is_ok());
    }

    #[test]
    fn validation_rejects_media_reference_time_order_and_compatibility_errors() {
        let video = media(None, vec![video_stream(None)]);
        let audio_only = MediaItem::new(
            MediaId::from_str("66666666-6666-4666-8666-666666666666").unwrap(),
            MediaSourceRef::local_file("file:///missing/audio.wav").unwrap(),
            MediaMetadata::from_probe(vec!["wav".to_owned()], None, 0, vec![audio_stream(None)]),
        )
        .unwrap();

        let cases = [
            ProjectTimeline::from_tracks_for_codec(vec![track(
                TRACK_ID,
                TrackKind::Video,
                vec![TimelineClip::from_parts_for_codec(
                    ClipId::from_str(CLIP_ID).unwrap(),
                    MediaId::from_str("77777777-7777-4777-8777-777777777777").unwrap(),
                    time(0, 1),
                    TimeRange::new(time(0, 1), time(1, 1)).unwrap(),
                )],
            )]),
            ProjectTimeline::from_tracks_for_codec(vec![track(
                TRACK_ID,
                TrackKind::Audio,
                vec![clip(CLIP_ID, time(0, 1), time(0, 1), time(1, 1))],
            )]),
            ProjectTimeline::from_tracks_for_codec(vec![track(
                TRACK_ID,
                TrackKind::Video,
                vec![clip(CLIP_ID, time(-1, 1), time(0, 1), time(1, 1))],
            )]),
            ProjectTimeline::from_tracks_for_codec(vec![track(
                TRACK_ID,
                TrackKind::Video,
                vec![clip(CLIP_ID, time(0, 1), time(-1, 1), time(1, 1))],
            )]),
            ProjectTimeline::from_tracks_for_codec(vec![track(
                TRACK_ID,
                TrackKind::Video,
                vec![clip(CLIP_ID, time(0, 1), time(0, 1), time(0, 1))],
            )]),
            ProjectTimeline::from_tracks_for_codec(vec![track(
                TRACK_ID,
                TrackKind::Video,
                vec![
                    clip(CLIP_ID, time(0, 1), time(0, 1), time(4, 1)),
                    clip(
                        "55555555-5555-4555-8555-555555555555",
                        time(3, 1),
                        time(0, 1),
                        time(1, 1),
                    ),
                ],
            )]),
            ProjectTimeline::from_tracks_for_codec(vec![
                track(
                    TRACK_ID,
                    TrackKind::Video,
                    vec![clip(CLIP_ID, time(3, 1), time(0, 1), time(1, 1))],
                ),
                track(
                    "88888888-8888-4888-8888-888888888888",
                    TrackKind::Audio,
                    vec![TimelineClip::from_parts_for_codec(
                        ClipId::from_str("99999999-9999-4999-8999-999999999999").unwrap(),
                        MediaId::from_str("66666666-6666-4666-8666-666666666666").unwrap(),
                        time(0, 1),
                        TimeRange::new(time(0, 1), time(1, 1)).unwrap(),
                    )],
                ),
            ]),
            ProjectTimeline::from_tracks_for_codec(vec![track(
                TRACK_ID,
                TrackKind::Video,
                vec![TimelineClip::from_parts_for_codec(
                    ClipId::from_str("99999999-9999-4999-8999-999999999999").unwrap(),
                    audio_only.id(),
                    time(0, 1),
                    TimeRange::new(time(0, 1), time(1, 1)).unwrap(),
                )],
            )]),
        ];
        assert!(
            cases[0]
                .validate(&[video.clone(), audio_only.clone()])
                .is_err()
        );
        assert!(
            cases[1]
                .validate(&[video.clone(), audio_only.clone()])
                .is_err()
        );
        assert!(cases[2].validate(std::slice::from_ref(&video)).is_err());
        assert!(cases[3].validate(std::slice::from_ref(&video)).is_err());
        assert!(cases[4].validate(std::slice::from_ref(&video)).is_err());
        assert!(cases[5].validate(std::slice::from_ref(&video)).is_err());
        assert!(
            cases[6]
                .validate(&[video.clone(), audio_only.clone()])
                .is_ok()
        );
        assert!(
            cases[7]
                .validate(std::slice::from_ref(&audio_only))
                .is_err()
        );
    }

    #[test]
    fn validation_rejects_duplicate_track_and_clip_ids_globally() {
        let first_media = media(None, vec![video_stream(None)]);
        let duplicate_tracks = ProjectTimeline::from_tracks_for_codec(vec![
            track(TRACK_ID, TrackKind::Video, vec![]),
            track(TRACK_ID, TrackKind::Audio, vec![]),
        ]);
        assert!(
            duplicate_tracks
                .validate(std::slice::from_ref(&first_media))
                .is_err()
        );

        let duplicate_clips = ProjectTimeline::from_tracks_for_codec(vec![
            track(
                TRACK_ID,
                TrackKind::Video,
                vec![clip(CLIP_ID, time(0, 1), time(0, 1), time(1, 1))],
            ),
            track(
                "88888888-8888-4888-8888-888888888888",
                TrackKind::Video,
                vec![clip(CLIP_ID, time(0, 1), time(1, 1), time(1, 1))],
            ),
        ]);
        assert!(duplicate_clips.validate(&[first_media]).is_err());

        let second_media = media(None, vec![video_stream(None)]);
        let duplicate_clips_on_one_track = ProjectTimeline::from_tracks_for_codec(vec![track(
            TRACK_ID,
            TrackKind::Video,
            vec![
                clip(CLIP_ID, time(0, 1), time(0, 1), time(1, 1)),
                clip(CLIP_ID, time(1, 1), time(1, 1), time(1, 1)),
            ],
        )]);
        assert!(
            duplicate_clips_on_one_track
                .validate(&[second_media])
                .is_err()
        );
    }

    #[test]
    fn validation_checks_clip_and_project_bounds_before_accepting() {
        let media = media(None, vec![video_stream(None)]);
        let three_tracks = ProjectTimeline::from_tracks_for_codec(vec![
            track(TRACK_ID, TrackKind::Video, vec![]),
            track(
                "55555555-5555-4555-8555-555555555555",
                TrackKind::Video,
                vec![],
            ),
            track(
                "66666666-6666-4666-8666-666666666666",
                TrackKind::Video,
                vec![],
            ),
        ]);
        assert!(
            three_tracks
                .validate_with_limits(std::slice::from_ref(&media), 3, 4, 4)
                .is_ok()
        );
        assert!(
            three_tracks
                .validate_with_limits(std::slice::from_ref(&media), 2, 4, 4)
                .is_err()
        );

        let clips = ProjectTimeline::from_tracks_for_codec(vec![track(
            TRACK_ID,
            TrackKind::Video,
            vec![
                clip(CLIP_ID, time(0, 1), time(0, 1), time(1, 1)),
                clip(
                    "55555555-5555-4555-8555-555555555555",
                    time(1, 1),
                    time(1, 1),
                    time(1, 1),
                ),
                clip(
                    "66666666-6666-4666-8666-666666666666",
                    time(2, 1),
                    time(2, 1),
                    time(1, 1),
                ),
            ],
        )]);
        assert!(
            clips
                .validate_with_limits(std::slice::from_ref(&media), 3, 3, 3)
                .is_ok()
        );
        assert!(
            clips
                .validate_with_limits(std::slice::from_ref(&media), 3, 2, 3)
                .is_err()
        );
        assert!(
            clips
                .validate_with_limits(std::slice::from_ref(&media), 3, 3, 2)
                .is_err()
        );
    }
}
