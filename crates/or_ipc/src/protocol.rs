use or_core::{
    ApplicationRequest, CommandResult, OperationError, ProjectId, ProjectInstanceId,
    ProjectRevision, QueryResult, TransactionResult,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, io};
use uuid::Uuid;

pub const OR_LOCAL_IPC_PROTOCOL_VERSION: u32 = 1;
pub const MAX_IPC_FRAME_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_DESCRIPTOR_BYTES: u64 = 16 * 1024;
const DESCRIPTOR_FORMAT: &str = "opencut-reinforced-ipc-endpoint";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcTransport {
    UnixDomainSocket,
    WindowsNamedPipe,
}

/// Strict connection data stored in a private endpoint descriptor.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointDescriptor {
    format: String,
    pub protocol_version: u32,
    pub transport: IpcTransport,
    pub endpoint: String,
    auth_token: String,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
}

impl EndpointDescriptor {
    pub fn transport(&self) -> IpcTransport {
        self.transport
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    pub fn project_instance_id(&self) -> ProjectInstanceId {
        self.project_instance_id
    }

    pub(crate) fn new(
        transport: IpcTransport,
        endpoint: String,
        auth_token: String,
        project_id: ProjectId,
        project_instance_id: ProjectInstanceId,
    ) -> Self {
        Self {
            format: DESCRIPTOR_FORMAT.to_owned(),
            protocol_version: OR_LOCAL_IPC_PROTOCOL_VERSION,
            transport,
            endpoint,
            auth_token,
            project_id,
            project_instance_id,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), IpcProtocolError> {
        if self.format != DESCRIPTOR_FORMAT
            || self.protocol_version != OR_LOCAL_IPC_PROTOCOL_VERSION
            || self.endpoint.is_empty()
        {
            return Err(IpcProtocolError::InvalidDescriptor);
        }
        let token =
            Uuid::parse_str(&self.auth_token).map_err(|_| IpcProtocolError::InvalidDescriptor)?;
        if token.get_version() != Some(uuid::Version::Random) {
            return Err(IpcProtocolError::InvalidDescriptor);
        }
        Ok(())
    }

    pub(crate) fn auth_token(&self) -> &str {
        &self.auth_token
    }
}

impl fmt::Debug for EndpointDescriptor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EndpointDescriptor")
            .field("format", &self.format)
            .field("protocol_version", &self.protocol_version)
            .field("transport", &self.transport)
            .field("endpoint", &self.endpoint)
            .field("auth_token", &"[REDACTED]")
            .field("project_id", &self.project_id)
            .field("project_instance_id", &self.project_instance_id)
            .finish()
    }
}

