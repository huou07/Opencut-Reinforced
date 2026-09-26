use crate::{
    MediaId, MediaItem, MediaMetadataValidationError, MediaProbeError, MediaSourceRef,
    MediaSourceUri, MediaSourceUriError, probe_media_file,
};
use std::{error::Error, fmt, fs, io, path::Path};

/// A failure while preparing, but not yet adding, one local media source.
#[derive(Debug)]
pub enum MediaImportError {
    SourceNotFound,
    SourceUnavailable,
    Probe(MediaProbeError),
    InvalidSourceUri(MediaSourceUriError),
    InvalidMetadata(MediaMetadataValidationError),
}

impl MediaImportError {
    pub const fn code_str(&self) -> &'static str {
        match self {
            Self::SourceNotFound => "SOURCE_NOT_FOUND",
            Self::SourceUnavailable => "SOURCE_UNAVAILABLE",
            Self::Probe(error) => error.code_str(),
            Self::InvalidSourceUri(MediaSourceUriError::Invalid) => "INVALID_SOURCE_URI",
            Self::InvalidSourceUri(MediaSourceUriError::TooLong) => "SOURCE_URI_TOO_LONG",
            Self::InvalidMetadata(_) => "INVALID_MEDIA_METADATA",
        }
    }

    pub const fn message(&self) -> &'static str {
        match self {
            Self::SourceNotFound => "media source path was not found",
            Self::SourceUnavailable => "media source path could not be resolved",
            Self::Probe(error) => error.message(),
            Self::InvalidSourceUri(MediaSourceUriError::Invalid) => {
                "media source could not be represented as a local file URI"
            }
            Self::InvalidSourceUri(MediaSourceUriError::TooLong) => {
                "media source URI exceeds the configured byte limit"
            }
            Self::InvalidMetadata(_) => "media probe returned invalid media metadata",
        }
    }

    pub fn diagnostic(&self) -> Option<&str> {
        match self {
            Self::Probe(error) => error.diagnostic(),
            _ => None,
        }
    }
}

impl fmt::Display for MediaImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl Error for MediaImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Probe(error) => Some(error),
            Self::InvalidSourceUri(error) => Some(error),
            Self::InvalidMetadata(error) => Some(error),
            Self::SourceNotFound | Self::SourceUnavailable => None,
        }
    }
}

/// Probes and validates one local file, returning a candidate library item.
///
/// The file is canonicalized before probing so the persisted URI refers to the
/// exact native file that was inspected. This function does not mutate a
/// project; callers must submit the returned item through `media.add`.
pub fn prepare_media_import(path: &Path) -> Result<MediaItem, MediaImportError> {
    let canonical_path = fs::canonicalize(path).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => MediaImportError::SourceNotFound,
        _ => MediaImportError::SourceUnavailable,
    })?;
    let metadata = probe_media_file(&canonical_path).map_err(MediaImportError::Probe)?;
    let source_uri = MediaSourceUri::from_canonical_path(&canonical_path)
        .map_err(MediaImportError::InvalidSourceUri)?;
    let source = MediaSourceRef::LocalFile { uri: source_uri };
    MediaItem::new(MediaId::generate(), source, metadata).map_err(MediaImportError::InvalidMetadata)
}

#[cfg(test)]
mod tests {
    use super::{MediaImportError, prepare_media_import};
    use crate::MediaId;
    use std::env;

    #[test]
    fn missing_path_is_rejected_before_probing() {
        let path = env::temp_dir().join(format!("or-missing-{}.mkv", MediaId::generate()));

        let error = prepare_media_import(&path).unwrap_err();

        assert!(matches!(error, MediaImportError::SourceNotFound));
        assert_eq!(error.code_str(), "SOURCE_NOT_FOUND");
    }

    #[test]
    fn non_regular_path_is_rejected_before_spawning_ffprobe() {
        let error = prepare_media_import(&env::temp_dir()).unwrap_err();

        assert!(matches!(
            error,
            MediaImportError::Probe(ref probe) if probe.code_str() == "MEDIA_NOT_REGULAR_FILE"
        ));
    }
}
