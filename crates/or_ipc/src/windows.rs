use crate::protocol::{IpcProtocolError, MAX_IPC_FRAME_BYTES};
use std::{
    fs::File,
    io::{self, Read, Write},
    mem::size_of,
    os::windows::{ffi::OsStrExt, io::FromRawHandle},
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
        GetLastError, HANDLE, INVALID_HANDLE_VALUE, LocalFree,
    },
    Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    },
    Security::SECURITY_ATTRIBUTES,
    Storage::FileSystem::{
        CREATE_NEW, CreateDirectoryW, CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_WRITE,
        FlushFileBuffers, OPEN_EXISTING, PIPE_ACCESS_DUPLEX, READ_CONTROL, ReadFile, WriteFile,
    },
    System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE,
        PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
    },
};

const PIPE_BUFFER_BYTES: u32 = MAX_IPC_FRAME_BYTES as u32 + 4;
const PIPE_INSTANCES: u32 = 2;
const OWNER_ONLY_DACL: &str = "D:P(A;;GA;;;OW)";
const OWNER_ONLY_DIRECTORY_DACL: &str = "D:P(A;OICI;GA;;;OW)";

struct PrivateSecurityAttributes {
    descriptor: *mut std::ffi::c_void,
    attributes: SECURITY_ATTRIBUTES,
}

impl PrivateSecurityAttributes {
    fn new(sddl: &str) -> io::Result<Self> {
        let sddl: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
        let mut descriptor = null_mut();
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                null_mut(),
            )
        } == 0
        {
            return Err(last_windows_error());
        }
        Ok(Self {
            descriptor,
            attributes: SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor,
                bInheritHandle: 0,
            },
        })
    }

    fn as_ptr(&self) -> *const SECURITY_ATTRIBUTES {
        &self.attributes
    }
}

impl Drop for PrivateSecurityAttributes {
    fn drop(&mut self) {
        if !self.descriptor.is_null() {
            unsafe {
                LocalFree(self.descriptor);
            }
        }
    }
}

fn last_windows_error() -> io::Error {
    io::Error::from_raw_os_error(unsafe { GetLastError() } as i32)
}

pub(crate) fn create_private_directory(path: &std::path::Path) -> io::Result<()> {
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let security = PrivateSecurityAttributes::new(OWNER_ONLY_DIRECTORY_DACL)?;
    if unsafe { CreateDirectoryW(path.as_ptr(), security.as_ptr()) } == 0 {
        return Err(last_windows_error());
    }
    Ok(())
}

pub(crate) fn create_private_file(path: &std::path::Path) -> io::Result<File> {
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let security = PrivateSecurityAttributes::new(OWNER_ONLY_DACL)?;
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            FILE_GENERIC_WRITE | READ_CONTROL,
            0,
            security.as_ptr(),
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_windows_error());
    }
    Ok(unsafe { File::from_raw_handle(handle) })
}

pub(crate) struct PipeHandle {
    raw: HANDLE,
}

// Windows kernel handles are process-wide and have no thread affinity. This
// wrapper uniquely owns the handle, so moving it to the server worker is safe.
unsafe impl Send for PipeHandle {}

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
    let security = PrivateSecurityAttributes::new(OWNER_ONLY_DACL).map_err(IpcProtocolError::Io)?;
    let raw = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_INSTANCES,
            PIPE_BUFFER_BYTES,
            PIPE_BUFFER_BYTES,
            0,
            security.as_ptr(),
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

        // Keep another instance ready before returning the current request so
        // consecutive CLI connections do not race the server's rebind.
        let next_pipe = create_server_pipe(pipe_name)?;
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
        pipe = next_pipe;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::windows::io::AsRawHandle, ptr::addr_of};
    use windows_sys::Win32::{
        Foundation::ERROR_SUCCESS,
        Security::Authorization::{
            ConvertStringSidToSidW, GetSecurityInfo, SE_FILE_OBJECT, SE_KERNEL_OBJECT,
        },
        Security::{
            ACCESS_ALLOWED_ACE, ACE_HEADER, ACL_SIZE_INFORMATION, AclSizeInformation,
            DACL_SECURITY_INFORMATION, EqualSid, GetAce, GetAclInformation,
            OWNER_SECURITY_INFORMATION,
        },
        Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        },
        System::SystemServices::ACCESS_ALLOWED_ACE_TYPE,
    };

    struct LocalAllocation(*mut std::ffi::c_void);

    impl Drop for LocalAllocation {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    LocalFree(self.0);
                }
            }
        }
    }

    fn has_owner_only_dacl(handle: HANDLE, object_type: i32) -> io::Result<bool> {
        unsafe {
            let mut owner = null_mut();
            let mut dacl = null_mut();
            let mut descriptor = null_mut();
            let result = GetSecurityInfo(
                handle,
                object_type,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut descriptor,
            );
            if result != ERROR_SUCCESS {
                return Err(io::Error::from_raw_os_error(result as i32));
            }
            let _descriptor = LocalAllocation(descriptor);
            if owner.is_null() || dacl.is_null() {
                return Ok(false);
            }

            let mut size = ACL_SIZE_INFORMATION::default();
            if GetAclInformation(
                dacl,
                (&mut size as *mut ACL_SIZE_INFORMATION).cast(),
                size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            ) == 0
                || size.AceCount != 1
            {
                return Ok(false);
            }

            let mut ace = null_mut();
            if GetAce(dacl, 0, &mut ace) == 0 || ace.is_null() {
                return Ok(false);
            }
            let header = &*ace.cast::<ACE_HEADER>();
            if header.AceType != ACCESS_ALLOWED_ACE_TYPE as u8 {
                return Ok(false);
            }
            let allowed = &*ace.cast::<ACCESS_ALLOWED_ACE>();
            let ace_sid = addr_of!(allowed.SidStart).cast_mut().cast();
            let owner_rights_text: Vec<u16> = "S-1-3-4".encode_utf16().chain(Some(0)).collect();
            let mut owner_rights = null_mut();
            if ConvertStringSidToSidW(owner_rights_text.as_ptr(), &mut owner_rights) == 0 {
                return Err(last_windows_error());
            }
            let _owner_rights = LocalAllocation(owner_rights);
            Ok(EqualSid(ace_sid, owner_rights) != 0)
        }
    }

    fn open_directory_for_security(path: &std::path::Path) -> io::Result<HANDLE> {
        let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                READ_CONTROL,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(last_windows_error())
        } else {
            Ok(handle)
        }
    }

    #[test]
    fn runtime_directory_descriptor_and_named_pipe_have_owner_only_dacls() {
        let directory =
            std::env::temp_dir().join(format!("or-ipc-private-{}", uuid::Uuid::new_v4()));
        create_private_directory(&directory).unwrap();
        let descriptor = directory.join("endpoint.json");
        let file = create_private_file(&descriptor).unwrap();
        let directory_handle = open_directory_for_security(&directory).unwrap();
        let pipe_name = format!(r"\\.\pipe\or-ipc-private-{}", uuid::Uuid::new_v4());
        let pipe = create_server_pipe(&pipe_name).unwrap();

        assert!(has_owner_only_dacl(directory_handle, SE_FILE_OBJECT).unwrap());
        assert!(has_owner_only_dacl(file.as_raw_handle() as HANDLE, SE_FILE_OBJECT).unwrap());
        assert!(has_owner_only_dacl(pipe.raw, SE_KERNEL_OBJECT).unwrap());

        unsafe {
            CloseHandle(directory_handle);
        }
        drop(pipe);
        drop(file);
        fs::remove_dir_all(directory).unwrap();
    }
}
