use sha2::{Digest, Sha256};
use std::{
    error::Error,
    ffi::{OsStr, OsString},
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

/// Version of the cache-key derivation contract.
pub const CACHE_SCHEMA_VERSION: u32 = 1;

const CACHE_ENTRY_EXTENSION: &str = "cache";
const TEMP_FILE_ATTEMPTS: usize = 8;

/// Concrete disposable cache namespaces. These are not job kinds.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CacheArtifactKind {
    Thumbnail,
    Waveform,
}

impl CacheArtifactKind {
    const fn namespace(self) -> &'static str {
        match self {
            Self::Thumbnail => "thumbnail",
            Self::Waveform => "waveform",
        }
    }

    const fn tag(self) -> u8 {
        match self {
            Self::Thumbnail => 1,
            Self::Waveform => 2,
        }
    }
}

/// Opaque fixed-size identity for a media source.
///
/// Phase 5C defines the contract only. Acquiring a production fingerprint from a
/// real media file remains a later checkpoint; no full-file hashing happens here.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceFingerprint([u8; 32]);

impl SourceFingerprint {
    /// Builds a fingerprint from explicitly supplied stable bytes.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(sha256(bytes))
    }

    /// Builds a fingerprint from a caller-supplied digest.
    pub const fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Opaque fixed-size identity for canonical generation parameters.
///
/// Phase 5C does not define thumbnail or waveform parameters yet.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ParametersFingerprint([u8; 32]);

impl ParametersFingerprint {
    /// Builds a fingerprint from explicitly supplied stable bytes.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(sha256(bytes))
    }

    /// Builds a fingerprint from a caller-supplied digest.
    pub const fn from_digest(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

/// Deterministic cache key over schema, artifact kind, source, and parameters.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheKey([u8; 32]);

impl CacheKey {
    pub fn new(
        kind: CacheArtifactKind,
        source: SourceFingerprint,
        parameters: ParametersFingerprint,
    ) -> Self {
        Self::derive(kind, source, parameters, CACHE_SCHEMA_VERSION)
    }

    fn derive(
        kind: CacheArtifactKind,
        source: SourceFingerprint,
        parameters: ParametersFingerprint,
        schema_version: u32,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(schema_version.to_be_bytes());
        hasher.update([kind.tag()]);
        hasher.update(source.digest());
        hasher.update(parameters.digest());
        Self(hasher.finalize().into())
    }

    /// Lowercase hexadecimal representation (64 characters).
    pub fn to_hex(self) -> String {
        hex_lower(&self.0)
    }
}

impl fmt::Display for CacheKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

/// Explicit bounded configuration for a [`CacheStore`]. Both values must be
/// non-zero; there is no unlimited mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CacheStoreConfig {
    max_entry_bytes: u64,
    max_total_bytes: u64,
}

impl CacheStoreConfig {
    pub fn new(max_entry_bytes: u64, max_total_bytes: u64) -> Result<Self, CacheStoreConfigError> {
        if max_entry_bytes == 0 || max_total_bytes == 0 {
            return Err(CacheStoreConfigError);
        }
        Ok(Self {
            max_entry_bytes,
            max_total_bytes,
        })
    }

    pub const fn max_entry_bytes(self) -> u64 {
        self.max_entry_bytes
    }

    pub const fn max_total_bytes(self) -> u64 {
        self.max_total_bytes
    }
}

/// A [`CacheStoreConfig`] value was zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CacheStoreConfigError;

impl fmt::Display for CacheStoreConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("cache store configuration values must be non-zero")
    }
}

impl Error for CacheStoreConfigError {}

/// Failure while reading or writing disposable cache data.
#[derive(Debug)]
pub enum CacheError {
    EntryTooLarge {
        max_bytes: u64,
        actual_bytes: u64,
    },
    BudgetExceeded {
        max_total_bytes: u64,
        projected_total_bytes: u64,
    },
    CorruptOrOversizedEntry {
        max_bytes: u64,
    },
    Io(io::Error),
}

impl fmt::Display for CacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntryTooLarge {
                max_bytes,
                actual_bytes,
            } => write!(
                formatter,
                "cache entry of {actual_bytes} bytes exceeds the {max_bytes}-byte limit"
            ),
            Self::BudgetExceeded {
                max_total_bytes,
                projected_total_bytes,
            } => write!(
                formatter,
                "cache write would raise the store to {projected_total_bytes} bytes, over the {max_total_bytes}-byte budget"
            ),
            Self::CorruptOrOversizedEntry { max_bytes } => {
                write!(
                    formatter,
                    "cache entry is corrupt or exceeds the {max_bytes}-byte limit"
                )
            }
            Self::Io(error) => write!(formatter, "cache I/O failed: {error}"),
        }
    }
}

