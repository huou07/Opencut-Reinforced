//! Timeline clip animation: keyframes for the transform channels and opacity.
//!
//! The view carries every channel in one value so the inspector can add,
//! remove and re-ease keyframes with a single update, which keeps one user
//! action to one undo step.

use super::project::*;
use or_core::{
    ClipAnimation, ClipSettings, CommandEnvelope, Keyframe, KeyframeEasing, KeyframeTrack,
};

/// One control point, with its time measured from the clip start.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectKeyframeView {
    pub at: RationalTimeView,
    pub value: i32,
    /// `linear`, `ease_in`, `ease_out`, `ease_in_out`, `ease_out_back` or
    /// `ease_out_bounce`.
    pub easing: String,
}

/// Every animated channel a clip has.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectClipAnimationView {
    pub opacity_basis_points: Vec<ProjectKeyframeView>,
    pub x_milli_canvas: Vec<ProjectKeyframeView>,
    pub y_milli_canvas: Vec<ProjectKeyframeView>,
    pub scale_x_milli: Vec<ProjectKeyframeView>,
    pub scale_y_milli: Vec<ProjectKeyframeView>,
    pub rotation_milli_degrees: Vec<ProjectKeyframeView>,
}

impl ProjectHostHandle {
    pub fn get_timeline_clip_animation(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
    ) -> Result<ProjectClipAnimationView, ProjectBridgeError> {
        let (_, _, _, clip) = self.find_visual_clip(
            &project_id,
            &project_instance_id,
            expected_revision,
            &track_id,
            &clip_id,
        )?;
        let ClipSettings::Visual(settings) = clip.settings else {
            return Err(unsupported_visual_settings());
        };
        Ok(animation_view(&settings.animation))
    }

    pub fn update_timeline_clip_animation(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        animation: ProjectClipAnimationView,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision, clip) = match self.find_visual_clip(
            &project_id,
            &project_instance_id,
            expected_revision,
            &track_id,
            &clip_id,
        ) {
            Ok(result) => result,
            Err(error) => return action_error(error),
        };
        let ClipSettings::Visual(mut visual) = clip.settings else {
            return action_error(unsupported_visual_settings());
        };
        visual.animation = match animation_from_view(animation) {
            Ok(animation) => animation,
            Err(error) => return action_error(error),
        };
        self.dispatch_command(CommandEnvelope::update_timeline_clip(
            project_id,
            project_instance_id,
            revision,
            clip.clip_id,
            clip.timeline_duration,
            clip.content,
            ClipSettings::Visual(visual),
        ))
    }
}

fn animation_view(animation: &ClipAnimation) -> ProjectClipAnimationView {
    ProjectClipAnimationView {
        opacity_basis_points: keyframes_view(&animation.opacity_basis_points),
        x_milli_canvas: keyframes_view(&animation.x_milli_canvas),
        y_milli_canvas: keyframes_view(&animation.y_milli_canvas),
        scale_x_milli: keyframes_view(&animation.scale_x_milli),
        scale_y_milli: keyframes_view(&animation.scale_y_milli),
        rotation_milli_degrees: keyframes_view(&animation.rotation_milli_degrees),
    }
}

fn keyframes_view(track: &KeyframeTrack) -> Vec<ProjectKeyframeView> {
    track
        .keyframes
        .iter()
        .map(|keyframe| ProjectKeyframeView {
            at: rational_time_view(keyframe.at),
            value: keyframe.value,
            easing: keyframe.easing.wire_name().to_owned(),
        })
        .collect()
}

fn animation_from_view(
    view: ProjectClipAnimationView,
) -> Result<ClipAnimation, ProjectBridgeError> {
    Ok(ClipAnimation {
        opacity_basis_points: keyframes_from_view(view.opacity_basis_points)?,
        x_milli_canvas: keyframes_from_view(view.x_milli_canvas)?,
        y_milli_canvas: keyframes_from_view(view.y_milli_canvas)?,
        scale_x_milli: keyframes_from_view(view.scale_x_milli)?,
        scale_y_milli: keyframes_from_view(view.scale_y_milli)?,
        rotation_milli_degrees: keyframes_from_view(view.rotation_milli_degrees)?,
    })
}

fn keyframes_from_view(
    keyframes: Vec<ProjectKeyframeView>,
) -> Result<KeyframeTrack, ProjectBridgeError> {
    let mut keyframes = keyframes
        .into_iter()
        .map(|keyframe| {
            let easing =
                KeyframeEasing::parse(&keyframe.easing).ok_or_else(|| ProjectBridgeError {
                    code: "INVALID_KEYFRAME_EASING".to_owned(),
                    message: "keyframe easing must be linear, ease_in, ease_out, \
                              ease_in_out, ease_out_back or ease_out_bounce"
                        .to_owned(),
                })?;
            Ok(Keyframe {
                at: time_from_view(keyframe.at)?,
                value: keyframe.value,
                easing,
            })
        })
        .collect::<Result<Vec<_>, ProjectBridgeError>>()?;
    // Adding a keyframe at the playhead usually appends, but dragging one back
    // can put it before its neighbours. Ordering here keeps the canonical file
    // sorted without making the UI repeat the rule.
    keyframes.sort_by_key(|keyframe| keyframe.at);
    Ok(KeyframeTrack { keyframes })
}
