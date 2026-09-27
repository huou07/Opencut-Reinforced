use super::{CACHE_INDEX_SCHEMA_VERSION, CacheArtifactKind, CacheError, CacheKey};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params, types::Type,
};
use std::{collections::HashMap, fs, path::Path, time::Duration};

pub(super) const MAX_CACHE_INDEX_ENTRIES: usize = 100_000;
const CACHE_INDEX_FILE: &str = "cache-index.sqlite3";
const BUSY_TIMEOUT: Duration = Duration::from_secs(1);
const MAX_SCANNED_DIRECTORY_ENTRIES: usize = MAX_CACHE_INDEX_ENTRIES * 4;

#[derive(Default)]
pub(super) struct IndexState {
    connection: Option<Connection>,
}

impl std::fmt::Debug for IndexState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("IndexState")
            .field("initialized", &self.connection.is_some())
            .finish()
    }
}

impl IndexState {
    pub(super) fn connection(&mut self, root: &Path) -> Result<&mut Connection, CacheError> {
        if self.connection.is_none() {
            self.connection = Some(open_and_reconcile(root)?);
        }
        Ok(self
            .connection
            .as_mut()
            .expect("cache index was initialized"))
    }

    pub(super) fn discard_connection(&mut self) {
        self.connection = None;
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct IndexedEntry {
    pub(super) kind: CacheArtifactKind,
    pub(super) key: CacheKey,
    pub(super) size_bytes: u64,
    pub(super) last_access_sequence: i64,
}

pub(super) fn current_total(transaction: &Transaction<'_>) -> Result<u64, CacheError> {
    let total: i64 = transaction
        .query_row(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM cache_entries",
            [],
            |row| row.get(0),
        )
        .map_err(index_error)?;
    u64::try_from(total).map_err(|_| CacheError::Index("invalid total artifact size".to_owned()))
}

pub(super) fn entry_sequence(
    transaction: &Transaction<'_>,
    kind: CacheArtifactKind,
    key: CacheKey,
) -> Result<Option<i64>, CacheError> {
    transaction
        .query_row(
            "SELECT last_access_sequence FROM cache_entries WHERE kind = ?1 AND cache_key = ?2",
            params![kind.namespace(), key.to_hex()],
            |row| row.get(0),
        )
        .optional()
        .map_err(index_error)
}

pub(super) fn upsert_entry(
    transaction: &Transaction<'_>,
    kind: CacheArtifactKind,
    key: CacheKey,
    size_bytes: u64,
    last_access_sequence: i64,
) -> Result<(), CacheError> {
    let size_bytes = i64::try_from(size_bytes)
        .map_err(|_| CacheError::Index("artifact size exceeds SQLite INTEGER".to_owned()))?;
    transaction
        .execute(
            "INSERT INTO cache_entries(kind, cache_key, size_bytes, last_access_sequence)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(kind, cache_key) DO UPDATE SET
                 size_bytes = excluded.size_bytes,
                 last_access_sequence = excluded.last_access_sequence",
            params![
                kind.namespace(),
                key.to_hex(),
                size_bytes,
                last_access_sequence
            ],
        )
        .map_err(index_error)?;
    Ok(())
}

pub(super) fn delete_entry(
    transaction: &Transaction<'_>,
    kind: CacheArtifactKind,
    key: CacheKey,
) -> Result<(), CacheError> {
    transaction
        .execute(
            "DELETE FROM cache_entries WHERE kind = ?1 AND cache_key = ?2",
            params![kind.namespace(), key.to_hex()],
        )
        .map_err(index_error)?;
    Ok(())
}

pub(super) fn delete_kind(
    transaction: &Transaction<'_>,
    kind: CacheArtifactKind,
) -> Result<(), CacheError> {
    transaction
        .execute(
            "DELETE FROM cache_entries WHERE kind = ?1",
            [kind.namespace()],
        )
        .map_err(index_error)?;
    Ok(())
}

pub(super) fn allocate_sequence(transaction: &Transaction<'_>) -> Result<i64, CacheError> {
    let next = next_sequence(transaction)?;
    let following = next
        .checked_add(1)
        .ok_or(CacheError::IndexSequenceExhausted)?;
    transaction
        .execute(
            "UPDATE cache_meta SET next_access_sequence = ?1 WHERE id = 1",
            [following],
        )
        .map_err(index_error)?;
    Ok(next)
}

pub(super) fn check_sequence_available(transaction: &Transaction<'_>) -> Result<(), CacheError> {
    next_sequence(transaction)?
        .checked_add(1)
        .ok_or(CacheError::IndexSequenceExhausted)?;
    Ok(())
}

fn next_sequence(transaction: &Transaction<'_>) -> Result<i64, CacheError> {
    let next: i64 = transaction
        .query_row(
            "SELECT next_access_sequence FROM cache_meta WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(index_error)?;
    if next <= 0 {
        return Err(CacheError::Index("invalid next access sequence".to_owned()));
    }
    Ok(next)
}

pub(super) fn lru_entries(
    transaction: &Transaction<'_>,
    protected: (CacheArtifactKind, CacheKey),
) -> Result<Vec<IndexedEntry>, CacheError> {
    let mut statement = transaction
        .prepare(
            "SELECT kind, cache_key, size_bytes, last_access_sequence
             FROM cache_entries
             WHERE NOT (kind = ?1 AND cache_key = ?2)
             ORDER BY last_access_sequence ASC, kind ASC, cache_key ASC",
        )
        .map_err(index_error)?;
    let rows = statement
        .query_map(
            params![protected.0.namespace(), protected.1.to_hex()],
            |row| {
                let kind_name: String = row.get(0)?;
                let key_hex: String = row.get(1)?;
                let size: i64 = row.get(2)?;
                let sequence: i64 = row.get(3)?;
                let kind = CacheArtifactKind::from_namespace(&kind_name).ok_or_else(|| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        Type::Text,
                        format!("unknown cache kind {kind_name}").into(),
                    )
                })?;
                let key = CacheKey::from_hex(&key_hex).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(1, Type::Text, error.into())
                })?;
                let size_bytes = u64::try_from(size).map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        Type::Integer,
                        "negative cache artifact size".into(),
                    )
                })?;
                Ok(IndexedEntry {
                    kind,
                    key,
                    size_bytes,
                    last_access_sequence: sequence,
                })
            },
        )
        .map_err(index_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(index_error)
}

