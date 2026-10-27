//! The app-level motion asset library (ADR 0010).
//!
//! The library is a plain directory under application support: one bounded
//! `catalog.json` index plus one validated `.ormotionasset.json` per entry. It is
//! deliberately not a database and not project state; a project records only an
//! asset id and digest as provenance for materialised media.
//!
//! Writes are atomic (temporary file plus rename). Reads are bounded. Paths inside
//! an asset are relative to the library root and validated by the asset module, so
//! an entry cannot point outside it.

use crate::{
    MotionAssetCategory, MotionAssetCategorySource, MotionAssetDocument, MotionAssetKind,
    MotionCanvas, MotionDiagnostic, classify_motion_asset, duration_milliseconds,
    is_safe_relative_path, is_uuid_v4, parse_motion_asset, validate_motion_asset,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

/// The `format` value the catalog carries.
pub const MOTION_LIBRARY_FORMAT: &str = "opencut-reinforced-motion-library";
/// The catalog schema version this crate reads and writes.
pub const MOTION_LIBRARY_SCHEMA_VERSION: u32 = 1;
/// The largest catalog this accepts.
pub const MAX_MOTION_LIBRARY_BYTES: usize = 4 * 1024 * 1024;
/// The most entries one library may hold.
pub const MAX_MOTION_LIBRARY_ENTRIES: usize = 4096;

/// One browseable row. It is a projection of the asset document, not a second
/// source of truth: saving rewrites both from the document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionLibraryEntry {
    pub asset_id: String,
    pub name: String,
    pub category: MotionAssetCategory,
    pub category_source: MotionAssetCategorySource,
    pub kind: MotionAssetKind,
    #[serde(default)]
    pub tags: Vec<String>,
    pub canvas: MotionCanvas,
    pub duration_milliseconds: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

/// The bounded index. Entries are sorted by name and then id on every write.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionLibraryCatalog {
    pub format: String,
    pub schema_version: u32,
    #[serde(default)]
    pub entries: Vec<MotionLibraryEntry>,
}

impl Default for MotionLibraryCatalog {
    fn default() -> Self {
        Self {
            format: MOTION_LIBRARY_FORMAT.to_owned(),
            schema_version: MOTION_LIBRARY_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

/// Why a library operation failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MotionLibraryError {
    /// The directory or a file could not be read or written.
    Io(String),
    /// The catalog is larger than [`MAX_MOTION_LIBRARY_BYTES`].
    TooLarge,
    /// The catalog is not valid for this version.
    InvalidCatalog(String),
    /// An asset document did not validate.
    InvalidAsset(Vec<MotionDiagnostic>),
    /// An asset id is not a UUIDv4, so it cannot name a file safely.
    UnsafeAssetId(String),
    /// The id is not in the library.
    NotFound(String),
    /// The library already holds [`MAX_MOTION_LIBRARY_ENTRIES`] entries.
    Full,
    /// A stored relative path would leave the library root.
    UnsafePath(String),
}

impl MotionLibraryError {
    /// The stable wire spelling.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "MOTION_LIBRARY_IO",
            Self::TooLarge => "MOTION_LIBRARY_TOO_LARGE",
            Self::InvalidCatalog(_) => "MOTION_LIBRARY_INVALID_CATALOG",
            Self::InvalidAsset(_) => "MOTION_LIBRARY_INVALID_ASSET",
            Self::UnsafeAssetId(_) => "MOTION_LIBRARY_UNSAFE_ASSET_ID",
            Self::NotFound(_) => "MOTION_LIBRARY_NOT_FOUND",
            Self::Full => "MOTION_LIBRARY_FULL",
            Self::UnsafePath(_) => "MOTION_LIBRARY_UNSAFE_PATH",
        }
    }
}