/// One supported local control request. Application operations use `or_core` contracts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "body",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum IpcRequest {
    Describe,
    Application(ApplicationRequest),
    Save,
    Shutdown { discard_unsaved: bool },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IpcCommandDescriptor {
    pub id: String,
    pub schema_version: u64,
    pub mutates_project: bool,
    pub allowed_in_transaction: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IpcQueryDescriptor {
    pub id: String,
    pub schema_version: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DescribeResponse {
    pub protocol_version: u32,
    pub project_id: ProjectId,
    pub project_instance_id: ProjectInstanceId,
    pub project_revision: ProjectRevision,
    pub dirty: bool,
    pub commands: Vec<IpcCommandDescriptor>,
    pub queries: Vec<IpcQueryDescriptor>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "result",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ApplicationSuccess {
    Command(CommandResult),
    Query(QueryResult),
    Transaction(TransactionResult),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveResponse {
    pub project_revision: ProjectRevision,
    pub dirty: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "body",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum IpcSuccess {
    Describe(DescribeResponse),
    Application(ApplicationSuccess),
    Save(SaveResponse),
    Shutdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IpcErrorCode {
    UnsupportedProtocol,
    AuthenticationFailed,
    InvalidFrame,
    FrameTooLarge,
    InvalidRequest,
    ServerStateError,
    ProjectFileChanged,
    RecoveryRequired,
    UnsavedChanges,
    EndpointUnavailable,
    UnsupportedPlatform,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "body",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum IpcResponseResult {
    Success(IpcSuccess),
    ApplicationError(OperationError),
    Error(IpcErrorCode),
}

/// Strict response envelope. The client checks both version and request identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IpcResponse {
    pub protocol_version: u32,
    pub request_id: Uuid,
    pub result: IpcResponseResult,
}

#[derive(Debug)]
pub enum IpcProtocolError {
    Io(io::Error),
    InvalidFrame,
    FrameTooLarge,
    InvalidUtf8,
    InvalidJson,
    InvalidRequest,
    InvalidDescriptor,
    UnsupportedProtocol,
    UnsupportedPlatform,
    EndpointUnavailable,
    ResponseMismatch,
    Remote(IpcErrorCode),
    Application(OperationError),
}

impl fmt::Display for IpcProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "local IPC I/O failed: {error}"),
            Self::InvalidFrame => formatter.write_str("local IPC frame is invalid or truncated"),
            Self::FrameTooLarge => formatter.write_str("local IPC frame exceeds 1 MiB"),
            Self::InvalidUtf8 => formatter.write_str("local IPC payload is not valid UTF-8"),
            Self::InvalidJson => formatter.write_str("local IPC payload is not valid JSON"),
            Self::InvalidRequest => formatter.write_str("local IPC request is invalid"),
            Self::InvalidDescriptor => {
                formatter.write_str("local IPC endpoint descriptor is invalid")
            }
            Self::UnsupportedProtocol => {
                formatter.write_str("local IPC protocol version is unsupported")
            }
            Self::UnsupportedPlatform => {
                formatter.write_str("local IPC is unsupported on this platform")
            }
            Self::EndpointUnavailable => formatter.write_str("local IPC endpoint is unavailable"),
            Self::ResponseMismatch => {
                formatter.write_str("local IPC response does not match the request")
            }
            Self::Remote(code) => write!(formatter, "local IPC server returned {code:?}"),
            Self::Application(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl Error for IpcProtocolError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Application(error) => Some(error),
            Self::InvalidFrame
            | Self::FrameTooLarge
            | Self::InvalidUtf8
            | Self::InvalidJson
            | Self::InvalidRequest
            | Self::InvalidDescriptor
            | Self::UnsupportedProtocol
            | Self::UnsupportedPlatform
            | Self::EndpointUnavailable
            | Self::ResponseMismatch
            | Self::Remote(_) => None,
        }
    }
}

pub(crate) fn is_uuid_v4(value: Uuid) -> bool {
    value.get_version() == Some(uuid::Version::Random)
}

pub(crate) fn read_frame(reader: &mut impl io::Read) -> Result<Vec<u8>, IpcProtocolError> {
    let mut prefix = [0; 4];
    reader
        .read_exact(&mut prefix)
        .map_err(|_| IpcProtocolError::InvalidFrame)?;
    let length = u32::from_be_bytes(prefix) as usize;
    if length == 0 {
        return Err(IpcProtocolError::InvalidFrame);
    }
    if length > MAX_IPC_FRAME_BYTES {
        return Err(IpcProtocolError::FrameTooLarge);
    }
    let mut payload = vec![0; length];
    reader
        .read_exact(&mut payload)
        .map_err(|_| IpcProtocolError::InvalidFrame)?;
    Ok(payload)
}

pub(crate) fn write_frame(
    writer: &mut impl io::Write,
    payload: &[u8],
) -> Result<(), IpcProtocolError> {
    if payload.is_empty() {
        return Err(IpcProtocolError::InvalidFrame);
    }
    if payload.len() > MAX_IPC_FRAME_BYTES {
        return Err(IpcProtocolError::FrameTooLarge);
    }
    let length = u32::try_from(payload.len()).map_err(|_| IpcProtocolError::FrameTooLarge)?;
    writer
        .write_all(&length.to_be_bytes())
        .and_then(|()| writer.write_all(payload))
        .map_err(IpcProtocolError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn descriptor() -> EndpointDescriptor {
        EndpointDescriptor::new(
            IpcTransport::UnixDomainSocket,
            "/tmp/or-ipc-test/s".to_owned(),
            Uuid::new_v4().to_string(),
            ProjectId::generate(),
            ProjectInstanceId::generate(),
        )
    }

    #[test]
    fn frame_uses_four_byte_big_endian_length_prefix() {
        let mut encoded = Vec::new();
        write_frame(&mut encoded, b"OR").unwrap();
        assert_eq!(&encoded[..4], &[0, 0, 0, 2]);
        assert_eq!(read_frame(&mut Cursor::new(encoded)).unwrap(), b"OR");
    }

    #[test]
    fn frame_reader_rejects_empty_oversized_and_truncated_frames_before_dispatch() {
        assert!(matches!(
            read_frame(&mut Cursor::new([0, 0, 0, 0])),
            Err(IpcProtocolError::InvalidFrame)
        ));
        assert!(matches!(
            read_frame(&mut Cursor::new(
                (MAX_IPC_FRAME_BYTES as u32 + 1).to_be_bytes()
            )),
            Err(IpcProtocolError::FrameTooLarge)
        ));
        assert!(matches!(
            read_frame(&mut Cursor::new([0, 0, 0, 2, b'a'])),
            Err(IpcProtocolError::InvalidFrame)
        ));
    }

    #[test]
    fn descriptor_round_trips_strictly_and_debug_redacts_the_token() {
        let descriptor = descriptor();
        descriptor.validate().unwrap();
        let encoded = serde_json::to_string(&descriptor).unwrap();
        let decoded: EndpointDescriptor = serde_json::from_str(&encoded).unwrap();
        decoded.validate().unwrap();
        assert_eq!(descriptor, decoded);
        assert!(format!("{descriptor:?}").contains("[REDACTED]"));
        assert!(!format!("{descriptor:?}").contains(descriptor.auth_token()));

        let mut with_unknown_field: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        with_unknown_field["unexpected"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<EndpointDescriptor>(with_unknown_field).is_err());
    }

    #[test]
    fn descriptor_rejects_wrong_format_protocol_ids_and_non_v4_tokens() {
        let descriptor = descriptor();
        let mut value: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&descriptor).unwrap()).unwrap();

        value["format"] = serde_json::Value::String("wrong".to_owned());
        assert!(
            serde_json::from_value::<EndpointDescriptor>(value.clone())
                .unwrap()
                .validate()
                .is_err()
        );
        value["format"] = serde_json::Value::String(DESCRIPTOR_FORMAT.to_owned());

        value["protocol_version"] = serde_json::Value::from(2);
        assert!(
            serde_json::from_value::<EndpointDescriptor>(value.clone())
                .unwrap()
                .validate()
                .is_err()
        );
        value["protocol_version"] = serde_json::Value::from(1);

        value["auth_token"] =
            serde_json::Value::String("00000000-0000-0000-0000-000000000000".to_owned());
        assert!(
            serde_json::from_value::<EndpointDescriptor>(value.clone())
                .unwrap()
                .validate()
                .is_err()
        );

        value["project_id"] = serde_json::Value::String("not-an-id".to_owned());
        assert!(serde_json::from_value::<EndpointDescriptor>(value).is_err());
    }
}
