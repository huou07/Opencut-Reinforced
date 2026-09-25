use crate::live_host::{LiveProjectHostState, ProjectHostEventKind, shared_host_state};
use crate::protocol::{
    ApplicationSuccess, DescribeResponse, EndpointDescriptor, IpcCommandDescriptor, IpcErrorCode,
    IpcProtocolError, IpcQueryDescriptor, IpcRequest, IpcResponse, IpcResponseResult, IpcSuccess,
    IpcTransport, MAX_DESCRIPTOR_BYTES, OR_LOCAL_IPC_PROTOCOL_VERSION, SaveResponse, is_uuid_v4,
    read_frame, write_frame,
};
use or_core::{
    ApplicationResponse, ProjectFileSession, ProjectFileSessionErrorCode, command_catalog,
    query_catalog,
};
use serde::Deserialize;
#[cfg(not(windows))]
use std::fs::OpenOptions;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};
use uuid::Uuid;

/// One live local IPC host for one file-backed project session.
pub struct LocalIpcServer {
    descriptor: EndpointDescriptor,
    descriptor_path: PathBuf,
    stopping: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<(), IpcProtocolError>>>,
}

impl LocalIpcServer {
    /// Binds the platform-local transport and writes a new private descriptor.
    pub fn start(
        session: ProjectFileSession,
        descriptor_path: Option<&Path>,
    ) -> Result<Self, IpcProtocolError> {
        Self::start_shared(shared_host_state(session), descriptor_path)
    }

    pub(crate) fn start_shared(
        shared: Arc<Mutex<LiveProjectHostState>>,
        descriptor_path: Option<&Path>,
    ) -> Result<Self, IpcProtocolError> {
        let (project_id, project_instance_id) = {
            let state = shared.lock().map_err(|_| {
                IpcProtocolError::Io(std::io::Error::other("project host state is unavailable"))
            })?;
            (
                state.session.session().project_id(),
                state.session.session().project_instance_id(),
            )
        };
        let mut auth_token_uuid = Uuid::new_v4();
        while auth_token_uuid.to_string() == project_id.to_string()
            || auth_token_uuid.to_string() == project_instance_id.to_string()
        {
            auth_token_uuid = Uuid::new_v4();
        }
        let auth_token = auth_token_uuid.to_string();
        let runtime_dir = create_runtime_directory()?;
        let descriptor_path = descriptor_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| runtime_dir.join("endpoint.json"));
        let mut resources = EndpointResources {
            runtime_dir: Some(runtime_dir.clone()),
            descriptor_path: descriptor_path.clone(),
            descriptor_created: false,
            socket_path: None,
        };

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let (endpoint, bound) = {
            use std::os::unix::{fs::PermissionsExt, net::UnixListener};

            let socket_path = runtime_dir.join("s");
            let listener = UnixListener::bind(&socket_path).map_err(IpcProtocolError::Io)?;
            resources.socket_path = Some(socket_path.clone());
            fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))
                .map_err(IpcProtocolError::Io)?;
            let endpoint = socket_path
                .to_str()
                .ok_or(IpcProtocolError::UnsupportedPlatform)?
                .to_owned();
            (endpoint, BoundEndpoint::Unix(listener))
        };

        #[cfg(windows)]
        let (endpoint, bound) = {
            let pipe_name = format!(r"\\.\pipe\opencut-reinforced-{}", Uuid::new_v4());
            let pipe = crate::windows::create_server_pipe(&pipe_name)?;
            (
                pipe_name.clone(),
                BoundEndpoint::Windows { pipe, pipe_name },
            )
        };

        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        let (endpoint, bound): (String, BoundEndpoint) = {
            let _ = (runtime_dir, auth_token);
            return Err(IpcProtocolError::UnsupportedPlatform);
        };

        let descriptor = EndpointDescriptor::new(
            bound.transport(),
            endpoint,
            auth_token,
            project_id,
            project_instance_id,
        );
        write_descriptor(
            &descriptor_path,
            &descriptor,
            &mut resources.descriptor_created,
        )?;

        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stopping = Arc::clone(&stopping);
        let worker_token = descriptor.auth_token().to_owned();
        let worker = thread::Builder::new()
            .name("or-local-ipc".to_owned())
            .spawn(move || {
                let result = serve(bound, shared, worker_token, worker_stopping);
                drop(resources);
                result
            })
            .map_err(IpcProtocolError::Io)?;

        Ok(Self {
            descriptor,
            descriptor_path,
            stopping,
            worker: Some(worker),
        })
    }

    pub fn descriptor(&self) -> &EndpointDescriptor {
        &self.descriptor
    }

    pub fn descriptor_path(&self) -> &Path {
        &self.descriptor_path
    }

    /// Waits until an authenticated shutdown request ends the server loop.
    pub fn wait(mut self) -> Result<(), IpcProtocolError> {
        self.join_worker()
    }

    fn join_worker(&mut self) -> Result<(), IpcProtocolError> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        worker.join().map_err(|_| {
            IpcProtocolError::Io(std::io::Error::other("local IPC server thread panicked"))
        })?
    }
}

