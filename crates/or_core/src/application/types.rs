use crate::{
    ClipContent, ClipId, ClipSettings, MarkerId, MediaId, MediaItem, ProjectDocument, ProjectId,
    ProjectInstanceId, ProjectRevision, RationalRate, RationalTime, SequenceCanvas, TimeRange,
    TimelineClip, TimelineMarker, TimelineTrack, TrackId, TrackKind, TrackState,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, fmt};

use super::timeline::ensure_media_matches_at;
use super::timeline::*;
use super::*;

/// Static discovery information for a command implemented by the core.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CommandDescriptor {
    pub id: &'static str,
    pub schema_version: u64,
    pub mutates_project: bool,
    pub allowed_in_transaction: bool,
}

/// Static discovery information for a query implemented by the core.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct QueryDescriptor {
    pub id: &'static str,
    pub schema_version: u64,
}

pub(super) const COMMANDS: [CommandDescriptor; 30] = [
    CommandDescriptor {
        id: PROJECT_RENAME_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: true,
    },
    CommandDescriptor {
        id: HISTORY_UNDO_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: HISTORY_REDO_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: MEDIA_ADD_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: MEDIA_REMOVE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: MEDIA_RELINK_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_TRACK_ADD_ID,
        schema_version: 2,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_TRACK_REMOVE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_TRACK_MOVE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_INSERT_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_TRACK_SET_STATE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_INSERT_CONTENT_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CAPTIONS_IMPORT_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_UPDATE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_MOVE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIPS_MOVE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_DELETE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_TRIM_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_SLIP_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_ROLL_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_SLIDE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_SPLIT_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_DUCK_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_CLIP_RIPPLE_DELETE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_MARKER_ADD_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_MARKER_MOVE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_MARKER_RENAME_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_MARKER_DELETE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_SEQUENCE_SET_FRAME_RATE_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
    CommandDescriptor {
        id: TIMELINE_SEQUENCE_SET_CANVAS_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
        mutates_project: true,
        allowed_in_transaction: false,
    },
];

pub(super) const QUERIES: [QueryDescriptor; 8] = [
    QueryDescriptor {
        id: PROJECT_SUMMARY_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
    },
    QueryDescriptor {
        id: MEDIA_LIST_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
    },
    QueryDescriptor {
        id: MEDIA_GET_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
    },
    QueryDescriptor {
        id: TIMELINE_TRACKS_ID,
        schema_version: 2,
    },
    QueryDescriptor {
        id: TIMELINE_CLIPS_ID,
        schema_version: 2,
    },
    QueryDescriptor {
        id: TIMELINE_SNAP_ID,
        schema_version: 2,
    },
    QueryDescriptor {
        id: TIMELINE_MARKERS_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
    },
    QueryDescriptor {
        id: TIMELINE_SEQUENCE_SETTINGS_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
    },
];

/// Returns the deterministic catalog of currently supported commands.
pub fn command_catalog() -> &'static [CommandDescriptor] {
    &COMMANDS
}

/// Returns the deterministic catalog of currently supported queries.
pub fn query_catalog() -> &'static [QueryDescriptor] {
    &QUERIES
}

/// A versioned command request with typed project/session/revision preconditions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandEnvelope {
    pub command_id: String,
    pub schema_version: u64,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub expected_project_revision: ProjectRevision,
    pub arguments: Value,
}