pub(super) fn reset_entries(transaction: &Transaction<'_>) -> Result<(), CacheError> {
    transaction
        .execute("DELETE FROM cache_entries", [])
        .map_err(index_error)?;
    transaction
        .execute(
            "UPDATE cache_meta SET next_access_sequence = 1 WHERE id = 1",
            [],
        )
        .map_err(index_error)?;
    Ok(())
}

fn open_and_reconcile(root: &Path) -> Result<Connection, CacheError> {
    fs::create_dir_all(root).map_err(CacheError::Io)?;
    if !fs::symlink_metadata(root)
        .map_err(CacheError::Io)?
        .file_type()
        .is_dir()
    {
        return Err(CacheError::Index(
            "cache root is not a real directory".to_owned(),
        ));
    }

    let database_path = root.join(CACHE_INDEX_FILE);
    for rebuild_attempt in 0..2 {
        let mut connection = open_connection(&database_path)?;
        match initialize_and_reconcile(&mut connection, root) {
            Ok(()) => return Ok(connection),
            Err(StartupError::Cache(error)) => return Err(error),
            Err(StartupError::Rebuild) if rebuild_attempt == 0 => {
                drop(connection);
                remove_disposable_index(&database_path)?;
            }
            Err(StartupError::Rebuild) => {
                return Err(CacheError::Index(
                    "cache index could not be rebuilt".to_owned(),
                ));
            }
        }
    }
    Err(CacheError::Index(
        "cache index could not be opened".to_owned(),
    ))
}