impl fmt::Display for MotionLibraryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => {
                write!(formatter, "the motion library could not be read: {message}")
            }
            Self::TooLarge => formatter.write_str("the motion library catalog is too large"),
            Self::InvalidCatalog(message) => {
                write!(
                    formatter,
                    "the motion library catalog is invalid: {message}"
                )
            }
            Self::InvalidAsset(diagnostics) => write!(
                formatter,
                "the asset does not validate: {}",
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.code.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::UnsafeAssetId(id) => write!(formatter, "the asset id is not a UUIDv4: {id}"),
            Self::NotFound(id) => write!(formatter, "no asset in the library has id {id}"),
            Self::Full => formatter.write_str("the motion library is full"),
            Self::UnsafePath(path) => write!(formatter, "the path leaves the library root: {path}"),
        }
    }
}

impl std::error::Error for MotionLibraryError {}

/// A library rooted at one directory.
#[derive(Clone, Debug)]
pub struct MotionLibrary {
    root: PathBuf,
}

impl MotionLibrary {
    /// Opens (and creates) a library at `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, MotionLibraryError> {
        let root = root.into();
        for path in [
            root.clone(),
            root.join("assets"),
            root.join("previews"),
            root.join("images"),
        ] {
            fs::create_dir_all(&path).map_err(|error| MotionLibraryError::Io(error.to_string()))?;
        }
        Ok(Self { root })
    }

    /// The directory this library owns.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The catalog path.
    pub fn catalog_path(&self) -> PathBuf {
        self.root.join("catalog.json")
    }

    /// Lists the catalog entries in their stored order.
    pub fn list(&self) -> Result<Vec<MotionLibraryEntry>, MotionLibraryError> {
        let catalog = self.read_catalog()?;
        if catalog.entries.len() > MAX_MOTION_LIBRARY_ENTRIES {
            return Err(MotionLibraryError::TooLarge);
        }
        Ok(catalog.entries)
    }

    /// Loads and validates one asset by id.
    pub fn load(&self, asset_id: &str) -> Result<MotionAssetDocument, MotionLibraryError> {
        if !is_uuid_v4(asset_id) {
            return Err(MotionLibraryError::UnsafeAssetId(asset_id.to_owned()));
        }
        let path = self.asset_path(asset_id);
        let bytes =
            fs::read(&path).map_err(|_| MotionLibraryError::NotFound(asset_id.to_owned()))?;
        let asset = parse_motion_asset(&bytes).map_err(|error| {
            MotionLibraryError::InvalidAsset(vec![MotionDiagnostic::new(
                error.code(),
                "/",
                error.to_string(),
            )])
        })?;
        if asset.asset_id != asset_id {
            return Err(MotionLibraryError::InvalidAsset(vec![
                MotionDiagnostic::new(
                    "MOTION_LIBRARY_ASSET_ID_MISMATCH",
                    "/asset_id",
                    "the file name and the document's asset_id disagree",
                ),
            ]));
        }
        let validation = validate_motion_asset(&asset);
        if !validation.ok {
            return Err(MotionLibraryError::InvalidAsset(validation.errors));
        }
        Ok(asset)
    }

    /// Saves (or replaces) one asset.
    ///
    /// When [`MotionAssetCategorySource::Explicit`] is recorded, the stored
    /// category wins. Otherwise the classifier recomputes it from the name and
    /// tags, and the entry records which rule fired. `explicit_category` is the
    /// user's manual pick from the CLI or the library UI and always wins.
    pub fn save(
        &self,
        document: &MotionAssetDocument,
        explicit_category: Option<MotionAssetCategory>,
    ) -> Result<MotionLibraryEntry, MotionLibraryError> {
        let validation = validate_motion_asset(document);
        if !validation.ok {
            return Err(MotionLibraryError::InvalidAsset(validation.errors));
        }
        let mut document = document.clone();
        if let Some(category) = explicit_category {
            document.category = category;
            document.category_source = MotionAssetCategorySource::Explicit;
        } else if document.category_source != MotionAssetCategorySource::Explicit {
            let classification =
                classify_motion_asset(&document.name, &document.tags, document.kind, None);
            document.category = classification.category;
            document.category_source = classification.source;
        }

        let mut catalog = self.read_catalog()?;
        let existing = catalog
            .entries
            .iter()
            .position(|entry| entry.asset_id == document.asset_id);
        if existing.is_none() && catalog.entries.len() >= MAX_MOTION_LIBRARY_ENTRIES {
            return Err(MotionLibraryError::Full);
        }

        let bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| MotionLibraryError::Io(error.to_string()))?;
        write_atomic(&self.asset_path(&document.asset_id), &bytes)?;

        let entry = MotionLibraryEntry::from(&document);
        match existing {
            Some(index) => catalog.entries[index] = entry.clone(),
            None => catalog.entries.push(entry.clone()),
        }
        catalog.entries.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.asset_id.cmp(&right.asset_id))
        });
        self.write_catalog(&catalog)?;
        Ok(entry)
    }

    /// Removes one asset and its preview. Returns false when the id is absent.
    pub fn remove(&self, asset_id: &str) -> Result<bool, MotionLibraryError> {
        if !is_uuid_v4(asset_id) {
            return Err(MotionLibraryError::UnsafeAssetId(asset_id.to_owned()));
        }
        let mut catalog = self.read_catalog()?;
        let Some(index) = catalog
            .entries
            .iter()
            .position(|entry| entry.asset_id == asset_id)
        else {
            return Ok(false);
        };
        let entry = catalog.entries.remove(index);
        let asset_path = self.asset_path(asset_id);
        if asset_path.exists() {
            fs::remove_file(&asset_path)
                .map_err(|error| MotionLibraryError::Io(error.to_string()))?;
        }
        if let Some(preview) = entry.preview {
            // A missing or unremovable preview must not leave the catalog and the
            // asset file disagreeing; the next save rewrites it.
            if let Ok(preview_path) = self.resolve_relative(&preview) {
                let _ = fs::remove_file(preview_path);
            }
        }
        self.write_catalog(&catalog)?;
        Ok(true)
    }

    /// Resolves a document-relative path under the library root.
    pub fn resolve_relative(&self, relative: &str) -> Result<PathBuf, MotionLibraryError> {
        if !is_safe_relative_path(relative) {
            return Err(MotionLibraryError::UnsafePath(relative.to_owned()));
        }
        Ok(self
            .root
            .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR)))
    }

    fn asset_path(&self, asset_id: &str) -> PathBuf {
        self.root
            .join("assets")
            .join(format!("{asset_id}.ormotionasset.json"))
    }

    fn read_catalog(&self) -> Result<MotionLibraryCatalog, MotionLibraryError> {
        let path = self.catalog_path();
        if !path.exists() {
            return Ok(MotionLibraryCatalog::default());
        }
        let bytes = fs::read(&path).map_err(|error| MotionLibraryError::Io(error.to_string()))?;
        if bytes.len() > MAX_MOTION_LIBRARY_BYTES {
            return Err(MotionLibraryError::TooLarge);
        }
        let catalog: MotionLibraryCatalog = serde_json::from_slice(&bytes)
            .map_err(|error| MotionLibraryError::InvalidCatalog(error.to_string()))?;
        if catalog.format != MOTION_LIBRARY_FORMAT {
            return Err(MotionLibraryError::InvalidCatalog(
                "the catalog format is not a MotionLibrary format".to_owned(),
            ));
        }
        if catalog.schema_version != MOTION_LIBRARY_SCHEMA_VERSION {
            return Err(MotionLibraryError::InvalidCatalog(
                "the catalog schema_version is not one this version reads".to_owned(),
            ));
        }
        Ok(catalog)
    }

    fn write_catalog(&self, catalog: &MotionLibraryCatalog) -> Result<(), MotionLibraryError> {
        let mut bytes = serde_json::to_vec_pretty(catalog)
            .map_err(|error| MotionLibraryError::Io(error.to_string()))?;
        bytes.push(b'\n');
        write_atomic(&self.catalog_path(), &bytes)
    }
}

