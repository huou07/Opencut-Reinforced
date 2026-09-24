use crate::{ProjectCodecError, ProjectDocument, decode_project, encode_project};
use std::{
    error::Error,
    ffi::OsString,
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

/// Maximum encoded project size accepted by the filesystem storage boundary.
pub const MAX_PROJECT_FILE_BYTES: u64 = 64 * 1024 * 1024;
const TEMP_FILE_ATTEMPTS: usize = 8;

/// Reads and validates a bounded `.orproj` document.
pub fn load_project_file(path: impl AsRef<Path>) -> Result<ProjectDocument, ProjectStorageError> {
    let file = File::open(path).map_err(ProjectStorageError::Io)?;
    if file.metadata().map_err(ProjectStorageError::Io)?.len() > MAX_PROJECT_FILE_BYTES {
        return Err(ProjectStorageError::TooLarge {
            max_bytes: MAX_PROJECT_FILE_BYTES,
        });
    }

    let mut bytes = Vec::new();
    file.take(MAX_PROJECT_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(ProjectStorageError::Io)?;
    if bytes.len() as u64 > MAX_PROJECT_FILE_BYTES {
        return Err(ProjectStorageError::TooLarge {
            max_bytes: MAX_PROJECT_FILE_BYTES,
        });
    }

    let encoded = std::str::from_utf8(&bytes).map_err(|_| ProjectStorageError::InvalidUtf8)?;
    decode_project(encoded).map_err(ProjectStorageError::Codec)
}

/// Writes canonical project state using a same-directory temporary file and atomic replace.
///
/// Success is reported only after the project file and its parent directory have reached
/// the platform's durability boundary. If that final boundary fails after replacement,
/// `DurabilityUncertain` reports that the new destination may already be present.
pub fn save_project_file_atomic(
    path: impl AsRef<Path>,
    document: &ProjectDocument,
) -> Result<(), ProjectStorageError> {
    let encoded = encode_project(document).map_err(ProjectStorageError::Codec)?;
    if encoded.len() as u64 > MAX_PROJECT_FILE_BYTES {
        return Err(ProjectStorageError::TooLarge {
            max_bytes: MAX_PROJECT_FILE_BYTES,
        });
    }

    let path = path.as_ref();
    let target_name = path.file_name().ok_or_else(|| {
        ProjectStorageError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "project path must name a file",
        ))
    })?;
    let parent = parent_directory(path);
    let (temporary_path, file) = create_temporary_file(parent, target_name)?;

    let write_result = write_temporary_file(file, encoded.as_bytes());
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    if let Err(error) = replace_file(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(ProjectStorageError::Replace(error));
    }

    sync_parent(parent).map_err(ProjectStorageError::DurabilityUncertain)
}

/// Failures while reading, validating, or durably replacing a project file.
#[derive(Debug)]
pub enum ProjectStorageError {
    Io(io::Error),
    TooLarge {
        max_bytes: u64,
    },
    InvalidUtf8,
    Codec(ProjectCodecError),
    TemporaryFile {
        operation: TempFileOperation,
        source: io::Error,
    },
    Replace(io::Error),
    DurabilityUncertain(io::Error),
}

impl fmt::Display for ProjectStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "project file I/O failed: {error}"),
            Self::TooLarge { max_bytes } => {
                write!(formatter, "project file exceeds the {max_bytes}-byte limit")
            }
            Self::InvalidUtf8 => formatter.write_str("project file is not valid UTF-8"),
            Self::Codec(error) => write!(formatter, "project document is invalid: {error}"),
            Self::TemporaryFile { operation, source } => {
                write!(
                    formatter,
                    "temporary project file {operation} failed: {source}"
                )
            }
            Self::Replace(error) => write!(formatter, "project file replacement failed: {error}"),
            Self::DurabilityUncertain(error) => write!(
                formatter,
                "project file was replaced, but its directory could not be synced: {error}"
            ),
        }
    }
}