impl CommandEnvelope {
    pub fn rename_project(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        name: impl Into<String>,
    ) -> Self {
        Self {
            command_id: PROJECT_RENAME_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "name": name.into() }),
        }
    }

    pub fn add_media(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        item: MediaItem,
    ) -> Self {
        Self {
            command_id: MEDIA_ADD_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "item": item }),
        }
    }

    pub fn remove_media(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        id: MediaId,
    ) -> Self {
        Self {
            command_id: MEDIA_REMOVE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "id": id }),
        }
    }

    pub fn relink_media(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        item: MediaItem,
    ) -> Self {
        Self {
            command_id: MEDIA_RELINK_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "item": item }),
        }
    }

    pub fn add_timeline_track(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        track_id: TrackId,
        kind: TrackKind,
    ) -> Self {
        Self {
            command_id: TIMELINE_TRACK_ADD_ID.to_owned(),
            schema_version: 2,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "track_id": track_id, "kind": kind }),
        }
    }

    pub fn set_timeline_track_state(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        track_id: TrackId,
        state: TrackState,
    ) -> Self {
        Self {
            command_id: TIMELINE_TRACK_SET_STATE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "track_id": track_id, "state": state }),
        }
    }

    pub fn remove_timeline_track(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        track_id: TrackId,
    ) -> Self {
        Self {
            command_id: TIMELINE_TRACK_REMOVE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "track_id": track_id }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_timeline_clip(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
        track_id: TrackId,
        media_id: MediaId,
        timeline_start: RationalTime,
        source_range: TimeRange,
    ) -> Self {
        Self::insert_timeline_clip_with_mode(
            project_id,
            project_instance_id,
            expected_project_revision,
            clip_id,
            track_id,
            media_id,
            timeline_start,
            source_range,
            TimelineEditMode::Insert,
            None,
        )
    }

    /// Inserts a clip with an explicit edit mode.
    ///
    /// The mode is an input, not project state: the same project can be edited in
    /// either mode, and a client's mode choice is its own presentation state.
    /// `overwrite_split_clip_id` is only read by overwrite when the new span lands
    /// strictly inside one existing clip; without it the command refuses.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_timeline_clip_with_mode(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
        track_id: TrackId,
        media_id: MediaId,
        timeline_start: RationalTime,
        source_range: TimeRange,
        mode: TimelineEditMode,
        overwrite_split_clip_id: Option<ClipId>,
    ) -> Self {
        let mut arguments = serde_json::json!({
            "clip_id": clip_id,
            "track_id": track_id,
            "media_id": media_id,
            "timeline_start": timeline_start,
            "source_range": source_range,
            "mode": mode.as_wire(),
        });
        if let Some(split_clip_id) = overwrite_split_clip_id {
            arguments["overwrite_split_clip_id"] = serde_json::json!(split_clip_id);
        }
        Self {
            command_id: TIMELINE_CLIP_INSERT_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_timeline_clip_content(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
        track_id: TrackId,
        timeline_start: RationalTime,
        timeline_duration: RationalTime,
        content: ClipContent,
        settings: ClipSettings,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIP_INSERT_CONTENT_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "clip_id": clip_id,
                "track_id": track_id,
                "timeline_start": timeline_start,
                "timeline_duration": timeline_duration,
                "content": content,
                "settings": settings,
            }),
        }
    }

    /// Imports a complete caption track as one revision-checked, undoable command.
    pub fn import_timeline_captions(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        track_id: TrackId,
        clips: Vec<TimelineClipState>,
    ) -> Self {
        Self {
            command_id: TIMELINE_CAPTIONS_IMPORT_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "track_id": track_id, "clips": clips }),
        }
    }

    pub fn update_timeline_clip(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
        timeline_duration: RationalTime,
        content: ClipContent,
        settings: ClipSettings,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIP_UPDATE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "clip_id": clip_id,
                "timeline_duration": timeline_duration,
                "content": content,
                "settings": settings,
            }),
        }
    }

    pub fn move_timeline_clip(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
        track_id: TrackId,
        timeline_start: RationalTime,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIP_MOVE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "clip_id": clip_id,
                "track_id": track_id,
                "timeline_start": timeline_start,
            }),
        }
    }

    /// Moves every listed clip on one track by the same delta in a single step.
    pub fn move_timeline_clips(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        track_id: TrackId,
        clip_ids: Vec<ClipId>,
        delta: RationalTime,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIPS_MOVE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "track_id": track_id,
                "clip_ids": clip_ids,
                "delta": delta,
            }),
        }
    }

    pub fn delete_timeline_clip(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIP_DELETE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "clip_id": clip_id }),
        }
    }

    pub fn trim_timeline_clip(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
        edge: TimelineTrimEdge,
        timeline_time: RationalTime,
    ) -> Self {
        Self::trim_timeline_clip_with_ripple(
            project_id,
            project_instance_id,
            expected_project_revision,
            clip_id,
            edge,
            timeline_time,
            false,
        )
    }

    /// Trims a clip's edge, optionally pulling every later clip on the track with
    /// it. `ripple` is an explicit input, never project state; an end edge is the
    /// only one ripple accepts, because a ripple-in has two defensible meanings.
    #[allow(clippy::too_many_arguments)]
    pub fn trim_timeline_clip_with_ripple(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
        edge: TimelineTrimEdge,
        timeline_time: RationalTime,
        ripple: bool,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIP_TRIM_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "clip_id": clip_id,
                "edge": edge,
                "timeline_time": timeline_time,
                "ripple": ripple,
            }),
        }
    }

    /// Moves a track to another position in the track list.
    pub fn move_timeline_track(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        track_id: TrackId,
        to_index: usize,
    ) -> Self {
        Self {
            command_id: TIMELINE_TRACK_MOVE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "track_id": track_id,
                "to_index": to_index,
            }),
        }
    }

    /// Moves a clip's source window by `delta` without moving the clip.
    pub fn slip_timeline_clip(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
        delta: RationalTime,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIP_SLIP_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "clip_id": clip_id,
                "delta": delta,
            }),
        }
    }

    /// Moves the boundary between two adjacent clips by `delta`.
    pub fn roll_timeline_clip(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
        delta: RationalTime,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIP_ROLL_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "clip_id": clip_id,
                "delta": delta,
            }),
        }
    }

    /// Moves a clip in time by `delta` and lets its neighbours absorb the move.
    pub fn slide_timeline_clip(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
        delta: RationalTime,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIP_SLIDE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "clip_id": clip_id,
                "delta": delta,
            }),
        }
    }

    pub fn split_timeline_clip(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
        new_clip_id: ClipId,
        timeline_time: RationalTime,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIP_SPLIT_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "clip_id": clip_id,
                "new_clip_id": new_clip_id,
                "timeline_time": timeline_time,
            }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn duck_timeline_clip(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        track_id: TrackId,
        clip_id: ClipId,
        voice_track_id: TrackId,
        duck_millidecibels: i32,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIP_DUCK_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "track_id": track_id,
                "clip_id": clip_id,
                "voice_track_id": voice_track_id,
                "duck_millidecibels": duck_millidecibels,
            }),
        }
    }

    pub fn ripple_delete_timeline_clip(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        clip_id: ClipId,
    ) -> Self {
        Self {
            command_id: TIMELINE_CLIP_RIPPLE_DELETE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "clip_id": clip_id }),
        }
    }

    pub fn add_timeline_marker(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        marker_id: MarkerId,
        timeline_time: RationalTime,
        label: impl Into<String>,
    ) -> Self {
        Self {
            command_id: TIMELINE_MARKER_ADD_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "marker_id": marker_id,
                "timeline_time": timeline_time,
                "label": label.into(),
            }),
        }
    }

    pub fn move_timeline_marker(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        marker_id: MarkerId,
        timeline_time: RationalTime,
    ) -> Self {
        Self {
            command_id: TIMELINE_MARKER_MOVE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "marker_id": marker_id,
                "timeline_time": timeline_time,
            }),
        }
    }

    pub fn rename_timeline_marker(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        marker_id: MarkerId,
        label: impl Into<String>,
    ) -> Self {
        Self {
            command_id: TIMELINE_MARKER_RENAME_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "marker_id": marker_id,
                "label": label.into(),
            }),
        }
    }

    pub fn delete_timeline_marker(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        marker_id: MarkerId,
    ) -> Self {
        Self {
            command_id: TIMELINE_MARKER_DELETE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "marker_id": marker_id }),
        }
    }

    pub fn set_timeline_sequence_frame_rate(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        sequence_frame_rate: Option<RationalRate>,
    ) -> Self {
        Self {
            command_id: TIMELINE_SEQUENCE_SET_FRAME_RATE_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "sequence_frame_rate": sequence_frame_rate }),
        }
    }

    pub fn set_timeline_sequence_canvas(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        sequence_canvas: Option<SequenceCanvas>,
    ) -> Self {
        Self {
            command_id: TIMELINE_SEQUENCE_SET_CANVAS_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "sequence_canvas": sequence_canvas }),
        }
    }

    pub fn undo(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
    ) -> Self {
        history_envelope(
            HISTORY_UNDO_ID,
            project_id,
            project_instance_id,
            expected_project_revision,
        )
    }

    pub fn redo(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
    ) -> Self {
        history_envelope(
            HISTORY_REDO_ID,
            project_id,
            project_instance_id,
            expected_project_revision,
        )
    }
}

