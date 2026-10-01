use rusqlite::{Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    ffi::{OsStr, OsString},
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

mod index;

/// Version of the cache-key derivation contract.
pub const CACHE_SCHEMA_VERSION: u32 = 1;

/// Version of the disposable SQLite cache-index schema.
pub const CACHE_INDEX_SCHEMA_VERSION: u32 = 1;

const CACHE_ENTRY_EXTENSION: &str = "cache";
const TEMP_FILE_ATTEMPTS: usize = 8;
pub(crate) const PROXY_MAX_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Concrete disposable cache namespaces. These are not job kinds.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CacheArtifactKind {
    Thumbnail,
    Waveform,
    Proxy,
}

impl CacheArtifactKind {
    const fn namespace(self) -> &'static str {
        match self {
            Self::Thumbnail => "thumbnail",
            Self::Waveform => "waveform",
            Self::Proxy => "proxy",
        }
    }

    const fn extension(self) -> &'static str {
        match self {
            Self::Proxy => "mkv",
            Self::Thumbnail | Self::Waveform => CACHE_ENTRY_EXTENSION,
        }
    }

    const fn tag(self) -> u8 {
        match self {
            Self::Thumbnail => 1,
            Self::Waveform => 2,
            Self::Proxy => 3,
        }
    }

    pub(super) fn from_namespace(namespace: &str) -> Option<Self> {
        match namespace {
            "thumbnail" => Some(Self::Thumbnail),
            "waveform" => Some(Self::Waveform),
            "proxy" => Some(Self::Proxy),
            _ => None,
        }
    }
}

/// Opaque fixed-size cache-invalidation fingerprint for a media source.
///
/// Phase 5D acquires this from bounded file metadata and content samples. It is
/// not a full-file identity or an integrity proof.
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

    /// Parses the canonical lowercase, 64-character cache-key form.
    pub fn from_hex(value: &str) -> Result<Self, CacheKeyParseError> {
        if value.len() != 64 {
            return Err(CacheKeyParseError::InvalidLength);
        }
        let bytes = value.as_bytes();
        let mut digest = [0_u8; 32];
        for (index, pair) in bytes.as_chunks::<2>().0.iter().enumerate() {
            let high = lower_hex_nibble(pair[0]).ok_or(CacheKeyParseError::InvalidCharacter)?;
            let low = lower_hex_nibble(pair[1]).ok_or(CacheKeyParseError::InvalidCharacter)?;
            digest[index] = (high << 4) | low;
        }
        Ok(Self(digest))
    }
}

impl FromStr for CacheKey {
    type Err = CacheKeyParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_hex(value)
    }
}

impl fmt::Display for CacheKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheKeyParseError {
    InvalidLength,
    InvalidCharacter,
}

impl fmt::Display for CacheKeyParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLength => {
                "cache key must contain exactly 64 lowercase hexadecimal characters"
            }
            Self::InvalidCharacter => "cache key contains a non-canonical hexadecimal character",
        })
    }
}

impl Error for CacheKeyParseError {}

fn lower_hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
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
    IndexTooLarge {
        max_entries: usize,
    },
    IndexSequenceExhausted,
    FileBackedApiRequired,
    InvalidFileArtifact,
    Index(String),
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
            Self::IndexTooLarge { max_entries } => write!(
                formatter,
                "cache index contains more than {max_entries} managed artifacts"
            ),
            Self::IndexSequenceExhausted => {
                formatter.write_str("cache access sequence is exhausted")
            }
            Self::FileBackedApiRequired => {
                formatter.write_str("proxy artifacts require the file-backed cache API")
            }
            Self::InvalidFileArtifact => {
                formatter.write_str("file-backed cache artifact is empty or not a regular file")
            }
            Self::Index(message) => write!(formatter, "cache index failed: {message}"),
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
            | Self::CorruptOrOversizedEntry { .. }
            | Self::IndexTooLarge { .. }
            | Self::IndexSequenceExhausted
            | Self::FileBackedApiRequired
            | Self::InvalidFileArtifact
            | Self::Index(_) => None,
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
    index: Arc<Mutex<index::IndexState>>,
}

/// Reserved same-directory staging path for one typed proxy key.
pub(crate) struct CacheStagingFile {
    key: CacheKey,
    path: PathBuf,
    max_bytes: u64,
}

impl CacheStagingFile {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) const fn max_bytes(&self) -> u64 {
        self.max_bytes
    }
}

impl Drop for CacheStagingFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl CacheStore {
    /// Creates a store rooted at an explicit caller-provided directory.
    pub fn new(root: impl Into<PathBuf>, config: CacheStoreConfig) -> Self {
        Self {
            root: root.into(),
            config,
            index: Arc::new(Mutex::new(index::IndexState::default())),
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
        if kind == CacheArtifactKind::Proxy {
            return Err(CacheError::FileBackedApiRequired);
        }
        let entry_bytes = bytes.len() as u64;
        if entry_bytes > self.config.max_entry_bytes() {
            return Err(CacheError::EntryTooLarge {
                max_bytes: self.config.max_entry_bytes(),
                actual_bytes: entry_bytes,
            });
        }
        if entry_bytes > self.config.max_total_bytes() {
            return Err(CacheError::BudgetExceeded {
                max_total_bytes: self.config.max_total_bytes(),
                projected_total_bytes: entry_bytes,
            });
        }

        let mut state = lock_index(&self.index);
        let transaction = state
            .connection(&self.root)?
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(index::index_error)?;
        let mut filesystem_changed = false;
        let result = (|| {
            let path = self.entry_path(kind, key);
            let target = entry_file_state(&self.root, kind, key)?;
            let target_size = match target {
                EntryFileState::File(size) => {
                    let sequence = index::entry_sequence(&transaction, kind, key)?.unwrap_or(0);
                    index::upsert_entry(&transaction, kind, key, size, sequence)?;
                    filesystem_changed = true;
                    size
                }
                EntryFileState::Missing => {
                    index::delete_entry(&transaction, kind, key)?;
                    filesystem_changed = true;
                    0
                }
                EntryFileState::Unsafe => {
                    return Err(CacheError::Index(
                        "cache entry path contains a symlink or non-file".to_owned(),
                    ));
                }
            };
            index::check_sequence_available(&transaction)?;

            let current_total = index::current_total(&transaction)?;
            let mut projected_total =
                u128::from(current_total - target_size) + u128::from(entry_bytes);
            let budget = u128::from(self.config.max_total_bytes());
            if projected_total > budget {
                for victim in index::lru_entries(&transaction, (kind, key))? {
                    if projected_total <= budget {
                        break;
                    }
                    let victim_path = self.entry_path(victim.kind, victim.key);
                    match entry_file_state(&self.root, victim.kind, victim.key)? {
                        EntryFileState::File(size) => {
                            if size != victim.size_bytes {
                                index::upsert_entry(
                                    &transaction,
                                    victim.kind,
                                    victim.key,
                                    size,
                                    victim.last_access_sequence,
                                )?;
                                filesystem_changed = true;
                            }
                            match fs::remove_file(&victim_path) {
                                Ok(()) => {
                                    filesystem_changed = true;
                                    index::delete_entry(&transaction, victim.kind, victim.key)?;
                                    projected_total = projected_total
                                        .saturating_sub(u128::from(victim.size_bytes));
                                }
                                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                                    filesystem_changed = true;
                                    index::delete_entry(&transaction, victim.kind, victim.key)?;
                                    projected_total = projected_total
                                        .saturating_sub(u128::from(victim.size_bytes));
                                }
                                Err(error) => {
                                    filesystem_changed = true;
                                    return Err(CacheError::Io(error));
                                }
                            }
                        }
                        EntryFileState::Missing | EntryFileState::Unsafe => {
                            filesystem_changed = true;
                            index::delete_entry(&transaction, victim.kind, victim.key)?;
                            projected_total =
                                projected_total.saturating_sub(u128::from(victim.size_bytes));
                        }
                    }
                }
            }

            if projected_total > budget {
                return Err(CacheError::BudgetExceeded {
                    max_total_bytes: self.config.max_total_bytes(),
                    projected_total_bytes: u64::try_from(projected_total).unwrap_or(u64::MAX),
                });
            }

            create_entry_parent(&self.root, kind, key)?;
            atomic_write(&path, bytes)?;
            filesystem_changed = true;
            let sequence = index::allocate_sequence(&transaction)?;
            index::upsert_entry(&transaction, kind, key, entry_bytes, sequence)
        })();
        let (result, discard) = finish_transaction(transaction, filesystem_changed, result);
        if discard {
            state.discard_connection();
        }
        result
    }

