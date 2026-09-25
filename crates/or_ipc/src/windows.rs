use crate::protocol::{IpcProtocolError, MAX_IPC_FRAME_BYTES};
use std::{
    io::{self, Read, Write},
    ptr::{null, null_mut},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_BROKEN_PIPE, ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE,
        GetLastError, HANDLE, INVALID_HANDLE_VALUE,
    },
    Storage::FileSystem::{
        CreateFileW, FlushFileBuffers, OPEN_EXISTING, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
    },
    System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE,
        PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
    },
};

const PIPE_BUFFER_BYTES: u32 = MAX_IPC_FRAME_BYTES as u32 + 4;

pub(crate) struct PipeHandle {
    raw: HANDLE,
}

impl PipeHandle {
    fn new(raw: HANDLE) -> Self {
        Self { raw }
    }

    fn into_stream(mut self) -> PipeStream {
        let raw = std::mem::replace(&mut self.raw, null_mut());
        PipeStream { raw }
    }
}

impl Drop for PipeHandle {
    fn drop(&mut self) {
        if !self.raw.is_null() && self.raw != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.raw);
            }
        }
    }
}

pub(crate) struct PipeStream {
    raw: HANDLE,
}

impl PipeStream {
    fn handle(&self) -> HANDLE {
        self.raw
    }
}

impl Drop for PipeStream {
    fn drop(&mut self) {
        if !self.raw.is_null() && self.raw != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.raw);
            }
        }
    }
}

impl Read for PipeStream {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let count = buffer.len().min(u32::MAX as usize) as u32;
        let mut read = 0;
        let success =
            unsafe { ReadFile(self.raw, buffer.as_mut_ptr(), count, &mut read, null_mut()) };
        if success == 0 {
            let code = unsafe { GetLastError() };
            if code == ERROR_BROKEN_PIPE {
                return Ok(0);
            }
            return Err(io::Error::from_raw_os_error(code as i32));
        }
        Ok(read as usize)
    }
}

impl Write for PipeStream {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let count = buffer.len().min(u32::MAX as usize) as u32;
        let mut written = 0;
        let success =
            unsafe { WriteFile(self.raw, buffer.as_ptr(), count, &mut written, null_mut()) };
        if success == 0 {
            return Err(io::Error::from_raw_os_error(
                unsafe { GetLastError() } as i32
            ));
        }
        Ok(written as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn create_server_pipe(name: &str) -> Result<PipeHandle, IpcProtocolError> {
    let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let raw = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            PIPE_BUFFER_BYTES,
            PIPE_BUFFER_BYTES,
            0,
            null(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(IpcProtocolError::Io(io::Error::from_raw_os_error(
            unsafe { GetLastError() } as i32,
        )));
    }
    Ok(PipeHandle::new(raw))
}

pub(crate) fn connect(name: &str) -> Result<PipeStream, IpcProtocolError> {
    let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let raw = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            null(),
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(IpcProtocolError::EndpointUnavailable);
    }
    Ok(PipeStream { raw })
}

pub(crate) fn serve(
    mut pipe: PipeHandle,
    pipe_name: &str,
    stopping: &Arc<AtomicBool>,
    mut handle_request: impl FnMut(&mut PipeStream) -> Result<bool, IpcProtocolError>,
) -> Result<(), IpcProtocolError> {
    loop {
        let connected = unsafe { ConnectNamedPipe(pipe.raw, null_mut()) };
        if connected == 0 && unsafe { GetLastError() } != ERROR_PIPE_CONNECTED {
            if stopping.load(Ordering::SeqCst) {
                break;
            }
            return Err(IpcProtocolError::Io(io::Error::from_raw_os_error(
                unsafe { GetLastError() } as i32,
            )));
        }

        let mut stream = pipe.into_stream();
        let shutdown = if stopping.load(Ordering::SeqCst) {
            true
        } else {
            match handle_request(&mut stream) {
                Ok(shutdown) => {
                    unsafe {
                        FlushFileBuffers(stream.handle());
                    }
                    shutdown
                }
                Err(_) => false,
            }
        };
        unsafe {
            DisconnectNamedPipe(stream.handle());
        }
        drop(stream);
        if shutdown || stopping.load(Ordering::SeqCst) {
            break;
        }
        pipe = create_server_pipe(pipe_name)?;
    }
    Ok(())
}

pub(crate) fn wake(name: &str) {
    for _ in 0..100 {
        if connect(name).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
