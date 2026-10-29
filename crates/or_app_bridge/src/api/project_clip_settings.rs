//! Timeline clip settings: visual, audio, playback speed, reverse and freeze.

use super::project::*;
use or_core::{
    AudioSettings, ClipContent, ClipSettings, CommandEnvelope, Crop, Opacity, ProjectId,
    ProjectInstanceId, ProjectRevision, RationalRate, TimelineClipState, TrackKind, Transform,
};

impl ProjectHostHandle {
    pub fn get_timeline_clip_visual_settings(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
    ) -> Result<ProjectTimelineVisualSettingsView, ProjectBridgeError> {
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
        Ok(visual_settings_view(&settings))
    }

    pub fn update_timeline_clip_visual_settings(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        settings: ProjectTimelineVisualSettingsView,
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
        visual.transform = Transform {
            x_milli_canvas: settings.x_milli_canvas,
            y_milli_canvas: settings.y_milli_canvas,
            scale_x_milli: settings.scale_x_milli,
            scale_y_milli: settings.scale_y_milli,
            rotation_milli_degrees: settings.rotation_milli_degrees,
            anchor_x_basis_points: settings.anchor_x_basis_points,
            anchor_y_basis_points: settings.anchor_y_basis_points,
        };
        visual.crop = Crop {
            left_basis_points: settings.crop_left_basis_points,
            top_basis_points: settings.crop_top_basis_points,
            right_basis_points: settings.crop_right_basis_points,
            bottom_basis_points: settings.crop_bottom_basis_points,
        };
        visual.opacity = Opacity {
            basis_points: settings.opacity_basis_points,
        };
        visual.effects = effects_from_view(&visual.effects, &settings);
        if settings.update_transition_in {
            visual.transition_in = match transition_from_view(
                settings.transition_in_kind,
                settings.transition_in_duration,
            ) {
                Ok(transition) => transition,
                Err(error) => return action_error(error),
            };
        }
        if settings.update_transition_out {
            visual.transition_out = match transition_from_view(
                settings.transition_out_kind,
                settings.transition_out_duration,
            ) {
                Ok(transition) => transition,
                Err(error) => return action_error(error),
            };
        }
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

    pub fn get_timeline_clip_audio_settings(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
    ) -> Result<ProjectTimelineAudioSettingsView, ProjectBridgeError> {
        let (_, _, _, clip) = self.find_audio_clip(
            &project_id,
            &project_instance_id,
            expected_revision,
            &track_id,
            &clip_id,
        )?;
        let ClipSettings::Audio(settings) = clip.settings else {
            return Err(unsupported_audio_settings());
        };
        Ok(audio_settings_view(settings))
    }

    pub fn update_timeline_clip_audio_settings(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        settings: ProjectTimelineAudioSettingsView,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision, clip) = match self.find_audio_clip(
            &project_id,
            &project_instance_id,
            expected_revision,
            &track_id,
            &clip_id,
        ) {
            Ok(result) => result,
            Err(error) => return action_error(error),
        };
        let fade_in = match time_from_view(settings.fade_in) {
            Ok(time) => time,
            Err(error) => return action_error(error),
        };
        let fade_out = match time_from_view(settings.fade_out) {
            Ok(time) => time,
            Err(error) => return action_error(error),
        };
        self.dispatch_command(CommandEnvelope::update_timeline_clip(
            project_id,
            project_instance_id,
            revision,
            clip.clip_id,
            clip.timeline_duration,
            clip.content,
            ClipSettings::Audio(AudioSettings {
                gain_millidecibels: settings.gain_millidecibels,
                pan_basis_points: settings.pan_basis_points,
                fade_in,
                fade_out,
            }),
        ))
    }

