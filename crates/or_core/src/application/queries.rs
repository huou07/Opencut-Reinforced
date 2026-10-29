use crate::{MediaItem, TrackKind};
use serde_json::Value;

use super::*;

impl ProjectSession {
    pub(super) fn timeline_tracks(
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

    pub(super) fn timeline_tracks_v2(
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

    pub(super) fn timeline_clips(&self, arguments: Value) -> Result<TimelineClipPage, OperationError> {
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

    pub(super) fn timeline_clips_v2(&self, arguments: Value) -> Result<TimelineClipPageV2, OperationError> {
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

    pub(super) fn timeline_markers(&self, arguments: Value) -> Result<TimelineMarkerPage, OperationError> {
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

    pub(super) fn timeline_snap(
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

    pub(super) fn media_list(&self, arguments: Value) -> Result<MediaListPage, OperationError> {
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

    pub(super) fn media_get(&self, arguments: Value) -> Result<MediaItem, OperationError> {
        let arguments: MediaGetArguments = serde_json::from_value(arguments)
            .map_err(|_| OperationError::new(OperationErrorCode::InvalidArguments))?;
        self.project
            .media_items()
            .iter()
            .find(|item| item.id() == arguments.media_id)
            .cloned()
            .ok_or_else(|| OperationError::new(OperationErrorCode::MediaNotFound))
    }
}
