//! Versioned, license-bearing model metadata and bounded artifact installation.

use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::AiTask;

pub const MAX_MODEL_ARTIFACT_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_MANIFEST_TEXT_BYTES: usize = 4096;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelManifest {
    pub schema_version: u16,
    pub model_id: String,
    pub version: String,
    pub tasks: Vec<AiTask>,
    pub provider_id: String,
    pub runtime: String,
    pub source: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub license: String,
    pub provenance: String,
    pub hardware_requirements: Vec<String>,
    pub languages: Vec<String>,
}

/// One selectable local transcription model and its reviewed artifact.
///
/// Every entry is multilingual, MIT licensed, pinned to the same whisper.cpp
/// revision and downloaded only after explicit consent. Sizes and digests are
/// the Hugging Face LFS values; `ModelManifest::validate` and the installer
/// re-check them.
const WHISPER_REVISION: &str = "5359861c739e955e79d9a303bcbc70fb988958b1";

fn whisper_manifest(
    model_id: &str,
    artifact: &str,
    sha256: &str,
    size_bytes: u64,
) -> ModelManifest {
    ModelManifest {
        schema_version: 1,
        model_id: model_id.to_owned(),
        version: "1.0".to_owned(),
        tasks: vec![AiTask::Transcribe],
        provider_id: "whisper.cpp".to_owned(),
        runtime: "whisper.cpp/1.9.4".to_owned(),
        source: format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/{WHISPER_REVISION}/{artifact}"
        ),
        sha256: sha256.to_owned(),
        size_bytes,
        license: "MIT".to_owned(),
        provenance: format!("ggerganov/whisper.cpp revision {WHISPER_REVISION}"),
        hardware_requirements: vec!["CPU".to_owned()],
        languages: vec!["vi".to_owned(), "en".to_owned()],
    }
}

/// The selectable whisper models, fast to best.
///
/// Base and Small are comfortable CPU downloads; Medium and Large v3 Turbo are
/// large but materially better for Vietnamese. The quantised Turbo is the best
/// quality-per-byte option because it keeps the large-v3 family while fitting
/// under 600 MiB.
pub fn whisper_models() -> Vec<ModelManifest> {
    vec![
        whisper_manifest(
            "whisper-base-multilingual",
            "ggml-base.bin",
            "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
            147_951_465,
        ),
        whisper_manifest(
            "whisper-small-multilingual",
            "ggml-small.bin",
            "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
            487_601_967,
        ),
        whisper_manifest(
            "whisper-medium-multilingual",
            "ggml-medium.bin",
            "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
            1_533_763_059,
        ),
        whisper_manifest(
            "whisper-large-v3-turbo-q5",
            "ggml-large-v3-turbo-q5_0.bin",
            "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
            574_041_195,
        ),
        whisper_manifest(
            "whisper-large-v3-turbo",
            "ggml-large-v3-turbo.bin",
            "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
            1_624_555_275,
        ),
    ]
}

/// The default model: small is the best balance for a first local install.
pub fn default_whisper_model() -> ModelManifest {
    whisper_models()
        .into_iter()
        .find(|model| model.model_id == "whisper-small-multilingual")
        .expect("the default whisper model is in the catalog")
}

/// Looks up one catalog entry by id. Unknown ids are refused rather than
/// guessed at, so a stale preference cannot download an arbitrary artifact.
pub fn whisper_model(model_id: &str) -> Option<ModelManifest> {
    whisper_models()
        .into_iter()
        .find(|model| model.model_id == model_id)
}

/// A short human label for the picker; ids remain the stable wire identity.
pub fn whisper_model_label(model_id: &str) -> &'static str {
    match model_id {
        "whisper-base-multilingual" => "Base - fastest",
        "whisper-small-multilingual" => "Small - balanced",
        "whisper-medium-multilingual" => "Medium - accurate",
        "whisper-large-v3-turbo-q5" => "Large v3 Turbo q5 - best value",
        "whisper-large-v3-turbo" => "Large v3 Turbo - best",
        _ => "Local speech model",
    }
}