fn history_envelope(
    command_id: &str,
    project_id: ProjectId,
    project_instance_id: ProjectInstanceId,
    expected_project_revision: ProjectRevision,
) -> CommandEnvelope {
    CommandEnvelope {
        command_id: command_id.to_owned(),
        schema_version: OPERATION_SCHEMA_VERSION,
        project_id,
        project_instance_id,
        expected_project_revision,
        arguments: serde_json::json!({}),
    }
}

/// A child operation in a grouped transaction. State preconditions live on the transaction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandCall {
    pub command_id: String,
    pub schema_version: u64,
    pub arguments: Value,
}

/// A versioned atomic group of command calls.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionEnvelope {
    pub schema_version: u64,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub expected_project_revision: ProjectRevision,
    pub commands: Vec<CommandCall>,
}

/// A read-only query request for the active project session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryEnvelope {
    pub query_id: String,
    pub schema_version: u64,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub arguments: Value,
}

impl QueryEnvelope {
    pub fn media_get(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        media_id: MediaId,
    ) -> Self {
        Self {
            query_id: MEDIA_GET_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            arguments: serde_json::json!({ "media_id": media_id }),
        }
    }

    pub fn media_list(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        offset: usize,
        limit: usize,
    ) -> Self {
        Self {
            query_id: MEDIA_LIST_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            arguments: serde_json::json!({ "offset": offset, "limit": limit }),
        }
    }

    pub fn timeline_tracks(project_id: ProjectId, project_instance_id: ProjectInstanceId) -> Self {
        Self {
            query_id: TIMELINE_TRACKS_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            arguments: serde_json::json!({}),
        }
    }

    pub fn timeline_tracks_v2(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
    ) -> Self {
        Self {
            query_id: TIMELINE_TRACKS_ID.to_owned(),
            schema_version: 2,
            project_id,
            project_instance_id,
            arguments: serde_json::json!({}),
        }
    }

    pub fn timeline_clips(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        track_id: TrackId,
        offset: usize,
        limit: usize,
    ) -> Self {
        Self {
            query_id: TIMELINE_CLIPS_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            arguments: serde_json::json!({
                "track_id": track_id,
                "offset": offset,
                "limit": limit,
            }),
        }
    }

    pub fn timeline_clips_v2(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        track_id: TrackId,
        offset: usize,
        limit: usize,
    ) -> Self {
        Self {
            query_id: TIMELINE_CLIPS_ID.to_owned(),
            schema_version: 2,
            project_id,
            project_instance_id,
            arguments: serde_json::json!({
                "track_id": track_id,
                "offset": offset,
                "limit": limit,
            }),
        }
    }

    pub fn timeline_snap(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        operation: TimelineSnapOperation,
        clip_id: ClipId,
        target_track_id: Option<TrackId>,
        target_time: RationalTime,
    ) -> Self {
        Self {
            query_id: TIMELINE_SNAP_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            arguments: serde_json::json!({
                "operation": operation,
                "clip_id": clip_id,
                "target_track_id": target_track_id,
                "target_time": target_time,
            }),
        }
    }

    pub fn timeline_snap_v2(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        operation: TimelineSnapOperation,
        clip_id: ClipId,
        target_track_id: Option<TrackId>,
        target_time: RationalTime,
    ) -> Self {
        Self {
            query_id: TIMELINE_SNAP_ID.to_owned(),
            schema_version: 2,
            project_id,
            project_instance_id,
            arguments: serde_json::json!({
                "operation": operation,
                "clip_id": clip_id,
                "target_track_id": target_track_id,
                "target_time": target_time,
            }),
        }
    }

    pub fn timeline_markers(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        offset: usize,
        limit: usize,
    ) -> Self {
        Self {
            query_id: TIMELINE_MARKERS_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            arguments: serde_json::json!({ "offset": offset, "limit": limit }),
        }
    }

    pub fn timeline_sequence_settings(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
    ) -> Self {
        Self {
            query_id: TIMELINE_SEQUENCE_SETTINGS_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            arguments: serde_json::json!({}),
        }
    }
}

/// Serializable application representation of one canonical timeline clip.
///
/// This is deliberately separate from the private `.orproj` codec structures.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineClipState {
    pub clip_id: ClipId,
    pub timeline_start: RationalTime,
    pub timeline_duration: RationalTime,
    pub content: ClipContent,
    pub settings: ClipSettings,
}

impl From<&TimelineClip> for TimelineClipState {
    fn from(clip: &TimelineClip) -> Self {
        Self {
            clip_id: clip.id(),
            timeline_start: clip.timeline_start(),
            timeline_duration: clip.timeline_duration(),
            content: clip.content().clone(),
            settings: clip.settings().clone(),
        }
    }
}

impl TimelineClipState {
    pub fn media_id(&self) -> Option<MediaId> {
        self.content.media_id()
    }

    pub fn source_range(&self) -> Option<TimeRange> {
        self.content.source_range()
    }

    pub(super) fn into_domain(self) -> TimelineClip {
        TimelineClip::from_content_for_command(
            self.clip_id,
            self.timeline_start,
            self.timeline_duration,
            self.content,
            self.settings,
        )
    }
}

/// Schema-v1 media-only timeline page retained for the existing Flutter bridge.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyTimelineClipState {
    pub clip_id: ClipId,
    pub media_id: MediaId,
    pub timeline_start: RationalTime,
    pub source_range: TimeRange,
}