fn open_connection(path: &Path) -> Result<Connection, CacheError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => {
            return Err(CacheError::Index(
                "cache index path is not a regular file".to_owned(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(CacheError::Io(error)),
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )
    .map_err(index_error)?;
    connection.busy_timeout(BUSY_TIMEOUT).map_err(index_error)?;
    Ok(connection)
}

enum StartupError {
    Rebuild,
    Cache(CacheError),
}

enum ExistingIndex {
    Empty,
    Valid {
        entries: HashMap<(CacheArtifactKind, CacheKey), i64>,
    },
    Rebuild,
}

fn initialize_and_reconcile(connection: &mut Connection, root: &Path) -> Result<(), StartupError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Exclusive)
        .map_err(classify_startup_sql)?;
    let existing = validate_existing_index(&transaction)?;
    let (old_sequences, is_new) = match existing {
        ExistingIndex::Empty => (HashMap::new(), true),
        ExistingIndex::Valid { entries } => (entries, false),
        ExistingIndex::Rebuild => return Err(StartupError::Rebuild),
    };

    if is_new {
        create_schema(&transaction).map_err(StartupError::Cache)?;
    }
    let files = scan_artifacts(root).map_err(StartupError::Cache)?;
    replace_reconciled_entries(&transaction, files, &old_sequences).map_err(StartupError::Cache)?;
    transaction.commit().map_err(classify_startup_sql)
}

fn classify_startup_sql(error: rusqlite::Error) -> StartupError {
    if is_busy(&error) {
        StartupError::Cache(index_error(error))
    } else {
        StartupError::Rebuild
    }
}