    /// Reads a bounded entry, or `Ok(None)` for a normal cache miss.
    pub fn get(
        &self,
        kind: CacheArtifactKind,
        key: CacheKey,
    ) -> Result<Option<Vec<u8>>, CacheError> {
        if kind == CacheArtifactKind::Proxy {
            return Err(CacheError::FileBackedApiRequired);
        }
        let mut state = lock_index(&self.index);
        let transaction = state
            .connection(&self.root)?
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(index::index_error)?;
        let result = (|| {
            let path = self.entry_path(kind, key);
            let size = match entry_file_state(&self.root, kind, key)? {
                EntryFileState::File(size) => size,
                EntryFileState::Missing | EntryFileState::Unsafe => {
                    index::delete_entry(&transaction, kind, key)?;
                    return Ok(None);
                }
            };
            if size > self.config.max_entry_bytes() {
                return Err(CacheError::CorruptOrOversizedEntry {
                    max_bytes: self.config.max_entry_bytes(),
                });
            }
            let file = match File::open(&path) {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    index::delete_entry(&transaction, kind, key)?;
                    return Ok(None);
                }
                Err(error) => return Err(CacheError::Io(error)),
            };
            let mut bytes = Vec::new();
            file.take(self.config.max_entry_bytes().saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(CacheError::Io)?;
            if bytes.len() as u64 > self.config.max_entry_bytes() {
                return Err(CacheError::CorruptOrOversizedEntry {
                    max_bytes: self.config.max_entry_bytes(),
                });
            }
            let sequence = index::allocate_sequence(&transaction)?;
            index::upsert_entry(&transaction, kind, key, bytes.len() as u64, sequence)?;
            Ok(Some(bytes))
        })();
        let (result, discard) = finish_transaction(transaction, false, result);
        if discard {
            state.discard_connection();
        }
        result
    }