impl Error for CacheError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::EntryTooLarge { .. }
            | Self::BudgetExceeded { .. }
            | Self::CorruptOrOversizedEntry { .. } => None,
        }
    }
}

impl From<io::Error> for CacheError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// A disposable filesystem cache store for opaque bounded artifacts.
///
/// The store is not canonical project state. Deleting its contents must never
/// damage a project. Paths are derived only from internally generated typed keys.
#[derive(Clone, Debug)]
pub struct CacheStore {
    root: PathBuf,
    config: CacheStoreConfig,
}

impl CacheStore {
    /// Creates a store rooted at an explicit caller-provided directory.
    pub fn new(root: impl Into<PathBuf>, config: CacheStoreConfig) -> Self {
        Self {
            root: root.into(),
            config,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub const fn config(&self) -> CacheStoreConfig {
        self.config
    }

    /// Atomically installs opaque bytes for a typed key.
    pub fn put(
        &self,
        kind: CacheArtifactKind,
        key: CacheKey,
        bytes: &[u8],
    ) -> Result<(), CacheError> {
        let entry_bytes = bytes.len() as u64;
        if entry_bytes > self.config.max_entry_bytes() {
            return Err(CacheError::EntryTooLarge {
                max_bytes: self.config.max_entry_bytes(),
                actual_bytes: entry_bytes,
            });
        }

        let path = self.entry_path(kind, key);
        let existing = file_len(&path)?;
        let current_total = self.total_bytes()?;
        let projected_total = current_total
            .saturating_sub(existing.unwrap_or(0))
            .saturating_add(entry_bytes);
        if projected_total > self.config.max_total_bytes() {
            return Err(CacheError::BudgetExceeded {
                max_total_bytes: self.config.max_total_bytes(),
                projected_total_bytes: projected_total,
            });
        }

        atomic_write(&path, bytes)
    }

    /// Reads a bounded entry, or `Ok(None)` for a normal cache miss.
    pub fn get(
        &self,
        kind: CacheArtifactKind,
        key: CacheKey,
    ) -> Result<Option<Vec<u8>>, CacheError> {
        let path = self.entry_path(kind, key);
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(CacheError::Io(error)),
        };

        let metadata = file.metadata().map_err(CacheError::Io)?;
        if metadata.len() > self.config.max_entry_bytes() {
            return Err(CacheError::CorruptOrOversizedEntry {
                max_bytes: self.config.max_entry_bytes(),
            });
        }

        let mut bytes = Vec::new();
        file.take(self.config.max_entry_bytes() + 1)
            .read_to_end(&mut bytes)
            .map_err(CacheError::Io)?;
        if bytes.len() as u64 > self.config.max_entry_bytes() {
            return Err(CacheError::CorruptOrOversizedEntry {
                max_bytes: self.config.max_entry_bytes(),
            });
        }
        Ok(Some(bytes))
    }

    /// Removes one exact key. Missing entries are an idempotent no-op.
    pub fn remove(&self, kind: CacheArtifactKind, key: CacheKey) -> Result<bool, CacheError> {
        match fs::remove_file(self.entry_path(kind, key)) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(CacheError::Io(error)),
        }
    }

    /// Removes one managed namespace.
    pub fn clear_namespace(&self, kind: CacheArtifactKind) -> Result<(), CacheError> {
        remove_dir_if_present(&self.root.join(kind.namespace()))
    }

    /// Removes every managed namespace. The store remains usable afterward.
    pub fn clear_all(&self) -> Result<(), CacheError> {
        self.clear_namespace(CacheArtifactKind::Thumbnail)?;
        self.clear_namespace(CacheArtifactKind::Waveform)
    }

    fn entry_path(&self, kind: CacheArtifactKind, key: CacheKey) -> PathBuf {
        let hex = key.to_hex();
        let prefix = &hex[..2];
        self.root
            .join(kind.namespace())
            .join(prefix)
            .join(format!("{hex}.{CACHE_ENTRY_EXTENSION}"))
    }

