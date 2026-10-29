use crate::{
    ClipId, ClipSettings, MAX_TIMELINE_MARKERS, RationalTime, TimelineTrack, TrackId, TrackKind,
    TrackState,
};
use serde_json::Value;

use super::media::deserialize_canonical_media_id;
use super::*;

impl ProjectSession {
    pub(super) fn apply_timeline_track_add(
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

    pub(super) fn apply_timeline_track_remove(
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

    pub(super) fn apply_timeline_track_set_state(
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

    pub(super) fn apply_timeline_clip_insert(
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

    pub(super) fn apply_timeline_clip_insert_content(
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
    pub(super) fn overwrite_timeline_clip_state(
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
    pub(super) fn ripple_trim_timeline_clip_end(
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

    pub(super) fn insert_timeline_clip_state(
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
    pub(super) fn apply_timeline_track_move(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
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

    pub(super) fn apply_timeline_clip_slip(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
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
    pub(super) fn apply_timeline_clip_roll(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
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
    pub(super) fn apply_timeline_clip_slide(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
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

    pub(super) fn apply_timeline_clip_update(
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

    pub(super) fn apply_timeline_clip_delete(
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

    pub(super) fn apply_timeline_clip_trim(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
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

    /// Lowers one audio clip wherever another track's voice clips overlap it.
    ///
    /// The project model has static clip gain, so the edit is expressed as a
    /// whole-track replacement: the music clip is split at every overlapping
    /// voice boundary and the overlapping segments get the duck gain. Reusing
    /// `TimelineClipsOverwritten` keeps this one history entry and one undo step,
    /// unlike a client issuing split and update commands one by one.
    pub(super) fn apply_timeline_clip_duck(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipDuckArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if !(-96_000..=-1_000).contains(&arguments.duck_millidecibels) {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let Some((track_index, index)) = find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        };
        let track = &self.project.timeline().tracks()[track_index];
        let track_id = track.id();
        if track_id != arguments.track_id {
            return Err(OperationError::new(
                OperationErrorCode::TimelineClipNotFound,
            ));
        }
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        if track.kind() != TrackKind::Audio {
            return Err(OperationError::new(
                OperationErrorCode::TimelineMediaIncompatible,
            ));
        }
        let Some(voice_index) = find_timeline_track_index(&self.project, arguments.voice_track_id)
        else {
            return Err(OperationError::new(
                OperationErrorCode::TimelineTrackNotFound,
            ));
        };
        if self.project.timeline().tracks()[voice_index].kind() != TrackKind::Audio {
            return Err(OperationError::new(
                OperationErrorCode::TimelineMediaIncompatible,
            ));
        }
        let before = track_states(&self.project, track_index)?;
        let music = before
            .get(index)
            .cloned()
            .ok_or_else(|| OperationError::new(OperationErrorCode::TimelineClipNotFound))?;
        let music_end = timeline_clip_end(&music)?;
        let voice_states = track_states(&self.project, voice_index)?;
        let mut intervals: Vec<(RationalTime, RationalTime)> = Vec::new();
        for voice in &voice_states {
            let voice_end = timeline_clip_end(voice)?;
            let start = voice.timeline_start.max(music.timeline_start);
            let end = voice_end.min(music_end);
            if start < end {
                intervals.push((start, end));
            }
        }
        if intervals.is_empty() {
            return Err(OperationError::new(
                OperationErrorCode::TimelineNothingToDuck,
            ));
        }
        intervals.sort_by(|left, right| left.0.cmp(&right.0));
        let mut merged: Vec<(RationalTime, RationalTime)> = Vec::new();
        for (start, end) in intervals {
            if let Some(last) = merged.last_mut() {
                if start <= last.1 {
                    if end > last.1 {
                        last.1 = end;
                    }
                    continue;
                }
            }
            merged.push((start, end));
        }
        let mut boundaries: Vec<RationalTime> = Vec::new();
        for (start, end) in &merged {
            if *start > music.timeline_start && *start < music_end {
                boundaries.push(*start);
            }
            if *end > music.timeline_start && *end < music_end {
                boundaries.push(*end);
            }
        }
        boundaries.sort();
        boundaries.dedup();
        let mut segments: Vec<TimelineClipState> = Vec::new();
        let mut current = music.clone();
        for boundary in boundaries {
            let left_duration = boundary
                .checked_sub(current.timeline_start)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
            let right_duration = current
                .timeline_duration
                .checked_sub(left_duration)
                .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
            let (left_content, right_content) =
                split_content(&current.content, left_duration, right_duration)?;
            segments.push(TimelineClipState {
                timeline_duration: left_duration,
                content: left_content,
                ..current.clone()
            });
            current = TimelineClipState {
                clip_id: ClipId::generate(),
                timeline_start: boundary,
                timeline_duration: right_duration,
                content: right_content,
                ..current.clone()
            };
        }
        segments.push(current);
        for segment in &mut segments {
            let segment_end = timeline_clip_end(segment)?;
            let overlaps = merged
                .iter()
                .any(|(start, end)| segment.timeline_start < *end && segment_end > *start);
            if overlaps && let ClipSettings::Audio(audio) = &mut segment.settings {
                audio.gain_millidecibels = arguments.duck_millidecibels;
            }
        }
        let mut after = Vec::with_capacity(before.len().saturating_sub(1) + segments.len());
        after.extend_from_slice(&before[..index]);
        after.extend(segments);
        after.extend_from_slice(&before[index + 1..]);
        validate_track_states(&self.project, TrackKind::Audio, &after)?;
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

    pub(super) fn apply_timeline_clip_split(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
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

    pub(super) fn apply_timeline_clip_ripple_delete(
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
    pub(super) fn apply_timeline_clips_move(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
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

    pub(super) fn apply_timeline_clip_move(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
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

    pub(super) fn apply_timeline_sequence_set_frame_rate(
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

    pub(super) fn apply_timeline_marker_add(
        &mut self,
        arguments: Value,
    ) -> Result<ChangeSet, OperationError> {
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

    pub(super) fn apply_timeline_marker_move(
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

    pub(super) fn apply_timeline_marker_rename(
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

    pub(super) fn apply_timeline_marker_delete(
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

    pub(super) fn apply_timeline_captions_import(
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
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineSequenceSetFrameRateArguments {
    // The outer option records key presence; the inner option represents clearing.
    #[serde(default, deserialize_with = "deserialize_nullable_field")]
    sequence_frame_rate: Option<Option<RationalRate>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineTrackAddArguments {
    track_id: TrackId,
    kind: TrackKind,
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
struct TimelineClipDuckArguments {
    track_id: TrackId,
    clip_id: ClipId,
    voice_track_id: TrackId,
    duck_millidecibels: i32,
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
    #[serde(default)]
    mode: TimelineEditMode,
    /// Only read by `mode: overwrite` when the new span lands strictly inside one
    /// existing clip. The head keeps the original id and the tail takes this one,
    /// mirroring `timeline.clip.split`'s caller-supplied `new_clip_id`.
    #[serde(default)]
    overwrite_split_clip_id: Option<ClipId>,
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

#[derive(Clone, Copy)]
pub(super) struct TimelineSnapCandidate {
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
pub(super) struct TimelineSnapChoice {
    candidate: TimelineSnapCandidate,
    distance: RationalTime,
    moving_anchor: TimelineSnapMovingAnchor,
    resolved_target_time: RationalTime,
}

pub(super) fn deserialize_nullable_field<'de, D>(
    deserializer: D,
) -> Result<Option<Option<RationalRate>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<RationalRate>::deserialize(deserializer).map(Some)
}

pub(super) fn deserialize_caption_clips<'de, D>(
    deserializer: D,
) -> Result<Vec<TimelineClipState>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    crate::media::deserialize_limited_vec(
        deserializer,
        crate::subtitle_io::MAX_IMPORTED_CAPTIONS,
        "caption clips",
    )
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

pub(super) fn timeline_snap_distance(
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

pub(super) fn timeline_snap_choice_order(
    left: TimelineSnapChoice,
    right: TimelineSnapChoice,
) -> Ordering {
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

pub(super) fn timeline_snap_anchor_priority(anchor: TimelineSnapMovingAnchor) -> u8 {
    match anchor {
        TimelineSnapMovingAnchor::Start => 0,
        TimelineSnapMovingAnchor::End => 1,
        TimelineSnapMovingAnchor::None => 2,
    }
}

pub(super) fn timeline_snap_source_priority(target_kind: TimelineSnapTargetKind) -> u8 {
    match target_kind {
        TimelineSnapTargetKind::TimelineZero => 0,
        TimelineSnapTargetKind::ClipStart | TimelineSnapTargetKind::ClipEnd => 1,
        TimelineSnapTargetKind::Marker => 2,
        TimelineSnapTargetKind::None => 3,
    }
}

pub(super) fn timeline_snap_boundary_priority(target_kind: TimelineSnapTargetKind) -> u8 {
    match target_kind {
        TimelineSnapTargetKind::ClipStart => 0,
        TimelineSnapTargetKind::ClipEnd => 1,
        TimelineSnapTargetKind::TimelineZero
        | TimelineSnapTargetKind::Marker
        | TimelineSnapTargetKind::None => 0,
    }
}

pub(super) fn resolve_timeline_snap_with_markers(
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

pub(super) fn consider_timeline_snap_candidate(
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

pub(super) fn timeline_change_sets(
    change: ProjectChange,
) -> Result<(ChangeSet, ChangeSet), OperationError> {
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
pub(super) fn states_before_ripple(
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

pub(super) fn history_conflict() -> OperationError {
    OperationError::new(OperationErrorCode::HistoryConflict)
}

pub(super) fn timeline_clip_end(clip: &TimelineClipState) -> Result<RationalTime, OperationError> {
    clip.timeline_start
        .checked_add(clip.timeline_duration)
        .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))
}

pub(super) fn track_states(
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

pub(super) fn states_into_domain(
    states: Vec<TimelineClipState>,
) -> Result<Vec<TimelineClip>, OperationError> {
    let mut clips = Vec::new();
    clips
        .try_reserve(states.len())
        .map_err(|_| OperationError::new(OperationErrorCode::HistoryStorageFailure))?;
    clips.extend(states.into_iter().map(TimelineClipState::into_domain));
    Ok(clips)
}

pub(super) fn timeline_states_fingerprint(states: &[TimelineClipState]) -> u64 {
    let bytes = serde_json::to_vec(states).expect("timeline state serialization is infallible");
    bytes
        .iter()
        .fold(14_695_981_039_346_656_037_u64, |hash, byte| {
            hash.wrapping_mul(1_099_511_628_211)
                .wrapping_add(u64::from(*byte))
        })
}

pub(super) fn validate_track_states(
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

pub(super) fn stage_track_history_change(
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

pub(super) fn stage_caption_track_history_change(
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

pub(super) fn ensure_empty_track_at(
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

pub(super) fn stage_track_state_history_change(
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

pub(super) fn ensure_track_insertable_for_history(
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

pub(super) fn stage_clip_history_change(
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

pub(super) fn ensure_clip_matches_at(
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

pub(super) fn stage_clip_trim_history_change(
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

pub(super) fn stage_clip_update_history_change(
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

pub(super) fn stage_clip_split_history_change(
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
pub(super) fn stage_clip_ripple_history_change(
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

pub(super) fn ensure_clip_insertable_at(
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

pub(super) fn stage_marker_history_change(
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

pub(super) fn ensure_marker_matches_at(
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

pub(super) fn ensure_marker_insertable_at(
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

pub(super) fn stage_marker_move_history_change(
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

pub(super) fn stage_marker_rename_history_change(
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

pub(super) fn stage_clip_move_history_change(
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

pub(super) fn find_timeline_track_index(
    project: &ProjectDocument,
    track_id: TrackId,
) -> Option<usize> {
    project
        .timeline()
        .tracks()
        .iter()
        .position(|track| track.id() == track_id)
}

pub(super) fn find_timeline_clip(
    project: &ProjectDocument,
    clip_id: ClipId,
) -> Option<(usize, usize)> {
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

pub(super) fn find_timeline_marker(
    project: &ProjectDocument,
    marker_id: MarkerId,
) -> Option<usize> {
    project
        .timeline()
        .markers()
        .iter()
        .position(|marker| marker.id() == marker_id)
}

pub(super) fn marker_insertion_index(
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

pub(super) fn validate_marker_time(time: RationalTime) -> Result<(), OperationError> {
    if time.is_negative() {
        Err(OperationError::new(OperationErrorCode::InvalidArguments))
    } else {
        Ok(())
    }
}

pub(super) fn validate_marker_label(label: &str) -> Result<(), OperationError> {
    if label.trim().is_empty() || label.len() > MAX_TIMELINE_MARKER_LABEL_BYTES {
        Err(OperationError::new(OperationErrorCode::InvalidArguments))
    } else {
        Ok(())
    }
}

pub(super) fn timeline_clip_count(project: &ProjectDocument) -> Option<usize> {
    project
        .timeline()
        .tracks()
        .iter()
        .try_fold(0usize, |count, track| {
            count.checked_add(track.clips().len())
        })
}

pub(super) fn ensure_timeline_clip_capacity(
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

pub(super) fn validate_clip_state(
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

pub(super) fn clip_insertion_index(
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

pub(super) fn trim_content_start(
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

pub(super) fn trim_content_end(
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

pub(super) fn split_content(
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

pub(super) fn ensure_media_id_available(
    project: &ProjectDocument,
    id: MediaId,
) -> Result<(), OperationError> {
    if project.media_items().iter().any(|item| item.id() == id) {
        Err(OperationError::new(
            OperationErrorCode::MediaIdAlreadyExists,
        ))
    } else {
        Ok(())
    }
}

pub(super) fn ensure_media_source_available(
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

pub(super) fn ensure_media_matches_at(
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

pub(super) fn ensure_media_insertable(
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