impl ModelManifest {
    pub fn validate(&self) -> Result<(), ModelValidationError> {
        if self.schema_version != 1
            || !valid_component(&self.model_id)
            || !valid_component(&self.version)
            || !bounded_text(&self.provider_id)
            || !bounded_text(&self.runtime)
            || !self.source.starts_with("https://")
            || !bounded_text(&self.source)
            || !bounded_text(&self.license)
            || !bounded_text(&self.provenance)
            || self.sha256.len() != 64
            || !self.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || self.size_bytes == 0
            || self.size_bytes > MAX_MODEL_ARTIFACT_BYTES
            || self.tasks.is_empty()
            || self.tasks.len() > 32
            || self.hardware_requirements.len() > 32
            || self.languages.len() > 64
            || self
                .hardware_requirements
                .iter()
                .chain(&self.languages)
                .any(|value| !bounded_text(value))
        {
            return Err(ModelValidationError::InvalidManifest);
        }
        Ok(())
    }

    fn installed_path(&self, model_root: &Path) -> PathBuf {
        // Length prefixes keep the pair unambiguous even when both safe
        // components contain hyphens.
        model_root.join(format!(
            "{}-{}-{}-{}.model",
            self.model_id.len(),
            self.model_id,
            self.version.len(),
            self.version
        ))
    }
}

/// The verified model identity and managed path returned after installation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledModel {
    pub model_id: String,
    pub version: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelValidationError {
    InvalidManifest,
    ArtifactTooLarge,
    SizeMismatch,
    DigestMismatch,
    AlreadyInstalled,
    StoreBusy,
    Cancelled,
    Io,
}

impl std::fmt::Display for ModelValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidManifest => "model manifest is invalid",
            Self::ArtifactTooLarge => "model artifact exceeds its manifest size",
            Self::SizeMismatch => "model artifact size does not match its manifest",
            Self::DigestMismatch => "model artifact checksum does not match its manifest",
            Self::AlreadyInstalled => "this model version is already installed",
            Self::StoreBusy => "another process is using the model store",
            Self::Cancelled => "model installation was cancelled",
            Self::Io => "model artifact could not be safely installed or read",
        })
    }
}

impl std::error::Error for ModelValidationError {}

/// Streams a verified artifact into the managed model directory and publishes
/// it atomically. The caller owns HTTPS download policy and runs this work off
/// the UI thread.
pub fn install_verified_model(
    model_root: &Path,
    manifest: &ModelManifest,
    mut artifact: impl Read,
    mut cancelled: impl FnMut() -> bool,
) -> Result<InstalledModel, ModelValidationError> {
    manifest.validate()?;
    fs::create_dir_all(model_root).map_err(|_| ModelValidationError::Io)?;
    let _store_lock = lock_model_store(model_root)?;
    let destination = manifest.installed_path(model_root);
    if destination.exists() {
        return Err(ModelValidationError::AlreadyInstalled);
    }
    let (temporary_path, mut output) = create_staging_file(model_root)?;
    let mut cleanup = StagingFile(Some(temporary_path.clone()));
    copy_and_verify(&mut artifact, &mut output, manifest, &mut cancelled)?;
    output.sync_all().map_err(|_| ModelValidationError::Io)?;
    drop(output);
    if cancelled() {
        return Err(ModelValidationError::Cancelled);
    }
    // hard_link fails atomically if the destination already exists, unlike
    // rename on platforms where rename would replace an existing model.
    fs::hard_link(&temporary_path, &destination).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            ModelValidationError::AlreadyInstalled
        } else {
            ModelValidationError::Io
        }
    })?;
    if fs::remove_file(&temporary_path).is_err() {
        let _ = fs::remove_file(&destination);
        return Err(ModelValidationError::Io);
    }
    cleanup.0 = None;
    Ok(InstalledModel {
        model_id: manifest.model_id.clone(),
        version: manifest.version.clone(),
        path: destination,
        size_bytes: manifest.size_bytes,
        sha256: manifest.sha256.to_ascii_lowercase(),
    })
}