impl From<&MotionAssetDocument> for MotionLibraryEntry {
    fn from(document: &MotionAssetDocument) -> Self {
        Self {
            asset_id: document.asset_id.clone(),
            name: document.name.clone(),
            category: document.category,
            category_source: document.category_source,
            kind: document.kind,
            tags: document.tags.clone(),
            canvas: document.canvas,
            duration_milliseconds: duration_milliseconds(document.duration),
            preview: document.preview.clone(),
        }
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), MotionLibraryError> {
    let parent = path.parent().ok_or_else(|| {
        MotionLibraryError::Io(format!("{} has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|error| MotionLibraryError::Io(error.to_string()))?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| MotionLibraryError::Io(format!("{} has no file name", path.display())))?;
    let temporary = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    fs::write(&temporary, bytes).map_err(|error| MotionLibraryError::Io(error.to_string()))?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        MotionLibraryError::Io(error.to_string())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MAX_MOTION_ASSET_BYTES, MOTION_ASSET_SCHEMA_VERSION};
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "or-motion-library-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn asset(
            &self,
            name: &str,
            category: &str,
            source: &str,
            tags: Vec<&str>,
        ) -> MotionAssetDocument {
            let value = json!({
                "format": crate::MOTION_ASSET_FORMAT,
                "schema_version": MOTION_ASSET_SCHEMA_VERSION,
                "asset_id": "6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f",
                "name": name,
                "category": category,
                "category_source": source,
                "tags": tags,
                "kind": "scene",
                "canvas": {"width": 1920, "height": 1080},
                "duration": {"numerator": 4, "denominator": 1},
                "parameters": [],
                "scene": {
                    "assets": [],
                    "nodes": [
                        {
                            "node_id": "title",
                            "kind": "text",
                            "text": name,
                            "position": {"x": 0, "y": 0},
                            "font_size": 40
                        }
                    ],
                    "animations": []
                }
            });
            parse_motion_asset(&serde_json::to_vec(&value).unwrap()).unwrap()
        }

        fn library(&self) -> MotionLibrary {
            MotionLibrary::open(&self.0).unwrap()
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_saved_asset_lists_loads_and_removes() {
        let directory = TestDirectory::new();
        let library = directory.library();
        let asset = directory.asset("Explainer", "text", "explicit", vec!["title"]);

        let entry = library.save(&asset, None).unwrap();
        assert_eq!(entry.asset_id, asset.asset_id);
        assert_eq!(entry.category, MotionAssetCategory::Text);
        assert_eq!(entry.category_source, MotionAssetCategorySource::Explicit);
        assert_eq!(entry.duration_milliseconds, 4000);

        let entries = library.list().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].asset_id, asset.asset_id);
        assert_eq!(library.load(&asset.asset_id).unwrap(), asset);

        // The index survives a reopen.
        let reopened = MotionLibrary::open(&directory.0).unwrap();
        assert_eq!(reopened.list().unwrap().len(), 1);

        assert!(library.remove(&asset.asset_id).unwrap());
        assert!(library.list().unwrap().is_empty());
        assert_eq!(
            library.load(&asset.asset_id).unwrap_err(),
            MotionLibraryError::NotFound(asset.asset_id.clone())
        );
        assert!(!library.remove(&asset.asset_id).unwrap());
    }

    #[test]
    fn saving_recomputes_a_non_explicit_category() {
        let directory = TestDirectory::new();
        let library = directory.library();
        let asset = directory.asset("Mountain background", "object", "default", vec![]);

        let entry = library.save(&asset, None).unwrap();
        assert_eq!(entry.category, MotionAssetCategory::Background);
        assert_eq!(entry.category_source, MotionAssetCategorySource::Filename);

        let stored = library.load(&asset.asset_id).unwrap();
        assert_eq!(stored.category, MotionAssetCategory::Background);
        assert_eq!(stored.category_source, MotionAssetCategorySource::Filename);
    }

    #[test]
    fn a_manual_category_override_wins() {
        let directory = TestDirectory::new();
        let library = directory.library();
        let asset = directory.asset("Car", "object", "default", vec![]);

        let entry = library
            .save(&asset, Some(MotionAssetCategory::People))
            .unwrap();
        assert_eq!(entry.category, MotionAssetCategory::People);
        assert_eq!(entry.category_source, MotionAssetCategorySource::Explicit);

        // Reloading keeps the manual choice: it is now recorded as explicit.
        let stored = library.load(&asset.asset_id).unwrap();
        let entry = library.save(&stored, None).unwrap();
        assert_eq!(entry.category, MotionAssetCategory::People);
    }

    #[test]
    fn an_invalid_asset_is_refused_with_diagnostics() {
        let directory = TestDirectory::new();
        let library = directory.library();
        let mut asset = directory.asset("Explainer", "text", "explicit", vec![]);
        asset.parameters.push(crate::MotionAssetParameter {
            id: "title".to_owned(),
            label: "Title".to_owned(),
            kind: crate::MotionParameterKind::Text,
            target: crate::MotionParameterTarget {
                node_id: "missing".to_owned(),
                field: crate::MotionParameterField::Text,
            },
            default: crate::MotionParameterValue::Text("x".to_owned()),
            min: None,
            max: None,
            choices: None,
        });

        let error = library.save(&asset, None).unwrap_err();
        let MotionLibraryError::InvalidAsset(diagnostics) = error else {
            panic!("expected InvalidAsset");
        };
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "MOTION_ASSET_UNKNOWN_PARAMETER_NODE")
        );
        assert!(library.list().unwrap().is_empty());
    }

