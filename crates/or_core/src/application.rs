use crate::{
    ClipId, MediaId, MediaItem, ProjectDocument, ProjectId, ProjectInstanceId, ProjectRevision,
    RationalTime, TimeRange, TimelineClip, TimelineTrack, TrackId, TrackKind,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{cmp::Ordering, error::Error, fmt};

const PROJECT_RENAME_ID: &str = "project.rename";
const PROJECT_SUMMARY_ID: &str = "project.summary";
const HISTORY_UNDO_ID: &str = "history.undo";
const HISTORY_REDO_ID: &str = "history.redo";
const MEDIA_ADD_ID: &str = "media.add";
const MEDIA_REMOVE_ID: &str = "media.remove";
const MEDIA_LIST_ID: &str = "media.list";
const MEDIA_GET_ID: &str = "media.get";
const TIMELINE_TRACK_ADD_ID: &str = "timeline.track.add";
const TIMELINE_TRACK_REMOVE_ID: &str = "timeline.track.remove";
const TIMELINE_CLIP_INSERT_ID: &str = "timeline.clip.insert";
const TIMELINE_CLIP_MOVE_ID: &str = "timeline.clip.move";
const TIMELINE_CLIP_DELETE_ID: &str = "timeline.clip.delete";
const TIMELINE_CLIP_TRIM_ID: &str = "timeline.clip.trim";
const TIMELINE_CLIP_SPLIT_ID: &str = "timeline.clip.split";
const TIMELINE_CLIP_RIPPLE_DELETE_ID: &str = "timeline.clip.ripple_delete";
const TIMELINE_TRACKS_ID: &str = "timeline.tracks";
const TIMELINE_CLIPS_ID: &str = "timeline.clips";
const TIMELINE_SNAP_ID: &str = "timeline.snap";
const OPERATION_SCHEMA_VERSION: u64 = 1;
pub const MAX_MEDIA_PAGE_SIZE: usize = 100;
pub const MAX_TIMELINE_CLIP_PAGE_SIZE: usize = 100;
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

const COMMANDS: [CommandDescriptor; 13] = [
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
        id: TIMELINE_TRACK_ADD_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
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
        id: TIMELINE_CLIP_INSERT_ID,
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
];

const QUERIES: [QueryDescriptor; 6] = [
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
        schema_version: OPERATION_SCHEMA_VERSION,
    },
    QueryDescriptor {
        id: TIMELINE_CLIPS_ID,
        schema_version: OPERATION_SCHEMA_VERSION,
    },
    QueryDescriptor {
        id: TIMELINE_SNAP_ID,
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

    pub fn add_timeline_track(
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
        expected_project_revision: ProjectRevision,
        track_id: TrackId,
        kind: TrackKind,
    ) -> Self {
        Self {
            command_id: TIMELINE_TRACK_ADD_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({ "track_id": track_id, "kind": kind }),
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
        Self {
            command_id: TIMELINE_CLIP_INSERT_ID.to_owned(),
            schema_version: OPERATION_SCHEMA_VERSION,
            project_id,
            project_instance_id,
            expected_project_revision,
            arguments: serde_json::json!({
                "clip_id": clip_id,
                "track_id": track_id,
                "media_id": media_id,
                "timeline_start": timeline_start,
                "source_range": source_range,
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
}

/// Serializable application representation of one canonical timeline clip.
///
/// This is deliberately separate from the private `.orproj` codec structures.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineClipState {
    pub clip_id: ClipId,
    pub media_id: MediaId,
    pub timeline_start: RationalTime,
    pub source_range: TimeRange,
}

impl From<&TimelineClip> for TimelineClipState {
    fn from(clip: &TimelineClip) -> Self {
        Self {
            clip_id: clip.id(),
            media_id: clip.media_id(),
            timeline_start: clip.timeline_start(),
            source_range: clip.source_range(),
        }
    }
}

impl TimelineClipState {
    fn into_domain(self) -> TimelineClip {
        TimelineClip::from_parts_for_command(
            self.clip_id,
            self.media_id,
            self.timeline_start,
            self.source_range,
        )
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
    MediaAdded {
        item: MediaItem,
        index: usize,
    },
    MediaRemoved {
        item: MediaItem,
        index: usize,
    },
    TimelineTrackAdded {
        track_id: TrackId,
        track_kind: TrackKind,
        index: usize,
    },
    TimelineTrackRemoved {
        track_id: TrackId,
        track_kind: TrackKind,
        index: usize,
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
        clip_id: ClipId,
        media_id: MediaId,
        source_range: TimeRange,
        from_track_id: TrackId,
        from_index: usize,
        from_timeline_start: RationalTime,
        to_track_id: TrackId,
        to_index: usize,
        to_timeline_start: RationalTime,
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
            ProjectChange::TimelineTrackAdded {
                track_id,
                track_kind,
                index,
            } => stage_track_history_change(project, *track_id, *track_kind, *index, reverse, true),
            ProjectChange::TimelineTrackRemoved {
                track_id,
                track_kind,
                index,
            } => {
                stage_track_history_change(project, *track_id, *track_kind, *index, reverse, false)
            }
            ProjectChange::TimelineClipInserted {
                track_id,
                index,
                clip,
            } => stage_clip_history_change(project, *track_id, *index, *clip, reverse, true),
            ProjectChange::TimelineClipDeleted {
                track_id,
                index,
                clip,
            } => stage_clip_history_change(project, *track_id, *index, *clip, reverse, false),
            ProjectChange::TimelineClipMoved {
                clip_id,
                media_id,
                source_range,
                from_track_id,
                from_index,
                from_timeline_start,
                to_track_id,
                to_index,
                to_timeline_start,
            } => stage_clip_move_history_change(
                project,
                *clip_id,
                *media_id,
                *source_range,
                (*from_track_id, *from_index, *from_timeline_start),
                (*to_track_id, *to_index, *to_timeline_start),
                reverse,
            ),
            ProjectChange::TimelineClipTrimmed {
                track_id,
                index,
                before,
                after,
            } => {
                stage_clip_trim_history_change(project, *track_id, *index, *before, *after, reverse)
            }
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
                *before,
                *left_after,
                *right_after,
                reverse,
            ),
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
                *deleted_clip,
                *shifted_count,
                *shift_duration,
                reverse,
                self.ripple_history_guard,
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
}

enum StagedHistoryChange {
    Rename(String),
    InsertMedia {
        item: MediaItem,
        index: usize,
    },
    RemoveMedia {
        index: usize,
    },
    InsertTimelineTrack {
        track_id: TrackId,
        kind: TrackKind,
        index: usize,
    },
    RemoveTimelineTrack {
        index: usize,
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
    pub timeline_clip_page: Option<Box<TimelineClipPage>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_snap: Option<Box<TimelineSnapResult>>,
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
    pub items: Vec<TimelineClipState>,
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
    TimelineClipIdAlreadyExists,
    TimelineClipNotFound,
    TimelineMediaIncompatible,
    TimelineOverlap,
    TimelineLimitExceeded,
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
    fn new(code: OperationErrorCode) -> Self {
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
            OperationErrorCode::TimelineLimitExceeded => {
                "timeline operation exceeds a configured limit"
            }
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
    Error(OperationError),
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
        if envelope.schema_version != descriptor.schema_version {
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
            TIMELINE_TRACK_ADD_ID => self.apply_timeline_track_add(envelope.arguments)?,
            TIMELINE_TRACK_REMOVE_ID => self.apply_timeline_track_remove(envelope.arguments)?,
            TIMELINE_CLIP_INSERT_ID => self.apply_timeline_clip_insert(envelope.arguments)?,
            TIMELINE_CLIP_MOVE_ID => self.apply_timeline_clip_move(envelope.arguments)?,
            TIMELINE_CLIP_DELETE_ID => self.apply_timeline_clip_delete(envelope.arguments)?,
            TIMELINE_CLIP_TRIM_ID => self.apply_timeline_clip_trim(envelope.arguments)?,
            TIMELINE_CLIP_SPLIT_ID => self.apply_timeline_clip_split(envelope.arguments)?,
            TIMELINE_CLIP_RIPPLE_DELETE_ID => {
                self.apply_timeline_clip_ripple_delete(envelope.arguments)?
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
        if envelope.schema_version != descriptor.schema_version {
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

        let (media_page, media_item, timeline_tracks, timeline_clip_page, timeline_snap) =
            if envelope.query_id == PROJECT_SUMMARY_ID {
                if !is_empty_object(&envelope.arguments) {
                    return Err(OperationError::new(OperationErrorCode::InvalidArguments));
                }
                (None, None, None, None, None)
            } else if envelope.query_id == MEDIA_LIST_ID {
                (
                    Some(self.media_list(envelope.arguments)?),
                    None,
                    None,
                    None,
                    None,
                )
            } else if envelope.query_id == MEDIA_GET_ID {
                (
                    None,
                    Some(Box::new(self.media_get(envelope.arguments)?)),
                    None,
                    None,
                    None,
                )
            } else if envelope.query_id == TIMELINE_TRACKS_ID {
                (
                    None,
                    None,
                    Some(self.timeline_tracks(envelope.arguments)?),
                    None,
                    None,
                )
            } else if envelope.query_id == TIMELINE_CLIPS_ID {
                (
                    None,
                    None,
                    None,
                    Some(Box::new(self.timeline_clips(envelope.arguments)?)),
                    None,
                )
            } else if envelope.query_id == TIMELINE_SNAP_ID {
                (
                    None,
                    None,
                    None,
                    None,
                    Some(Box::new(self.timeline_snap(envelope.arguments)?)),
                )
            } else {
                return Err(OperationError::new(OperationErrorCode::UnknownQuery));
            };

        Ok(QueryResult {
            query_id: envelope.query_id,
            schema_version: descriptor.schema_version,
            summary: ProjectSummary {
                project_id: self.project_id(),
                project_instance_id: self.project_instance_id,
                project_revision: self.project_revision(),
                name: self.project.name().to_owned(),
            },
            media_page,
            media_item,
            timeline_tracks,
            timeline_clip_page,
            timeline_snap,
        })
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

    fn apply_timeline_track_add(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineTrackAddArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
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
        let kind = track.kind();
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineTrackRemoved {
                track_id: arguments.track_id,
                track_kind: kind,
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

    fn apply_timeline_clip_insert(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipInsertArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let track_id = arguments.track_id;
        let clip = arguments.into_clip_state()?;
        let track_index = find_timeline_track_index(&self.project, track_id)
            .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineTrackNotFound))?;
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
        let index = clip_insertion_index(&self.project, track_index, clip, None)?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) =
            timeline_change_sets(ProjectChange::TimelineClipInserted {
                track_id,
                index,
                clip,
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
        let timeline_end = timeline_clip_end(before)?;
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
                let source_start = before
                    .source_range
                    .start()
                    .checked_add(delta)
                    .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
                let duration = timeline_end
                    .checked_sub(target)
                    .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
                TimelineClipState {
                    timeline_start: target,
                    source_range: TimeRange::new(source_start, duration)
                        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?,
                    ..before
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
                    source_range: TimeRange::new(before.source_range.start(), duration)
                        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?,
                    ..before
                }
            }
        };

        let mut states = track_states(&self.project, track_index)?;
        states[index] = after;
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
                after,
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
        let track_id = track.id();
        let before = TimelineClipState::from(&track.clips()[index]);
        let original_end = timeline_clip_end(before)?;
        if split_time <= before.timeline_start || split_time >= original_end {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let left_duration = split_time
            .checked_sub(before.timeline_start)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let right_duration = before
            .source_range
            .duration()
            .checked_sub(left_duration)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let right_source_start = before
            .source_range
            .start()
            .checked_add(left_duration)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        let left_after = TimelineClipState {
            source_range: TimeRange::new(before.source_range.start(), left_duration)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?,
            ..before
        };
        let right_after = TimelineClipState {
            clip_id: arguments.new_clip_id,
            timeline_start: split_time,
            source_range: TimeRange::new(right_source_start, right_duration)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?,
            ..before
        };
        let mut states = track_states(&self.project, track_index)?;
        states[index] = left_after;
        states
            .try_reserve(1)
            .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
        states.insert(index + 1, right_after);
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
        let track_id = track.id();
        let deleted_clip = TimelineClipState::from(&track.clips()[index]);
        let shift_duration = deleted_clip.source_range.duration();
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
        if from_track.kind() != to_track.kind() {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let current = TimelineClipState::from(&from_track.clips()[from_index]);
        let moved = TimelineClipState {
            timeline_start: arguments.timeline_start.into_time()?,
            ..current
        };
        validate_clip_state(&self.project, to_track.kind(), moved)?;

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
            moved,
            Some(arguments.clip_id),
        )?;
        let after_revision = self
            .project_revision()
            .checked_next()
            .map_err(|_| OperationError::new(OperationErrorCode::RevisionOverflow))?;
        let (change_set, history_entry) = timeline_change_sets(ProjectChange::TimelineClipMoved {
            clip_id: current.clip_id,
            media_id: current.media_id,
            source_range: current.source_range,
            from_track_id: from_track.id(),
            from_index,
            from_timeline_start: current.timeline_start,
            to_track_id: to_track.id(),
            to_index,
            to_timeline_start: moved.timeline_start,
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

    fn timeline_tracks(
        &self,
        arguments: Value,
    ) -> Result<Vec<TimelineTrackSummary>, OperationError> {
        parse_empty_arguments(arguments)?;
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
        let total_count = track.clips().len();
        let start = offset.min(total_count);
        let end = offset.saturating_add(limit).min(total_count);
        let items = track.clips()[start..end]
            .iter()
            .map(TimelineClipState::from)
            .collect();
        Ok(TimelineClipPage {
            track_id: arguments.track_id,
            items,
            total_count,
            offset,
            limit,
            next_offset: (end < total_count).then_some(end),
        })
    }

    fn timeline_snap(&self, arguments: Value) -> Result<TimelineSnapResult, OperationError> {
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
                let duration = clip.source_range().duration();
                resolve_timeline_snap(
                    &self.project,
                    arguments.clip_id,
                    TimelineSnapMode::Move { duration },
                    target_time,
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
                resolve_timeline_snap(
                    &self.project,
                    arguments.clip_id,
                    TimelineSnapMode::Trim { edge },
                    target_time,
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
            StagedHistoryChange::InsertTimelineTrack { .. } => {
                self.project.try_reserve_timeline_tracks(1)
            }
            StagedHistoryChange::InsertTimelineClip { track_index, .. } => self
                .project
                .try_reserve_timeline_track_clips(*track_index, 1),
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
            StagedHistoryChange::InsertMedia { item, index } => {
                self.project
                    .insert_media_for_command(item, index, after_revision);
            }
            StagedHistoryChange::RemoveMedia { index } => {
                self.project.remove_media_for_command(index, after_revision);
            }
            StagedHistoryChange::InsertTimelineTrack {
                track_id,
                kind,
                index,
            } => self.project.insert_timeline_track_for_command(
                index,
                TimelineTrack::empty_for_command(track_id, kind),
                after_revision,
            ),
            StagedHistoryChange::RemoveTimelineTrack { index } => {
                self.project
                    .remove_timeline_track_for_command(index, after_revision);
            }
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
struct TimelineTrackAddArguments {
    track_id: TrackId,
    kind: TrackKind,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineTrackRemoveArguments {
    track_id: TrackId,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipInsertArguments {
    clip_id: ClipId,
    track_id: TrackId,
    #[serde(deserialize_with = "deserialize_canonical_media_id")]
    media_id: MediaId,
    timeline_start: RationalTimeArguments,
    source_range: TimelineSourceRangeArguments,
}

impl TimelineClipInsertArguments {
    fn into_clip_state(self) -> Result<TimelineClipState, OperationError> {
        let start = self.source_range.start.into_time()?;
        let duration = self.source_range.duration.into_time()?;
        let source_range = TimeRange::new(start, duration)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        Ok(TimelineClipState {
            clip_id: self.clip_id,
            media_id: self.media_id,
            timeline_start: self.timeline_start.into_time()?,
            source_range,
        })
    }
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
struct TimelineClipDeleteArguments {
    clip_id: ClipId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineClipTrimArguments {
    clip_id: ClipId,
    edge: TimelineTrimEdge,
    timeline_time: RationalTimeArguments,
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
struct TimelineClipsQueryArguments {
    track_id: TrackId,
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
    target_track_id: Option<TrackId>,
    target_clip_id: Option<ClipId>,
}

#[derive(Clone, Copy)]
struct TimelineSnapChoice {
    candidate: TimelineSnapCandidate,
    distance: RationalTime,
    moving_anchor: TimelineSnapMovingAnchor,
    resolved_target_time: RationalTime,
}

fn resolve_timeline_snap(
    project: &ProjectDocument,
    active_clip_id: ClipId,
    mode: TimelineSnapMode,
    raw_target_time: RationalTime,
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
        target_track_id: None,
        target_clip_id: None,
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
                        .checked_add(clip.source_range().duration())
                        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?,
                ),
            ];
            for (target_kind, time) in boundaries {
                let candidate = TimelineSnapCandidate {
                    time,
                    target_kind,
                    track_index,
                    clip_index,
                    target_track_id: Some(track.id()),
                    target_clip_id: Some(clip.id()),
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
    )
        .cmp(&(
            right.distance,
            right.candidate.time,
            timeline_snap_anchor_priority(right.moving_anchor),
            timeline_snap_source_priority(right.candidate.target_kind),
            right.candidate.track_index,
            right.candidate.clip_index,
            timeline_snap_boundary_priority(right.candidate.target_kind),
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
        TimelineSnapTargetKind::None => 2,
    }
}

fn timeline_snap_boundary_priority(target_kind: TimelineSnapTargetKind) -> u8 {
    match target_kind {
        TimelineSnapTargetKind::ClipStart => 0,
        TimelineSnapTargetKind::ClipEnd => 1,
        TimelineSnapTargetKind::TimelineZero | TimelineSnapTargetKind::None => 0,
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

fn history_conflict() -> OperationError {
    OperationError::new(OperationErrorCode::HistoryConflict)
}

fn timeline_clip_end(clip: TimelineClipState) -> Result<RationalTime, OperationError> {
    clip.timeline_start
        .checked_add(clip.source_range.duration())
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
        let end = validate_clip_state(project, track_kind, *state)?;
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
    index: usize,
    reverse: bool,
    was_added: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let remove = reverse == was_added;
    if remove {
        ensure_empty_track_at(project, track_id, kind, index)?;
        Ok((
            StagedHistoryChange::RemoveTimelineTrack { index },
            ChangeSet::try_single(ProjectChange::TimelineTrackRemoved {
                track_id,
                track_kind: kind,
                index,
            })?,
        ))
    } else {
        ensure_track_insertable_for_history(project, track_id, index)?;
        Ok((
            StagedHistoryChange::InsertTimelineTrack {
                track_id,
                kind,
                index,
            },
            ChangeSet::try_single(ProjectChange::TimelineTrackAdded {
                track_id,
                track_kind: kind,
                index,
            })?,
        ))
    }
}

fn ensure_empty_track_at(
    project: &ProjectDocument,
    track_id: TrackId,
    kind: TrackKind,
    index: usize,
) -> Result<(), OperationError> {
    match project.timeline().tracks().get(index) {
        Some(track)
            if track.id() == track_id && track.kind() == kind && track.clips().is_empty() =>
        {
            Ok(())
        }
        _ => Err(history_conflict()),
    }
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
    clip: TimelineClipState,
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
                clip,
            })?,
        ))
    } else {
        let track_index = ensure_clip_insertable_at(project, track_id, index, clip)?;
        Ok((
            StagedHistoryChange::InsertTimelineClip {
                track_index,
                index,
                clip,
            },
            ChangeSet::try_single(ProjectChange::TimelineClipInserted {
                track_id,
                index,
                clip,
            })?,
        ))
    }
}

fn ensure_clip_matches_at(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    expected: TimelineClipState,
) -> Result<usize, OperationError> {
    let track_index = find_timeline_track_index(project, track_id).ok_or_else(history_conflict)?;
    let Some(clip) = project.timeline().tracks()[track_index].clips().get(index) else {
        return Err(history_conflict());
    };
    if TimelineClipState::from(clip) == expected {
        Ok(track_index)
    } else {
        Err(history_conflict())
    }
}

fn stage_clip_trim_history_change(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    before: TimelineClipState,
    after: TimelineClipState,
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
    states[index] = target;
    validate_track_states(project, track.kind(), &states).map_err(|_| history_conflict())?;
    Ok((
        StagedHistoryChange::ReplaceTimelineClip {
            track_index,
            index,
            clip: target,
        },
        ChangeSet::try_single(ProjectChange::TimelineClipTrimmed {
            track_id,
            index,
            before: expected,
            after: target,
        })?,
    ))
}

fn stage_clip_split_history_change(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    before: TimelineClipState,
    left_after: TimelineClipState,
    right_after: TimelineClipState,
    reverse: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let track_index = find_timeline_track_index(project, track_id).ok_or_else(history_conflict)?;
    let track = &project.timeline().tracks()[track_index];
    let mut states = track_states(project, track_index).map_err(|_| history_conflict())?;
    if reverse {
        if states.get(index) != Some(&left_after) || states.get(index + 1) != Some(&right_after) {
            return Err(history_conflict());
        }
        states.remove(index + 1);
        states[index] = before;
    } else {
        if states.get(index) != Some(&before)
            || find_timeline_clip(project, right_after.clip_id).is_some()
            || timeline_clip_count(project).is_none_or(|count| count >= crate::MAX_TIMELINE_CLIPS)
            || states.len() >= crate::MAX_TIMELINE_CLIPS_PER_TRACK
        {
            return Err(history_conflict());
        }
        states[index] = left_after;
        states.try_reserve(1).map_err(|_| history_conflict())?;
        states.insert(index + 1, right_after);
    }
    validate_track_states(project, track.kind(), &states).map_err(|_| history_conflict())?;
    let clips = states_into_domain(states).map_err(|_| history_conflict())?;
    Ok((
        StagedHistoryChange::ReplaceTimelineTrack { track_index, clips },
        ChangeSet::try_single(ProjectChange::TimelineClipSplit {
            track_id,
            index,
            before,
            left_after,
            right_after,
        })?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn stage_clip_ripple_history_change(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    deleted_clip: TimelineClipState,
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
        states.insert(index, deleted_clip);
        for state in states.iter_mut().skip(index + 1) {
            state.timeline_start = state
                .timeline_start
                .checked_add(shift_duration)
                .map_err(|_| history_conflict())?;
        }
    } else {
        let suffix_len = shifted_count.checked_add(1).ok_or_else(history_conflict)?;
        let expected_len = index.checked_add(suffix_len).ok_or_else(history_conflict)?;
        if states.len() != expected_len || states.get(index) != Some(&deleted_clip) {
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
            deleted_clip,
            shifted_count,
            shift_duration,
        })?,
    ))
}

fn ensure_clip_insertable_at(
    project: &ProjectDocument,
    track_id: TrackId,
    index: usize,
    clip: TimelineClipState,
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

fn stage_clip_move_history_change(
    project: &ProjectDocument,
    clip_id: ClipId,
    media_id: MediaId,
    source_range: TimeRange,
    from: (TrackId, usize, RationalTime),
    to: (TrackId, usize, RationalTime),
    reverse: bool,
) -> Result<(StagedHistoryChange, ChangeSet), OperationError> {
    let (current, target) = if reverse { (to, from) } else { (from, to) };
    let current_track_index =
        find_timeline_track_index(project, current.0).ok_or_else(history_conflict)?;
    let target_track_index =
        find_timeline_track_index(project, target.0).ok_or_else(history_conflict)?;
    let expected = TimelineClipState {
        clip_id,
        media_id,
        timeline_start: current.2,
        source_range,
    };
    ensure_clip_matches_at(project, current.0, current.1, expected)?;
    let tracks = project.timeline().tracks();
    if tracks[current_track_index].kind() != tracks[target_track_index].kind() {
        return Err(history_conflict());
    }
    let moved = TimelineClipState {
        timeline_start: target.2,
        ..expected
    };
    let actual_index = clip_insertion_index(project, target_track_index, moved, Some(clip_id))
        .map_err(|_| history_conflict())?;
    if actual_index != target.1
        || (current_track_index != target_track_index
            && tracks[target_track_index].clips().len() >= crate::MAX_TIMELINE_CLIPS_PER_TRACK)
    {
        return Err(history_conflict());
    }
    Ok((
        StagedHistoryChange::MoveTimelineClip {
            from_track_index: current_track_index,
            from_index: current.1,
            to_track_index: target_track_index,
            to_index: target.1,
            to_timeline_start: target.2,
        },
        ChangeSet::try_single(ProjectChange::TimelineClipMoved {
            clip_id,
            media_id,
            source_range,
            from_track_id: current.0,
            from_index: current.1,
            from_timeline_start: current.2,
            to_track_id: target.0,
            to_index: target.1,
            to_timeline_start: target.2,
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
    clip: TimelineClipState,
) -> Result<RationalTime, OperationError> {
    if clip.timeline_start.is_negative()
        || clip.source_range.start().is_negative()
        || !clip.source_range.duration().is_positive()
    {
        return Err(OperationError::new(OperationErrorCode::InvalidArguments));
    }
    clip.timeline_start
        .checked_add(clip.source_range.duration())
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
    let source_end = clip
        .source_range
        .start()
        .checked_add(clip.source_range.duration())
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
    let media = project
        .media_items()
        .iter()
        .find(|item| item.id() == clip.media_id)
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
    clip.timeline_start
        .checked_add(clip.source_range.duration())
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))
}

fn clip_insertion_index(
    project: &ProjectDocument,
    track_index: usize,
    clip: TimelineClipState,
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
            .checked_add(existing.source_range().duration())
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
    if command.schema_version != descriptor.schema_version {
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
        AudioStreamMetadata, ClipId, MAX_MEDIA_PAGE_SIZE, MAX_TIMELINE_CLIP_PAGE_SIZE,
        MAX_TIMELINE_CLIPS, MAX_TIMELINE_TRACKS, MediaId, MediaItem, MediaMetadata, MediaSourceRef,
        MediaStreamMetadata, ProjectDocument, ProjectId, ProjectInstanceId, ProjectRevision,
        ProjectTimeline, RationalRate, RationalTime, TimeRange, TimelineClip, TimelineTrack,
        TrackId, TrackKind, VideoStreamMetadata, decode_project, encode_project,
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
                    id: "timeline.track.add",
                    schema_version: 1,
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
                    id: "timeline.clip.insert",
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
                    schema_version: 1,
                },
                QueryDescriptor {
                    id: "timeline.clips",
                    schema_version: 1,
                },
                QueryDescriptor {
                    id: "timeline.snap",
                    schema_version: 1,
                },
            ]
        );
        assert_eq!(command_catalog(), command_catalog());
        assert_eq!(COMMANDS.len(), 13);
        assert_eq!(QUERIES.len(), 6);
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
        assert_eq!(clips[1].media_id().to_string(), MEDIA_A);
        assert_eq!(clips[1].timeline_start(), RationalTime::new(4, 1).unwrap());
        assert_eq!(
            clips[1].source_range(),
            TimeRange::new(
                RationalTime::new(3, 2).unwrap(),
                RationalTime::new(2, 1).unwrap()
            )
            .unwrap()
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
                    && after.source_range.start() == RationalTime::new(2, 1).unwrap()
                    && after.source_range.duration() == RationalTime::new(5, 1).unwrap()
        ));
        assert_eq!(
            start.after_revision,
            start.before_revision.checked_next().unwrap()
        );
        let trimmed = &session.project().timeline().tracks()[0].clips()[0];
        assert_eq!(trimmed.timeline_start(), RationalTime::new(3, 1).unwrap());
        assert_eq!(
            trimmed.source_range().start(),
            RationalTime::new(2, 1).unwrap()
        );
        assert_eq!(
            trimmed.source_range().duration(),
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
            TimeRange::new(
                RationalTime::new(2, 1).unwrap(),
                RationalTime::new(4, 1).unwrap()
            )
            .unwrap()
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
            clip.source_range().start(),
            RationalTime::new(7, 2).unwrap()
        );
        assert_eq!(
            clip.source_range().duration(),
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
                    && left_after.source_range == TimeRange::new(RationalTime::new(1, 1).unwrap(), RationalTime::new(3, 1).unwrap()).unwrap()
                    && right_after.clip_id == ClipId::from_str(CLIP_B).unwrap()
                    && right_after.timeline_start == RationalTime::new(5, 1).unwrap()
                    && right_after.source_range == TimeRange::new(RationalTime::new(4, 1).unwrap(), RationalTime::new(3, 1).unwrap()).unwrap()
        ));
        let clips = session.project().timeline().tracks()[0].clips();
        assert_eq!(clips.len(), 2);
        assert_eq!(clips[0].id().to_string(), CLIP_A);
        assert_eq!(clips[1].id().to_string(), CLIP_B);
        assert_eq!(
            clips[0].source_range().duration(),
            RationalTime::new(3, 1).unwrap()
        );
        assert_eq!(
            clips[1].source_range().start(),
            RationalTime::new(4, 1).unwrap()
        );
        assert_eq!(
            clips[1].source_range().duration(),
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
        assert_eq!(moved_clip.media_id.to_string(), MEDIA_A);
        assert_eq!(
            moved_clip.source_range.start(),
            RationalTime::new(1, 2).unwrap()
        );
        assert_eq!(
            moved_clip.source_range.duration(),
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
        assert_eq!(destination.media_id().to_string(), MEDIA_A);
        assert_eq!(
            destination.source_range().start(),
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
            "timeline.clip.insert",
            "timeline.clip.move",
            "timeline.clip.delete",
            "timeline.clip.trim",
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
}
