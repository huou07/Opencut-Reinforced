use super::*;

impl ProjectSession {
    pub(super) fn apply_history(
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
