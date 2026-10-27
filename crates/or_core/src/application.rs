use crate::{
    ClipContent, ClipId, ClipSettings, MAX_TIMELINE_MARKER_LABEL_BYTES, MAX_TIMELINE_MARKERS,
    MarkerId, MediaId, MediaItem, ProjectDocument, ProjectId, ProjectInstanceId, ProjectRevision,
    RationalRate, RationalTime, TimeRange, TimelineClip, TimelineMarker, TimelineTrack, TrackId,
    TrackKind, TrackState,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{cmp::Ordering, collections::HashSet, error::Error, fmt};

const PROJECT_RENAME_ID: &str = "project.rename";
const PROJECT_SUMMARY_ID: &str = "project.summary";
const HISTORY_UNDO_ID: &str = "history.undo";
const HISTORY_REDO_ID: &str = "history.redo";
const MEDIA_ADD_ID: &str = "media.add";
const MEDIA_REMOVE_ID: &str = "media.remove";
const MEDIA_RELINK_ID: &str = "media.relink";
const MEDIA_LIST_ID: &str = "media.list";
const MEDIA_GET_ID: &str = "media.get";
const TIMELINE_TRACK_ADD_ID: &str = "timeline.track.add";
const TIMELINE_TRACK_REMOVE_ID: &str = "timeline.track.remove";
const TIMELINE_TRACK_SET_STATE_ID: &str = "timeline.track.set_state";
const TIMELINE_TRACK_MOVE_ID: &str = "timeline.track.move";
const TIMELINE_CLIP_INSERT_ID: &str = "timeline.clip.insert";
const TIMELINE_CLIP_INSERT_CONTENT_ID: &str = "timeline.clip.insert_content";
const TIMELINE_CAPTIONS_IMPORT_ID: &str = "timeline.captions.import";
const TIMELINE_CLIP_MOVE_ID: &str = "timeline.clip.move";
const TIMELINE_CLIPS_MOVE_ID: &str = "timeline.clips.move";
const TIMELINE_CLIP_UPDATE_ID: &str = "timeline.clip.update";
const TIMELINE_CLIP_DELETE_ID: &str = "timeline.clip.delete";
const TIMELINE_CLIP_TRIM_ID: &str = "timeline.clip.trim";
const TIMELINE_CLIP_SPLIT_ID: &str = "timeline.clip.split";
const TIMELINE_CLIP_SLIP_ID: &str = "timeline.clip.slip";
const TIMELINE_CLIP_ROLL_ID: &str = "timeline.clip.roll";
const TIMELINE_CLIP_SLIDE_ID: &str = "timeline.clip.slide";
const TIMELINE_CLIP_RIPPLE_DELETE_ID: &str = "timeline.clip.ripple_delete";
const TIMELINE_MARKER_ADD_ID: &str = "timeline.marker.add";
const TIMELINE_MARKER_MOVE_ID: &str = "timeline.marker.move";
const TIMELINE_MARKER_RENAME_ID: &str = "timeline.marker.rename";
const TIMELINE_MARKER_DELETE_ID: &str = "timeline.marker.delete";
const TIMELINE_TRACKS_ID: &str = "timeline.tracks";
const TIMELINE_CLIPS_ID: &str = "timeline.clips";
const TIMELINE_SNAP_ID: &str = "timeline.snap";
const TIMELINE_MARKERS_ID: &str = "timeline.markers";
const TIMELINE_SEQUENCE_SET_FRAME_RATE_ID: &str = "timeline.sequence.set_frame_rate";
const TIMELINE_SEQUENCE_SETTINGS_ID: &str = "timeline.sequence.settings";
const OPERATION_SCHEMA_VERSION: u64 = 1;
pub const MAX_MEDIA_PAGE_SIZE: usize = 100;
pub const MAX_TIMELINE_CLIP_PAGE_SIZE: usize = 100;
pub const MAX_TIMELINE_MARKER_PAGE_SIZE: usize = 100;
pub const CURRENT_TRANSACTION_SCHEMA_VERSION: u64 = 1;

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

const COMMANDS: [CommandDescriptor; 28] = [
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
];

const QUERIES: [QueryDescriptor; 8] = [
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

    fn into_domain(self) -> TimelineClip {
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
    fn from_media_clip(clip: &TimelineClip) -> Option<Self> {
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
    fn into_domain(self) -> TimelineMarker {
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
    changes: Vec<ProjectChange>,
    #[serde(skip)]
    ripple_history_guard: Option<RippleHistoryGuard>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RippleHistoryGuard {
    before_fingerprint: u64,
    after_fingerprint: u64,
}

impl ChangeSet {
    /// Returns the transaction's normalized canonical changes.
    pub fn changes(&self) -> &[ProjectChange] {
        &self.changes
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    fn project_name(before: &str, after: &str) -> Self {
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

    fn timeline_sequence_frame_rate_changed(
        before: Option<RationalRate>,
        after: Option<RationalRate>,
    ) -> Result<Self, OperationError> {
        if before == after {
            return Ok(Self::default());
        }
        Self::try_single(ProjectChange::TimelineSequenceFrameRateChanged { before, after })
    }

    fn stage_history_change(
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

    fn try_single(change: ProjectChange) -> Result<Self, OperationError> {
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

    fn media_added(item: MediaItem, index: usize) -> Self {
        Self {
            changes: vec![ProjectChange::MediaAdded { item, index }],
            ripple_history_guard: None,
        }
    }

    fn media_removed(item: MediaItem, index: usize) -> Self {
        Self {
            changes: vec![ProjectChange::MediaRemoved { item, index }],
            ripple_history_guard: None,
        }
    }

    fn media_relinked(
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

enum StagedHistoryChange {
    Rename(String),
    SetSequenceFrameRate(Option<RationalRate>),
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

    fn revision_conflict(current_revision: ProjectRevision) -> Self {
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

#[cfg(test)]
mod export_profile_tests {
    use super::{ExportProfile, ExportRequest};
    use crate::{ProjectId, ProjectInstanceId};

    #[test]
    fn legacy_export_start_defaults_to_the_lossless_profile() {
        let request: ExportRequest = serde_json::from_value(serde_json::json!({
            "operation": "start",
            "request": {
                "project_id": ProjectId::generate(),
                "project_instance_id": ProjectInstanceId::generate(),
                "expected_project_revision": 0,
                "destination": "/tmp/legacy.mkv"
            }
        }))
        .unwrap();

        assert!(matches!(
            request,
            ExportRequest::Start {
                profile: ExportProfile::MatroskaFfv1PcmS16le,
                ..
            }
        ));
    }

    #[test]
    fn motion_materialize_round_trips_on_the_application_contract() {
        use super::{
            ApplicationRequest, ApplicationResponse, MotionMaterializeRequest,
            MotionMaterializeResponse, RationalTime,
        };
        let project_id = ProjectId::generate();
        let project_instance_id = ProjectInstanceId::generate();
        let request = ApplicationRequest::MotionMaterialize(MotionMaterializeRequest::FromCode {
            project_id,
            project_instance_id,
            source_path: "/tmp/sample.rs".to_owned(),
            destination: "/tmp/out.mkv".to_owned(),
            frame_rate_numerator: 12,
            frame_rate_denominator: 1,
            width: 640,
            height: 360,
            title: "Fixture".to_owned(),
        });
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["kind"], "motion_materialize");
        assert_eq!(value["request"]["operation"], "from_code");
        let decoded: ApplicationRequest = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, request);

        let response = ApplicationResponse::MotionMaterialize(MotionMaterializeResponse {
            succeeded: true,
            error_code: String::new(),
            message: "Rendered.".to_owned(),
            output_path: "/tmp/out.mkv".to_owned(),
            scene_id: "scene".to_owned(),
            scene_name: "Fixture".to_owned(),
            width: 640,
            height: 360,
            frame_rate_numerator: 12,
            frame_rate_denominator: 1,
            frame_count: 24,
            duration: RationalTime::new(2, 1).unwrap(),
            bytes_written: 1024,
        });
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["kind"], "motion_materialize");
        let decoded: ApplicationResponse = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, response);
    }

    #[test]
    fn transcription_round_trips_on_the_application_contract() {
        use super::{
            ApplicationRequest, ApplicationResponse, TranscriptionJobView,
            TranscriptionProposalView, TranscriptionRequest, TranscriptionResponse,
            TranscriptionSegmentView,
        };
        use crate::{MediaId, ProjectRevision};
        let project_id = ProjectId::generate();
        let project_instance_id = ProjectInstanceId::generate();
        let media_id = MediaId::generate();
        let job_id = crate::JobId::generate();
        let request = ApplicationRequest::Transcription(TranscriptionRequest::Start {
            project_id,
            project_instance_id,
            expected_project_revision: ProjectRevision::new(3),
            media_id,
        });
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["kind"], "transcription");
        assert_eq!(value["request"]["operation"], "start");
        let decoded: ApplicationRequest = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, request);

        let response = ApplicationResponse::Transcription(TranscriptionResponse::Proposal(
            TranscriptionProposalView {
                project_id,
                project_revision: ProjectRevision::new(3),
                media_id,
                provider_id: "whisper.cpp".to_owned(),
                model_id: "whisper-small-multilingual".to_owned(),
                model_version: "1.0".to_owned(),
                model_license: "MIT".to_owned(),
                language: "vi".to_owned(),
                segments: vec![TranscriptionSegmentView {
                    start_ms: 0,
                    end_ms: 1200,
                    text: "Xin chào".to_owned(),
                }],
            },
        ));
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["kind"], "transcription");
        assert_eq!(value["result"]["status"], "proposal");
        let decoded: ApplicationResponse = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, response);

        let job =
            ApplicationResponse::Transcription(TranscriptionResponse::Job(TranscriptionJobView {
                job_id,
                state: crate::JobState::Running,
                failure_message: None,
            }));
        assert_eq!(
            serde_json::to_value(&job).unwrap()["result"]["body"]["state"],
            "running"
        );
    }

    #[test]
    fn webm_profile_has_a_stable_wire_name() {
        assert_eq!(
            serde_json::to_value(ExportProfile::WebmVp9Opus).unwrap(),
            serde_json::Value::String("webm_vp9_opus".to_owned())
        );
    }
}

/// Result for an export request. Export work never changes the project revision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportResponse {
    pub succeeded: bool,
    pub error_code: String,
    pub message: String,
    pub job: Option<crate::JobSnapshot>,
}

impl ExportResponse {
    pub fn success(message: impl Into<String>, job: crate::JobSnapshot) -> Self {
        Self {
            succeeded: true,
            error_code: String::new(),
            message: message.into(),
            job: Some(job),
        }
    }

    pub fn failure(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            succeeded: false,
            error_code: code.into(),
            message: message.into(),
            job: None,
        }
    }
}

/// The typed result of dispatching an [`ApplicationRequest`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "result",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ApplicationResponse {
    Command(CommandResult),
    Query(QueryResult),
    Transaction(TransactionResult),
    Export(ExportResponse),
    MotionMaterialize(MotionMaterializeResponse),
    Transcription(TranscriptionResponse),
    Error(OperationError),
}

/// Result of materialising a MotionScene. It never changes the project revision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionMaterializeResponse {
    pub succeeded: bool,
    pub error_code: String,
    pub message: String,
    pub output_path: String,
    pub scene_id: String,
    pub scene_name: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate_numerator: u32,
    pub frame_rate_denominator: u32,
    pub frame_count: u64,
    pub duration: RationalTime,
    pub bytes_written: u64,
}

impl MotionMaterializeResponse {
    pub fn failure(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            succeeded: false,
            error_code: code.into(),
            message: message.into(),
            output_path: String::new(),
            scene_id: String::new(),
            scene_name: String::new(),
            width: 0,
            height: 0,
            frame_rate_numerator: 0,
            frame_rate_denominator: 0,
            frame_count: 0,
            duration: RationalTime::ZERO,
            bytes_written: 0,
        }
    }
}

/// A runtime-only local transcription request routed through the same
/// application and IPC path as edits.
///
/// Only the application runtime links the media decoder and the local model, so
/// the CLI and agents ask the host to transcribe instead of linking a second
/// pipeline. A transcription never changes the project; the reviewed cues are
/// applied later through the ordinary caption import command.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "request",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TranscriptionRequest {
    Start {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        media_id: MediaId,
    },
    Status {
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        job_id: crate::JobId,
    },
    Proposal {
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

/// One background transcription job as the host reports it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptionJobView {
    pub job_id: crate::JobId,
    pub state: crate::JobState,
    pub failure_message: Option<String>,
}

/// One reviewed transcript cue. Confidence is intentionally not part of this
/// transport contract: the UI shows it from its own bounded view, and no
/// command consumes it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptionSegmentView {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// A provider-independent transcript proposal bound to the exact project,
/// revision and media the host asked about.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptionProposalView {
    pub project_id: ProjectId,
    pub project_revision: ProjectRevision,
    pub media_id: MediaId,
    pub provider_id: String,
    pub model_id: String,
    pub model_version: String,
    pub model_license: String,
    pub language: String,
    pub segments: Vec<TranscriptionSegmentView>,
}

/// Outcome of one transcription request. It never carries executable edits.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "body",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TranscriptionResponse {
    Job(TranscriptionJobView),
    Proposal(TranscriptionProposalView),
    Failure(TranscriptionFailure),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptionFailure {
    pub code: String,
    pub message: String,
}

impl TranscriptionFailure {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SessionHistory {
    undo: Vec<ChangeSet>,
    redo: Vec<ChangeSet>,
}

#[derive(Clone, Copy)]
enum HistoryDirection {
    Undo,
    Redo,
}

/// One loaded runtime instance of a canonical project document.
///
/// The instance ID and transaction history are session-only and never part of the
/// persisted document.
#[derive(Debug, Eq, PartialEq)]
pub struct ProjectSession {
    project: ProjectDocument,
    project_instance_id: ProjectInstanceId,
    history: SessionHistory,
}

impl ProjectSession {
    /// Opens a document into a new runtime instance without changing its state.
    pub fn open(project: ProjectDocument) -> Self {
        Self {
            project,
            project_instance_id: ProjectInstanceId::generate(),
            history: SessionHistory::default(),
        }
    }

    pub const fn project_id(&self) -> ProjectId {
        self.project.id()
    }

    pub const fn project_instance_id(&self) -> ProjectInstanceId {
        self.project_instance_id
    }

    pub const fn project_revision(&self) -> ProjectRevision {
        self.project.revision()
    }

    pub const fn project(&self) -> &ProjectDocument {
        &self.project
    }

    /// Dispatches a supported application operation through the existing semantic paths.
    pub fn handle_application_request(
        &mut self,
        request: ApplicationRequest,
    ) -> ApplicationResponse {
        match request {
            ApplicationRequest::Command(envelope) => self
                .execute_command(envelope)
                .map(ApplicationResponse::Command)
                .unwrap_or_else(ApplicationResponse::Error),
            ApplicationRequest::Query(envelope) => self
                .execute_query(envelope)
                .map(ApplicationResponse::Query)
                .unwrap_or_else(ApplicationResponse::Error),
            ApplicationRequest::Transaction(envelope) => self
                .execute_transaction(envelope)
                .map(ApplicationResponse::Transaction)
                .unwrap_or_else(ApplicationResponse::Error),
            ApplicationRequest::Export(_) => ApplicationResponse::Export(ExportResponse::failure(
                "EXPORT_RUNTIME_UNAVAILABLE",
                "the project host has no export runtime",
            )),
            ApplicationRequest::MotionMaterialize(_) => {
                ApplicationResponse::MotionMaterialize(MotionMaterializeResponse::failure(
                    "MOTION_RUNTIME_UNAVAILABLE",
                    "the project host has no motion materialisation runtime",
                ))
            }
            ApplicationRequest::Transcription(_) => ApplicationResponse::Transcription(
                TranscriptionResponse::Failure(TranscriptionFailure::new(
                    "TRANSCRIPTION_RUNTIME_UNAVAILABLE",
                    "the project host has no transcription runtime",
                )),
            ),
        }
    }

    pub fn into_project(self) -> ProjectDocument {
        self.project
    }

    /// Dispatches a versioned command after validating its project/session state.
    pub fn execute_command(
        &mut self,
        envelope: CommandEnvelope,
    ) -> Result<CommandResult, OperationError> {
        let descriptor = COMMANDS
            .iter()
            .find(|descriptor| descriptor.id == envelope.command_id.as_str())
            .ok_or_else(|| OperationError::new(OperationErrorCode::UnknownCommand))?;
        let schema_supported = if descriptor.id == TIMELINE_TRACK_ADD_ID {
            matches!(envelope.schema_version, 1 | 2)
        } else {
            envelope.schema_version == descriptor.schema_version
        };
        if !schema_supported {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedCommandSchema,
            ));
        }
        self.check_project_preconditions(
            envelope.project_id,
            envelope.project_instance_id,
            envelope.expected_project_revision,
        )?;

        let command_id = envelope.command_id;
        let schema_version = envelope.schema_version;
        let before_revision = self.project_revision();
        let change_set = match command_id.as_str() {
            PROJECT_RENAME_ID => {
                let call = CommandCall {
                    command_id: command_id.clone(),
                    schema_version,
                    arguments: envelope.arguments,
                };
                self.apply_forward_commands(std::slice::from_ref(&call))?.1
            }
            HISTORY_UNDO_ID => {
                parse_empty_arguments(envelope.arguments)?;
                self.apply_history(HistoryDirection::Undo)?.2
            }
            HISTORY_REDO_ID => {
                parse_empty_arguments(envelope.arguments)?;
                self.apply_history(HistoryDirection::Redo)?.2
            }
            MEDIA_ADD_ID => self.apply_media_add(envelope.arguments)?,
            MEDIA_REMOVE_ID => self.apply_media_remove(envelope.arguments)?,
            MEDIA_RELINK_ID => self.apply_media_relink(envelope.arguments)?,
            TIMELINE_TRACK_ADD_ID => {
                self.apply_timeline_track_add(envelope.arguments, schema_version)?
            }
            TIMELINE_TRACK_REMOVE_ID => self.apply_timeline_track_remove(envelope.arguments)?,
            TIMELINE_TRACK_SET_STATE_ID => {
                self.apply_timeline_track_set_state(envelope.arguments)?
            }
            TIMELINE_TRACK_MOVE_ID => self.apply_timeline_track_move(envelope.arguments)?,
            TIMELINE_CLIP_INSERT_ID => self.apply_timeline_clip_insert(envelope.arguments)?,
            TIMELINE_CLIP_INSERT_CONTENT_ID => {
                self.apply_timeline_clip_insert_content(envelope.arguments)?
            }
            TIMELINE_CAPTIONS_IMPORT_ID => {
                self.apply_timeline_captions_import(envelope.arguments)?
            }
            TIMELINE_CLIP_MOVE_ID => self.apply_timeline_clip_move(envelope.arguments)?,
            TIMELINE_CLIPS_MOVE_ID => self.apply_timeline_clips_move(envelope.arguments)?,
            TIMELINE_CLIP_UPDATE_ID => self.apply_timeline_clip_update(envelope.arguments)?,
            TIMELINE_CLIP_DELETE_ID => self.apply_timeline_clip_delete(envelope.arguments)?,
            TIMELINE_CLIP_TRIM_ID => self.apply_timeline_clip_trim(envelope.arguments)?,
            TIMELINE_CLIP_SPLIT_ID => self.apply_timeline_clip_split(envelope.arguments)?,
            TIMELINE_CLIP_SLIP_ID => self.apply_timeline_clip_slip(envelope.arguments)?,
            TIMELINE_CLIP_ROLL_ID => self.apply_timeline_clip_roll(envelope.arguments)?,
            TIMELINE_CLIP_SLIDE_ID => self.apply_timeline_clip_slide(envelope.arguments)?,
            TIMELINE_CLIP_RIPPLE_DELETE_ID => {
                self.apply_timeline_clip_ripple_delete(envelope.arguments)?
            }
            TIMELINE_MARKER_ADD_ID => self.apply_timeline_marker_add(envelope.arguments)?,
            TIMELINE_MARKER_MOVE_ID => self.apply_timeline_marker_move(envelope.arguments)?,
            TIMELINE_MARKER_RENAME_ID => self.apply_timeline_marker_rename(envelope.arguments)?,
            TIMELINE_MARKER_DELETE_ID => self.apply_timeline_marker_delete(envelope.arguments)?,
            TIMELINE_SEQUENCE_SET_FRAME_RATE_ID => {
                self.apply_timeline_sequence_set_frame_rate(envelope.arguments)?
            }
            _ => return Err(OperationError::new(OperationErrorCode::UnknownCommand)),
        };

