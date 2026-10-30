//! Timeline structure commands: tracks, clips, text clips and markers.
//!
//! These are the bridge's timeline edit surface. They all funnel through
//! `timeline_command`, so the identity, revision and dispatch rules stay in
//! one place.

use super::project::*;
use or_core::{
    ClipContent, ClipId, ClipSettings, CommandEnvelope, MarkerId, MediaId, RationalTime, TimeRange,
    TimelineTrimEdge, TrackId, TrackKind, TrackState, VisualSettings,
};
use std::str::FromStr;

impl ProjectHostHandle {
    pub fn add_timeline_track(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        kind: TimelineTrackKindView,
    ) -> ProjectActionResult {
        let track_id = TrackId::generate();
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::add_timeline_track(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                    match kind {
                        TimelineTrackKindView::Video => TrackKind::Video,
                        TimelineTrackKindView::Audio => TrackKind::Audio,
                        TimelineTrackKindView::Text => TrackKind::Text,
                        TimelineTrackKindView::Caption => TrackKind::Caption,
                    },
                )
            },
        )
    }

    pub fn remove_timeline_track(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::remove_timeline_track(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                )
            },
        )
    }

    pub fn set_timeline_track_state(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        state: ProjectTimelineTrackStateView,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        let state = TrackState::new(state.locked, state.visible, state.muted, state.solo);
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::set_timeline_track_state(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                    state,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        media_id: String,
        timeline_start_numerator: i64,
        timeline_start_denominator: u32,
        source_start_numerator: i64,
        source_start_denominator: u32,
        duration_numerator: i64,
        duration_denominator: u32,
        // `insert` (the default) or `overwrite`: an explicit command input, never
        // canonical project state.
        mode: Option<String>,
        // Only read by overwrite when the new span lands strictly inside one
        // existing clip, mirroring `timeline.clip.split`'s `new_clip_id`.
        overwrite_split_clip_id: Option<String>,
    ) -> ProjectActionResult {
        let edit_mode = match mode.as_deref() {
            None => or_core::TimelineEditMode::Insert,
            Some(value) => match or_core::TimelineEditMode::parse_wire(value) {
                Some(edit_mode) => edit_mode,
                None => return action_error(timeline_arguments_error()),
            },
        };
        // The bridge owns id generation for this command, exactly as it does for
        // the inserted clip itself. A client may name the fragment; when it does
        // not, one is generated here so an overwrite that lands inside an existing
        // clip completes instead of refusing for want of an id no user could type.
        let split_clip_id = match overwrite_split_clip_id {
            None => None,
            Some(value) => match ClipId::from_str(&value) {
                Ok(clip_id) => Some(clip_id),
                Err(_) => return action_error(timeline_arguments_error()),
            },
        };
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        let media_id = match MediaId::from_str(&media_id) {
            Ok(media_id) => media_id,
            Err(error) => return invalid_timeline_id("INVALID_MEDIA_ID", error.to_string()),
        };
        let timeline_start =
            match RationalTime::new(timeline_start_numerator, timeline_start_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        let source_start = match RationalTime::new(source_start_numerator, source_start_denominator)
        {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        let duration = match RationalTime::new(duration_numerator, duration_denominator) {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        let source_range = match TimeRange::new(source_start, duration) {
            Ok(range) => range,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::insert_timeline_clip_with_mode(
                    project_id,
                    project_instance_id,
                    revision,
                    ClipId::generate(),
                    track_id,
                    media_id,
                    timeline_start,
                    source_range,
                    edit_mode,
                    if edit_mode == or_core::TimelineEditMode::Overwrite {
                        Some(split_clip_id.unwrap_or_else(ClipId::generate))
                    } else {
                        None
                    },
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_timeline_text_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        content_kind: TimelineClipContentKindView,
        timeline_start_numerator: i64,
        timeline_start_denominator: u32,
        timeline_duration_numerator: i64,
        timeline_duration_denominator: u32,
        text: String,
        formatting: ProjectTextFormattingView,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        let timeline_start =
            match RationalTime::new(timeline_start_numerator, timeline_start_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        let timeline_duration =
            match RationalTime::new(timeline_duration_numerator, timeline_duration_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        let content = match text_clip_content(content_kind, text, formatting) {
            Ok(content) => content,
            Err(error) => return action_error(error),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::insert_timeline_clip_content(
                    project_id,
                    project_instance_id,
                    revision,
                    ClipId::generate(),
                    track_id,
                    timeline_start,
                    timeline_duration,
                    content,
                    ClipSettings::Visual(VisualSettings::default()),
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_timeline_text_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        content_kind: TimelineClipContentKindView,
        timeline_duration_numerator: i64,
        timeline_duration_denominator: u32,
        text: String,
        formatting: ProjectTextFormattingView,
    ) -> ProjectActionResult {
        let (_, _, _, track_kind, clip) = match self.find_timeline_clip_state(
            &project_id,
            &project_instance_id,
            expected_revision,
            &track_id,
            &clip_id,
        ) {
            Ok(found) => found,
            Err(error) => return action_error(error),
        };
        let content = match text_clip_content(content_kind, text, formatting) {
            Ok(content) => content,
            Err(error) => return action_error(error),
        };
        let matching_content = matches!(
            (&clip.content, content_kind),
            (ClipContent::Text { .. }, TimelineClipContentKindView::Text)
                | (
                    ClipContent::Caption { .. },
                    TimelineClipContentKindView::Caption
                )
        );
        let matching_track = matches!(
            (track_kind, content_kind),
            (TrackKind::Text, TimelineClipContentKindView::Text)
                | (TrackKind::Caption, TimelineClipContentKindView::Caption)
        );
        if !matching_content || !matching_track {
            return action_error(ProjectBridgeError {
                code: "TIMELINE_CLIP_CONTENT_MISMATCH".to_owned(),
                message: "The selected clip is not the requested text or caption type.".to_owned(),
            });
        }
        let timeline_duration =
            match RationalTime::new(timeline_duration_numerator, timeline_duration_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::update_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    timeline_duration,
                    content,
                    clip.settings,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn move_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        track_id: String,
        timeline_start_numerator: i64,
        timeline_start_denominator: u32,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        let timeline_start =
            match RationalTime::new(timeline_start_numerator, timeline_start_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::move_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    track_id,
                    timeline_start,
                )
            },
        )
    }

    /// Moves every listed clip on one track by the same delta, as one undo step.
    ///
    /// A group drag is one gesture: five separate moves would cost five undos
    /// and could stop half-applied, so the whole selection travels in a single
    /// command whose change carries the track before and after.
    #[allow(clippy::too_many_arguments)]
    pub fn move_timeline_clips(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_ids: Vec<String>,
        delta_numerator: i64,
        delta_denominator: u32,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        if clip_ids.is_empty() {
            return action_error(ProjectBridgeError {
                code: "INVALID_CLIP_IDS".to_owned(),
                message: "Select at least one clip to move.".to_owned(),
            });
        }
        let mut parsed = Vec::with_capacity(clip_ids.len());
        for clip_id in &clip_ids {
            match ClipId::from_str(clip_id) {
                Ok(clip_id) => parsed.push(clip_id),
                Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
            }
        }
        let delta = match RationalTime::new(delta_numerator, delta_denominator) {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::move_timeline_clips(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                    parsed,
                    delta,
                )
            },
        )
    }

    pub fn delete_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::delete_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trim_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        edge: TimelineTrimEdgeView,
        timeline_time_numerator: i64,
        timeline_time_denominator: u32,
        // Opt-in end-edge ripple; absent keeps the plain trim meaning exactly.
        ripple: Option<bool>,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let timeline_time =
            match RationalTime::new(timeline_time_numerator, timeline_time_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::trim_timeline_clip_with_ripple(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    match edge {
                        TimelineTrimEdgeView::Start => TimelineTrimEdge::Start,
                        TimelineTrimEdgeView::End => TimelineTrimEdge::End,
                    },
                    timeline_time,
                    ripple.unwrap_or(false),
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    /// Moves a track to another position in the track list.
    pub fn move_timeline_track(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        to_index: u32,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::move_timeline_track(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                    to_index as usize,
                )
            },
        )
    }

    /// Moves a clip's source window by the given delta without moving the clip.
    pub fn slip_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        delta_numerator: i64,
        delta_denominator: u32,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let delta = match RationalTime::new(delta_numerator, delta_denominator) {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::slip_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    delta,
                )
            },
        )
    }

    /// Moves the boundary between two adjacent clips by the given delta.
    pub fn roll_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        delta_numerator: i64,
        delta_denominator: u32,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let delta = match RationalTime::new(delta_numerator, delta_denominator) {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::roll_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    delta,
                )
            },
        )
    }

    /// Moves a clip in time by the given delta, letting its neighbours absorb it.
    pub fn slide_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        delta_numerator: i64,
        delta_denominator: u32,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let delta = match RationalTime::new(delta_numerator, delta_denominator) {
            Ok(time) => time,
            Err(_) => return action_error(timeline_arguments_error()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::slide_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    delta,
                )
            },
        )
    }

    pub fn split_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
        timeline_time_numerator: i64,
        timeline_time_denominator: u32,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let timeline_time =
            match RationalTime::new(timeline_time_numerator, timeline_time_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        let new_clip_id = ClipId::generate();
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::split_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                    new_clip_id,
                    timeline_time,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn duck_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        voice_track_id: String,
        duck_millidecibels: i32,
    ) -> ProjectActionResult {
        let track_id = match TrackId::from_str(&track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        let voice_track_id = match TrackId::from_str(&voice_track_id) {
            Ok(track_id) => track_id,
            Err(error) => return invalid_timeline_id("INVALID_TRACK_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::duck_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    track_id,
                    clip_id,
                    voice_track_id,
                    duck_millidecibels,
                )
            },
        )
    }

    pub fn ripple_delete_timeline_clip(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        clip_id: String,
    ) -> ProjectActionResult {
        let clip_id = match ClipId::from_str(&clip_id) {
            Ok(clip_id) => clip_id,
            Err(error) => return invalid_timeline_id("INVALID_CLIP_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::ripple_delete_timeline_clip(
                    project_id,
                    project_instance_id,
                    revision,
                    clip_id,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_timeline_marker(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        timeline_time_numerator: i64,
        timeline_time_denominator: u32,
        label: String,
    ) -> ProjectActionResult {
        let timeline_time =
            match RationalTime::new(timeline_time_numerator, timeline_time_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::add_timeline_marker(
                    project_id,
                    project_instance_id,
                    revision,
                    MarkerId::generate(),
                    timeline_time,
                    label,
                )
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn move_timeline_marker(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        marker_id: String,
        timeline_time_numerator: i64,
        timeline_time_denominator: u32,
    ) -> ProjectActionResult {
        let marker_id = match MarkerId::from_str(&marker_id) {
            Ok(marker_id) => marker_id,
            Err(error) => return invalid_timeline_id("INVALID_MARKER_ID", error.to_string()),
        };
        let timeline_time =
            match RationalTime::new(timeline_time_numerator, timeline_time_denominator) {
                Ok(time) => time,
                Err(_) => return action_error(timeline_arguments_error()),
            };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::move_timeline_marker(
                    project_id,
                    project_instance_id,
                    revision,
                    marker_id,
                    timeline_time,
                )
            },
        )
    }

    pub fn rename_timeline_marker(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        marker_id: String,
        label: String,
    ) -> ProjectActionResult {
        let marker_id = match MarkerId::from_str(&marker_id) {
            Ok(marker_id) => marker_id,
            Err(error) => return invalid_timeline_id("INVALID_MARKER_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::rename_timeline_marker(
                    project_id,
                    project_instance_id,
                    revision,
                    marker_id,
                    label,
                )
            },
        )
    }

    pub fn delete_timeline_marker(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        marker_id: String,
    ) -> ProjectActionResult {
        let marker_id = match MarkerId::from_str(&marker_id) {
            Ok(marker_id) => marker_id,
            Err(error) => return invalid_timeline_id("INVALID_MARKER_ID", error.to_string()),
        };
        self.timeline_command(
            project_id,
            project_instance_id,
            expected_revision,
            |project_id, project_instance_id, revision| {
                CommandEnvelope::delete_timeline_marker(
                    project_id,
                    project_instance_id,
                    revision,
                    marker_id,
                )
            },
        )
    }
}