impl Drop for LocalIpcServer {
    fn drop(&mut self) {
        if self.worker.is_none() {
            return;
        }
        self.stopping.store(true, Ordering::SeqCst);
        wake_server(&self.descriptor);
        let _ = self.join_worker();
    }
}

struct EndpointResources {
    runtime_dir: Option<PathBuf>,
    descriptor_path: PathBuf,
    descriptor_created: bool,
    socket_path: Option<PathBuf>,
}

impl Drop for EndpointResources {
    fn drop(&mut self) {
        if self.descriptor_created {
            let _ = fs::remove_file(&self.descriptor_path);
        }
        if let Some(path) = &self.socket_path {
            let _ = fs::remove_file(path);
        }
        if let Some(path) = &self.runtime_dir {
            let _ = fs::remove_dir(path);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
enum BoundEndpoint {
    Unix(std::os::unix::net::UnixListener),
}

#[cfg(windows)]
enum BoundEndpoint {
    Windows {
        pipe: crate::windows::PipeHandle,
        pipe_name: String,
    },
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
enum BoundEndpoint {}

impl BoundEndpoint {
    fn transport(&self) -> IpcTransport {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let _ = self;
            IpcTransport::UnixDomainSocket
        }
        #[cfg(windows)]
        {
            let _ = self;
            IpcTransport::WindowsNamedPipe
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        match *self {}
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestEnvelope {
    protocol_version: u32,
    request_id: Uuid,
    auth_token: String,
    request: IpcRequest,
}

fn create_runtime_directory() -> Result<PathBuf, IpcProtocolError> {
    let id = Uuid::new_v4();
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let path = PathBuf::from("/tmp").join(format!("or-ipc-{id}"));
    #[cfg(windows)]
    let path = std::env::temp_dir().join(format!("or-ipc-{id}"));
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    let path = {
        let _ = id;
        return Err(IpcProtocolError::UnsupportedPlatform);
    };

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = fs::DirBuilder::new();
        builder
            .mode(0o700)
            .create(&path)
            .map_err(IpcProtocolError::Io)?;
    }
    #[cfg(windows)]
    crate::windows::create_private_directory(&path).map_err(IpcProtocolError::Io)?;
    Ok(path)
}

fn write_descriptor(
    path: &Path,
    descriptor: &EndpointDescriptor,
    created: &mut bool,
) -> Result<(), IpcProtocolError> {
    let encoded =
        serde_json::to_vec_pretty(descriptor).map_err(|_| IpcProtocolError::InvalidDescriptor)?;
    if encoded.len() as u64 > MAX_DESCRIPTOR_BYTES {
        return Err(IpcProtocolError::InvalidDescriptor);
    }
    #[cfg(windows)]
    let mut file = crate::windows::create_private_file(path).map_err(IpcProtocolError::Io)?;
    #[cfg(not(windows))]
    let mut file = {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path).map_err(IpcProtocolError::Io)?
    };
    *created = true;
    file.write_all(&encoded).map_err(IpcProtocolError::Io)?;
    file.sync_all().map_err(IpcProtocolError::Io)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn serve(
    bound: BoundEndpoint,
    shared: Arc<Mutex<LiveProjectHostState>>,
    auth_token: String,
    stopping: Arc<AtomicBool>,
) -> Result<(), IpcProtocolError> {
    let BoundEndpoint::Unix(listener) = bound;
    loop {
        let (mut stream, _) = match listener.accept() {
            Ok(connection) => connection,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(IpcProtocolError::Io(error)),
        };
        if stopping.load(Ordering::SeqCst) {
            break;
        }
        if let Ok(shutdown) = handle_shared_connection(&mut stream, &shared, &auth_token) {
            if shutdown {
                break;
            }
        }
    }
    drop(listener);
    Ok(())
}

#[cfg(windows)]
fn serve(
    bound: BoundEndpoint,
    shared: Arc<Mutex<LiveProjectHostState>>,
    auth_token: String,
    stopping: Arc<AtomicBool>,
) -> Result<(), IpcProtocolError> {
    let BoundEndpoint::Windows { pipe, pipe_name } = bound;
    crate::windows::serve(pipe, &pipe_name, &stopping, |stream| {
        if stopping.load(Ordering::SeqCst) {
            return Ok(true);
        }
        handle_shared_connection(stream, &shared, &auth_token)
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn serve(
    _bound: BoundEndpoint,
    _shared: Arc<Mutex<LiveProjectHostState>>,
    _auth_token: String,
    _stopping: Arc<AtomicBool>,
) -> Result<(), IpcProtocolError> {
    Err(IpcProtocolError::UnsupportedPlatform)
}

#[cfg(test)]
fn handle_connection(
    stream: &mut (impl Read + Write),
    session: &mut ProjectFileSession,
    auth_token: &str,
) -> Result<bool, IpcProtocolError> {
    handle_connection_with(stream, auth_token, |request| dispatch(session, request))
}

fn handle_shared_connection(
    stream: &mut (impl Read + Write),
    shared: &Arc<Mutex<LiveProjectHostState>>,
    auth_token: &str,
) -> Result<bool, IpcProtocolError> {
    handle_connection_with(stream, auth_token, |request| {
        dispatch_shared(shared, request)
    })
}

fn handle_connection_with(
    stream: &mut (impl Read + Write),
    auth_token: &str,
    mut dispatch_request: impl FnMut(IpcRequest) -> (IpcResponseResult, bool),
) -> Result<bool, IpcProtocolError> {
    let payload = read_frame(stream)?;
    let request_text = std::str::from_utf8(&payload).map_err(|_| IpcProtocolError::InvalidUtf8)?;
    let envelope: RequestEnvelope =
        serde_json::from_str(request_text).map_err(|_| IpcProtocolError::InvalidRequest)?;
    if !is_uuid_v4(envelope.request_id) {
        return Err(IpcProtocolError::InvalidRequest);
    }

    let (result, shutdown) = if envelope.protocol_version != OR_LOCAL_IPC_PROTOCOL_VERSION {
        (
            IpcResponseResult::Error(IpcErrorCode::UnsupportedProtocol),
            false,
        )
    } else if !token_matches(auth_token.as_bytes(), envelope.auth_token.as_bytes()) {
        (
            IpcResponseResult::Error(IpcErrorCode::AuthenticationFailed),
            false,
        )
    } else {
        dispatch_request(envelope.request)
    };

    let response = IpcResponse {
        protocol_version: OR_LOCAL_IPC_PROTOCOL_VERSION,
        request_id: envelope.request_id,
        result,
    };
    let encoded = serde_json::to_vec(&response).map_err(|_| IpcProtocolError::InvalidRequest)?;
    write_frame(stream, &encoded)?;
    Ok(shutdown)
}

fn dispatch(session: &mut ProjectFileSession, request: IpcRequest) -> (IpcResponseResult, bool) {
    match request {
        IpcRequest::Describe => (
            IpcResponseResult::Success(IpcSuccess::Describe(describe(session))),
            false,
        ),
        IpcRequest::Application(request) => {
            let response = session.handle_application_request(request);
            let result = match response {
                ApplicationResponse::Command(result) => IpcResponseResult::Success(
                    IpcSuccess::Application(ApplicationSuccess::Command(result)),
                ),
                ApplicationResponse::Query(result) => IpcResponseResult::Success(
                    IpcSuccess::Application(ApplicationSuccess::Query(result)),
                ),
                ApplicationResponse::Transaction(result) => IpcResponseResult::Success(
                    IpcSuccess::Application(ApplicationSuccess::Transaction(result)),
                ),
                ApplicationResponse::Error(error) => IpcResponseResult::ApplicationError(error),
            };
            (result, false)
        }
        IpcRequest::Save => match session.save() {
            Ok(()) => (
                IpcResponseResult::Success(IpcSuccess::Save(SaveResponse {
                    project_revision: session.session().project_revision(),
                    dirty: session.is_dirty(),
                })),
                false,
            ),
            Err(error) => (
                IpcResponseResult::Error(map_file_error(error.code())),
                false,
            ),
        },
        IpcRequest::Shutdown { discard_unsaved } => {
            if session.is_dirty() && !discard_unsaved {
                (
                    IpcResponseResult::Error(IpcErrorCode::UnsavedChanges),
                    false,
                )
            } else {
                (IpcResponseResult::Success(IpcSuccess::Shutdown), true)
            }
        }
    }
}

fn dispatch_shared(
    shared: &Arc<Mutex<LiveProjectHostState>>,
    request: IpcRequest,
) -> (IpcResponseResult, bool) {
    let Ok(mut state) = shared.lock() else {
        return (
            IpcResponseResult::Error(IpcErrorCode::ServerStateError),
            false,
        );
    };
    if state.closing {
        return (
            IpcResponseResult::Error(IpcErrorCode::ServerStateError),
            false,
        );
    }

    let before = state.session.session().project_revision();
    let (result, shutdown) = dispatch(&mut state.session, request);
    if state.session.session().project_revision() != before {
        state.publish(ProjectHostEventKind::ProjectChanged);
    }
    if matches!(&result, IpcResponseResult::Success(IpcSuccess::Save(_))) {
        state.publish(ProjectHostEventKind::ProjectSaved);
    }
    if shutdown {
        state.closing = true;
        state.publish(ProjectHostEventKind::SessionClosing);
    }
    (result, shutdown)
}

fn describe(session: &ProjectFileSession) -> DescribeResponse {
    DescribeResponse {
        protocol_version: OR_LOCAL_IPC_PROTOCOL_VERSION,
        project_id: session.session().project_id(),
        project_instance_id: session.session().project_instance_id(),
        project_revision: session.session().project_revision(),
        dirty: session.is_dirty(),
        commands: command_catalog()
            .iter()
            .map(|descriptor| IpcCommandDescriptor {
                id: descriptor.id.to_owned(),
                schema_version: descriptor.schema_version,
                mutates_project: descriptor.mutates_project,
                allowed_in_transaction: descriptor.allowed_in_transaction,
            })
            .collect(),
        queries: query_catalog()
            .iter()
            .map(|descriptor| IpcQueryDescriptor {
                id: descriptor.id.to_owned(),
                schema_version: descriptor.schema_version,
            })
            .collect(),
    }
}

fn map_file_error(code: ProjectFileSessionErrorCode) -> IpcErrorCode {
    match code {
        ProjectFileSessionErrorCode::RecoveryRequired => IpcErrorCode::RecoveryRequired,
        ProjectFileSessionErrorCode::ProjectFileChanged => IpcErrorCode::ProjectFileChanged,
        ProjectFileSessionErrorCode::DestinationExists => IpcErrorCode::ServerStateError,
        ProjectFileSessionErrorCode::StorageFailure => IpcErrorCode::ServerStateError,
    }
}

fn token_matches(expected: &[u8], provided: &[u8]) -> bool {
    let mut difference = expected.len() ^ provided.len();
    for index in 0..expected.len().max(provided.len()) {
        difference |= usize::from(
            expected.get(index).copied().unwrap_or(0) ^ provided.get(index).copied().unwrap_or(0),
        );
    }
    difference == 0
}

fn wake_server(descriptor: &EndpointDescriptor) {
    match descriptor.transport() {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        IpcTransport::UnixDomainSocket => {
            let _ = std::os::unix::net::UnixStream::connect(descriptor.endpoint());
        }
        #[cfg(windows)]
        IpcTransport::WindowsNamedPipe => crate::windows::wake(descriptor.endpoint()),
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use or_core::{ProjectDocument, save_project_file_atomic};
    use std::{
        fs,
        io::{Cursor, Read},
    };

    struct MemoryIo {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl MemoryIo {
        fn new(input: Vec<u8>) -> Self {
            Self {
                input: Cursor::new(input),
                output: Vec::new(),
            }
        }
    }

    impl Read for MemoryIo {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            self.input.read(buffer)
        }
    }

    impl Write for MemoryIo {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.output.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn fixture() -> (PathBuf, ProjectFileSession) {
        let directory = std::env::temp_dir().join(format!("or-ipc-server-test-{}", Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("sample.orproj");
        save_project_file_atomic(&path, &ProjectDocument::new("A")).unwrap();
        let session = ProjectFileSession::open(&path).unwrap();
        (directory, session)
    }

    fn framed_request(protocol_version: u32, auth_token: &str, request: IpcRequest) -> Vec<u8> {
        let value = serde_json::json!({
            "protocol_version": protocol_version,
            "request_id": Uuid::new_v4(),
            "auth_token": auth_token,
            "request": request,
        });
        let payload = serde_json::to_vec(&value).unwrap();
        let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
        frame.extend(payload);
        frame
    }

    fn response(io: &MemoryIo) -> IpcResponse {
        let mut cursor = Cursor::new(io.output.as_slice());
        let payload = read_frame(&mut cursor).unwrap();
        serde_json::from_slice(&payload).unwrap()
    }

    #[test]
    fn authentication_and_protocol_rejections_happen_before_application_dispatch() {
        let (directory, mut session) = fixture();
        let original = session.session().project().clone();
        let token = Uuid::new_v4().to_string();
        let application_request = or_core::ApplicationRequest::Command(or_core::CommandEnvelope {
            command_id: "project.rename".to_owned(),
            schema_version: 1,
            project_id: session.session().project_id(),
            project_instance_id: session.session().project_instance_id(),
            expected_project_revision: session.session().project_revision(),
            arguments: serde_json::json!({ "name": "changed" }),
        });

        let mut bad_auth = MemoryIo::new(framed_request(
            OR_LOCAL_IPC_PROTOCOL_VERSION,
            "wrong-token",
            IpcRequest::Application(application_request),
        ));
        handle_connection(&mut bad_auth, &mut session, &token).unwrap();
        let bad_auth_response = response(&bad_auth);
        assert!(matches!(
            bad_auth_response.result,
            IpcResponseResult::Error(IpcErrorCode::AuthenticationFailed)
        ));
        let encoded_response = String::from_utf8(bad_auth.output[4..].to_vec()).unwrap();
        assert!(!encoded_response.contains(&original.id().to_string()));
        assert_eq!(session.session().project(), &original);

        let mut unsupported = MemoryIo::new(framed_request(
            OR_LOCAL_IPC_PROTOCOL_VERSION + 1,
            &token,
            IpcRequest::Application(or_core::ApplicationRequest::Command(
                or_core::CommandEnvelope {
                    command_id: "project.rename".to_owned(),
                    schema_version: 1,
                    project_id: session.session().project_id(),
                    project_instance_id: session.session().project_instance_id(),
                    expected_project_revision: session.session().project_revision(),
                    arguments: serde_json::json!({ "name": "changed" }),
                },
            )),
        ));
        handle_connection(&mut unsupported, &mut session, &token).unwrap();
        assert!(matches!(
            response(&unsupported).result,
            IpcResponseResult::Error(IpcErrorCode::UnsupportedProtocol)
        ));
        assert_eq!(session.session().project(), &original);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn malformed_utf8_and_json_are_rejected_without_mutation() {
        let (directory, mut session) = fixture();
        let original = session.session().project().clone();

        let mut invalid_utf8 = MemoryIo::new(vec![0, 0, 0, 1, 0xff]);
        assert!(matches!(
            handle_connection(&mut invalid_utf8, &mut session, "token"),
            Err(IpcProtocolError::InvalidUtf8)
        ));

        let mut invalid_json = MemoryIo::new(vec![0, 0, 0, 1, b'{']);
        assert!(matches!(
            handle_connection(&mut invalid_json, &mut session, "token"),
            Err(IpcProtocolError::InvalidRequest)
        ));
        assert_eq!(session.session().project(), &original);
        fs::remove_dir_all(directory).unwrap();
    }
}
