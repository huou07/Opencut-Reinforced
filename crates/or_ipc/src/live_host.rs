use crate::{IpcProtocolError, LocalIpcServer};
use or_core::{
    ApplicationRequest, ApplicationResponse, ExportRequest, ExportResponse, ProjectFileSession,
    ProjectFileSessionError, ProjectRevision, ProjectSession, QueryEnvelope, QueryResult,
};

/// Runtime extension for application operations that need desktop media services.
pub trait ExportRequestHandler: Send + Sync {
    fn handle_export_request(
        &self,
        session: &ProjectSession,
        request: ExportRequest,
    ) -> ExportResponse;
}
use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, mpsc},
};

/// An ordered invalidation emitted by the single live Rust project session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectHostEvent {
    pub sequence: u64,
    pub kind: ProjectHostEventKind,
    pub project_id: String,
    pub project_instance_id: String,
    pub project_revision: u64,
    pub dirty: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectHostEventKind {
    ProjectChanged,
    ProjectSaved,
    SessionClosing,
}

/// Failures from direct access to a live project host.
#[derive(Debug)]
pub enum LiveProjectHostError {
    Ipc(IpcProtocolError),
    Session(ProjectFileSessionError),
    Operation(or_core::OperationError),
    UnexpectedResponse,
    LockPoisoned,
    SessionClosing,
    UnsavedChanges,
}

impl fmt::Display for LiveProjectHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ipc(error) => fmt::Display::fmt(error, formatter),
            Self::Session(error) => fmt::Display::fmt(error, formatter),
            Self::Operation(error) => fmt::Display::fmt(error, formatter),
            Self::UnexpectedResponse => {
                formatter.write_str("project host returned an unexpected response")
            }
            Self::LockPoisoned => formatter.write_str("project host state is unavailable"),
            Self::SessionClosing => formatter.write_str("project host is closing"),
            Self::UnsavedChanges => formatter.write_str("project has unsaved changes"),
        }
    }
}

impl Error for LiveProjectHostError {}

impl From<IpcProtocolError> for LiveProjectHostError {
    fn from(error: IpcProtocolError) -> Self {
        Self::Ipc(error)
    }
}

impl From<ProjectFileSessionError> for LiveProjectHostError {
    fn from(error: ProjectFileSessionError) -> Self {
        Self::Session(error)
    }
}

/// One Rust-owned project session shared by in-process access and local IPC.
pub struct LiveProjectHost {
    shared: Arc<Mutex<LiveProjectHostState>>,
    server: Option<LocalIpcServer>,
}

impl LiveProjectHost {
    /// Starts authenticated local IPC around the exact session passed to the host.
    pub fn start(
        session: ProjectFileSession,
        descriptor_path: Option<&Path>,
    ) -> Result<Self, LiveProjectHostError> {
        Self::start_with_handler(session, descriptor_path, None)
    }

    /// Starts IPC with one shared export handler for direct and remote requests.
    pub fn start_with_export_handler(
        session: ProjectFileSession,
        descriptor_path: Option<&Path>,
        handler: Arc<dyn ExportRequestHandler>,
    ) -> Result<Self, LiveProjectHostError> {
        Self::start_with_handler(session, descriptor_path, Some(handler))
    }

    /// Creates the same Rust-owned host without starting a local IPC server.
    pub fn in_process_with_export_handler(
        session: ProjectFileSession,
        handler: Arc<dyn ExportRequestHandler>,
    ) -> Self {
        Self::in_process_with_handler(session, Some(handler))
    }

    fn start_with_handler(
        session: ProjectFileSession,
        descriptor_path: Option<&Path>,
        handler: Option<Arc<dyn ExportRequestHandler>>,
    ) -> Result<Self, LiveProjectHostError> {
        let mut host = Self::in_process_with_handler(session, handler);
        host.server = Some(LocalIpcServer::start_shared(
            Arc::clone(&host.shared),
            descriptor_path,
        )?);
        Ok(host)
    }

    fn in_process_with_handler(
        session: ProjectFileSession,
        handler: Option<Arc<dyn ExportRequestHandler>>,
    ) -> Self {
        Self {
            shared: shared_host_state(session, handler),
            server: None,
        }
    }

    pub fn describe(&self) -> Result<QueryResult, LiveProjectHostError> {
        let mut state = self.lock_open()?;
        let session = &mut state.session;
        let request = ApplicationRequest::Query(QueryEnvelope {
            query_id: "project.summary".to_owned(),
            schema_version: 1,
            project_id: session.session().project_id(),
            project_instance_id: session.session().project_instance_id(),
            arguments: serde_json::json!({}),
        });
        match session.handle_application_request(request) {
            ApplicationResponse::Query(result) => Ok(result),
            ApplicationResponse::Error(error) => Err(LiveProjectHostError::Operation(error)),
            _ => Err(LiveProjectHostError::UnexpectedResponse),
        }
    }