impl LegacyTimelineClipState {
    pub(super) fn from_media_clip(clip: &TimelineClip) -> Option<Self> {
        Some(Self {
            clip_id: clip.id(),
            media_id: clip.media_id()?,
            timeline_start: clip.timeline_start(),
            source_range: clip.source_range()?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineMarkerState {
    pub marker_id: MarkerId,
    pub timeline_time: RationalTime,
    pub label: String,
}

impl From<&TimelineMarker> for TimelineMarkerState {
    fn from(marker: &TimelineMarker) -> Self {
        Self {
            marker_id: marker.id(),
            timeline_time: marker.timeline_time(),
            label: marker.label().to_owned(),
        }
    }
}

impl TimelineMarkerState {
    pub(super) fn into_domain(self) -> TimelineMarker {
        TimelineMarker::from_parts_for_command(self.marker_id, self.timeline_time, self.label)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimelineTrimEdge {
    Start,
    End,
}

/// One canonical project-state delta produced by a command or transaction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum ProjectChange {
    ProjectName {
        before: String,
        after: String,
    },
    TimelineSequenceFrameRateChanged {
        before: Option<RationalRate>,
        after: Option<RationalRate>,
    },
    TimelineSequenceCanvasChanged {
        before: Option<SequenceCanvas>,
        after: Option<SequenceCanvas>,
    },
    MediaAdded {
        item: MediaItem,
        index: usize,
    },
    MediaRemoved {
        item: MediaItem,
        index: usize,
    },
    MediaRelinked {
        before: MediaItem,
        after: MediaItem,
        index: usize,
    },
    TimelineTrackAdded {
        track_id: TrackId,
        track_kind: TrackKind,
        track_state: TrackState,
        index: usize,
    },
    /// One track changed position in the track list.
    TimelineTrackMoved {
        track_id: TrackId,
        from_index: usize,
        to_index: usize,
    },
    TimelineTrackRemoved {
        track_id: TrackId,
        track_kind: TrackKind,
        track_state: TrackState,
        index: usize,
    },
    TimelineCaptionTrackAdded {
        track_id: TrackId,
        index: usize,
        clips: Vec<TimelineClipState>,
    },
    TimelineCaptionTrackRemoved {
        track_id: TrackId,
        index: usize,
        clips: Vec<TimelineClipState>,
    },
    TimelineTrackStateChanged {
        track_id: TrackId,
        before: TrackState,
        after: TrackState,
    },
    TimelineClipInserted {
        track_id: TrackId,
        index: usize,
        clip: TimelineClipState,
    },
    TimelineClipDeleted {
        track_id: TrackId,
        index: usize,
        clip: TimelineClipState,
    },
    TimelineClipMoved {
        from_track_id: TrackId,
        from_index: usize,
        to_track_id: TrackId,
        to_index: usize,
        before: TimelineClipState,
        after: TimelineClipState,
    },
    TimelineClipUpdated {
        track_id: TrackId,
        index: usize,
        before: TimelineClipState,
        after: TimelineClipState,
    },
    TimelineClipTrimmed {
        track_id: TrackId,
        index: usize,
        before: TimelineClipState,
        after: TimelineClipState,
    },
    TimelineClipSplit {
        track_id: TrackId,
        index: usize,
        before: TimelineClipState,
        left_after: TimelineClipState,
        right_after: TimelineClipState,
    },
    TimelineClipRippleDeleted {
        track_id: TrackId,
        index: usize,
        deleted_clip: TimelineClipState,
        shifted_count: usize,
        shift_duration: RationalTime,
    },
    /// One slide moved a clip in time and adjusted the clips either side of it.
    TimelineClipsSlid {
        track_id: TrackId,
        before: Vec<TimelineClipState>,
        after: Vec<TimelineClipState>,
    },
    /// One roll moved the boundary between two adjacent clips on one track.
    TimelineClipsRolled {
        track_id: TrackId,
        before: Vec<TimelineClipState>,
        after: Vec<TimelineClipState>,
    },
    /// One ripple trim changed one clip and shifted every later clip on its track.
    ///
    /// Like `TimelineClipsOverwritten` this carries the whole track because a
    /// history entry is exactly one change and the ripple moves more than one clip.
    TimelineClipsRippleTrimmed {
        track_id: TrackId,
        before: Vec<TimelineClipState>,
        after: Vec<TimelineClipState>,
    },
    /// One overwrite insert replaced a whole range on one track.
    ///
    /// This is the only change an overwrite records, because a history entry must
    /// be exactly one change (`ChangeSet::stage_history_change`) and undo has to
    /// put back removed, trimmed and split clips in a single step. `before` and
    /// `after` are the complete clip lists of the affected track, so the staged
    /// history change is a whole-track replacement in both directions.
    TimelineClipsOverwritten {
        track_id: TrackId,
        before: Vec<TimelineClipState>,
        after: Vec<TimelineClipState>,
    },
    TimelineMarkerAdded {
        marker: TimelineMarkerState,
        index: usize,
    },
    TimelineMarkerDeleted {
        marker: TimelineMarkerState,
        index: usize,
    },
    TimelineMarkerMoved {
        marker_id: MarkerId,
        label: String,
        from_time: RationalTime,
        from_index: usize,
        to_time: RationalTime,
        to_index: usize,
    },
    TimelineMarkerRenamed {
        marker_id: MarkerId,
        time: RationalTime,
        before_label: String,
        after_label: String,
    },
}

/// Net canonical content changes produced by one transaction.
///
/// A ChangeSet is a result description, not a mutation request.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeSet {
    pub(super) changes: Vec<ProjectChange>,
    #[serde(skip)]
    pub(super) ripple_history_guard: Option<RippleHistoryGuard>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RippleHistoryGuard {
    pub(super) before_fingerprint: u64,
    pub(super) after_fingerprint: u64,
}

impl ChangeSet {
    /// Returns the transaction's normalized canonical changes.
    pub fn changes(&self) -> &[ProjectChange] {
        &self.changes
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub(super) fn project_name(before: &str, after: &str) -> Self {
        if before == after {
            Self::default()
        } else {
            Self {
                changes: vec![ProjectChange::ProjectName {
                    before: before.to_owned(),
                    after: after.to_owned(),
                }],
                ripple_history_guard: None,
            }
        }
    }

    pub(super) fn timeline_sequence_frame_rate_changed(
        before: Option<RationalRate>,
        after: Option<RationalRate>,
    ) -> Result<Self, OperationError> {
        if before == after {
            return Ok(Self::default());
        }
        Self::try_single(ProjectChange::TimelineSequenceFrameRateChanged { before, after })
    }

    pub(super) fn timeline_sequence_canvas_changed(
        before: Option<SequenceCanvas>,
        after: Option<SequenceCanvas>,
    ) -> Result<Self, OperationError> {
        if before == after {
            return Ok(Self::default());
        }
        Self::try_single(ProjectChange::TimelineSequenceCanvasChanged { before, after })
    }

    pub(super) fn stage_history_change(
        &self,
        project: &ProjectDocument,
        reverse: bool,
    ) -> Result<(StagedHistoryChange, Self), OperationError> {
        let [change] = self.changes.as_slice() else {
            return Err(OperationError::new(OperationErrorCode::HistoryConflict));
        };

        match change {
            ProjectChange::ProjectName { before, after } => {
                let (expected, target) = if reverse {
                    (after, before)
                } else {
                    (before, after)
                };
                if project.name() != expected {
                    return Err(OperationError::new(OperationErrorCode::HistoryConflict));
                }
                Ok((
                    StagedHistoryChange::Rename(target.clone()),
                    Self::project_name(project.name(), target),
                ))
            }
            ProjectChange::TimelineSequenceFrameRateChanged { before, after } => {
                let (expected, target) = if reverse {
                    (*after, *before)
                } else {
                    (*before, *after)
                };
                let current = project.timeline().sequence_frame_rate();
                if current != expected {
                    return Err(OperationError::new(OperationErrorCode::HistoryConflict));
                }
                Ok((
                    StagedHistoryChange::SetSequenceFrameRate(target),
                    Self::timeline_sequence_frame_rate_changed(current, target)?,
                ))
            }
            ProjectChange::TimelineSequenceCanvasChanged { before, after } => {
                let (expected, target) = if reverse {
                    (*after, *before)
                } else {
                    (*before, *after)
                };
                let current = project.timeline().sequence_canvas();
                if current != expected {
                    return Err(OperationError::new(OperationErrorCode::HistoryConflict));
                }
                Ok((
                    StagedHistoryChange::SetSequenceCanvas(target),
                    Self::timeline_sequence_canvas_changed(current, target)?,
                ))
            }
            ProjectChange::MediaAdded { item, index } => {
                if reverse {
                    ensure_media_matches_at(project, *index, item)?;
                    Ok((
                        StagedHistoryChange::RemoveMedia { index: *index },
                        Self::media_removed(item.clone(), *index),
                    ))
                } else {
                    ensure_media_insertable(project, *index, item)?;
                    Ok((
                        StagedHistoryChange::InsertMedia {
                            item: item.clone(),
                            index: *index,
                        },
                        Self::media_added(item.clone(), *index),
                    ))
                }
            }
            ProjectChange::MediaRemoved { item, index } => {
                if reverse {
                    ensure_media_insertable(project, *index, item)?;
                    Ok((
                        StagedHistoryChange::InsertMedia {
                            item: item.clone(),
                            index: *index,
                        },
                        Self::media_added(item.clone(), *index),
                    ))
                } else {
                    ensure_media_matches_at(project, *index, item)?;
                    Ok((
                        StagedHistoryChange::RemoveMedia { index: *index },
                        Self::media_removed(item.clone(), *index),
                    ))
                }
            }
            ProjectChange::MediaRelinked {
                before,
                after,
                index,
            } => {
                let (expected, target) = if reverse {
                    (after, before)
                } else {
                    (before, after)
                };
                ensure_media_matches_at(project, *index, expected)?;
                if project
                    .media_items()
                    .iter()
                    .enumerate()
                    .any(|(candidate_index, item)| {
                        candidate_index != *index && item.source() == target.source()
                    })
                    || project
                        .timeline()
                        .validate_replacing_media(project.media_items(), target)
                        .is_err()
                {
                    return Err(OperationError::new(OperationErrorCode::HistoryConflict));
                }
                Ok((
                    StagedHistoryChange::ReplaceMedia {
                        item: target.clone(),
                        index: *index,
                    },
                    Self::media_relinked(expected.clone(), target.clone(), *index)?,
                ))
            }
            ProjectChange::TimelineTrackAdded {
                track_id,
                track_kind,
                track_state,
                index,
            } => stage_track_history_change(
                project,
                *track_id,
                *track_kind,
                *track_state,
                *index,
                reverse,
                true,
            ),
            ProjectChange::TimelineTrackMoved {
                track_id,
                from_index,
                to_index,
            } => {
                // A move is its own inverse, so the effect applied depends only on
                // the direction: undoing it moves the track back from where the
                // change left it. The change pushed onto the opposite stack is the
                // pair as recorded, because after either direction the track sits at
                // the position that pair's next application starts from.
                let index =
                    find_timeline_track_index(project, *track_id).ok_or_else(history_conflict)?;
                let (expected_start, staged_from, staged_to) = if reverse {
                    (*to_index, *to_index, *from_index)
                } else {
                    (*from_index, *from_index, *to_index)
                };
                if index != expected_start {
                    return Err(history_conflict());
                }
                Ok((
                    StagedHistoryChange::MoveTimelineTrack {
                        from_index: staged_from,
                        to_index: staged_to,
                    },
                    Self::try_single(ProjectChange::TimelineTrackMoved {
                        track_id: *track_id,
                        from_index: *from_index,
                        to_index: *to_index,
                    })?,
                ))
            }
            ProjectChange::TimelineTrackRemoved {
                track_id,
                track_kind,
                track_state,
                index,
            } => stage_track_history_change(
                project,
                *track_id,
                *track_kind,
                *track_state,
                *index,
                reverse,
                false,
            ),
            ProjectChange::TimelineCaptionTrackAdded {
                track_id,
                index,
                clips,
            } => {
                stage_caption_track_history_change(project, *track_id, *index, clips, reverse, true)
            }
            ProjectChange::TimelineCaptionTrackRemoved {
                track_id,
                index,
                clips,
            } => stage_caption_track_history_change(
                project, *track_id, *index, clips, reverse, false,
            ),
            ProjectChange::TimelineTrackStateChanged {
                track_id,
                before,
                after,
            } => stage_track_state_history_change(project, *track_id, *before, *after, reverse),
            ProjectChange::TimelineClipInserted {
                track_id,
                index,
                clip,
            } => stage_clip_history_change(project, *track_id, *index, clip, reverse, true),
            ProjectChange::TimelineClipDeleted {
                track_id,
                index,
                clip,
            } => stage_clip_history_change(project, *track_id, *index, clip, reverse, false),
            ProjectChange::TimelineClipMoved {
                from_track_id,
                from_index,
                to_track_id,
                to_index,
                before,
                after,
            } => stage_clip_move_history_change(
                project,
                (*from_track_id, *from_index),
                (*to_track_id, *to_index),
                before,
                after,
                reverse,
            ),
            ProjectChange::TimelineClipUpdated {
                track_id,
                index,
                before,
                after,
            } => {
                stage_clip_update_history_change(project, *track_id, *index, before, after, reverse)
            }
            ProjectChange::TimelineClipTrimmed {
                track_id,
                index,
                before,
                after,
            } => stage_clip_trim_history_change(project, *track_id, *index, before, after, reverse),
            ProjectChange::TimelineClipSplit {
                track_id,
                index,
                before,
                left_after,
                right_after,
            } => stage_clip_split_history_change(
                project,
                *track_id,
                *index,
                before,
                left_after,
                right_after,
                reverse,
            ),
            ProjectChange::TimelineClipsSlid {
                track_id,
                before,
                after,
            }
            | ProjectChange::TimelineClipsRolled {
                track_id,
                before,
                after,
            }
            | ProjectChange::TimelineClipsRippleTrimmed {
                track_id,
                before,
                after,
            }
            | ProjectChange::TimelineClipsOverwritten {
                track_id,
                before,
                after,
            } => {
                let track_index =
                    find_timeline_track_index(project, *track_id).ok_or_else(history_conflict)?;
                let (expected, target) = if reverse {
                    (after, before)
                } else {
                    (before, after)
                };
                let current = track_states(project, track_index)?;
                if current != *expected {
                    return Err(history_conflict());
                }
                let clips = states_into_domain(target.clone())?;
                validate_track_states(
                    project,
                    project.timeline().tracks()[track_index].kind(),
                    target,
                )
                .map_err(|_| history_conflict())?;
                Ok((
                    StagedHistoryChange::ReplaceTimelineTrack { track_index, clips },
                    Self::try_single(ProjectChange::TimelineClipsOverwritten {
                        track_id: *track_id,
                        before: target.clone(),
                        after: expected.clone(),
                    })?,
                ))
            }
            ProjectChange::TimelineClipRippleDeleted {
                track_id,
                index,
                deleted_clip,
                shifted_count,
                shift_duration,
            } => stage_clip_ripple_history_change(
                project,
                *track_id,
                *index,
                deleted_clip,
                *shifted_count,
                *shift_duration,
                reverse,
                self.ripple_history_guard,
            ),
            ProjectChange::TimelineMarkerAdded { marker, index } => {
                stage_marker_history_change(project, marker, *index, reverse, true)
            }
            ProjectChange::TimelineMarkerDeleted { marker, index } => {
                stage_marker_history_change(project, marker, *index, reverse, false)
            }
            ProjectChange::TimelineMarkerMoved {
                marker_id,
                label,
                from_time,
                from_index,
                to_time,
                to_index,
            } => stage_marker_move_history_change(
                project,
                *marker_id,
                label,
                (*from_time, *from_index),
                (*to_time, *to_index),
                reverse,
            ),
            ProjectChange::TimelineMarkerRenamed {
                marker_id,
                time,
                before_label,
                after_label,
            } => stage_marker_rename_history_change(
                project,
                *marker_id,
                *time,
                before_label,
                after_label,
                reverse,
            ),
        }
    }

    pub(super) fn try_single(change: ProjectChange) -> Result<Self, OperationError> {
        let mut changes = Vec::new();
        changes
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        changes.push(change);
        Ok(Self {
            changes,
            ripple_history_guard: None,
        })
    }

    pub(super) fn media_added(item: MediaItem, index: usize) -> Self {
        Self {
            changes: vec![ProjectChange::MediaAdded { item, index }],
            ripple_history_guard: None,
        }
    }

    pub(super) fn media_removed(item: MediaItem, index: usize) -> Self {
        Self {
            changes: vec![ProjectChange::MediaRemoved { item, index }],
            ripple_history_guard: None,
        }
    }

    pub(super) fn media_relinked(
        before: MediaItem,
        after: MediaItem,
        index: usize,
    ) -> Result<Self, OperationError> {
        if before.id() != after.id() {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        Self::try_single(ProjectChange::MediaRelinked {
            before,
            after,
            index,
        })
    }
}

pub(super) enum StagedHistoryChange {
    Rename(String),
    SetSequenceFrameRate(Option<RationalRate>),
    SetSequenceCanvas(Option<SequenceCanvas>),
    InsertMedia {
        item: MediaItem,
        index: usize,
    },
    RemoveMedia {
        index: usize,
    },
    ReplaceMedia {
        item: MediaItem,
        index: usize,
    },
    InsertTimelineTrack {
        track_id: TrackId,
        kind: TrackKind,
        state: TrackState,
        index: usize,
    },
    InsertCaptionTrack {
        track: TimelineTrack,
        index: usize,
    },
    RemoveTimelineTrack {
        index: usize,
    },
    /// Tracks keep their contents and their ids; only the order changes.
    MoveTimelineTrack {
        from_index: usize,
        to_index: usize,
    },
    SetTimelineTrackState {
        track_index: usize,
        state: TrackState,
    },
    InsertTimelineClip {
        track_index: usize,
        index: usize,
        clip: TimelineClipState,
    },
    RemoveTimelineClip {
        track_index: usize,
        index: usize,
    },
    MoveTimelineClip {
        from_track_index: usize,
        from_index: usize,
        to_track_index: usize,
        to_index: usize,
        to_timeline_start: RationalTime,
    },
    ReplaceTimelineClip {
        track_index: usize,
        index: usize,
        clip: TimelineClipState,
    },
    ReplaceTimelineTrack {
        track_index: usize,
        clips: Vec<TimelineClip>,
    },
    InsertTimelineMarker {
        index: usize,
        marker: TimelineMarkerState,
    },
    RemoveTimelineMarker {
        index: usize,
    },
    MoveTimelineMarker {
        from_index: usize,
        to_index: usize,
        to_time: RationalTime,
    },
    RenameTimelineMarker {
        index: usize,
        label: String,
    },
}

/// Result of applying one command to a project session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandResult {
    pub command_id: String,
    pub schema_version: u64,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub before_revision: ProjectRevision,
    pub after_revision: ProjectRevision,
    pub changed: bool,
    pub change_set: ChangeSet,
}

/// Result of applying one transaction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionResult {
    pub schema_version: u64,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub before_revision: ProjectRevision,
    pub after_revision: ProjectRevision,
    pub changed: bool,
    pub command_count: usize,
    pub change_set: ChangeSet,
}

/// Canonical and runtime identity values returned by project.summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectSummary {
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub project_revision: ProjectRevision,
    pub name: String,
}

/// Result of a project query. The summary is a read-only snapshot of current state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryResult {
    pub query_id: String,
    pub schema_version: u64,
    #[serde(flatten)]
    pub summary: ProjectSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_page: Option<MediaListPage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_item: Option<Box<MediaItem>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_tracks: Option<Vec<TimelineTrackSummary>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_tracks_v2: Option<Vec<TimelineTrackSummaryV2>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_clip_page: Option<Box<TimelineClipPage>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_clip_page_v2: Option<Box<TimelineClipPageV2>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_snap: Option<Box<TimelineSnapResult>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_marker_page: Option<Box<TimelineMarkerPage>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_sequence_settings: Option<Box<TimelineSequenceSettings>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineSequenceSettings {
    pub sequence_frame_rate: Option<RationalRate>,
    pub sequence_canvas: Option<SequenceCanvas>,
}

/// One bounded, insertion-ordered page from the persistent media library.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaListPage {
    pub items: Vec<MediaItem>,
    pub total_count: usize,
    pub offset: usize,
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
}

/// One track in the bounded `timeline.tracks` query result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineTrackSummary {
    pub track_id: TrackId,
    pub kind: TrackKind,
    pub clip_count: usize,
}

/// One bounded page from a track's clips in canonical timeline order.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineClipPage {
    pub track_id: TrackId,
    pub items: Vec<LegacyTimelineClipState>,
    pub total_count: usize,
    pub offset: usize,
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineTrackSummaryV2 {
    pub track_id: TrackId,
    pub kind: TrackKind,
    pub state: TrackState,
    pub clip_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineClipPageV2 {
    pub track_id: TrackId,
    pub items: Vec<TimelineClipState>,
    pub total_count: usize,
    pub offset: usize,
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineMarkerPage {
    pub items: Vec<TimelineMarkerState>,
    pub total_count: usize,
    pub offset: usize,
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineSnapOperation {
    Move,
    TrimStart,
    TrimEnd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineSnapMovingAnchor {
    None,
    Start,
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineSnapTargetKind {
    None,
    TimelineZero,
    ClipStart,
    ClipEnd,
    Marker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineSnapResult {
    pub raw_target_time: RationalTime,
    pub resolved_target_time: RationalTime,
    pub snapped: bool,
    pub moving_anchor: TimelineSnapMovingAnchor,
    pub target_kind: TimelineSnapTargetKind,
    pub target_time: RationalTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_track_id: Option<TrackId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_clip_id: Option<ClipId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_marker_id: Option<MarkerId>,
}

/// Stable machine-readable operation failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OperationErrorCode {
    UnknownCommand,
    UnsupportedCommandSchema,
    UnknownQuery,
    UnsupportedQuerySchema,
    UnsupportedTransactionSchema,
    EmptyTransaction,
    CommandNotAllowedInTransaction,
    ProjectIdMismatch,
    ProjectInstanceMismatch,
    RevisionConflict,
    InvalidArguments,
    RevisionOverflow,
    NothingToUndo,
    NothingToRedo,
    HistoryConflict,
    HistoryStorageFailure,
    MediaIdAlreadyExists,
    MediaSourceAlreadyExists,
    MediaNotFound,
    MediaInUse,
    TimelineTrackIdAlreadyExists,
    TimelineTrackNotFound,
    TimelineTrackNotEmpty,
    TimelineTrackLocked,
    TimelineClipIdAlreadyExists,
    TimelineClipNotFound,
    TimelineMediaIncompatible,
    TimelineOverlap,
    /// An overwrite would have to split a clip that contains the new span, which
    /// needs the caller's fragment id exactly as `timeline.clip.split` does.
    TimelineOverwriteSplitRequiresClipId,
    TimelineLimitExceeded,
    TimelineMarkerIdAlreadyExists,
    TimelineMarkerNotFound,
    /// No voice clip on the selected track overlaps the music clip.
    TimelineNothingToDuck,
}

/// A safe structured operation error with a stable code and optional context.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationError {
    pub code: OperationErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<ProjectRevision>,
}

impl OperationError {
    pub(crate) fn new(code: OperationErrorCode) -> Self {
        Self {
            code,
            current_revision: None,
        }
    }

    pub(super) fn revision_conflict(current_revision: ProjectRevision) -> Self {
        Self {
            code: OperationErrorCode::RevisionConflict,
            current_revision: Some(current_revision),
        }
    }
}

impl fmt::Display for OperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.code {
            OperationErrorCode::UnknownCommand => "unknown command",
            OperationErrorCode::UnsupportedCommandSchema => "unsupported command schema",
            OperationErrorCode::UnknownQuery => "unknown query",
            OperationErrorCode::UnsupportedQuerySchema => "unsupported query schema",
            OperationErrorCode::UnsupportedTransactionSchema => "unsupported transaction schema",
            OperationErrorCode::EmptyTransaction => "transaction contains no commands",
            OperationErrorCode::CommandNotAllowedInTransaction => {
                "command is not allowed in a transaction"
            }
            OperationErrorCode::ProjectIdMismatch => "command or query targets another project",
            OperationErrorCode::ProjectInstanceMismatch => {
                "command or query targets another project instance"
            }
            OperationErrorCode::RevisionConflict => "expected project revision is stale",
            OperationErrorCode::InvalidArguments => "operation arguments are invalid",
            OperationErrorCode::RevisionOverflow => "project revision cannot be incremented",
            OperationErrorCode::NothingToUndo => "there is no project change to undo",
            OperationErrorCode::NothingToRedo => "there is no project change to redo",
            OperationErrorCode::HistoryConflict => {
                "project state does not match the history change"
            }
            OperationErrorCode::HistoryStorageFailure => {
                "session history could not reserve storage"
            }
            OperationErrorCode::MediaIdAlreadyExists => "media ID already exists in the project",
            OperationErrorCode::MediaSourceAlreadyExists => {
                "media source already exists in the project"
            }
            OperationErrorCode::MediaNotFound => "media item was not found in the project",
            OperationErrorCode::MediaInUse => {
                "media item is referenced by one or more timeline clips"
            }
            OperationErrorCode::TimelineTrackIdAlreadyExists => {
                "timeline track ID already exists in the project"
            }
            OperationErrorCode::TimelineTrackNotFound => "timeline track was not found",
            OperationErrorCode::TimelineTrackNotEmpty => {
                "timeline track must be empty before it can be removed"
            }
            OperationErrorCode::TimelineTrackLocked => "timeline track is locked for editing",
            OperationErrorCode::TimelineClipIdAlreadyExists => {
                "timeline clip ID already exists in the project"
            }
            OperationErrorCode::TimelineClipNotFound => "timeline clip was not found",
            OperationErrorCode::TimelineNothingToDuck => "no voice clip overlaps the music clip",
            OperationErrorCode::TimelineMediaIncompatible => {
                "media has no stream compatible with the timeline track"
            }
            OperationErrorCode::TimelineOverlap => {
                "timeline clip overlaps another clip on the same track"
            }
            OperationErrorCode::TimelineOverwriteSplitRequiresClipId => {
                "an overwrite inside an existing clip needs overwrite_split_clip_id"
            }
            OperationErrorCode::TimelineLimitExceeded => {
                "timeline operation exceeds a configured limit"
            }
            OperationErrorCode::TimelineMarkerIdAlreadyExists => {
                "timeline marker ID already exists in the project"
            }
            OperationErrorCode::TimelineMarkerNotFound => "timeline marker was not found",
        };

        formatter.write_str(message)?;
        if let Some(revision) = self.current_revision {
            write!(formatter, " (current revision: {revision})")?;
        }
        Ok(())
    }
}

impl Error for OperationError {}

/// A transport-independent operation accepted by an active project session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "request",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ApplicationRequest {
    Command(CommandEnvelope),
    Query(QueryEnvelope),
    Transaction(TransactionEnvelope),
    Export(ExportRequest),
    MotionMaterialize(MotionMaterializeRequest),
    Transcription(TranscriptionRequest),
}

/// A bounded request to turn a code file or an existing scene into a video.
///
/// Materialisation writes a persistent file and never changes the project; the
/// output is registered through the ordinary media and timeline commands. It
/// lives on the application contract because only the application runtime links
/// the FFmpeg encoder.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "request",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MotionMaterializeRequest {
    FromCode {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        source_path: String,
        destination: String,
        frame_rate_numerator: u32,
        frame_rate_denominator: u32,
        width: u32,
        height: u32,
        #[serde(default)]
        title: String,
    },
    FromScene {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        scene_path: String,
        destination: String,
        frame_rate_numerator: u32,
        frame_rate_denominator: u32,
        width: u32,
        height: u32,
    },
}

/// Runtime-only export command routed through the same application and IPC path as edits.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "request",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ExportRequest {
    Start {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        destination: String,
        #[serde(default)]
        profile: ExportProfile,
        /// How far a lossy profile may trade encoding time for quality.
        #[serde(default)]
        quality: ExportQuality,
        /// Tallest delivery resolution; `None` keeps the project canvas or the
        /// natural source size. Values are rounded down to an even dimension.
        #[serde(default)]
        short_side: Option<u32>,
    },
    Status {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        job_id: crate::JobId,
    },
    Cancel {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        job_id: crate::JobId,
    },
}

/// Container and codec combination used for a rendered video export.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportProfile {
    /// Large, lossless Matroska video with PCM audio.
    #[default]
    MatroskaFfv1PcmS16le,
    /// Smaller WebM delivery output using VP9 video and Opus audio.
    WebmVp9Opus,
    /// Broadly compatible MP4 output using H.264 video and AAC audio.
    Mp4H264Aac,
}

/// What a lossy export may spend on quality.
///
/// A lossless profile ignores this: FFV1 has no rate control to trade. A lossy
/// profile spends more encoding time and more bytes for a cleaner picture in
/// `Quality`, and the fewest bytes per second in `Speed`. `Quality` is the
/// default because an export is usually the last thing a project does, and the
/// local host encodes far faster than it renders.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportQuality {
    /// Spend the encoding time: slower encode, cleaner picture, larger file.
    #[default]
    Quality,
    /// Spend the least time: fastest encode, smaller file, visible artefacts.
    Speed,
}