fn validate_existing_index(transaction: &Transaction<'_>) -> Result<ExistingIndex, StartupError> {
    let quick_check = transaction
        .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
        .map_err(classify_startup_sql)?;
    if quick_check != "ok" {
        return Ok(ExistingIndex::Rebuild);
    }

    let version = transaction
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map_err(classify_startup_sql)?;
    let objects = schema_objects(transaction).map_err(classify_startup_sql)?;
    if version == 0 && objects.is_empty() {
        return Ok(ExistingIndex::Empty);
    }
    if version != i64::from(CACHE_INDEX_SCHEMA_VERSION)
        || objects
            != [
                ("index".to_owned(), "cache_entries_lru".to_owned()),
                ("table".to_owned(), "cache_entries".to_owned()),
                ("table".to_owned(), "cache_meta".to_owned()),
            ]
    {
        return Ok(ExistingIndex::Rebuild);
    }
    if !table_columns_match(
        transaction,
        "cache_meta",
        &[
            ("id", "INTEGER", false, 1),
            ("next_access_sequence", "INTEGER", true, 0),
        ],
    )
    .map_err(classify_startup_sql)?
        || !table_columns_match(
            transaction,
            "cache_entries",
            &[
                ("kind", "TEXT", true, 1),
                ("cache_key", "TEXT", true, 2),
                ("size_bytes", "INTEGER", true, 0),
                ("last_access_sequence", "INTEGER", true, 0),
            ],
        )
        .map_err(classify_startup_sql)?
        || !lru_index_matches(transaction).map_err(classify_startup_sql)?
    {
        return Ok(ExistingIndex::Rebuild);
    }

    let mut metadata = transaction
        .prepare("SELECT id, next_access_sequence FROM cache_meta LIMIT 2")
        .map_err(classify_startup_sql)?;
    let meta_rows = metadata
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
        .map_err(classify_startup_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_startup_sql)?;
    if meta_rows.len() != 1 || meta_rows[0].0 != 1 || meta_rows[0].1 <= 0 {
        return Ok(ExistingIndex::Rebuild);
    }
    let next_sequence = meta_rows[0].1;

    let mut statement = transaction
        .prepare(
            "SELECT kind, cache_key, size_bytes, last_access_sequence
             FROM cache_entries LIMIT ?1",
        )
        .map_err(classify_startup_sql)?;
    let rows = statement
        .query_map([(MAX_CACHE_INDEX_ENTRIES + 1) as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(classify_startup_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(classify_startup_sql)?;
    if rows.len() > MAX_CACHE_INDEX_ENTRIES {
        return Ok(ExistingIndex::Rebuild);
    }

    let mut entries = HashMap::with_capacity(rows.len());
    let mut largest_sequence = 0;
    for (kind_text, key_text, size, sequence) in rows {
        let Some(kind) = CacheArtifactKind::from_namespace(&kind_text) else {
            return Ok(ExistingIndex::Rebuild);
        };
        let Ok(key) = CacheKey::from_hex(&key_text) else {
            return Ok(ExistingIndex::Rebuild);
        };
        if size < 0 || sequence < 0 {
            return Ok(ExistingIndex::Rebuild);
        }
        largest_sequence = largest_sequence.max(sequence);
        entries.insert((kind, key), sequence);
    }
    if next_sequence <= largest_sequence {
        return Ok(ExistingIndex::Rebuild);
    }
    Ok(ExistingIndex::Valid { entries })
}

fn schema_objects(transaction: &Transaction<'_>) -> rusqlite::Result<Vec<(String, String)>> {
    let mut statement = transaction.prepare(
        "SELECT type, name FROM sqlite_master
         WHERE name NOT LIKE 'sqlite_%' AND type IN ('table', 'index', 'trigger', 'view')
         ORDER BY type, name",
    )?;
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect()
}

fn table_columns_match(
    transaction: &Transaction<'_>,
    table: &str,
    expected: &[(&str, &str, bool, i64)],
) -> rusqlite::Result<bool> {
    let sql = match table {
        "cache_meta" => "PRAGMA table_info(cache_meta)",
        "cache_entries" => "PRAGMA table_info(cache_entries)",
        _ => return Ok(false),
    };
    let mut statement = transaction.prepare(sql)?;
    let columns = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? != 0,
                row.get::<_, i64>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(columns.len() == expected.len()
        && columns
            .iter()
            .zip(expected)
            .all(|((name, kind, not_null, primary_key), expected)| {
                name == expected.0
                    && kind.eq_ignore_ascii_case(expected.1)
                    && *not_null == expected.2
                    && *primary_key == expected.3
            }))
}

fn lru_index_matches(transaction: &Transaction<'_>) -> rusqlite::Result<bool> {
    let mut indexes = transaction.prepare("PRAGMA index_list(cache_entries)")?;
    let found = indexes
        .query_map([], |row| {
            Ok((row.get::<_, String>(1)?, row.get::<_, i64>(2)? != 0))
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .any(|(name, unique)| name == "cache_entries_lru" && !unique);
    if !found {
        return Ok(false);
    }
    let mut columns = transaction.prepare("PRAGMA index_info(cache_entries_lru)")?;
    let columns = columns
        .query_map([], |row| row.get::<_, String>(2))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(columns == ["last_access_sequence", "kind", "cache_key"])
}

fn create_schema(transaction: &Transaction<'_>) -> Result<(), CacheError> {
    transaction
        .execute_batch(
            "CREATE TABLE cache_meta (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 next_access_sequence INTEGER NOT NULL CHECK (next_access_sequence > 0)
             );
             CREATE TABLE cache_entries (
                 kind TEXT NOT NULL,
                 cache_key TEXT NOT NULL,
                 size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
                 last_access_sequence INTEGER NOT NULL CHECK (last_access_sequence >= 0),
                 PRIMARY KEY (kind, cache_key)
             );
             CREATE INDEX cache_entries_lru
                 ON cache_entries(last_access_sequence, kind, cache_key);
             INSERT INTO cache_meta(id, next_access_sequence) VALUES (1, 1);",
        )
        .map_err(index_error)?;
    transaction
        .pragma_update(None, "user_version", CACHE_INDEX_SCHEMA_VERSION)
        .map_err(index_error)?;
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) struct ManagedArtifact {
    pub(super) kind: CacheArtifactKind,
    pub(super) key: CacheKey,
    pub(super) size_bytes: u64,
}

fn replace_reconciled_entries(
    transaction: &Transaction<'_>,
    artifacts: Vec<ManagedArtifact>,
    old_sequences: &HashMap<(CacheArtifactKind, CacheKey), i64>,
) -> Result<(), CacheError> {
    transaction
        .execute("DELETE FROM cache_entries", [])
        .map_err(index_error)?;
    let mut statement = transaction
        .prepare(
            "INSERT INTO cache_entries(kind, cache_key, size_bytes, last_access_sequence)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .map_err(index_error)?;
    for artifact in artifacts {
        let size = i64::try_from(artifact.size_bytes)
            .map_err(|_| CacheError::Index("artifact size exceeds SQLite INTEGER".to_owned()))?;
        statement
            .execute(params![
                artifact.kind.namespace(),
                artifact.key.to_hex(),
                size,
                old_sequences
                    .get(&(artifact.kind, artifact.key))
                    .copied()
                    .unwrap_or(0)
            ])
            .map_err(index_error)?;
    }
    Ok(())
}

pub(super) fn scan_artifacts(root: &Path) -> Result<Vec<ManagedArtifact>, CacheError> {
    scan_artifacts_with_limit(root, MAX_CACHE_INDEX_ENTRIES)
}

pub(super) fn scan_artifacts_with_limit(
    root: &Path,
    max_entries: usize,
) -> Result<Vec<ManagedArtifact>, CacheError> {
    scan_artifacts_bounded(root, max_entries)
}

fn scan_artifacts_bounded(
    root: &Path,
    max_entries: usize,
) -> Result<Vec<ManagedArtifact>, CacheError> {
    let mut artifacts = Vec::new();
    let mut scanned = 0;
    let root_metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(artifacts),
        Err(error) => return Err(CacheError::Io(error)),
    };
    if !root_metadata.file_type().is_dir() {
        return Err(CacheError::Index(
            "cache root is not a real directory".to_owned(),
        ));
    }

    for namespace in fs::read_dir(root).map_err(CacheError::Io)? {
        let namespace = namespace.map_err(CacheError::Io)?;
        bump_scan_count(&mut scanned)?;
        let Some(kind) = namespace
            .file_name()
            .to_str()
            .and_then(CacheArtifactKind::from_namespace)
        else {
            continue;
        };
        if !namespace.file_type().map_err(CacheError::Io)?.is_dir() {
            continue;
        }
        for prefix_dir in fs::read_dir(namespace.path()).map_err(CacheError::Io)? {
            let prefix_dir = prefix_dir.map_err(CacheError::Io)?;
            bump_scan_count(&mut scanned)?;
            let prefix = prefix_dir.file_name();
            let Some(prefix) = prefix.to_str() else {
                continue;
            };
            if !is_lower_hex_prefix(prefix)
                || !prefix_dir.file_type().map_err(CacheError::Io)?.is_dir()
            {
                continue;
            }
            for candidate in fs::read_dir(prefix_dir.path()).map_err(CacheError::Io)? {
                let candidate = candidate.map_err(CacheError::Io)?;
                bump_scan_count(&mut scanned)?;
                if !candidate.file_type().map_err(CacheError::Io)?.is_file() {
                    continue;
                }
                let Some(file_name) = candidate.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                let Some(key_text) = file_name.strip_suffix(".cache") else {
                    continue;
                };
                let Ok(key) = CacheKey::from_hex(key_text) else {
                    continue;
                };
                if !key_text.starts_with(prefix) {
                    continue;
                }
                let metadata = fs::symlink_metadata(candidate.path()).map_err(CacheError::Io)?;
                if !metadata.file_type().is_file() {
                    continue;
                }
                if artifacts.len() == max_entries {
                    return Err(CacheError::IndexTooLarge { max_entries });
                }
                artifacts.push(ManagedArtifact {
                    kind,
                    key,
                    size_bytes: metadata.len(),
                });
            }
        }
    }
    Ok(artifacts)
}

fn bump_scan_count(scanned: &mut usize) -> Result<(), CacheError> {
    *scanned += 1;
    if *scanned > MAX_SCANNED_DIRECTORY_ENTRIES {
        return Err(CacheError::IndexTooLarge {
            max_entries: MAX_CACHE_INDEX_ENTRIES,
        });
    }
    Ok(())
}

fn is_lower_hex_prefix(value: &str) -> bool {
    value.len() == 2
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn remove_disposable_index(path: &Path) -> Result<(), CacheError> {
    remove_if_regular_file(path)?;
    let mut journal = path.as_os_str().to_os_string();
    journal.push("-journal");
    remove_if_regular_file(Path::new(&journal))
}

fn remove_if_regular_file(path: &Path) -> Result<(), CacheError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() || metadata.file_type().is_symlink() => {
            fs::remove_file(path).map_err(CacheError::Io)
        }
        Ok(_) => Err(CacheError::Index(
            "cache index metadata path is not a file".to_owned(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(CacheError::Io(error)),
    }
}

pub(super) fn index_error(error: rusqlite::Error) -> CacheError {
    CacheError::Index(error.to_string())
}

fn is_busy(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(
                failure.code,
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
            )
    )
}

#[cfg(test)]
pub(super) fn index_path(root: &Path) -> std::path::PathBuf {
    root.join(CACHE_INDEX_FILE)
}