    /// Reserves a hidden proxy staging file beside its derived final path.
    pub(crate) fn create_proxy_staging_file(
        &self,
        key: CacheKey,
    ) -> Result<CacheStagingFile, CacheError> {
        let final_path = self.entry_path(CacheArtifactKind::Proxy, key);
        let parent = final_path.parent().ok_or_else(|| {
            CacheError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "proxy path must have a parent directory",
            ))
        })?;
        create_entry_parent(&self.root, CacheArtifactKind::Proxy, key)?;
        let hex = key.to_hex();
        let max_bytes = PROXY_MAX_ARTIFACT_BYTES.min(self.config.max_total_bytes());
        let mut last_collision = None;
        for _ in 0..TEMP_FILE_ATTEMPTS {
            let path = parent.join(format!(".{hex}.or-proxy-tmp-{}.mkv", Uuid::new_v4()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    drop(file);
                    return Ok(CacheStagingFile {
                        key,
                        path,
                        max_bytes,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    last_collision = Some(error);
                }
                Err(error) => return Err(CacheError::Io(error)),
            }
        }
        Err(CacheError::Io(last_collision.unwrap_or_else(|| {
            io::Error::other("proxy staging file attempts were exhausted")
        })))
    }

    /// Looks up a proxy without reading its contents and touches its LRU entry.
    pub(crate) fn proxy_path_if_present(
        &self,
        key: CacheKey,
    ) -> Result<Option<PathBuf>, CacheError> {
        let kind = CacheArtifactKind::Proxy;
        let mut state = lock_index(&self.index);
        let transaction = state
            .connection(&self.root)?
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(index::index_error)?;
        let path = self.entry_path(kind, key);
        let mut filesystem_changed = false;
        let result = (|| match entry_file_state(&self.root, kind, key)? {
            EntryFileState::File(size) if size > 0 && size <= PROXY_MAX_ARTIFACT_BYTES => {
                let sequence = index::allocate_sequence(&transaction)?;
                index::upsert_entry(&transaction, kind, key, size, sequence)?;
                Ok(Some(path))
            }
            EntryFileState::File(_) => {
                match fs::remove_file(&path) {
                    Ok(()) => filesystem_changed = true,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        filesystem_changed = true;
                    }
                    Err(error) => return Err(CacheError::Io(error)),
                }
                index::delete_entry(&transaction, kind, key)?;
                Ok(None)
            }
            EntryFileState::Missing | EntryFileState::Unsafe => {
                index::delete_entry(&transaction, kind, key)?;
                Ok(None)
            }
        })();
        let (result, discard) = finish_transaction(transaction, filesystem_changed, result);
        if discard {
            state.discard_connection();
        }
        result
    }

    /// Atomically installs one validated staged proxy and accounts it in the global LRU.
    pub(crate) fn commit_proxy(&self, staging: CacheStagingFile) -> Result<PathBuf, CacheError> {
        let kind = CacheArtifactKind::Proxy;
        let key = staging.key;
        let path = self.entry_path(kind, key);
        if staging.path.parent() != path.parent()
            || !staging
                .path
                .file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| {
                    name.starts_with(&format!(".{}.or-proxy-tmp-", key.to_hex()))
                        && name.ends_with(".mkv")
                })
        {
            return Err(CacheError::InvalidFileArtifact);
        }
        let metadata = fs::symlink_metadata(&staging.path).map_err(CacheError::Io)?;
        if !metadata.file_type().is_file() || metadata.len() == 0 {
            return Err(CacheError::InvalidFileArtifact);
        }
        let entry_bytes = metadata.len();
        if entry_bytes > PROXY_MAX_ARTIFACT_BYTES {
            return Err(CacheError::EntryTooLarge {
                max_bytes: PROXY_MAX_ARTIFACT_BYTES,
                actual_bytes: entry_bytes,
            });
        }
        if entry_bytes > self.config.max_total_bytes() {
            return Err(CacheError::BudgetExceeded {
                max_total_bytes: self.config.max_total_bytes(),
                projected_total_bytes: entry_bytes,
            });
        }
        OpenOptions::new()
            .write(true)
            .open(&staging.path)
            .and_then(|file| file.sync_all())
            .map_err(CacheError::Io)?;

        let mut state = lock_index(&self.index);
        let transaction = state
            .connection(&self.root)?
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(index::index_error)?;
        let mut filesystem_changed = false;
        let result = (|| {
            let target_size = match entry_file_state(&self.root, kind, key)? {
                EntryFileState::File(size) => {
                    let sequence = index::entry_sequence(&transaction, kind, key)?.unwrap_or(0);
                    index::upsert_entry(&transaction, kind, key, size, sequence)?;
                    filesystem_changed = true;
                    size
                }
                EntryFileState::Missing => {
                    index::delete_entry(&transaction, kind, key)?;
                    filesystem_changed = true;
                    0
                }
                EntryFileState::Unsafe => {
                    return Err(CacheError::Index(
                        "proxy cache path contains a symlink or non-file".to_owned(),
                    ));
                }
            };
            index::check_sequence_available(&transaction)?;

            let current_total = index::current_total(&transaction)?;
            let mut projected_total =
                u128::from(current_total - target_size) + u128::from(entry_bytes);
            let budget = u128::from(self.config.max_total_bytes());
            if projected_total > budget {
                for victim in index::lru_entries(&transaction, (kind, key))? {
                    if projected_total <= budget {
                        break;
                    }
                    let victim_path = self.entry_path(victim.kind, victim.key);
                    match entry_file_state(&self.root, victim.kind, victim.key)? {
                        EntryFileState::File(size) => {
                            if size != victim.size_bytes {
                                index::upsert_entry(
                                    &transaction,
                                    victim.kind,
                                    victim.key,
                                    size,
                                    victim.last_access_sequence,
                                )?;
                                filesystem_changed = true;
                            }
                            match fs::remove_file(&victim_path) {
                                Ok(()) => {
                                    filesystem_changed = true;
                                    index::delete_entry(&transaction, victim.kind, victim.key)?;
                                    projected_total = projected_total
                                        .saturating_sub(u128::from(victim.size_bytes));
                                }
                                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                                    filesystem_changed = true;
                                    index::delete_entry(&transaction, victim.kind, victim.key)?;
                                    projected_total = projected_total
                                        .saturating_sub(u128::from(victim.size_bytes));
                                }
                                Err(error) => {
                                    filesystem_changed = true;
                                    return Err(CacheError::Io(error));
                                }
                            }
                        }
                        EntryFileState::Missing | EntryFileState::Unsafe => {
                            filesystem_changed = true;
                            index::delete_entry(&transaction, victim.kind, victim.key)?;
                            projected_total =
                                projected_total.saturating_sub(u128::from(victim.size_bytes));
                        }
                    }
                }
            }
            if projected_total > budget {
                return Err(CacheError::BudgetExceeded {
                    max_total_bytes: self.config.max_total_bytes(),
                    projected_total_bytes: u64::try_from(projected_total).unwrap_or(u64::MAX),
                });
            }

            create_entry_parent(&self.root, kind, key)?;
            fs::rename(&staging.path, &path).map_err(CacheError::Io)?;
            filesystem_changed = true;
            let sequence = index::allocate_sequence(&transaction)?;
            index::upsert_entry(&transaction, kind, key, entry_bytes, sequence)?;
            Ok(path.clone())
        })();
        let (result, discard) = finish_transaction(transaction, filesystem_changed, result);
        if discard {
            state.discard_connection();
        }
        result
    }

    /// Removes one exact key. Missing entries are an idempotent no-op.
    pub fn remove(&self, kind: CacheArtifactKind, key: CacheKey) -> Result<bool, CacheError> {
        let mut state = lock_index(&self.index);
        let transaction = state
            .connection(&self.root)?
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(index::index_error)?;
        let path = self.entry_path(kind, key);
        let file_state = entry_file_state(&self.root, kind, key)?;
        let repair_needed = !matches!(file_state, EntryFileState::File(_));
        let removed = match file_state {
            EntryFileState::File(_) => match fs::remove_file(&path) {
                Ok(()) => true,
                Err(error) if error.kind() == io::ErrorKind::NotFound => false,
                Err(error) => return Err(CacheError::Io(error)),
            },
            EntryFileState::Missing | EntryFileState::Unsafe => false,
        };
        let result = index::delete_entry(&transaction, kind, key).map(|()| removed);
        let (result, discard) = finish_transaction(transaction, removed || repair_needed, result);
        if discard {
            state.discard_connection();
        }
        result
    }

    /// Removes one managed namespace.
    pub fn clear_namespace(&self, kind: CacheArtifactKind) -> Result<(), CacheError> {
        self.clear_managed(Some(kind))
    }

    /// Removes every managed namespace. The store remains usable afterward.
    pub fn clear_all(&self) -> Result<(), CacheError> {
        self.clear_managed(None)
    }

    fn entry_path(&self, kind: CacheArtifactKind, key: CacheKey) -> PathBuf {
        let hex = key.to_hex();
        let prefix = &hex[..2];
        self.root
            .join(kind.namespace())
            .join(prefix)
            .join(format!("{hex}.{}", kind.extension()))
    }

    fn clear_managed(&self, kind: Option<CacheArtifactKind>) -> Result<(), CacheError> {
        let mut state = lock_index(&self.index);
        let transaction = state
            .connection(&self.root)?
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(index::index_error)?;
        let artifacts = index::scan_artifacts(&self.root)?;
        let mut filesystem_changed = false;
        let mut failure = None;
        for artifact in artifacts
            .into_iter()
            .filter(|artifact| kind.is_none_or(|kind| kind == artifact.kind))
        {
            let path = self.entry_path(artifact.kind, artifact.key);
            match fs::remove_file(&path) {
                Ok(()) => {
                    filesystem_changed = true;
                    if let Err(error) =
                        index::delete_entry(&transaction, artifact.kind, artifact.key)
                    {
                        failure = Some(error);
                        break;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    filesystem_changed = true;
                    if let Err(error) =
                        index::delete_entry(&transaction, artifact.kind, artifact.key)
                    {
                        failure = Some(error);
                        break;
                    }
                }
                Err(error) => {
                    failure = Some(CacheError::Io(error));
                    break;
                }
            }
        }
        if failure.is_none() {
            failure = match kind {
                Some(kind) => index::delete_kind(&transaction, kind),
                None => index::reset_entries(&transaction),
            }
            .err();
        }
        match failure {
            Some(error) => {
                let (result, discard) =
                    finish_transaction(transaction, filesystem_changed, Err(error));
                if discard {
                    state.discard_connection();
                }
                result
            }
            None => {
                let (result, discard) = finish_transaction(transaction, filesystem_changed, Ok(()));
                if discard {
                    state.discard_connection();
                }
                result
            }
        }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntryFileState {
    Missing,
    File(u64),
    Unsafe,
}

fn entry_file_state(
    root: &Path,
    kind: CacheArtifactKind,
    key: CacheKey,
) -> Result<EntryFileState, CacheError> {
    let root_metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(EntryFileState::Missing);
        }
        Err(error) => return Err(CacheError::Io(error)),
    };
    if !root_metadata.file_type().is_dir() {
        return Ok(EntryFileState::Unsafe);
    }

    let hex = key.to_hex();
    let namespace = root.join(kind.namespace());
    let prefix = namespace.join(&hex[..2]);
    for directory in [&namespace, &prefix] {
        match fs::symlink_metadata(directory) {
            Ok(metadata) if metadata.file_type().is_dir() => {}
            Ok(_) => return Ok(EntryFileState::Unsafe),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(EntryFileState::Missing);
            }
            Err(error) => return Err(CacheError::Io(error)),
        }
    }

    let path = prefix.join(format!("{hex}.{}", kind.extension()));
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(EntryFileState::File(metadata.len())),
        Ok(_) => Ok(EntryFileState::Unsafe),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(EntryFileState::Missing),
        Err(error) => Err(CacheError::Io(error)),
    }
}