impl Error for ProjectStorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error)
            | Self::Replace(error)
            | Self::DurabilityUncertain(error)
            | Self::TemporaryFile { source: error, .. } => Some(error),
            Self::Codec(error) => Some(error),
            Self::TooLarge { .. } | Self::InvalidUtf8 => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TempFileOperation {
    Create,
    Write,
    Flush,
    Sync,
}

impl fmt::Display for TempFileOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Create => "creation",
            Self::Write => "write",
            Self::Flush => "flush",
            Self::Sync => "sync",
        })
    }
}

fn parent_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn create_temporary_file(
    parent: &Path,
    target_name: &std::ffi::OsStr,
) -> Result<(PathBuf, File), ProjectStorageError> {
    let mut last_collision = None;
    for _ in 0..TEMP_FILE_ATTEMPTS {
        let mut temporary_name = OsString::from(".");
        temporary_name.push(target_name);
        temporary_name.push(format!(".or-tmp-{}", Uuid::new_v4()));
        let temporary_path = parent.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((temporary_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                last_collision = Some(error);
            }
            Err(source) => {
                return Err(ProjectStorageError::TemporaryFile {
                    operation: TempFileOperation::Create,
                    source,
                });
            }
        }
    }

    Err(ProjectStorageError::TemporaryFile {
        operation: TempFileOperation::Create,
        source: last_collision
            .unwrap_or_else(|| io::Error::other("temporary project file attempts were exhausted")),
    })
}

fn write_temporary_file(file: File, contents: &[u8]) -> Result<(), ProjectStorageError> {
    let mut writer = BufWriter::new(file);
    writer
        .write_all(contents)
        .map_err(|source| ProjectStorageError::TemporaryFile {
            operation: TempFileOperation::Write,
            source,
        })?;
    writer
        .flush()
        .map_err(|source| ProjectStorageError::TemporaryFile {
            operation: TempFileOperation::Flush,
            source,
        })?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|source| ProjectStorageError::TemporaryFile {
            operation: TempFileOperation::Sync,
            source,
        })
}

#[cfg(unix)]
fn replace_file(temporary_path: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary_path, destination)
}

#[cfg(windows)]
fn replace_file(temporary_path: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    fn wide_path(path: &Path) -> io::Result<Vec<u16>> {
        let mut wide: Vec<_> = path.as_os_str().encode_wide().collect();
        if wide.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "project path contains a null character",
            ));
        }
        wide.push(0);
        Ok(wide)
    }

    let source = wide_path(temporary_path)?;
    let destination = wide_path(destination)?;
    // Both names are same-directory siblings; replacement is atomic and requests write-through.
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
fn replace_file(_temporary_path: &Path, _destination: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic project replacement is unsupported on this platform",
    ))
}

#[cfg(unix)]
fn sync_parent(parent: &Path) -> io::Result<()> {
    File::open(parent)?.sync_all()
}

#[cfg(windows)]
fn sync_parent(_parent: &Path) -> io::Result<()> {
    // MoveFileExW was called with MOVEFILE_WRITE_THROUGH during replacement.
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn sync_parent(_parent: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "project directory durability sync is unsupported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::{create_temporary_file, parent_directory, replace_file};
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("or-core-storage-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn temporary_file_is_created_as_a_sibling_of_the_destination() {
        let directory = TestDirectory::new();
        let destination = directory.0.join("example.orproj");
        let (temporary_path, file) = create_temporary_file(
            parent_directory(&destination),
            destination.file_name().unwrap(),
        )
        .unwrap();

        assert_eq!(temporary_path.parent(), destination.parent());
        assert!(temporary_path.exists());
        drop(file);
        fs::remove_file(temporary_path).unwrap();
    }

    #[test]
    fn failed_platform_replacement_does_not_remove_the_destination() {
        let directory = TestDirectory::new();
        let source = directory.0.join("missing-temp");
        let destination = directory.0.join("existing.orproj");
        fs::write(&destination, b"preserve this").unwrap();

        assert!(replace_file(&source, &destination).is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"preserve this");
    }
}