    fn total_bytes(&self) -> Result<u64, CacheError> {
        if !self.root.exists() {
            return Ok(0);
        }

        let mut total = 0u64;
        let mut stack = vec![self.root.clone()];
        while let Some(directory) = stack.pop() {
            let entries = match fs::read_dir(&directory) {
                Ok(entries) => entries,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(CacheError::Io(error)),
            };
            for entry in entries {
                let entry = entry.map_err(CacheError::Io)?;
                let file_type = entry.file_type().map_err(CacheError::Io)?;
                if file_type.is_dir() {
                    stack.push(entry.path());
                } else if file_type.is_file() {
                    total = total.saturating_add(entry.metadata().map_err(CacheError::Io)?.len());
                }
            }
        }
        Ok(total)
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn file_len(path: &Path) -> Result<Option<u64>, CacheError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(Some(metadata.len())),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(CacheError::Io(error)),
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CacheError> {
    let parent = path.parent().ok_or_else(|| {
        CacheError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cache entry path must have a parent directory",
        ))
    })?;
    fs::create_dir_all(parent).map_err(CacheError::Io)?;

    let file_name = path.file_name().ok_or_else(|| {
        CacheError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cache entry path must name a file",
        ))
    })?;
    let (temporary_path, file) =
        create_temporary_file(parent, file_name).map_err(CacheError::Io)?;

    if let Err(error) = write_and_sync(file, bytes) {
        let _ = fs::remove_file(&temporary_path);
        return Err(CacheError::Io(error));
    }
    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(CacheError::Io(error));
    }
    Ok(())
}

fn create_temporary_file(parent: &Path, target_name: &OsStr) -> io::Result<(PathBuf, File)> {
    let mut last_collision = None;
    for _ in 0..TEMP_FILE_ATTEMPTS {
        let mut name = OsString::from(".");
        name.push(target_name);
        name.push(format!(".or-cache-tmp-{}", Uuid::new_v4()));
        let path = parent.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                last_collision = Some(error);
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_collision
        .unwrap_or_else(|| io::Error::other("cache temporary file attempts were exhausted")))
}

fn write_and_sync(file: File, bytes: &[u8]) -> io::Result<()> {
    let mut writer = BufWriter::new(file);
    writer.write_all(bytes)?;
    writer.flush()?;
    writer.get_ref().sync_all()
}