fn create_entry_parent(
    root: &Path,
    kind: CacheArtifactKind,
    key: CacheKey,
) -> Result<(), CacheError> {
    ensure_real_directory(root)?;
    let hex = key.to_hex();
    ensure_real_directory(&root.join(kind.namespace()))?;
    ensure_real_directory(&root.join(kind.namespace()).join(&hex[..2]))
}

fn ensure_real_directory(path: &Path) -> Result<(), CacheError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(()),
        Ok(_) => Err(CacheError::Index(
            "cache entry parent is not a real directory".to_owned(),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => match fs::create_dir(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                ensure_real_directory(path)
            }
            Err(error) => Err(CacheError::Io(error)),
        },
        Err(error) => Err(CacheError::Io(error)),
    }
}

fn finish_transaction<T>(
    transaction: Transaction<'_>,
    filesystem_changed: bool,
    result: Result<T, CacheError>,
) -> (Result<T, CacheError>, bool) {
    match result {
        Ok(value) => match transaction.commit() {
            Ok(()) => (Ok(value), false),
            Err(error) => (Err(index::index_error(error)), true),
        },
        Err(error) if filesystem_changed => match transaction.commit() {
            Ok(()) => (Err(error), true),
            Err(commit_error) => (Err(index::index_error(commit_error)), true),
        },
        Err(error) => {
            let discard = matches!(error, CacheError::Index(_));
            drop(transaction);
            (Err(error), discard)
        }
    }
}

