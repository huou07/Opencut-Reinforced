//! Media bridge methods: one-shot imports, relinking and removal, kept off the
//! timeline command surface.

use std::str::FromStr;

use or_core::{CommandEnvelope, MediaId, MediaItem};

use super::project::{
    ProjectBridgeError, ProjectHostHandle, action_error, parse_session_identity,
    prepare_import_source,
};
use super::views::{MediaCleanupResultView, ProjectActionResult};

impl ProjectHostHandle {
    pub fn import_media(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        path: String,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => return action_error(error),
            };
        let item = match prepare_import_source(&path) {
            Ok(item) => item,
            Err(error) => {
                let mut message = error.to_string();
                // A missing `moov` is the signature of a recording that is still
                // being written or a copy that was interrupted, not of a codec
                // the editor refuses. Say what to do instead of repeating the
                // backend's English reason alone.
                if let or_core::MediaImportError::Probe(probe) = &error
                    && probe
                        .diagnostic()
                        .is_some_and(|detail| detail.contains("moov atom not found"))
                {
                    message.push_str(
                        " The file may still be recording or was not fully copied; \
                         finish writing it and import again.",
                    );
                }
                return ProjectActionResult {
                    succeeded: false,
                    error_code: error.code_str().to_owned(),
                    message,
                    view: None,
                };
            }
        };
        self.dispatch_command(CommandEnvelope::add_media(
            project_id,
            project_instance_id,
            revision,
            item,
        ))
    }

    /// Reduces noise in one media file and writes the result to `destination`.
    ///
    /// This is a file-to-file media operation, not a project mutation: no project is
    /// opened, nothing in one changes, and the result is ordinary media the caller
    /// can import and compare. The noise floor is derived from the material when
    /// `noise_floor_db` is `None`, because a constant would leave the noise in one
    /// recording and eat the audio in another.
    pub fn clean_up_audio(
        &self,
        source: String,
        destination: String,
        noise_floor_db: Option<i32>,
        reduction_db: u32,
    ) -> MediaCleanupResultView {
        if reduction_db == 0 {
            return MediaCleanupResultView::failure(
                "INVALID_ARGUMENTS",
                "reduction cannot be zero",
            );
        }
        let source_path = std::path::PathBuf::from(&source);
        let destination_path = std::path::PathBuf::from(&destination);
        let noise_floor_db = match noise_floor_db {
            Some(value) => value,
            None => match or_core::measure_noise_floor(&source_path) {
                Ok(value) => value,
                Err(error) => {
                    return MediaCleanupResultView::failure(
                        error.code().as_str(),
                        format!(
                            "could not measure the noise floor: {}",
                            error.code().as_str()
                        ),
                    );
                }
            },
        };
        match or_core::clean_audio_file(
            &source_path,
            &destination_path,
            noise_floor_db,
            reduction_db,
        ) {
            Ok(result) => MediaCleanupResultView {
                succeeded: true,
                error_code: String::new(),
                message: String::new(),
                destination: result.destination.display().to_string(),
                noise_floor_db: result.noise_floor_db,
                reduction_db: result.reduction_db,
                bytes_written: result.bytes_written,
            },
            Err(error) => MediaCleanupResultView::failure(
                error.code().as_str(),
                format!("noise reduction failed: {}", error.code().as_str()),
            ),
        }
    }

    /// Re-probes a replacement source without changing the target media identity.
    pub fn relink_media(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        media_id: String,
        path: String,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => return action_error(error),
            };
        let media_id = match MediaId::from_str(&media_id) {
            Ok(media_id) => media_id,
            Err(error) => {
                return action_error(ProjectBridgeError {
                    code: "INVALID_MEDIA_ID".to_owned(),
                    message: error.to_string(),
                });
            }
        };
        let prepared = match prepare_import_source(&path) {
            Ok(item) => item,
            Err(error) => {
                return ProjectActionResult {
                    succeeded: false,
                    error_code: error.code_str().to_owned(),
                    message: error.to_string(),
                    view: None,
                };
            }
        };
        let item = match MediaItem::new(
            media_id,
            prepared.source().clone(),
            prepared.metadata().clone(),
        ) {
            Ok(item) => item,
            Err(error) => {
                return action_error(ProjectBridgeError {
                    code: "INVALID_MEDIA_METADATA".to_owned(),
                    message: error.to_string(),
                });
            }
        };
        self.dispatch_command(CommandEnvelope::relink_media(
            project_id,
            project_instance_id,
            revision,
            item,
        ))
    }

    pub fn remove_media(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        media_id: String,
    ) -> ProjectActionResult {
        let (project_id, project_instance_id, revision) =
            match parse_session_identity(&project_id, &project_instance_id, expected_revision) {
                Ok(identity) => identity,
                Err(error) => return action_error(error),
            };
        let media_id = match MediaId::from_str(&media_id) {
            Ok(media_id) => media_id,
            Err(error) => {
                return action_error(ProjectBridgeError {
                    code: "INVALID_MEDIA_ID".to_owned(),
                    message: error.to_string(),
                });
            }
        };
        self.dispatch_command(CommandEnvelope::remove_media(
            project_id,
            project_instance_id,
            revision,
            media_id,
        ))
    }
}
