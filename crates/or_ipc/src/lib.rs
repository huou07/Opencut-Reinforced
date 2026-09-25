mod client;
mod live_host;
mod protocol;
mod server;
#[cfg(windows)]
mod windows;

pub use client::{IpcClientError, LocalIpcClient};
pub use live_host::{
    LiveProjectHost, LiveProjectHostError, ProjectHostEvent, ProjectHostEventKind,
};
pub use protocol::{
    ApplicationSuccess, DescribeResponse, EndpointDescriptor, IpcCommandDescriptor, IpcErrorCode,
    IpcProtocolError, IpcQueryDescriptor, IpcRequest, IpcResponse, IpcResponseResult, IpcSuccess,
    IpcTransport, MAX_IPC_FRAME_BYTES, OR_LOCAL_IPC_PROTOCOL_VERSION, SaveResponse,
};
pub use server::LocalIpcServer;