/// Removes abandoned staging files after an unclean shutdown. An exclusive
/// cross-process lock prevents removal while another installer is active.
pub fn cleanup_abandoned_model_staging(model_root: &Path) -> Result<usize, ModelValidationError> {
    if !model_root.exists() {
        return Ok(0);
    }
    let _store_lock = lock_model_store(model_root)?;
    let entries = match fs::read_dir(model_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(_) => return Err(ModelValidationError::Io),
    };
    let mut removed = 0;
    for entry in entries {
        let entry = entry.map_err(|_| ModelValidationError::Io)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with(".or-model-") && name.ends_with(".part") {
            fs::remove_file(entry.path()).map_err(|_| ModelValidationError::Io)?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn lock_model_store(model_root: &Path) -> Result<File, ModelValidationError> {
    let lock_path = model_root.join(".or-model-store.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|_| ModelValidationError::Io)?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(ModelValidationError::StoreBusy),
        Err(TryLockError::Error(_)) => Err(ModelValidationError::Io),
    }
}

/// A fast presence check for pickers and catalogs.
///
/// It checks the managed file's identity and exact size without hashing a
/// multi-gigabyte artifact. The transcription job still calls
/// [`verify_installed_model`] before running, so a corrupted file is caught
/// where it matters rather than on every catalog render.
pub fn model_artifact_present(model_root: &Path, manifest: &ModelManifest) -> bool {
    if manifest.validate().is_err() {
        return false;
    }
    fs::metadata(manifest.installed_path(model_root))
        .map(|metadata| metadata.is_file() && metadata.len() == manifest.size_bytes)
        .unwrap_or(false)
}

pub fn verify_installed_model(
    model_root: &Path,
    manifest: &ModelManifest,
) -> Result<InstalledModel, ModelValidationError> {
    manifest.validate()?;
    let _store_lock = lock_model_store(model_root)?;
    let path = manifest.installed_path(model_root);
    let file = File::open(&path).map_err(|_| ModelValidationError::Io)?;
    let metadata = file.metadata().map_err(|_| ModelValidationError::Io)?;
    if metadata.len() != manifest.size_bytes {
        return Err(ModelValidationError::SizeMismatch);
    }
    let digest = hash_reader(file)?;
    if !digest.eq_ignore_ascii_case(&manifest.sha256) {
        return Err(ModelValidationError::DigestMismatch);
    }
    Ok(InstalledModel {
        model_id: manifest.model_id.clone(),
        version: manifest.version.clone(),
        path,
        size_bytes: manifest.size_bytes,
        sha256: digest,
    })
}

pub fn remove_installed_model(
    model_root: &Path,
    manifest: &ModelManifest,
) -> Result<(), ModelValidationError> {
    manifest.validate()?;
    let _store_lock = lock_model_store(model_root)?;
    fs::remove_file(manifest.installed_path(model_root)).map_err(|_| ModelValidationError::Io)
}

fn copy_and_verify(
    input: &mut impl Read,
    output: &mut impl Write,
    manifest: &ModelManifest,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(), ModelValidationError> {
    let mut buffer = [0_u8; 64 * 1024];
    let mut size = 0_u64;
    let mut hasher = Sha256::new();
    loop {
        if cancelled() {
            return Err(ModelValidationError::Cancelled);
        }
        let read = input
            .read(&mut buffer)
            .map_err(|_| ModelValidationError::Io)?;
        if read == 0 {
            break;
        }
        size = size
            .checked_add(read as u64)
            .ok_or(ModelValidationError::ArtifactTooLarge)?;
        if size > manifest.size_bytes || size > MAX_MODEL_ARTIFACT_BYTES {
            return Err(ModelValidationError::ArtifactTooLarge);
        }
        hasher.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(|_| ModelValidationError::Io)?;
    }
    if size != manifest.size_bytes {
        return Err(ModelValidationError::SizeMismatch);
    }
    let digest = hex_digest(hasher.finalize().as_slice());
    if !digest.eq_ignore_ascii_case(&manifest.sha256) {
        return Err(ModelValidationError::DigestMismatch);
    }
    Ok(())
}

fn hash_reader(mut reader: impl Read) -> Result<String, ModelValidationError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| ModelValidationError::Io)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(hasher.finalize().as_slice()))
}

fn hex_digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

fn create_staging_file(model_root: &Path) -> Result<(PathBuf, File), ModelValidationError> {
    for _ in 0..8 {
        let path = model_root.join(format!(".or-model-{}.part", Uuid::new_v4()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(ModelValidationError::Io),
        }
    }
    Err(ModelValidationError::Io)
}

struct StagingFile(Option<PathBuf>);

impl Drop for StagingFile {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && value != "."
        && value != ".."
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_MANIFEST_TEXT_BYTES
        && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Cursor};

    use super::*;

    const ARTIFACT: &[u8] = b"verified-model-bytes";

    fn manifest() -> ModelManifest {
        ModelManifest {
            schema_version: 1,
            model_id: "whisper-tiny-en".to_owned(),
            version: "1.0".to_owned(),
            tasks: vec![AiTask::Transcribe],
            provider_id: "whisper.cpp".to_owned(),
            runtime: "whisper.cpp/1.9.4".to_owned(),
            source: "https://example.invalid/model.bin".to_owned(),
            sha256: hex_digest(Sha256::digest(ARTIFACT).as_slice()),
            size_bytes: ARTIFACT.len() as u64,
            license: "MIT".to_owned(),
            provenance: "pinned upstream revision".to_owned(),
            hardware_requirements: vec!["cpu".to_owned()],
            languages: vec!["en".to_owned()],
        }
    }

    #[test]
    fn shipped_transcription_manifest_matches_the_reviewed_artifact_identity() {
        let manifest = default_whisper_model();
        manifest.validate().unwrap();
        assert_eq!(manifest.model_id, "whisper-small-multilingual");
        assert_eq!(manifest.runtime, "whisper.cpp/1.9.4");
        assert_eq!(manifest.size_bytes, 487_601_967);
        assert_eq!(
            manifest.sha256,
            "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b"
        );
        assert_eq!(manifest.license, "MIT");
        assert_eq!(manifest.languages, ["vi", "en"]);
    }

    #[test]
    fn catalog_is_unique_valid_and_ordered_fast_to_best() {
        let models = whisper_models();
        assert_eq!(models.len(), 5);
        let mut ids: Vec<&str> = models.iter().map(|model| model.model_id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), models.len(), "model ids are unique");
        for model in &models {
            model.validate().unwrap();
            assert_eq!(model.license, "MIT");
            assert_eq!(model.languages, ["vi", "en"]);
            assert!(model.source.contains("/resolve/"));
        }
        assert_eq!(
            models.first().unwrap().model_id,
            "whisper-base-multilingual"
        );
        assert_eq!(models.last().unwrap().model_id, "whisper-large-v3-turbo");
        assert_eq!(
            default_whisper_model().model_id,
            "whisper-small-multilingual"
        );
        assert!(whisper_model("missing-model").is_none());
        assert_eq!(
            whisper_model_label("whisper-medium-multilingual"),
            "Medium - accurate"
        );
    }

    fn directory() -> PathBuf {
        let directory = std::env::temp_dir().join(format!("or-ai-model-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn assert_no_staging_files(root: &Path) {
        assert!(fs::read_dir(root).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".part")
        }));
    }

    #[test]
    fn install_verify_and_remove_model_artifact() {
        let root = directory();
        let manifest = manifest();
        let installed =
            install_verified_model(&root, &manifest, Cursor::new(ARTIFACT), || false).unwrap();
        assert_eq!(fs::read(&installed.path).unwrap(), ARTIFACT);
        assert_eq!(verify_installed_model(&root, &manifest).unwrap(), installed);
        remove_installed_model(&root, &manifest).unwrap();
        assert!(!manifest.installed_path(&root).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn model_identity_filename_is_unambiguous() {
        let root = directory();
        let mut left = manifest();
        left.model_id = "a-b".to_owned();
        left.version = "c".to_owned();
        let mut right = manifest();
        right.model_id = "a".to_owned();
        right.version = "b-c".to_owned();
        assert_ne!(left.installed_path(&root), right.installed_path(&root));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn startup_cleanup_removes_only_managed_staging_files() {
        let root = directory();
        fs::write(root.join(".or-model-abandoned.part"), b"partial").unwrap();
        fs::write(root.join("user-file.part"), b"keep").unwrap();
        assert_eq!(cleanup_abandoned_model_staging(&root).unwrap(), 1);
        assert!(!root.join(".or-model-abandoned.part").exists());
        assert!(root.join("user-file.part").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn active_model_store_lock_prevents_cleanup_and_second_install() {
        let root = directory();
        let lock = lock_model_store(&root).unwrap();
        fs::write(root.join(".or-model-active.part"), b"active").unwrap();
        assert_eq!(
            cleanup_abandoned_model_staging(&root),
            Err(ModelValidationError::StoreBusy)
        );
        assert_eq!(
            install_verified_model(&root, &manifest(), Cursor::new(ARTIFACT), || false),
            Err(ModelValidationError::StoreBusy)
        );
        assert_eq!(
            verify_installed_model(&root, &manifest()),
            Err(ModelValidationError::StoreBusy)
        );
        assert_eq!(
            remove_installed_model(&root, &manifest()),
            Err(ModelValidationError::StoreBusy)
        );
        assert!(root.join(".or-model-active.part").exists());
        drop(lock);
        assert_eq!(cleanup_abandoned_model_staging(&root).unwrap(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_artifacts_never_replace_or_leave_partial_model_files() {
        let root = directory();
        let mut bad_digest = manifest();
        bad_digest.sha256 = "0".repeat(64);
        assert_eq!(
            install_verified_model(&root, &bad_digest, Cursor::new(ARTIFACT), || false),
            Err(ModelValidationError::DigestMismatch)
        );
        assert_no_staging_files(&root);
        assert_eq!(
            install_verified_model(&root, &manifest(), Cursor::new(ARTIFACT), || true),
            Err(ModelValidationError::Cancelled)
        );
        assert_no_staging_files(&root);
        let mut checks = 0;
        assert_eq!(
            install_verified_model(&root, &manifest(), Cursor::new(ARTIFACT), || {
                checks += 1;
                checks == 3
            }),
            Err(ModelValidationError::Cancelled),
            "cancellation after verification but before publication must not publish"
        );
        assert_no_staging_files(&root);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn install_rejects_oversize_wrong_size_and_existing_version() {
        let root = directory();
        let mut too_large = manifest();
        too_large.size_bytes = MAX_MODEL_ARTIFACT_BYTES + 1;
        assert_eq!(
            too_large.validate(),
            Err(ModelValidationError::InvalidManifest)
        );

        let mut wrong_size = manifest();
        wrong_size.size_bytes += 1;
        assert_eq!(
            install_verified_model(&root, &wrong_size, Cursor::new(ARTIFACT), || false),
            Err(ModelValidationError::SizeMismatch)
        );
        let manifest = manifest();
        install_verified_model(&root, &manifest, Cursor::new(ARTIFACT), || false).unwrap();
        assert_eq!(
            install_verified_model(&root, &manifest, Cursor::new(ARTIFACT), || false),
            Err(ModelValidationError::AlreadyInstalled)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn manifest_rejects_path_components_and_non_https_sources() {
        let mut candidate = manifest();
        candidate.model_id = "../escape".to_owned();
        assert_eq!(
            candidate.validate(),
            Err(ModelValidationError::InvalidManifest)
        );
        candidate = manifest();
        candidate.source = "file:///tmp/model.bin".to_owned();
        assert_eq!(
            candidate.validate(),
            Err(ModelValidationError::InvalidManifest)
        );
    }
}
