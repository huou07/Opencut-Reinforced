//! Preview transport and preparation commands.

use super::project::*;
use crate::preview::{PreviewError, PreviewPreparationAction, PreviewSnapshot};
use or_core::{RationalRate, RationalTime};

impl ProjectHostHandle {
    pub fn preview_state(&self) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .status(&self.host)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_seek(
        &self,
        position: RationalTimeView,
    ) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        let position =
            RationalTime::new(position.numerator, position.denominator).map_err(|error| {
                ProjectBridgeError {
                    code: "INVALID_PREVIEW_TIME".to_owned(),
                    message: error.to_string(),
                }
            })?;
        self.preview_runtime
            .seek(&self.host, position)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_play(&self) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .play(&self.host)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_pause(&self) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .pause(&self.host)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_step(
        &self,
        direction: PreviewFrameStepView,
    ) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .step(
                &self.host,
                match direction {
                    PreviewFrameStepView::Previous => or_runtime::PreviewFrameStep::Previous,
                    PreviewFrameStepView::Next => or_runtime::PreviewFrameStep::Next,
                },
            )
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_tick(&self) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .tick(&self.host)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_set_speed(
        &self,
        speed: f64,
    ) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        let rate = match speed {
            0.25 => RationalRate::new(1, 4),
            0.5 => RationalRate::new(1, 2),
            0.75 => RationalRate::new(3, 4),
            1.0 => RationalRate::new(1, 1),
            1.5 => RationalRate::new(3, 2),
            2.0 => RationalRate::new(2, 1),
            _ => {
                return Err(ProjectBridgeError {
                    code: "INVALID_PREVIEW_SPEED".to_owned(),
                    message: "preview speed must be 0.25, 0.5, 0.75, 1, 1.5 or 2".to_owned(),
                });
            }
        }
        .map_err(|error| ProjectBridgeError {
            code: "INVALID_PREVIEW_SPEED".to_owned(),
            message: error.to_string(),
        })?;
        self.preview_runtime
            .set_speed(&self.host, rate)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_prepare(
        &self,
        action: PreviewPreparationActionView,
        position: Option<RationalTimeView>,
    ) -> Result<ProjectPreviewPreparationView, ProjectBridgeError> {
        let action = match action {
            PreviewPreparationActionView::Seek => {
                let position = position.ok_or_else(|| ProjectBridgeError {
                    code: "INVALID_PREVIEW_TIME".to_owned(),
                    message: "Seek requires a preview position.".to_owned(),
                })?;
                PreviewPreparationAction::Seek(
                    RationalTime::new(position.numerator, position.denominator).map_err(
                        |error| ProjectBridgeError {
                            code: "INVALID_PREVIEW_TIME".to_owned(),
                            message: error.to_string(),
                        },
                    )?,
                )
            }
            PreviewPreparationActionView::Play => PreviewPreparationAction::Play,
            PreviewPreparationActionView::StepPrevious => {
                PreviewPreparationAction::Step(or_runtime::PreviewFrameStep::Previous)
            }
            PreviewPreparationActionView::StepNext => {
                PreviewPreparationAction::Step(or_runtime::PreviewFrameStep::Next)
            }
            PreviewPreparationActionView::Tick => PreviewPreparationAction::Tick,
        };
        self.preview_runtime
            .prepare(&self.host, action)
            .map(|prepared| ProjectPreviewPreparationView {
                request_id: prepared.request_id,
                sources: prepared.sources,
                state: preview_state_view(prepared.snapshot),
            })
            .map_err(preview_bridge_error)
    }

    pub fn preview_complete_prepared(
        &self,
        request_id: u64,
    ) -> Result<ProjectPreviewStateView, ProjectBridgeError> {
        self.preview_runtime
            .complete_prepared(&self.host, request_id)
            .map(preview_state_view)
            .map_err(preview_bridge_error)
    }

    pub fn preview_abort_prepared(&self, request_id: u64) -> Result<(), ProjectBridgeError> {
        self.preview_runtime
            .abort_prepared(request_id)
            .map_err(preview_bridge_error)
    }
}

fn preview_state_view(snapshot: PreviewSnapshot) -> ProjectPreviewStateView {
    ProjectPreviewStateView {
        position: rational_time_view(snapshot.playback.position),
        presented_time: snapshot.playback.presented_time.map(rational_time_view),
        sequence_frame_rate: snapshot.playback.frame_rate.map(rational_rate_view),
        content_end: snapshot.playback.content_end.map(rational_time_view),
        playing: snapshot.playback.playing,
        generation: snapshot.playback.generation,
        frame_sequence: snapshot.playback.frame_sequence,
        width: snapshot.width,
        height: snapshot.height,
        error_code: snapshot.error_code,
        error_message: snapshot.error_message,
    }
}

fn preview_bridge_error(error: PreviewError) -> ProjectBridgeError {
    ProjectBridgeError {
        code: error.code,
        message: error.message,
    }
}
