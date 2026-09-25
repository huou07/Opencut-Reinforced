use crate::protocol::{
    ApplicationSuccess, DescribeResponse, EndpointDescriptor, IpcProtocolError, IpcRequest,
    IpcResponse, IpcResponseResult, IpcSuccess, MAX_DESCRIPTOR_BYTES,
    OR_LOCAL_IPC_PROTOCOL_VERSION, read_frame, write_frame,
};
use serde::Serialize;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
use std::io::Write;
use std::{fs::File, io::Read, path::Path};
use uuid::Uuid;

pub type IpcClientError = IpcProtocolError;

/// A one-request-per-connection client for an explicit endpoint descriptor.
pub struct LocalIpcClient {
    descriptor: EndpointDescriptor,
}

impl LocalIpcClient {
    pub fn open(descriptor_path: impl AsRef<Path>) -> Result<Self, IpcClientError> {
        let file = File::open(descriptor_path).map_err(|error| {
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
            ) {
                IpcProtocolError::EndpointUnavailable
            } else {
                IpcProtocolError::Io(error)
            }
        })?;
        if file.metadata().map_err(IpcProtocolError::Io)?.len() > MAX_DESCRIPTOR_BYTES {
            return Err(IpcProtocolError::InvalidDescriptor);
        }
        let mut bytes = Vec::new();
        file.take(MAX_DESCRIPTOR_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(IpcProtocolError::Io)?;
        if bytes.len() as u64 > MAX_DESCRIPTOR_BYTES {
            return Err(IpcProtocolError::InvalidDescriptor);
        }
        let encoded =
            std::str::from_utf8(&bytes).map_err(|_| IpcProtocolError::InvalidDescriptor)?;
        let descriptor: EndpointDescriptor =
            serde_json::from_str(encoded).map_err(|_| IpcProtocolError::InvalidDescriptor)?;
        descriptor.validate()?;
        validate_platform_endpoint(&descriptor)?;
        Ok(Self { descriptor })
    }

    pub fn descriptor(&self) -> &EndpointDescriptor {
        &self.descriptor
    }

    pub fn request(&self, request: IpcRequest) -> Result<IpcSuccess, IpcClientError> {
        let request_id = Uuid::new_v4();
        let envelope = RequestEnvelope {
            protocol_version: OR_LOCAL_IPC_PROTOCOL_VERSION,
            request_id,
            auth_token: self.descriptor.auth_token().to_owned(),
            request,
        };
        let payload =
            serde_json::to_vec(&envelope).map_err(|_| IpcProtocolError::InvalidRequest)?;

        with_connection(&self.descriptor, |stream| {
            write_frame(stream, &payload)?;
            let payload = read_frame(stream)?;
            let response_text =
                std::str::from_utf8(&payload).map_err(|_| IpcProtocolError::InvalidUtf8)?;
            let response: IpcResponse =
                serde_json::from_str(response_text).map_err(|_| IpcProtocolError::InvalidJson)?;
            if response.protocol_version != OR_LOCAL_IPC_PROTOCOL_VERSION {
                return Err(IpcProtocolError::UnsupportedProtocol);
            }
            if response.request_id != request_id {
                return Err(IpcProtocolError::ResponseMismatch);
            }
            match response.result {
                IpcResponseResult::Success(result) => Ok(result),
                IpcResponseResult::ApplicationError(error) => {
                    Err(IpcProtocolError::Application(error))
                }
                IpcResponseResult::Error(code) => Err(IpcProtocolError::Remote(code)),
            }
        })
    }

    pub fn describe(&self) -> Result<DescribeResponse, IpcClientError> {
        match self.request(IpcRequest::Describe)? {
            IpcSuccess::Describe(response) => {
                if response.protocol_version != OR_LOCAL_IPC_PROTOCOL_VERSION {
                    return Err(IpcProtocolError::UnsupportedProtocol);
                }
                if response.project_id != self.descriptor.project_id()
                    || response.project_instance_id != self.descriptor.project_instance_id()
                {
                    return Err(IpcProtocolError::InvalidRequest);
                }
                Ok(response)
            }
            _ => Err(IpcProtocolError::InvalidRequest),
        }
    }

    pub fn application(
        &self,
        request: or_core::ApplicationRequest,
    ) -> Result<ApplicationSuccess, IpcClientError> {
        match self.request(IpcRequest::Application(request))? {
            IpcSuccess::Application(response) => Ok(response),
            _ => Err(IpcProtocolError::InvalidRequest),
        }
    }

    pub fn save(&self) -> Result<crate::protocol::SaveResponse, IpcClientError> {
        match self.request(IpcRequest::Save)? {
            IpcSuccess::Save(response) => Ok(response),
            _ => Err(IpcProtocolError::InvalidRequest),
        }
    }