    pub fn update_timeline_clip_playback_rate(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        rate: RationalRateView,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision, track_kind, clip) = match self
            .find_timeline_clip_state(
                &project_id,
                &project_instance_id,
                expected_revision,
                &track_id,
                &clip_id,
            ) {
            Ok(result) => result,
            Err(error) => return action_error(error),
        };
        if !matches!(track_kind, TrackKind::Video | TrackKind::Audio) {
            return action_error(ProjectBridgeError {
                code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                message: "Playback speed is available for video and audio media clips.".to_owned(),
            });
        }
        let rate = match RationalRate::new(rate.numerator, rate.denominator) {
            Ok(rate) => rate,
            Err(error) => {
                return action_error(ProjectBridgeError {
                    code: "INVALID_PLAYBACK_RATE".to_owned(),
                    message: format!("The playback rate is invalid: {error}"),
                });
            }
        };
        let source_range = match &clip.content {
            ClipContent::Media { source_range, .. } => *source_range,
            ClipContent::Text { .. } | ClipContent::Caption { .. } => {
                return action_error(ProjectBridgeError {
                    code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                    message: "Playback speed is available for video and audio media clips."
                        .to_owned(),
                });
            }
        };
        let timeline_duration = match RationalRate::new(rate.denominator(), rate.numerator())
            .and_then(|inverse_rate| inverse_rate.apply(source_range.duration()))
        {
            Ok(duration) => duration,
            Err(error) => {
                return action_error(ProjectBridgeError {
                    code: "INVALID_PLAYBACK_RATE".to_owned(),
                    message: format!("The playback rate cannot be represented exactly: {error}"),
                });
            }
        };
        let mut content = clip.content;
        if let ClipContent::Media { playback_rate, .. } = &mut content {
            *playback_rate = rate;
        }
        self.dispatch_command(CommandEnvelope::update_timeline_clip(
            project_id,
            project_instance_id,
            revision,
            clip.clip_id,
            timeline_duration,
            content,
            clip.settings,
        ))
    }

    /// Flip the direction a media clip plays without touching its duration.
    ///
    /// Reverse is not a rate: the clip still covers the same source span and the same
    /// timeline span, so only the direction changes and the caller keeps every other
    /// property of the clip.
    pub fn set_timeline_clip_reversed(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        reversed: bool,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision, track_kind, clip) = match self
            .find_timeline_clip_state(
                &project_id,
                &project_instance_id,
                expected_revision,
                &track_id,
                &clip_id,
            ) {
            Ok(result) => result,
            Err(error) => return action_error(error),
        };
        if !matches!(track_kind, TrackKind::Video | TrackKind::Audio) {
            return action_error(ProjectBridgeError {
                code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                message: "Reverse is available for video and audio media clips.".to_owned(),
            });
        }
        let mut content = clip.content;
        match &mut content {
            ClipContent::Media {
                reversed: current, ..
            } => *current = reversed,
            ClipContent::Text { .. } | ClipContent::Caption { .. } => {
                return action_error(ProjectBridgeError {
                    code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                    message: "Reverse is available for video and audio media clips.".to_owned(),
                });
            }
        }
        self.dispatch_command(CommandEnvelope::update_timeline_clip(
            project_id,
            project_instance_id,
            revision,
            clip.clip_id,
            clip.timeline_duration,
            content,
            clip.settings,
        ))
    }

    /// Holds one frame for the clip's whole duration, or releases it.
    ///
    /// The source range is left untouched, so unfreezing restores the original
    /// footage instead of a shortened clip.
    pub fn set_timeline_clip_frozen(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        track_id: String,
        clip_id: String,
        frozen: bool,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision, track_kind, clip) = match self
            .find_timeline_clip_state(
                &project_id,
                &project_instance_id,
                expected_revision,
                &track_id,
                &clip_id,
            ) {
            Ok(result) => result,
            Err(error) => return action_error(error),
        };
        if !matches!(track_kind, TrackKind::Video) {
            return action_error(ProjectBridgeError {
                code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                message: "Freeze is available for video clips.".to_owned(),
            });
        }
        let mut content = clip.content;
        match &mut content {
            ClipContent::Media {
                frozen: current, ..
            } => *current = frozen,
            ClipContent::Text { .. } | ClipContent::Caption { .. } => {
                return action_error(ProjectBridgeError {
                    code: "UNSUPPORTED_CLIP_CONTENT".to_owned(),
                    message: "Freeze is available for video clips.".to_owned(),
                });
            }
        }
        self.dispatch_command(CommandEnvelope::update_timeline_clip(
            project_id,
            project_instance_id,
            revision,
            clip.clip_id,
            clip.timeline_duration,
            content,
            clip.settings,
        ))
    }

    fn find_visual_clip(
        &self,
        project_id: &str,
        project_instance_id: &str,
        expected_revision: u64,
        track_id: &str,
        clip_id: &str,
    ) -> Result<
        (
            ProjectId,
            ProjectInstanceId,
            ProjectRevision,
            TimelineClipState,
        ),
        ProjectBridgeError,
    > {
        let (project_id, project_instance_id, revision, track_kind, clip) = self
            .find_timeline_clip_state(
                project_id,
                project_instance_id,
                expected_revision,
                track_id,
                clip_id,
            )?;
        if !track_kind.is_visual() {
            return Err(unsupported_visual_settings());
        }
        Ok((project_id, project_instance_id, revision, clip))
    }

    fn find_audio_clip(
        &self,
        project_id: &str,
        project_instance_id: &str,
        expected_revision: u64,
        track_id: &str,
        clip_id: &str,
    ) -> Result<
        (
            ProjectId,
            ProjectInstanceId,
            ProjectRevision,
            TimelineClipState,
        ),
        ProjectBridgeError,
    > {
        let (project_id, project_instance_id, revision, track_kind, clip) = self
            .find_timeline_clip_state(
                project_id,
                project_instance_id,
                expected_revision,
                track_id,
                clip_id,
            )?;
        if track_kind != TrackKind::Audio {
            return Err(unsupported_audio_settings());
        }
        Ok((project_id, project_instance_id, revision, clip))
    }
}