    pub fn handle_application_request(
        &self,
        request: ApplicationRequest,
    ) -> Result<ApplicationResponse, LiveProjectHostError> {
        let mut state = self.lock_open()?;
        let before = state.session.session().project_revision();
        let response = state.handle_application_request(request);
        let after = state.session.session().project_revision();
        if after != before {
            state.publish(ProjectHostEventKind::ProjectChanged);
        }
        Ok(response)
    }

    pub fn save(&self) -> Result<ProjectRevision, LiveProjectHostError> {
        let mut state = self.lock_open()?;
        state.session.save()?;
        let revision = state.session.session().project_revision();
        state.publish(ProjectHostEventKind::ProjectSaved);
        Ok(revision)
    }

    /// Periodically checkpoints dirty state to the recovery sidecar.
    pub fn autosave_checkpoint(&self) -> Result<bool, LiveProjectHostError> {
        self.lock_open()?
            .session
            .autosave_checkpoint()
            .map_err(Into::into)
    }

    pub fn is_dirty(&self) -> Result<bool, LiveProjectHostError> {
        Ok(self.lock_open()?.session.is_dirty())
    }

    pub fn descriptor_path(&self) -> Option<PathBuf> {
        self.server
            .as_ref()
            .map(|server| server.descriptor_path().to_path_buf())
    }

    pub fn subscribe_events(
        &self,
    ) -> Result<mpsc::Receiver<ProjectHostEvent>, LiveProjectHostError> {
        let mut state = self.lock_open()?;
        let (sender, receiver) = mpsc::channel();
        state.subscribers.push(sender);
        Ok(receiver)
    }

    /// Stops IPC and releases this host only after the caller permits any dirty discard.
    pub fn shutdown(&mut self, discard_unsaved: bool) -> Result<(), LiveProjectHostError> {
        self.shutdown_with_recovery_policy(discard_unsaved, false)
    }

    /// Stops IPC, optionally removing only this session's matching recovery snapshot.
    pub fn shutdown_with_recovery_policy(
        &mut self,
        discard_unsaved: bool,
        discard_recovery_checkpoint: bool,
    ) -> Result<(), LiveProjectHostError> {
        {
            let mut state = self
                .shared
                .lock()
                .map_err(|_| LiveProjectHostError::LockPoisoned)?;
            if state.session.is_dirty() && !discard_unsaved {
                return Err(LiveProjectHostError::UnsavedChanges);
            }
            if state.session.is_dirty() && discard_recovery_checkpoint {
                state.session.discard_unsaved_recovery_checkpoint()?;
            }
            if !state.closing {
                state.closing = true;
                state.publish(ProjectHostEventKind::SessionClosing);
            }
        }
        self.server.take();
        Ok(())
    }

    fn lock_open(&self) -> Result<MutexGuard<'_, LiveProjectHostState>, LiveProjectHostError> {
        let state = self
            .shared
            .lock()
            .map_err(|_| LiveProjectHostError::LockPoisoned)?;
        if state.closing {
            return Err(LiveProjectHostError::SessionClosing);
        }
        Ok(state)
    }
}

pub(crate) struct LiveProjectHostState {
    pub(crate) session: ProjectFileSession,
    export_handler: Option<Arc<dyn ExportRequestHandler>>,
    pub(crate) closing: bool,
    sequence: u64,
    subscribers: Vec<mpsc::Sender<ProjectHostEvent>>,
}

impl LiveProjectHostState {
    pub(crate) fn handle_application_request(
        &mut self,
        request: ApplicationRequest,
    ) -> ApplicationResponse {
        match request {
            ApplicationRequest::Export(export_request) => {
                if let Some(handler) = &self.export_handler {
                    ApplicationResponse::Export(
                        handler.handle_export_request(self.session.session(), export_request),
                    )
                } else {
                    ApplicationResponse::Export(ExportResponse::failure(
                        "EXPORT_RUNTIME_UNAVAILABLE",
                        "the project host has no export runtime",
                    ))
                }
            }
            request => self.session.handle_application_request(request),
        }
    }

    pub(crate) fn publish(&mut self, kind: ProjectHostEventKind) {
        let Some(sequence) = self.sequence.checked_add(1) else {
            return;
        };
        self.sequence = sequence;
        let session = &self.session;
        let summary = session.session();
        let event = ProjectHostEvent {
            sequence,
            kind,
            project_id: summary.project_id().to_string(),
            project_instance_id: summary.project_instance_id().to_string(),
            project_revision: summary.project_revision().value(),
            dirty: session.is_dirty(),
        };
        self.subscribers
            .retain(|subscriber| subscriber.send(event.clone()).is_ok());
    }
}

pub(crate) fn shared_host_state(
    session: ProjectFileSession,
    export_handler: Option<Arc<dyn ExportRequestHandler>>,
) -> Arc<Mutex<LiveProjectHostState>> {
    Arc::new(Mutex::new(LiveProjectHostState {
        session,
        export_handler,
        closing: false,
        sequence: 0,
        subscribers: Vec::new(),
    }))
}