    pub fn shutdown(&self, discard_unsaved: bool) -> Result<(), IpcClientError> {
        match self.request(IpcRequest::Shutdown { discard_unsaved })? {
            IpcSuccess::Shutdown => Ok(()),
            _ => Err(IpcProtocolError::InvalidRequest),
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn validate_platform_endpoint(descriptor: &EndpointDescriptor) -> Result<(), IpcProtocolError> {
    use crate::protocol::IpcTransport;

    if descriptor.transport() != IpcTransport::UnixDomainSocket
        || !Path::new(descriptor.endpoint()).is_absolute()
    {
        return Err(IpcProtocolError::InvalidDescriptor);
    }
    Ok(())
}

#[cfg(windows)]
fn validate_platform_endpoint(descriptor: &EndpointDescriptor) -> Result<(), IpcProtocolError> {
    use crate::protocol::IpcTransport;

    if descriptor.transport() != IpcTransport::WindowsNamedPipe
        || !descriptor.endpoint().starts_with(r"\\.\pipe\")
    {
        return Err(IpcProtocolError::InvalidDescriptor);
    }
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn validate_platform_endpoint(_descriptor: &EndpointDescriptor) -> Result<(), IpcProtocolError> {
    Err(IpcProtocolError::UnsupportedPlatform)
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct RequestEnvelope {
    protocol_version: u32,
    request_id: Uuid,
    auth_token: String,
    request: IpcRequest,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn with_connection<T>(
    descriptor: &EndpointDescriptor,
    operation: impl FnOnce(&mut std::os::unix::net::UnixStream) -> Result<T, IpcProtocolError>,
) -> Result<T, IpcProtocolError> {
    use crate::protocol::IpcTransport;
    use std::{os::unix::net::UnixStream, path::Path};

    if descriptor.transport() != IpcTransport::UnixDomainSocket {
        return Err(IpcProtocolError::UnsupportedPlatform);
    }
    let mut stream = UnixStream::connect(Path::new(descriptor.endpoint())).map_err(|error| {
        if matches!(
            error.kind(),
            std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
        ) {
            IpcProtocolError::EndpointUnavailable
        } else {
            IpcProtocolError::Io(error)
        }
    })?;
    operation(&mut stream)
}

#[cfg(windows)]
fn with_connection<T>(
    descriptor: &EndpointDescriptor,
    operation: impl FnOnce(&mut crate::windows::PipeStream) -> Result<T, IpcProtocolError>,
) -> Result<T, IpcProtocolError> {
    use crate::protocol::IpcTransport;

    if descriptor.transport() != IpcTransport::WindowsNamedPipe {
        return Err(IpcProtocolError::UnsupportedPlatform);
    }
    let mut stream = crate::windows::connect(descriptor.endpoint())?;
    operation(&mut stream)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn with_connection<T>(
    _descriptor: &EndpointDescriptor,
    _operation: impl FnOnce(&mut UnsupportedStream) -> Result<T, IpcProtocolError>,
) -> Result<T, IpcProtocolError> {
    Err(IpcProtocolError::UnsupportedPlatform)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
struct UnsupportedStream;

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
impl Read for UnsupportedStream {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "local IPC is unsupported on this platform",
        ))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
impl Write for UnsupportedStream {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "local IPC is unsupported on this platform",
        ))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        IpcErrorCode, IpcResponseResult, IpcTransport, MAX_IPC_FRAME_BYTES,
        OR_LOCAL_IPC_PROTOCOL_VERSION, write_frame,
    };
    use or_core::{ProjectId, ProjectInstanceId};
    use std::{fs, io::Read, os::unix::net::UnixListener, thread};

    #[test]
    fn client_rejects_a_response_with_a_different_request_id() {
        let directory = Path::new("/tmp").join(format!("or-rsp-{}", Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        let socket_path = directory.join("s");
        let listener = UnixListener::bind(&socket_path).unwrap();
        let token = Uuid::new_v4().to_string();
        let descriptor = EndpointDescriptor::new(
            IpcTransport::UnixDomainSocket,
            socket_path.to_string_lossy().into_owned(),
            token,
            ProjectId::generate(),
            ProjectInstanceId::generate(),
        );
        let client = LocalIpcClient { descriptor };

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut prefix = [0; 4];
            stream.read_exact(&mut prefix).unwrap();
            let length = u32::from_be_bytes(prefix) as usize;
            assert!(length > 0 && length <= MAX_IPC_FRAME_BYTES);
            let mut payload = vec![0; length];
            stream.read_exact(&mut payload).unwrap();
            let request: serde_json::Value = serde_json::from_slice(&payload).unwrap();
            let request_id = Uuid::parse_str(request["request_id"].as_str().unwrap()).unwrap();
            let response = IpcResponse {
                protocol_version: OR_LOCAL_IPC_PROTOCOL_VERSION,
                request_id: Uuid::new_v4(),
                result: IpcResponseResult::Error(IpcErrorCode::ServerStateError),
            };
            assert_ne!(response.request_id, request_id);
            write_frame(&mut stream, &serde_json::to_vec(&response).unwrap()).unwrap();
        });

        assert!(matches!(
            client.request(IpcRequest::Describe),
            Err(IpcProtocolError::ResponseMismatch)
        ));
        server.join().unwrap();
        fs::remove_dir_all(directory).unwrap();
    }
}