        let after_revision = self.project_revision();
        Ok(CommandResult {
            command_id,
            schema_version,
            project_id: self.project_id(),
            project_instance_id: self.project_instance_id,
            before_revision,
            after_revision,
            changed: !change_set.is_empty(),
            change_set,
        })
    }

    /// Evaluates a command group in staged state and commits its net change atomically.
    pub fn execute_transaction(
        &mut self,
        envelope: TransactionEnvelope,
    ) -> Result<TransactionResult, OperationError> {
        if envelope.schema_version != CURRENT_TRANSACTION_SCHEMA_VERSION {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedTransactionSchema,
            ));
        }
        self.check_project_preconditions(
            envelope.project_id,
            envelope.project_instance_id,
            envelope.expected_project_revision,
        )?;
        if envelope.commands.is_empty() {
            return Err(OperationError::new(OperationErrorCode::EmptyTransaction));
        }

        let before_revision = self.project_revision();
        let command_count = envelope.commands.len();
        let (after_revision, change_set) = self.apply_forward_commands(&envelope.commands)?;
        Ok(TransactionResult {
            schema_version: CURRENT_TRANSACTION_SCHEMA_VERSION,
            project_id: self.project_id(),
            project_instance_id: self.project_instance_id,
            before_revision,
            after_revision,
            changed: !change_set.is_empty(),
            command_count,
            change_set,
        })
    }

    /// Dispatches and executes a read-only query against current session state.
    pub fn execute_query(&self, envelope: QueryEnvelope) -> Result<QueryResult, OperationError> {
        let descriptor = QUERIES
            .iter()
            .find(|descriptor| descriptor.id == envelope.query_id)
            .ok_or_else(|| OperationError::new(OperationErrorCode::UnknownQuery))?;
        let schema_supported = if matches!(
            envelope.query_id.as_str(),
            TIMELINE_SNAP_ID | TIMELINE_TRACKS_ID | TIMELINE_CLIPS_ID
        ) {
            matches!(envelope.schema_version, 1 | 2)
        } else {
            envelope.schema_version == descriptor.schema_version
        };
        if !schema_supported {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedQuerySchema,
            ));
        }
        if envelope.project_id != self.project_id() {
            return Err(OperationError::new(OperationErrorCode::ProjectIdMismatch));
        }
        if envelope.project_instance_id != self.project_instance_id {
            return Err(OperationError::new(
                OperationErrorCode::ProjectInstanceMismatch,
            ));
        }
        let mut result = QueryResult {
            query_id: envelope.query_id,
            schema_version: envelope.schema_version,
            summary: ProjectSummary {
                project_id: self.project_id(),
                project_instance_id: self.project_instance_id,
                project_revision: self.project_revision(),
                name: self.project.name().to_owned(),
            },
            media_page: None,
            media_item: None,
            timeline_tracks: None,
            timeline_tracks_v2: None,
            timeline_clip_page: None,
            timeline_clip_page_v2: None,
            timeline_snap: None,
            timeline_marker_page: None,
            timeline_sequence_settings: None,
        };

        if result.query_id == PROJECT_SUMMARY_ID {
            if !is_empty_object(&envelope.arguments) {
                return Err(OperationError::new(OperationErrorCode::InvalidArguments));
            }
        } else if result.query_id == MEDIA_LIST_ID {
            result.media_page = Some(self.media_list(envelope.arguments)?);
        } else if result.query_id == MEDIA_GET_ID {
            result.media_item = Some(Box::new(self.media_get(envelope.arguments)?));
        } else if result.query_id == TIMELINE_TRACKS_ID {
            if envelope.schema_version == 2 {
                result.timeline_tracks_v2 = Some(self.timeline_tracks_v2(envelope.arguments)?);
            } else {
                result.timeline_tracks = Some(self.timeline_tracks(envelope.arguments)?);
            }
        } else if result.query_id == TIMELINE_CLIPS_ID {
            if envelope.schema_version == 2 {
                result.timeline_clip_page_v2 =
                    Some(Box::new(self.timeline_clips_v2(envelope.arguments)?));
            } else {
                result.timeline_clip_page =
                    Some(Box::new(self.timeline_clips(envelope.arguments)?));
            }
        } else if result.query_id == TIMELINE_SNAP_ID {
            result.timeline_snap = Some(Box::new(
                self.timeline_snap(envelope.arguments, envelope.schema_version == 2)?,
            ));
        } else if result.query_id == TIMELINE_MARKERS_ID {
            result.timeline_marker_page =
                Some(Box::new(self.timeline_markers(envelope.arguments)?));
        } else if result.query_id == TIMELINE_SEQUENCE_SETTINGS_ID {
            if !is_empty_object(&envelope.arguments) {
                return Err(OperationError::new(OperationErrorCode::InvalidArguments));
            }
            result.timeline_sequence_settings = Some(Box::new(TimelineSequenceSettings {
                sequence_frame_rate: self.project.timeline().sequence_frame_rate(),
            }));
        } else {
            return Err(OperationError::new(OperationErrorCode::UnknownQuery));
        }

        Ok(result)
    }

    fn apply_media_add(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: MediaAddArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if arguments.item.metadata().validate().is_err() {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        ensure_media_id_available(&self.project, arguments.item.id())?;
        ensure_media_source_available(&self.project, arguments.item.source())?;

        let before_revision = self.project_revision();
        let after_revision = before_revision
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        self.project
            .try_reserve_media_items(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        let index = self.project.media_items().len();
        let change_set = ChangeSet::media_added(arguments.item.clone(), index);
        // All validation, revision checks, and required storage allocations are complete.
        self.project
            .insert_media_for_command(arguments.item, index, after_revision);
        self.history.undo.push(change_set.clone());
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_media_remove(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: MediaRemoveArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some(index) = self
            .project
            .media_items()
            .iter()
            .position(|item| item.id() == arguments.id)
        else {
            return Err(OperationError::new(OperationErrorCode::MediaNotFound));
        };
        if self.project.timeline().references_media(arguments.id) {
            return Err(OperationError::new(OperationErrorCode::MediaInUse));
        }

        let before_revision = self.project_revision();
        let after_revision = before_revision
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        let item = self.project.media_items()[index].clone();
        let change_set = ChangeSet::media_removed(item, index);
        // All validation, revision checks, and history allocation are complete.
        self.project.remove_media_for_command(index, after_revision);
        self.history.undo.push(change_set.clone());
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_media_relink(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: MediaRelinkArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if arguments.item.metadata().validate().is_err() {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let Some(index) = self
            .project
            .media_items()
            .iter()
            .position(|item| item.id() == arguments.item.id())
        else {
            return Err(OperationError::new(OperationErrorCode::MediaNotFound));
        };
        let before = &self.project.media_items()[index];
        if before == &arguments.item {
            return Ok(ChangeSet::default());
        }
        if self
            .project
            .media_items()
            .iter()
            .enumerate()
            .any(|(candidate_index, item)| {
                candidate_index != index && item.source() == arguments.item.source()
            })
        {
            return Err(OperationError::new(
                OperationErrorCode::MediaSourceAlreadyExists,
            ));
        }
        self.project
            .timeline()
            .validate_replacing_media(self.project.media_items(), &arguments.item)
            .map_err(|_| OperationError::new(OperationErrorCode::TimelineMediaIncompatible))?;

        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        let change_set = ChangeSet::media_relinked(before.clone(), arguments.item.clone(), index)?;
        self.project
            .replace_media_for_command(index, arguments.item, after_revision);
        self.history.undo.push(change_set.clone());
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_captions_import(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineCaptionsImportArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if arguments.clips.is_empty()
            || arguments.clips.len() > crate::subtitle_io::MAX_IMPORTED_CAPTIONS
        {
            return Err(OperationError::new(
                OperationErrorCode::TimelineLimitExceeded,
            ));
        }
        let tracks = self.project.timeline().tracks();
        if tracks.iter().any(|track| track.id() == arguments.track_id) {
            return Err(OperationError::new(
                OperationErrorCode::TimelineTrackIdAlreadyExists,
            ));
        }
        if tracks.len() >= crate::MAX_TIMELINE_TRACKS {
            return Err(OperationError::new(
                OperationErrorCode::TimelineLimitExceeded,
            ));
        }
        let current_clip_count = timeline_clip_count(&self.project)
            .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineLimitExceeded))?;
        if current_clip_count
            .checked_add(arguments.clips.len())
            .is_none_or(|count| count > crate::MAX_TIMELINE_CLIPS)
        {
            return Err(OperationError::new(
                OperationErrorCode::TimelineLimitExceeded,
            ));
        }

        let mut clip_ids = HashSet::new();
        clip_ids
            .try_reserve(arguments.clips.len())
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        for clip in &arguments.clips {
            if find_timeline_clip(&self.project, clip.clip_id).is_some()
                || !clip_ids.insert(clip.clip_id)
            {
                return Err(OperationError::new(
                    OperationErrorCode::TimelineClipIdAlreadyExists,
                ));
            }
        }
        validate_track_states(&self.project, TrackKind::Caption, &arguments.clips)?;

        let index = tracks.len();
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineCaptionTrackAdded {
                track_id: arguments.track_id,
                index,
                clips: arguments.clips.clone(),
            })?;
        self.project
            .try_reserve_timeline_tracks(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        let mut domain_clips = Vec::new();
        domain_clips
            .try_reserve(arguments.clips.len())
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        domain_clips.extend(
            arguments
                .clips
                .into_iter()
                .map(TimelineClipState::into_domain),
        );
        let track = TimelineTrack::from_parts_for_codec(
            arguments.track_id,
            TrackKind::Caption,
            domain_clips,
        );
        self.project
            .insert_timeline_track_for_command(index, track, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_track_add(
        &mut self,
        arguments: Value,
        schema_version: u64,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineTrackAddArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if schema_version == 1 && matches!(arguments.kind, TrackKind::Text | TrackKind::Caption) {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedCommandSchema,
            ));
        }
        let tracks = self.project.timeline().tracks();
        if tracks.iter().any(|track| track.id() == arguments.track_id) {
            return Err(OperationError::new(
                OperationErrorCode::TimelineTrackIdAlreadyExists,
            ));
        }
        if tracks.len() >= crate::MAX_TIMELINE_TRACKS {
            return Err(OperationError::new(
                OperationErrorCode::TimelineLimitExceeded,
            ));
        }

        let index = tracks.len();
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineTrackAdded {
                track_id: arguments.track_id,
                track_kind: arguments.kind,
                track_state: TrackState::DEFAULT,
                index,
            })?;
        self.project
            .try_reserve_timeline_tracks(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        let track = TimelineTrack::empty_for_command(arguments.track_id, arguments.kind);
        self.project
            .insert_timeline_track_for_command(index, track, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_track_remove(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineTrackRemoveArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some(index) = self
            .project
            .timeline()
            .tracks()
            .iter()
            .position(|track| track.id() == arguments.track_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineTrackNotFound,
            ));
        };
        let track = &self.project.timeline().tracks()[index];
        if !track.clips().is_empty() {
            return Err(OperationError::new(
                OperationErrorCode::TimelineTrackNotEmpty,
            ));
        }
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let kind = track.kind();
        let state = track.state();
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineTrackRemoved {
                track_id: arguments.track_id,
                track_kind: kind,
                track_state: state,
                index,
            })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project
            .remove_timeline_track_for_command(index, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_track_set_state(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineTrackSetStateArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let track_index = find_timeline_track_index(&self.project, arguments.track_id)
            .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineTrackNotFound))?;
        let before = self.project.timeline().tracks()[track_index].state();
        if before == arguments.state {
            return Ok(ChangeSet::default());
        }
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineTrackStateChanged {
                track_id: arguments.track_id,
                before,
                after: arguments.state,
            })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project.set_timeline_track_state_for_command(
            track_index,
            arguments.state,
            after_revision,
        );
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_clip_insert(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipInsertArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let track_id = arguments.track_id;
        let track_index = find_timeline_track_index(&self.project, track_id)
            .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineTrackNotFound))?;
        let track = &self.project.timeline().tracks()[track_index];
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let mode = arguments.mode;
        let split_clip_id = arguments.overwrite_split_clip_id;
        let clip = arguments.into_clip_state(track.kind())?;
        if mode.is_insert() {
            self.insert_timeline_clip_state(track_id, track_index, clip)
        } else {
            self.overwrite_timeline_clip_state(track_id, track_index, clip, split_clip_id)
        }
    }

    fn apply_timeline_clip_insert_content(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipInsertContentArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let track_id = arguments.track_id;
        let track_index = find_timeline_track_index(&self.project, track_id)
            .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineTrackNotFound))?;
        if self.project.timeline().tracks()[track_index]
            .state()
            .locked()
        {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let clip = arguments.into_clip_state()?;
        self.insert_timeline_clip_state(track_id, track_index, clip)
    }

    /// Inserts a clip and clears the range it occupies, in one transaction.
    ///
    /// A clip the new span fully covers is removed; a clip the span crosses at its
    /// head is shortened; a clip the span crosses at its tail is moved and its
    /// source advanced; a clip the span lands strictly inside is split into a head
    /// fragment (which keeps the original id) and a tail fragment that takes the
    /// caller's id. The whole range is one history entry, so undo restores exactly
    /// what the insert replaced.
    fn overwrite_timeline_clip_state(
        &mut self,
        track_id: TrackId,
        track_index: usize,
        clip: TimelineClipState,
        split_clip_id: Option<ClipId>,
    ) -> Result<ChangeSet, OperationError> {
        if find_timeline_clip(&self.project, clip.clip_id).is_some() {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipIdAlreadyExists,
            ));
        }
        if split_clip_id
            .is_some_and(|id| id == clip.clip_id || find_timeline_clip(&self.project, id).is_some())
        {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipIdAlreadyExists,
            ));
        }
        let track_kind = self.project.timeline().tracks()[track_index].kind();
        // A containment split adds one clip on top of the insert itself.
        ensure_timeline_clip_capacity(
            &self.project,
            track_index,
            2,
            2,
            crate::MAX_TIMELINE_CLIPS,
            crate::MAX_TIMELINE_CLIPS_PER_TRACK,
        )?;
        let new_start = clip.timeline_start;
        let new_end = timeline_clip_end(&clip)?;
        let before = track_states(&self.project, track_index)?;
        let mut after: Vec<TimelineClipState> = Vec::new();
        after
            .try_reserve(before.len() + 2)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        for existing in &before {
            let existing_end = timeline_clip_end(existing)?;
            if existing.timeline_start >= new_end || new_start >= existing_end {
                after.push(existing.clone());
                continue;
            }
            let covers_head = new_start <= existing.timeline_start;
            let covers_tail = new_end >= existing_end;
            if covers_head && covers_tail {
                continue;
            }
            if covers_tail {
                let duration = new_start
                    .checked_sub(existing.timeline_start)
                    .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
                if !duration.is_positive() {
                    return Err(OperationError::new(OperationErrorCode::TimelineOverlap));
                }
                after.push(TimelineClipState {
                    timeline_start: existing.timeline_start,
                    timeline_duration: duration,
                    content: trim_content_end(&existing.content, duration)?,
                    ..existing.clone()
                });
                continue;
            }
            if covers_head {
                let delta = new_end
                    .checked_sub(existing.timeline_start)
                    .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
                let duration = existing_end
                    .checked_sub(new_end)
                    .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
                if !duration.is_positive() {
                    return Err(OperationError::new(OperationErrorCode::TimelineOverlap));
                }
                after.push(TimelineClipState {
                    timeline_start: new_end,
                    timeline_duration: duration,
                    content: trim_content_start(&existing.content, delta, duration)?,
                    ..existing.clone()
                });
                continue;
            }
            let Some(fragment_id) = split_clip_id else {
                return Err(OperationError::new(
                    OperationErrorCode::TimelineOverwriteSplitRequiresClipId,
                ));
            };
            let head_duration = new_start
                .checked_sub(existing.timeline_start)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
            let tail_delta = new_end
                .checked_sub(existing.timeline_start)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
            let tail_duration = existing_end
                .checked_sub(new_end)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
            if !head_duration.is_positive() || !tail_duration.is_positive() {
                return Err(OperationError::new(OperationErrorCode::TimelineOverlap));
            }
            after.push(TimelineClipState {
                timeline_duration: head_duration,
                content: trim_content_end(&existing.content, head_duration)?,
                ..existing.clone()
            });
            after.push(TimelineClipState {
                clip_id: fragment_id,
                timeline_start: new_end,
                timeline_duration: tail_duration,
                content: trim_content_start(&existing.content, tail_delta, tail_duration)?,
                ..existing.clone()
            });
        }
        let index = after
            .iter()
            .take_while(|state| state.timeline_start < new_start)
            .count();
        after.insert(index, clip);
        validate_track_states(&self.project, track_kind, &after)?;
        let clips = states_into_domain(after.clone())?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (_, history_entry) = timeline_change_sets(ProjectChange::TimelineClipsOverwritten {
            track_id,
            before: before.clone(),
            after: after.clone(),
        })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.project
            .replace_timeline_track_clips_for_command(track_index, clips, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        let (change_set, _) = timeline_change_sets(ProjectChange::TimelineClipsOverwritten {
            track_id,
            before,
            after,
        })?;
        Ok(change_set)
    }

    /// Trims a clip's end and moves every later clip on the track by the same
    /// delta, so shortening closes the gap and lengthening opens one.
    ///
    /// `delta = new_end - old_end` is negative when the clip shortens. Clips before
    /// the trimmed one never move, and `validate_track_states` proves the shifted
    /// list is still ordered and positive, so a shift that would cross a boundary is
    /// rejected rather than written.
    fn ripple_trim_timeline_clip_end(
        &mut self,
        track_id: TrackId,
        track_index: usize,
        index: usize,
        before: TimelineClipState,
        after: TimelineClipState,
        old_end: RationalTime,
    ) -> Result<ChangeSet, OperationError> {
        let track_kind = self.project.timeline().tracks()[track_index].kind();
        let new_end = timeline_clip_end(&after)?;
        let delta = new_end
            .checked_sub(old_end)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let mut states = track_states(&self.project, track_index)?;
        states[index] = after.clone();
        for state in states.iter_mut().skip(index + 1) {
            state.timeline_start = state
                .timeline_start
                .checked_add(delta)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        }
        validate_track_states(&self.project, track_kind, &states)?;
        let clips = states_into_domain(states.clone())?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let before_states = states_before_ripple(&states, index, &before)?;
        let (_, history_entry) = timeline_change_sets(ProjectChange::TimelineClipsRippleTrimmed {
            track_id,
            before: before_states.clone(),
            after: states.clone(),
        })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.project
            .replace_timeline_track_clips_for_command(track_index, clips, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        let (change_set, _) = timeline_change_sets(ProjectChange::TimelineClipsRippleTrimmed {
            track_id,
            before: before_states,
            after: states,
        })?;
        Ok(change_set)
    }

    fn insert_timeline_clip_state(
        &mut self,
        track_id: TrackId,
        track_index: usize,
        clip: TimelineClipState,
    ) -> Result<ChangeSet, OperationError> {
        if find_timeline_clip(&self.project, clip.clip_id).is_some() {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipIdAlreadyExists,
            ));
        }
        ensure_timeline_clip_capacity(
            &self.project,
            track_index,
            1,
            1,
            crate::MAX_TIMELINE_CLIPS,
            crate::MAX_TIMELINE_CLIPS_PER_TRACK,
        )?;
        let index = clip_insertion_index(&self.project, track_index, &clip, None)?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineClipInserted {
                track_id,
                index,
                clip: clip.clone(),
            })?;
        self.project
            .try_reserve_timeline_track_clips(track_index, 1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project.insert_timeline_clip_for_command(
            track_index,
            index,
            clip.into_domain(),
            after_revision,
        );
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    /// Moves a clip's source window without moving the clip.
    ///
    /// A slip changes which part of the media the clip shows while its timeline
    /// start and duration stay exactly where they were, so a cut never moves. The
    /// resulting content is validated by the same `validate_track_states` every
    /// other edit uses, which is what refuses a slip past the end of the media
    /// instead of letting the clip reference material that does not exist.
    /// Moves a track to another position in the track list.
    ///
    /// Order is presentation-visible but canonical: it decides which track a clip
    /// lands on when a client names one by index, and which track is "the first
    /// video track" everywhere. A move therefore goes through the command path and
    /// is undoable like any other edit. The track keeps its id, kind, state and
    /// clips; only its position changes.
    fn apply_timeline_track_move(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineTrackMoveArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let from_index = find_timeline_track_index(&self.project, arguments.track_id)
            .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineTrackNotFound))?;
        let track_count = self.project.timeline().tracks().len();
        if arguments.to_index >= track_count {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        if from_index == arguments.to_index {
            return Ok(ChangeSet::default());
        }
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineTrackMoved {
                track_id: arguments.track_id,
                from_index,
                to_index: arguments.to_index,
            })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.project.move_timeline_track_for_command(
            from_index,
            arguments.to_index,
            after_revision,
        );
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_clip_slip(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipSlipArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some((track_index, index)) = find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };
        let track = &self.project.timeline().tracks()[track_index];
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let track_id = track.id();
        let before = TimelineClipState::from(&track.clips()[index]);
        let delta = arguments.delta.into_time()?;
        if !delta.is_positive() && !delta.is_negative() {
            return Ok(ChangeSet::default());
        }
        let after = TimelineClipState {
            content: trim_content_start(&before.content, delta, before.timeline_duration)?,
            ..before.clone()
        };
        if before == after {
            return Ok(ChangeSet::default());
        }
        let mut states = track_states(&self.project, track_index)?;
        states[index] = after.clone();
        validate_track_states(&self.project, track.kind(), &states)?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineClipUpdated {
                track_id,
                index,
                before,
                after: after.clone(),
            })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.project.replace_timeline_clip_for_command(
            track_index,
            index,
            after.into_domain(),
            after_revision,
        );
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    /// Moves the boundary between two adjacent clips without moving either clip as
    /// a whole: the left clip's end and the right clip's start move by the same
    /// delta, and the right clip's source advances with its start.
    ///
    /// The two clips must be exactly adjacent — a gap means there is no shared
    /// boundary to roll — and both must keep a positive duration with their source
    /// inside the media, which `validate_track_states` proves rather than trusting
    /// the arithmetic above it.
    fn apply_timeline_clip_roll(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipRollArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some((track_index, index)) = find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };
        let track = &self.project.timeline().tracks()[track_index];
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let track_id = track.id();
        let track_kind = track.kind();
        let before = track_states(&self.project, track_index)?;
        let Some(left) = before.get(index).cloned() else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };
        let Some(right) = before.get(index + 1).cloned() else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };
        let left_end = timeline_clip_end(&left)?;
        if right.timeline_start != left_end {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let delta = arguments.delta.into_time()?;
        if !delta.is_positive() && !delta.is_negative() {
            return Ok(ChangeSet::default());
        }
        let new_boundary = left_end
            .checked_add(delta)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        // Both sides must keep a positive duration; anything else is not a roll.
        let left_duration = new_boundary
            .checked_sub(left.timeline_start)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let right_end = timeline_clip_end(&right)?;
        let right_duration = right_end
            .checked_sub(new_boundary)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if !left_duration.is_positive() || !right_duration.is_positive() {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let after_left = TimelineClipState {
            timeline_duration: left_duration,
            content: trim_content_end(&left.content, left_duration)?,
            ..left.clone()
        };
        let after_right = TimelineClipState {
            timeline_start: new_boundary,
            timeline_duration: right_duration,
            content: trim_content_start(&right.content, delta, right_duration)?,
            ..right.clone()
        };
        let mut after = before.clone();
        after[index] = after_left;
        after[index + 1] = after_right;
        validate_track_states(&self.project, track_kind, &after)?;
        let clips = states_into_domain(after.clone())?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (_, history_entry) = timeline_change_sets(ProjectChange::TimelineClipsRolled {
            track_id,
            before: before.clone(),
            after: after.clone(),
        })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.project
            .replace_timeline_track_clips_for_command(track_index, clips, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        let (change_set, _) = timeline_change_sets(ProjectChange::TimelineClipsRolled {
            track_id,
            before,
            after,
        })?;
        Ok(change_set)
    }

    /// Moves a clip in time and lets its neighbours absorb the move.
    ///
    /// The clip keeps its duration and its source; the clip before it gives up or
    /// gains the same amount at its end, and the clip after it does the same at its
    /// start with its source advancing. A slide therefore never opens a gap or an
    /// overlap. It needs a clip on both sides that exactly touches this one, and
    /// both neighbours must keep a positive duration with their source inside the
    /// media, which `validate_track_states` proves.
    fn apply_timeline_clip_slide(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipSlideArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some((track_index, index)) = find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };
        let track = &self.project.timeline().tracks()[track_index];
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let track_id = track.id();
        let track_kind = track.kind();
        let before = track_states(&self.project, track_index)?;
        if index == 0 || index + 1 >= before.len() {
            // A slide is defined by the neighbours it borrows from; at either end
            // of the track there is nothing to slide against.
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let previous = before[index - 1].clone();
        let moving = before[index].clone();
        let next = before[index + 1].clone();
        let old_start = moving.timeline_start;
        let old_end = timeline_clip_end(&moving)?;
        if timeline_clip_end(&previous)? != old_start || next.timeline_start != old_end {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let delta = arguments.delta.into_time()?;
        if !delta.is_positive() && !delta.is_negative() {
            return Ok(ChangeSet::default());
        }
        let new_start = old_start
            .checked_add(delta)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let new_end = old_end
            .checked_add(delta)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let previous_duration = new_start
            .checked_sub(previous.timeline_start)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let next_end = timeline_clip_end(&next)?;
        let next_duration = next_end
            .checked_sub(new_end)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if !previous_duration.is_positive() || !next_duration.is_positive() {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let mut after = before.clone();
        after[index - 1] = TimelineClipState {
            timeline_duration: previous_duration,
            content: trim_content_end(&previous.content, previous_duration)?,
            ..previous
        };
        after[index] = TimelineClipState {
            timeline_start: new_start,
            ..moving
        };
        after[index + 1] = TimelineClipState {
            timeline_start: new_end,
            timeline_duration: next_duration,
            content: trim_content_start(&next.content, delta, next_duration)?,
            ..next
        };
        validate_track_states(&self.project, track_kind, &after)?;
        let clips = states_into_domain(after.clone())?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (_, history_entry) = timeline_change_sets(ProjectChange::TimelineClipsSlid {
            track_id,
            before: before.clone(),
            after: after.clone(),
        })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.project
            .replace_timeline_track_clips_for_command(track_index, clips, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        let (change_set, _) = timeline_change_sets(ProjectChange::TimelineClipsSlid {
            track_id,
            before,
            after,
        })?;
        Ok(change_set)
    }

    fn apply_timeline_clip_update(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipUpdateArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some((track_index, index)) = find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };
        let track = &self.project.timeline().tracks()[track_index];
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let track_id = track.id();
        let before = TimelineClipState::from(&track.clips()[index]);
        let after = TimelineClipState {
            clip_id: before.clip_id,
            timeline_start: before.timeline_start,
            timeline_duration: arguments.timeline_duration.into_time()?,
            content: arguments.content,
            settings: arguments.settings,
        };
        if before == after {
            return Ok(ChangeSet::default());
        }
        let mut states = track_states(&self.project, track_index)?;
        states[index] = after.clone();
        validate_track_states(&self.project, track.kind(), &states)?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineClipUpdated {
                track_id,
                index,
                before,
                after: after.clone(),
            })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project.replace_timeline_clip_for_command(
            track_index,
            index,
            after.into_domain(),
            after_revision,
        );
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_clip_delete(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipDeleteArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some((track_index, index)) = find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };
        if self.project.timeline().tracks()[track_index]
            .state()
            .locked()
        {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let track_id = self.project.timeline().tracks()[track_index].id();
        let clip =
            TimelineClipState::from(&self.project.timeline().tracks()[track_index].clips()[index]);
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineClipDeleted {
                track_id,
                index,
                clip,
            })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project
            .remove_timeline_clip_for_command(track_index, index, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_clip_trim(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipTrimArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let target = arguments.timeline_time.into_time()?;
        let Some((track_index, index)) = find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };

        let track = &self.project.timeline().tracks()[track_index];
        let track_id = track.id();
        let before = TimelineClipState::from(&track.clips()[index]);
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let timeline_end = timeline_clip_end(&before)?;
        let current_edge = match arguments.edge {
            TimelineTrimEdge::Start => before.timeline_start,
            TimelineTrimEdge::End => timeline_end,
        };
        if target == current_edge {
            return Ok(ChangeSet::default());
        }

        let after = match arguments.edge {
            TimelineTrimEdge::Start => {
                if target.is_negative() || target >= timeline_end {
                    return Err(OperationError::new(OperationErrorCode::InvalidArguments));
                }
                let delta = target
                    .checked_sub(before.timeline_start)
                    .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
                let duration = timeline_end
                    .checked_sub(target)
                    .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
                TimelineClipState {
                    timeline_start: target,
                    timeline_duration: duration,
                    content: trim_content_start(&before.content, delta, duration)?,
                    ..before.clone()
                }
            }
            TimelineTrimEdge::End => {
                if target <= before.timeline_start {
                    return Err(OperationError::new(OperationErrorCode::InvalidArguments));
                }
                let duration = target
                    .checked_sub(before.timeline_start)
                    .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
                TimelineClipState {
                    timeline_duration: duration,
                    content: trim_content_end(&before.content, duration)?,
                    ..before.clone()
                }
            }
        };

        if arguments.ripple {
            if arguments.edge != TimelineTrimEdge::End {
                return Err(OperationError::new(OperationErrorCode::InvalidArguments));
            }
            return self.ripple_trim_timeline_clip_end(
                track_id,
                track_index,
                index,
                before,
                after,
                timeline_end,
            );
        }

        let mut states = track_states(&self.project, track_index)?;
        states[index] = after.clone();
        validate_track_states(&self.project, track.kind(), &states)?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineClipTrimmed {
                track_id,
                index,
                before,
                after: after.clone(),
            })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project.replace_timeline_clip_for_command(
            track_index,
            index,
            after.into_domain(),
            after_revision,
        );
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_clip_split(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipSplitArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let split_time = arguments.timeline_time.into_time()?;
        let Some((track_index, index)) = find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };
        if find_timeline_clip(&self.project, arguments.new_clip_id).is_some() {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipIdAlreadyExists,
            ));
        }
        ensure_timeline_clip_capacity(
            &self.project,
            track_index,
            1,
            1,
            crate::MAX_TIMELINE_CLIPS,
            crate::MAX_TIMELINE_CLIPS_PER_TRACK,
        )?;

        let track = &self.project.timeline().tracks()[track_index];
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let track_id = track.id();
        let before = TimelineClipState::from(&track.clips()[index]);
        let original_end = timeline_clip_end(&before)?;
        if split_time <= before.timeline_start || split_time >= original_end {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let left_duration = split_time
            .checked_sub(before.timeline_start)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let right_duration = before
            .timeline_duration
            .checked_sub(left_duration)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let (left_content, right_content) =
            split_content(&before.content, left_duration, right_duration)?;
        let left_after = TimelineClipState {
            timeline_duration: left_duration,
            content: left_content,
            ..before.clone()
        };
        let right_after = TimelineClipState {
            clip_id: arguments.new_clip_id,
            timeline_start: split_time,
            timeline_duration: right_duration,
            content: right_content,
            ..before.clone()
        };
        let mut states = track_states(&self.project, track_index)?;
        states[index] = left_after.clone();
        states
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        states.insert(index + 1, right_after.clone());
        validate_track_states(&self.project, track.kind(), &states)?;
        let clips = states_into_domain(states)?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) = timeline_change_sets(ProjectChange::TimelineClipSplit {
            track_id,
            index,
            before,
            left_after,
            right_after,
        })?;
        self.project
            .try_reserve_timeline_track_clips(track_index, 1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project
            .replace_timeline_track_clips_for_command(track_index, clips, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_clip_ripple_delete(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipDeleteArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some((track_index, index)) = find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };
        let track = &self.project.timeline().tracks()[track_index];
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let track_id = track.id();
        let deleted_clip = TimelineClipState::from(&track.clips()[index]);
        let shift_duration = deleted_clip.timeline_duration;
        let shifted_count = track.clips().len().saturating_sub(index + 1);
        let mut states = track_states(&self.project, track_index)?;
        let before_fingerprint = timeline_states_fingerprint(&states);
        states.remove(index);
        for state in states.iter_mut().skip(index) {
            state.timeline_start = state
                .timeline_start
                .checked_sub(shift_duration)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        }
        validate_track_states(&self.project, track.kind(), &states)?;
        let after_fingerprint = timeline_states_fingerprint(&states);
        let clips = states_into_domain(states)?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, mut history_entry) =
            timeline_change_sets(ProjectChange::TimelineClipRippleDeleted {
                track_id,
                index,
                deleted_clip,
                shifted_count,
                shift_duration,
            })?;
        history_entry.ripple_history_guard = Some(RippleHistoryGuard {
            before_fingerprint,
            after_fingerprint,
        });
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project
            .replace_timeline_track_clips_for_command(track_index, clips, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    /// Moves several clips on one track by the same delta as a single step.
    ///
    /// A group drag is one gesture and one undo: the change carries the whole
    /// track before and after, exactly like overwrite, because five separate
    /// moves would take five undos and could stop half-applied.
    fn apply_timeline_clips_move(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipsMoveArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if arguments.clip_ids.is_empty() {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let track_id = arguments.track_id;
        let track_index = find_timeline_track_index(&self.project, track_id)
            .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineTrackNotFound))?;
        let tracks = self.project.timeline().tracks();
        let track = &tracks[track_index];
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        let track_kind = track.kind();
        let before: Vec<TimelineClipState> =
            track.clips().iter().map(TimelineClipState::from).collect();
        let delta = arguments
            .delta
            .into_time()
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let mut after = before.clone();
        for clip_id in &arguments.clip_ids {
            let index = after
                .iter()
                .position(|state| state.clip_id == *clip_id)
                .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineClipNotFound))?;
            let timeline_start = after[index]
                .timeline_start
                .checked_add(delta)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
            after[index].timeline_start = timeline_start;
        }
        if after == before {
            return Ok(ChangeSet::default());
        }
        validate_track_states(&self.project, track_kind, &after)?;
        let clips = states_into_domain(after.clone())?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineClipsOverwritten {
                track_id,
                before: before.clone(),
                after: after.clone(),
            })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.project
            .replace_timeline_track_clips_for_command(track_index, clips, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_clip_move(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipMoveArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some((from_track_index, from_index)) =
            find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };
        let to_track_index = find_timeline_track_index(&self.project, arguments.track_id)
            .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineTrackNotFound))?;
        let tracks = self.project.timeline().tracks();
        let from_track = &tracks[from_track_index];
        let to_track = &tracks[to_track_index];
        if from_track.state().locked() || to_track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        if from_track.kind() != to_track.kind() {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let current = TimelineClipState::from(&from_track.clips()[from_index]);
        let moved = TimelineClipState {
            timeline_start: arguments.timeline_start.into_time()?,
            ..current.clone()
        };
        validate_clip_state(&self.project, to_track.kind(), &moved)?;

        if from_track_index == to_track_index && moved.timeline_start == current.timeline_start {
            return Ok(ChangeSet::default());
        }

        if from_track_index != to_track_index {
            ensure_timeline_clip_capacity(
                &self.project,
                to_track_index,
                0,
                1,
                crate::MAX_TIMELINE_CLIPS,
                crate::MAX_TIMELINE_CLIPS_PER_TRACK,
            )?;
        }
        let to_index = clip_insertion_index(
            &self.project,
            to_track_index,
            &moved,
            Some(arguments.clip_id),
        )?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) = timeline_change_sets(ProjectChange::TimelineClipMoved {
            from_track_id: from_track.id(),
            from_index,
            to_track_id: to_track.id(),
            to_index,
            before: current.clone(),
            after: moved.clone(),
        })?;
        if from_track_index != to_track_index {
            self.project
                .try_reserve_timeline_track_clips(to_track_index, 1)
                .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        }
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project.move_timeline_clip_for_command(
            from_track_index,
            from_index,
            to_track_index,
            to_index,
            moved.timeline_start,
            after_revision,
        );
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_sequence_set_frame_rate(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineSequenceSetFrameRateArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let before = self.project.timeline().sequence_frame_rate();
        let after = arguments
            .sequence_frame_rate
            .ok_or_else(|| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if before == after {
            return Ok(ChangeSet::default());
        }
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let change_set = ChangeSet::timeline_sequence_frame_rate_changed(before, after)?;
        let history_entry = change_set.clone();
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project
            .set_sequence_frame_rate_for_command(after, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_marker_add(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineMarkerAddArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let timeline_time = arguments.timeline_time.into_time()?;
        validate_marker_time(timeline_time)?;
        validate_marker_label(&arguments.label)?;
        if find_timeline_marker(&self.project, arguments.marker_id).is_some() {
            return Err(OperationError::new(
                OperationErrorCode::TimelineMarkerIdAlreadyExists,
            ));
        }
        if self.project.timeline().markers().len() >= MAX_TIMELINE_MARKERS {
            return Err(OperationError::new(
                OperationErrorCode::TimelineLimitExceeded,
            ));
        }
        let marker = TimelineMarkerState {
            marker_id: arguments.marker_id,
            timeline_time,
            label: arguments.label,
        };
        let index =
            marker_insertion_index(&self.project, marker.marker_id, marker.timeline_time, None);
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineMarkerAdded {
                marker: marker.clone(),
                index,
            })?;
        self.project
            .try_reserve_timeline_markers(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project.insert_timeline_marker_for_command(
            index,
            marker.into_domain(),
            after_revision,
        );
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_marker_move(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineMarkerMoveArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some(index) = find_timeline_marker(&self.project, arguments.marker_id) else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineMarkerNotFound,
            ));
        };
        let timeline_time = arguments.timeline_time.into_time()?;
        validate_marker_time(timeline_time)?;
        let current = &self.project.timeline().markers()[index];
        if current.timeline_time() == timeline_time {
            return Ok(ChangeSet::default());
        }
        let to_index = marker_insertion_index(
            &self.project,
            arguments.marker_id,
            timeline_time,
            Some(arguments.marker_id),
        );
        let change = ProjectChange::TimelineMarkerMoved {
            marker_id: current.id(),
            label: current.label().to_owned(),
            from_time: current.timeline_time(),
            from_index: index,
            to_time: timeline_time,
            to_index,
        };
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) = timeline_change_sets(change)?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project.move_timeline_marker_for_command(
            index,
            to_index,
            timeline_time,
            after_revision,
        );
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_marker_rename(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineMarkerRenameArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some(index) = find_timeline_marker(&self.project, arguments.marker_id) else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineMarkerNotFound,
            ));
        };
        let current = &self.project.timeline().markers()[index];
        if current.label() == arguments.label {
            return Ok(ChangeSet::default());
        }
        validate_marker_label(&arguments.label)?;
        let change = ProjectChange::TimelineMarkerRenamed {
            marker_id: current.id(),
            time: current.timeline_time(),
            before_label: current.label().to_owned(),
            after_label: arguments.label.clone(),
        };
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) = timeline_change_sets(change)?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project.replace_timeline_marker_for_command(
            index,
            current.clone().with_label_for_command(arguments.label),
            after_revision,
        );
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn apply_timeline_marker_delete(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineMarkerDeleteArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let Some(index) = find_timeline_marker(&self.project, arguments.marker_id) else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineMarkerNotFound,
            ));
        };
        let marker = TimelineMarkerState::from(&self.project.timeline().markers()[index]);
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineMarkerDeleted { marker, index })?;
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        self.project
            .remove_timeline_marker_for_command(index, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok(change_set)
    }

    fn timeline_tracks(
        &self,
        arguments: Value,
    ) -> Result<Vec<TimelineTrackSummary>, OperationError> {
        parse_empty_arguments(arguments)?;
        if self
            .project
            .timeline()
            .tracks()
            .iter()
            .any(|track| matches!(track.kind(), TrackKind::Text | TrackKind::Caption))
        {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedQuerySchema,
            ));
        }
        Ok(self
            .project
            .timeline()
            .tracks()
            .iter()
            .map(|track| TimelineTrackSummary {
                track_id: track.id(),
                kind: track.kind(),
                clip_count: track.clips().len(),
            })
            .collect())
    }

    fn timeline_tracks_v2(
        &self,
        arguments: Value,
    ) -> Result<Vec<TimelineTrackSummaryV2>, OperationError> {
        parse_empty_arguments(arguments)?;
        Ok(self
            .project
            .timeline()
            .tracks()
            .iter()
            .map(|track| TimelineTrackSummaryV2 {
                track_id: track.id(),
                kind: track.kind(),
                state: track.state(),
                clip_count: track.clips().len(),
            })
            .collect())
    }

    fn timeline_clips(&self, arguments: Value) -> Result<TimelineClipPage, OperationError> {
        let arguments: TimelineClipsQueryArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let offset = usize::try_from(arguments.offset)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let limit = usize::try_from(arguments.limit)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if limit == 0 || limit > MAX_TIMELINE_CLIP_PAGE_SIZE {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let track = self
            .project
            .timeline()
            .tracks()
            .iter()
            .find(|track| track.id() == arguments.track_id)
            .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineTrackNotFound))?;
        if matches!(track.kind(), TrackKind::Text | TrackKind::Caption) {
            return Err(OperationError::new(
                OperationErrorCode::UnsupportedQuerySchema,
            ));
        }
        let total_count = track.clips().len();
        let start = offset.min(total_count);
        let end = offset.saturating_add(limit).min(total_count);
        let items = track.clips()[start..end]
            .iter()
            .map(LegacyTimelineClipState::from_media_clip)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| OperationError::new(OperationErrorCode::UnsupportedQuerySchema))?;
        Ok(TimelineClipPage {
            track_id: arguments.track_id,
            items,
            total_count,
            offset,
            limit,
            next_offset: (end < total_count).then_some(end),
        })
    }

    fn timeline_clips_v2(&self, arguments: Value) -> Result<TimelineClipPageV2, OperationError> {
        let arguments: TimelineClipsQueryArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let offset = usize::try_from(arguments.offset)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let limit = usize::try_from(arguments.limit)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if limit == 0 || limit > MAX_TIMELINE_CLIP_PAGE_SIZE {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let track = self
            .project
            .timeline()
            .tracks()
            .iter()
            .find(|track| track.id() == arguments.track_id)
            .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineTrackNotFound))?;
        let total_count = track.clips().len();
        let start = offset.min(total_count);
        let end = offset.saturating_add(limit).min(total_count);
        let items = track.clips()[start..end]
            .iter()
            .map(TimelineClipState::from)
            .collect();
        Ok(TimelineClipPageV2 {
            track_id: arguments.track_id,
            items,
            total_count,
            offset,
            limit,
            next_offset: (end < total_count).then_some(end),
        })
    }

    fn timeline_markers(&self, arguments: Value) -> Result<TimelineMarkerPage, OperationError> {
        let arguments: TimelineMarkersQueryArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let offset = usize::try_from(arguments.offset)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let limit = usize::try_from(arguments.limit)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if limit == 0 || limit > MAX_TIMELINE_MARKER_PAGE_SIZE {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let markers = self.project.timeline().markers();
        let total_count = markers.len();
        let start = offset.min(total_count);
        let end = offset.saturating_add(limit).min(total_count);
        let items = markers[start..end]
            .iter()
            .map(TimelineMarkerState::from)
            .collect();
        Ok(TimelineMarkerPage {
            items,
            total_count,
            offset,
            limit,
            next_offset: (end < total_count).then_some(end),
        })
    }

    fn timeline_snap(
        &self,
        arguments: Value,
        marker_aware: bool,
    ) -> Result<TimelineSnapResult, OperationError> {
        let arguments: TimelineSnapQueryArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let target_time = arguments.target_time.into_time()?;
        let Some((source_track_index, source_clip_index)) =
            find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };

        match arguments.operation {
            TimelineSnapOperation::Move => {
                let target_track_id = arguments
                    .target_track_id
                    .ok_or_else(|| OperationError::new(OperationErrorCode::InvalidArguments))?;
                let target_track_index = find_timeline_track_index(&self.project, target_track_id)
                    .ok_or_else(|| {
                        OperationError::new(OperationErrorCode::TimelineTrackNotFound)
                    })?;
                let tracks = self.project.timeline().tracks();
                if tracks[source_track_index].kind() != tracks[target_track_index].kind() {
                    return Err(OperationError::new(OperationErrorCode::InvalidArguments));
                }
                let clip = &tracks[source_track_index].clips()[source_clip_index];
                let duration = clip.timeline_duration();
                resolve_timeline_snap_with_markers(
                    &self.project,
                    arguments.clip_id,
                    TimelineSnapMode::Move { duration },
                    target_time,
                    marker_aware,
                )
            }
            TimelineSnapOperation::TrimStart | TimelineSnapOperation::TrimEnd => {
                if arguments.target_track_id.is_some() {
                    return Err(OperationError::new(OperationErrorCode::InvalidArguments));
                }
                let edge = match arguments.operation {
                    TimelineSnapOperation::TrimStart => TimelineTrimEdge::Start,
                    TimelineSnapOperation::TrimEnd => TimelineTrimEdge::End,
                    TimelineSnapOperation::Move => unreachable!(),
                };
                resolve_timeline_snap_with_markers(
                    &self.project,
                    arguments.clip_id,
                    TimelineSnapMode::Trim { edge },
                    target_time,
                    marker_aware,
                )
            }
        }
    }

    fn media_list(&self, arguments: Value) -> Result<MediaListPage, OperationError> {
        let arguments: MediaListArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let offset = usize::try_from(arguments.offset)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let limit = usize::try_from(arguments.limit)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if limit == 0 || limit > MAX_MEDIA_PAGE_SIZE {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }

        let items = self.project.media_items();
        let total_count = items.len();
        let start = offset.min(total_count);
        let end = offset.saturating_add(limit).min(total_count);
        let page_items = items[start..end].to_vec();
        Ok(MediaListPage {
            items: page_items,
            total_count,
            offset,
            limit,
            next_offset: (end < total_count).then_some(end),
        })
    }

    fn media_get(&self, arguments: Value) -> Result<MediaItem, OperationError> {
        let arguments: MediaGetArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        self.project
            .media_items()
            .iter()
            .find(|item| item.id() == arguments.media_id)
            .cloned()
            .ok_or_else(|| OperationError::new(OperationErrorCode::MediaNotFound))
    }

    fn check_project_preconditions(
        &self,
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_revision: ProjectRevision,
    ) -> Result<(), OperationError> {
        if project_id != self.project_id() {
            return Err(OperationError::new(OperationErrorCode::ProjectIdMismatch));
        }
        if project_instance_id != self.project_instance_id {
            return Err(OperationError::new(
                OperationErrorCode::ProjectInstanceMismatch,
            ));
        }
        if expected_revision != self.project_revision() {
            return Err(OperationError::revision_conflict(self.project_revision()));
        }
        Ok(())
    }

    fn apply_forward_commands(
        &mut self,
        commands: &[CommandCall],
    ) -> Result<(ProjectRevision, ChangeSet), OperationError> {
        let before_revision = self.project_revision();
        let original_name = self.project.name().to_owned();
        let mut staged_name = original_name.clone();
        for command in commands {
            stage_groupable_command(command, &mut staged_name)?;
        }

        let change_set = ChangeSet::project_name(&original_name, &staged_name);
        if change_set.is_empty() {
            return Ok((before_revision, change_set));
        }

        let after_revision = before_revision
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let history_entry = change_set.clone();
        self.history
            .undo
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        // All validation, staging, revision checks, and history allocation are complete.
        self.project.rename_for_command(staged_name, after_revision);
        self.history.undo.push(history_entry);
        self.history.redo.clear();
        Ok((after_revision, change_set))
    }

    fn apply_history(
        &mut self,
        direction: HistoryDirection,
    ) -> Result<(ProjectRevision, ProjectRevision, ChangeSet), OperationError> {
        let entry = match direction {
            HistoryDirection::Undo => self
                .history
                .undo
                .last()
                .ok_or_else(|| OperationError::new(OperationErrorCode::NothingToUndo))?,
            HistoryDirection::Redo => self
                .history
                .redo
                .last()
                .ok_or_else(|| OperationError::new(OperationErrorCode::NothingToRedo))?,
        };
        let (staged_change, applied_change) = entry
            .stage_history_change(&self.project, matches!(direction, HistoryDirection::Undo))?;
        let before_revision = self.project_revision();
        let after_revision = before_revision
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;

        match direction {
            HistoryDirection::Undo => self.history.redo.try_reserve(1),
            HistoryDirection::Redo => self.history.undo.try_reserve(1),
        }
        .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        match &staged_change {
            StagedHistoryChange::InsertMedia { .. } => self.project.try_reserve_media_items(1),
            StagedHistoryChange::InsertTimelineTrack { .. }
            | StagedHistoryChange::InsertCaptionTrack { .. } => {
                self.project.try_reserve_timeline_tracks(1)
            }
            StagedHistoryChange::InsertTimelineClip { track_index, .. } => self
                .project
                .try_reserve_timeline_track_clips(*track_index, 1),
            StagedHistoryChange::InsertTimelineMarker { .. } => {
                self.project.try_reserve_timeline_markers(1)
            }
            StagedHistoryChange::MoveTimelineClip {
                from_track_index,
                to_track_index,
                ..
            } if from_track_index != to_track_index => self
                .project
                .try_reserve_timeline_track_clips(*to_track_index, 1),
            StagedHistoryChange::ReplaceTimelineTrack { track_index, clips }
                if clips.len() > self.project.timeline().tracks()[*track_index].clips().len() =>
            {
                self.project.try_reserve_timeline_track_clips(
                    *track_index,
                    clips.len() - self.project.timeline().tracks()[*track_index].clips().len(),
                )
            }
            _ => Ok(()),
        }
        .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;

        let moved_entry = match direction {
            HistoryDirection::Undo => self.history.undo.pop(),
            HistoryDirection::Redo => self.history.redo.pop(),
        }
        .ok_or_else(|| OperationError::new(OperationErrorCode::HistoryConflict))?;

        // The destination stack was reserved and the paired document update cannot fail.
        match staged_change {
            StagedHistoryChange::Rename(name) => {
                self.project.rename_for_command(name, after_revision);
            }
            StagedHistoryChange::SetSequenceFrameRate(rate) => {
                self.project
                    .set_sequence_frame_rate_for_command(rate, after_revision);
            }
            StagedHistoryChange::InsertMedia { item, index } => {
                self.project
                    .insert_media_for_command(item, index, after_revision);
            }
            StagedHistoryChange::RemoveMedia { index } => {
                self.project.remove_media_for_command(index, after_revision);
            }
            StagedHistoryChange::ReplaceMedia { item, index } => {
                self.project
                    .replace_media_for_command(index, item, after_revision);
            }
            StagedHistoryChange::InsertTimelineTrack {
                track_id,
                kind,
                state,
                index,
            } => self.project.insert_timeline_track_for_command(
                index,
                TimelineTrack::empty_for_command(track_id, kind).with_state_for_command(state),
                after_revision,
            ),
            StagedHistoryChange::InsertCaptionTrack { track, index } => self
                .project
                .insert_timeline_track_for_command(index, track, after_revision),
            StagedHistoryChange::MoveTimelineTrack {
                from_index,
                to_index,
            } => {
                self.project
                    .move_timeline_track_for_command(from_index, to_index, after_revision);
            }
            StagedHistoryChange::RemoveTimelineTrack { index } => {
                self.project
                    .remove_timeline_track_for_command(index, after_revision);
            }
            StagedHistoryChange::SetTimelineTrackState { track_index, state } => self
                .project
                .set_timeline_track_state_for_command(track_index, state, after_revision),
            StagedHistoryChange::InsertTimelineClip {
                track_index,
                index,
                clip,
            } => self.project.insert_timeline_clip_for_command(
                track_index,
                index,
                clip.into_domain(),
                after_revision,
            ),
            StagedHistoryChange::RemoveTimelineClip { track_index, index } => {
                self.project
                    .remove_timeline_clip_for_command(track_index, index, after_revision);
            }
            StagedHistoryChange::MoveTimelineClip {
                from_track_index,
                from_index,
                to_track_index,
                to_index,
                to_timeline_start,
            } => self.project.move_timeline_clip_for_command(
                from_track_index,
                from_index,
                to_track_index,
                to_index,
                to_timeline_start,
                after_revision,
            ),
            StagedHistoryChange::ReplaceTimelineClip {
                track_index,
                index,
                clip,
            } => self.project.replace_timeline_clip_for_command(
                track_index,
                index,
                clip.into_domain(),
                after_revision,
            ),
            StagedHistoryChange::ReplaceTimelineTrack { track_index, clips } => self
                .project
                .replace_timeline_track_clips_for_command(track_index, clips, after_revision),
            StagedHistoryChange::InsertTimelineMarker { index, marker } => self
                .project
                .insert_timeline_marker_for_command(index, marker.into_domain(), after_revision),
            StagedHistoryChange::RemoveTimelineMarker { index } => {
                self.project
                    .remove_timeline_marker_for_command(index, after_revision);
            }
            StagedHistoryChange::MoveTimelineMarker {
                from_index,
                to_index,
                to_time,
            } => self.project.move_timeline_marker_for_command(
                from_index,
                to_index,
                to_time,
                after_revision,
            ),
            StagedHistoryChange::RenameTimelineMarker { index, label } => {
                let marker = self.project.timeline().markers()[index]
                    .clone()
                    .with_label_for_command(label);
                self.project
                    .replace_timeline_marker_for_command(index, marker, after_revision);
            }
        }
        match direction {
            HistoryDirection::Undo => self.history.redo.push(moved_entry),
            HistoryDirection::Redo => self.history.undo.push(moved_entry),
        }
        Ok((before_revision, after_revision, applied_change))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaAddArguments {
    item: MediaItem,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaRemoveArguments {
    id: MediaId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaRelinkArguments {
    item: MediaItem,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaListArguments {
    offset: u64,
    limit: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaGetArguments {
    media_id: MediaId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineSequenceSetFrameRateArguments {
    // The outer option records key presence; the inner option represents clearing.
    #[serde(default, deserialize_with = "deserialize_nullable_field")]
    sequence_frame_rate: Option<Option<RationalRate>>,
}

fn deserialize_nullable_field<'de, D>(
    deserializer: D,
) -> Result<Option<Option<RationalRate>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<RationalRate>::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineTrackAddArguments {
    track_id: TrackId,
    kind: TrackKind,
}

fn deserialize_caption_clips<'de, D>(deserializer: D) -> Result<Vec<TimelineClipState>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::media::deserialize_limited_vec(
        deserializer,
        crate::subtitle_io::MAX_IMPORTED_CAPTIONS,
        "caption clips",
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineCaptionsImportArguments {
    track_id: TrackId,
    #[serde(deserialize_with = "deserialize_caption_clips")]
    clips: Vec<TimelineClipState>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineTrackRemoveArguments {
    track_id: TrackId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineTrackSetStateArguments {
    track_id: TrackId,
    state: TrackState,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RationalTimeArguments {
    numerator: i64,
    denominator: u32,
}

impl RationalTimeArguments {
    fn into_time(self) -> Result<RationalTime, OperationError> {
        RationalTime::new(self.numerator, self.denominator)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineSourceRangeArguments {
    start: RationalTimeArguments,
    duration: RationalTimeArguments,
}

/// How an insert treats the range it lands on.
///
/// The mode is an explicit command input, never canonical project state: the same
/// project can be edited in either mode, and the mode a client is using is that
/// client's presentation choice.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEditMode {
    /// Refuse to overlap: the target range must be free.
    #[default]
    Insert,
    /// Replace the target range: covered clips are removed, crossed clips are
    /// trimmed, and a clip strictly containing the new span is split.
    Overwrite,
}

impl TimelineEditMode {
    fn is_insert(&self) -> bool {
        matches!(self, Self::Insert)
    }

    /// The wire spelling used by the command arguments.
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::Overwrite => "overwrite",
        }
    }

    /// Parses the wire spelling a client sends; anything else is not a mode.
    pub fn parse_wire(value: &str) -> Option<Self> {
        match value {
            "insert" => Some(Self::Insert),
            "overwrite" => Some(Self::Overwrite),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipInsertArguments {
    clip_id: ClipId,
    track_id: TrackId,
    #[serde(deserialize_with = "deserialize_canonical_media_id")]
    media_id: MediaId,
    timeline_start: RationalTimeArguments,
    source_range: TimelineSourceRangeArguments,
    #[serde(default)]
    mode: TimelineEditMode,
    /// Only read by `mode: overwrite` when the new span lands strictly inside one
    /// existing clip. The head keeps the original id and the tail takes this one,
    /// mirroring `timeline.clip.split`'s caller-supplied `new_clip_id`.
    #[serde(default)]
    overwrite_split_clip_id: Option<ClipId>,
}

impl TimelineClipInsertArguments {
    fn into_clip_state(self, track_kind: TrackKind) -> Result<TimelineClipState, OperationError> {
        let start = self.source_range.start.into_time()?;
        let duration = self.source_range.duration.into_time()?;
        let source_range = TimeRange::new(start, duration)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        Ok(TimelineClipState {
            clip_id: self.clip_id,
            timeline_start: self.timeline_start.into_time()?,
            timeline_duration: duration,
            content: ClipContent::Media {
                media_id: self.media_id,
                source_range,
                playback_rate: RationalRate::ONE,
                reversed: false,
                frozen: false,
            },
            settings: ClipSettings::default_for(track_kind),
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipInsertContentArguments {
    clip_id: ClipId,
    track_id: TrackId,
    timeline_start: RationalTimeArguments,
    timeline_duration: RationalTimeArguments,
    content: ClipContent,
    settings: ClipSettings,
}

impl TimelineClipInsertContentArguments {
    fn into_clip_state(self) -> Result<TimelineClipState, OperationError> {
        Ok(TimelineClipState {
            clip_id: self.clip_id,
            timeline_start: self.timeline_start.into_time()?,
            timeline_duration: self.timeline_duration.into_time()?,
            content: self.content,
            settings: self.settings,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipUpdateArguments {
    clip_id: ClipId,
    timeline_duration: RationalTimeArguments,
    content: ClipContent,
    settings: ClipSettings,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipMoveArguments {
    clip_id: ClipId,
    track_id: TrackId,
    timeline_start: RationalTimeArguments,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipsMoveArguments {
    track_id: TrackId,
    clip_ids: Vec<ClipId>,
    delta: RationalTimeArguments,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipDeleteArguments {
    clip_id: ClipId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipTrimArguments {
    clip_id: ClipId,
    edge: TimelineTrimEdge,
    timeline_time: RationalTimeArguments,
    /// Trim the clip's end and pull every later clip on the track with it, so
    /// shortening closes the gap instead of leaving one. Absent or false keeps the
    /// plain trim meaning. Start-edge ripple is refused: it has two defensible
    /// meanings and choosing one silently would be a product decision.
    #[serde(default)]
    ripple: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipSlipArguments {
    clip_id: ClipId,
    /// How far to move the clip's source window, in timeline units. Positive takes
    /// later source material, negative takes earlier material; the clip's timeline
    /// span never changes.
    delta: RationalTimeArguments,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineTrackMoveArguments {
    track_id: TrackId,
    /// Where the track should end up. Tracks above it shift down and tracks below
    /// shift up; the ids and contents never change.
    to_index: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipSlideArguments {
    clip_id: ClipId,
    /// How far the clip moves in time. Its duration and source never change; the
    /// clip before it gives up or gains that much at its end, and the clip after it
    /// does the same at its start, so the track stays gapless.
    delta: RationalTimeArguments,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipRollArguments {
    /// The left-hand clip of the boundary. The clip that follows it on the same
    /// track must be exactly adjacent, or there is no boundary to roll.
    clip_id: ClipId,
    /// How far the boundary moves: positive shortens the left clip and lengthens
    /// the right one, negative does the opposite.
    delta: RationalTimeArguments,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipSplitArguments {
    clip_id: ClipId,
    new_clip_id: ClipId,
    timeline_time: RationalTimeArguments,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineMarkerAddArguments {
    marker_id: MarkerId,
    timeline_time: RationalTimeArguments,
    label: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineMarkerMoveArguments {
    marker_id: MarkerId,
    timeline_time: RationalTimeArguments,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineMarkerRenameArguments {
    marker_id: MarkerId,
    label: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineMarkerDeleteArguments {
    marker_id: MarkerId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipsQueryArguments {
    track_id: TrackId,
    offset: u64,
    limit: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineMarkersQueryArguments {
    offset: u64,
    limit: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineSnapQueryArguments {
    operation: TimelineSnapOperation,
    clip_id: ClipId,
    #[serde(default)]
    target_track_id: Option<TrackId>,
    target_time: RationalTimeArguments,
}

#[derive(Clone, Copy)]
enum TimelineSnapMode {
    Move { duration: RationalTime },
    Trim { edge: TimelineTrimEdge },
}

#[derive(Clone, Copy)]
struct TimelineSnapCandidate {
    time: RationalTime,
    target_kind: TimelineSnapTargetKind,
    track_index: usize,
    clip_index: usize,
    marker_index: usize,
    target_track_id: Option<TrackId>,
    target_clip_id: Option<ClipId>,
    target_marker_id: Option<MarkerId>,
}

#[derive(Clone, Copy)]
struct TimelineSnapChoice {
    candidate: TimelineSnapCandidate,
    distance: RationalTime,
    moving_anchor: TimelineSnapMovingAnchor,
    resolved_target_time: RationalTime,
}

fn resolve_timeline_snap_with_markers(
    project: &ProjectDocument,
    active_clip_id: ClipId,
    mode: TimelineSnapMode,
    raw_target_time: RationalTime,
    marker_aware: bool,
) -> Result<TimelineSnapResult, OperationError> {
    let threshold = RationalTime::new(1, 8)
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
    let (active_track_index, active_clip_index) = find_timeline_clip(project, active_clip_id)
        .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineClipNotFound))?;
    let moving_anchors = match mode {
        TimelineSnapMode::Move { duration } => [
            (TimelineSnapMovingAnchor::Start, raw_target_time),
            (
                TimelineSnapMovingAnchor::End,
                raw_target_time
                    .checked_add(duration)
                    .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?,
            ),
        ],
        TimelineSnapMode::Trim { edge } => {
            [(
                match edge {
                    TimelineTrimEdge::Start => TimelineSnapMovingAnchor::Start,
                    TimelineTrimEdge::End => TimelineSnapMovingAnchor::End,
                },
                raw_target_time,
            ); 2]
        }
    };
    let anchor_count = match mode {
        TimelineSnapMode::Move { .. } => 2,
        TimelineSnapMode::Trim { .. } => 1,
    };

    let mut best = None;
    let timeline_zero = TimelineSnapCandidate {
        time: RationalTime::ZERO,
        target_kind: TimelineSnapTargetKind::TimelineZero,
        track_index: 0,
        clip_index: 0,
        marker_index: 0,
        target_track_id: None,
        target_clip_id: None,
        target_marker_id: None,
    };
    for &(moving_anchor, anchor_time) in moving_anchors.iter().take(anchor_count) {
        consider_timeline_snap_candidate(
            &mut best,
            timeline_zero,
            moving_anchor,
            anchor_time,
            raw_target_time,
            threshold,
        )?;
    }

    for (track_index, track) in project.timeline().tracks().iter().enumerate() {
        for (clip_index, clip) in track.clips().iter().enumerate() {
            if track_index == active_track_index
                && clip_index == active_clip_index
                && clip.id() == active_clip_id
            {
                continue;
            }
            let boundaries = [
                (TimelineSnapTargetKind::ClipStart, clip.timeline_start()),
                (
                    TimelineSnapTargetKind::ClipEnd,
                    clip.timeline_start()
                        .checked_add(clip.timeline_duration())
                        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?,
                ),
            ];
            for (target_kind, time) in boundaries {
                let candidate = TimelineSnapCandidate {
                    time,
                    target_kind,
                    track_index,
                    clip_index,
                    marker_index: 0,
                    target_track_id: Some(track.id()),
                    target_clip_id: Some(clip.id()),
                    target_marker_id: None,
                };
                for &(moving_anchor, anchor_time) in moving_anchors.iter().take(anchor_count) {
                    consider_timeline_snap_candidate(
                        &mut best,
                        candidate,
                        moving_anchor,
                        anchor_time,
                        raw_target_time,
                        threshold,
                    )?;
                }
            }
        }
    }

    if marker_aware {
        for (marker_index, marker) in project.timeline().markers().iter().enumerate() {
            let candidate = TimelineSnapCandidate {
                time: marker.timeline_time(),
                target_kind: TimelineSnapTargetKind::Marker,
                track_index: 0,
                clip_index: 0,
                marker_index,
                target_track_id: None,
                target_clip_id: None,
                target_marker_id: Some(marker.id()),
            };
            for &(moving_anchor, anchor_time) in moving_anchors.iter().take(anchor_count) {
                consider_timeline_snap_candidate(
                    &mut best,
                    candidate,
                    moving_anchor,
                    anchor_time,
                    raw_target_time,
                    threshold,
                )?;
            }
        }
    }

    let Some(choice) = best else {
        return Ok(TimelineSnapResult {
            raw_target_time,
            resolved_target_time: raw_target_time,
            snapped: false,
            moving_anchor: TimelineSnapMovingAnchor::None,
            target_kind: TimelineSnapTargetKind::None,
            target_time: raw_target_time,
            target_track_id: None,
            target_clip_id: None,
            target_marker_id: None,
        });
    };

    let resolved_target_time = match mode {
        TimelineSnapMode::Move { .. } => choice.resolved_target_time,
        TimelineSnapMode::Trim { .. } => choice.candidate.time,
    };
    Ok(TimelineSnapResult {
        raw_target_time,
        resolved_target_time,
        snapped: true,
        moving_anchor: choice.moving_anchor,
        target_kind: choice.candidate.target_kind,
        target_time: choice.candidate.time,
        target_track_id: choice.candidate.target_track_id,
        target_clip_id: choice.candidate.target_clip_id,
        target_marker_id: choice.candidate.target_marker_id,
    })
}

fn consider_timeline_snap_candidate(
    best: &mut Option<TimelineSnapChoice>,
    candidate: TimelineSnapCandidate,
    moving_anchor: TimelineSnapMovingAnchor,
    anchor_time: RationalTime,
    raw_target_time: RationalTime,
    threshold: RationalTime,
) -> Result<(), OperationError> {
    let distance = timeline_snap_distance(anchor_time, candidate.time)?;
    if distance > threshold {
        return Ok(());
    }
    let adjustment = candidate
        .time
        .checked_sub(anchor_time)
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
    let resolved_target_time = raw_target_time
        .checked_add(adjustment)
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
    let choice = TimelineSnapChoice {
        candidate,
        distance,
        moving_anchor,
        resolved_target_time,
    };
    if best.is_none_or(|current| timeline_snap_choice_order(choice, current) == Ordering::Less) {
        *best = Some(choice);
    }
    Ok(())
}

fn timeline_snap_distance(
    left: RationalTime,
    right: RationalTime,
) -> Result<RationalTime, OperationError> {
    let delta = left
        .checked_sub(right)
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
    if delta.is_negative() {
        RationalTime::ZERO
            .checked_sub(delta)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))
    } else {
        Ok(delta)
    }
}

fn timeline_snap_choice_order(left: TimelineSnapChoice, right: TimelineSnapChoice) -> Ordering {
    (
        left.distance,
        left.candidate.time,
        timeline_snap_anchor_priority(left.moving_anchor),
        timeline_snap_source_priority(left.candidate.target_kind),
        left.candidate.track_index,
        left.candidate.clip_index,
        timeline_snap_boundary_priority(left.candidate.target_kind),
        left.candidate.marker_index,
    )
        .cmp(&(
            right.distance,
            right.candidate.time,
            timeline_snap_anchor_priority(right.moving_anchor),
            timeline_snap_source_priority(right.candidate.target_kind),
            right.candidate.track_index,
            right.candidate.clip_index,
            timeline_snap_boundary_priority(right.candidate.target_kind),
            right.candidate.marker_index,
        ))
}

fn timeline_snap_anchor_priority(anchor: TimelineSnapMovingAnchor) -> u8 {
    match anchor {
        TimelineSnapMovingAnchor::Start => 0,
        TimelineSnapMovingAnchor::End => 1,
        TimelineSnapMovingAnchor::None => 2,
    }
}

fn timeline_snap_source_priority(target_kind: TimelineSnapTargetKind) -> u8 {
    match target_kind {
        TimelineSnapTargetKind::TimelineZero => 0,
        TimelineSnapTargetKind::ClipStart | TimelineSnapTargetKind::ClipEnd => 1,
        TimelineSnapTargetKind::Marker => 2,
        TimelineSnapTargetKind::None => 3,
    }
}

fn timeline_snap_boundary_priority(target_kind: TimelineSnapTargetKind) -> u8 {
    match target_kind {
        TimelineSnapTargetKind::ClipStart => 0,
        TimelineSnapTargetKind::ClipEnd => 1,
        TimelineSnapTargetKind::TimelineZero
        | TimelineSnapTargetKind::Marker
        | TimelineSnapTargetKind::None => 0,
    }
}

fn deserialize_canonical_media_id<'de, D>(deserializer: D) -> Result<MediaId, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let id = value.parse::<MediaId>().map_err(serde::de::Error::custom)?;
    if id.to_string() != value {
        return Err(serde::de::Error::custom(
            "media ID must use canonical lowercase UUID text",
        ));
    }
    Ok(id)
}

fn timeline_change_sets(change: ProjectChange) -> Result<(ChangeSet, ChangeSet), OperationError> {
    let mut result_changes = Vec::new();
    result_changes
        .try_reserve(1)
        .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
    let mut history_changes = Vec::new();
    history_changes
        .try_reserve(1)
        .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
    history_changes.push(change.clone());
    result_changes.push(change);
    Ok((
        ChangeSet {
            changes: result_changes,
            ripple_history_guard: None,
        },
        ChangeSet {
            changes: history_changes,
            ripple_history_guard: None,
        },
    ))
}

/// Reconstructs the clip list before a ripple trim from the list after it.
///
/// It returns a `Result` rather than skipping a failed subtraction: a silent skip
/// would put a wrong pre-state into the history entry, and undo would then restore
/// something the project never contained.
fn states_before_ripple(
    after: &[TimelineClipState],
    index: usize,
    before: &TimelineClipState,
) -> Result<Vec<TimelineClipState>, OperationError> {
    let mut states = after.to_vec();
    let Some(current) = states.get(index) else {
        return Err(OperationError::new(OperationErrorCode::InvalidArguments));
    };
    // The trimmed clip's own duration change is the delta the others moved by.
    let delta = timeline_clip_end(current)?
        .checked_sub(timeline_clip_end(before)?)
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
    for state in states.iter_mut().skip(index + 1) {
        state.timeline_start = state
            .timeline_start
            .checked_sub(delta)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
    }
    states[index] = before.clone();
    Ok(states)
}

fn history_conflict() -> OperationError {
    OperationError::new(OperationErrorCode::HistoryConflict)
}

fn timeline_clip_end(clip: &TimelineClipState) -> Result<RationalTime, OperationError> {
    clip.timeline_start
        .checked_add(clip.timeline_duration)
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))
}

fn track_states(
    project: &ProjectDocument,
    track_index: usize,
) -> Result<Vec<TimelineClipState>, OperationError> {
    let clips = project.timeline().tracks()[track_index].clips();
    let mut states = Vec::new();
    states
        .try_reserve(clips.len())
        .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
    states.extend(clips.iter().map(TimelineClipState::from));
    Ok(states)
}

fn states_into_domain(states: Vec<TimelineClipState>) -> Result<Vec<TimelineClip>, OperationError> {
    let mut clips = Vec::new();
    clips
        .try_reserve(states.len())
        .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
    clips.extend(states.into_iter().map(TimelineClipState::into_domain));
    Ok(clips)
}

fn timeline_states_fingerprint(states: &[TimelineClipState]) -> u64 {
    let bytes = serde_json::to_vec(states).expect("timeline state serialization is infallible");
    bytes
        .iter()
        .fold(14_695_981_039_346_656_037_u64, |hash, byte| {
            hash.wrapping_mul(1_099_511_628_211)
                .wrapping_add(u64::from(*byte))
        })
}

fn validate_track_states(
    project: &ProjectDocument,
    track_kind: TrackKind,
    states: &[TimelineClipState],
) -> Result<(), OperationError> {
    let mut previous_start = None;
    let mut previous_end = None;
    for state in states {
        let end = validate_clip_state(project, track_kind, state)?;
        if previous_start.is_some_and(|start| state.timeline_start <= start)
            || previous_end.is_some_and(|previous| state.timeline_start < previous)
        {
            return Err(OperationError::new(OperationErrorCode::TimelineOverlap));
        }
        previous_start = Some(state.timeline_start);
        previous_end = Some(end);
    }
    Ok(())
}

fn stage_track_history_change(
    project: &ProjectDocument,
    track_id: TrackId,
    kind: TrackKind,
    state: TrackState,
    index: usize,
    reverse: bool,
    was_added: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let remove = reverse == was_added;
    if remove {
        ensure_empty_track_at(project, track_id, kind, state, index)?;
        Ok((
            StagedHistoryChange::RemoveTimelineTrack { index },
            ChangeSet::try_single(ProjectChange::TimelineTrackRemoved {
                track_id,
                track_kind: kind,
                track_state: state,
                index,
            })?,
        ))
    } else {
        ensure_track_insertable_for_history(project, track_id, index)?;
        Ok((
            StagedHistoryChange::InsertTimelineTrack {
                track_id,
                kind,
                state,
                index,
            },
            ChangeSet::try_single(ProjectChange::TimelineTrackAdded {
                track_id,
                track_kind: kind,
                track_state: state,
                index,
            })?,
        ))
    }
}

fn stage_caption_track_history_change(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    clips: &[TimelineClipState],
    reverse: bool,
    was_added: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let remove = reverse == was_added;
    let mut domain_clips = Vec::new();
    domain_clips
        .try_reserve(clips.len())
        .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
    domain_clips.extend(clips.iter().cloned().map(TimelineClipState::into_domain));
    let track = TimelineTrack::from_parts_for_codec(track_id, TrackKind::Caption, domain_clips);

    if remove {
        if project.timeline().tracks().get(index) != Some(&track) {
            return Err(history_conflict());
        }
        let change = if was_added {
            ProjectChange::TimelineCaptionTrackRemoved {
                track_id,
                index,
                clips: clips.to_vec(),
            }
        } else {
            ProjectChange::TimelineCaptionTrackAdded {
                track_id,
                index,
                clips: clips.to_vec(),
            }
        };
        Ok((
            StagedHistoryChange::RemoveTimelineTrack { index },
            ChangeSet::try_single(change)?,
        ))
    } else {
        let tracks = project.timeline().tracks();
        if index > tracks.len()
            || tracks.len() >= crate::MAX_TIMELINE_TRACKS
            || tracks.iter().any(|candidate| candidate.id() == track_id)
            || clips
                .iter()
                .any(|clip| find_timeline_clip(project, clip.clip_id).is_some())
        {
            return Err(history_conflict());
        }
        let existing_count = timeline_clip_count(project).ok_or_else(history_conflict)?;
        if existing_count
            .checked_add(clips.len())
            .is_none_or(|count| count > crate::MAX_TIMELINE_CLIPS)
        {
            return Err(history_conflict());
        }
        validate_track_states(project, TrackKind::Caption, clips)
            .map_err(|_| history_conflict())?;
        let change = if was_added {
            ProjectChange::TimelineCaptionTrackAdded {
                track_id,
                index,
                clips: clips.to_vec(),
            }
        } else {
            ProjectChange::TimelineCaptionTrackRemoved {
                track_id,
                index,
                clips: clips.to_vec(),
            }
        };
        Ok((
            StagedHistoryChange::InsertCaptionTrack { track, index },
            ChangeSet::try_single(change)?,
        ))
    }
}

fn ensure_empty_track_at(
    project: &ProjectDocument,
    track_id: TrackId,
    kind: TrackKind,
    state: TrackState,
    index: usize,
) -> Result<(), OperationError> {
    match project.timeline().tracks().get(index) {
        Some(track)
            if track.id() == track_id
                && track.kind() == kind
                && track.state() == state
                && track.clips().is_empty() =>
        {
            Ok(())
        }
        _ => Err(history_conflict()),
    }
}

fn stage_track_state_history_change(
    project: &ProjectDocument,
    track_id: TrackId,
    before: TrackState,
    after: TrackState,
    reverse: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let track_index = find_timeline_track_index(project, track_id).ok_or_else(history_conflict)?;
    let current = project.timeline().tracks()[track_index].state();
    let (expected, target) = if reverse {
        (after, before)
    } else {
        (before, after)
    };
    if current != expected {
        return Err(history_conflict());
    }
    Ok((
        StagedHistoryChange::SetTimelineTrackState {
            track_index,
            state: target,
        },
        ChangeSet::try_single(ProjectChange::TimelineTrackStateChanged {
            track_id,
            before: expected,
            after: target,
        })?,
    ))
}

fn ensure_track_insertable_for_history(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
) -> Result<(), OperationError> {
    let tracks = project.timeline().tracks();
    if index > tracks.len()
        || tracks.len() >= crate::MAX_TIMELINE_TRACKS
        || tracks.iter().any(|track| track.id() == track_id)
    {
        Err(history_conflict())
    } else {
        Ok(())
    }
}

fn stage_clip_history_change(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    clip: &TimelineClipState,
    reverse: bool,
    was_inserted: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let remove = reverse == was_inserted;
    if remove {
        let track_index = ensure_clip_matches_at(project, track_id, index, clip)?;
        Ok((
            StagedHistoryChange::RemoveTimelineClip { track_index, index },
            ChangeSet::try_single(ProjectChange::TimelineClipDeleted {
                track_id,
                index,
                clip: clip.clone(),
            })?,
        ))
    } else {
        let track_index = ensure_clip_insertable_at(project, track_id, index, clip)?;
        Ok((
            StagedHistoryChange::InsertTimelineClip {
                track_index,
                index,
                clip: clip.clone(),
            },
            ChangeSet::try_single(ProjectChange::TimelineClipInserted {
                track_id,
                index,
                clip: clip.clone(),
            })?,
        ))
    }
}

fn ensure_clip_matches_at(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    expected: &TimelineClipState,
) -> Result<usize, OperationError> {
    let track_index = find_timeline_track_index(project, track_id).ok_or_else(history_conflict)?;
    let Some(clip) = project.timeline().tracks()[track_index].clips().get(index) else {
        return Err(history_conflict());
    };
    if &TimelineClipState::from(clip) == expected {
        Ok(track_index)
    } else {
        Err(history_conflict())
    }
}

fn stage_clip_trim_history_change(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    before: &TimelineClipState,
    after: &TimelineClipState,
    reverse: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let (expected, target) = if reverse {
        (after, before)
    } else {
        (before, after)
    };
    let track_index = ensure_clip_matches_at(project, track_id, index, expected)?;
    let track = &project.timeline().tracks()[track_index];
    let mut states = track_states(project, track_index).map_err(|_| history_conflict())?;
    states[index] = target.clone();
    validate_track_states(project, track.kind(), &states).map_err(|_| history_conflict())?;
    Ok((
        StagedHistoryChange::ReplaceTimelineClip {
            track_index,
            index,
            clip: target.clone(),
        },
        ChangeSet::try_single(ProjectChange::TimelineClipTrimmed {
            track_id,
            index,
            before: expected.clone(),
            after: target.clone(),
        })?,
    ))
}

fn stage_clip_update_history_change(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    before: &TimelineClipState,
    after: &TimelineClipState,
    reverse: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let (expected, target) = if reverse {
        (after, before)
    } else {
        (before, after)
    };
    let track_index = ensure_clip_matches_at(project, track_id, index, expected)?;
    let track = &project.timeline().tracks()[track_index];
    let mut states = track_states(project, track_index).map_err(|_| history_conflict())?;
    states[index] = target.clone();
    validate_track_states(project, track.kind(), &states).map_err(|_| history_conflict())?;
    Ok((
        StagedHistoryChange::ReplaceTimelineClip {
            track_index,
            index,
            clip: target.clone(),
        },
        ChangeSet::try_single(ProjectChange::TimelineClipUpdated {
            track_id,
            index,
            before: expected.clone(),
            after: target.clone(),
        })?,
    ))
}

fn stage_clip_split_history_change(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    before: &TimelineClipState,
    left_after: &TimelineClipState,
    right_after: &TimelineClipState,
    reverse: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let track_index = find_timeline_track_index(project, track_id).ok_or_else(history_conflict)?;
    let track = &project.timeline().tracks()[track_index];
    let mut states = track_states(project, track_index).map_err(|_| history_conflict())?;
    if reverse {
        if states.get(index) != Some(left_after) || states.get(index + 1) != Some(right_after) {
            return Err(history_conflict());
        }
        states.remove(index + 1);
        states[index] = before.clone();
    } else {
        if states.get(index) != Some(before)
            || find_timeline_clip(project, right_after.clip_id).is_some()
            || timeline_clip_count(project).is_none_or(|count| count >= crate::MAX_TIMELINE_CLIPS)
            || states.len() >= crate::MAX_TIMELINE_CLIPS_PER_TRACK
        {
            return Err(history_conflict());
        }
        states[index] = left_after.clone();
        states.try_reserve(1).map_err(|_| history_conflict())?;
        states.insert(index + 1, right_after.clone());
    }
    validate_track_states(project, track.kind(), &states).map_err(|_| history_conflict())?;
    let clips = states_into_domain(states).map_err(|_| history_conflict())?;
    Ok((
        StagedHistoryChange::ReplaceTimelineTrack { track_index, clips },
        ChangeSet::try_single(ProjectChange::TimelineClipSplit {
            track_id,
            index,
            before: before.clone(),
            left_after: left_after.clone(),
            right_after: right_after.clone(),
        })?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn stage_clip_ripple_history_change(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    deleted_clip: &TimelineClipState,
    shifted_count: usize,
    shift_duration: RationalTime,
    reverse: bool,
    guard: Option<RippleHistoryGuard>,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let track_index = find_timeline_track_index(project, track_id).ok_or_else(history_conflict)?;
    let track = &project.timeline().tracks()[track_index];
    let mut states = track_states(project, track_index).map_err(|_| history_conflict())?;
    if let Some(guard) = guard {
        let expected_fingerprint = if reverse {
            guard.after_fingerprint
        } else {
            guard.before_fingerprint
        };
        if timeline_states_fingerprint(&states) != expected_fingerprint {
            return Err(history_conflict());
        }
    }
    if reverse {
        let expected_len = index
            .checked_add(shifted_count)
            .ok_or_else(history_conflict)?;
        if states.len() != expected_len
            || timeline_clip_count(project).is_none_or(|count| count >= crate::MAX_TIMELINE_CLIPS)
            || find_timeline_clip(project, deleted_clip.clip_id).is_some()
        {
            return Err(history_conflict());
        }
        states.try_reserve(1).map_err(|_| history_conflict())?;
        states.insert(index, deleted_clip.clone());
        for state in states.iter_mut().skip(index + 1) {
            state.timeline_start = state
                .timeline_start
                .checked_add(shift_duration)
                .map_err(|_| history_conflict())?;
        }
    } else {
        let suffix_len = shifted_count.checked_add(1).ok_or_else(history_conflict)?;
        let expected_len = index.checked_add(suffix_len).ok_or_else(history_conflict)?;
        if states.len() != expected_len || states.get(index) != Some(deleted_clip) {
            return Err(history_conflict());
        }
        states.remove(index);
        for state in states.iter_mut().skip(index) {
            state.timeline_start = state
                .timeline_start
                .checked_sub(shift_duration)
                .map_err(|_| history_conflict())?;
        }
    }
    validate_track_states(project, track.kind(), &states).map_err(|_| history_conflict())?;
    if let Some(guard) = guard {
        let expected_fingerprint = if reverse {
            guard.before_fingerprint
        } else {
            guard.after_fingerprint
        };
        if timeline_states_fingerprint(&states) != expected_fingerprint {
            return Err(history_conflict());
        }
    }
    let clips = states_into_domain(states).map_err(|_| history_conflict())?;
    Ok((
        StagedHistoryChange::ReplaceTimelineTrack { track_index, clips },
        ChangeSet::try_single(ProjectChange::TimelineClipRippleDeleted {
            track_id,
            index,
            deleted_clip: deleted_clip.clone(),
            shifted_count,
            shift_duration,
        })?,
    ))
}

fn ensure_clip_insertable_at(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    clip: &TimelineClipState,
) -> Result<usize, OperationError> {
    let track_index = find_timeline_track_index(project, track_id).ok_or_else(history_conflict)?;
    if find_timeline_clip(project, clip.clip_id).is_some()
        || timeline_clip_count(project).is_none_or(|count| count >= crate::MAX_TIMELINE_CLIPS)
        || project.timeline().tracks()[track_index].clips().len()
            >= crate::MAX_TIMELINE_CLIPS_PER_TRACK
    {
        return Err(history_conflict());
    }
    let actual_index =
        clip_insertion_index(project, track_index, clip, None).map_err(|_| history_conflict())?;
    if actual_index == index {
        Ok(track_index)
    } else {
        Err(history_conflict())
    }
}

fn stage_marker_history_change(
    project: &ProjectDocument,
    marker: &TimelineMarkerState,
    index: usize,
    reverse: bool,
    was_added: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let remove = reverse == was_added;
    if remove {
        ensure_marker_matches_at(project, index, marker)?;
        Ok((
            StagedHistoryChange::RemoveTimelineMarker { index },
            ChangeSet::try_single(ProjectChange::TimelineMarkerDeleted {
                marker: marker.clone(),
                index,
            })?,
        ))
    } else {
        ensure_marker_insertable_at(project, index, marker)?;
        Ok((
            StagedHistoryChange::InsertTimelineMarker {
                index,
                marker: marker.clone(),
            },
            ChangeSet::try_single(ProjectChange::TimelineMarkerAdded {
                marker: marker.clone(),
                index,
            })?,
        ))
    }
}

fn ensure_marker_matches_at(
    project: &ProjectDocument,
    index: usize,
    expected: &TimelineMarkerState,
) -> Result<(), OperationError> {
    if project
        .timeline()
        .markers()
        .get(index)
        .is_some_and(|marker| TimelineMarkerState::from(marker) == *expected)
    {
        Ok(())
    } else {
        Err(history_conflict())
    }
}

fn ensure_marker_insertable_at(
    project: &ProjectDocument,
    index: usize,
    marker: &TimelineMarkerState,
) -> Result<(), OperationError> {
    if index > project.timeline().markers().len()
        || project.timeline().markers().len() >= MAX_TIMELINE_MARKERS
        || find_timeline_marker(project, marker.marker_id).is_some()
        || marker_insertion_index(project, marker.marker_id, marker.timeline_time, None) != index
    {
        Err(history_conflict())
    } else {
        Ok(())
    }
}

fn stage_marker_move_history_change(
    project: &ProjectDocument,
    marker_id: MarkerId,
    label: &str,
    from: (RationalTime, usize),
    to: (RationalTime, usize),
    reverse: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let (current, target) = if reverse { (to, from) } else { (from, to) };
    let Some(marker) = project.timeline().markers().get(current.1) else {
        return Err(history_conflict());
    };
    if marker.id() != marker_id
        || marker.timeline_time() != current.0
        || marker.label() != label
        || marker_insertion_index(project, marker_id, target.0, Some(marker_id)) != target.1
    {
        return Err(history_conflict());
    }
    Ok((
        StagedHistoryChange::MoveTimelineMarker {
            from_index: current.1,
            to_index: target.1,
            to_time: target.0,
        },
        ChangeSet::try_single(ProjectChange::TimelineMarkerMoved {
            marker_id,
            label: label.to_owned(),
            from_time: current.0,
            from_index: current.1,
            to_time: target.0,
            to_index: target.1,
        })?,
    ))
}

fn stage_marker_rename_history_change(
    project: &ProjectDocument,
    marker_id: MarkerId,
    time: RationalTime,
    before_label: &str,
    after_label: &str,
    reverse: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let (expected, target) = if reverse {
        (after_label, before_label)
    } else {
        (before_label, after_label)
    };
    let Some((index, marker)) = project
        .timeline()
        .markers()
        .iter()
        .enumerate()
        .find(|(_, marker)| marker.id() == marker_id)
    else {
        return Err(history_conflict());
    };
    if marker.timeline_time() != time || marker.label() != expected {
        return Err(history_conflict());
    }
    Ok((
        StagedHistoryChange::RenameTimelineMarker {
            index,
            label: target.to_owned(),
        },
        ChangeSet::try_single(ProjectChange::TimelineMarkerRenamed {
            marker_id,
            time,
            before_label: expected.to_owned(),
            after_label: target.to_owned(),
        })?,
    ))
}

fn stage_clip_move_history_change(
    project: &ProjectDocument,
    from: (TrackId, usize),
    to: (TrackId, usize),
    before: &TimelineClipState,
    after: &TimelineClipState,
    reverse: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let (
        current_track_id,
        current_index,
        current_state,
        target_track_id,
        target_index,
        target_state,
    ) = if reverse {
        (to.0, to.1, after, from.0, from.1, before)
    } else {
        (from.0, from.1, before, to.0, to.1, after)
    };
    let current_track_index =
        find_timeline_track_index(project, current_track_id).ok_or_else(history_conflict)?;
    let target_track_index =
        find_timeline_track_index(project, target_track_id).ok_or_else(history_conflict)?;
    ensure_clip_matches_at(project, current_track_id, current_index, current_state)?;
    let tracks = project.timeline().tracks();
    if tracks[current_track_index].kind() != tracks[target_track_index].kind() {
        return Err(history_conflict());
    }
    let actual_index = clip_insertion_index(
        project,
        target_track_index,
        target_state,
        Some(target_state.clip_id),
    )
    .map_err(|_| history_conflict())?;
    if actual_index != target_index
        || (current_track_index != target_track_index
            && tracks[target_track_index].clips().len() >= crate::MAX_TIMELINE_CLIPS_PER_TRACK)
    {
        return Err(history_conflict());
    }
    Ok((
        StagedHistoryChange::MoveTimelineClip {
            from_track_index: current_track_index,
            from_index: current_index,
            to_track_index: target_track_index,
            to_index: target_index,
            to_timeline_start: target_state.timeline_start,
        },
        ChangeSet::try_single(ProjectChange::TimelineClipMoved {
            from_track_id: current_track_id,
            from_index: current_index,
            to_track_id: target_track_id,
            to_index: target_index,
            before: current_state.clone(),
            after: target_state.clone(),
        })?,
    ))
}

fn find_timeline_track_index(project: &ProjectDocument, track_id: TrackId) -> Option<usize> {
    project
        .timeline()
        .tracks()
        .iter()
        .position(|track| track.id() == track_id)
}

fn find_timeline_clip(project: &ProjectDocument, clip_id: ClipId) -> Option<(usize, usize)> {
    project
        .timeline()
        .tracks()
        .iter()
        .enumerate()
        .find_map(|(track_index, track)| {
            track
                .clips()
                .iter()
                .position(|clip| clip.id() == clip_id)
                .map(|clip_index| (track_index, clip_index))
        })
}

fn find_timeline_marker(project: &ProjectDocument, marker_id: MarkerId) -> Option<usize> {
    project
        .timeline()
        .markers()
        .iter()
        .position(|marker| marker.id() == marker_id)
}

fn marker_insertion_index(
    project: &ProjectDocument,
    marker_id: MarkerId,
    timeline_time: RationalTime,
    excluded_marker_id: Option<MarkerId>,
) -> usize {
    let mut index = 0;
    for marker in project.timeline().markers() {
        if Some(marker.id()) == excluded_marker_id {
            continue;
        }
        if marker.timeline_time() < timeline_time
            || (marker.timeline_time() == timeline_time && marker.id() < marker_id)
        {
            index += 1;
        } else {
            break;
        }
    }
    index
}

fn validate_marker_time(time: RationalTime) -> Result<(), OperationError> {
    if time.is_negative() {
        Err(OperationError::new(OperationErrorCode::InvalidArguments))
    } else {
        Ok(())
    }
}

fn validate_marker_label(label: &str) -> Result<(), OperationError> {
    if label.trim().is_empty() || label.len() > MAX_TIMELINE_MARKER_LABEL_BYTES {
        Err(OperationError::new(OperationErrorCode::InvalidArguments))
    } else {
        Ok(())
    }
}

fn timeline_clip_count(project: &ProjectDocument) -> Option<usize> {
    project
        .timeline()
        .tracks()
        .iter()
        .try_fold(0usize, |count, track| {
            count.checked_add(track.clips().len())
        })
}

fn ensure_timeline_clip_capacity(
    project: &ProjectDocument,
    track_index: usize,
    additional_total: usize,
    additional_track: usize,
    max_total: usize,
    max_per_track: usize,
) -> Result<(), OperationError> {
    let total = timeline_clip_count(project).and_then(|count| count.checked_add(additional_total));
    let track_count = project.timeline().tracks()[track_index]
        .clips()
        .len()
        .checked_add(additional_track);
    if total.is_none_or(|count| count > max_total)
        || track_count.is_none_or(|count| count > max_per_track)
    {
        Err(OperationError::new(
            OperationErrorCode::TimelineLimitExceeded,
        ))
    } else {
        Ok(())
    }
}

fn validate_clip_state(
    project: &ProjectDocument,
    track_kind: TrackKind,
    clip: &TimelineClipState,
) -> Result<RationalTime, OperationError> {
    if clip.timeline_start.is_negative()
        || !clip.timeline_duration.is_positive()
        || !clip
            .settings
            .is_valid_for(track_kind, clip.timeline_duration)
    {
        return Err(OperationError::new(OperationErrorCode::InvalidArguments));
    }
    match (&clip.content, track_kind) {
        (
            ClipContent::Media {
                media_id,
                source_range,
                playback_rate,
                reversed: _,
                frozen: _,
            },
            TrackKind::Video | TrackKind::Audio,
        ) => {
            if source_range.start().is_negative()
                || !source_range.duration().is_positive()
                || playback_rate.numerator() > 16 * playback_rate.denominator()
                || playback_rate.denominator() > 16 * playback_rate.numerator()
                || playback_rate.apply(clip.timeline_duration).ok() != Some(source_range.duration())
            {
                return Err(OperationError::new(OperationErrorCode::InvalidArguments));
            }
            let source_end = source_range
                .start()
                .checked_add(source_range.duration())
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
            let media = project
                .media_items()
                .iter()
                .find(|item| item.id() == *media_id)
                .ok_or_else(|| OperationError::new(OperationErrorCode::MediaNotFound))?;
            let (compatible, stream_duration) = crate::timeline::matching_stream(track_kind, media);
            if !compatible {
                return Err(OperationError::new(
                    OperationErrorCode::TimelineMediaIncompatible,
                ));
            }
            if stream_duration
                .or(media.metadata().duration())
                .is_some_and(|duration| source_end > duration)
            {
                return Err(OperationError::new(OperationErrorCode::InvalidArguments));
            }
        }
        (ClipContent::Text { text, formatting }, TrackKind::Text)
        | (ClipContent::Caption { text, formatting }, TrackKind::Caption) => {
            let max_bytes = if track_kind == TrackKind::Caption {
                crate::MAX_TIMELINE_CAPTION_BYTES
            } else {
                crate::MAX_TIMELINE_TEXT_BYTES
            };
            if text.trim().is_empty() || text.len() > max_bytes || !formatting.is_valid() {
                return Err(OperationError::new(OperationErrorCode::InvalidArguments));
            }
        }
        _ => return Err(OperationError::new(OperationErrorCode::InvalidArguments)),
    }
    clip.timeline_start
        .checked_add(clip.timeline_duration)
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))
}

fn clip_insertion_index(
    project: &ProjectDocument,
    track_index: usize,
    clip: &TimelineClipState,
    excluded_clip_id: Option<ClipId>,
) -> Result<usize, OperationError> {
    let track = &project.timeline().tracks()[track_index];
    let clip_end = validate_clip_state(project, track.kind(), clip)?;
    let mut insertion_index = 0;
    for existing in track.clips() {
        if Some(existing.id()) == excluded_clip_id {
            continue;
        }
        let existing_end = existing
            .timeline_start()
            .checked_add(existing.timeline_duration())
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if clip.timeline_start < existing_end && existing.timeline_start() < clip_end {
            return Err(OperationError::new(OperationErrorCode::TimelineOverlap));
        }
        if existing.timeline_start() < clip.timeline_start {
            insertion_index += 1;
        } else {
            return Ok(insertion_index);
        }
    }
    Ok(insertion_index)
}

fn trim_content_start(
    content: &ClipContent,
    delta: RationalTime,
    duration: RationalTime,
) -> Result<ClipContent, OperationError> {
    match content {
        ClipContent::Media {
            media_id,
            source_range,
            playback_rate,
            reversed: _,
            frozen: _,
        } => {
            let source_start = source_range
                .start()
                .checked_add(delta)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
            let source_range = TimeRange::new(source_start, duration)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
            Ok(ClipContent::Media {
                media_id: *media_id,
                source_range,
                playback_rate: *playback_rate,
                reversed: false,
                frozen: false,
            })
        }
        _ => Ok(content.clone()),
    }
}

fn trim_content_end(
    content: &ClipContent,
    duration: RationalTime,
) -> Result<ClipContent, OperationError> {
    match content {
        ClipContent::Media {
            media_id,
            source_range,
            playback_rate,
            reversed: _,
            frozen: _,
        } => {
            let source_range = TimeRange::new(source_range.start(), duration)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
            Ok(ClipContent::Media {
                media_id: *media_id,
                source_range,
                playback_rate: *playback_rate,
                reversed: false,
                frozen: false,
            })
        }
        _ => Ok(content.clone()),
    }
}

fn split_content(
    content: &ClipContent,
    left_duration: RationalTime,
    right_duration: RationalTime,
) -> Result<(ClipContent, ClipContent), OperationError> {
    match content {
        ClipContent::Media {
            media_id,
            source_range,
            playback_rate,
            reversed,
            frozen,
        } => {
            let right_start = source_range
                .start()
                .checked_add(left_duration)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
            Ok((
                ClipContent::Media {
                    media_id: *media_id,
                    source_range: TimeRange::new(source_range.start(), left_duration)
                        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?,
                    playback_rate: *playback_rate,
                    reversed: *reversed,
                    frozen: *frozen,
                },
                ClipContent::Media {
                    media_id: *media_id,
                    source_range: TimeRange::new(right_start, right_duration)
                        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?,
                    playback_rate: *playback_rate,
                    reversed: *reversed,
                    frozen: *frozen,
                },
            ))
        }
        _ => Ok((content.clone(), content.clone())),
    }
}

fn is_empty_object(arguments: &Value) -> bool {
    arguments
        .as_object()
        .is_some_and(|arguments| arguments.is_empty())
}

fn ensure_media_id_available(project: &ProjectDocument, id: MediaId) -> Result<(), OperationError> {
    if project.media_items().iter().any(|item| item.id() == id) {
        Err(OperationError::new(
            OperationErrorCode::MediaIdAlreadyExists,
        ))
    } else {
        Ok(())
    }
}

fn ensure_media_source_available(
    project: &ProjectDocument,
    source: &crate::MediaSourceRef,
) -> Result<(), OperationError> {
    if project
        .media_items()
        .iter()
        .any(|item| item.source() == source)
    {
        Err(OperationError::new(
            OperationErrorCode::MediaSourceAlreadyExists,
        ))
    } else {
        Ok(())
    }
}

fn ensure_media_matches_at(
    project: &ProjectDocument,
    index: usize,
    item: &MediaItem,
) -> Result<(), OperationError> {
    if project.media_items().get(index) == Some(item) {
        Ok(())
    } else {
        Err(OperationError::new(OperationErrorCode::HistoryConflict))
    }
}

fn ensure_media_insertable(
    project: &ProjectDocument,
    index: usize,
    item: &MediaItem,
) -> Result<(), OperationError> {
    if index > project.media_items().len()
        || project
            .media_items()
            .iter()
            .any(|current| current.id() == item.id() || current.source() == item.source())
    {
        Err(OperationError::new(OperationErrorCode::HistoryConflict))
    } else {
        Ok(())
    }
}

fn stage_groupable_command(
    command: &CommandCall,
    staged_name: &mut String,
) -> Result<(), OperationError> {
    let descriptor = COMMANDS
        .iter()
        .find(|descriptor| descriptor.id == command.command_id.as_str())
        .ok_or_else(|| OperationError::new(OperationErrorCode::UnknownCommand))?;
    let schema_supported = if descriptor.id == TIMELINE_TRACK_ADD_ID {
        matches!(command.schema_version, 1 | 2)
    } else {
        command.schema_version == descriptor.schema_version
    };
    if !schema_supported {
        return Err(OperationError::new(
            OperationErrorCode::UnsupportedCommandSchema,
        ));
    }
    if !descriptor.allowed_in_transaction {
        return Err(OperationError::new(
            OperationErrorCode::CommandNotAllowedInTransaction,
        ));
    }

    let arguments: RenameArguments = serde_json::from_value(command.arguments.clone())
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
    *staged_name = arguments.name;
    Ok(())
}

fn parse_empty_arguments(arguments: Value) -> Result<(), OperationError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct EmptyArguments {}

    serde_json::from_value::<EmptyArguments>(arguments)
        .map(|_| ())
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameArguments {
    name: String,
}
#[cfg(test)]
mod tests {
    use super::{
        ApplicationRequest, ApplicationResponse, COMMANDS, CURRENT_TRANSACTION_SCHEMA_VERSION,
        ChangeSet, CommandCall, CommandDescriptor, CommandEnvelope, OperationErrorCode,
        ProjectChange, ProjectSession, QUERIES, QueryDescriptor, QueryEnvelope, TimelineClipState,
        TimelineSnapMovingAnchor, TimelineSnapTargetKind, TimelineTrackSummary,
        TransactionEnvelope, command_catalog, query_catalog,
    };
    use crate::{
        AudioStreamMetadata, ClipContent, ClipId, ClipSettings, Crop, MAX_MEDIA_PAGE_SIZE,
        MAX_TIMELINE_CLIP_PAGE_SIZE, MAX_TIMELINE_CLIPS, MAX_TIMELINE_MARKER_LABEL_BYTES,
        MAX_TIMELINE_MARKER_PAGE_SIZE, MAX_TIMELINE_MARKERS, MAX_TIMELINE_TRACKS, MarkerId,
        MediaId, MediaItem, MediaMetadata, MediaSourceRef, MediaStreamMetadata, Opacity,
        ProjectDocument, ProjectId, ProjectInstanceId, ProjectRevision, ProjectTimeline,
        RationalRate, RationalTime, TextFormatting, TimeRange, TimelineClip, TimelineMarker,
        TimelineTrack, TrackId, TrackKind, TrackState, Transform, VideoStreamMetadata,
        VisualSettings, decode_project, encode_project,
    };
    use serde_json::{Value, json};
    use std::{num::NonZeroU32, str::FromStr};

    const PROJECT_ID: &str = "01234567-89ab-4def-8123-456789abcdef";
    const OTHER_PROJECT_ID: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    const INSTANCE_ID: &str = "fedcba98-7654-4cba-8fed-cba987654321";
    const OTHER_INSTANCE_ID: &str = "11111111-1111-4111-8111-111111111111";
    const TRACK_A: &str = "22222222-2222-4222-8222-222222222222";
    const TRACK_B: &str = "88888888-8888-4888-8888-888888888888";
    const TRACK_C: &str = "99999999-9999-4999-8999-999999999999";
    const TRACK_D: &str = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";
    const CLIP_A: &str = "33333333-3333-4333-8333-333333333333";
    const CLIP_B: &str = "55555555-5555-4555-8555-555555555555";
    const CLIP_C: &str = "66666666-6666-4666-8666-666666666666";
    const CLIP_D: &str = "99999999-9999-4999-8999-999999999998";
    const CLIP_E: &str = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeef";
    const MEDIA_A: &str = "44444444-4444-4444-8444-444444444444";
    const MEDIA_B: &str = "77777777-7777-4777-8777-777777777777";
    const MEDIA_C: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaab";
    const MEDIA_D: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    const MARKER_A: &str = "11111111-1111-4111-8111-111111111111";
    const MARKER_B: &str = "33333333-3333-4333-8333-333333333333";
    const MARKER_C: &str = "55555555-5555-4555-8555-555555555555";

    fn project() -> ProjectDocument {
        decode_project(&format!(
            r#"{{"format":"opencut-reinforced-project","schema_version":1,"project":{{"id":"{PROJECT_ID}","revision":0,"name":"A"}}}}"#
        ))
        .unwrap()
    }

    fn fixed_session() -> ProjectSession {
        ProjectSession {
            project: project(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            history: super::SessionHistory::default(),
        }
    }

    fn media_item(id: &str, uri: &str) -> MediaItem {
        MediaItem::new(
            MediaId::from_str(id).unwrap(),
            MediaSourceRef::local_file(uri).unwrap(),
            MediaMetadata::from_probe(
                vec!["matroska".to_owned()],
                None,
                42,
                vec![MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                    0,
                    None,
                    NonZeroU32::new(1920).unwrap(),
                    NonZeroU32::new(1080).unwrap(),
                    None,
                    Some(RationalRate::new(24, 1).unwrap()),
                    None,
                ))],
            ),
        )
        .unwrap()
    }

    fn audio_media_item(id: &str, uri: &str) -> MediaItem {
        MediaItem::new(
            MediaId::from_str(id).unwrap(),
            MediaSourceRef::local_file(uri).unwrap(),
            MediaMetadata::from_probe(
                vec!["wav".to_owned()],
                None,
                42,
                vec![MediaStreamMetadata::Audio(AudioStreamMetadata::from_probe(
                    0, None, None, None, None, None,
                ))],
            ),
        )
        .unwrap()
    }

    fn video_media_item_with_duration(
        id: &str,
        uri: &str,
        container_duration: Option<RationalTime>,
        stream_duration: Option<RationalTime>,
    ) -> MediaItem {
        MediaItem::new(
            MediaId::from_str(id).unwrap(),
            MediaSourceRef::local_file(uri).unwrap(),
            MediaMetadata::from_probe(
                vec!["matroska".to_owned()],
                container_duration,
                42,
                vec![MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                    0,
                    None,
                    NonZeroU32::new(1920).unwrap(),
                    NonZeroU32::new(1080).unwrap(),
                    None,
                    Some(RationalRate::new(24, 1).unwrap()),
                    stream_duration,
                ))],
            ),
        )
        .unwrap()
    }

    fn audio_video_media_item(id: &str, uri: &str) -> MediaItem {
        MediaItem::new(
            MediaId::from_str(id).unwrap(),
            MediaSourceRef::local_file(uri).unwrap(),
            MediaMetadata::from_probe(
                vec!["matroska".to_owned()],
                None,
                42,
                vec![
                    MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                        0,
                        None,
                        NonZeroU32::new(1920).unwrap(),
                        NonZeroU32::new(1080).unwrap(),
                        None,
                        Some(RationalRate::new(24, 1).unwrap()),
                        None,
                    )),
                    MediaStreamMetadata::Audio(AudioStreamMetadata::from_probe(
                        1, None, None, None, None, None,
                    )),
                ],
            ),
        )
        .unwrap()
    }

    fn media_add(item: MediaItem, revision: u64) -> CommandEnvelope {
        CommandEnvelope::add_media(
            ProjectId::from_str(PROJECT_ID).unwrap(),
            ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            ProjectRevision::new(revision),
            item,
        )
    }

    fn media_remove(id: &str, revision: u64) -> CommandEnvelope {
        CommandEnvelope::remove_media(
            ProjectId::from_str(PROJECT_ID).unwrap(),
            ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            ProjectRevision::new(revision),
            MediaId::from_str(id).unwrap(),
        )
    }

    fn media_relink(item: MediaItem, revision: u64) -> CommandEnvelope {
        CommandEnvelope::relink_media(
            ProjectId::from_str(PROJECT_ID).unwrap(),
            ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            ProjectRevision::new(revision),
            item,
        )
    }

    fn session_with_state(name: &str, revision: u64) -> ProjectSession {
        let project = decode_project(&format!(
            r#"{{"format":"opencut-reinforced-project","schema_version":1,"project":{{"id":"{PROJECT_ID}","revision":{revision},"name":"{name}"}}}}"#
        ))
        .unwrap();
        ProjectSession {
            project,
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            history: super::SessionHistory::default(),
        }
    }

    fn rename(name: &str, revision: u64) -> CommandEnvelope {
        CommandEnvelope {
            command_id: "project.rename".to_owned(),
            schema_version: 1,
            project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            expected_project_revision: ProjectRevision::new(revision),
            arguments: json!({ "name": name }),
        }
    }

    fn call(command_id: &str, schema_version: u64, arguments: Value) -> CommandCall {
        CommandCall {
            command_id: command_id.to_owned(),
            schema_version,
            arguments,
        }
    }

    fn rename_call(name: &str) -> CommandCall {
        call("project.rename", 1, json!({ "name": name }))
    }

    fn transaction(commands: Vec<CommandCall>, revision: u64) -> TransactionEnvelope {
        TransactionEnvelope {
            schema_version: CURRENT_TRANSACTION_SCHEMA_VERSION,
            project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            expected_project_revision: ProjectRevision::new(revision),
            commands,
        }
    }

    fn history_command(command_id: &str, revision: u64, arguments: Value) -> CommandEnvelope {
        CommandEnvelope {
            command_id: command_id.to_owned(),
            schema_version: 1,
            project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            expected_project_revision: ProjectRevision::new(revision),
            arguments,
        }
    }

    fn undo(revision: u64) -> CommandEnvelope {
        history_command("history.undo", revision, json!({}))
    }

    fn redo(revision: u64) -> CommandEnvelope {
        history_command("history.redo", revision, json!({}))
    }

    fn sequence_rate(rate: Option<RationalRate>, revision: u64) -> CommandEnvelope {
        CommandEnvelope::set_timeline_sequence_frame_rate(
            ProjectId::from_str(PROJECT_ID).unwrap(),
            ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            ProjectRevision::new(revision),
            rate,
        )
    }

    fn execute(
        session: &mut ProjectSession,
        command_id: &str,
        arguments: Value,
    ) -> Result<super::CommandResult, super::OperationError> {
        session.execute_command(history_command(
            command_id,
            session.project_revision().value(),
            arguments,
        ))
    }

    fn add_track(
        session: &mut ProjectSession,
        id: &str,
        kind: &str,
    ) -> Result<super::CommandResult, super::OperationError> {
        execute(
            session,
            "timeline.track.add",
            json!({"track_id": id, "kind": kind}),
        )
    }

    fn rational_json(numerator: i64, denominator: u32) -> Value {
        json!({"numerator": numerator, "denominator": denominator})
    }

    fn marker_id(value: &str) -> MarkerId {
        value.parse().unwrap()
    }

    fn marker_add(
        session: &mut ProjectSession,
        marker_id: &str,
        timeline_time: (i64, u32),
        label: &str,
    ) -> Result<super::CommandResult, super::OperationError> {
        execute(
            session,
            "timeline.marker.add",
            json!({
                "marker_id": marker_id,
                "timeline_time": rational_json(timeline_time.0, timeline_time.1),
                "label": label,
            }),
        )
    }

    fn snap_query(
        session: &ProjectSession,
        operation: super::TimelineSnapOperation,
        clip_id: &str,
        target_track_id: Option<&str>,
        target_time: (i64, u32),
    ) -> QueryEnvelope {
        QueryEnvelope::timeline_snap(
            session.project_id(),
            session.project_instance_id(),
            operation,
            clip_id.parse().unwrap(),
            target_track_id.map(|id| id.parse().unwrap()),
            RationalTime::new(target_time.0, target_time.1).unwrap(),
        )
    }

    fn insert_clip(
        session: &mut ProjectSession,
        clip_id: &str,
        track_id: &str,
        media_id: &str,
        timeline_start: (i64, u32),
        source_start: (i64, u32),
        duration: (i64, u32),
    ) -> Result<super::CommandResult, super::OperationError> {
        execute(
            session,
            "timeline.clip.insert",
            json!({
                "clip_id": clip_id,
                "track_id": track_id,
                "media_id": media_id,
                "timeline_start": rational_json(timeline_start.0, timeline_start.1),
                "source_range": {
                    "start": rational_json(source_start.0, source_start.1),
                    "duration": rational_json(duration.0, duration.1),
                }
            }),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_clip_mode(
        session: &mut ProjectSession,
        clip_id: &str,
        track_id: &str,
        media_id: &str,
        timeline_start: (i64, u32),
        duration: (i64, u32),
        mode: &str,
        split_clip_id: Option<&str>,
    ) -> Result<super::CommandResult, super::OperationError> {
        let mut arguments = json!({
            "clip_id": clip_id,
            "track_id": track_id,
            "media_id": media_id,
            "timeline_start": rational_json(timeline_start.0, timeline_start.1),
            "source_range": {
                "start": rational_json(0, 1),
                "duration": rational_json(duration.0, duration.1),
            },
            "mode": mode,
        });
        if let Some(split_clip_id) = split_clip_id {
            arguments["overwrite_split_clip_id"] = json!(split_clip_id);
        }
        execute(session, "timeline.clip.insert", arguments)
    }

    /// The track's clips as `(id, start, duration)` in timeline order.
    fn clip_spans(
        session: &ProjectSession,
        track_id: &str,
    ) -> Vec<(String, RationalTime, RationalTime)> {
        let tracks = session.project().timeline().tracks();
        let index = tracks
            .iter()
            .position(|track| track.id().to_string() == track_id)
            .expect("track exists");
        tracks[index]
            .clips()
            .iter()
            .map(|clip| {
                (
                    clip.id().to_string(),
                    clip.timeline_start(),
                    clip.timeline_duration(),
                )
            })
            .collect()
    }

    fn seed_media_and_track(session: &mut ProjectSession, media_id: &str, track_id: &str) {
        session
            .execute_command(media_add(
                media_item(media_id, "file:///missing/offline.mov"),
                session.project_revision().value(),
            ))
            .unwrap();
        add_track(session, track_id, "video").unwrap();
    }

    #[test]
    fn overwrite_insert_removes_a_clip_it_fully_covers() {
        let mut session = session_with_state("overwrite-test", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (4, 1),
        )
        .unwrap();
        let revision_before = session.project_revision();

        insert_clip_mode(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (4, 1),
            "overwrite",
            None,
        )
        .unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans.len(), 1, "the covered clip must be gone");
        assert_eq!(spans[0].0, CLIP_B);
        assert_eq!(spans[0].1, RationalTime::new(0, 1).unwrap());
        assert_eq!(spans[0].2, RationalTime::new(4, 1).unwrap());
        assert_eq!(
            session.project_revision().value(),
            revision_before.value() + 1,
            "an overwrite is one revision"
        );

        execute(&mut session, "history.undo", json!({})).unwrap();
        let restored = clip_spans(&session, TRACK_A);
        assert_eq!(restored.len(), 1, "one undo restores the replaced range");
        assert_eq!(restored[0].0, CLIP_A);
        assert_eq!(restored[0].1, RationalTime::new(0, 1).unwrap());
        assert_eq!(restored[0].2, RationalTime::new(4, 1).unwrap());
    }

    #[test]
    fn overwrite_insert_trims_a_clip_it_crosses_at_its_head() {
        let mut session = session_with_state("overwrite-test", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (0, 1),
            (4, 1),
        )
        .unwrap();

        insert_clip_mode(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (4, 1),
            "overwrite",
            None,
        )
        .unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].0, CLIP_B);
        assert_eq!(spans[1].0, CLIP_A);
        assert_eq!(
            spans[1].1,
            RationalTime::new(4, 1).unwrap(),
            "the crossed clip keeps only its tail"
        );
        assert_eq!(spans[1].2, RationalTime::new(2, 1).unwrap());
    }

    #[test]
    fn overwrite_insert_trims_a_clip_it_crosses_at_its_tail() {
        let mut session = session_with_state("overwrite-test", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (4, 1),
        )
        .unwrap();

        insert_clip_mode(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (4, 1),
            "overwrite",
            None,
        )
        .unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].0, CLIP_A);
        assert_eq!(spans[0].1, RationalTime::new(0, 1).unwrap());
        assert_eq!(
            spans[0].2,
            RationalTime::new(2, 1).unwrap(),
            "the crossed clip keeps only its head"
        );
        assert_eq!(spans[1].0, CLIP_B);
    }

    #[test]
    fn overwrite_inside_a_clip_needs_a_fragment_id_and_names_it() {
        let mut session = session_with_state("overwrite-test", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (8, 1),
        )
        .unwrap();

        let error = insert_clip_mode(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (2, 1),
            "overwrite",
            None,
        )
        .unwrap_err();

        assert_eq!(
            error.code,
            super::OperationErrorCode::TimelineOverwriteSplitRequiresClipId,
            "silently dropping the tail would be worse than refusing"
        );
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans.len(), 1, "a refused overwrite changes nothing");
        assert_eq!(spans[0].0, CLIP_A);
    }

    #[test]
    fn overwrite_inside_a_clip_splits_it_with_the_callers_id() {
        let mut session = session_with_state("overwrite-test", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (8, 1),
        )
        .unwrap();

        insert_clip_mode(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (2, 1),
            "overwrite",
            Some(CLIP_C),
        )
        .unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].0, CLIP_A);
        assert_eq!(spans[0].2, RationalTime::new(2, 1).unwrap());
        assert_eq!(spans[1].0, CLIP_B);
        assert_eq!(spans[1].1, RationalTime::new(2, 1).unwrap());
        assert_eq!(spans[1].2, RationalTime::new(2, 1).unwrap());
        assert_eq!(spans[2].0, CLIP_C);
        assert_eq!(spans[2].1, RationalTime::new(4, 1).unwrap());
        assert_eq!(spans[2].2, RationalTime::new(4, 1).unwrap());

        execute(&mut session, "history.undo", json!({})).unwrap();
        let restored = clip_spans(&session, TRACK_A);
        assert_eq!(restored.len(), 1, "undo restores the unsplit clip");
        assert_eq!(restored[0].0, CLIP_A);
        assert_eq!(restored[0].2, RationalTime::new(8, 1).unwrap());
    }

    #[test]
    fn insert_mode_still_rejects_an_overlap() {
        let mut session = session_with_state("overwrite-test", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (4, 1),
        )
        .unwrap();

        let error = insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap_err();

        assert_eq!(
            error.code,
            super::OperationErrorCode::TimelineOverlap,
            "the default mode keeps its old meaning"
        );
    }

    fn insert_clip_at(
        session: &mut ProjectSession,
        clip_id: &str,
        track_id: &str,
        media_id: &str,
        timeline_start: (i64, u32),
        duration: (i64, u32),
    ) {
        insert_clip(
            session,
            clip_id,
            track_id,
            media_id,
            timeline_start,
            (0, 1),
            duration,
        )
        .unwrap();
    }

    fn ripple_trim_end(
        session: &mut ProjectSession,
        clip_id: &str,
        target: (i64, u32),
        ripple: bool,
    ) -> Result<super::CommandResult, super::OperationError> {
        execute(
            session,
            "timeline.clip.trim",
            json!({
                "clip_id": clip_id,
                "edge": "end",
                "timeline_time": rational_json(target.0, target.1),
                "ripple": ripple,
            }),
        )
    }

    #[test]
    fn ripple_trim_pulls_every_later_clip_left() {
        let mut session = session_with_state("ripple-trim", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (4, 1), (2, 1));
        insert_clip_at(&mut session, CLIP_C, TRACK_A, MEDIA_A, (6, 1), (2, 1));
        let revision_before = session.project_revision();

        ripple_trim_end(&mut session, CLIP_A, (2, 1), true).unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].2, RationalTime::new(2, 1).unwrap());
        assert_eq!(
            spans[1].1,
            RationalTime::new(2, 1).unwrap(),
            "the next clip closes the gap"
        );
        assert_eq!(
            spans[2].1,
            RationalTime::new(4, 1).unwrap(),
            "and the one after it moves by the same delta"
        );
        assert_eq!(spans[1].2, RationalTime::new(2, 1).unwrap());
        assert_eq!(
            session.project_revision().value(),
            revision_before.value() + 1,
            "a ripple trim is one revision"
        );

        execute(&mut session, "history.undo", json!({})).unwrap();
        let restored = clip_spans(&session, TRACK_A);
        assert_eq!(restored[0].2, RationalTime::new(4, 1).unwrap());
        assert_eq!(
            restored[1].1,
            RationalTime::new(4, 1).unwrap(),
            "one undo restores every shifted clip"
        );
        assert_eq!(restored[2].1, RationalTime::new(6, 1).unwrap());
    }

    #[test]
    fn ripple_trim_opens_a_gap_when_the_clip_lengthens() {
        let mut session = session_with_state("ripple-lengthen", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (2, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (2, 1));

        ripple_trim_end(&mut session, CLIP_A, (3, 1), true).unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(3, 1).unwrap());
        assert_eq!(
            spans[1].1,
            RationalTime::new(3, 1).unwrap(),
            "the later clip moves right with the extended edge"
        );
    }

    #[test]
    fn plain_trim_leaves_later_clips_where_they_are() {
        let mut session = session_with_state("plain-trim", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (4, 1), (2, 1));

        // No ripple argument at all: the default must keep the old meaning, which
        // is the regression that would silently move every user's later clips.
        execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_A,
                "edge": "end",
                "timeline_time": rational_json(2, 1),
            }),
        )
        .unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(2, 1).unwrap());
        assert_eq!(
            spans[1].1,
            RationalTime::new(4, 1).unwrap(),
            "a plain trim leaves the gap"
        );
    }

    #[test]
    fn a_ripple_trim_to_the_current_edge_is_a_no_op() {
        let mut session = session_with_state("ripple-noop", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (4, 1), (2, 1));
        let revision_before = session.project_revision();

        // A ripple is still a trim: an edge that does not move changes nothing and,
        // in particular, must not shift the later clips.
        let result = ripple_trim_end(&mut session, CLIP_A, (4, 1), true).unwrap();
        assert!(!result.changed);
        assert_eq!(session.project_revision().value(), revision_before.value());
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(4, 1).unwrap());
        assert_eq!(spans[1].1, RationalTime::new(4, 1).unwrap());
    }

    #[test]
    fn a_ripple_trim_outside_the_clip_is_refused_and_changes_nothing() {
        let mut session = session_with_state("ripple-refused", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (2, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (2, 1));
        let revision_before = session.project_revision();

        // An end trim to or before the clip's own start is invalid for any end
        // trim, ripple or not, and the track must be untouched afterwards. (An end
        // beyond the current end is a lengthening and is legal.)
        let error = ripple_trim_end(&mut session, CLIP_A, (0, 1), true).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        assert_eq!(session.project_revision().value(), revision_before.value());
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(2, 1).unwrap());
        assert_eq!(spans[1].1, RationalTime::new(2, 1).unwrap());
    }

    #[test]
    fn ripple_on_a_start_edge_is_refused_with_the_plain_trim_meaning() {
        let mut session = session_with_state("ripple-start", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (4, 1));

        let error = execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_A,
                "edge": "start",
                "timeline_time": rational_json(1, 1),
                "ripple": true,
            }),
        )
        .unwrap_err();

        assert_eq!(
            error.code,
            super::OperationErrorCode::InvalidArguments,
            "ripple-in has two meanings; refusing is the honest default"
        );
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].1, RationalTime::new(0, 1).unwrap());
        assert_eq!(spans[0].2, RationalTime::new(4, 1).unwrap());
    }

    fn slip_clip(
        session: &mut ProjectSession,
        clip_id: &str,
        delta: (i64, u32),
    ) -> Result<super::CommandResult, super::OperationError> {
        execute(
            session,
            "timeline.clip.slip",
            json!({
                "clip_id": clip_id,
                "delta": rational_json(delta.0, delta.1),
            }),
        )
    }

    /// The clip's source window start, which is what a slip moves.
    fn source_start(session: &ProjectSession, track_id: &str, clip_id: &str) -> RationalTime {
        let tracks = session.project().timeline().tracks();
        let index = tracks
            .iter()
            .position(|track| track.id().to_string() == track_id)
            .expect("track exists");
        let clip = tracks[index]
            .clips()
            .iter()
            .find(|clip| clip.id().to_string() == clip_id)
            .expect("clip exists");
        match clip.content() {
            ClipContent::Media { source_range, .. } => source_range.start(),
            other => panic!("expected media content, found {other:?}"),
        }
    }

    #[test]
    fn slip_moves_the_source_window_and_leaves_the_cut_where_it_was() {
        let mut session = session_with_state("slip", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (1, 1), (1, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (1, 1));
        let revision_before = session.project_revision();
        let spans_before = clip_spans(&session, TRACK_A);

        slip_clip(&mut session, CLIP_A, (1, 4)).unwrap();

        let spans_after = clip_spans(&session, TRACK_A);
        assert_eq!(
            spans_before, spans_after,
            "a slip never moves a clip or its neighbours"
        );
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(1, 4).unwrap(),
            "the source window moved by the delta"
        );
        assert_eq!(
            session.project_revision().value(),
            revision_before.value() + 1,
            "a slip is one revision"
        );

        execute(&mut session, "history.undo", json!({})).unwrap();
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(0, 1).unwrap(),
            "undo restores the original source window"
        );
        assert_eq!(spans_before, clip_spans(&session, TRACK_A));
    }

    #[test]
    fn slip_backwards_takes_earlier_source_material() {
        let mut session = session_with_state("slip-back", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        // Start an eighth of a second into the source so there is room to go back.
        slip_clip(&mut session, CLIP_A, (1, 8)).unwrap();
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(1, 8).unwrap()
        );

        slip_clip(&mut session, CLIP_A, (-1, 8)).unwrap();
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(0, 1).unwrap(),
            "a negative delta takes earlier material"
        );
    }

    #[test]
    fn a_slip_past_the_start_of_the_source_is_refused() {
        let mut session = session_with_state("slip-refused", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        let revision_before = session.project_revision();

        // Nothing exists before the start of the media, so the clip must not be
        // left pointing at it.
        let error = slip_clip(&mut session, CLIP_A, (-1, 1)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        assert_eq!(session.project_revision().value(), revision_before.value());
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(0, 1).unwrap()
        );
    }

    #[test]
    fn a_slip_of_zero_is_a_no_op() {
        let mut session = session_with_state("slip-zero", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        let revision_before = session.project_revision();

        let result = slip_clip(&mut session, CLIP_A, (0, 1)).unwrap();
        assert!(!result.changed);
        assert_eq!(session.project_revision().value(), revision_before.value());
    }

    fn roll_boundary(
        session: &mut ProjectSession,
        clip_id: &str,
        delta: (i64, u32),
    ) -> Result<super::CommandResult, super::OperationError> {
        execute(
            session,
            "timeline.clip.roll",
            json!({
                "clip_id": clip_id,
                "delta": rational_json(delta.0, delta.1),
            }),
        )
    }

    #[test]
    fn roll_moves_the_boundary_and_both_source_windows() {
        let mut session = session_with_state("roll", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
        let revision_before = session.project_revision();

        roll_boundary(&mut session, CLIP_A, (1, 4)).unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(
            spans[0].2,
            RationalTime::new(5, 4).unwrap(),
            "the left clip gives up a quarter second"
        );
        assert_eq!(
            spans[1].1,
            RationalTime::new(5, 4).unwrap(),
            "the right clip starts where the boundary moved to"
        );
        assert_eq!(spans[1].2, RationalTime::new(3, 4).unwrap());
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_A),
            RationalTime::new(0, 1).unwrap(),
            "the left clip's source does not move"
        );
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_B),
            RationalTime::new(1, 4).unwrap(),
            "the right clip's source advances with its start"
        );
        assert_eq!(
            session.project_revision().value(),
            revision_before.value() + 1,
            "a roll is one revision"
        );

        execute(&mut session, "history.undo", json!({})).unwrap();
        let restored = clip_spans(&session, TRACK_A);
        assert_eq!(restored[0].2, RationalTime::new(1, 1).unwrap());
        assert_eq!(restored[1].1, RationalTime::new(1, 1).unwrap());
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_B),
            RationalTime::new(0, 1).unwrap(),
            "one undo restores both sides"
        );
    }

    #[test]
    fn roll_needs_two_clips_that_actually_touch() {
        let mut session = session_with_state("roll-gap", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        // A one-second gap: there is no shared boundary to roll.
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (1, 1));

        let error = roll_boundary(&mut session, CLIP_A, (1, 4)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(1, 1).unwrap());
        assert_eq!(spans[1].1, RationalTime::new(2, 1).unwrap());
    }

    #[test]
    fn roll_past_either_clip_is_refused() {
        let mut session = session_with_state("roll-bounds", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));

        // Consuming the whole right clip leaves it no duration.
        let error = roll_boundary(&mut session, CLIP_A, (1, 1)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        // And the same on the other side.
        let error = roll_boundary(&mut session, CLIP_A, (-1, 1)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(1, 1).unwrap());
        assert_eq!(spans[1].2, RationalTime::new(1, 1).unwrap());
    }

    #[test]
    fn a_roll_of_zero_is_a_no_op() {
        let mut session = session_with_state("roll-zero", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
        let revision_before = session.project_revision();

        let result = roll_boundary(&mut session, CLIP_A, (0, 1)).unwrap();
        assert!(!result.changed);
        assert_eq!(session.project_revision().value(), revision_before.value());
    }

    fn slide_clip(
        session: &mut ProjectSession,
        clip_id: &str,
        delta: (i64, u32),
    ) -> Result<super::CommandResult, super::OperationError> {
        execute(
            session,
            "timeline.clip.slide",
            json!({
                "clip_id": clip_id,
                "delta": rational_json(delta.0, delta.1),
            }),
        )
    }

    /// Three adjacent one-second clips, which is the smallest track a slide has a
    /// middle clip to move on.
    fn seed_three_adjacent_clips(session: &mut ProjectSession) {
        seed_media_and_track(session, MEDIA_A, TRACK_A);
        insert_clip_at(session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
        insert_clip_at(session, CLIP_C, TRACK_A, MEDIA_A, (2, 1), (1, 1));
    }

    #[test]
    fn slide_moves_the_clip_and_lets_both_neighbours_absorb_it() {
        let mut session = session_with_state("slide", 0);
        seed_three_adjacent_clips(&mut session);
        let revision_before = session.project_revision();

        slide_clip(&mut session, CLIP_B, (1, 4)).unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(
            spans[0].2,
            RationalTime::new(5, 4).unwrap(),
            "the previous clip grows"
        );
        assert_eq!(
            spans[1].1,
            RationalTime::new(5, 4).unwrap(),
            "the clip moves"
        );
        assert_eq!(
            spans[1].2,
            RationalTime::new(1, 1).unwrap(),
            "and keeps its duration"
        );
        assert_eq!(
            spans[2].1,
            RationalTime::new(9, 4).unwrap(),
            "the next clip starts later"
        );
        assert_eq!(
            spans[2].2,
            RationalTime::new(3, 4).unwrap(),
            "and gives up the same amount"
        );
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_B),
            RationalTime::new(0, 1).unwrap(),
            "the moving clip's source never changes"
        );
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_C),
            RationalTime::new(1, 4).unwrap(),
            "the next clip's source advances with its start"
        );
        assert_eq!(
            session.project_revision().value(),
            revision_before.value() + 1
        );

        execute(&mut session, "history.undo", json!({})).unwrap();
        let restored = clip_spans(&session, TRACK_A);
        assert_eq!(restored[0].2, RationalTime::new(1, 1).unwrap());
        assert_eq!(restored[1].1, RationalTime::new(1, 1).unwrap());
        assert_eq!(restored[2].1, RationalTime::new(2, 1).unwrap());
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_C),
            RationalTime::new(0, 1).unwrap(),
            "one undo restores all three clips"
        );
    }

    #[test]
    fn slide_backwards_is_refused_when_the_next_clip_has_no_earlier_material() {
        let mut session = session_with_state("slide-back-refused", 0);
        seed_three_adjacent_clips(&mut session);

        // The next clip starts at the beginning of its own source: it has nothing
        // earlier to give, so pulling the clip back would reference material that
        // does not exist. That is a refusal, not a trim.
        let error = slide_clip(&mut session, CLIP_B, (-1, 4)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[1].1, RationalTime::new(1, 1).unwrap());
        assert_eq!(spans[2].1, RationalTime::new(2, 1).unwrap());
    }

    #[test]
    fn slide_backwards_pulls_the_clip_earlier_when_the_material_allows_it() {
        let mut session = session_with_state("slide-back", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(&mut session, CLIP_B, TRACK_A, MEDIA_A, (1, 1), (1, 1));
        // This clip starts a quarter second into its source, so it can give that
        // quarter second back when the slide pulls it earlier.
        insert_clip(
            &mut session,
            CLIP_C,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (1, 4),
            (1, 1),
        )
        .unwrap();

        slide_clip(&mut session, CLIP_B, (-1, 4)).unwrap();

        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(3, 4).unwrap());
        assert_eq!(spans[1].1, RationalTime::new(3, 4).unwrap());
        assert_eq!(
            spans[1].2,
            RationalTime::new(1, 1).unwrap(),
            "the moving clip keeps its duration"
        );
        assert_eq!(spans[2].1, RationalTime::new(7, 4).unwrap());
        assert_eq!(spans[2].2, RationalTime::new(5, 4).unwrap());
        assert_eq!(
            source_start(&session, TRACK_A, CLIP_C),
            RationalTime::new(0, 1).unwrap(),
            "the next clip's source moves back with its start"
        );
    }

    #[test]
    fn slide_needs_touching_neighbours_on_both_sides() {
        let mut session = session_with_state("slide-edges", 0);
        seed_three_adjacent_clips(&mut session);
        // The first and last clip of a track have no neighbour to borrow from.
        let error = slide_clip(&mut session, CLIP_A, (1, 4)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let error = slide_clip(&mut session, CLIP_C, (1, 4)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);

        // And a gap means there is nothing to borrow either.
        let mut gapped = session_with_state("slide-gap", 0);
        seed_media_and_track(&mut gapped, MEDIA_A, TRACK_A);
        insert_clip_at(&mut gapped, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        insert_clip_at(&mut gapped, CLIP_B, TRACK_A, MEDIA_A, (2, 1), (1, 1));
        insert_clip_at(&mut gapped, CLIP_C, TRACK_A, MEDIA_A, (3, 1), (1, 1));
        let error = slide_clip(&mut gapped, CLIP_B, (1, 4)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let spans = clip_spans(&gapped, TRACK_A);
        assert_eq!(spans[1].1, RationalTime::new(2, 1).unwrap());
    }

    #[test]
    fn slide_that_would_consume_a_neighbour_is_refused() {
        let mut session = session_with_state("slide-bounds", 0);
        seed_three_adjacent_clips(&mut session);

        let error = slide_clip(&mut session, CLIP_B, (2, 1)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let error = slide_clip(&mut session, CLIP_B, (-2, 1)).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let spans = clip_spans(&session, TRACK_A);
        assert_eq!(spans[0].2, RationalTime::new(1, 1).unwrap());
        assert_eq!(spans[1].1, RationalTime::new(1, 1).unwrap());
    }

    #[test]
    fn a_slide_of_zero_is_a_no_op() {
        let mut session = session_with_state("slide-zero", 0);
        seed_three_adjacent_clips(&mut session);
        let revision_before = session.project_revision();

        let result = slide_clip(&mut session, CLIP_B, (0, 1)).unwrap();
        assert!(!result.changed);
        assert_eq!(session.project_revision().value(), revision_before.value());
    }

    fn move_track(
        session: &mut ProjectSession,
        track_id: &str,
        to_index: i64,
    ) -> Result<super::CommandResult, super::OperationError> {
        execute(
            session,
            "timeline.track.move",
            json!({
                "track_id": track_id,
                "to_index": to_index,
            }),
        )
    }

    /// The track kinds in order, which is the only thing a move changes.
    fn track_order(session: &ProjectSession) -> Vec<String> {
        session
            .project()
            .timeline()
            .tracks()
            .iter()
            .map(|track| track.id().to_string())
            .collect()
    }

    #[test]
    fn moving_a_track_reorders_it_without_touching_its_clips() {
        let mut session = session_with_state("track-move", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        add_track(&mut session, TRACK_B, "audio").unwrap();
        let third = "77777777-7777-4777-8777-777777777777";
        add_track(&mut session, third, "video").unwrap();
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        let clips_before = clip_spans(&session, TRACK_A);
        let revision_before = session.project_revision();

        // Move the first track to the end.
        move_track(&mut session, TRACK_A, 2).unwrap();

        assert_eq!(
            track_order(&session),
            vec![TRACK_B.to_owned(), third.to_owned(), TRACK_A.to_owned()],
            "the other tracks shift up by one"
        );
        assert_eq!(
            clip_spans(&session, TRACK_A),
            clips_before,
            "the moved track keeps its clips"
        );
        assert_eq!(
            session.project_revision().value(),
            revision_before.value() + 1
        );

        execute(&mut session, "history.undo", json!({})).unwrap();
        assert_eq!(
            track_order(&session),
            vec![TRACK_A.to_owned(), TRACK_B.to_owned(), third.to_owned()],
            "one undo puts the track back"
        );
        assert_eq!(clip_spans(&session, TRACK_A), clips_before);

        // Redo must work too: a move is its own inverse, and the inverse change is
        // what the history entry records.
        execute(&mut session, "history.redo", json!({})).unwrap();
        assert_eq!(
            track_order(&session),
            vec![TRACK_B.to_owned(), third.to_owned(), TRACK_A.to_owned()]
        );
    }

    #[test]
    fn moving_a_track_to_its_own_index_is_a_no_op() {
        let mut session = session_with_state("track-move-noop", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        add_track(&mut session, TRACK_B, "audio").unwrap();
        let revision_before = session.project_revision();

        let result = move_track(&mut session, TRACK_A, 0).unwrap();
        assert!(!result.changed);
        assert_eq!(session.project_revision().value(), revision_before.value());
    }

    #[test]
    fn moving_a_track_past_the_end_or_an_unknown_track_is_refused() {
        let mut session = session_with_state("track-move-refused", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        add_track(&mut session, TRACK_B, "audio").unwrap();
        let order_before = track_order(&session);
        let revision_before = session.project_revision();

        let error = move_track(&mut session, TRACK_A, 2).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        let unknown = "99999999-9999-4999-8999-999999999999";
        let error = move_track(&mut session, unknown, 0).unwrap_err();
        assert_eq!(error.code, super::OperationErrorCode::TimelineTrackNotFound);
        assert_eq!(track_order(&session), order_before);
        assert_eq!(session.project_revision().value(), revision_before.value());
    }

    /// A media clip without an explicit rate still plays at identity speed, so its
    /// source window and timeline duration must be exactly equal.
    #[test]
    fn a_media_clip_without_a_rate_cannot_map_different_source_and_timeline_durations() {
        let mut session = session_with_state("rate-invariant", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        let before = clip_spans(&session, TRACK_A);
        let revision_before = session.project_revision();

        // One second of source over half a second of timeline needs an explicit 2x
        // rate; omitting it means identity speed and remains invalid.
        let error = execute(
            &mut session,
            "timeline.clip.update",
            json!({
                "clip_id": CLIP_A,
                "timeline_duration": rational_json(1, 2),
                "content": {
                    "type": "media",
                    "media_id": MEDIA_A,
                    "source_range": {
                        "start": rational_json(0, 1),
                        "duration": rational_json(1, 1),
                    },
                },
                "settings": {
                    "type": "visual",
                    "transform": {
                        "x_milli_canvas": 0,
                        "y_milli_canvas": 0,
                        "scale_x_milli": 1000,
                        "scale_y_milli": 1000,
                        "rotation_milli_degrees": 0,
                        "anchor_x_basis_points": 5000,
                        "anchor_y_basis_points": 5000,
                    },
                    "crop": {
                        "left_basis_points": 0,
                        "top_basis_points": 0,
                        "right_basis_points": 0,
                        "bottom_basis_points": 0,
                    },
                    "opacity": {"basis_points": 10000},
                    "effects": [],
                    "transition_in": null,
                    "transition_out": null,
                },
            }),
        )
        .unwrap_err();

        assert_eq!(error.code, super::OperationErrorCode::InvalidArguments);
        assert_eq!(session.project_revision().value(), revision_before.value());
        assert_eq!(clip_spans(&session, TRACK_A), before);
    }

    #[test]
    fn a_media_clip_can_play_at_a_different_rate_and_undo_restores_identity() {
        let mut session = session_with_state("rate-update", 0);
        seed_media_and_track(&mut session, MEDIA_A, TRACK_A);
        insert_clip_at(&mut session, CLIP_A, TRACK_A, MEDIA_A, (0, 1), (1, 1));
        let revision_before = session.project_revision().value();

        let media_id = MediaId::from_str(MEDIA_A).unwrap();
        let clip_id = ClipId::from_str(CLIP_A).unwrap();
        session
            .execute_command(CommandEnvelope::update_timeline_clip(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                clip_id,
                RationalTime::new(1, 2).unwrap(),
                ClipContent::Media {
                    media_id,
                    source_range: TimeRange::new(
                        RationalTime::ZERO,
                        RationalTime::new(1, 1).unwrap(),
                    )
                    .unwrap(),
                    playback_rate: RationalRate::new(2, 1).unwrap(),
                    reversed: false,
                    frozen: false,
                },
                ClipSettings::default_for(TrackKind::Video),
            ))
            .unwrap();

        let clip = &session.project().timeline().tracks()[0].clips()[0];
        assert_eq!(clip.timeline_duration(), RationalTime::new(1, 2).unwrap());
        assert_eq!(
            clip.content().playback_rate(),
            RationalRate::new(2, 1).unwrap()
        );
        assert_eq!(session.project_revision().value(), revision_before + 1);

        execute(&mut session, "history.undo", json!({})).unwrap();
        let clip = &session.project().timeline().tracks()[0].clips()[0];
        assert_eq!(clip.timeline_duration(), RationalTime::new(1, 1).unwrap());
        assert_eq!(clip.content().playback_rate(), RationalRate::ONE);

        execute(&mut session, "history.redo", json!({})).unwrap();
        let clip = &session.project().timeline().tracks()[0].clips()[0];
        assert_eq!(clip.timeline_duration(), RationalTime::new(1, 2).unwrap());
        assert_eq!(
            clip.content().playback_rate(),
            RationalRate::new(2, 1).unwrap()
        );
    }

    fn seed_video_track_and_clip(
        session: &mut ProjectSession,
        media_id: &str,
        track_id: &str,
        clip_id: &str,
        timeline_start: (i64, u32),
        duration: (i64, u32),
    ) {
        session
            .execute_command(media_add(
                media_item(media_id, "file:///missing/offline.mov"),
                session.project_revision().value(),
            ))
            .unwrap();
        add_track(session, track_id, "video").unwrap();
        insert_clip(
            session,
            clip_id,
            track_id,
            media_id,
            timeline_start,
            (0, 1),
            duration,
        )
        .unwrap();
    }

    fn fixture_clip(
        clip_id: &str,
        media_id: &str,
        timeline_start: (i64, u32),
        source_start: (i64, u32),
        duration: (i64, u32),
    ) -> TimelineClip {
        TimelineClip::from_parts_for_codec(
            clip_id.parse().unwrap(),
            media_id.parse().unwrap(),
            RationalTime::new(timeline_start.0, timeline_start.1).unwrap(),
            TimeRange::new(
                RationalTime::new(source_start.0, source_start.1).unwrap(),
                RationalTime::new(duration.0, duration.1).unwrap(),
            )
            .unwrap(),
        )
    }

    fn fixture_track(track_id: &str, kind: TrackKind, clips: Vec<TimelineClip>) -> TimelineTrack {
        TimelineTrack::from_parts_for_codec(track_id.parse().unwrap(), kind, clips)
    }

    fn assert_name_change(change_set: &ChangeSet, before: &str, after: &str) {
        assert!(matches!(
            change_set.changes(),
            [ProjectChange::ProjectName { before: actual_before, after: actual_after }]
                if actual_before.as_str() == before && actual_after.as_str() == after
        ));
    }

    fn summary_query() -> QueryEnvelope {
        QueryEnvelope {
            query_id: "project.summary".to_owned(),
            schema_version: 1,
            project_id: ProjectId::from_str(PROJECT_ID).unwrap(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            arguments: json!({}),
        }
    }

    fn code<T>(result: &Result<T, super::OperationError>) -> OperationErrorCode {
        result.as_ref().err().expect("operation should fail").code
    }

    #[test]
    fn common_dispatch_reuses_command_query_and_transaction_paths() {
        let mut session = fixed_session();
        let command =
            session.handle_application_request(ApplicationRequest::Command(rename("B", 0)));
        assert!(
            matches!(command, ApplicationResponse::Command(result) if result.after_revision == ProjectRevision::new(1))
        );

        let query = session.handle_application_request(ApplicationRequest::Query(summary_query()));
        assert!(matches!(query, ApplicationResponse::Query(result) if result.summary.name == "B"));

        let transaction = session.handle_application_request(ApplicationRequest::Transaction(
            transaction(vec![rename_call("C"), rename_call("D")], 1),
        ));
        assert!(
            matches!(transaction, ApplicationResponse::Transaction(result) if result.after_revision == ProjectRevision::new(2) && result.command_count == 2)
        );

        let stale = session.handle_application_request(ApplicationRequest::Command(rename("E", 1)));
        assert!(
            matches!(stale, ApplicationResponse::Error(error) if error.code == OperationErrorCode::RevisionConflict)
        );
    }

    #[test]
    fn catalogs_are_exact_unique_and_deterministic() {
        assert_eq!(
            command_catalog(),
            &[
                CommandDescriptor {
                    id: "project.rename",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: true,
                },
                CommandDescriptor {
                    id: "history.undo",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "history.redo",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "media.add",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "media.remove",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "media.relink",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.track.add",
                    schema_version: 2,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.track.remove",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.track.move",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clip.insert",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.track.set_state",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clip.insert_content",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.captions.import",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clip.update",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clip.move",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clips.move",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clip.delete",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clip.trim",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clip.slip",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clip.roll",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clip.slide",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clip.split",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.clip.ripple_delete",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.marker.add",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.marker.move",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.marker.rename",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.marker.delete",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
                CommandDescriptor {
                    id: "timeline.sequence.set_frame_rate",
                    schema_version: 1,
                    mutates_project: true,
                    allowed_in_transaction: false,
                },
            ]
        );
        assert_eq!(
            query_catalog(),
            &[
                QueryDescriptor {
                    id: "project.summary",
                    schema_version: 1,
                },
                QueryDescriptor {
                    id: "media.list",
                    schema_version: 1,
                },
                QueryDescriptor {
                    id: "media.get",
                    schema_version: 1,
                },
                QueryDescriptor {
                    id: "timeline.tracks",
                    schema_version: 2,
                },
                QueryDescriptor {
                    id: "timeline.clips",
                    schema_version: 2,
                },
                QueryDescriptor {
                    id: "timeline.snap",
                    schema_version: 2,
                },
                QueryDescriptor {
                    id: "timeline.markers",
                    schema_version: 1,
                },
                QueryDescriptor {
                    id: "timeline.sequence.settings",
                    schema_version: 1,
                },
            ]
        );
        assert_eq!(command_catalog(), command_catalog());
        assert_eq!(COMMANDS.len(), 28);
        assert_eq!(QUERIES.len(), 8);
    }

    #[test]
    fn phase_4d_error_codes_serialize_to_stable_machine_names() {
        for (code, expected) in [
            (
                OperationErrorCode::UnsupportedTransactionSchema,
                "UNSUPPORTED_TRANSACTION_SCHEMA",
            ),
            (OperationErrorCode::EmptyTransaction, "EMPTY_TRANSACTION"),
            (
                OperationErrorCode::CommandNotAllowedInTransaction,
                "COMMAND_NOT_ALLOWED_IN_TRANSACTION",
            ),
            (OperationErrorCode::NothingToUndo, "NOTHING_TO_UNDO"),
            (OperationErrorCode::NothingToRedo, "NOTHING_TO_REDO"),
            (OperationErrorCode::HistoryConflict, "HISTORY_CONFLICT"),
            (
                OperationErrorCode::HistoryStorageFailure,
                "HISTORY_STORAGE_FAILURE",
            ),
        ] {
            assert_eq!(
                serde_json::to_string(&code).unwrap(),
                format!("\"{expected}\"")
            );
        }
    }

    #[test]
    fn media_operation_error_codes_serialize_to_stable_machine_names() {
        for (code, expected) in [
            (
                OperationErrorCode::MediaIdAlreadyExists,
                "MEDIA_ID_ALREADY_EXISTS",
            ),
            (
                OperationErrorCode::MediaSourceAlreadyExists,
                "MEDIA_SOURCE_ALREADY_EXISTS",
            ),
            (OperationErrorCode::MediaNotFound, "MEDIA_NOT_FOUND"),
            (OperationErrorCode::MediaInUse, "MEDIA_IN_USE"),
        ] {
            assert_eq!(
                serde_json::to_string(&code).unwrap(),
                format!("\"{expected}\"")
            );
        }
    }

    #[test]
    fn timeline_operation_error_codes_serialize_to_stable_machine_names() {
        for (code, expected) in [
            (
                OperationErrorCode::TimelineTrackIdAlreadyExists,
                "TIMELINE_TRACK_ID_ALREADY_EXISTS",
            ),
            (
                OperationErrorCode::TimelineTrackNotFound,
                "TIMELINE_TRACK_NOT_FOUND",
            ),
            (
                OperationErrorCode::TimelineTrackNotEmpty,
                "TIMELINE_TRACK_NOT_EMPTY",
            ),
            (
                OperationErrorCode::TimelineClipIdAlreadyExists,
                "TIMELINE_CLIP_ID_ALREADY_EXISTS",
            ),
            (
                OperationErrorCode::TimelineClipNotFound,
                "TIMELINE_CLIP_NOT_FOUND",
            ),
            (
                OperationErrorCode::TimelineMediaIncompatible,
                "TIMELINE_MEDIA_INCOMPATIBLE",
            ),
            (OperationErrorCode::TimelineOverlap, "TIMELINE_OVERLAP"),
            (
                OperationErrorCode::TimelineLimitExceeded,
                "TIMELINE_LIMIT_EXCEEDED",
            ),
        ] {
            assert_eq!(
                serde_json::to_string(&code).unwrap(),
                format!("\"{expected}\"")
            );
        }
    }

    #[test]
    fn timeline_track_add_preserves_requested_id_kind_revision_and_history() {
        let mut session = fixed_session();
        let result = add_track(&mut session, TRACK_A, "video").unwrap();

        assert!(result.changed);
        assert_eq!(result.before_revision, ProjectRevision::new(0));
        assert_eq!(result.after_revision, ProjectRevision::new(1));
        assert!(matches!(
            result.change_set.changes(),
            [ProjectChange::TimelineTrackAdded {
                track_id,
                track_kind: TrackKind::Video,
                index: 0,
                ..
            }] if *track_id == TrackId::from_str(TRACK_A).unwrap()
        ));
        assert_eq!(session.project().timeline().tracks().len(), 1);
        assert_eq!(
            session.project().timeline().tracks()[0].id().to_string(),
            TRACK_A
        );
        assert_eq!(
            session.project().timeline().tracks()[0].kind(),
            TrackKind::Video
        );
        assert!(session.project().timeline().tracks()[0].clips().is_empty());
        assert_eq!(session.history.undo.len(), 1);
        assert!(session.history.undo[0].changes().len() == 1);
        assert!(session.history.redo.is_empty());

        session.execute_command(undo(1)).unwrap();
        assert!(session.project().timeline().tracks().is_empty());
        assert_eq!(session.project_revision(), ProjectRevision::new(2));
        session.execute_command(redo(2)).unwrap();
        assert_eq!(
            session.project().timeline().tracks()[0].id().to_string(),
            TRACK_A
        );
        assert_eq!(session.project_revision(), ProjectRevision::new(3));
    }

    fn caption_clip_state(
        id: &str,
        start: (i64, u32),
        duration: (i64, u32),
        text: &str,
    ) -> TimelineClipState {
        TimelineClipState {
            clip_id: ClipId::from_str(id).unwrap(),
            timeline_start: RationalTime::new(start.0, start.1).unwrap(),
            timeline_duration: RationalTime::new(duration.0, duration.1).unwrap(),
            content: ClipContent::Caption {
                text: text.to_owned(),
                formatting: crate::TextFormatting::DEFAULT,
            },
            settings: ClipSettings::Visual(crate::VisualSettings::default()),
        }
    }

    #[test]
    fn caption_track_import_is_one_atomic_revision_and_one_undo_entry() {
        let mut session = fixed_session();
        let track_id = TrackId::from_str(TRACK_A).unwrap();
        let clips = vec![
            caption_clip_state(CLIP_A, (125, 1000), (9, 4), "First caption"),
            caption_clip_state(CLIP_B, (5, 2), (1, 2), "Second caption"),
        ];
        let result = session
            .execute_command(CommandEnvelope::import_timeline_captions(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                track_id,
                clips,
            ))
            .unwrap();

        assert_eq!(result.before_revision, ProjectRevision::new(0));
        assert_eq!(result.after_revision, ProjectRevision::new(1));
        assert!(matches!(
            result.change_set.changes(),
            [ProjectChange::TimelineCaptionTrackAdded { track_id: added_id, clips, .. }]
                if *added_id == track_id && clips.len() == 2
        ));
        assert_eq!(session.project().timeline().tracks().len(), 1);
        let track = &session.project().timeline().tracks()[0];
        assert_eq!(track.id(), track_id);
        assert_eq!(track.kind(), TrackKind::Caption);
        assert_eq!(track.clips().len(), 2);
        assert_eq!(session.history.undo.len(), 1);

        let undone = session.execute_command(undo(1)).unwrap();
        assert!(session.project().timeline().tracks().is_empty());
        assert!(matches!(
            undone.change_set.changes(),
            [ProjectChange::TimelineCaptionTrackRemoved { .. }]
        ));
        assert!(matches!(
            session.history.redo[0].changes(),
            [ProjectChange::TimelineCaptionTrackAdded { .. }]
        ));
        session.execute_command(redo(2)).unwrap();
        assert_eq!(session.project().timeline().tracks()[0].clips().len(), 2);
        assert_eq!(session.project_revision(), ProjectRevision::new(3));
    }

    #[test]
    fn caption_track_import_rejects_overlap_and_duplicate_ids_without_partial_change() {
        let mut session = fixed_session();
        let import = |session: &ProjectSession, track: &str, clips| {
            CommandEnvelope::import_timeline_captions(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                TrackId::from_str(track).unwrap(),
                clips,
            )
        };
        let before = session.project().clone();
        let error = session
            .execute_command(import(
                &session,
                TRACK_A,
                vec![
                    caption_clip_state(CLIP_A, (0, 1), (2, 1), "one"),
                    caption_clip_state(CLIP_B, (1, 1), (2, 1), "two"),
                ],
            ))
            .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::TimelineOverlap);
        assert_eq!(session.project(), &before);
        assert!(session.history.undo.is_empty());

        let error = session
            .execute_command(import(
                &session,
                TRACK_A,
                vec![
                    caption_clip_state(CLIP_A, (0, 1), (1, 2), "one"),
                    caption_clip_state(CLIP_A, (1, 1), (1, 2), "two"),
                ],
            ))
            .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::TimelineClipIdAlreadyExists);
        assert_eq!(session.project(), &before);
        assert!(session.history.undo.is_empty());
    }

    #[test]
    fn timeline_track_add_appends_in_canonical_order_and_duplicate_is_atomic() {
        let mut session = fixed_session();
        add_track(&mut session, TRACK_A, "video").unwrap();
        add_track(&mut session, TRACK_B, "audio").unwrap();
        add_track(&mut session, TRACK_C, "video").unwrap();
        let ids = session
            .project()
            .timeline()
            .tracks()
            .iter()
            .map(|track| track.id().to_string())
            .collect::<Vec<_>>();
        assert_eq!(ids, [TRACK_A, TRACK_B, TRACK_C]);

        let before_project = session.project().clone();
        let before_undo = session.history.undo.clone();
        let before_redo = session.history.redo.clone();
        let error = add_track(&mut session, TRACK_A, "audio").unwrap_err();
        assert_eq!(error.code, OperationErrorCode::TimelineTrackIdAlreadyExists);
        assert_eq!(session.project(), &before_project);
        assert_eq!(session.history.undo, before_undo);
        assert_eq!(session.history.redo, before_redo);
    }

    #[test]
    fn timeline_track_limit_rejects_the_next_track_without_mutation() {
        let mut session = fixed_session();
        for _ in 0..MAX_TIMELINE_TRACKS {
            add_track(&mut session, &TrackId::generate().to_string(), "video").unwrap();
        }
        let before_revision = session.project_revision();
        let before_history = session.history.undo.len();
        let error = add_track(&mut session, TRACK_A, "audio").unwrap_err();

        assert_eq!(error.code, OperationErrorCode::TimelineLimitExceeded);
        assert_eq!(
            session.project().timeline().tracks().len(),
            MAX_TIMELINE_TRACKS
        );
        assert_eq!(session.project_revision(), before_revision);
        assert_eq!(session.history.undo.len(), before_history);
    }

    #[test]
    fn timeline_track_remove_is_empty_only_and_undo_restores_exact_order() {
        let mut session = fixed_session();
        add_track(&mut session, TRACK_A, "video").unwrap();
        add_track(&mut session, TRACK_B, "audio").unwrap();
        add_track(&mut session, TRACK_C, "video").unwrap();
        let result = execute(
            &mut session,
            "timeline.track.remove",
            json!({"track_id": TRACK_B}),
        )
        .unwrap();
        assert_eq!(result.after_revision, ProjectRevision::new(4));
        let ids = session
            .project()
            .timeline()
            .tracks()
            .iter()
            .map(|track| track.id().to_string())
            .collect::<Vec<_>>();
        assert_eq!(ids, [TRACK_A, TRACK_C]);

        session.execute_command(undo(4)).unwrap();
        let ids = session
            .project()
            .timeline()
            .tracks()
            .iter()
            .map(|track| track.id().to_string())
            .collect::<Vec<_>>();
        assert_eq!(ids, [TRACK_A, TRACK_B, TRACK_C]);
        assert_eq!(session.project_revision(), ProjectRevision::new(5));
        session.execute_command(redo(5)).unwrap();
        assert_eq!(
            session.project().timeline().tracks()[1].id().to_string(),
            TRACK_C
        );
        assert_eq!(session.project_revision(), ProjectRevision::new(6));

        let error = execute(
            &mut session,
            "timeline.track.remove",
            json!({"track_id": TRACK_B}),
        )
        .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::TimelineTrackNotFound);
    }

    #[test]
    fn timeline_track_remove_rejects_nonempty_track_without_partial_change() {
        let mut session = fixed_session();
        seed_video_track_and_clip(&mut session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
        let before = session.project().clone();
        let before_undo = session.history.undo.clone();
        let error = execute(
            &mut session,
            "timeline.track.remove",
            json!({"track_id": TRACK_A}),
        )
        .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::TimelineTrackNotEmpty);
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo, before_undo);
    }

    #[test]
    fn timeline_clip_insert_sorts_exact_clips_and_undo_redo_restore_them() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (4, 1),
            (3, 2),
            (2, 1),
        )
        .unwrap();
        let result = insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_C,
            TRACK_A,
            MEDIA_A,
            (8, 1),
            (5, 2),
            (2, 1),
        )
        .unwrap();

        let clips = session.project().timeline().tracks()[0].clips();
        assert_eq!(
            clips
                .iter()
                .map(|clip| clip.id().to_string())
                .collect::<Vec<_>>(),
            [CLIP_A, CLIP_B, CLIP_C]
        );
        assert_eq!(clips[1].media_id().unwrap().to_string(), MEDIA_A);
        assert_eq!(clips[1].timeline_start(), RationalTime::new(4, 1).unwrap());
        assert_eq!(
            clips[1].source_range(),
            Some(
                TimeRange::new(
                    RationalTime::new(3, 2).unwrap(),
                    RationalTime::new(2, 1).unwrap()
                )
                .unwrap()
            )
        );
        assert_eq!(result.change_set.changes().len(), 1);
        assert_eq!(session.history.undo.last().unwrap().changes().len(), 1);
        let before_undo = session.project().timeline().clone();
        session
            .execute_command(undo(session.project_revision().value()))
            .unwrap();
        assert!(
            !session.project().timeline().tracks()[0]
                .clips()
                .iter()
                .any(|clip| clip.id().to_string() == CLIP_C)
        );
        session
            .execute_command(redo(session.project_revision().value()))
            .unwrap();
        assert_eq!(session.project().timeline(), &before_undo);
    }

    #[test]
    fn timeline_clip_overlap_is_rejected_and_adjacency_is_allowed() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (4, 1),
        )
        .unwrap();
        let before = session.project().clone();
        let before_revision = session.project_revision();
        let before_history = session.history.undo.clone();
        let error = insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (3, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::TimelineOverlap);
        assert_eq!(session.project(), &before);
        assert_eq!(session.project_revision(), before_revision);
        assert_eq!(session.history.undo, before_history);

        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (4, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap();
        assert_eq!(
            session.project().timeline().tracks()[0].clips()[1].timeline_start(),
            RationalTime::new(4, 1).unwrap()
        );
    }

    #[test]
    fn timeline_clip_validation_uses_compatible_stream_and_known_duration_bounds() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                video_media_item_with_duration(
                    MEDIA_A,
                    "file:///missing/video.mov",
                    Some(RationalTime::new(10, 1).unwrap()),
                    Some(RationalTime::new(5, 1).unwrap()),
                ),
                0,
            ))
            .unwrap();
        session
            .execute_command(media_add(
                audio_media_item(MEDIA_B, "file:///missing/audio.wav"),
                1,
            ))
            .unwrap();
        session
            .execute_command(media_add(
                video_media_item_with_duration(
                    MEDIA_C,
                    "file:///missing/container-duration.mov",
                    Some(RationalTime::new(5, 1).unwrap()),
                    None,
                ),
                2,
            ))
            .unwrap();
        session
            .execute_command(media_add(
                video_media_item_with_duration(MEDIA_D, "file:///missing/unknown.mov", None, None),
                3,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        add_track(&mut session, TRACK_B, "audio").unwrap();

        let missing = insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            (0, 1),
            (0, 1),
            (1, 1),
        )
        .unwrap_err();
        assert_eq!(missing.code, OperationErrorCode::MediaNotFound);

        for (track_id, media_id) in [(TRACK_A, MEDIA_B), (TRACK_B, MEDIA_A)] {
            let error = insert_clip(
                &mut session,
                CLIP_A,
                track_id,
                media_id,
                (0, 1),
                (0, 1),
                (1, 1),
            )
            .unwrap_err();
            assert_eq!(error.code, OperationErrorCode::TimelineMediaIncompatible);
        }

        let stream_overrun = insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (6, 1),
        )
        .unwrap_err();
        assert_eq!(stream_overrun.code, OperationErrorCode::InvalidArguments);

        let container_overrun = insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_C,
            (0, 1),
            (4, 1),
            (2, 1),
        )
        .unwrap_err();
        assert_eq!(container_overrun.code, OperationErrorCode::InvalidArguments);

        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_D,
            (0, 1),
            (100, 1),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_B,
            MEDIA_B,
            (0, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap();
        assert_eq!(
            session.project().timeline().tracks()[0].clips()[0]
                .media_id()
                .unwrap()
                .to_string(),
            MEDIA_D
        );
    }

    #[test]
    fn timeline_clip_move_rejects_both_cross_kind_directions_for_audio_video_media() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                audio_video_media_item(MEDIA_A, "file:///missing/audio-video.mkv"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        add_track(&mut session, TRACK_B, "audio").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (1, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_B,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (1, 1),
        )
        .unwrap();

        for (clip_id, destination) in [(CLIP_A, TRACK_B), (CLIP_B, TRACK_A)] {
            let before = session.project().clone();
            let before_undo = session.history.undo.clone();
            let before_revision = session.project_revision();
            let error = execute(
                &mut session,
                "timeline.clip.move",
                json!({
                    "clip_id": clip_id,
                    "track_id": destination,
                    "timeline_start": rational_json(2, 1),
                }),
            )
            .unwrap_err();
            assert_eq!(error.code, OperationErrorCode::InvalidArguments);
            assert_eq!(session.project(), &before);
            assert_eq!(session.history.undo, before_undo);
            assert_eq!(session.project_revision(), before_revision);
        }
    }

    #[test]
    fn timeline_clip_trim_uses_absolute_exact_edges_and_round_trips_history() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (1, 1),
            (6, 1),
        )
        .unwrap();
        let original = session.project().timeline().clone();

        let start = execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_A,
                "edge": "start",
                "timeline_time": rational_json(3, 1),
            }),
        )
        .unwrap();
        assert!(matches!(
            start.change_set.changes(),
            [ProjectChange::TimelineClipTrimmed { before, after, .. }]
                if before.timeline_start == RationalTime::new(2, 1).unwrap()
                    && after.timeline_start == RationalTime::new(3, 1).unwrap()
                    && after.source_range().unwrap().start() == RationalTime::new(2, 1).unwrap()
                    && after.source_range().unwrap().duration() == RationalTime::new(5, 1).unwrap()
        ));
        assert_eq!(
            start.after_revision,
            start.before_revision.checked_next().unwrap()
        );
        let trimmed = &session.project().timeline().tracks()[0].clips()[0];
        assert_eq!(trimmed.timeline_start(), RationalTime::new(3, 1).unwrap());
        assert_eq!(
            trimmed.source_range().unwrap().start(),
            RationalTime::new(2, 1).unwrap()
        );
        assert_eq!(
            trimmed.source_range().unwrap().duration(),
            RationalTime::new(5, 1).unwrap()
        );

        let end = execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_A,
                "edge": "end",
                "timeline_time": rational_json(7, 1),
            }),
        )
        .unwrap();
        assert!(end.changed);
        assert_eq!(
            session.project().timeline().tracks()[0].clips()[0].source_range(),
            Some(
                TimeRange::new(
                    RationalTime::new(2, 1).unwrap(),
                    RationalTime::new(4, 1).unwrap()
                )
                .unwrap()
            )
        );

        session
            .execute_command(undo(session.project_revision().value()))
            .unwrap();
        let no_op_revision = session.project_revision();
        let no_op_undo = session.history.undo.clone();
        let no_op_redo = session.history.redo.clone();
        let no_op = execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_A,
                "edge": "start",
                "timeline_time": rational_json(3, 1),
            }),
        )
        .unwrap();
        assert!(!no_op.changed);
        assert_eq!(session.project_revision(), no_op_revision);
        assert_eq!(session.history.undo, no_op_undo);
        assert_eq!(session.history.redo, no_op_redo);

        session
            .execute_command(undo(session.project_revision().value()))
            .unwrap();
        assert_eq!(session.project().timeline(), &original);
        session
            .execute_command(redo(session.project_revision().value()))
            .unwrap();
        assert_eq!(
            session.project().timeline().tracks()[0].clips()[0].timeline_start(),
            RationalTime::new(3, 1).unwrap()
        );
    }

    #[test]
    fn timeline_clip_trim_rejects_unknown_fields_overlap_negative_source_and_bounds_atomically() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (3, 1),
            (2, 1),
            (4, 1),
        )
        .unwrap();
        let before = session.project().clone();
        let before_undo = session.history.undo.clone();
        let overlap = execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_B,
                "edge": "start",
                "timeline_time": rational_json(1, 1),
            }),
        )
        .unwrap_err();
        assert_eq!(overlap.code, OperationErrorCode::TimelineOverlap);
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo, before_undo);

        let negative_source = execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_B,
                "edge": "start",
                "timeline_time": rational_json(0, 1),
            }),
        )
        .unwrap_err();
        assert_eq!(negative_source.code, OperationErrorCode::InvalidArguments);
        let strict = execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_B,
                "edge": "end",
                "timeline_time": rational_json(8, 1),
                "extra": true,
            }),
        )
        .unwrap_err();
        assert_eq!(strict.code, OperationErrorCode::InvalidArguments);
    }

    #[test]
    fn timeline_clip_trim_allows_exact_earlier_extension_and_enforces_source_bounds() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                video_media_item_with_duration(
                    MEDIA_A,
                    "file:///missing/bounded.mov",
                    Some(RationalTime::new(10, 1).unwrap()),
                    Some(RationalTime::new(10, 1).unwrap()),
                ),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (4, 1),
            (4, 1),
        )
        .unwrap();

        let extended = execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_A,
                "edge": "start",
                "timeline_time": rational_json(3, 2),
            }),
        )
        .unwrap();
        assert!(extended.changed);
        let clip = &session.project().timeline().tracks()[0].clips()[0];
        assert_eq!(clip.timeline_start(), RationalTime::new(3, 2).unwrap());
        assert_eq!(
            clip.source_range().unwrap().start(),
            RationalTime::new(7, 2).unwrap()
        );
        assert_eq!(
            clip.source_range().unwrap().duration(),
            RationalTime::new(9, 2).unwrap()
        );

        let before = session.project().clone();
        let before_history = session.history.clone();
        let overrun = execute(
            &mut session,
            "timeline.clip.trim",
            json!({
                "clip_id": CLIP_A,
                "edge": "end",
                "timeline_time": rational_json(9, 1),
            }),
        )
        .unwrap_err();
        assert_eq!(overrun.code, OperationErrorCode::InvalidArguments);
        assert_eq!(session.project(), &before);
        assert_eq!(session.history, before_history);
    }

    #[test]
    fn timeline_clip_split_partitions_exactly_and_restores_history() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (1, 1),
            (6, 1),
        )
        .unwrap();
        let original = session.project().timeline().clone();
        let split = execute(
            &mut session,
            "timeline.clip.split",
            json!({
                "clip_id": CLIP_A,
                "new_clip_id": CLIP_B,
                "timeline_time": rational_json(5, 1),
            }),
        )
        .unwrap();
        assert!(matches!(
            split.change_set.changes(),
            [ProjectChange::TimelineClipSplit { before, left_after, right_after, .. }]
                if before.clip_id == left_after.clip_id
                    && left_after.timeline_start == RationalTime::new(2, 1).unwrap()
                    && left_after.source_range() == Some(TimeRange::new(RationalTime::new(1, 1).unwrap(), RationalTime::new(3, 1).unwrap()).unwrap())
                    && right_after.clip_id == ClipId::from_str(CLIP_B).unwrap()
                    && right_after.timeline_start == RationalTime::new(5, 1).unwrap()
                    && right_after.source_range() == Some(TimeRange::new(RationalTime::new(4, 1).unwrap(), RationalTime::new(3, 1).unwrap()).unwrap())
        ));
        let clips = session.project().timeline().tracks()[0].clips();
        assert_eq!(clips.len(), 2);
        assert_eq!(clips[0].id().to_string(), CLIP_A);
        assert_eq!(clips[1].id().to_string(), CLIP_B);
        assert_eq!(
            clips[0].source_range().unwrap().duration(),
            RationalTime::new(3, 1).unwrap()
        );
        assert_eq!(
            clips[1].source_range().unwrap().start(),
            RationalTime::new(4, 1).unwrap()
        );
        assert_eq!(
            clips[1].source_range().unwrap().duration(),
            RationalTime::new(3, 1).unwrap()
        );

        session
            .execute_command(undo(session.project_revision().value()))
            .unwrap();
        assert_eq!(session.project().timeline(), &original);
        session
            .execute_command(redo(session.project_revision().value()))
            .unwrap();
        assert_eq!(session.project().timeline().tracks()[0].clips().len(), 2);

        let before = session.project().clone();
        let before_history = session.history.clone();
        for (time, expected) in [
            ((2, 1), OperationErrorCode::InvalidArguments),
            ((8, 1), OperationErrorCode::InvalidArguments),
        ] {
            let error = execute(
                &mut session,
                "timeline.clip.split",
                json!({
                    "clip_id": CLIP_A,
                    "new_clip_id": CLIP_C,
                    "timeline_time": rational_json(time.0, time.1),
                }),
            )
            .unwrap_err();
            assert_eq!(error.code, expected);
            assert_eq!(session.project(), &before);
            assert_eq!(session.history, before_history);
        }
        let duplicate = execute(
            &mut session,
            "timeline.clip.split",
            json!({
                "clip_id": CLIP_A,
                "new_clip_id": CLIP_B,
                "timeline_time": rational_json(3, 1),
            }),
        )
        .unwrap_err();
        assert_eq!(
            duplicate.code,
            OperationErrorCode::TimelineClipIdAlreadyExists
        );
        let before_strict = session.project().clone();
        let before_strict_history = session.history.clone();
        let strict = execute(
            &mut session,
            "timeline.clip.split",
            json!({
                "clip_id": CLIP_A,
                "new_clip_id": CLIP_C,
                "timeline_time": rational_json(3, 1),
                "extra": true,
            }),
        )
        .unwrap_err();
        assert_eq!(strict.code, OperationErrorCode::InvalidArguments);
        assert_eq!(session.project(), &before_strict);
        assert_eq!(session.history, before_strict_history);
    }

    #[test]
    fn timeline_clip_split_rejects_capacity_before_mutation() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        let clips = (0..MAX_TIMELINE_CLIPS)
            .map(|index| {
                TimelineClip::from_parts_for_codec(
                    if index == 0 {
                        ClipId::from_str(CLIP_A).unwrap()
                    } else {
                        ClipId::generate()
                    },
                    MediaId::from_str(MEDIA_A).unwrap(),
                    RationalTime::new(index as i64, 1).unwrap(),
                    TimeRange::new(
                        RationalTime::new(0, 1).unwrap(),
                        RationalTime::new(1, 1).unwrap(),
                    )
                    .unwrap(),
                )
            })
            .collect();
        session
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
                TRACK_A,
                TrackKind::Video,
                clips,
            )]));
        let before = session.project().clone();
        let before_history = session.history.clone();
        let error = execute(
            &mut session,
            "timeline.clip.split",
            json!({
                "clip_id": CLIP_A,
                "new_clip_id": CLIP_B,
                "timeline_time": rational_json(1, 2),
            }),
        )
        .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::TimelineLimitExceeded);
        assert_eq!(session.project(), &before);
        assert_eq!(session.history, before_history);
    }

    #[test]
    fn timeline_clip_ripple_delete_shifts_only_a_track_and_keeps_a_compact_recipe() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        add_track(&mut session, TRACK_B, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (4, 1),
            (2, 1),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_C,
            TRACK_A,
            MEDIA_A,
            (9, 1),
            (4, 1),
            (1, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_D,
            TRACK_B,
            MEDIA_A,
            (1, 1),
            (5, 1),
            (1, 1),
        )
        .unwrap();
        let before = session.project().timeline().clone();

        let strict_before = session.project().clone();
        let strict_history = session.history.clone();
        let strict = execute(
            &mut session,
            "timeline.clip.ripple_delete",
            json!({"clip_id": CLIP_B, "extra": true}),
        )
        .unwrap_err();
        assert_eq!(strict.code, OperationErrorCode::InvalidArguments);
        assert_eq!(session.project(), &strict_before);
        assert_eq!(session.history, strict_history);

        let result = execute(
            &mut session,
            "timeline.clip.ripple_delete",
            json!({"clip_id": CLIP_B}),
        )
        .unwrap();
        assert!(matches!(
            result.change_set.changes(),
            [ProjectChange::TimelineClipRippleDeleted { index: 1, shifted_count: 1, shift_duration, .. }]
                if *shift_duration == RationalTime::new(2, 1).unwrap()
        ));
        let encoded_change = serde_json::to_string(&result.change_set).unwrap();
        assert!(encoded_change.contains("shifted_count"));
        assert!(!encoded_change.contains("clips"));
        let track = &session.project().timeline().tracks()[0];
        assert_eq!(track.clips().len(), 2);
        assert_eq!(track.clips()[0].id().to_string(), CLIP_A);
        assert_eq!(
            track.clips()[0].timeline_start(),
            RationalTime::new(0, 1).unwrap()
        );
        assert_eq!(track.clips()[1].id().to_string(), CLIP_C);
        assert_eq!(
            track.clips()[1].timeline_start(),
            RationalTime::new(7, 1).unwrap()
        );
        assert_eq!(
            session.project().timeline().tracks()[1].clips()[0]
                .id()
                .to_string(),
            CLIP_D
        );
        assert_eq!(
            session.project().timeline().tracks()[1].clips()[0].timeline_start(),
            RationalTime::new(1, 1).unwrap()
        );

        session
            .execute_command(undo(session.project_revision().value()))
            .unwrap();
        assert_eq!(session.project().timeline(), &before);
        session
            .execute_command(redo(session.project_revision().value()))
            .unwrap();
        assert_eq!(
            session.project().timeline().tracks()[0].clips()[1]
                .id()
                .to_string(),
            CLIP_C
        );
        assert_eq!(
            session.project().timeline().tracks()[0].clips()[1].timeline_start(),
            RationalTime::new(7, 1).unwrap()
        );
    }

    #[test]
    fn timeline_clip_ripple_history_conflict_is_atomic_even_when_the_suffix_stays_valid() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (4, 1),
            (2, 1),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_C,
            TRACK_A,
            MEDIA_A,
            (9, 1),
            (4, 1),
            (1, 1),
        )
        .unwrap();
        execute(
            &mut session,
            "timeline.clip.ripple_delete",
            json!({"clip_id": CLIP_B}),
        )
        .unwrap();
        session
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
                TRACK_A,
                TrackKind::Video,
                vec![
                    fixture_clip(CLIP_A, MEDIA_A, (0, 1), (0, 1), (2, 1)),
                    fixture_clip(CLIP_C, MEDIA_A, (8, 1), (4, 1), (1, 1)),
                ],
            )]));
        assert_history_conflict(&mut session, "history.undo");
    }

    #[test]
    fn timeline_clip_ids_are_project_wide_and_clip_limits_are_checked_before_mutation() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        add_track(&mut session, TRACK_B, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap();

        let before = session.project().clone();
        let before_undo = session.history.undo.clone();
        let duplicate = insert_clip(
            &mut session,
            CLIP_A,
            TRACK_B,
            MEDIA_A,
            (0, 1),
            (2, 1),
            (2, 1),
        )
        .unwrap_err();
        assert_eq!(
            duplicate.code,
            OperationErrorCode::TimelineClipIdAlreadyExists
        );
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo, before_undo);

        let track_a =
            super::find_timeline_track_index(&session.project, TrackId::from_str(TRACK_A).unwrap())
                .unwrap();
        for (max_total, max_per_track) in [(0, 1), (1, 0)] {
            assert_eq!(
                super::ensure_timeline_clip_capacity(
                    &session.project,
                    track_a,
                    1,
                    1,
                    max_total,
                    max_per_track,
                )
                .unwrap_err()
                .code,
                OperationErrorCode::TimelineLimitExceeded
            );
        }
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo, before_undo);
    }

    #[test]
    fn timeline_clips_move_moves_a_selection_in_one_step_and_one_undo() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (1, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (1, 1),
            (1, 1),
        )
        .unwrap();
        let before = clip_spans(&session, TRACK_A);

        let moved = execute(
            &mut session,
            "timeline.clips.move",
            json!({
                "track_id": TRACK_A,
                "clip_ids": [CLIP_A, CLIP_B],
                "delta": rational_json(1, 1),
            }),
        )
        .unwrap();
        assert!(moved.changed);
        assert_eq!(moved.change_set.changes().len(), 1);
        let after = clip_spans(&session, TRACK_A);
        assert_eq!(after[0].1, RationalTime::new(1, 1).unwrap());
        assert_eq!(after[1].1, RationalTime::new(3, 1).unwrap());

        execute(&mut session, "history.undo", json!({})).unwrap();
        assert_eq!(
            clip_spans(&session, TRACK_A),
            before,
            "one undo restores the whole selection"
        );
    }

    #[test]
    fn timeline_clips_move_rejects_unknown_clip_and_a_start_below_zero() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (1, 1),
        )
        .unwrap();

        // A malformed id never reaches the handler: the envelope rejects it.
        let malformed = execute(
            &mut session,
            "timeline.clips.move",
            json!({
                "track_id": TRACK_A,
                "clip_ids": ["missing-clip"],
                "delta": rational_json(1, 1),
            }),
        );
        assert_eq!(
            malformed.unwrap_err().code,
            OperationErrorCode::InvalidArguments
        );

        // A well-formed id that is not on the track is the handler's own error.
        let unknown = execute(
            &mut session,
            "timeline.clips.move",
            json!({
                "track_id": TRACK_A,
                "clip_ids": ["00000000-0000-4000-8000-000000000000"],
                "delta": rational_json(1, 1),
            }),
        );
        assert_eq!(
            unknown.unwrap_err().code,
            OperationErrorCode::TimelineClipNotFound
        );

        let behind_zero = execute(
            &mut session,
            "timeline.clips.move",
            json!({
                "track_id": TRACK_A,
                "clip_ids": [CLIP_A],
                "delta": rational_json(-1, 1),
            }),
        );
        assert!(behind_zero.is_err());
    }

    fn timeline_clip_move_reorders_and_preserves_semantics_across_same_kind_tracks() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        add_track(&mut session, TRACK_B, "video").unwrap();
        add_track(&mut session, TRACK_C, "audio").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (1, 2),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (4, 1),
            (3, 2),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_C,
            TRACK_A,
            MEDIA_A,
            (8, 1),
            (5, 2),
            (2, 1),
        )
        .unwrap();

        let moved = execute(
            &mut session,
            "timeline.clip.move",
            json!({"clip_id": CLIP_A, "track_id": TRACK_A, "timeline_start": rational_json(10, 1)}),
        )
        .unwrap();
        assert!(moved.changed);
        assert_eq!(
            moved.after_revision.value(),
            moved.before_revision.value() + 1
        );
        assert_eq!(
            session.project().timeline().tracks()[0]
                .clips()
                .iter()
                .map(|clip| clip.id().to_string())
                .collect::<Vec<_>>(),
            [CLIP_B, CLIP_C, CLIP_A]
        );
        let moved_clip =
            TimelineClipState::from(&session.project().timeline().tracks()[0].clips()[2]);
        assert_eq!(moved_clip.media_id().unwrap().to_string(), MEDIA_A);
        assert_eq!(
            moved_clip.source_range().unwrap().start(),
            RationalTime::new(1, 2).unwrap()
        );
        assert_eq!(
            moved_clip.source_range().unwrap().duration(),
            RationalTime::new(2, 1).unwrap()
        );
        assert_eq!(moved_clip.timeline_start, RationalTime::new(10, 1).unwrap());

        session
            .execute_command(undo(session.project_revision().value()))
            .unwrap();
        assert_eq!(
            session.project().timeline().tracks()[0]
                .clips()
                .iter()
                .map(|clip| clip.id().to_string())
                .collect::<Vec<_>>(),
            [CLIP_A, CLIP_B, CLIP_C]
        );
        session
            .execute_command(redo(session.project_revision().value()))
            .unwrap();
        assert_eq!(
            session.project().timeline().tracks()[0].clips()[2].timeline_start(),
            RationalTime::new(10, 1).unwrap()
        );

        execute(
            &mut session,
            "timeline.clip.move",
            json!({"clip_id": CLIP_B, "track_id": TRACK_B, "timeline_start": rational_json(10, 1)}),
        )
        .unwrap();
        let destination = &session.project().timeline().tracks()[1].clips()[0];
        assert_eq!(destination.id().to_string(), CLIP_B);
        assert_eq!(destination.media_id().unwrap().to_string(), MEDIA_A);
        assert_eq!(
            destination.source_range().unwrap().start(),
            RationalTime::new(3, 2).unwrap()
        );
        assert_eq!(
            destination.timeline_start(),
            RationalTime::new(10, 1).unwrap()
        );
        assert!(
            session.project().timeline().tracks()[0]
                .clips()
                .iter()
                .all(|clip| clip.id().to_string() != CLIP_B)
        );
        session
            .execute_command(undo(session.project_revision().value()))
            .unwrap();
        assert_eq!(
            session.project().timeline().tracks()[0].clips()[0]
                .id()
                .to_string(),
            CLIP_B
        );
        assert!(session.project().timeline().tracks()[1].clips().is_empty());
        session
            .execute_command(redo(session.project_revision().value()))
            .unwrap();
        assert_eq!(
            session.project().timeline().tracks()[1].clips()[0]
                .id()
                .to_string(),
            CLIP_B
        );
        let before_media_remove = session.project().clone();
        assert_eq!(
            code(
                &session
                    .execute_command(media_remove(MEDIA_A, session.project_revision().value(),))
            ),
            OperationErrorCode::MediaInUse
        );
        assert_eq!(session.project(), &before_media_remove);

        let before = session.project().clone();
        let before_undo = session.history.undo.clone();
        let cross_kind = execute(
            &mut session,
            "timeline.clip.move",
            json!({"clip_id": CLIP_B, "track_id": TRACK_C, "timeline_start": rational_json(0, 1)}),
        )
        .unwrap_err();
        assert_eq!(cross_kind.code, OperationErrorCode::InvalidArguments);
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo, before_undo);
    }

    #[test]
    fn timeline_clip_move_overlap_failure_and_no_op_preserve_redo() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (4, 1),
            (2, 1),
            (2, 1),
        )
        .unwrap();
        execute(
            &mut session,
            "timeline.clip.move",
            json!({"clip_id": CLIP_B, "track_id": TRACK_A, "timeline_start": rational_json(6, 1)}),
        )
        .unwrap();
        session
            .execute_command(undo(session.project_revision().value()))
            .unwrap();
        assert_eq!(session.history.redo.len(), 1);
        let before = session.project().clone();
        let before_undo = session.history.undo.clone();
        let before_redo = session.history.redo.clone();
        let before_revision = session.project_revision();

        let overlap = execute(
            &mut session,
            "timeline.clip.move",
            json!({"clip_id": CLIP_B, "track_id": TRACK_A, "timeline_start": rational_json(1, 1)}),
        )
        .unwrap_err();
        assert_eq!(overlap.code, OperationErrorCode::TimelineOverlap);
        assert_eq!(session.project(), &before);
        assert_eq!(session.project_revision(), before_revision);
        assert_eq!(session.history.undo, before_undo);
        assert_eq!(session.history.redo, before_redo);

        let no_op = execute(
            &mut session,
            "timeline.clip.move",
            json!({"clip_id": CLIP_B, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
        )
        .unwrap();
        assert!(!no_op.changed);
        assert!(no_op.change_set.is_empty());
        assert_eq!(no_op.before_revision, before_revision);
        assert_eq!(no_op.after_revision, before_revision);
        assert_eq!(session.history.undo, before_undo);
        assert_eq!(session.history.redo, before_redo);

        let mut stale_no_op = history_command(
            "timeline.clip.move",
            before_revision.value().saturating_sub(1),
            json!({"clip_id": CLIP_B, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
        );
        stale_no_op.project_id = session.project_id();
        assert_eq!(
            code(&session.execute_command(stale_no_op)),
            OperationErrorCode::RevisionConflict
        );
        assert_eq!(session.history.redo, before_redo);

        session
            .execute_command(redo(session.project_revision().value()))
            .unwrap();
        assert_eq!(
            session.project().timeline().tracks()[0].clips()[1].timeline_start(),
            RationalTime::new(6, 1).unwrap()
        );
    }

    #[test]
    fn timeline_clip_delete_restores_exact_clip_and_media_remove_guard_is_preserved() {
        let mut session = fixed_session();
        seed_video_track_and_clip(&mut session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (4, 1),
            (2, 1),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_C,
            TRACK_A,
            MEDIA_A,
            (8, 1),
            (4, 1),
            (2, 1),
        )
        .unwrap();
        let expected =
            TimelineClipState::from(&session.project().timeline().tracks()[0].clips()[1]);
        let before_remove = session.project().clone();
        assert_eq!(
            code(
                &session.execute_command(media_remove(MEDIA_A, session.project_revision().value()))
            ),
            OperationErrorCode::MediaInUse
        );
        assert_eq!(session.project(), &before_remove);

        let deleted = execute(
            &mut session,
            "timeline.clip.delete",
            json!({"clip_id": CLIP_B}),
        )
        .unwrap();
        assert!(deleted.changed);
        assert_eq!(deleted.change_set.changes().len(), 1);
        assert_eq!(
            session.project().timeline().tracks()[0]
                .clips()
                .iter()
                .map(|clip| clip.id().to_string())
                .collect::<Vec<_>>(),
            [CLIP_A, CLIP_C]
        );
        assert_eq!(session.project().media_items().len(), 1);
        session
            .execute_command(undo(session.project_revision().value()))
            .unwrap();
        assert_eq!(
            TimelineClipState::from(&session.project().timeline().tracks()[0].clips()[1]),
            expected
        );
        assert_eq!(
            session.project().timeline().tracks()[0]
                .clips()
                .iter()
                .map(|clip| clip.id().to_string())
                .collect::<Vec<_>>(),
            [CLIP_A, CLIP_B, CLIP_C]
        );
        session
            .execute_command(redo(session.project_revision().value()))
            .unwrap();
        assert_eq!(
            session.project().timeline().tracks()[0]
                .clips()
                .iter()
                .map(|clip| clip.id().to_string())
                .collect::<Vec<_>>(),
            [CLIP_A, CLIP_C]
        );
        let still_in_use = session
            .execute_command(media_remove(MEDIA_A, session.project_revision().value()))
            .unwrap_err();
        assert_eq!(still_in_use.code, OperationErrorCode::MediaInUse);
        execute(
            &mut session,
            "timeline.clip.delete",
            json!({"clip_id": CLIP_A}),
        )
        .unwrap();
        execute(
            &mut session,
            "timeline.clip.delete",
            json!({"clip_id": CLIP_C}),
        )
        .unwrap();
        assert!(session.project().timeline().tracks()[0].clips().is_empty());
        let removed_media = session
            .execute_command(media_remove(MEDIA_A, session.project_revision().value()))
            .unwrap();
        assert!(removed_media.changed);
        assert!(session.project().media_items().is_empty());
    }

    #[test]
    fn timeline_queries_are_read_only_canonical_and_bounded() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        add_track(&mut session, TRACK_B, "audio").unwrap();
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (4, 1),
            (2, 1),
            (1, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (1, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_C,
            TRACK_A,
            MEDIA_A,
            (8, 1),
            (4, 1),
            (1, 1),
        )
        .unwrap();

        let before = session.project().clone();
        let undo = session.history.undo.clone();
        let tracks = session
            .execute_query(QueryEnvelope::timeline_tracks(
                session.project_id(),
                session.project_instance_id(),
            ))
            .unwrap();
        assert_eq!(tracks.summary.project_id, session.project_id());
        assert_eq!(
            tracks.summary.project_instance_id,
            session.project_instance_id()
        );
        assert_eq!(tracks.summary.project_revision, session.project_revision());
        assert_eq!(
            tracks.timeline_tracks.unwrap(),
            [
                TimelineTrackSummary {
                    track_id: TrackId::from_str(TRACK_A).unwrap(),
                    kind: TrackKind::Video,
                    clip_count: 3,
                },
                TimelineTrackSummary {
                    track_id: TrackId::from_str(TRACK_B).unwrap(),
                    kind: TrackKind::Audio,
                    clip_count: 0,
                },
            ]
        );

        let page = session
            .execute_query(QueryEnvelope::timeline_clips(
                session.project_id(),
                session.project_instance_id(),
                TrackId::from_str(TRACK_A).unwrap(),
                1,
                1,
            ))
            .unwrap()
            .timeline_clip_page
            .unwrap();
        assert_eq!(page.track_id.to_string(), TRACK_A);
        assert_eq!(page.items[0].clip_id.to_string(), CLIP_B);
        assert_eq!(
            page.items[0].timeline_start,
            RationalTime::new(4, 1).unwrap()
        );
        assert_eq!(page.total_count, 3);
        assert_eq!(page.offset, 1);
        assert_eq!(page.limit, 1);
        assert_eq!(page.next_offset, Some(2));

        let final_page = session
            .execute_query(QueryEnvelope::timeline_clips(
                session.project_id(),
                session.project_instance_id(),
                TrackId::from_str(TRACK_A).unwrap(),
                3,
                MAX_TIMELINE_CLIP_PAGE_SIZE,
            ))
            .unwrap()
            .timeline_clip_page
            .unwrap();
        assert!(final_page.items.is_empty());
        assert_eq!(final_page.next_offset, None);
        let beyond = session
            .execute_query(QueryEnvelope::timeline_clips(
                session.project_id(),
                session.project_instance_id(),
                TrackId::from_str(TRACK_A).unwrap(),
                4,
                1,
            ))
            .unwrap()
            .timeline_clip_page
            .unwrap();
        assert!(beyond.items.is_empty());

        let mut invalid = QueryEnvelope::timeline_clips(
            session.project_id(),
            session.project_instance_id(),
            TrackId::from_str(TRACK_A).unwrap(),
            0,
            MAX_TIMELINE_CLIP_PAGE_SIZE + 1,
        );
        assert_eq!(
            code(&session.execute_query(invalid.clone())),
            OperationErrorCode::InvalidArguments
        );
        invalid.arguments = json!({"track_id": TRACK_C, "offset": 0, "limit": 1});
        assert_eq!(
            code(&session.execute_query(invalid)),
            OperationErrorCode::TimelineTrackNotFound
        );
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo, undo);
    }

    #[test]
    fn typed_clip_commands_queries_locking_and_history_preserve_exact_time_and_identity() {
        let mut session = fixed_session();
        let text_track = TrackId::from_str(TRACK_A).unwrap();
        let caption_track = TrackId::from_str(TRACK_B).unwrap();
        for (track_id, kind) in [
            (text_track, TrackKind::Text),
            (caption_track, TrackKind::Caption),
        ] {
            session
                .execute_command(CommandEnvelope::add_timeline_track(
                    session.project_id(),
                    session.project_instance_id(),
                    session.project_revision(),
                    track_id,
                    kind,
                ))
                .unwrap();
        }

        let text_clip = ClipId::from_str(CLIP_A).unwrap();
        let initial_content = ClipContent::Text {
            text: "Title".to_owned(),
            formatting: TextFormatting::default(),
        };
        let visual_settings = ClipSettings::Visual(VisualSettings::default());
        let inserted = session
            .execute_command(CommandEnvelope::insert_timeline_clip_content(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                text_clip,
                text_track,
                RationalTime::new(1, 3).unwrap(),
                RationalTime::new(7, 3).unwrap(),
                initial_content.clone(),
                visual_settings.clone(),
            ))
            .unwrap();
        assert_eq!(inserted.after_revision, ProjectRevision::new(3));
        assert!(inserted.changed);

        let caption_clip = ClipId::from_str(CLIP_B).unwrap();
        session
            .execute_command(CommandEnvelope::insert_timeline_clip_content(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                caption_clip,
                caption_track,
                RationalTime::new(3, 2).unwrap(),
                RationalTime::new(1, 2).unwrap(),
                ClipContent::Caption {
                    text: "One caption cue".to_owned(),
                    formatting: TextFormatting::default(),
                },
                visual_settings.clone(),
            ))
            .unwrap();

        let page = session
            .execute_query(QueryEnvelope::timeline_clips_v2(
                session.project_id(),
                session.project_instance_id(),
                text_track,
                0,
                10,
            ))
            .unwrap()
            .timeline_clip_page_v2
            .unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].clip_id, text_clip);
        assert_eq!(
            page.items[0].timeline_start,
            RationalTime::new(1, 3).unwrap()
        );
        assert_eq!(
            page.items[0].timeline_duration,
            RationalTime::new(7, 3).unwrap()
        );
        assert_eq!(page.items[0].content, initial_content);
        assert_eq!(page.items[0].media_id(), None);
        assert_eq!(page.items[0].source_range(), None);

        let track_state = TrackState::new(true, true, false, false);
        session
            .execute_command(CommandEnvelope::set_timeline_track_state(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                text_track,
                track_state,
            ))
            .unwrap();
        let locked_update = CommandEnvelope::update_timeline_clip(
            session.project_id(),
            session.project_instance_id(),
            session.project_revision(),
            text_clip,
            RationalTime::new(3, 1).unwrap(),
            ClipContent::Text {
                text: "Edited".to_owned(),
                formatting: TextFormatting::default(),
            },
            visual_settings.clone(),
        );
        assert_eq!(
            code(&session.execute_command(locked_update)),
            OperationErrorCode::TimelineTrackLocked
        );
        assert_eq!(session.project_revision(), ProjectRevision::new(5));

        session
            .execute_command(CommandEnvelope::undo(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
            ))
            .unwrap();
        let updated_content = ClipContent::Text {
            text: "Edited".to_owned(),
            formatting: TextFormatting::default(),
        };
        session
            .execute_command(CommandEnvelope::update_timeline_clip(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                text_clip,
                RationalTime::new(3, 1).unwrap(),
                updated_content.clone(),
                visual_settings,
            ))
            .unwrap();
        let updated = &session.project().timeline().tracks()[0].clips()[0];
        assert_eq!(updated.id(), text_clip);
        assert_eq!(updated.timeline_start(), RationalTime::new(1, 3).unwrap());
        assert_eq!(
            updated.timeline_duration(),
            RationalTime::new(3, 1).unwrap()
        );
        assert_eq!(updated.content(), &updated_content);

        session
            .execute_command(CommandEnvelope::undo(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
            ))
            .unwrap();
        let restored = &session.project().timeline().tracks()[0].clips()[0];
        assert_eq!(restored.id(), text_clip);
        assert_eq!(
            restored.timeline_duration(),
            RationalTime::new(7, 3).unwrap()
        );
        assert_eq!(restored.content(), &initial_content);
        session
            .execute_command(CommandEnvelope::redo(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
            ))
            .unwrap();
        assert_eq!(
            session.project().timeline().tracks()[0].clips()[0].content(),
            &updated_content
        );
    }

    #[test]
    fn video_visual_settings_update_validation_history_and_serialization_are_exact() {
        let mut session = fixed_session();
        let media_id = MediaId::from_str(MEDIA_A).unwrap();
        let track_id = TrackId::from_str(TRACK_A).unwrap();
        let clip_id = ClipId::from_str(CLIP_A).unwrap();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/transform.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();

        let content = ClipContent::Media {
            media_id,
            source_range: TimeRange::new(
                RationalTime::new(1, 4).unwrap(),
                RationalTime::new(2, 1).unwrap(),
            )
            .unwrap(),
            playback_rate: RationalRate::ONE,
            reversed: false,
            frozen: false,
        };
        let duration = RationalTime::new(2, 1).unwrap();
        session
            .execute_command(CommandEnvelope::insert_timeline_clip_content(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                clip_id,
                track_id,
                RationalTime::new(3, 2).unwrap(),
                duration,
                content.clone(),
                ClipSettings::Visual(VisualSettings::default()),
            ))
            .unwrap();

        let visual = VisualSettings {
            transform: Transform {
                x_milli_canvas: 250,
                y_milli_canvas: -125,
                scale_x_milli: 1_500,
                scale_y_milli: 750,
                rotation_milli_degrees: 15_000,
                anchor_x_basis_points: 2_500,
                anchor_y_basis_points: 7_500,
            },
            crop: Crop {
                left_basis_points: 1_000,
                top_basis_points: 2_000,
                right_basis_points: 500,
                bottom_basis_points: 1_500,
            },
            opacity: Opacity {
                basis_points: 6_250,
            },
            ..VisualSettings::default()
        };
        session
            .execute_command(CommandEnvelope::update_timeline_clip(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                clip_id,
                duration,
                content.clone(),
                ClipSettings::Visual(visual.clone()),
            ))
            .unwrap();

        let read_settings = |session: &ProjectSession| {
            session
                .execute_query(QueryEnvelope::timeline_clips_v2(
                    session.project_id(),
                    session.project_instance_id(),
                    track_id,
                    0,
                    10,
                ))
                .unwrap()
                .timeline_clip_page_v2
                .unwrap()
                .items
                .remove(0)
        };
        let updated = read_settings(&session);
        assert_eq!(updated.timeline_start, RationalTime::new(3, 2).unwrap());
        assert_eq!(updated.timeline_duration, duration);
        assert_eq!(updated.content, content);
        assert_eq!(updated.settings, ClipSettings::Visual(visual.clone()));

        let invalid_before = session.project().clone();
        let invalid_history = session.history.clone();
        let mut invalid_visual = visual.clone();
        invalid_visual.transform.scale_x_milli = 0;
        let error = session
            .execute_command(CommandEnvelope::update_timeline_clip(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
                clip_id,
                duration,
                content.clone(),
                ClipSettings::Visual(invalid_visual),
            ))
            .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::InvalidArguments);
        assert_eq!(session.project(), &invalid_before);
        assert_eq!(session.history, invalid_history);

        let decoded = decode_project(&encode_project(session.project()).unwrap()).unwrap();
        assert_eq!(
            decoded.timeline().tracks()[0].clips()[0].settings(),
            &ClipSettings::Visual(visual.clone())
        );

        session
            .execute_command(CommandEnvelope::undo(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
            ))
            .unwrap();
        assert_eq!(
            read_settings(&session).settings,
            ClipSettings::Visual(VisualSettings::default())
        );
        session
            .execute_command(CommandEnvelope::redo(
                session.project_id(),
                session.project_instance_id(),
                session.project_revision(),
            ))
            .unwrap();
        assert_eq!(
            read_settings(&session).settings,
            ClipSettings::Visual(visual)
        );
    }

    fn snap_fixture() -> ProjectSession {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/snap.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        add_track(&mut session, TRACK_B, "video").unwrap();
        add_track(&mut session, TRACK_C, "audio").unwrap();
        add_track(&mut session, TRACK_D, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (2, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_B,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (1, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_C,
            TRACK_B,
            MEDIA_A,
            (4, 1),
            (1, 1),
            (1, 2),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_D,
            TRACK_B,
            MEDIA_A,
            (49, 10),
            (2, 1),
            (1, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            CLIP_E,
            TRACK_B,
            MEDIA_A,
            (61, 10),
            (3, 1),
            (1, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaac",
            TRACK_B,
            MEDIA_A,
            (8, 1),
            (4, 1),
            (1, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaad",
            TRACK_B,
            MEDIA_A,
            (9, 1),
            (5, 1),
            (1, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbc",
            TRACK_D,
            MEDIA_A,
            (9, 1),
            (6, 1),
            (1, 1),
        )
        .unwrap();
        insert_clip(
            &mut session,
            "cccccccc-cccc-4ccc-8ccc-cccccccccccd",
            TRACK_D,
            MEDIA_A,
            (43, 5),
            (7, 1),
            (1, 5),
        )
        .unwrap();
        session
    }

    #[test]
    fn timeline_snap_resolves_zero_clip_boundaries_and_move_end_anchor_exactly() {
        let session = snap_fixture();
        let before_revision = session.project_revision();

        let zero = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::Move,
                CLIP_A,
                Some(TRACK_B),
                (1, 20),
            ))
            .unwrap();
        let zero = zero.timeline_snap.unwrap();
        assert_eq!(zero.resolved_target_time, RationalTime::ZERO);
        assert!(zero.snapped);
        assert_eq!(zero.moving_anchor, TimelineSnapMovingAnchor::Start);
        assert_eq!(zero.target_kind, TimelineSnapTargetKind::TimelineZero);
        assert_eq!(zero.target_time, RationalTime::ZERO);
        assert_eq!(zero.target_track_id, None);
        assert_eq!(zero.target_clip_id, None);

        let clip_start = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::Move,
                CLIP_A,
                Some(TRACK_B),
                (161, 20),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert_eq!(
            clip_start.resolved_target_time,
            RationalTime::new(8, 1).unwrap()
        );
        assert_eq!(clip_start.target_kind, TimelineSnapTargetKind::ClipStart);
        assert_eq!(clip_start.target_track_id.unwrap().to_string(), TRACK_B);
        assert_eq!(
            clip_start.target_clip_id.unwrap().to_string(),
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaac"
        );

        let outside = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::Move,
                CLIP_A,
                Some(TRACK_B),
                (51, 5),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert!(!outside.snapped);
        assert_eq!(
            outside.resolved_target_time,
            RationalTime::new(51, 5).unwrap()
        );
        assert_eq!(outside.moving_anchor, TimelineSnapMovingAnchor::None);
        assert_eq!(outside.target_kind, TimelineSnapTargetKind::None);

        let end_anchor = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::Move,
                CLIP_A,
                Some(TRACK_B),
                (13, 2),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert!(end_anchor.snapped);
        assert_eq!(end_anchor.moving_anchor, TimelineSnapMovingAnchor::End);
        assert_eq!(end_anchor.target_time, RationalTime::new(43, 5).unwrap());
        assert_eq!(
            end_anchor.resolved_target_time,
            RationalTime::new(33, 5).unwrap()
        );

        let trim_start = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::TrimStart,
                CLIP_A,
                None,
                (1, 20),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert_eq!(trim_start.moving_anchor, TimelineSnapMovingAnchor::Start);
        assert_eq!(trim_start.target_kind, TimelineSnapTargetKind::TimelineZero);
        assert_eq!(trim_start.resolved_target_time, RationalTime::ZERO);

        let trim_end = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::TrimEnd,
                CLIP_A,
                None,
                (4, 1),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert_eq!(trim_end.moving_anchor, TimelineSnapMovingAnchor::End);
        assert_eq!(trim_end.target_kind, TimelineSnapTargetKind::ClipStart);
        assert_eq!(trim_end.target_time, RationalTime::new(4, 1).unwrap());
        assert_eq!(session.project_revision(), before_revision);
    }

    #[test]
    fn timeline_snap_tie_breaks_are_deterministic_and_exclude_the_active_clip() {
        let session = snap_fixture();
        let candidate_tie = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::Move,
                CLIP_A,
                Some(TRACK_B),
                (9, 1),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert_eq!(candidate_tie.target_kind, TimelineSnapTargetKind::ClipEnd);
        assert_eq!(
            candidate_tie.target_clip_id.unwrap().to_string(),
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaac"
        );
        assert_eq!(candidate_tie.target_track_id.unwrap().to_string(), TRACK_B);

        let self_excluded = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::TrimStart,
                CLIP_A,
                None,
                (2, 1),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert!(!self_excluded.snapped);

        let mut short = fixed_session();
        short
            .execute_command(media_add(media_item(MEDIA_A, "file:///missing/tie.mov"), 0))
            .unwrap();
        add_track(&mut short, TRACK_A, "video").unwrap();
        add_track(&mut short, TRACK_B, "video").unwrap();
        insert_clip(&mut short, CLIP_A, TRACK_A, MEDIA_A, (5, 1), (0, 1), (1, 5)).unwrap();
        insert_clip(
            &mut short,
            CLIP_B,
            TRACK_B,
            MEDIA_A,
            (51, 10),
            (1, 1),
            (1, 1),
        )
        .unwrap();
        let anchor_tie = short
            .execute_query(snap_query(
                &short,
                super::TimelineSnapOperation::Move,
                CLIP_A,
                Some(TRACK_B),
                (5, 1),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert_eq!(anchor_tie.moving_anchor, TimelineSnapMovingAnchor::Start);
    }

    #[test]
    fn timeline_snap_scans_unpaged_canonical_clips_and_keeps_query_read_only() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/unloaded.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        add_track(&mut session, TRACK_B, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (1, 1),
        )
        .unwrap();
        for index in 0..101 {
            let clip_id = ClipId::generate().to_string();
            insert_clip(
                &mut session,
                &clip_id,
                TRACK_B,
                MEDIA_A,
                ((20 + index * 2) as i64, 1),
                (0, 1),
                (1, 1),
            )
            .unwrap();
        }
        let before_revision = session.project_revision();
        let before_history = session.history.undo.clone();
        let result = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::Move,
                CLIP_A,
                Some(TRACK_B),
                (220, 1),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert!(result.snapped);
        assert_eq!(result.target_kind, TimelineSnapTargetKind::ClipStart);
        assert_eq!(result.target_time, RationalTime::new(220, 1).unwrap());
        assert_eq!(
            result.resolved_target_time,
            RationalTime::new(220, 1).unwrap()
        );
        assert_eq!(session.project_revision(), before_revision);
        assert_eq!(session.history.undo, before_history);
    }

    #[test]
    fn timeline_snap_preconditions_and_arguments_are_strict() {
        let session = snap_fixture();
        let before_revision = session.project_revision();
        let base = json!({
            "operation": "move",
            "clip_id": CLIP_A,
            "target_track_id": TRACK_B,
            "target_time": rational_json(0, 1),
        });
        let mut unknown = QueryEnvelope::timeline_snap(
            session.project_id(),
            session.project_instance_id(),
            super::TimelineSnapOperation::Move,
            ClipId::from_str(CLIP_A).unwrap(),
            Some(TrackId::from_str(TRACK_B).unwrap()),
            RationalTime::ZERO,
        );
        unknown.arguments = json!({
            "operation": "move",
            "clip_id": CLIP_A,
            "target_track_id": TRACK_B,
            "target_time": rational_json(0, 1),
            "threshold": rational_json(1, 8),
        });
        assert_eq!(
            code(&session.execute_query(unknown)),
            OperationErrorCode::InvalidArguments
        );

        let mut missing_track = QueryEnvelope::timeline_snap(
            session.project_id(),
            session.project_instance_id(),
            super::TimelineSnapOperation::Move,
            ClipId::from_str(CLIP_A).unwrap(),
            None,
            RationalTime::ZERO,
        );
        missing_track.arguments = base.clone();
        missing_track.arguments["target_track_id"] = Value::Null;
        assert_eq!(
            code(&session.execute_query(missing_track)),
            OperationErrorCode::InvalidArguments
        );

        let trim_track = QueryEnvelope::timeline_snap(
            session.project_id(),
            session.project_instance_id(),
            super::TimelineSnapOperation::TrimStart,
            ClipId::from_str(CLIP_A).unwrap(),
            Some(TrackId::from_str(TRACK_B).unwrap()),
            RationalTime::ZERO,
        );
        assert_eq!(
            code(&session.execute_query(trim_track)),
            OperationErrorCode::InvalidArguments
        );

        let wrong_kind = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::Move,
                CLIP_A,
                Some(TRACK_C),
                (0, 1),
            ))
            .unwrap_err();
        assert_eq!(wrong_kind.code, OperationErrorCode::InvalidArguments);

        let missing_clip = session
            .execute_query(snap_query(
                &session,
                super::TimelineSnapOperation::Move,
                "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbd",
                Some(TRACK_B),
                (0, 1),
            ))
            .unwrap_err();
        assert_eq!(missing_clip.code, OperationErrorCode::TimelineClipNotFound);
        assert_eq!(session.project_revision(), before_revision);
    }

    #[test]
    fn timeline_arguments_are_strict_and_invalid_times_never_mutate() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        let base = json!({
            "clip_id": CLIP_A,
            "track_id": TRACK_A,
            "media_id": MEDIA_A,
            "timeline_start": {"numerator": 0, "denominator": 1},
            "source_range": {
                "start": {"numerator": 0, "denominator": 1},
                "duration": {"numerator": 2, "denominator": 1},
            }
        });
        let mut invalids = vec![json!({})];
        let mut invalid = base.clone();
        invalid.as_object_mut().unwrap().remove("clip_id");
        invalids.push(invalid);
        let mut invalid = base.clone();
        invalid["unknown"] = json!(true);
        invalids.push(invalid);
        let mut invalid = base.clone();
        invalid["timeline_start"] = json!("0/1");
        invalids.push(invalid);
        let mut invalid = base.clone();
        invalid["timeline_start"]["extra"] = json!(1);
        invalids.push(invalid);
        let mut invalid = base.clone();
        invalid["source_range"]["start"]["denominator"] = json!(0);
        invalids.push(invalid);
        let mut invalid = base.clone();
        invalid["clip_id"] = json!("bad");
        invalids.push(invalid);
        let mut invalid = base.clone();
        invalid["track_id"] = json!("bad");
        invalids.push(invalid);
        let mut invalid = base.clone();
        invalid["media_id"] = json!("bad");
        invalids.push(invalid);
        for field in ["timeline_start", "source_range.start"] {
            let mut invalid = base.clone();
            let time = if field == "timeline_start" {
                &mut invalid["timeline_start"]
            } else {
                &mut invalid["source_range"]["start"]
            };
            time["numerator"] = json!(-1);
            invalids.push(invalid);
        }
        for duration in [0, -1] {
            let mut invalid = base.clone();
            invalid["source_range"]["duration"]["numerator"] = json!(duration);
            invalids.push(invalid);
        }
        for field in ["timeline_start", "source_range.start"] {
            let mut invalid = base.clone();
            let time = if field == "timeline_start" {
                &mut invalid["timeline_start"]
            } else {
                &mut invalid["source_range"]["start"]
            };
            time["numerator"] = json!(i64::MAX);
            invalids.push(invalid);
        }

        let before = session.project().clone();
        let undo = session.history.undo.clone();
        for arguments in invalids {
            assert_eq!(
                code(&execute(&mut session, "timeline.clip.insert", arguments)),
                OperationErrorCode::InvalidArguments
            );
            assert_eq!(session.project(), &before);
            assert_eq!(session.history.undo, undo);
        }
    }

    #[test]
    fn every_timeline_mutation_is_rejected_from_grouped_transactions() {
        for command_id in [
            "timeline.track.add",
            "timeline.track.remove",
            "timeline.track.move",
            "timeline.clip.insert",
            "timeline.clip.move",
            "timeline.clip.delete",
            "timeline.clip.trim",
            "timeline.clip.slip",
            "timeline.clip.roll",
            "timeline.clip.slide",
            "timeline.clip.split",
            "timeline.clip.ripple_delete",
        ] {
            let mut session = fixed_session();
            let before = session.project().clone();
            let error = session
                .execute_transaction(transaction(vec![call(command_id, 1, json!({}))], 0))
                .unwrap_err();
            assert_eq!(
                error.code,
                OperationErrorCode::CommandNotAllowedInTransaction
            );
            assert_eq!(session.project(), &before);
            assert!(session.history.undo.is_empty());
            assert!(session.history.redo.is_empty());
        }
    }

    #[test]
    fn representative_timeline_mutation_checks_project_instance_and_revision_preconditions() {
        let mut session = fixed_session();
        let mut command = history_command(
            "timeline.track.add",
            0,
            json!({"track_id": TRACK_A, "kind": "video"}),
        );
        command.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
        assert_eq!(
            code(&session.execute_command(command.clone())),
            OperationErrorCode::ProjectIdMismatch
        );
        command.project_id = session.project_id();
        command.project_instance_id = ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
        assert_eq!(
            code(&session.execute_command(command.clone())),
            OperationErrorCode::ProjectInstanceMismatch
        );
        command.project_instance_id = session.project_instance_id();
        command.expected_project_revision = ProjectRevision::new(1);
        assert_eq!(
            code(&session.execute_command(command)),
            OperationErrorCode::RevisionConflict
        );
        assert!(session.project().timeline().tracks().is_empty());
        assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
        assert!(session.history.undo.is_empty());
    }

    #[test]
    fn timeline_history_chain_undoes_and_redoes_each_semantic_edit_once() {
        let mut session = fixed_session();
        session
            .execute_command(media_add(
                media_item(MEDIA_A, "file:///missing/offline.mov"),
                0,
            ))
            .unwrap();
        add_track(&mut session, TRACK_A, "video").unwrap();
        insert_clip(
            &mut session,
            CLIP_A,
            TRACK_A,
            MEDIA_A,
            (0, 1),
            (0, 1),
            (2, 1),
        )
        .unwrap();
        execute(
            &mut session,
            "timeline.clip.move",
            json!({"clip_id": CLIP_A, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
        )
        .unwrap();
        execute(
            &mut session,
            "timeline.clip.delete",
            json!({"clip_id": CLIP_A}),
        )
        .unwrap();
        assert_eq!(session.history.undo.len(), 5);
        assert!(session.project().timeline().tracks()[0].clips().is_empty());

        let mut observed_revisions = Vec::new();
        for _ in 0..4 {
            let result = session
                .execute_command(undo(session.project_revision().value()))
                .unwrap();
            observed_revisions.push(result.after_revision.value());
        }
        assert_eq!(observed_revisions, [6, 7, 8, 9]);
        assert!(session.project().timeline().tracks().is_empty());
        assert_eq!(session.history.redo.len(), 4);

        for expected_revision in 10..=13 {
            let result = session
                .execute_command(redo(session.project_revision().value()))
                .unwrap();
            assert_eq!(result.after_revision.value(), expected_revision);
        }
        assert_eq!(session.project_revision(), ProjectRevision::new(13));
        assert!(session.project().timeline().tracks()[0].clips().is_empty());
        assert!(session.history.undo.len() >= 4);
        assert!(session.history.redo.is_empty());
    }

    #[test]
    fn timeline_real_edit_clears_redo_history() {
        let mut session = fixed_session();
        seed_video_track_and_clip(&mut session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
        execute(
            &mut session,
            "timeline.clip.move",
            json!({"clip_id": CLIP_A, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
        )
        .unwrap();
        session
            .execute_command(undo(session.project_revision().value()))
            .unwrap();
        assert_eq!(session.history.redo.len(), 1);
        insert_clip(
            &mut session,
            CLIP_B,
            TRACK_A,
            MEDIA_A,
            (4, 1),
            (2, 1),
            (2, 1),
        )
        .unwrap();
        assert!(session.history.redo.is_empty());
    }

    #[test]
    fn timeline_history_conflicts_leave_project_and_both_stacks_unchanged() {
        let mut added_track = fixed_session();
        add_track(&mut added_track, TRACK_A, "video").unwrap();
        added_track
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
                TRACK_B,
                TrackKind::Audio,
                Vec::new(),
            )]));
        assert_history_conflict(&mut added_track, "history.undo");

        let mut removed_track = fixed_session();
        add_track(&mut removed_track, TRACK_A, "video").unwrap();
        add_track(&mut removed_track, TRACK_B, "audio").unwrap();
        execute(
            &mut removed_track,
            "timeline.track.remove",
            json!({"track_id": TRACK_B}),
        )
        .unwrap();
        removed_track
            .execute_command(undo(removed_track.project_revision().value()))
            .unwrap();
        removed_track
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![
                fixture_track(TRACK_A, TrackKind::Video, Vec::new()),
                fixture_track(TRACK_B, TrackKind::Video, Vec::new()),
            ]));
        assert_history_conflict(&mut removed_track, "history.redo");

        let mut inserted_clip = fixed_session();
        seed_video_track_and_clip(&mut inserted_clip, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
        inserted_clip
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
                TRACK_A,
                TrackKind::Video,
                vec![fixture_clip(CLIP_A, MEDIA_A, (1, 1), (0, 1), (2, 1))],
            )]));
        assert_history_conflict(&mut inserted_clip, "history.undo");

        let mut deleted_clip = fixed_session();
        seed_video_track_and_clip(&mut deleted_clip, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
        execute(
            &mut deleted_clip,
            "timeline.clip.delete",
            json!({"clip_id": CLIP_A}),
        )
        .unwrap();
        deleted_clip
            .execute_command(undo(deleted_clip.project_revision().value()))
            .unwrap();
        deleted_clip
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
                TRACK_A,
                TrackKind::Video,
                vec![fixture_clip(CLIP_A, MEDIA_A, (1, 1), (0, 1), (2, 1))],
            )]));
        assert_history_conflict(&mut deleted_clip, "history.redo");

        let mut moved_clip = fixed_session();
        seed_video_track_and_clip(&mut moved_clip, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (2, 1));
        add_track(&mut moved_clip, TRACK_B, "video").unwrap();
        execute(
            &mut moved_clip,
            "timeline.clip.move",
            json!({"clip_id": CLIP_A, "track_id": TRACK_B, "timeline_start": rational_json(4, 1)}),
        )
        .unwrap();
        moved_clip
            .execute_command(undo(moved_clip.project_revision().value()))
            .unwrap();
        moved_clip
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![
                fixture_track(
                    TRACK_A,
                    TrackKind::Video,
                    vec![fixture_clip(CLIP_A, MEDIA_A, (1, 1), (0, 1), (2, 1))],
                ),
                fixture_track(TRACK_B, TrackKind::Video, Vec::new()),
            ]));
        assert_history_conflict(&mut moved_clip, "history.redo");
    }

    fn assert_history_conflict(session: &mut ProjectSession, command_id: &str) {
        let before_project = session.project().clone();
        let before_undo = session.history.undo.clone();
        let before_redo = session.history.redo.clone();
        let before_revision = session.project_revision();
        let result = session.execute_command(history_command(
            command_id,
            before_revision.value(),
            json!({}),
        ));
        assert_eq!(code(&result), OperationErrorCode::HistoryConflict);
        assert_eq!(session.project(), &before_project);
        assert_eq!(session.history.undo, before_undo);
        assert_eq!(session.history.redo, before_redo);
        assert_eq!(session.project_revision(), before_revision);
    }

    #[test]
    fn every_timeline_edit_rejects_revision_overflow_before_mutation() {
        let mut add = session_with_state("A", u64::MAX);
        let add_before = add.project().clone();
        assert_eq!(
            code(&add.execute_command(history_command(
                "timeline.track.add",
                u64::MAX,
                json!({"track_id": TRACK_A, "kind": "video"}),
            ))),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(add.project(), &add_before);

        let mut remove = session_with_state("A", u64::MAX);
        remove
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
                TRACK_A,
                TrackKind::Video,
                Vec::new(),
            )]));
        let remove_before = remove.project().clone();
        assert_eq!(
            code(&execute(
                &mut remove,
                "timeline.track.remove",
                json!({"track_id": TRACK_A})
            )),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(remove.project(), &remove_before);

        let mut insert = max_revision_session_with_clip(false);
        let insert_before = insert.project().clone();
        assert_eq!(
            code(&insert_clip(
                &mut insert,
                CLIP_B,
                TRACK_A,
                MEDIA_A,
                (4, 1),
                (2, 1),
                (2, 1)
            )),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(insert.project(), &insert_before);

        let mut move_clip = max_revision_session_with_clip(true);
        let move_before = move_clip.project().clone();
        assert_eq!(
            code(&execute(
                &mut move_clip,
                "timeline.clip.move",
                json!({"clip_id": CLIP_A, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
            )),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(move_clip.project(), &move_before);

        let mut delete_clip = max_revision_session_with_clip(true);
        let delete_before = delete_clip.project().clone();
        assert_eq!(
            code(&execute(
                &mut delete_clip,
                "timeline.clip.delete",
                json!({"clip_id": CLIP_A})
            )),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(delete_clip.project(), &delete_before);
        let mut trim_clip = max_revision_session_with_clip(true);
        let trim_before = trim_clip.project().clone();
        assert_eq!(
            code(&execute(
                &mut trim_clip,
                "timeline.clip.trim",
                json!({
                    "clip_id": CLIP_A,
                    "edge": "end",
                    "timeline_time": rational_json(1, 1),
                }),
            )),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(trim_clip.project(), &trim_before);

        let mut split_clip = max_revision_session_with_clip(true);
        let split_before = split_clip.project().clone();
        assert_eq!(
            code(&execute(
                &mut split_clip,
                "timeline.clip.split",
                json!({
                    "clip_id": CLIP_A,
                    "new_clip_id": CLIP_B,
                    "timeline_time": rational_json(1, 1),
                }),
            )),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(split_clip.project(), &split_before);

        let mut ripple_delete_clip = max_revision_session_with_clip(true);
        let ripple_delete_before = ripple_delete_clip.project().clone();
        assert_eq!(
            code(&execute(
                &mut ripple_delete_clip,
                "timeline.clip.ripple_delete",
                json!({"clip_id": CLIP_A}),
            )),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(ripple_delete_clip.project(), &ripple_delete_before);

        for session in [
            &add,
            &remove,
            &insert,
            &move_clip,
            &delete_clip,
            &trim_clip,
            &split_clip,
            &ripple_delete_clip,
        ] {
            assert_eq!(session.project_revision(), ProjectRevision::new(u64::MAX));
            assert!(session.history.undo.is_empty());
            assert!(session.history.redo.is_empty());
        }
    }

    fn max_revision_session_with_clip(include_clip: bool) -> ProjectSession {
        let mut session = session_with_state("A", u64::MAX);
        session.project.insert_media_for_command(
            media_item(MEDIA_A, "file:///missing/offline.mov"),
            0,
            ProjectRevision::new(u64::MAX),
        );
        let clips = if include_clip {
            vec![fixture_clip(CLIP_A, MEDIA_A, (0, 1), (0, 1), (2, 1))]
        } else {
            Vec::new()
        };
        session
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![fixture_track(
                TRACK_A,
                TrackKind::Video,
                clips,
            )]));
        session
    }

    #[test]
    fn media_add_mutates_once_and_reports_the_persisted_item() {
        let mut session = fixed_session();
        let item = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/one.mkv",
        );

        let result = session.execute_command(media_add(item.clone(), 0)).unwrap();

        assert_eq!(result.before_revision, ProjectRevision::new(0));
        assert_eq!(result.after_revision, ProjectRevision::new(1));
        assert!(result.changed);
        assert_eq!(session.project().media_items(), std::slice::from_ref(&item));
        assert_eq!(session.history.undo.len(), 1);
        assert!(session.history.redo.is_empty());
        assert!(matches!(
            result.change_set.changes(),
            [ProjectChange::MediaAdded { item: added, index: 0 }] if added == &item
        ));
        let persisted = decode_project(&encode_project(session.project()).unwrap()).unwrap();
        assert_eq!(persisted.media_items(), &[item]);
        assert_eq!(persisted.revision(), ProjectRevision::new(1));
    }

    #[test]
    fn duplicate_media_id_and_source_are_rejected_without_state_change() {
        let mut session = fixed_session();
        let original = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/one.mkv",
        );
        session
            .execute_command(media_add(original.clone(), 0))
            .unwrap();
        let before = session.project().clone();
        let history_len = session.history.undo.len();

        let duplicate_id = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/two.mkv",
        );
        assert_eq!(
            code(&session.execute_command(media_add(duplicate_id, 1))),
            OperationErrorCode::MediaIdAlreadyExists
        );

        let duplicate_source = media_item(
            "00000000-0000-4000-8000-000000000002",
            "file:///media/one.mkv",
        );
        assert_eq!(
            code(&session.execute_command(media_add(duplicate_source, 1))),
            OperationErrorCode::MediaSourceAlreadyExists
        );
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo.len(), history_len);
        assert!(session.history.redo.is_empty());
    }

    #[test]
    fn stale_prepared_media_add_returns_revision_conflict_without_retry() {
        let mut session = fixed_session();
        let prepared = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/one.mkv",
        );
        let captured_revision_command = media_add(prepared, 0);
        session
            .execute_command(rename("Changed during probe", 0))
            .unwrap();
        let before = session.project().clone();
        let undo_len = session.history.undo.len();

        assert_eq!(
            code(&session.execute_command(captured_revision_command)),
            OperationErrorCode::RevisionConflict
        );
        assert_eq!(session.project(), &before);
        assert_eq!(session.project_revision(), ProjectRevision::new(1));
        assert_eq!(session.history.undo.len(), undo_len);
        assert!(session.history.redo.is_empty());
    }

    #[test]
    fn media_add_rejects_invalid_arguments_and_is_not_transactional() {
        let mut session = fixed_session();
        for arguments in [json!({}), json!({ "item": {}, "extra": true })] {
            let mut command = media_add(
                media_item(
                    "00000000-0000-4000-8000-000000000001",
                    "file:///media/one.mkv",
                ),
                0,
            );
            command.arguments = arguments;
            assert_eq!(
                code(&session.execute_command(command)),
                OperationErrorCode::InvalidArguments
            );
        }
        let result = session.execute_transaction(transaction(
            vec![call(
                "media.add",
                1,
                json!({ "item": media_item(
                    "00000000-0000-4000-8000-000000000001",
                    "file:///media/one.mkv"
                ) }),
            )],
            0,
        ));
        assert_eq!(
            code(&result),
            OperationErrorCode::CommandNotAllowedInTransaction
        );
        assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
        assert!(session.project().media_items().is_empty());
        assert!(session.history.undo.is_empty());
    }

    #[test]
    fn media_remove_rejects_unknown_id_and_success_increments_once() {
        let mut session = fixed_session();
        assert_eq!(
            code(&session.execute_command(media_remove("00000000-0000-4000-8000-000000000001", 0))),
            OperationErrorCode::MediaNotFound
        );
        let item = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/one.mkv",
        );
        session.execute_command(media_add(item.clone(), 0)).unwrap();

        let result = session
            .execute_command(media_remove("00000000-0000-4000-8000-000000000001", 1))
            .unwrap();
        assert_eq!(result.before_revision, ProjectRevision::new(1));
        assert_eq!(result.after_revision, ProjectRevision::new(2));
        assert!(session.project().media_items().is_empty());
        assert!(matches!(
            result.change_set.changes(),
            [ProjectChange::MediaRemoved { item: removed, index: 0 }] if removed == &item
        ));
        assert_eq!(
            code(&session.execute_command(media_remove("00000000-0000-4000-8000-000000000001", 2))),
            OperationErrorCode::MediaNotFound
        );
        assert_eq!(session.project_revision(), ProjectRevision::new(2));
    }

    #[test]
    fn media_relink_preserves_identity_and_timeline_references_through_history() {
        let mut session = fixed_session();
        let original = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/original.mkv",
        );
        let replacement = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/relinked.mkv",
        );
        let media_id = original.id();
        session
            .execute_command(media_add(original.clone(), 0))
            .unwrap();
        session
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![
                TimelineTrack::from_parts_for_codec(
                    TrackId::from_str("22222222-2222-4222-8222-222222222222").unwrap(),
                    TrackKind::Video,
                    vec![TimelineClip::from_parts_for_codec(
                        ClipId::from_str("33333333-3333-4333-8333-333333333333").unwrap(),
                        media_id,
                        RationalTime::ZERO,
                        TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap())
                            .unwrap(),
                    )],
                ),
            ]));

        let result = session
            .execute_command(media_relink(replacement.clone(), 1))
            .unwrap();
        assert_eq!(result.after_revision, ProjectRevision::new(2));
        assert_eq!(
            session.project().media_items(),
            std::slice::from_ref(&replacement)
        );
        assert!(session.project().timeline().references_media(media_id));
        assert!(matches!(
            result.change_set.changes(),
            [ProjectChange::MediaRelinked { before, after, index: 0 }]
                if before == &original && after == &replacement
        ));

        let undone = session.execute_command(undo(2)).unwrap();
        assert_eq!(
            session.project().media_items(),
            std::slice::from_ref(&original)
        );
        assert!(session.project().timeline().references_media(media_id));
        assert!(matches!(
            undone.change_set.changes(),
            [ProjectChange::MediaRelinked { before, after, index: 0 }]
                if before == &replacement && after == &original
        ));

        session.execute_command(redo(3)).unwrap();
        assert_eq!(session.project().media_items(), &[replacement]);
        assert_eq!(session.project_revision(), ProjectRevision::new(4));
    }

    #[test]
    fn media_relink_rejects_incompatible_or_duplicate_sources_atomically() {
        let mut session = fixed_session();
        let original = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/original.mkv",
        );
        let other = media_item(
            "00000000-0000-4000-8000-000000000002",
            "file:///media/other.mkv",
        );
        session
            .execute_command(media_add(original.clone(), 0))
            .unwrap();
        session
            .execute_command(media_add(other.clone(), 1))
            .unwrap();
        session
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![
                TimelineTrack::from_parts_for_codec(
                    TrackId::from_str("22222222-2222-4222-8222-222222222222").unwrap(),
                    TrackKind::Video,
                    vec![TimelineClip::from_parts_for_codec(
                        ClipId::from_str("33333333-3333-4333-8333-333333333333").unwrap(),
                        original.id(),
                        RationalTime::ZERO,
                        TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap())
                            .unwrap(),
                    )],
                ),
            ]));
        let before_project = session.project().clone();
        let before_undo = session.history.undo.clone();

        let incompatible =
            audio_media_item(&original.id().to_string(), "file:///media/audio-only.mkv");
        assert_eq!(
            code(&session.execute_command(media_relink(incompatible, 2))),
            OperationErrorCode::TimelineMediaIncompatible
        );
        let duplicate = media_item(&original.id().to_string(), other.source().uri());
        assert_eq!(
            code(&session.execute_command(media_relink(duplicate, 2))),
            OperationErrorCode::MediaSourceAlreadyExists
        );
        assert_eq!(session.project(), &before_project);
        assert_eq!(session.history.undo, before_undo);
    }

    #[test]
    fn media_remove_rejects_referenced_media_without_changing_state_or_history() {
        let mut session = fixed_session();
        let item = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/one.mkv",
        );
        let media_id = item.id();
        session.execute_command(media_add(item, 0)).unwrap();
        session.execute_command(rename("B", 1)).unwrap();
        session.execute_command(undo(2)).unwrap();
        assert_eq!(session.history.redo.len(), 1);

        session
            .project
            .set_timeline_for_test(ProjectTimeline::from_tracks_for_codec(vec![
                TimelineTrack::from_parts_for_codec(
                    TrackId::from_str("22222222-2222-4222-8222-222222222222").unwrap(),
                    TrackKind::Video,
                    vec![TimelineClip::from_parts_for_codec(
                        crate::ClipId::from_str("33333333-3333-4333-8333-333333333333").unwrap(),
                        media_id,
                        RationalTime::ZERO,
                        TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 1).unwrap())
                            .unwrap(),
                    )],
                ),
            ]));
        let before_project = session.project().clone();
        let before_undo = session.history.undo.clone();
        let before_redo = session.history.redo.clone();

        let result = session.execute_command(media_remove(&media_id.to_string(), 3));

        assert!(matches!(
            result,
            Err(error) if error.code == OperationErrorCode::MediaInUse
        ));
        assert_eq!(session.project(), &before_project);
        assert_eq!(session.history.undo, before_undo);
        assert_eq!(session.history.redo, before_redo);
        assert_eq!(session.project_revision(), ProjectRevision::new(3));
        assert_eq!(session.project().media_items().len(), 1);
        assert_eq!(session.project().timeline(), before_project.timeline());
    }

    #[test]
    fn undo_and_redo_media_add_restore_the_same_identity_and_metadata() {
        let mut session = fixed_session();
        let item = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/one.mkv",
        );
        session.execute_command(media_add(item.clone(), 0)).unwrap();

        let undo_result = session.execute_command(undo(1)).unwrap();
        assert!(session.project().media_items().is_empty());
        assert_eq!(undo_result.after_revision, ProjectRevision::new(2));
        assert!(matches!(
            undo_result.change_set.changes(),
            [ProjectChange::MediaRemoved { item: removed, index: 0 }] if removed == &item
        ));

        let redo_result = session.execute_command(redo(2)).unwrap();
        assert_eq!(session.project().media_items(), std::slice::from_ref(&item));
        assert_eq!(redo_result.after_revision, ProjectRevision::new(3));
        assert!(matches!(
            redo_result.change_set.changes(),
            [ProjectChange::MediaAdded { item: added, index: 0 }] if added == &item
        ));
    }

    #[test]
    fn undo_remove_restores_original_media_order_and_redo_removes_it_again() {
        let mut session = fixed_session();
        let a = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/a.mkv",
        );
        let b = media_item(
            "00000000-0000-4000-8000-000000000002",
            "file:///media/b.mkv",
        );
        let c = media_item(
            "00000000-0000-4000-8000-000000000003",
            "file:///media/c.mkv",
        );
        session.execute_command(media_add(a.clone(), 0)).unwrap();
        session.execute_command(media_add(b.clone(), 1)).unwrap();
        session.execute_command(media_add(c.clone(), 2)).unwrap();
        session
            .execute_command(media_remove("00000000-0000-4000-8000-000000000002", 3))
            .unwrap();
        assert_eq!(session.project().media_items(), &[a.clone(), c.clone()]);

        let undo_result = session.execute_command(undo(4)).unwrap();
        assert_eq!(
            session.project().media_items(),
            &[a.clone(), b.clone(), c.clone()]
        );
        assert!(matches!(
            undo_result.change_set.changes(),
            [ProjectChange::MediaAdded { item, index: 1 }] if item == &b
        ));

        session.execute_command(redo(5)).unwrap();
        assert_eq!(session.project().media_items(), &[a, c]);
        assert_eq!(session.project_revision(), ProjectRevision::new(6));
    }

    #[test]
    fn media_history_conflicts_do_not_partially_change_state_or_stacks() {
        let mut session = fixed_session();
        let item = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/one.mkv",
        );
        session.execute_command(media_add(item, 0)).unwrap();
        session
            .project
            .remove_media_for_command(0, ProjectRevision::new(1));
        let before = session.project().clone();
        let undo_len = session.history.undo.len();
        let redo_len = session.history.redo.len();

        assert_eq!(
            code(&session.execute_command(undo(1))),
            OperationErrorCode::HistoryConflict
        );
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo.len(), undo_len);
        assert_eq!(session.history.redo.len(), redo_len);
    }

    #[test]
    fn media_list_pages_are_bounded_ordered_and_read_only() {
        let mut session = fixed_session();
        let expected_ids = (0..250)
            .map(|index| {
                let item = MediaItem::new(
                    MediaId::generate(),
                    MediaSourceRef::local_file(format!("file:///generated/{index}.mkv")).unwrap(),
                    MediaMetadata::from_probe(vec!["matroska".to_owned()], None, 42, Vec::new()),
                )
                .unwrap();
                let id = item.id();
                let revision = session.project_revision().value();
                session.execute_command(media_add(item, revision)).unwrap();
                id
            })
            .collect::<Vec<_>>();
        let before = session.project().clone();
        let mut collected = Vec::new();
        let mut offset = 0;

        loop {
            let result = session
                .execute_query(QueryEnvelope::media_list(
                    session.project_id(),
                    session.project_instance_id(),
                    offset,
                    MAX_MEDIA_PAGE_SIZE,
                ))
                .unwrap();
            let page = result.media_page.as_ref().unwrap();
            assert_eq!(page.total_count, expected_ids.len());
            assert_eq!(page.offset, offset);
            assert!(page.items.len() <= MAX_MEDIA_PAGE_SIZE);
            let encoded = serde_json::to_vec(&result).unwrap();
            assert!(encoded.len() < 1024 * 1024);
            let wire = serde_json::from_slice::<Value>(&encoded).unwrap();
            let wire = wire.as_object().unwrap();
            assert!(!wire.contains_key("media_item"));
            assert!(!wire.contains_key("timeline"));
            assert!(!wire.contains_key("timeline_tracks"));
            assert!(!wire.contains_key("timeline_clip_page"));
            assert_eq!(
                serde_json::from_slice::<super::QueryResult>(&encoded).unwrap(),
                result
            );
            collected.extend(page.items.iter().map(MediaItem::id));
            match page.next_offset {
                Some(next) => offset = next,
                None => break,
            }
        }

        assert_eq!(collected, expected_ids);
        let beyond_end = session
            .execute_query(QueryEnvelope::media_list(
                session.project_id(),
                session.project_instance_id(),
                1_000,
                MAX_MEDIA_PAGE_SIZE,
            ))
            .unwrap()
            .media_page
            .unwrap();
        assert!(beyond_end.items.is_empty());
        assert_eq!(beyond_end.total_count, expected_ids.len());
        assert_eq!(beyond_end.next_offset, None);
        assert_eq!(session.project(), &before);
    }

    #[test]
    fn media_list_rejects_invalid_page_bounds_and_argument_shapes() {
        let session = fixed_session();
        for arguments in [
            json!({ "offset": 0, "limit": 0 }),
            json!({ "offset": 0, "limit": MAX_MEDIA_PAGE_SIZE + 1 }),
            json!({ "offset": "0", "limit": 1 }),
            json!({ "offset": 0, "limit": 1, "extra": true }),
        ] {
            let mut query = QueryEnvelope::media_list(
                session.project_id(),
                session.project_instance_id(),
                0,
                1,
            );
            query.arguments = arguments;
            assert_eq!(
                code(&session.execute_query(query)),
                OperationErrorCode::InvalidArguments
            );
        }
        assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
    }

    #[test]
    fn media_get_returns_one_persisted_item_without_mutating_project_or_revision() {
        let mut session = fixed_session();
        let item = media_item(
            "00000000-0000-4000-8000-000000000001",
            "file:///media/one.mkv",
        );
        session.execute_command(media_add(item.clone(), 0)).unwrap();
        let before = session.project().clone();
        let result = session
            .execute_query(QueryEnvelope::media_get(
                session.project_id(),
                session.project_instance_id(),
                item.id(),
            ))
            .unwrap();

        assert_eq!(result.query_id, "media.get");
        assert_eq!(result.media_page, None);
        assert_eq!(result.media_item.as_deref(), Some(&item));
        let wire = serde_json::to_value(&result).unwrap();
        assert!(!wire.as_object().unwrap().contains_key("timeline"));
        assert!(!wire.as_object().unwrap().contains_key("timeline_tracks"));
        assert!(!wire.as_object().unwrap().contains_key("timeline_clip_page"));
        assert_eq!(
            serde_json::from_value::<super::QueryResult>(wire).unwrap(),
            result
        );
        assert_eq!(session.project(), &before);
        assert_eq!(session.project_revision(), ProjectRevision::new(1));
    }

    #[test]
    fn media_get_rejects_unknown_ids_and_invalid_arguments() {
        let session = fixed_session();
        let missing = MediaId::generate();
        assert_eq!(
            code(&session.execute_query(QueryEnvelope::media_get(
                session.project_id(),
                session.project_instance_id(),
                missing,
            ))),
            OperationErrorCode::MediaNotFound
        );

        let mut invalid =
            QueryEnvelope::media_get(session.project_id(), session.project_instance_id(), missing);
        invalid.arguments = json!({ "media_id": missing, "extra": true });
        assert_eq!(
            code(&session.execute_query(invalid)),
            OperationErrorCode::InvalidArguments
        );
        assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
    }

    #[test]
    fn opening_session_preserves_document_and_has_valid_runtime_identity() {
        let document = project();
        let original = document.clone();
        let session = ProjectSession::open(document);

        assert_eq!(session.project(), &original);
        assert_eq!(session.project_id(), original.id());
        assert_eq!(session.project_revision(), original.revision());
        assert!(ProjectInstanceId::from_str(&session.project_instance_id().to_string()).is_ok());
        assert_eq!(session.into_project(), original);
    }

    #[test]
    fn rename_changes_name_and_increments_revision_once() {
        let mut session = fixed_session();
        let result = session.execute_command(rename("B", 0)).unwrap();

        assert_eq!(session.project().name(), "B");
        assert_eq!(session.project_revision(), ProjectRevision::new(1));
        assert_eq!(result.before_revision, ProjectRevision::new(0));
        assert_eq!(result.after_revision, ProjectRevision::new(1));
        assert!(result.changed);
    }

    #[test]
    fn rename_preserves_unicode_exactly() {
        let mut session = fixed_session();
        let name = "  Tiếng Việt – cà phê 🎬  ";
        let result = session.execute_command(rename(name, 0)).unwrap();

        assert_eq!(session.project().name(), name);
        assert!(result.changed);
    }

    #[test]
    fn each_real_rename_increments_revision_exactly_once() {
        let mut session = fixed_session();
        session.execute_command(rename("B", 0)).unwrap();
        let result = session.execute_command(rename("C", 1)).unwrap();

        assert_eq!(session.project().name(), "C");
        assert_eq!(result.before_revision, ProjectRevision::new(1));
        assert_eq!(result.after_revision, ProjectRevision::new(2));
    }

    #[test]
    fn rename_to_same_name_is_a_successful_no_op() {
        let mut session = fixed_session();
        let result = session.execute_command(rename("A", 0)).unwrap();

        assert_eq!(session.project().name(), "A");
        assert_eq!(result.before_revision, ProjectRevision::INITIAL);
        assert_eq!(result.after_revision, ProjectRevision::INITIAL);
        assert!(!result.changed);
    }

    #[test]
    fn revision_conflict_returns_current_revision_without_mutation() {
        let mut session = fixed_session();
        session.execute_command(rename("B", 0)).unwrap();
        let before = session.project().clone();
        let result = session.execute_command(rename("C", 0));

        assert_eq!(code(&result), OperationErrorCode::RevisionConflict);
        let error = result.unwrap_err();
        assert_eq!(error.current_revision, Some(ProjectRevision::new(1)));
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({ "code": "REVISION_CONFLICT", "current_revision": 1 })
        );
        assert_eq!(session.project(), &before);
    }

    #[test]
    fn wrong_instance_is_rejected_even_with_matching_project_and_revision() {
        let mut session = fixed_session();
        let mut command = rename("B", 0);
        command.project_instance_id = ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
        let before = session.project().clone();

        assert_eq!(
            code(&session.execute_command(command)),
            OperationErrorCode::ProjectInstanceMismatch
        );
        assert_eq!(session.project(), &before);
    }

    #[test]
    fn wrong_project_is_rejected_without_mutation() {
        let mut session = fixed_session();
        let mut command = rename("B", 0);
        command.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
        let before = session.project().clone();

        assert_eq!(
            code(&session.execute_command(command)),
            OperationErrorCode::ProjectIdMismatch
        );
        assert_eq!(session.project(), &before);
    }

    #[test]
    fn unknown_and_unsupported_commands_are_rejected_before_mutation() {
        let mut session = fixed_session();
        let before = session.project().clone();
        let mut unknown = rename("B", 0);
        unknown.command_id = "project.delete".to_owned();
        assert_eq!(
            code(&session.execute_command(unknown)),
            OperationErrorCode::UnknownCommand
        );

        let mut unsupported = rename("B", 0);
        unsupported.schema_version = 2;
        assert_eq!(
            code(&session.execute_command(unsupported)),
            OperationErrorCode::UnsupportedCommandSchema
        );
        assert_eq!(session.project(), &before);
    }

    #[test]
    fn command_validation_follows_id_schema_project_instance_revision_order() {
        let mut session = fixed_session();
        let mut command = rename("B", 99);
        command.command_id = "unknown.command".to_owned();
        command.schema_version = 2;
        command.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
        command.project_instance_id = ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
        assert_eq!(
            code(&session.execute_command(command.clone())),
            OperationErrorCode::UnknownCommand
        );

        command.command_id = "project.rename".to_owned();
        assert_eq!(
            code(&session.execute_command(command.clone())),
            OperationErrorCode::UnsupportedCommandSchema
        );

        command.schema_version = 1;
        assert_eq!(
            code(&session.execute_command(command.clone())),
            OperationErrorCode::ProjectIdMismatch
        );

        command.project_id = ProjectId::from_str(PROJECT_ID).unwrap();
        assert_eq!(
            code(&session.execute_command(command.clone())),
            OperationErrorCode::ProjectInstanceMismatch
        );

        command.project_instance_id = ProjectInstanceId::from_str(INSTANCE_ID).unwrap();
        assert_eq!(
            code(&session.execute_command(command)),
            OperationErrorCode::RevisionConflict
        );
        assert_eq!(session.project().name(), "A");
        assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
    }

    #[test]
    fn invalid_rename_arguments_are_rejected_without_mutation() {
        let mut session = fixed_session();
        let before = session.project().clone();
        for arguments in [
            json!({}),
            json!({ "name": 5 }),
            json!({ "name": "B", "extra": true }),
        ] {
            let mut command = rename("B", 0);
            command.arguments = arguments;
            assert_eq!(
                code(&session.execute_command(command)),
                OperationErrorCode::InvalidArguments
            );
            assert_eq!(session.project(), &before);
        }
    }

    #[test]
    fn revision_overflow_rejects_rename_without_partial_mutation() {
        let mut session = ProjectSession {
            project: decode_project(
                r#"{"format":"opencut-reinforced-project","schema_version":1,"project":{"id":"01234567-89ab-4def-8123-456789abcdef","revision":18446744073709551615,"name":"A"}}"#,
            )
            .unwrap(),
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            history: super::SessionHistory::default(),
        };
        let before = session.project().clone();
        let result = session.execute_command(rename("B", u64::MAX));

        assert_eq!(code(&result), OperationErrorCode::RevisionOverflow);
        assert_eq!(session.project(), &before);
    }

    #[test]
    fn project_summary_is_read_only_and_returns_current_state() {
        let session = fixed_session();
        let before = session.project().clone();
        let result = session.execute_query(summary_query()).unwrap();

        assert_eq!(result.query_id, "project.summary");
        assert_eq!(result.schema_version, 1);
        assert_eq!(result.summary.project_id, session.project_id());
        assert_eq!(
            result.summary.project_instance_id,
            session.project_instance_id()
        );
        assert_eq!(result.summary.project_revision, ProjectRevision::INITIAL);
        assert_eq!(result.summary.name, "A");
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            json!({
                "query_id": "project.summary",
                "schema_version": 1,
                "project_id": PROJECT_ID,
                "project_instance_id": INSTANCE_ID,
                "project_revision": 0,
                "name": "A"
            })
        );
        assert_eq!(session.project(), &before);
        assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
    }

    #[test]
    fn summary_after_rename_observes_new_canonical_state() {
        let mut session = fixed_session();
        session.execute_command(rename("B", 0)).unwrap();
        let summary = session.execute_query(summary_query()).unwrap().summary;

        assert_eq!(summary.name, "B");
        assert_eq!(summary.project_revision, ProjectRevision::new(1));
    }

    #[test]
    fn wrong_project_and_instance_queries_are_rejected_without_state_change() {
        let session = fixed_session();
        let before = session.project().clone();
        let mut wrong_project = summary_query();
        wrong_project.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
        assert_eq!(
            code(&session.execute_query(wrong_project)),
            OperationErrorCode::ProjectIdMismatch
        );

        let mut wrong_instance = summary_query();
        wrong_instance.project_instance_id =
            ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
        assert_eq!(
            code(&session.execute_query(wrong_instance)),
            OperationErrorCode::ProjectInstanceMismatch
        );
        assert_eq!(session.project(), &before);
    }

    #[test]
    fn unknown_and_unsupported_queries_are_rejected() {
        let session = fixed_session();
        let mut unknown = summary_query();
        unknown.query_id = "project.timeline".to_owned();
        assert_eq!(
            code(&session.execute_query(unknown)),
            OperationErrorCode::UnknownQuery
        );

        let mut unsupported = summary_query();
        unsupported.schema_version = 2;
        assert_eq!(
            code(&session.execute_query(unsupported)),
            OperationErrorCode::UnsupportedQuerySchema
        );
    }

    #[test]
    fn query_validation_follows_id_schema_project_instance_arguments_order() {
        let session = fixed_session();
        let mut query = summary_query();
        query.query_id = "unknown.query".to_owned();
        query.schema_version = 2;
        query.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
        query.project_instance_id = ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
        query.arguments = json!({ "unexpected": true });
        assert_eq!(
            code(&session.execute_query(query.clone())),
            OperationErrorCode::UnknownQuery
        );

        query.query_id = "project.summary".to_owned();
        assert_eq!(
            code(&session.execute_query(query.clone())),
            OperationErrorCode::UnsupportedQuerySchema
        );

        query.schema_version = 1;
        assert_eq!(
            code(&session.execute_query(query.clone())),
            OperationErrorCode::ProjectIdMismatch
        );

        query.project_id = ProjectId::from_str(PROJECT_ID).unwrap();
        assert_eq!(
            code(&session.execute_query(query.clone())),
            OperationErrorCode::ProjectInstanceMismatch
        );

        query.project_instance_id = ProjectInstanceId::from_str(INSTANCE_ID).unwrap();
        assert_eq!(
            code(&session.execute_query(query)),
            OperationErrorCode::InvalidArguments
        );
        assert_eq!(session.project().name(), "A");
        assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
    }

    #[test]
    fn summary_requires_an_empty_object_of_arguments() {
        let session = fixed_session();
        for arguments in [json!({ "unexpected": true }), json!([]), Value::Null] {
            let mut query = summary_query();
            query.arguments = arguments;
            assert_eq!(
                code(&session.execute_query(query)),
                OperationErrorCode::InvalidArguments
            );
        }
    }

    #[test]
    fn envelopes_round_trip_and_reject_unknown_fields_and_invalid_typed_ids() {
        let command = rename("Tiếng Việt", 7);
        let encoded = serde_json::to_string(&command).unwrap();
        assert_eq!(
            serde_json::from_str::<CommandEnvelope>(&encoded).unwrap(),
            command
        );
        let with_extra_command_field =
            encoded.replace("\"command_id\"", "\"unexpected\":true,\"command_id\"");
        assert!(serde_json::from_str::<CommandEnvelope>(&with_extra_command_field).is_err());

        let query = summary_query();
        let encoded = serde_json::to_string(&query).unwrap();
        assert_eq!(
            serde_json::from_str::<QueryEnvelope>(&encoded).unwrap(),
            query
        );
        let with_extra_query_field =
            encoded.replace("\"query_id\"", "\"unexpected\":true,\"query_id\"");
        assert!(serde_json::from_str::<QueryEnvelope>(&with_extra_query_field).is_err());

        let bad_id = encoded.replace(PROJECT_ID, "00000000-0000-1000-8000-000000000000");
        assert!(serde_json::from_str::<QueryEnvelope>(&bad_id).is_err());
        let bad_instance_id = encoded.replace(INSTANCE_ID, "00000000-0000-1000-8000-000000000000");
        assert!(serde_json::from_str::<QueryEnvelope>(&bad_instance_id).is_err());
    }

    #[test]
    fn rename_round_trips_through_project_codec_without_persisting_session_identity() {
        let mut session = fixed_session();
        session.execute_command(rename("Renamed", 0)).unwrap();
        let encoded = encode_project(session.project()).unwrap();
        let decoded = decode_project(&encoded).unwrap();

        assert_eq!(decoded.name(), "Renamed");
        assert_eq!(decoded.revision(), ProjectRevision::new(1));
        assert!(!encoded.contains(&session.project_instance_id().to_string()));
        assert!(!encoded.contains("instance_id"));
    }

    #[test]
    fn rename_results_report_changes_and_no_op_changesets_are_empty() {
        let mut session = fixed_session();
        let result = session.execute_command(rename("B", 0)).unwrap();
        assert_name_change(&result.change_set, "A", "B");

        let no_op = session.execute_command(rename("B", 1)).unwrap();
        assert!(!no_op.changed);
        assert_eq!(no_op.before_revision, no_op.after_revision);
        assert!(no_op.change_set.is_empty());
        assert_eq!(session.history.undo.len(), 1);
        assert!(session.history.redo.is_empty());
    }

    #[test]
    fn grouped_renames_commit_once_with_a_normalized_changeset_and_one_history_entry() {
        let mut session = fixed_session();
        let result = session
            .execute_transaction(transaction(vec![rename_call("B"), rename_call("C")], 0))
            .unwrap();

        assert_eq!(session.project().name(), "C");
        assert_eq!(result.schema_version, 1);
        assert_eq!(result.before_revision, ProjectRevision::new(0));
        assert_eq!(result.after_revision, ProjectRevision::new(1));
        assert!(result.changed);
        assert_eq!(result.command_count, 2);
        assert_name_change(&result.change_set, "A", "C");
        assert_eq!(session.history.undo.len(), 1);
        assert!(session.history.redo.is_empty());
    }

    #[test]
    fn net_no_op_transaction_keeps_revision_and_redo_history() {
        let mut session = fixed_session();
        session.execute_command(rename("B", 0)).unwrap();
        session.execute_command(undo(1)).unwrap();
        let result = session
            .execute_transaction(transaction(vec![rename_call("B"), rename_call("A")], 2))
            .unwrap();

        assert_eq!(session.project().name(), "A");
        assert_eq!(session.project_revision(), ProjectRevision::new(2));
        assert!(!result.changed);
        assert_eq!(result.before_revision, result.after_revision);
        assert!(result.change_set.is_empty());
        assert!(session.history.undo.is_empty());
        assert_eq!(session.history.redo.len(), 1);

        session.execute_command(redo(2)).unwrap();
        assert_eq!(session.project().name(), "B");
        assert_eq!(session.project_revision(), ProjectRevision::new(3));
    }

    #[test]
    fn invalid_child_rolls_back_the_entire_transaction_and_preserves_history() {
        let mut session = fixed_session();
        let before = session.project().clone();
        let result = session.execute_transaction(transaction(
            vec![
                rename_call("B"),
                call("project.rename", 1, json!({ "extra": true })),
            ],
            0,
        ));

        assert_eq!(code(&result), OperationErrorCode::InvalidArguments);
        assert_eq!(session.project(), &before);
        assert!(session.history.undo.is_empty());
        assert!(session.history.redo.is_empty());
    }

    #[test]
    fn unknown_or_unsupported_child_rolls_back_the_entire_transaction() {
        for child in [
            call("project.missing", 1, json!({})),
            call("project.rename", 2, json!({ "name": "C" })),
        ] {
            let mut session = fixed_session();
            let before = session.project().clone();
            let result = session.execute_transaction(transaction(vec![rename_call("B"), child], 0));

            let expected = if result
                .as_ref()
                .is_err_and(|error| error.code == OperationErrorCode::UnknownCommand)
            {
                OperationErrorCode::UnknownCommand
            } else {
                OperationErrorCode::UnsupportedCommandSchema
            };
            assert_eq!(code(&result), expected);
            assert_eq!(session.project(), &before);
            assert!(session.history.undo.is_empty());
        }
    }

    #[test]
    fn history_commands_are_not_allowed_inside_a_transaction() {
        let mut session = fixed_session();
        let before = session.project().clone();
        let result = session.execute_transaction(transaction(
            vec![rename_call("B"), call("history.undo", 1, json!({}))],
            0,
        ));

        assert_eq!(
            code(&result),
            OperationErrorCode::CommandNotAllowedInTransaction
        );
        assert_eq!(session.project(), &before);
        assert!(session.history.undo.is_empty());
        assert!(session.history.redo.is_empty());
    }

    #[test]
    fn empty_transaction_is_rejected_without_mutation() {
        let mut session = fixed_session();
        let before = session.project().clone();
        assert_eq!(
            code(&session.execute_transaction(transaction(Vec::new(), 0))),
            OperationErrorCode::EmptyTransaction
        );
        assert_eq!(session.project(), &before);
        assert!(session.history.undo.is_empty());
        assert!(session.history.redo.is_empty());
    }

    #[test]
    fn transaction_preconditions_and_schema_are_checked_before_commands() {
        let mut session = fixed_session();
        let mut wrong_project = transaction(Vec::new(), 0);
        wrong_project.project_id = ProjectId::from_str(OTHER_PROJECT_ID).unwrap();
        assert_eq!(
            code(&session.execute_transaction(wrong_project)),
            OperationErrorCode::ProjectIdMismatch
        );

        let mut wrong_instance = transaction(Vec::new(), 0);
        wrong_instance.project_instance_id =
            ProjectInstanceId::from_str(OTHER_INSTANCE_ID).unwrap();
        assert_eq!(
            code(&session.execute_transaction(wrong_instance)),
            OperationErrorCode::ProjectInstanceMismatch
        );

        assert_eq!(
            code(&session.execute_transaction(transaction(Vec::new(), 1))),
            OperationErrorCode::RevisionConflict
        );

        let mut unsupported = transaction(Vec::new(), 99);
        unsupported.schema_version = 2;
        assert_eq!(
            code(&session.execute_transaction(unsupported)),
            OperationErrorCode::UnsupportedTransactionSchema
        );

        assert_eq!(session.project().name(), "A");
        assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
        assert!(session.history.undo.is_empty());
    }

    #[test]
    fn single_rename_creates_one_history_entry_and_undo_returns_inverse_change() {
        let mut session = fixed_session();
        let renamed = session.execute_command(rename("B", 0)).unwrap();
        assert_eq!(renamed.after_revision, ProjectRevision::new(1));
        assert_eq!(session.history.undo.len(), 1);
        assert!(session.history.redo.is_empty());

        let result = session.execute_command(undo(1)).unwrap();
        assert_eq!(session.project().name(), "A");
        assert_eq!(result.before_revision, ProjectRevision::new(1));
        assert_eq!(result.after_revision, ProjectRevision::new(2));
        assert!(result.changed);
        assert_name_change(&result.change_set, "B", "A");
        assert!(session.history.undo.is_empty());
        assert_eq!(session.history.redo.len(), 1);
    }

    #[test]
    fn redo_restores_a_change_with_a_new_revision() {
        let mut session = fixed_session();
        session.execute_command(rename("B", 0)).unwrap();
        session.execute_command(undo(1)).unwrap();

        let result = session.execute_command(redo(2)).unwrap();
        assert_eq!(session.project().name(), "B");
        assert_eq!(result.before_revision, ProjectRevision::new(2));
        assert_eq!(result.after_revision, ProjectRevision::new(3));
        assert!(result.changed);
        assert_name_change(&result.change_set, "A", "B");
        assert_eq!(session.history.undo.len(), 1);
        assert!(session.history.redo.is_empty());
    }

    #[test]
    fn one_undo_reverses_a_whole_grouped_transaction() {
        let mut session = fixed_session();
        session
            .execute_transaction(transaction(vec![rename_call("B"), rename_call("C")], 0))
            .unwrap();

        let result = session.execute_command(undo(1)).unwrap();
        assert_eq!(session.project().name(), "A");
        assert_eq!(result.after_revision, ProjectRevision::new(2));
        assert_name_change(&result.change_set, "C", "A");
        assert!(session.history.undo.is_empty());
        assert_eq!(session.history.redo.len(), 1);
    }

    #[test]
    fn new_real_edit_clears_redo_history() {
        let mut session = fixed_session();
        session.execute_command(rename("B", 0)).unwrap();
        session.execute_command(undo(1)).unwrap();
        assert_eq!(session.history.redo.len(), 1);

        session.execute_command(rename("C", 2)).unwrap();
        assert_eq!(session.project().name(), "C");
        assert_eq!(session.project_revision(), ProjectRevision::new(3));
        assert!(session.history.redo.is_empty());
        assert_eq!(
            code(&session.execute_command(redo(3))),
            OperationErrorCode::NothingToRedo
        );
    }

    #[test]
    fn successful_no_op_command_does_not_clear_redo_history() {
        let mut session = fixed_session();
        session.execute_command(rename("B", 0)).unwrap();
        session.execute_command(undo(1)).unwrap();

        let result = session.execute_command(rename("A", 2)).unwrap();
        assert!(!result.changed);
        assert_eq!(session.project_revision(), ProjectRevision::new(2));
        assert_eq!(session.history.redo.len(), 1);
        session.execute_command(redo(2)).unwrap();
        assert_eq!(session.project().name(), "B");
    }

    #[test]
    fn failed_edit_does_not_clear_redo_history() {
        let mut session = fixed_session();
        session.execute_command(rename("B", 0)).unwrap();
        session.execute_command(undo(1)).unwrap();
        let mut invalid = rename("C", 2);
        invalid.arguments = json!({});

        assert_eq!(
            code(&session.execute_command(invalid)),
            OperationErrorCode::InvalidArguments
        );
        assert_eq!(session.project().name(), "A");
        assert_eq!(session.project_revision(), ProjectRevision::new(2));
        assert_eq!(session.history.redo.len(), 1);
        session.execute_command(redo(2)).unwrap();
        assert_eq!(session.project().name(), "B");
    }

    #[test]
    fn empty_history_and_invalid_history_arguments_are_rejected() {
        let mut session = fixed_session();
        assert_eq!(
            code(&session.execute_command(undo(0))),
            OperationErrorCode::NothingToUndo
        );
        assert_eq!(
            code(&session.execute_command(redo(0))),
            OperationErrorCode::NothingToRedo
        );

        for arguments in [Value::Null, json!({ "unexpected": true })] {
            let invalid = history_command("history.undo", 0, arguments);
            assert_eq!(
                code(&session.execute_command(invalid)),
                OperationErrorCode::InvalidArguments
            );
        }
        assert_eq!(session.project_revision(), ProjectRevision::INITIAL);
    }

    #[test]
    fn forward_revision_overflow_changes_neither_project_nor_history() {
        let mut session = session_with_state("A", u64::MAX);
        session.history.redo.push(ChangeSet::project_name("A", "B"));
        let before = session.project().clone();
        let redo_before = session.history.redo.clone();
        assert_eq!(
            code(&session.execute_command(rename("B", u64::MAX))),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(session.project(), &before);
        assert!(session.history.undo.is_empty());
        assert_eq!(session.history.redo, redo_before);
    }

    #[test]
    fn undo_and_redo_revision_overflow_preserve_document_and_stacks() {
        let mut undo_session = session_with_state("B", u64::MAX);
        undo_session
            .history
            .undo
            .push(ChangeSet::project_name("A", "B"));
        let before = undo_session.project().clone();
        let undo_before = undo_session.history.undo.clone();
        assert_eq!(
            code(&undo_session.execute_command(undo(u64::MAX))),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(undo_session.project(), &before);
        assert_eq!(undo_session.history.undo, undo_before);
        assert!(undo_session.history.redo.is_empty());

        let mut redo_session = session_with_state("A", u64::MAX);
        redo_session
            .history
            .redo
            .push(ChangeSet::project_name("A", "B"));
        let before = redo_session.project().clone();
        let redo_before = redo_session.history.redo.clone();
        assert_eq!(
            code(&redo_session.execute_command(redo(u64::MAX))),
            OperationErrorCode::RevisionOverflow
        );
        assert_eq!(redo_session.project(), &before);
        assert_eq!(redo_session.history.redo, redo_before);
        assert!(redo_session.history.undo.is_empty());
    }

    #[test]
    fn history_conflicts_fail_without_partial_mutation_or_stack_changes() {
        let mut undo_session = session_with_state("wrong", 0);
        undo_session
            .history
            .undo
            .push(ChangeSet::project_name("A", "B"));
        let before = undo_session.project().clone();
        let undo_before = undo_session.history.undo.clone();
        assert_eq!(
            code(&undo_session.execute_command(undo(0))),
            OperationErrorCode::HistoryConflict
        );
        assert_eq!(undo_session.project(), &before);
        assert_eq!(undo_session.history.undo, undo_before);
        assert!(undo_session.history.redo.is_empty());

        let mut redo_session = session_with_state("wrong", 0);
        redo_session
            .history
            .redo
            .push(ChangeSet::project_name("A", "B"));
        let before = redo_session.project().clone();
        let redo_before = redo_session.history.redo.clone();
        assert_eq!(
            code(&redo_session.execute_command(redo(0))),
            OperationErrorCode::HistoryConflict
        );
        assert_eq!(redo_session.project(), &before);
        assert_eq!(redo_session.history.redo, redo_before);
        assert!(redo_session.history.undo.is_empty());
    }

    #[test]
    fn transaction_and_command_call_serde_are_strict_and_round_trip() {
        let call = rename_call("B");
        let call_json = serde_json::to_string(&call).unwrap();
        assert_eq!(
            serde_json::from_str::<CommandCall>(&call_json).unwrap(),
            call
        );
        assert_eq!(
            serde_json::from_str::<Value>(&call_json)
                .unwrap()
                .as_object()
                .unwrap()
                .len(),
            3
        );
        let extra_call = call_json.replace("\"command_id\"", "\"extra\":true,\"command_id\"");
        assert!(serde_json::from_str::<CommandCall>(&extra_call).is_err());

        let envelope = transaction(vec![call], 7);
        let encoded = serde_json::to_string(&envelope).unwrap();
        assert_eq!(
            serde_json::from_str::<TransactionEnvelope>(&encoded).unwrap(),
            envelope
        );
        let with_extra = encoded.replace("\"schema_version\"", "\"extra\":true,\"schema_version\"");
        assert!(serde_json::from_str::<TransactionEnvelope>(&with_extra).is_err());

        let bad_project = encoded.replace(PROJECT_ID, "00000000-0000-1000-8000-000000000000");
        assert!(serde_json::from_str::<TransactionEnvelope>(&bad_project).is_err());
        let bad_instance = encoded.replace(INSTANCE_ID, "00000000-0000-1000-8000-000000000000");
        assert!(serde_json::from_str::<TransactionEnvelope>(&bad_instance).is_err());
        assert!(serde_json::from_str::<TransactionEnvelope>("{}").is_err());
    }

    #[test]
    fn grouped_transaction_persists_only_canonical_state_through_orproj_v3() {
        let mut session = fixed_session();
        session
            .execute_transaction(transaction(vec![rename_call("B"), rename_call("C")], 0))
            .unwrap();
        let encoded = encode_project(session.project()).unwrap();
        let decoded = decode_project(&encoded).unwrap();

        assert_eq!(decoded.name(), "C");
        assert_eq!(decoded.revision(), ProjectRevision::new(1));
        assert!(!encoded.contains(&session.project_instance_id().to_string()));
        for forbidden in ["instance_id", "undo", "redo", "change_set"] {
            assert!(!encoded.contains(forbidden));
        }
    }

    #[test]
    fn persistence_after_undo_keeps_current_revision_and_omits_history() {
        let mut session = fixed_session();
        session.execute_command(rename("B", 0)).unwrap();
        session.execute_command(undo(1)).unwrap();
        let encoded = encode_project(session.project()).unwrap();
        let decoded = decode_project(&encoded).unwrap();

        assert_eq!(decoded.name(), "A");
        assert_eq!(decoded.revision(), ProjectRevision::new(2));
        assert!(!encoded.contains(&session.project_instance_id().to_string()));
        for forbidden in ["instance_id", "undo", "redo", "change_set"] {
            assert!(!encoded.contains(forbidden));
        }
    }

    #[test]
    fn reopening_a_document_starts_with_empty_session_history() {
        let mut session = fixed_session();
        session.execute_command(rename("B", 0)).unwrap();
        session.execute_command(undo(1)).unwrap();
        let encoded = encode_project(session.project()).unwrap();
        let mut reopened = ProjectSession::open(decode_project(&encoded).unwrap());

        assert_eq!(reopened.project_id(), session.project_id());
        assert_eq!(reopened.project_revision(), ProjectRevision::new(2));
        assert_eq!(reopened.project().name(), "A");
        assert!(ProjectInstanceId::from_str(&reopened.project_instance_id().to_string()).is_ok());
        let mut undo = undo(2);
        undo.project_instance_id = reopened.project_instance_id();
        assert_eq!(
            code(&reopened.execute_command(undo)),
            OperationErrorCode::NothingToUndo
        );
        let mut redo = redo(2);
        redo.project_instance_id = reopened.project_instance_id();
        assert_eq!(
            code(&reopened.execute_command(redo)),
            OperationErrorCode::NothingToRedo
        );
    }

    #[test]
    fn sequence_rate_command_query_noop_and_history_use_the_generic_application_path() {
        let mut session = fixed_session();
        let initial = session.handle_application_request(ApplicationRequest::Query(
            QueryEnvelope::timeline_sequence_settings(
                session.project_id(),
                session.project_instance_id(),
            ),
        ));
        assert!(matches!(
            initial,
            ApplicationResponse::Query(result)
                if result.timeline_sequence_settings.as_ref().is_some_and(|settings| settings.sequence_frame_rate.is_none())
                    && result.summary.project_revision == ProjectRevision::new(0)
        ));

        let rate = RationalRate::new(24_000, 1_001).unwrap();
        let changed = session
            .handle_application_request(ApplicationRequest::Command(sequence_rate(Some(rate), 0)));
        assert!(matches!(
            changed,
            ApplicationResponse::Command(result)
                if result.changed
                    && result.before_revision == ProjectRevision::new(0)
                    && result.after_revision == ProjectRevision::new(1)
                    && matches!(result.change_set.changes(), [ProjectChange::TimelineSequenceFrameRateChanged { before: None, after: Some(value) }] if *value == rate)
        ));
        assert_eq!(
            session.project().timeline().sequence_frame_rate(),
            Some(rate)
        );

        let query = session.handle_application_request(ApplicationRequest::Query(
            QueryEnvelope::timeline_sequence_settings(
                session.project_id(),
                session.project_instance_id(),
            ),
        ));
        assert!(matches!(
            query,
            ApplicationResponse::Query(result)
                if result.timeline_sequence_settings.as_ref().is_some_and(|settings| settings.sequence_frame_rate == Some(rate))
        ));

        let no_op = session
            .execute_command(sequence_rate(Some(rate), 1))
            .unwrap();
        assert!(!no_op.changed);
        assert_eq!(session.project_revision(), ProjectRevision::new(1));
        assert_eq!(session.history.undo.len(), 1);

        let cleared = session.execute_command(sequence_rate(None, 1)).unwrap();
        assert!(cleared.changed);
        assert_eq!(session.project_revision(), ProjectRevision::new(2));
        assert_eq!(session.project().timeline().sequence_frame_rate(), None);
        let undo_depth = session.history.undo.len();
        let redo_depth = session.history.redo.len();
        let clear_no_op = session.execute_command(sequence_rate(None, 2)).unwrap();
        assert!(!clear_no_op.changed);
        assert_eq!(session.history.undo.len(), undo_depth);
        assert_eq!(session.history.redo.len(), redo_depth);

        session.execute_command(undo(2)).unwrap();
        assert_eq!(session.project_revision(), ProjectRevision::new(3));
        assert_eq!(
            session.project().timeline().sequence_frame_rate(),
            Some(rate)
        );
        session.execute_command(redo(3)).unwrap();
        assert_eq!(session.project_revision(), ProjectRevision::new(4));
        assert_eq!(session.project().timeline().sequence_frame_rate(), None);

        for invalid in [
            json!({}),
            json!({"sequence_frame_rate": {"numerator": 0, "denominator": 1}}),
            json!({"sequence_frame_rate": {"numerator": 24, "denominator": 1, "extra": true}}),
        ] {
            assert_eq!(
                code(&session.execute_command(history_command(
                    "timeline.sequence.set_frame_rate",
                    session.project_revision().value(),
                    invalid,
                ))),
                OperationErrorCode::InvalidArguments
            );
        }

        let query = QueryEnvelope {
            query_id: "timeline.sequence.settings".to_owned(),
            schema_version: 1,
            project_id: session.project_id(),
            project_instance_id: session.project_instance_id(),
            arguments: json!({"unexpected": true}),
        };
        assert_eq!(
            code(&session.execute_query(query)),
            OperationErrorCode::InvalidArguments
        );
        let transaction = transaction(
            vec![call(
                "timeline.sequence.set_frame_rate",
                1,
                json!({"sequence_frame_rate": {"numerator": 24, "denominator": 1}}),
            )],
            4,
        );
        assert_eq!(
            code(&session.execute_transaction(transaction)),
            OperationErrorCode::CommandNotAllowedInTransaction
        );
    }

    #[test]
    fn timeline_marker_commands_keep_canonical_order_and_semantic_history() {
        let mut session = fixed_session();
        marker_add(&mut session, MARKER_B, (2, 1), "same").unwrap();
        marker_add(&mut session, MARKER_A, (2, 1), "same").unwrap();
        marker_add(&mut session, MARKER_C, (1, 1), "same").unwrap();
        assert_eq!(session.project_revision(), ProjectRevision::new(3));
        assert_eq!(
            session
                .project()
                .timeline()
                .markers()
                .iter()
                .map(|marker| marker.id().to_string())
                .collect::<Vec<_>>(),
            [MARKER_C, MARKER_A, MARKER_B]
        );

        session.execute_command(undo(3)).unwrap();
        assert_eq!(session.project_revision(), ProjectRevision::new(4));
        assert_eq!(session.project().timeline().markers().len(), 2);
        let redo_depth = session.history.redo.len();
        let no_op_move = execute(
            &mut session,
            "timeline.marker.move",
            json!({
                "marker_id": MARKER_A,
                "timeline_time": rational_json(2, 1),
            }),
        )
        .unwrap();
        assert!(!no_op_move.changed);
        assert_eq!(session.project_revision(), ProjectRevision::new(4));
        assert_eq!(session.history.redo.len(), redo_depth);
        let no_op_rename = execute(
            &mut session,
            "timeline.marker.rename",
            json!({"marker_id": MARKER_A, "label": "same"}),
        )
        .unwrap();
        assert!(!no_op_rename.changed);
        assert_eq!(session.history.redo.len(), redo_depth);

        session.execute_command(redo(4)).unwrap();
        let moved = execute(
            &mut session,
            "timeline.marker.move",
            json!({
                "marker_id": MARKER_B,
                "timeline_time": rational_json(0, 1),
            }),
        )
        .unwrap();
        assert!(moved.changed);
        assert_eq!(session.project_revision(), ProjectRevision::new(6));
        assert_eq!(
            session
                .project()
                .timeline()
                .markers()
                .iter()
                .map(|marker| marker.id().to_string())
                .collect::<Vec<_>>(),
            [MARKER_B, MARKER_C, MARKER_A]
        );
        session.execute_command(undo(6)).unwrap();
        assert_eq!(
            session
                .project()
                .timeline()
                .markers()
                .iter()
                .map(|marker| marker.id().to_string())
                .collect::<Vec<_>>(),
            [MARKER_C, MARKER_A, MARKER_B]
        );
        session.execute_command(redo(7)).unwrap();
        assert_eq!(
            session.project().timeline().markers()[0].id(),
            marker_id(MARKER_B)
        );
    }

    #[test]
    fn timeline_marker_rename_delete_and_history_restore_exact_semantic_state() {
        let mut session = fixed_session();
        let added = marker_add(&mut session, MARKER_A, (1, 1), "before").unwrap();
        assert!(matches!(
            added.change_set.changes(),
            [ProjectChange::TimelineMarkerAdded { marker, index: 0 }]
                if marker.marker_id == marker_id(MARKER_A)
                    && marker.timeline_time == RationalTime::new(1, 1).unwrap()
                    && marker.label == "before"
        ));

        let renamed = execute(
            &mut session,
            "timeline.marker.rename",
            json!({"marker_id": MARKER_A, "label": "after"}),
        )
        .unwrap();
        assert!(matches!(
            renamed.change_set.changes(),
            [ProjectChange::TimelineMarkerRenamed {
                marker_id: id,
                time,
                before_label,
                after_label,
            }]
                if *id == marker_id(MARKER_A)
                    && *time == RationalTime::new(1, 1).unwrap()
                    && before_label == "before"
                    && after_label == "after"
        ));
        session.execute_command(undo(2)).unwrap();
        assert_eq!(session.project().timeline().markers()[0].label(), "before");
        session.execute_command(redo(3)).unwrap();
        assert_eq!(session.project().timeline().markers()[0].label(), "after");

        execute(
            &mut session,
            "timeline.marker.move",
            json!({
                "marker_id": MARKER_A,
                "timeline_time": rational_json(2, 1),
            }),
        )
        .unwrap();
        assert_eq!(
            session.project().timeline().markers()[0].timeline_time(),
            RationalTime::new(2, 1).unwrap()
        );
        session.execute_command(undo(5)).unwrap();
        assert_eq!(
            session.project().timeline().markers()[0].timeline_time(),
            RationalTime::new(1, 1).unwrap()
        );
        session.execute_command(redo(6)).unwrap();
        assert_eq!(
            session.project().timeline().markers()[0].timeline_time(),
            RationalTime::new(2, 1).unwrap()
        );

        marker_add(&mut session, MARKER_B, (3, 1), "delete me").unwrap();
        execute(
            &mut session,
            "timeline.marker.delete",
            json!({"marker_id": MARKER_B}),
        )
        .unwrap();
        assert_eq!(session.project().timeline().markers().len(), 1);
        session.execute_command(undo(9)).unwrap();
        assert_eq!(session.project().timeline().markers().len(), 2);
        assert_eq!(
            session.project().timeline().markers()[1].id(),
            marker_id(MARKER_B)
        );
        session.execute_command(redo(10)).unwrap();
        assert_eq!(session.project().timeline().markers().len(), 1);
        assert_eq!(
            session.project().timeline().markers()[0].id(),
            marker_id(MARKER_A)
        );
    }

    #[test]
    fn timeline_marker_commands_validate_bounds_duplicates_and_transactions() {
        let mut session = fixed_session();
        marker_add(&mut session, MARKER_A, (1, 1), "label").unwrap();
        let before = session.project().clone();
        for arguments in [
            json!({
                "marker_id": MARKER_A,
                "timeline_time": rational_json(2, 1),
                "label": "duplicate",
            }),
            json!({
                "marker_id": MARKER_B,
                "timeline_time": rational_json(-1, 1),
                "label": "negative",
            }),
            json!({
                "marker_id": MARKER_B,
                "timeline_time": rational_json(2, 1),
                "label": "   ",
            }),
            json!({
                "marker_id": MARKER_B,
                "timeline_time": rational_json(2, 1),
                "label": "ok",
                "extra": true,
            }),
        ] {
            let result = execute(&mut session, "timeline.marker.add", arguments);
            assert!(matches!(
                result,
                Err(error) if error.code == OperationErrorCode::InvalidArguments
                    || error.code == OperationErrorCode::TimelineMarkerIdAlreadyExists
            ));
            assert_eq!(session.project(), &before);
        }
        let oversized = "x".repeat(MAX_TIMELINE_MARKER_LABEL_BYTES + 1);
        assert_eq!(
            code(&execute(
                &mut session,
                "timeline.marker.add",
                json!({
                    "marker_id": MARKER_B,
                    "timeline_time": rational_json(2, 1),
                    "label": oversized,
                }),
            )),
            OperationErrorCode::InvalidArguments
        );
        let transaction = transaction(
            vec![call(
                "timeline.marker.add",
                1,
                json!({
                    "marker_id": MARKER_B,
                    "timeline_time": rational_json(2, 1),
                    "label": "blocked",
                }),
            )],
            1,
        );
        assert_eq!(
            code(&session.execute_transaction(transaction)),
            OperationErrorCode::CommandNotAllowedInTransaction
        );
        assert_eq!(session.project_revision(), ProjectRevision::new(1));
    }

    #[test]
    fn timeline_marker_missing_and_revision_overflow_fail_atomically() {
        let mut missing = fixed_session();
        let before = missing.project().clone();
        for (command_id, arguments) in [
            (
                "timeline.marker.move",
                json!({
                    "marker_id": MARKER_A,
                    "timeline_time": rational_json(1, 1),
                }),
            ),
            (
                "timeline.marker.rename",
                json!({"marker_id": MARKER_A, "label": "missing"}),
            ),
            ("timeline.marker.delete", json!({"marker_id": MARKER_A})),
        ] {
            assert_eq!(
                code(&execute(&mut missing, command_id, arguments)),
                OperationErrorCode::TimelineMarkerNotFound
            );
            assert_eq!(missing.project(), &before);
            assert!(missing.history.undo.is_empty());
            assert!(missing.history.redo.is_empty());
        }

        for (command_id, arguments, has_marker) in [
            (
                "timeline.marker.add",
                json!({
                    "marker_id": MARKER_A,
                    "timeline_time": rational_json(1, 1),
                    "label": "overflow",
                }),
                false,
            ),
            (
                "timeline.marker.move",
                json!({
                    "marker_id": MARKER_A,
                    "timeline_time": rational_json(2, 1),
                }),
                true,
            ),
            (
                "timeline.marker.rename",
                json!({"marker_id": MARKER_A, "label": "overflow"}),
                true,
            ),
            (
                "timeline.marker.delete",
                json!({"marker_id": MARKER_A}),
                true,
            ),
        ] {
            let mut session = session_with_state("A", u64::MAX);
            if has_marker {
                session
                    .project
                    .set_timeline_for_test(ProjectTimeline::from_parts_for_codec(
                        vec![],
                        vec![TimelineMarker::from_parts_for_codec(
                            marker_id(MARKER_A),
                            RationalTime::new(1, 1).unwrap(),
                            "existing".to_owned(),
                        )],
                    ));
            }
            let before = session.project().clone();
            assert_eq!(
                code(&execute(&mut session, command_id, arguments)),
                OperationErrorCode::RevisionOverflow
            );
            assert_eq!(session.project(), &before);
            assert!(session.history.undo.is_empty());
            assert!(session.history.redo.is_empty());
        }
    }

    #[test]
    fn timeline_marker_limit_and_history_conflict_are_bounded_and_atomic() {
        let mut markers = Vec::with_capacity(MAX_TIMELINE_MARKERS);
        for index in 0..MAX_TIMELINE_MARKERS {
            markers.push(TimelineMarker::from_parts_for_codec(
                MarkerId::generate(),
                RationalTime::new(index as i64, 1).unwrap(),
                "bounded".to_owned(),
            ));
        }
        let mut project = ProjectDocument::new("A");
        project.set_timeline_for_test(ProjectTimeline::from_parts_for_codec(vec![], markers));
        let mut session = ProjectSession {
            project,
            project_instance_id: ProjectInstanceId::from_str(INSTANCE_ID).unwrap(),
            history: super::SessionHistory::default(),
        };
        let before = session.project().clone();
        assert_eq!(
            code(
                &session.execute_command(CommandEnvelope::add_timeline_marker(
                    session.project_id(),
                    session.project_instance_id(),
                    session.project_revision(),
                    marker_id(MARKER_A),
                    RationalTime::new(10001, 1).unwrap(),
                    "overflow",
                ))
            ),
            OperationErrorCode::TimelineLimitExceeded
        );
        assert_eq!(session.project(), &before);

        let mut session = fixed_session();
        marker_add(&mut session, MARKER_A, (1, 1), "conflict").unwrap();
        session
            .project
            .set_timeline_for_test(ProjectTimeline::default());
        let before = session.project().clone();
        let undo_stack = session.history.undo.clone();
        assert_eq!(
            code(&session.execute_command(undo(1))),
            OperationErrorCode::HistoryConflict
        );
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo, undo_stack);
        assert!(session.history.redo.is_empty());
    }

    #[test]
    fn timeline_marker_query_is_bounded_canonical_read_only_and_wire_compatible() {
        let mut session = fixed_session();
        marker_add(&mut session, MARKER_B, (2, 1), "same").unwrap();
        marker_add(&mut session, MARKER_A, (2, 1), "same").unwrap();
        marker_add(&mut session, MARKER_C, (1, 1), "same").unwrap();
        let before = session.project().clone();
        let undo = session.history.undo.clone();

        let page = session
            .execute_query(QueryEnvelope::timeline_markers(
                session.project_id(),
                session.project_instance_id(),
                1,
                2,
            ))
            .unwrap();
        let page_value = page.timeline_marker_page.unwrap();
        assert_eq!(page_value.total_count, 3);
        assert_eq!(page_value.offset, 1);
        assert_eq!(page_value.limit, 2);
        assert_eq!(page_value.next_offset, None);
        assert_eq!(page_value.items[0].marker_id, marker_id(MARKER_A));
        assert_eq!(page_value.items[1].marker_id, marker_id(MARKER_B));

        let empty = session
            .execute_query(QueryEnvelope::timeline_markers(
                session.project_id(),
                session.project_instance_id(),
                99,
                MAX_TIMELINE_MARKER_PAGE_SIZE,
            ))
            .unwrap();
        assert!(empty.timeline_marker_page.unwrap().items.is_empty());
        assert_eq!(
            code(&session.execute_query(QueryEnvelope::timeline_markers(
                session.project_id(),
                session.project_instance_id(),
                0,
                MAX_TIMELINE_MARKER_PAGE_SIZE + 1,
            ))),
            OperationErrorCode::InvalidArguments
        );
        for arguments in [
            json!({"offset": 0, "limit": 1, "unexpected": true}),
            json!({"offset": -1, "limit": 1}),
            json!({"offset": 0.5, "limit": 1}),
            json!({"offset": 0, "limit": "1"}),
        ] {
            let invalid = QueryEnvelope {
                query_id: "timeline.markers".to_owned(),
                schema_version: 1,
                project_id: session.project_id(),
                project_instance_id: session.project_instance_id(),
                arguments,
            };
            assert_eq!(
                code(&session.execute_query(invalid)),
                OperationErrorCode::InvalidArguments
            );
        }
        assert_eq!(session.project(), &before);
        assert_eq!(session.history.undo, undo);

        let summary = session
            .execute_query(summary_query())
            .expect("summary query should still use its original wire shape");
        assert!(
            !serde_json::to_value(summary)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("timeline_marker_page")
        );

        let mut bounded = fixed_session();
        let label = "x".repeat(MAX_TIMELINE_MARKER_LABEL_BYTES);
        for index in 0..MAX_TIMELINE_MARKER_PAGE_SIZE {
            let marker_id = format!("00000000-0000-4000-8000-{index:012x}");
            marker_add(&mut bounded, &marker_id, (index as i64, 1), &label).unwrap();
        }
        let encoded = serde_json::to_vec(
            &bounded
                .execute_query(QueryEnvelope::timeline_markers(
                    bounded.project_id(),
                    bounded.project_instance_id(),
                    0,
                    MAX_TIMELINE_MARKER_PAGE_SIZE,
                ))
                .unwrap(),
        )
        .unwrap();
        assert!(encoded.len() < 1_000_000);
    }

    #[test]
    fn timeline_markers_do_not_change_when_clip_edits_run() {
        let mut session = fixed_session();
        seed_video_track_and_clip(&mut session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (4, 1));
        marker_add(&mut session, MARKER_A, (10, 1), "persistent").unwrap();
        let expected = session.project().timeline().markers().to_vec();
        execute(
            &mut session,
            "timeline.clip.move",
            json!({"clip_id": CLIP_A, "track_id": TRACK_A, "timeline_start": rational_json(4, 1)}),
        )
        .unwrap();
        execute(
            &mut session,
            "timeline.clip.trim",
            json!({"clip_id": CLIP_A, "edge": "end", "timeline_time": rational_json(6, 1)}),
        )
        .unwrap();
        execute(
            &mut session,
            "timeline.clip.split",
            json!({
                "clip_id": CLIP_A,
                "new_clip_id": CLIP_B,
                "timeline_time": rational_json(5, 1),
            }),
        )
        .unwrap();
        execute(
            &mut session,
            "timeline.clip.ripple_delete",
            json!({"clip_id": CLIP_B}),
        )
        .unwrap();
        assert_eq!(session.project().timeline().markers(), expected.as_slice());
    }

    #[test]
    fn timeline_snap_v1_ignores_markers_and_v2_reports_marker_winners_and_ties() {
        let mut session = fixed_session();
        seed_video_track_and_clip(&mut session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (1, 1));
        marker_add(&mut session, MARKER_A, (3, 1), "snap").unwrap();
        let before_revision = session.project_revision();
        let before_history = session.history.undo.clone();
        let v1 = session
            .execute_query(QueryEnvelope::timeline_snap(
                session.project_id(),
                session.project_instance_id(),
                super::TimelineSnapOperation::TrimEnd,
                CLIP_A.parse().unwrap(),
                None,
                RationalTime::new(31, 10).unwrap(),
            ))
            .unwrap();
        assert_eq!(v1.schema_version, 1);
        let v1_json = serde_json::to_value(&v1).unwrap();
        assert!(
            !v1_json["timeline_snap"]
                .as_object()
                .unwrap()
                .contains_key("target_marker_id")
        );
        assert_eq!(
            v1.timeline_snap.unwrap().target_kind,
            TimelineSnapTargetKind::None
        );

        let v2 = session
            .execute_query(QueryEnvelope::timeline_snap_v2(
                session.project_id(),
                session.project_instance_id(),
                super::TimelineSnapOperation::TrimEnd,
                CLIP_A.parse().unwrap(),
                None,
                RationalTime::new(31, 10).unwrap(),
            ))
            .unwrap();
        assert_eq!(v2.schema_version, 2);
        let snap = v2.timeline_snap.unwrap();
        assert_eq!(snap.target_kind, TimelineSnapTargetKind::Marker);
        assert_eq!(snap.target_marker_id, Some(marker_id(MARKER_A)));
        assert_eq!(snap.target_time, RationalTime::new(3, 1).unwrap());
        assert_eq!(session.project_revision(), before_revision);
        assert_eq!(session.history.undo, before_history);

        let mut unsupported = QueryEnvelope::timeline_snap_v2(
            session.project_id(),
            session.project_instance_id(),
            super::TimelineSnapOperation::TrimEnd,
            CLIP_A.parse().unwrap(),
            None,
            RationalTime::new(31, 10).unwrap(),
        );
        unsupported.schema_version = 3;
        assert_eq!(
            code(&session.execute_query(unsupported)),
            OperationErrorCode::UnsupportedQuerySchema
        );
        let mut other_query = summary_query();
        other_query.schema_version = 2;
        assert_eq!(
            code(&session.execute_query(other_query)),
            OperationErrorCode::UnsupportedQuerySchema
        );

        let mut tie_session = fixed_session();
        seed_video_track_and_clip(&mut tie_session, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (1, 1));
        add_track(&mut tie_session, TRACK_B, "video").unwrap();
        insert_clip(
            &mut tie_session,
            CLIP_B,
            TRACK_B,
            MEDIA_A,
            (1, 1),
            (1, 1),
            (1, 1),
        )
        .unwrap();
        marker_add(&mut tie_session, MARKER_A, (1, 1), "tie").unwrap();
        let tie = tie_session
            .execute_query(QueryEnvelope::timeline_snap_v2(
                tie_session.project_id(),
                tie_session.project_instance_id(),
                super::TimelineSnapOperation::TrimEnd,
                CLIP_A.parse().unwrap(),
                None,
                RationalTime::new(1, 1).unwrap(),
            ))
            .unwrap();
        assert_eq!(
            tie.timeline_snap.unwrap().target_kind,
            TimelineSnapTargetKind::ClipStart
        );

        let mut zero_tie = fixed_session();
        seed_video_track_and_clip(&mut zero_tie, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (1, 1));
        marker_add(&mut zero_tie, MARKER_A, (0, 1), "zero").unwrap();
        let zero_tie = zero_tie
            .execute_query(QueryEnvelope::timeline_snap_v2(
                zero_tie.project_id(),
                zero_tie.project_instance_id(),
                super::TimelineSnapOperation::TrimStart,
                CLIP_A.parse().unwrap(),
                None,
                RationalTime::new(1, 20).unwrap(),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert_eq!(zero_tie.target_kind, TimelineSnapTargetKind::TimelineZero);
        assert_eq!(zero_tie.target_marker_id, None);

        let mut marker_tie = fixed_session();
        seed_video_track_and_clip(&mut marker_tie, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (1, 1));
        marker_add(&mut marker_tie, MARKER_B, (3, 1), "later").unwrap();
        marker_add(&mut marker_tie, MARKER_A, (3, 1), "earlier").unwrap();
        let marker_tie = marker_tie
            .execute_query(QueryEnvelope::timeline_snap_v2(
                marker_tie.project_id(),
                marker_tie.project_instance_id(),
                super::TimelineSnapOperation::TrimEnd,
                CLIP_A.parse().unwrap(),
                None,
                RationalTime::new(31, 10).unwrap(),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert_eq!(marker_tie.target_kind, TimelineSnapTargetKind::Marker);
        assert_eq!(marker_tie.target_marker_id, Some(marker_id(MARKER_A)));

        let mut move_end = fixed_session();
        seed_video_track_and_clip(&mut move_end, MEDIA_A, TRACK_A, CLIP_A, (0, 1), (1, 1));
        marker_add(&mut move_end, MARKER_A, (3, 1), "move end").unwrap();
        let move_end = move_end
            .execute_query(QueryEnvelope::timeline_snap_v2(
                move_end.project_id(),
                move_end.project_instance_id(),
                super::TimelineSnapOperation::Move,
                CLIP_A.parse().unwrap(),
                Some(TRACK_A.parse().unwrap()),
                RationalTime::new(21, 10).unwrap(),
            ))
            .unwrap()
            .timeline_snap
            .unwrap();
        assert_eq!(move_end.target_kind, TimelineSnapTargetKind::Marker);
        assert_eq!(move_end.target_marker_id, Some(marker_id(MARKER_A)));
        assert_eq!(move_end.moving_anchor, TimelineSnapMovingAnchor::End);
        assert_eq!(
            move_end.resolved_target_time,
            RationalTime::new(2, 1).unwrap()
        );
    }
}
