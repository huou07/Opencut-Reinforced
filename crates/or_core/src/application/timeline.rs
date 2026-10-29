use crate::{
    ClipId, ClipSettings, MAX_TIMELINE_MARKERS, RationalTime, TimelineTrack, TrackId, TrackKind,
    TrackState,
};
use serde_json::Value;

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
    pub(super) fn apply_timeline_track_move(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
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

    pub(super) fn apply_timeline_clip_slip(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
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
    pub(super) fn apply_timeline_clip_roll(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
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
    pub(super) fn apply_timeline_clip_slide(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
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

    pub(super) fn apply_timeline_clip_trim(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
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
    pub(super) fn apply_timeline_clip_duck(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
        let arguments: TimelineClipDuckArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        if !(-96_000..=-1_000).contains(&arguments.duck_millidecibels) {
            return Err(OperationError::new(OperationErrorCode::InvalidArguments));
        }
        let Some((track_index, index)) = find_timeline_clip(&self.project, arguments.clip_id)
        else {
            return Err(OperationError::new(OperationErrorCode::TimelineClipNotFound));
        };
        let track = &self.project.timeline().tracks()[track_index];
        let track_id = track.id();
        if track_id != arguments.track_id {
            return Err(OperationError::new(OperationErrorCode::TimelineClipNotFound));
        }
        if track.state().locked() {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackLocked));
        }
        if track.kind() != TrackKind::Audio {
            return Err(OperationError::new(OperationErrorCode::TimelineMediaIncompatible));
        }
        let Some(voice_index) =
            find_timeline_track_index(&self.project, arguments.voice_track_id)
        else {
            return Err(OperationError::new(OperationErrorCode::TimelineTrackNotFound));
        };
        if self.project.timeline().tracks()[voice_index].kind() != TrackKind::Audio {
            return Err(OperationError::new(OperationErrorCode::TimelineMediaIncompatible));
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
            return Err(OperationError::new(OperationErrorCode::TimelineNothingToDuck));
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
            if overlaps
                && let ClipSettings::Audio(audio) = &mut segment.settings
            {
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

    pub(super) fn apply_timeline_clip_split(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
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
    pub(super) fn apply_timeline_clips_move(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
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

    pub(super) fn apply_timeline_clip_move(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
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

    pub(super) fn apply_timeline_marker_add(&mut self, arguments: Value) -> Result<ChangeSet, OperationError> {
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
}
