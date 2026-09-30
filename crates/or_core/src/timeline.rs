use crate::{
    MediaId, MediaItem, MediaStreamMetadata, RationalRate, RationalTime, TimeError, TimeRange,
};
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
pub const MAX_TIMELINE_MARKERS: usize = 10_000;
pub const MAX_TIMELINE_MARKER_LABEL_BYTES: usize = 256;

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
timeline_id!(MarkerId);

impl Ord for MarkerId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.as_bytes().cmp(other.0.as_bytes())
    }
}

impl PartialOrd for MarkerId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

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
pub struct TimelineMarker {
    id: MarkerId,
    timeline_time: RationalTime,
    label: String,
}

impl TimelineMarker {
    pub const fn id(&self) -> MarkerId {
        self.id
    }

    pub const fn timeline_time(&self) -> RationalTime {
        self.timeline_time
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn from_parts_for_codec(
        id: MarkerId,
        timeline_time: RationalTime,
        label: String,
    ) -> Self {
        Self {
            id,
            timeline_time,
            label,
        }
    }

    pub(crate) fn from_parts_for_command(
        id: MarkerId,
        timeline_time: RationalTime,
        label: String,
    ) -> Self {
        Self {
            id,
            timeline_time,
            label,
        }
    }

    pub(crate) fn with_timeline_time_for_command(self, timeline_time: RationalTime) -> Self {
        Self {
            timeline_time,
            ..self
        }
    }

    pub(crate) fn with_label_for_command(self, label: String) -> Self {
        Self { label, ..self }
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
    markers: Vec<TimelineMarker>,
    sequence_frame_rate: Option<RationalRate>,
}

impl ProjectTimeline {
    pub fn tracks(&self) -> &[TimelineTrack] {
        &self.tracks
    }

    pub fn markers(&self) -> &[TimelineMarker] {
        &self.markers
    }

    pub const fn sequence_frame_rate(&self) -> Option<RationalRate> {
        self.sequence_frame_rate
    }

    /// Returns the greatest exact clip end across audio and video tracks.
    pub fn content_end(&self) -> Result<Option<RationalTime>, TimeError> {
        let mut end = None;
        for clip in self.tracks.iter().flat_map(|track| &track.clips) {
            let clip_end = clip
                .timeline_start
                .checked_add(clip.source_range.duration())?;
            end = Some(end.map_or(clip_end, |current: RationalTime| current.max(clip_end)));
        }
        Ok(end)
    }

    /// Returns the exact global sequence-lattice time if it precedes the content end.
    pub fn frame_time(
        &self,
        frame_index: u64,
    ) -> Result<Option<RationalTime>, SequenceTimingError> {
        let rate = self
            .sequence_frame_rate
            .ok_or(SequenceTimingError::FrameRateUnavailable)?;
        let Some(content_end) = self.content_end()? else {
            return Ok(None);
        };
        let time = rate.frame_time(frame_index)?;
        Ok((time < content_end).then_some(time))
    }

    /// Selects `floor(playhead × rate) + 1`, or no frame at/beyond content end.
    pub fn next_frame_time(
        &self,
        playhead: RationalTime,
    ) -> Result<Option<RationalTime>, SequenceTimingError> {
        let rate = self
            .sequence_frame_rate
            .ok_or(SequenceTimingError::FrameRateUnavailable)?;
        if playhead.is_negative() {
            return Err(TimeError::NegativeTime.into());
        }
        let Some(content_end) = self.content_end()? else {
            return Ok(None);
        };
        if playhead >= content_end {
            return Ok(None);
        }
        let next_index = rate
            .frame_index_floor(playhead)?
            .checked_add(1)
            .ok_or(TimeError::ArithmeticOverflow)?;
        let time = rate.frame_time(next_index)?;
        Ok((time < content_end).then_some(time))
    }

    /// Selects `ceil(playhead × rate) - 1`, clamped to the valid sequence frames.
    pub fn previous_frame_time(
        &self,
        playhead: RationalTime,
    ) -> Result<Option<RationalTime>, SequenceTimingError> {
        let rate = self
            .sequence_frame_rate
            .ok_or(SequenceTimingError::FrameRateUnavailable)?;
        if playhead.is_negative() {
            return Err(TimeError::NegativeTime.into());
        }
        let Some(content_end) = self.content_end()? else {
            return Ok(None);
        };
        let Some(last_index) = rate.frame_index_ceil(content_end)?.checked_sub(1) else {
            return Ok(None);
        };
        let index = rate
            .frame_index_ceil(playhead)?
            .saturating_sub(1)
            .min(last_index);
        Ok(Some(rate.frame_time(index)?))
    }

    pub(crate) fn from_tracks_for_codec(tracks: Vec<TimelineTrack>) -> Self {
        Self {
            tracks,
            markers: Vec::new(),
            sequence_frame_rate: None,
        }
    }

    pub(crate) fn from_parts_for_codec(
        tracks: Vec<TimelineTrack>,
        markers: Vec<TimelineMarker>,
    ) -> Self {
        Self {
            tracks,
            markers,
            sequence_frame_rate: None,
        }
    }

    pub(crate) fn from_parts_with_sequence_rate_for_codec(
        tracks: Vec<TimelineTrack>,
        markers: Vec<TimelineMarker>,
        sequence_frame_rate: Option<RationalRate>,
    ) -> Self {
        Self {
            tracks,
            markers,
            sequence_frame_rate,
        }
    }

    pub(crate) fn set_sequence_frame_rate_for_command(
        &mut self,
        sequence_frame_rate: Option<RationalRate>,
    ) {
        self.sequence_frame_rate = sequence_frame_rate;
    }

    pub(crate) fn validate(&self, media: &[MediaItem]) -> Result<(), TimelineValidationError> {
        self.validate_with_limits(
            media,
            MAX_TIMELINE_TRACKS,
            MAX_TIMELINE_CLIPS,
            MAX_TIMELINE_CLIPS_PER_TRACK,
            MAX_TIMELINE_MARKERS,
        )
    }

    fn validate_with_limits(
        &self,
        media: &[MediaItem],
        max_tracks: usize,
        max_clips: usize,
        max_clips_per_track: usize,
        max_markers: usize,
    ) -> Result<(), TimelineValidationError> {
        if self.tracks.len() > max_tracks {
            return Err(TimelineValidationError);
        }

        if self.markers.len() > max_markers {
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

        let mut marker_ids = HashSet::with_capacity(self.markers.len());
        let mut previous_marker: Option<(RationalTime, MarkerId)> = None;
        for marker in &self.markers {
            if !marker_ids.insert(marker.id)
                || marker.timeline_time.is_negative()
                || marker.label.trim().is_empty()
                || marker.label.len() > MAX_TIMELINE_MARKER_LABEL_BYTES
            {
                return Err(TimelineValidationError);
            }
            if previous_marker.is_some_and(|previous| (marker.timeline_time, marker.id) <= previous)
            {
                return Err(TimelineValidationError);
            }
            previous_marker = Some((marker.timeline_time, marker.id));
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

    pub(crate) fn try_reserve_markers(
        &mut self,
        additional: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.markers.try_reserve(additional)
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

    pub(crate) fn replace_clip_for_command(
        &mut self,
        track_index: usize,
        index: usize,
        clip: TimelineClip,
    ) {
        self.tracks[track_index].clips[index] = clip;
    }

    pub(crate) fn replace_track_clips_for_command(
        &mut self,
        track_index: usize,
        clips: Vec<TimelineClip>,
    ) {
        self.tracks[track_index].clips = clips;
    }

    pub(crate) fn insert_marker_for_command(&mut self, index: usize, marker: TimelineMarker) {
        self.markers.insert(index, marker);
    }

    pub(crate) fn remove_marker_for_command(&mut self, index: usize) -> TimelineMarker {
        self.markers.remove(index)
    }

    pub(crate) fn replace_marker_for_command(&mut self, index: usize, marker: TimelineMarker) {
        self.markers[index] = marker;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceTimingError {
    FrameRateUnavailable,
    Time(TimeError),
}

impl From<TimeError> for SequenceTimingError {
    fn from(error: TimeError) -> Self {
        Self::Time(error)
    }
}

impl fmt::Display for SequenceTimingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameRateUnavailable => {
                formatter.write_str("sequence frame rate is not configured")
            }
            Self::Time(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl Error for SequenceTimingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::FrameRateUnavailable => None,
            Self::Time(error) => Some(error),
        }
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
        ClipId, MAX_TIMELINE_CLIPS, MAX_TIMELINE_CLIPS_PER_TRACK, MAX_TIMELINE_MARKER_LABEL_BYTES,
        MAX_TIMELINE_MARKERS, MAX_TIMELINE_TRACKS, MarkerId, ProjectTimeline, SequenceTimingError,
        TimelineClip, TimelineMarker, TimelineTrack, TrackId, TrackKind,
    };
    use crate::{
        AudioStreamMetadata, MediaId, MediaItem, MediaMetadata, MediaSourceRef,
        MediaStreamMetadata, OtherStreamMetadata, RationalRate, RationalTime, TimeError, TimeRange,
        VideoStreamMetadata,
    };
    use std::{num::NonZeroU32, str::FromStr};

    const TRACK_ID: &str = "22222222-2222-4222-8222-222222222222";
    const CLIP_ID: &str = "33333333-3333-4333-8333-333333333333";
    const MEDIA_ID: &str = "44444444-4444-4444-8444-444444444444";
    const MARKER_A: &str = "11111111-1111-4111-8111-111111111111";
    const MARKER_B: &str = "33333333-3333-4333-8333-333333333333";

    fn time(numerator: i64, denominator: u32) -> RationalTime {
        RationalTime::new(numerator, denominator).unwrap()
    }

    fn marker(id: &str, timeline_time: RationalTime, label: &str) -> TimelineMarker {
        TimelineMarker::from_parts_for_codec(
            MarkerId::from_str(id).unwrap(),
            timeline_time,
            label.to_owned(),
        )
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
                .validate_with_limits(std::slice::from_ref(&media), 3, 4, 4, MAX_TIMELINE_MARKERS)
                .is_ok()
        );
        assert!(
            three_tracks
                .validate_with_limits(std::slice::from_ref(&media), 2, 4, 4, MAX_TIMELINE_MARKERS)
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
                .validate_with_limits(std::slice::from_ref(&media), 3, 3, 3, MAX_TIMELINE_MARKERS)
                .is_ok()
        );
        assert!(
            clips
                .validate_with_limits(std::slice::from_ref(&media), 3, 2, 3, MAX_TIMELINE_MARKERS)
                .is_err()
        );
        assert!(
            clips
                .validate_with_limits(std::slice::from_ref(&media), 3, 3, 2, MAX_TIMELINE_MARKERS)
                .is_err()
        );
    }

    #[test]
    fn marker_ids_and_marker_validation_preserve_global_canonical_invariants() {
        let generated = MarkerId::generate();
        assert_eq!(
            generated.to_string().parse::<MarkerId>().unwrap(),
            generated
        );
        let marker_id = MarkerId::from_str(MARKER_A).unwrap();
        assert_eq!(marker_id.to_string(), MARKER_A);
        assert!(MarkerId::from_str("11111111-1111-4111-8111-111111111112").is_ok());
        assert!(MarkerId::from_str("11111111-1111-4111-8111-11111111111A").is_err());
        assert!(MarkerId::from_str("11111111-1111-3111-8111-111111111111").is_err());

        let valid = ProjectTimeline::from_parts_for_codec(
            vec![],
            vec![
                marker(MARKER_A, time(2, 1), "same"),
                marker(MARKER_B, time(2, 1), "same"),
            ],
        );
        assert!(valid.validate(&[]).is_ok());
        assert!(
            ProjectTimeline::from_parts_for_codec(
                vec![],
                vec![marker(MARKER_A, time(3, 1), "  exact surrounding text  ")],
            )
            .validate(&[])
            .is_ok()
        );
        assert!(
            ProjectTimeline::from_parts_for_codec(
                vec![],
                vec![marker(MARKER_A, time(3, 1), &"é".repeat(128))],
            )
            .validate(&[])
            .is_ok()
        );
        assert!(
            ProjectTimeline::from_parts_for_codec(
                vec![],
                vec![
                    marker(MARKER_B, time(2, 1), "same"),
                    marker(MARKER_A, time(2, 1), "same")
                ],
            )
            .validate(&[])
            .is_err()
        );
        assert!(
            ProjectTimeline::from_parts_for_codec(
                vec![],
                vec![
                    marker(MARKER_A, time(2, 1), "same"),
                    marker(MARKER_A, time(3, 1), "again")
                ],
            )
            .validate(&[])
            .is_err()
        );
        for invalid in [
            marker(MARKER_A, time(-1, 1), "negative"),
            marker(MARKER_A, time(1, 1), "   "),
            marker(
                MARKER_A,
                time(1, 1),
                &"x".repeat(MAX_TIMELINE_MARKER_LABEL_BYTES + 1),
            ),
            marker(MARKER_A, time(1, 1), &"é".repeat(129)),
        ] {
            assert!(
                ProjectTimeline::from_parts_for_codec(vec![], vec![invalid])
                    .validate(&[])
                    .is_err()
            );
        }
    }

    #[test]
    fn marker_count_validation_rejects_the_next_marker_without_truncation() {
        let markers = (0..=MAX_TIMELINE_MARKERS)
            .map(|index| {
                marker(
                    &MarkerId::generate().to_string(),
                    time(index as i64, 1),
                    "bounded",
                )
            })
            .collect();
        let timeline = ProjectTimeline::from_parts_for_codec(vec![], markers);
        assert_eq!(timeline.markers().len(), MAX_TIMELINE_MARKERS + 1);
        assert!(timeline.validate(&[]).is_err());
    }

    #[test]
    fn sequence_lattice_steps_exactly_with_audio_content_end_and_ignores_markers() {
        let video_clip = TimelineClip::from_parts_for_codec(
            ClipId::from_str(CLIP_ID).unwrap(),
            MediaId::from_str(MEDIA_ID).unwrap(),
            time(0, 1),
            TimeRange::new(time(0, 1), time(1, 1)).unwrap(),
        );
        let audio_clip = TimelineClip::from_parts_for_codec(
            ClipId::generate(),
            MediaId::from_str(MEDIA_ID).unwrap(),
            time(1, 1),
            TimeRange::new(time(0, 1), time(1, 1)).unwrap(),
        );
        let mut timeline = ProjectTimeline::from_parts_for_codec(
            vec![
                TimelineTrack::from_parts_for_codec(
                    TrackId::from_str(TRACK_ID).unwrap(),
                    TrackKind::Video,
                    vec![video_clip],
                ),
                TimelineTrack::from_parts_for_codec(
                    TrackId::generate(),
                    TrackKind::Audio,
                    vec![audio_clip],
                ),
            ],
            vec![marker(MARKER_A, time(10, 1), "outside content")],
        );

        assert_eq!(timeline.content_end(), Ok(Some(time(2, 1))));
        assert_eq!(
            timeline.frame_time(0),
            Err(SequenceTimingError::FrameRateUnavailable)
        );
        assert_eq!(
            timeline.next_frame_time(time(0, 1)),
            Err(SequenceTimingError::FrameRateUnavailable)
        );

        timeline.set_sequence_frame_rate_for_command(Some(RationalRate::new(2, 1).unwrap()));
        assert_eq!(timeline.frame_time(0), Ok(Some(time(0, 1))));
        assert_eq!(timeline.frame_time(3), Ok(Some(time(3, 2))));
        assert_eq!(timeline.frame_time(4), Ok(None));
        assert_eq!(timeline.next_frame_time(time(0, 1)), Ok(Some(time(1, 2))));
        assert_eq!(timeline.next_frame_time(time(1, 1)), Ok(Some(time(3, 2))));
        assert_eq!(timeline.next_frame_time(time(2, 1)), Ok(None));
        assert_eq!(
            timeline.previous_frame_time(time(0, 1)),
            Ok(Some(time(0, 1)))
        );
        assert_eq!(
            timeline.previous_frame_time(time(1, 1)),
            Ok(Some(time(1, 2)))
        );
        assert_eq!(
            timeline.previous_frame_time(time(2, 1)),
            Ok(Some(time(3, 2)))
        );
        assert_eq!(
            timeline.previous_frame_time(time(10, 1)),
            Ok(Some(time(3, 2)))
        );
        assert_eq!(
            timeline.next_frame_time(time(-1, 1)),
            Err(SequenceTimingError::Time(TimeError::NegativeTime))
        );
    }

    #[test]
    fn empty_sequence_has_no_frames_even_when_its_rate_is_configured() {
        let mut timeline = ProjectTimeline::default();
        timeline.set_sequence_frame_rate_for_command(Some(RationalRate::new(24, 1).unwrap()));
        assert_eq!(timeline.content_end(), Ok(None));
        assert_eq!(timeline.frame_time(0), Ok(None));
        assert_eq!(timeline.next_frame_time(time(0, 1)), Ok(None));
        assert_eq!(timeline.previous_frame_time(time(0, 1)), Ok(None));
    }
}
