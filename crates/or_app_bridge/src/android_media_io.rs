use or_core::{AndroidSafDocumentUri, MediaSourceRef};
use or_media::SeekableMediaIoCapability;
use std::{
    collections::HashMap,
    ffi::{CStr, c_char},
    fs::File,
    os::fd::BorrowedFd,
    sync::{Mutex, MutexGuard, OnceLock},
};

const MAX_MEDIA_SOURCES: usize = 64;

static MEDIA_IO: OnceLock<Mutex<HashMap<String, SeekableMediaIoCapability>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, SeekableMediaIoCapability>> {
    MEDIA_IO.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_registry() -> MutexGuard<'static, HashMap<String, SeekableMediaIoCapability>> {
    registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn capability_for(source: &MediaSourceRef) -> Option<SeekableMediaIoCapability> {
    let MediaSourceRef::AndroidSafDocumentUri { uri } = source else {
        return None;
    };
    lock_registry().get(uri.as_str()).cloned()
}

/// Registers an Android-owned duplicate of a SAF descriptor. The descriptor
/// itself and its seekable capability stay in process memory only.
#[cfg_attr(target_os = "android", unsafe(no_mangle))]
pub extern "C" fn or_media_register_seekable_fd(uri: *const c_char, fd: i32) -> bool {
    if uri.is_null() || fd < 0 {
        return false;
    }
    let Ok(uri) = unsafe { CStr::from_ptr(uri) }.to_str() else {
        return false;
    };
    let Ok(uri) = AndroidSafDocumentUri::parse(uri) else {
        return false;
    };
    // The caller keeps its descriptor open through this call. Duplicate it so
    // closing the provider descriptor cannot invalidate FFmpeg's later reads.
    let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
    let Ok(owned) = borrowed.try_clone_to_owned() else {
        return false;
    };
    let file = File::from(owned);
    let Ok(capability) = SeekableMediaIoCapability::from_file(file) else {
        return false;
    };

    let mut media_io = lock_registry();
    if !media_io.contains_key(uri.as_str()) && media_io.len() >= MAX_MEDIA_SOURCES {
        return false;
    }
    media_io.insert(uri.as_str().to_owned(), capability);
    true
}

#[cfg_attr(target_os = "android", unsafe(no_mangle))]
pub extern "C" fn or_media_clear_seekable_fds() {
    lock_registry().clear();
}

#[cfg_attr(target_os = "android", unsafe(no_mangle))]
pub extern "C" fn or_media_seekable_fd_count() -> usize {
    lock_registry().len()
}

#[cfg(all(test, unix))]
mod tests {
    use super::{
        AndroidSafDocumentUri, File, MediaSourceRef, capability_for, lock_registry,
        or_media_clear_seekable_fds, or_media_register_seekable_fd, or_media_seekable_fd_count,
    };
    use std::{
        ffi::CString,
        io::Write,
        os::fd::AsRawFd,
        os::unix::net::UnixStream,
        path::PathBuf,
        sync::{Mutex, MutexGuard},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn lock_tests() -> MutexGuard<'static, ()> {
        TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn uri(value: &str) -> (CString, MediaSourceRef) {
        (
            CString::new(value).unwrap(),
            MediaSourceRef::AndroidSafDocumentUri {
                uri: AndroidSafDocumentUri::parse(value).unwrap(),
            },
        )
    }

    fn media_file() -> (PathBuf, File) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "or-android-media-io-{}-{stamp}.mkv",
            std::process::id()
        ));
        let mut file = File::create(&path).unwrap();
        file.write_all(b"seekable-media-fixture").unwrap();
        (path, file)
    }

    #[test]
    fn duplicated_saf_descriptor_outlives_the_provider_descriptor() {
        let _test = lock_tests();
        or_media_clear_seekable_fds();
        let (path, file) = media_file();
        let (uri, source) = uri("content://com.example.provider/document/clip-1");

        assert!(or_media_register_seekable_fd(
            uri.as_ptr(),
            file.as_raw_fd()
        ));
        drop(file);
        assert!(capability_for(&source).is_some());

        or_media_clear_seekable_fds();
        assert!(capability_for(&source).is_none());
        assert_eq!(or_media_seekable_fd_count(), 0);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn nonseekable_saf_descriptor_is_rejected() {
        let _test = lock_tests();
        or_media_clear_seekable_fds();
        let (reader, _writer) = UnixStream::pair().unwrap();
        let (uri, source) = uri("content://com.example.provider/document/pipe-1");

        assert!(!or_media_register_seekable_fd(
            uri.as_ptr(),
            reader.as_raw_fd()
        ));
        assert!(capability_for(&source).is_none());
        assert!(lock_registry().is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn clear_and_final_capability_drop_close_the_actual_duplicate_fd() {
        let _test = lock_tests();
        or_media_clear_seekable_fds();
        let (path, file) = media_file();
        let (uri, source) = uri("content://com.example.provider/document/release");
        let count = || {
            std::fs::read_dir("/proc/self/fd")
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| std::fs::read_link(entry.path()).is_ok_and(|target| target == path))
                .count()
        };
        assert_eq!(count(), 1);
        assert!(or_media_register_seekable_fd(
            uri.as_ptr(),
            file.as_raw_fd()
        ));
        assert_eq!(count(), 2);
        let capability = capability_for(&source).unwrap();
        drop(file);
        or_media_clear_seekable_fds();
        assert_eq!(or_media_seekable_fd_count(), 0);
        assert_eq!(
            count(),
            1,
            "A decoder capability owns the live duplicate after clear"
        );
        drop(capability);
        assert_eq!(
            count(),
            0,
            "The last capability must close the OS descriptor"
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn descriptor_registry_rejects_more_than_its_source_budget() {
        let _test = lock_tests();
        or_media_clear_seekable_fds();
        let (path, file) = media_file();
        for index in 0..64 {
            let uri = CString::new(format!(
                "content://com.example.provider/document/clip-{index}"
            ))
            .unwrap();
            assert!(or_media_register_seekable_fd(
                uri.as_ptr(),
                file.as_raw_fd()
            ));
        }
        let extra = CString::new("content://com.example.provider/document/clip-extra").unwrap();

        assert!(!or_media_register_seekable_fd(
            extra.as_ptr(),
            file.as_raw_fd()
        ));
        assert_eq!(lock_registry().len(), 64);
        assert_eq!(or_media_seekable_fd_count(), 64);
        or_media_clear_seekable_fds();
        std::fs::remove_file(path).unwrap();
    }
}
