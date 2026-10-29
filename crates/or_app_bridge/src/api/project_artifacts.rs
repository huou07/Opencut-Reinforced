//! Media artifact bridge methods: thumbnail/waveform requests, cached reads and
//! the event stream, kept out of the project command surface.

use std::str::FromStr;
use std::sync::mpsc;
use std::thread;

use crate::frb_generated::StreamSink;
use or_core::{
    ApplicationRequest, ApplicationResponse, CacheKey, MediaArtifactEvent,
    MediaArtifactService, MediaId, QueryEnvelope,
};

use super::project::{
    cache_artifact_kind, cache_unavailable_error, forward_media_artifact_events, host_error,
    operation_bridge_error, unexpected_response_error, ProjectBridgeError, ProjectHostHandle,
};
use super::views::{
    MediaArtifactBytesView, MediaArtifactEventView, MediaArtifactKindView,
    MediaArtifactRequestView,
};

impl ProjectHostHandle {
    pub fn request_media_thumbnail(&self, media_id: String) -> MediaArtifactRequestView {
        self.request_media_artifact(media_id, MediaArtifactKindView::Thumbnail)
    }

    pub fn request_media_waveform(&self, media_id: String) -> MediaArtifactRequestView {
        self.request_media_artifact(media_id, MediaArtifactKindView::Waveform)
    }

    pub fn read_media_artifact(
        &self,
        kind: MediaArtifactKindView,
        cache_key: String,
    ) -> Result<Option<MediaArtifactBytesView>, ProjectBridgeError> {
        let cache_key = CacheKey::from_hex(&cache_key).map_err(|error| ProjectBridgeError {
            code: "INVALID_CACHE_KEY".to_owned(),
            message: error.to_string(),
        })?;
        let service = self
            .media_artifact_service
            .as_ref()
            .ok_or_else(cache_unavailable_error)?;
        let bytes = service
            .read_artifact(cache_artifact_kind(kind), cache_key)
            .map_err(|error| ProjectBridgeError {
                code: error.code().to_owned(),
                message: error.to_string(),
            })?;
        Ok(bytes.map(|bytes| MediaArtifactBytesView {
            bytes,
            mime_type: "image/png".to_owned(),
        }))
    }

    pub fn subscribe_media_artifact_events(
        &self,
        sink: StreamSink<MediaArtifactEventView>,
    ) -> Result<(), ProjectBridgeError> {
        // A platform without cached artifacts has no events to deliver, so the
        // subscription is empty rather than failed. The generated stream wrapper
        // discards this call's future, so an error here would surface as an
        // unobserved bridge error instead of anything the caller could handle.
        let Some(receiver) = self.media_artifact_event_receiver() else {
            return Ok(());
        };
        thread::Builder::new()
            .name("or-flutter-media-artifact-events".to_owned())
            .spawn(move || forward_media_artifact_events(receiver, sink))
            .map_err(|error| ProjectBridgeError {
                code: "EVENT_SUBSCRIPTION_FAILED".to_owned(),
                message: error.to_string(),
            })?;
        Ok(())
    }

    pub(crate) fn media_artifact_event_receiver(&self) -> Option<mpsc::Receiver<MediaArtifactEvent>> {
        self.media_artifact_service
            .as_ref()
            .map(MediaArtifactService::subscribe_events)
    }

    fn request_media_artifact(
        &self,
        media_id: String,
        kind: MediaArtifactKindView,
    ) -> MediaArtifactRequestView {
        let id = match MediaId::from_str(&media_id) {
            Ok(id) => id,
            Err(error) => {
                return MediaArtifactRequestView::failed(
                    media_id,
                    kind,
                    "INVALID_MEDIA_ID",
                    error.to_string(),
                );
            }
        };
        let summary = match self.host.describe() {
            Ok(summary) => summary,
            Err(error) => {
                let error = host_error(error);
                return MediaArtifactRequestView::failed(media_id, kind, error.code, error.message);
            }
        };
        let request = QueryEnvelope::media_get(
            summary.summary.project_id,
            summary.summary.project_instance_id,
            id,
        );
        let item = match self
            .host
            .handle_application_request(ApplicationRequest::Query(request))
        {
            Ok(ApplicationResponse::Query(result)) => match result.media_item {
                Some(item) => *item,
                None => {
                    let error = unexpected_response_error();
                    return MediaArtifactRequestView::failed(
                        media_id,
                        kind,
                        error.code,
                        error.message,
                    );
                }
            },
            Ok(ApplicationResponse::Error(error)) => {
                let error = operation_bridge_error(error);
                return MediaArtifactRequestView::failed(media_id, kind, error.code, error.message);
            }
            Ok(_) => {
                let error = unexpected_response_error();
                return MediaArtifactRequestView::failed(media_id, kind, error.code, error.message);
            }
            Err(error) => {
                let error = host_error(error);
                return MediaArtifactRequestView::failed(media_id, kind, error.code, error.message);
            }
        };
        let Some(service) = self.media_artifact_service.as_ref() else {
            let error = cache_unavailable_error();
            return MediaArtifactRequestView::failed(media_id, kind, error.code, error.message);
        };
        let result = match kind {
            MediaArtifactKindView::Thumbnail => service.request_thumbnail(&item),
            MediaArtifactKindView::Waveform => service.request_waveform(&item),
        };
        match result {
            Ok(result) => MediaArtifactRequestView::from_request(media_id, kind, result),
            Err(error) => {
                MediaArtifactRequestView::failed(media_id, kind, error.code(), error.to_string())
            }
        }
    }
}