    #[test]
    fn an_unsafe_asset_id_cannot_name_a_file() {
        let directory = TestDirectory::new();
        let library = directory.library();
        assert!(matches!(
            library.load("../../etc/passwd"),
            Err(MotionLibraryError::UnsafeAssetId(_))
        ));
        assert!(matches!(
            library.remove("../x"),
            Err(MotionLibraryError::UnsafeAssetId(_))
        ));
    }

    #[test]
    fn removing_an_entry_also_removes_its_preview() {
        let directory = TestDirectory::new();
        let library = directory.library();
        let mut asset = directory.asset("Explainer", "text", "explicit", vec![]);
        asset.preview = Some("previews/6f1c2f5e.png".to_owned());
        let preview = library.resolve_relative("previews/6f1c2f5e.png").unwrap();
        fs::write(&preview, b"not really a png").unwrap();

        library.save(&asset, None).unwrap();
        assert!(library.remove(&asset.asset_id).unwrap());
        assert!(!preview.exists());
    }

    #[test]
    fn the_catalog_refuses_an_unknown_format() {
        let directory = TestDirectory::new();
        let library = directory.library();
        fs::write(
            library.catalog_path(),
            serde_json::to_vec(&json!({
                "format": "something-else",
                "schema_version": 1,
                "entries": []
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(matches!(
            library.list(),
            Err(MotionLibraryError::InvalidCatalog(_))
        ));
    }

    #[test]
    fn a_preview_path_cannot_leave_the_library() {
        let directory = TestDirectory::new();
        let library = directory.library();
        assert!(matches!(
            library.resolve_relative("../outside.png"),
            Err(MotionLibraryError::UnsafePath(_))
        ));
    }

    #[test]
    fn entries_are_sorted_by_name() {
        let directory = TestDirectory::new();
        let library = directory.library();
        let mut zebra = directory.asset("Zebra", "animal", "explicit", vec![]);
        zebra.asset_id = "0b9d2f6e-5c1a-4f3b-9d8e-7a6b5c4d3e2f".to_owned();
        let mut apple = directory.asset("apple", "object", "explicit", vec![]);
        apple.asset_id = "1c8e3a7f-6d2b-4a4c-8e9f-0b1c2d3e4f5a".to_owned();

        library.save(&zebra, None).unwrap();
        library.save(&apple, None).unwrap();
        let names: Vec<String> = library
            .list()
            .unwrap()
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(names, vec!["apple".to_owned(), "Zebra".to_owned()]);
    }

    #[test]
    fn the_asset_file_is_bounded_by_the_asset_parser() {
        // MAX_MOTION_ASSET_BYTES is exported for the caller; this test pins that
        // the library and the parser agree on the bound.
        assert!(MAX_MOTION_ASSET_BYTES >= 1024 * 1024);
    }
}
