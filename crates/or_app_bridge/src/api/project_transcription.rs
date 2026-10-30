//! Transcription bridge methods: model selection, download lifecycle, job
//! start/status and transcript proposals, kept off the timeline command surface.

use crate::transcription::{TranscriptionModelInfo, TranscriptionService};
use std::str::FromStr;

use or_core::{ApplicationRequest, ApplicationResponse, JobId, MediaId, QueryEnvelope};

use super::project::{
    ProjectBridgeError, ProjectHostHandle, host_error, job_state_name, operation_bridge_error,
    parse_session_identity, revision_conflict_error, unexpected_response_error,
};
use super::views::{
    ProjectModelDownloadView, ProjectTranscriptProposalView, ProjectTranscriptionJobView,
    ProjectTranscriptionModelOptionView, ProjectTranscriptionModelView, model_download_view,
    transcription_error, transcription_job_view, transcription_model_option_view,
    transcription_model_view, transcription_proposal_view,
};

impl ProjectHostHandle {
    pub fn transcription_model(&self) -> ProjectTranscriptionModelView {
        let info = self
            .transcription_service
            .as_deref()
            .map(TranscriptionService::model_info)
            .unwrap_or_else(TranscriptionModelInfo::unavailable);
        transcription_model_view(info)
    }

    pub fn transcription_models(&self) -> Vec<ProjectTranscriptionModelOptionView> {
        self.transcription_service
            .as_deref()
            .map(TranscriptionService::models)
            .unwrap_or_default()
            .into_iter()
            .map(transcription_model_option_view)
            .collect()
    }

    pub fn select_transcription_model(
        &self,
        model_id: String,
    ) -> Result<ProjectTranscriptionModelView, ProjectBridgeError> {
        let info = self
            .transcription_service()?
            .select_model(&model_id)
            .map_err(transcription_error)?;
        Ok(transcription_model_view(info))
    }

    pub fn remove_transcription_model(
        &self,
        model_id: String,
    ) -> Result<ProjectTranscriptionModelView, ProjectBridgeError> {
        let info = self
            .transcription_service()?
            .remove_model(&model_id)
            .map_err(transcription_error)?;
        Ok(transcription_model_view(info))
    }

    pub fn begin_transcription_model_download(
        &self,
        model_id: String,
    ) -> Result<ProjectModelDownloadView, ProjectBridgeError> {
        let download = self
            .transcription_service()?
            .begin_model_download(&model_id)
            .map_err(transcription_error)?;
        Ok(model_download_view(download))
    }

    pub fn finish_transcription_model_download(
        &self,
        download_id: String,
    ) -> Result<ProjectTranscriptionModelView, ProjectBridgeError> {
        let info = self
            .transcription_service()?
            .finish_model_download(&download_id)
            .map_err(transcription_error)?;
        Ok(transcription_model_view(info))
    }

    pub fn cancel_transcription_model_download(
        &self,
        download_id: String,
    ) -> Result<(), ProjectBridgeError> {
        self.transcription_service()?
            .cancel_model_download(&download_id)
            .map_err(transcription_error)
    }

    pub fn start_transcription(
        &self,
        project_id: String,
        project_instance_id: String,
        expected_revision: u64,
        media_id: String,
    ) -> Result<ProjectTranscriptionJobView, ProjectBridgeError> {
        let (project_id, project_instance_id, revision) =
            parse_session_identity(&project_id, &project_instance_id, expected_revision)?;
        let id = MediaId::from_str(&media_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_MEDIA_ID".to_owned(),
            message: error.to_string(),
        })?;
        let summary = self.host.describe().map_err(host_error)?;
        if summary.summary.project_id != project_id
            || summary.summary.project_instance_id != project_instance_id
            || summary.summary.project_revision != revision
        {
            return Err(revision_conflict_error());
        }
        let request = QueryEnvelope::media_get(project_id, project_instance_id, id);
        let media = match self
            .host
            .handle_application_request(ApplicationRequest::Query(request))
            .map_err(host_error)?
        {
            ApplicationResponse::Query(result) => result
                .media_item
                .map(|item| *item)
                .ok_or_else(unexpected_response_error)?,
            ApplicationResponse::Error(error) => return Err(operation_bridge_error(error)),
            _ => return Err(unexpected_response_error()),
        };
        let job = self
            .transcription_service()?
            .start(
                uuid::Uuid::parse_str(&project_id.to_string()).map_err(|error| {
                    ProjectBridgeError {
                        code: "INVALID_PROJECT_ID".to_owned(),
                        message: error.to_string(),
                    }
                })?,
                revision.value(),
                media,
            )
            .map_err(transcription_error)?;
        Ok(transcription_job_view(job))
    }

    pub fn transcription_job(
        &self,
        job_id: String,
    ) -> Result<ProjectTranscriptionJobView, ProjectBridgeError> {
        let id = JobId::from_str(&job_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_JOB_ID".to_owned(),
            message: error.to_string(),
        })?;
        let job = self
            .transcription_service()?
            .snapshot(id)
            .ok_or_else(|| ProjectBridgeError {
                code: "JOB_NOT_FOUND".to_owned(),
                message: "The transcription job is no longer available.".to_owned(),
            })?;
        Ok(transcription_job_view(job))
    }

    pub fn transcription_proposal(
        &self,
        job_id: String,
    ) -> Result<Option<ProjectTranscriptProposalView>, ProjectBridgeError> {
        let id = JobId::from_str(&job_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_JOB_ID".to_owned(),
            message: error.to_string(),
        })?;
        Ok(self
            .transcription_service()?
            .proposal(id)
            .map(transcription_proposal_view))
    }

    pub fn clear_transcription_proposal(&self, job_id: String) -> Result<(), ProjectBridgeError> {
        let id = JobId::from_str(&job_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_JOB_ID".to_owned(),
            message: error.to_string(),
        })?;
        self.transcription_service()?.clear_proposal(id);
        Ok(())
    }

    pub fn cancel_transcription(&self, job_id: String) -> Result<String, ProjectBridgeError> {
        let id = JobId::from_str(&job_id).map_err(|error| ProjectBridgeError {
            code: "INVALID_JOB_ID".to_owned(),
            message: error.to_string(),
        })?;
        let outcome = self
            .transcription_service()?
            .cancel(id)
            .map_err(|_| ProjectBridgeError {
                code: "JOB_NOT_FOUND".to_owned(),
                message: "The transcription job is no longer available.".to_owned(),
            })?;
        Ok(match outcome {
            or_core::JobCancelOutcome::CancelledQueued => "cancelled".to_owned(),
            or_core::JobCancelOutcome::CancellationRequested => "cancelling".to_owned(),
            or_core::JobCancelOutcome::AlreadyTerminal(state) => job_state_name(state).to_owned(),
        })
    }

    fn transcription_service(&self) -> Result<&TranscriptionService, ProjectBridgeError> {
        self.transcription_service.as_deref().ok_or_else(|| {
            transcription_error(
                crate::transcription::TranscriptionServiceError::ProviderUnavailable,
            )
        })
    }
}