fn remove_dir_if_present(directory: &Path) -> Result<(), CacheError> {
    match fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            fs::remove_file(directory).map_err(CacheError::Io)
        }
        Ok(_) => fs::remove_dir_all(directory).map_err(CacheError::Io),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(CacheError::Io(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JobKind, JobManager, JobManagerConfig, JobState, ProjectDocument, ProjectSession};
    use std::{
        path::Path,
        sync::{Arc, Barrier},
        thread,
        time::{Duration, Instant},
    };

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("or-core-cache-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn key(kind: CacheArtifactKind, source: &[u8], parameters: &[u8]) -> CacheKey {
        CacheKey::new(
            kind,
            SourceFingerprint::from_bytes(source),
            ParametersFingerprint::from_bytes(parameters),
        )
    }

    fn store(directory: &TestDirectory, max_entry: u64, max_total: u64) -> CacheStore {
        CacheStore::new(
            directory.path(),
            CacheStoreConfig::new(max_entry, max_total).unwrap(),
        )
    }

    fn wait_for_state(manager: &JobManager, id: crate::JobId, state: JobState) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while manager.snapshot(id).map(|snapshot| snapshot.state) != Some(state) {
            assert!(
                Instant::now() < deadline,
                "job did not reach the expected state before the deadlock guard timeout"
            );
            thread::yield_now();
        }
    }

    #[test]
    fn cache_key_is_deterministic() {
        let first = key(CacheArtifactKind::Thumbnail, b"media-1", b"params-1");
        let second = key(CacheArtifactKind::Thumbnail, b"media-1", b"params-1");
        assert_eq!(first, second);
        assert_eq!(first.to_hex(), second.to_hex());
    }

    #[test]
    fn cache_key_separates_kind_source_parameters_and_schema() {
        let base = key(CacheArtifactKind::Thumbnail, b"media-1", b"params-1");
        assert_ne!(
            base,
            key(CacheArtifactKind::Waveform, b"media-1", b"params-1")
        );
        assert_ne!(
            base,
            key(CacheArtifactKind::Thumbnail, b"media-2", b"params-1")
        );
        assert_ne!(
            base,
            key(CacheArtifactKind::Thumbnail, b"media-1", b"params-2")
        );

        let next_schema = CacheKey::derive(
            CacheArtifactKind::Thumbnail,
            SourceFingerprint::from_bytes(b"media-1"),
            ParametersFingerprint::from_bytes(b"params-1"),
            CACHE_SCHEMA_VERSION + 1,
        );
        assert_ne!(base, next_schema);
    }

    #[test]
    fn cache_key_hex_is_a_safe_path_segment() {
        let hex = key(CacheArtifactKind::Waveform, b"m", b"p").to_hex();
        assert_eq!(hex.len(), 64);
        assert!(
            hex.bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        assert!(!hex.contains('/') && !hex.contains('\\') && !hex.contains(".."));
    }

    #[test]
    fn cache_round_trips_exact_bytes() {
        let directory = TestDirectory::new();
        let store = store(&directory, 1024, 4096);
        let key = key(CacheArtifactKind::Thumbnail, b"m", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, key, b"artifact-bytes")
            .unwrap();
        assert_eq!(
            store
                .get(CacheArtifactKind::Thumbnail, key)
                .unwrap()
                .as_deref(),
            Some(&b"artifact-bytes"[..])
        );
    }

    #[test]
    fn missing_entry_is_a_normal_miss() {
        let directory = TestDirectory::new();
        let store = store(&directory, 1024, 4096);
        let key = key(CacheArtifactKind::Thumbnail, b"absent", b"p");
        assert_eq!(store.get(CacheArtifactKind::Thumbnail, key).unwrap(), None);
    }

    #[test]
    fn oversized_entry_is_rejected_before_creation() {
        let directory = TestDirectory::new();
        let store = store(&directory, 4, 4096);
        let key = key(CacheArtifactKind::Thumbnail, b"m", b"p");
        assert!(matches!(
            store.put(CacheArtifactKind::Thumbnail, key, b"12345"),
            Err(CacheError::EntryTooLarge { .. })
        ));
        assert!(!store.entry_path(CacheArtifactKind::Thumbnail, key).exists());
    }

    #[test]
    fn externally_oversized_entry_reads_as_corrupt() {
        let directory = TestDirectory::new();
        let store = store(&directory, 4, 4096);
        let key = key(CacheArtifactKind::Waveform, b"m", b"p");
        let path = store.entry_path(CacheArtifactKind::Waveform, key);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"0123456789").unwrap();
        assert!(matches!(
            store.get(CacheArtifactKind::Waveform, key),
            Err(CacheError::CorruptOrOversizedEntry { .. })
        ));
    }

    #[test]
    fn total_budget_is_enforced_without_touching_existing_entries() {
        let directory = TestDirectory::new();
        let store = store(&directory, 100, 100);
        let first = key(CacheArtifactKind::Thumbnail, b"a", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, first, &[0u8; 60])
            .unwrap();

        let second = key(CacheArtifactKind::Thumbnail, b"b", b"p");
        assert!(matches!(
            store.put(CacheArtifactKind::Thumbnail, second, &[0u8; 60]),
            Err(CacheError::BudgetExceeded { .. })
        ));
        assert_eq!(
            store
                .get(CacheArtifactKind::Thumbnail, first)
                .unwrap()
                .map(|b| b.len()),
            Some(60)
        );
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, second).unwrap(),
            None
        );
    }

    #[test]
    fn replacing_a_key_accounts_for_its_existing_size() {
        let directory = TestDirectory::new();
        let store = store(&directory, 100, 100);
        let key = key(CacheArtifactKind::Thumbnail, b"a", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, key, &[1u8; 60])
            .unwrap();
        store
            .put(CacheArtifactKind::Thumbnail, key, &[2u8; 80])
            .unwrap();
        assert_eq!(
            store
                .get(CacheArtifactKind::Thumbnail, key)
                .unwrap()
                .map(|b| b.len()),
            Some(80)
        );
    }

    #[test]
    fn removing_an_entry_is_idempotent() {
        let directory = TestDirectory::new();
        let store = store(&directory, 1024, 4096);
        let key = key(CacheArtifactKind::Thumbnail, b"m", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, key, b"data")
            .unwrap();
        assert!(store.remove(CacheArtifactKind::Thumbnail, key).unwrap());
        assert_eq!(store.get(CacheArtifactKind::Thumbnail, key).unwrap(), None);
        assert!(!store.remove(CacheArtifactKind::Thumbnail, key).unwrap());
    }

    #[test]
    fn clearing_one_namespace_preserves_the_other() {
        let directory = TestDirectory::new();
        let store = store(&directory, 1024, 4096);
        let thumbnail = key(CacheArtifactKind::Thumbnail, b"m", b"p");
        let waveform = key(CacheArtifactKind::Waveform, b"m", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, thumbnail, b"t")
            .unwrap();
        store
            .put(CacheArtifactKind::Waveform, waveform, b"w")
            .unwrap();

        store.clear_namespace(CacheArtifactKind::Thumbnail).unwrap();
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, thumbnail).unwrap(),
            None
        );
        assert_eq!(
            store
                .get(CacheArtifactKind::Waveform, waveform)
                .unwrap()
                .as_deref(),
            Some(&b"w"[..])
        );
    }

    #[test]
    fn clearing_all_leaves_the_store_usable() {
        let directory = TestDirectory::new();
        let store = store(&directory, 1024, 4096);
        let thumbnail = key(CacheArtifactKind::Thumbnail, b"m", b"p");
        let waveform = key(CacheArtifactKind::Waveform, b"m", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, thumbnail, b"t")
            .unwrap();
        store
            .put(CacheArtifactKind::Waveform, waveform, b"w")
            .unwrap();

        store.clear_all().unwrap();
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, thumbnail).unwrap(),
            None
        );
        assert_eq!(
            store.get(CacheArtifactKind::Waveform, waveform).unwrap(),
            None
        );

        store
            .put(CacheArtifactKind::Thumbnail, thumbnail, b"t2")
            .unwrap();
        assert_eq!(
            store
                .get(CacheArtifactKind::Thumbnail, thumbnail)
                .unwrap()
                .as_deref(),
            Some(&b"t2"[..])
        );
    }

    #[test]
    fn failed_replacement_leaves_no_partial_entry() {
        let directory = TestDirectory::new();
        let store = store(&directory, 1024, 4096);
        let key = key(CacheArtifactKind::Thumbnail, b"m", b"p");
        let path = store.entry_path(CacheArtifactKind::Thumbnail, key);
        fs::create_dir_all(&path).unwrap();

        assert!(
            store
                .put(CacheArtifactKind::Thumbnail, key, b"payload")
                .is_err()
        );
        assert!(path.is_dir());

        let leftovers: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(leftovers.len(), 1);
        assert!(
            !leftovers
                .iter()
                .any(|name| name.to_string_lossy().contains("or-cache-tmp"))
        );
    }

    #[test]
    fn concurrent_same_key_writes_produce_one_complete_payload() {
        let directory = TestDirectory::new();
        let store = Arc::new(store(&directory, 4096, 8192));
        let key = key(CacheArtifactKind::Thumbnail, b"m", b"p");
        let barrier = Arc::new(Barrier::new(3));
        let payload_a = vec![0xAAu8; 1000];
        let payload_b = vec![0xBBu8; 1000];

        let mut handles = Vec::new();
        for payload in [payload_a.clone(), payload_b.clone()] {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                store
                    .put(CacheArtifactKind::Thumbnail, key, &payload)
                    .unwrap();
            }));
        }
        barrier.wait();
        for handle in handles {
            handle.join().unwrap();
        }

        let bytes = store
            .get(CacheArtifactKind::Thumbnail, key)
            .unwrap()
            .unwrap();
        assert!(bytes == payload_a || bytes == payload_b);
    }

    #[test]
    fn unicode_and_spaced_root_paths_work() {
        let base = TestDirectory::new();
        let root = base.path().join("or cache √ dir");
        let store = CacheStore::new(&root, CacheStoreConfig::new(1024, 4096).unwrap());
        let key = key(CacheArtifactKind::Waveform, b"m", b"p");
        store
            .put(CacheArtifactKind::Waveform, key, "payload-√".as_bytes())
            .unwrap();
        assert_eq!(
            store
                .get(CacheArtifactKind::Waveform, key)
                .unwrap()
                .as_deref(),
            Some("payload-√".as_bytes())
        );
        assert!(store.remove(CacheArtifactKind::Waveform, key).unwrap());
    }

    #[test]
    fn job_and_cache_work_does_not_touch_project_state() {
        let directory = TestDirectory::new();
        let session = ProjectSession::open(ProjectDocument::new("Scene"));
        let revision_before = session.project_revision();

        let store = store(&directory, 1024, 4096);
        let key = key(CacheArtifactKind::Thumbnail, b"m", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, key, b"artifact")
            .unwrap();
        assert!(
            store
                .get(CacheArtifactKind::Thumbnail, key)
                .unwrap()
                .is_some()
        );

        let manager = JobManager::new(JobManagerConfig::new(1, 4, 8).unwrap());
        let id = manager.submit(JobKind::MediaProbe, |_| Ok(())).unwrap();
        wait_for_state(&manager, id, JobState::Succeeded);
        manager.shutdown();

        assert_eq!(session.project_revision(), revision_before);
        assert!(!contains_project_file(directory.path()));
    }

    fn contains_project_file(root: &Path) -> bool {
        let mut stack = vec![root.to_path_buf()];
        while let Some(directory) = stack.pop() {
            for entry in fs::read_dir(&directory).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    stack.push(entry.path());
                } else if entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "orproj")
                {
                    return true;
                }
            }
        }
        false
    }
}