fn lock_index(index: &Mutex<index::IndexState>) -> std::sync::MutexGuard<'_, index::IndexState> {
    index
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CacheError> {
    let parent = path.parent().ok_or_else(|| {
        CacheError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cache entry path must have a parent directory",
        ))
    })?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JobKind, JobManager, JobManagerConfig, JobState, ProjectDocument, ProjectSession};
    use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
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

    fn open_index(root: &Path) -> Connection {
        Connection::open(index::index_path(root)).unwrap()
    }

    fn indexed_sequence(root: &Path, kind: CacheArtifactKind, key: CacheKey) -> Option<i64> {
        open_index(root)
            .query_row(
                "SELECT last_access_sequence FROM cache_entries WHERE kind = ?1 AND cache_key = ?2",
                rusqlite::params![kind.namespace(), key.to_hex()],
                |row| row.get(0),
            )
            .optional()
            .unwrap()
    }

    fn indexed_size(root: &Path, kind: CacheArtifactKind, key: CacheKey) -> Option<i64> {
        open_index(root)
            .query_row(
                "SELECT size_bytes FROM cache_entries WHERE kind = ?1 AND cache_key = ?2",
                rusqlite::params![kind.namespace(), key.to_hex()],
                |row| row.get(0),
            )
            .optional()
            .unwrap()
    }

    fn proxy_staging_with_bytes(
        store: &CacheStore,
        key: CacheKey,
        bytes: &[u8],
    ) -> CacheStagingFile {
        let staging = store.create_proxy_staging_file(key).unwrap();
        fs::write(staging.path(), bytes).unwrap();
        staging
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
        assert_eq!(CacheArtifactKind::Thumbnail.tag(), 1);
        assert_eq!(CacheArtifactKind::Waveform.tag(), 2);
        assert_eq!(CacheArtifactKind::Proxy.tag(), 3);

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
        assert_eq!(CacheKey::from_hex(&hex).unwrap().to_hex(), hex);
        assert!(CacheKey::from_hex(&hex.to_uppercase()).is_err());
        assert!(CacheKey::from_hex(&format!("{hex}/")).is_err());
        assert!(CacheKey::from_hex(&hex[..63]).is_err());
        assert!(CacheKey::from_hex(&format!("{}g", &hex[..63])).is_err());
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
    fn proxy_file_api_keeps_large_artifacts_out_of_the_byte_api_and_reopens() {
        let directory = TestDirectory::new();
        let store = store(&directory, 8 * 1024 * 1024, 9 * 1024 * 1024);
        let proxy_key = key(CacheArtifactKind::Proxy, b"proxy source", b"profile v1");
        let staging = store.create_proxy_staging_file(proxy_key).unwrap();
        let final_path = store.entry_path(CacheArtifactKind::Proxy, proxy_key);
        assert_eq!(final_path.extension().unwrap(), "mkv");
        assert_eq!(staging.path().parent(), final_path.parent());
        assert!(
            staging
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!(".{}.or-proxy-tmp-", proxy_key.to_hex()))
        );
        assert_eq!(staging.max_bytes(), 9 * 1024 * 1024);

        let mut file = OpenOptions::new().write(true).open(staging.path()).unwrap();
        let chunk = [0x5a_u8; 64 * 1024];
        for _ in 0..=128 {
            file.write_all(&chunk).unwrap();
        }
        drop(file);
        let installed = store.commit_proxy(staging).unwrap();
        assert_eq!(installed, final_path);
        assert_eq!(fs::metadata(&installed).unwrap().len(), 129 * 64 * 1024);
        assert_eq!(
            indexed_size(directory.path(), CacheArtifactKind::Proxy, proxy_key),
            Some(129 * 64 * 1024)
        );
        assert!(matches!(
            store.get(CacheArtifactKind::Proxy, proxy_key),
            Err(CacheError::FileBackedApiRequired)
        ));
        assert!(matches!(
            store.put(CacheArtifactKind::Proxy, proxy_key, b"never bytes"),
            Err(CacheError::FileBackedApiRequired)
        ));

        let reopened = CacheStore::new(
            directory.path(),
            CacheStoreConfig::new(8 * 1024 * 1024, 9 * 1024 * 1024).unwrap(),
        );
        assert_eq!(
            reopened.proxy_path_if_present(proxy_key).unwrap(),
            Some(installed.clone())
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Proxy, proxy_key),
            Some(2)
        );

        fs::write(&installed, b"repaired-size").unwrap();
        assert_eq!(
            reopened.proxy_path_if_present(proxy_key).unwrap(),
            Some(installed.clone())
        );
        assert_eq!(
            indexed_size(directory.path(), CacheArtifactKind::Proxy, proxy_key),
            Some(13)
        );
        assert!(
            reopened
                .remove(CacheArtifactKind::Proxy, proxy_key)
                .unwrap()
        );
        assert_eq!(reopened.proxy_path_if_present(proxy_key).unwrap(), None);

        let clear_key = key(CacheArtifactKind::Proxy, b"clear proxy", b"p");
        reopened
            .commit_proxy(proxy_staging_with_bytes(&reopened, clear_key, b"mkv"))
            .unwrap();
        reopened.clear_namespace(CacheArtifactKind::Proxy).unwrap();
        assert_eq!(reopened.proxy_path_if_present(clear_key).unwrap(), None);

        let empty_key = key(CacheArtifactKind::Proxy, b"empty proxy", b"p");
        let empty_stage = reopened.create_proxy_staging_file(empty_key).unwrap();
        let empty_stage_path = empty_stage.path().to_path_buf();
        assert!(matches!(
            reopened.commit_proxy(empty_stage),
            Err(CacheError::InvalidFileArtifact)
        ));
        assert!(!empty_stage_path.exists());
    }

    #[test]
    fn proxy_budget_failure_preserves_existing_artifacts_and_cleans_stage() {
        let directory = TestDirectory::new();
        let store = store(&directory, 8, 5);
        let thumbnail = key(CacheArtifactKind::Thumbnail, b"budget thumbnail", b"p");
        let proxy = key(CacheArtifactKind::Proxy, b"budget proxy", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, thumbnail, b"keep")
            .unwrap();
        let staging = proxy_staging_with_bytes(&store, proxy, b"too big");
        let staging_path = staging.path().to_path_buf();
        assert!(matches!(
            store.commit_proxy(staging),
            Err(CacheError::BudgetExceeded {
                max_total_bytes: 5,
                projected_total_bytes: 7,
            })
        ));
        assert!(!staging_path.exists());
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, thumbnail).unwrap(),
            Some(b"keep".to_vec())
        );
        assert_eq!(store.proxy_path_if_present(proxy).unwrap(), None);
    }

    #[test]
    fn proxy_commit_replaces_its_target_without_evicting_the_target_key() {
        let directory = TestDirectory::new();
        let store = store(&directory, 16, 16);
        let proxy = key(CacheArtifactKind::Proxy, b"replacement proxy", b"p");
        let thumbnail = key(CacheArtifactKind::Thumbnail, b"replacement neighbor", b"p");
        store
            .commit_proxy(proxy_staging_with_bytes(&store, proxy, b"first"))
            .unwrap();
        store
            .put(CacheArtifactKind::Thumbnail, thumbnail, b"keep")
            .unwrap();
        let path = store.entry_path(CacheArtifactKind::Proxy, proxy);

        store
            .commit_proxy(proxy_staging_with_bytes(&store, proxy, b"replacement!!"))
            .unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"replacement!!");
        assert_eq!(
            indexed_size(directory.path(), CacheArtifactKind::Proxy, proxy),
            Some(13)
        );
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, thumbnail).unwrap(),
            None,
            "the replacement proxy key is protected while the oldest other entry is evicted"
        );
    }

    #[test]
    fn proxy_artifacts_share_global_lru_and_clear_leaves_active_staging_alone() {
        let directory = TestDirectory::new();
        let store = store(&directory, 8, 10);
        let thumbnail = key(CacheArtifactKind::Thumbnail, b"lru thumbnail", b"p");
        let proxy = key(CacheArtifactKind::Proxy, b"lru proxy", b"p");
        let waveform = key(CacheArtifactKind::Waveform, b"lru waveform", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, thumbnail, b"1111")
            .unwrap();
        store
            .commit_proxy(proxy_staging_with_bytes(&store, proxy, b"2222"))
            .unwrap();
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, thumbnail).unwrap(),
            Some(b"1111".to_vec())
        );
        store
            .put(CacheArtifactKind::Waveform, waveform, b"33333")
            .unwrap();
        assert_eq!(store.proxy_path_if_present(proxy).unwrap(), None);
        assert!(
            store
                .entry_path(CacheArtifactKind::Thumbnail, thumbnail)
                .is_file()
        );
        assert!(
            store
                .entry_path(CacheArtifactKind::Waveform, waveform)
                .is_file()
        );

        let staging_key = key(CacheArtifactKind::Proxy, b"active staging", b"p");
        let staging = store.create_proxy_staging_file(staging_key).unwrap();
        let staging_path = staging.path().to_path_buf();
        store.clear_all().unwrap();
        assert!(
            staging_path.is_file(),
            "clear_all must leave an active stage untouched"
        );
        drop(staging);
        assert!(
            !staging_path.exists(),
            "the stage owner cleans up its own path"
        );
        assert_eq!(store.proxy_path_if_present(staging_key).unwrap(), None);
    }

    #[test]
    fn proxy_reconciliation_accepts_only_canonical_mkv_paths() {
        let directory = TestDirectory::new();
        let store = store(&directory, 1024, 4096);
        let valid = key(CacheArtifactKind::Proxy, b"valid proxy", b"p");
        let valid_path = store.entry_path(CacheArtifactKind::Proxy, valid);
        create_entry_parent(store.root(), CacheArtifactKind::Proxy, valid).unwrap();
        fs::write(&valid_path, b"valid").unwrap();

        let wrong_extension = key(CacheArtifactKind::Proxy, b"wrong extension", b"p");
        let wrong_path = store.entry_path(CacheArtifactKind::Proxy, wrong_extension);
        create_entry_parent(store.root(), CacheArtifactKind::Proxy, wrong_extension).unwrap();
        fs::write(wrong_path.with_extension("cache"), b"unknown").unwrap();

        let mismatched_prefix = key(CacheArtifactKind::Proxy, b"mismatched prefix", b"p");
        let mismatched_hex = mismatched_prefix.to_hex();
        let valid_prefix = &mismatched_hex[..2];
        let wrong_prefix =
            store
                .root()
                .join("proxy")
                .join(if valid_prefix == "00" { "01" } else { "00" });
        fs::create_dir_all(&wrong_prefix).unwrap();
        fs::write(
            wrong_prefix.join(format!("{mismatched_hex}.mkv")),
            b"unknown",
        )
        .unwrap();

        let scan = index::scan_artifacts(store.root()).unwrap();
        assert_eq!(scan.len(), 1);
        assert_eq!(scan[0].kind, CacheArtifactKind::Proxy);
        assert_eq!(scan[0].key, valid);
        assert_eq!(scan[0].size_bytes, 5);
    }

    #[test]
    fn index_is_lazy_and_uses_only_the_v1_cache_schema() {
        let directory = TestDirectory::new();
        let root = directory.path().join("lazy-cache");
        let store = CacheStore::new(&root, CacheStoreConfig::new(1024, 4096).unwrap());
        assert!(!root.exists());

        let missing = key(CacheArtifactKind::Thumbnail, b"missing", b"p");
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, missing).unwrap(),
            None
        );
        assert!(index::index_path(&root).is_file());

        let connection = open_index(&root);
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let journal_mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, i64::from(CACHE_INDEX_SCHEMA_VERSION));
        assert_eq!(journal_mode.to_ascii_lowercase(), "delete");
        let tables: Vec<String> = connection
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(tables, ["cache_entries", "cache_meta"]);
        let columns: Vec<String> = connection
            .prepare("PRAGMA table_info(cache_entries)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            columns,
            ["kind", "cache_key", "size_bytes", "last_access_sequence"]
        );
    }

    #[test]
    fn legacy_phase_5d_artifacts_are_discovered_without_regeneration() {
        let directory = TestDirectory::new();
        let store = store(&directory, 1024, 4096);
        let artifact = key(CacheArtifactKind::Thumbnail, b"legacy", b"profile");
        let path = store.entry_path(CacheArtifactKind::Thumbnail, artifact);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"phase 5d preview").unwrap();

        let missing = key(CacheArtifactKind::Waveform, b"not present", b"profile");
        assert_eq!(
            store.get(CacheArtifactKind::Waveform, missing).unwrap(),
            None
        );
        assert_eq!(
            indexed_size(directory.path(), CacheArtifactKind::Thumbnail, artifact),
            Some(16)
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Thumbnail, artifact),
            Some(0)
        );
        assert_eq!(
            store
                .get(CacheArtifactKind::Thumbnail, artifact)
                .unwrap()
                .as_deref(),
            Some(&b"phase 5d preview"[..])
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Thumbnail, artifact),
            Some(1)
        );
    }

    #[test]
    fn reopen_reconciliation_preserves_order_repairs_sizes_and_removes_stale_rows() {
        let directory = TestDirectory::new();
        let first = key(CacheArtifactKind::Thumbnail, b"first", b"p");
        let stale = key(CacheArtifactKind::Waveform, b"stale", b"p");
        let new_file = key(CacheArtifactKind::Waveform, b"new file", b"p");
        {
            let store = store(&directory, 1024, 4096);
            store
                .put(CacheArtifactKind::Thumbnail, first, b"old")
                .unwrap();
            store
                .put(CacheArtifactKind::Waveform, stale, b"remove me")
                .unwrap();
        }

        fs::write(
            store(&directory, 1024, 4096).entry_path(CacheArtifactKind::Thumbnail, first),
            b"changed size",
        )
        .unwrap();
        fs::remove_file(
            store(&directory, 1024, 4096).entry_path(CacheArtifactKind::Waveform, stale),
        )
        .unwrap();
        let new_path =
            store(&directory, 1024, 4096).entry_path(CacheArtifactKind::Waveform, new_file);
        fs::create_dir_all(new_path.parent().unwrap()).unwrap();
        fs::write(&new_path, b"added without a row").unwrap();

        let reopened = store(&directory, 1024, 4096);
        let missing = key(CacheArtifactKind::Thumbnail, b"miss", b"p");
        assert_eq!(
            reopened.get(CacheArtifactKind::Thumbnail, missing).unwrap(),
            None
        );
        assert_eq!(
            indexed_size(directory.path(), CacheArtifactKind::Thumbnail, first),
            Some(12)
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Thumbnail, first),
            Some(1)
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Waveform, stale),
            None
        );
        assert_eq!(
            indexed_size(directory.path(), CacheArtifactKind::Waveform, new_file),
            Some(19)
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Waveform, new_file),
            Some(0)
        );
    }

    #[test]
    fn corrupt_index_rebuilds_without_deleting_artifacts() {
        let directory = TestDirectory::new();
        let artifact = key(CacheArtifactKind::Thumbnail, b"corrupt", b"p");
        {
            let store = store(&directory, 1024, 4096);
            store
                .put(CacheArtifactKind::Thumbnail, artifact, b"keep")
                .unwrap();
        }
        fs::write(
            index::index_path(directory.path()),
            b"not a sqlite database",
        )
        .unwrap();

        let reopened = store(&directory, 1024, 4096);
        let missing = key(CacheArtifactKind::Thumbnail, b"missing", b"p");
        assert_eq!(
            reopened.get(CacheArtifactKind::Thumbnail, missing).unwrap(),
            None
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Thumbnail, artifact),
            Some(0)
        );
        assert_eq!(
            reopened
                .get(CacheArtifactKind::Thumbnail, artifact)
                .unwrap()
                .as_deref(),
            Some(&b"keep"[..])
        );
    }

    #[test]
    fn unsupported_index_version_rebuilds_from_existing_artifacts() {
        let directory = TestDirectory::new();
        let artifact = key(CacheArtifactKind::Waveform, b"unsupported", b"p");
        {
            let store = store(&directory, 1024, 4096);
            store
                .put(CacheArtifactKind::Waveform, artifact, b"keep")
                .unwrap();
        }
        open_index(directory.path())
            .pragma_update(None, "user_version", 99)
            .unwrap();

        let reopened = store(&directory, 1024, 4096);
        let missing = key(CacheArtifactKind::Thumbnail, b"missing", b"p");
        assert_eq!(
            reopened.get(CacheArtifactKind::Thumbnail, missing).unwrap(),
            None
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Waveform, artifact),
            Some(0)
        );
        assert_eq!(
            open_index(directory.path())
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn inconsistent_index_metadata_is_disposable_and_rebuilt() {
        let directory = TestDirectory::new();
        let artifact = key(CacheArtifactKind::Thumbnail, b"invalid kind", b"p");
        {
            let store = store(&directory, 1024, 4096);
            store
                .put(CacheArtifactKind::Thumbnail, artifact, b"keep")
                .unwrap();
        }
        open_index(directory.path())
            .execute("UPDATE cache_entries SET kind = 'proxy'", [])
            .unwrap();

        let reopened = store(&directory, 1024, 4096);
        let missing = key(CacheArtifactKind::Waveform, b"miss", b"p");
        assert_eq!(
            reopened.get(CacheArtifactKind::Waveform, missing).unwrap(),
            None
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Thumbnail, artifact),
            Some(0)
        );
        assert!(
            reopened
                .entry_path(CacheArtifactKind::Thumbnail, artifact)
                .is_file()
        );
    }

    #[test]
    fn misses_and_oversized_reads_do_not_touch_lru_order() {
        let directory = TestDirectory::new();
        let store = store(&directory, 4, 20);
        let artifact = key(CacheArtifactKind::Thumbnail, b"bounded", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, artifact, b"1234")
            .unwrap();
        let miss = key(CacheArtifactKind::Thumbnail, b"miss", b"p");
        assert_eq!(store.get(CacheArtifactKind::Thumbnail, miss).unwrap(), None);
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Thumbnail, artifact),
            Some(1)
        );

        fs::write(
            store.entry_path(CacheArtifactKind::Thumbnail, artifact),
            b"12345",
        )
        .unwrap();
        assert!(matches!(
            store.get(CacheArtifactKind::Thumbnail, artifact),
            Err(CacheError::CorruptOrOversizedEntry { .. })
        ));
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Thumbnail, artifact),
            Some(1)
        );
        let next: i64 = open_index(directory.path())
            .query_row(
                "SELECT next_access_sequence FROM cache_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(next, 2);
    }

    #[test]
    fn lru_order_persists_and_evicts_only_the_oldest_minimum_set() {
        let directory = TestDirectory::new();
        let first = key(CacheArtifactKind::Thumbnail, b"first", b"p");
        let second = key(CacheArtifactKind::Waveform, b"second", b"p");
        let third = key(CacheArtifactKind::Thumbnail, b"third", b"p");
        let incoming = key(CacheArtifactKind::Waveform, b"incoming", b"p");
        {
            let store = store(&directory, 100, 100);
            store
                .put(CacheArtifactKind::Thumbnail, first, &[1; 40])
                .unwrap();
            store
                .put(CacheArtifactKind::Waveform, second, &[2; 30])
                .unwrap();
            store
                .put(CacheArtifactKind::Thumbnail, third, &[3; 20])
                .unwrap();
            store.get(CacheArtifactKind::Thumbnail, first).unwrap();
        }
        let reopened = store(&directory, 100, 100);
        reopened
            .put(CacheArtifactKind::Waveform, incoming, &[4; 35])
            .unwrap();

        assert_eq!(
            reopened.get(CacheArtifactKind::Waveform, second).unwrap(),
            None
        );
        assert_eq!(
            reopened
                .get(CacheArtifactKind::Thumbnail, first)
                .unwrap()
                .unwrap()
                .len(),
            40
        );
        assert_eq!(
            reopened
                .get(CacheArtifactKind::Thumbnail, third)
                .unwrap()
                .unwrap()
                .len(),
            20
        );
        assert_eq!(
            reopened
                .get(CacheArtifactKind::Waveform, incoming)
                .unwrap()
                .unwrap()
                .len(),
            35
        );
        let total: i64 = open_index(directory.path())
            .query_row("SELECT SUM(size_bytes) FROM cache_entries", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(total, 95);
    }

    #[test]
    fn lru_ties_use_kind_then_cache_key_order() {
        let directory = TestDirectory::new();
        let store = store(&directory, 10, 4);
        let thumbnail = key(CacheArtifactKind::Thumbnail, b"old thumbnail", b"p");
        let waveform = key(CacheArtifactKind::Waveform, b"old waveform", b"p");
        for (kind, key, bytes) in [
            (CacheArtifactKind::Thumbnail, thumbnail, b"123".as_slice()),
            (CacheArtifactKind::Waveform, waveform, b"4".as_slice()),
        ] {
            let path = store.entry_path(kind, key);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        let miss = key(CacheArtifactKind::Thumbnail, b"miss", b"p");
        assert_eq!(store.get(CacheArtifactKind::Thumbnail, miss).unwrap(), None);
        let incoming = key(CacheArtifactKind::Thumbnail, b"incoming", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, incoming, b"12")
            .unwrap();
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, thumbnail).unwrap(),
            None
        );
        assert_eq!(
            store.get(CacheArtifactKind::Waveform, waveform).unwrap(),
            Some(b"4".to_vec())
        );
    }

    #[test]
    fn lru_key_ties_use_ascending_cache_key_order() {
        let directory = TestDirectory::new();
        let store = store(&directory, 10, 4);
        let first = key(CacheArtifactKind::Thumbnail, b"first tie", b"p");
        let second = key(CacheArtifactKind::Thumbnail, b"second tie", b"p");
        let (lower, higher) = if first.to_hex() < second.to_hex() {
            (first, second)
        } else {
            (second, first)
        };
        for (artifact, bytes) in [(lower, b"123".as_slice()), (higher, b"4".as_slice())] {
            let path = store.entry_path(CacheArtifactKind::Thumbnail, artifact);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        let missing = key(CacheArtifactKind::Waveform, b"tie miss", b"p");
        assert_eq!(
            store.get(CacheArtifactKind::Waveform, missing).unwrap(),
            None
        );
        let incoming = key(CacheArtifactKind::Waveform, b"tie incoming", b"p");
        store
            .put(CacheArtifactKind::Waveform, incoming, b"12")
            .unwrap();
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, lower).unwrap(),
            None
        );
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, higher).unwrap(),
            Some(b"4".to_vec())
        );
    }

    #[test]
    fn lru_evicts_only_the_oldest_prefix_needed_to_fit() {
        let directory = TestDirectory::new();
        let store = store(&directory, 10, 10);
        let first = key(CacheArtifactKind::Thumbnail, b"prefix first", b"p");
        let second = key(CacheArtifactKind::Waveform, b"prefix second", b"p");
        let third = key(CacheArtifactKind::Thumbnail, b"prefix third", b"p");
        let incoming = key(CacheArtifactKind::Waveform, b"prefix incoming", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, first, b"111")
            .unwrap();
        store
            .put(CacheArtifactKind::Waveform, second, b"222")
            .unwrap();
        store
            .put(CacheArtifactKind::Thumbnail, third, b"333")
            .unwrap();
        store
            .put(CacheArtifactKind::Waveform, incoming, b"44444")
            .unwrap();

        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, first).unwrap(),
            None
        );
        assert_eq!(
            store.get(CacheArtifactKind::Waveform, second).unwrap(),
            None
        );
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, third).unwrap(),
            Some(b"333".to_vec())
        );
        assert_eq!(
            store.get(CacheArtifactKind::Waveform, incoming).unwrap(),
            Some(b"44444".to_vec())
        );
    }

    #[test]
    fn replacing_a_key_protects_it_from_lru_eviction() {
        let directory = TestDirectory::new();
        let store = store(&directory, 10, 8);
        let target = key(CacheArtifactKind::Thumbnail, b"target", b"p");
        let other = key(CacheArtifactKind::Waveform, b"other", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, target, b"1234")
            .unwrap();
        store
            .put(CacheArtifactKind::Waveform, other, b"abcd")
            .unwrap();
        store
            .put(CacheArtifactKind::Thumbnail, target, b"123456")
            .unwrap();
        assert_eq!(store.get(CacheArtifactKind::Waveform, other).unwrap(), None);
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, target).unwrap(),
            Some(b"123456".to_vec())
        );
    }

    #[cfg(unix)]
    #[test]
    fn eviction_delete_failure_keeps_index_reconciled_and_returns_an_error() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let store = store(&directory, 100, 10);
        let victim = key(CacheArtifactKind::Thumbnail, b"victim", b"p");
        let incoming = key(CacheArtifactKind::Waveform, b"incoming", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, victim, b"123456")
            .unwrap();
        let victim_path = store.entry_path(CacheArtifactKind::Thumbnail, victim);
        let parent = victim_path.parent().unwrap();
        fs::set_permissions(parent, fs::Permissions::from_mode(0o500)).unwrap();

        let result = store.put(CacheArtifactKind::Waveform, incoming, b"abcdef");
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).unwrap();

        assert!(matches!(result, Err(CacheError::Io(_))));
        assert!(victim_path.is_file());
        assert_eq!(
            indexed_size(directory.path(), CacheArtifactKind::Thumbnail, victim),
            Some(6)
        );
        assert_eq!(
            store.get(CacheArtifactKind::Waveform, incoming).unwrap(),
            None
        );
    }

    #[test]
    fn cache_index_entry_scan_has_a_hard_bound() {
        let directory = TestDirectory::new();
        let store = store(&directory, 10, 20);
        for source in [b"one".as_slice(), b"two".as_slice(), b"three".as_slice()] {
            let artifact = key(CacheArtifactKind::Thumbnail, source, b"p");
            let path = store.entry_path(CacheArtifactKind::Thumbnail, artifact);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"x").unwrap();
        }
        assert!(matches!(
            index::scan_artifacts_with_limit(directory.path(), 2),
            Err(CacheError::IndexTooLarge { max_entries: 2 })
        ));
    }

    #[test]
    fn index_and_unknown_files_do_not_consume_artifact_budget() {
        let directory = TestDirectory::new();
        let store = store(&directory, 10, 1);
        fs::write(directory.path().join("unmanaged.bin"), [0u8; 1024]).unwrap();
        let artifact = key(CacheArtifactKind::Thumbnail, b"small budget", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, artifact, b"x")
            .unwrap();
        assert_eq!(index::MAX_CACHE_INDEX_ENTRIES, 100_000);
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, artifact).unwrap(),
            Some(b"x".to_vec())
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
    fn total_budget_evicts_the_oldest_entry_when_one_victim_is_enough() {
        let directory = TestDirectory::new();
        let store = store(&directory, 100, 100);
        let first = key(CacheArtifactKind::Thumbnail, b"a", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, first, &[0u8; 60])
            .unwrap();

        let second = key(CacheArtifactKind::Thumbnail, b"b", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, second, &[0u8; 60])
            .unwrap();
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, second).unwrap(),
            Some(vec![0u8; 60])
        );
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, first).unwrap(),
            None
        );
    }

    #[test]
    fn impossible_budget_write_fails_without_evicting_existing_entries() {
        let directory = TestDirectory::new();
        let store = store(&directory, 100, 10);
        let first = key(CacheArtifactKind::Thumbnail, b"a", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, first, &[0u8; 6])
            .unwrap();
        let second = key(CacheArtifactKind::Thumbnail, b"b", b"p");
        assert!(matches!(
            store.put(CacheArtifactKind::Thumbnail, second, &[0u8; 11]),
            Err(CacheError::BudgetExceeded {
                max_total_bytes: 10,
                projected_total_bytes: 11
            })
        ));
        assert_eq!(
            store.get(CacheArtifactKind::Thumbnail, first).unwrap(),
            Some(vec![0u8; 6])
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
    fn clear_operations_preserve_unknown_files_and_update_the_index() {
        let directory = TestDirectory::new();
        let store = store(&directory, 1024, 4096);
        let thumbnail = key(CacheArtifactKind::Thumbnail, b"managed t", b"p");
        let waveform = key(CacheArtifactKind::Waveform, b"managed w", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, thumbnail, b"t")
            .unwrap();
        store
            .put(CacheArtifactKind::Waveform, waveform, b"w")
            .unwrap();

        let root_unknown = directory.path().join("keep-me.txt");
        let namespace_unknown = directory
            .path()
            .join("thumbnail")
            .join(&thumbnail.to_hex()[..2])
            .join("notes.txt");
        fs::write(&root_unknown, b"unknown root file").unwrap();
        fs::write(&namespace_unknown, b"unknown namespace file").unwrap();

        store.clear_namespace(CacheArtifactKind::Thumbnail).unwrap();
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Thumbnail, thumbnail),
            None
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Waveform, waveform),
            Some(2)
        );
        assert!(root_unknown.is_file());
        assert!(namespace_unknown.is_file());
        assert_eq!(
            store.get(CacheArtifactKind::Waveform, waveform).unwrap(),
            Some(b"w".to_vec())
        );

        store.clear_all().unwrap();
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Waveform, waveform),
            None
        );
        assert!(root_unknown.is_file());
        assert!(namespace_unknown.is_file());
        let next: i64 = open_index(directory.path())
            .query_row(
                "SELECT next_access_sequence FROM cache_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(next, 1);
        let after_clear = key(CacheArtifactKind::Thumbnail, b"after clear", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, after_clear, b"x")
            .unwrap();
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Thumbnail, after_clear),
            Some(1)
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
    fn cloned_stores_serialize_concurrent_budget_updates() {
        let directory = TestDirectory::new();
        let store = store(&directory, 100, 100);
        let clone = store.clone();
        let first = key(CacheArtifactKind::Thumbnail, b"clone first", b"p");
        let second = key(CacheArtifactKind::Waveform, b"clone second", b"p");
        let barrier = Arc::new(Barrier::new(3));
        let handles = [
            (store, CacheArtifactKind::Thumbnail, first),
            (clone, CacheArtifactKind::Waveform, second),
        ]
        .into_iter()
        .map(|(store, kind, key)| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                store.put(kind, key, &[7; 60])
            })
        })
        .collect::<Vec<_>>();
        barrier.wait();
        for handle in handles {
            handle.join().unwrap().unwrap();
        }

        let total: i64 = open_index(directory.path())
            .query_row(
                "SELECT COALESCE(SUM(size_bytes), 0) FROM cache_entries",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(total <= 100);
    }

    #[test]
    fn independent_stores_coordinate_budget_updates_through_sqlite() {
        let directory = TestDirectory::new();
        let first_store = store(&directory, 100, 100);
        let second_store = store(&directory, 100, 100);
        let first = key(CacheArtifactKind::Thumbnail, b"store first", b"p");
        let second = key(CacheArtifactKind::Waveform, b"store second", b"p");
        let barrier = Arc::new(Barrier::new(3));
        let first_barrier = Arc::clone(&barrier);
        let first_handle = thread::spawn(move || {
            first_barrier.wait();
            first_store.put(CacheArtifactKind::Thumbnail, first, &[1; 60])
        });
        let second_barrier = Arc::clone(&barrier);
        let second_handle = thread::spawn(move || {
            second_barrier.wait();
            second_store.put(CacheArtifactKind::Waveform, second, &[2; 60])
        });
        barrier.wait();
        first_handle.join().unwrap().unwrap();
        second_handle.join().unwrap().unwrap();

        let total: i64 = open_index(directory.path())
            .query_row(
                "SELECT COALESCE(SUM(size_bytes), 0) FROM cache_entries",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(total <= 100);
    }

    #[test]
    fn sqlite_lock_wait_is_bounded_to_one_second() {
        let directory = TestDirectory::new();
        let store = store(&directory, 100, 100);
        let artifact = key(CacheArtifactKind::Thumbnail, b"lock", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, artifact, b"x")
            .unwrap();

        let mut blocker = open_index(directory.path());
        let transaction = blocker
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        let started = Instant::now();
        let result = store.get(CacheArtifactKind::Thumbnail, artifact);
        let elapsed = started.elapsed();
        assert!(matches!(result, Err(CacheError::Index(_))));
        assert!(elapsed >= Duration::from_millis(900));
        assert!(elapsed < Duration::from_secs(3));
        drop(transaction);
    }

    #[test]
    fn access_sequence_exhaustion_is_controlled_and_does_not_wrap() {
        let directory = TestDirectory::new();
        let store = store(&directory, 100, 100);
        let artifact = key(CacheArtifactKind::Thumbnail, b"sequence", b"p");
        store
            .put(CacheArtifactKind::Thumbnail, artifact, b"x")
            .unwrap();
        open_index(directory.path())
            .execute(
                "UPDATE cache_meta SET next_access_sequence = ?1 WHERE id = 1",
                [i64::MAX],
            )
            .unwrap();
        assert!(matches!(
            store.get(CacheArtifactKind::Thumbnail, artifact),
            Err(CacheError::IndexSequenceExhausted)
        ));
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Thumbnail, artifact),
            Some(1)
        );
        let next: i64 = open_index(directory.path())
            .query_row(
                "SELECT next_access_sequence FROM cache_meta WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(next, i64::MAX);
    }

    #[cfg(unix)]
    #[test]
    fn reconciliation_and_clear_do_not_follow_cache_symlinks() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new();
        let store = store(&directory, 100, 100);
        let target = directory.path().join("outside.cache");
        fs::write(&target, b"outside").unwrap();
        let linked = key(CacheArtifactKind::Thumbnail, b"linked", b"p");
        let linked_path = store.entry_path(CacheArtifactKind::Thumbnail, linked);
        fs::create_dir_all(linked_path.parent().unwrap()).unwrap();
        symlink(&target, &linked_path).unwrap();

        let prefix_linked = key(CacheArtifactKind::Waveform, b"prefix", b"p");
        let outside_prefix = directory.path().join("outside-prefix");
        fs::create_dir(&outside_prefix).unwrap();
        let outside_artifact = outside_prefix.join(format!("{}.cache", prefix_linked.to_hex()));
        fs::write(&outside_artifact, b"not managed").unwrap();
        let prefix_path = store.entry_path(CacheArtifactKind::Waveform, prefix_linked);
        fs::create_dir_all(prefix_path.parent().unwrap().parent().unwrap()).unwrap();
        symlink(&outside_prefix, prefix_path.parent().unwrap()).unwrap();

        let missing = key(CacheArtifactKind::Waveform, b"miss", b"p");
        assert_eq!(
            store.get(CacheArtifactKind::Waveform, missing).unwrap(),
            None
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Thumbnail, linked),
            None
        );
        assert_eq!(
            indexed_sequence(directory.path(), CacheArtifactKind::Waveform, prefix_linked),
            None
        );
        store.clear_all().unwrap();
        assert!(
            fs::symlink_metadata(&linked_path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(
            fs::symlink_metadata(prefix_path.parent().unwrap())
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(target.is_file());
        assert!(outside_artifact.is_file());
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
