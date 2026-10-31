use crate::{
    MediaId, MediaItem, MediaMetadataValidationError, MediaProbeError, MediaSourceRef,
    MediaSourceUri, MediaSourceUriError, parse_media_probe_output, probe_media_file,
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
    /// The file passed the import matrix, but this build has no decoder for one
    /// of its streams, so admitting it would create a clip that cannot play.
    DecoderUnavailable {
        codec: String,
    },
}

impl MediaImportError {
    pub const fn code_str(&self) -> &'static str {
        match self {
            Self::SourceNotFound => "SOURCE_NOT_FOUND",
            Self::SourceUnavailable => "SOURCE_UNAVAILABLE",
            Self::Probe(error) => error.code_str(),
            Self::InvalidSourceUri(
                MediaSourceUriError::Invalid
                | MediaSourceUriError::InvalidSafDocumentUri
                | MediaSourceUriError::NotLocalFile,
            ) => "INVALID_SOURCE_URI",
            Self::InvalidSourceUri(MediaSourceUriError::TooLong) => "SOURCE_URI_TOO_LONG",
            Self::InvalidMetadata(_) => "INVALID_MEDIA_METADATA",
            Self::DecoderUnavailable { .. } => "UNSUPPORTED_CODEC",
        }
    }

    pub const fn message(&self) -> &'static str {
        match self {
            Self::SourceNotFound => "media source was not found",
            Self::SourceUnavailable => "media source could not be opened",
            Self::Probe(error) => error.message(),
            Self::InvalidSourceUri(
                MediaSourceUriError::Invalid
                | MediaSourceUriError::InvalidSafDocumentUri
                | MediaSourceUriError::NotLocalFile,
            ) => "media source could not be represented as a local file URI",
            Self::InvalidSourceUri(MediaSourceUriError::TooLong) => {
                "media source URI exceeds the configured byte limit"
            }
            Self::InvalidMetadata(_) => "media probe returned invalid media metadata",
            Self::DecoderUnavailable { .. } => {
                "this build has no decoder for one of the file's media codecs"
            }
        }
    }

    pub fn diagnostic(&self) -> Option<&str> {
        match self {
            Self::Probe(error) => error.diagnostic(),
            Self::DecoderUnavailable { codec } => Some(codec),
            _ => None,
        }
    }
}

impl fmt::Display for MediaImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Probe(error) => fmt::Display::fmt(error, formatter),
            Self::DecoderUnavailable { codec } => {
                write!(formatter, "no decoder for codec {codec} in this build")
            }
            _ => formatter.write_str(self.message()),
        }
    }
}

impl Error for MediaImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Probe(error) => Some(error),
            Self::InvalidSourceUri(error) => Some(error),
            Self::InvalidMetadata(error) => Some(error),
            Self::SourceNotFound | Self::SourceUnavailable | Self::DecoderUnavailable { .. } => {
                None
            }
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
    let source_uri = MediaSourceUri::from_file_path(&canonical_path)
        .map_err(MediaImportError::InvalidSourceUri)?;
    let source = MediaSourceRef::FileUri { uri: source_uri };
    MediaItem::new(MediaId::generate(), source, metadata).map_err(MediaImportError::InvalidMetadata)
}

/// Builds a candidate import from a probe this process produced itself.
///
/// The packaged `ffprobe` helper normally owns probing. When that helper cannot
/// run, the application's linked libavformat can still inspect the file; this
/// entry point applies the same source identity, size and import-matrix rules to
/// that output, so the fallback cannot admit anything the helper would reject.
pub fn prepare_media_import_with_probe(
    path: &Path,
    probe_output: &[u8],
) -> Result<MediaItem, MediaImportError> {
    let canonical_path = fs::canonicalize(path).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => MediaImportError::SourceNotFound,
        _ => MediaImportError::SourceUnavailable,
    })?;
    let file_size_bytes = fs::metadata(&canonical_path)
        .map_err(|_| MediaImportError::SourceUnavailable)?
        .len();
    let metadata =
        parse_media_probe_output(probe_output, file_size_bytes).map_err(MediaImportError::Probe)?;
    let source_uri = MediaSourceUri::from_file_path(&canonical_path)
        .map_err(MediaImportError::InvalidSourceUri)?;
    MediaItem::new(
        MediaId::generate(),
        MediaSourceRef::FileUri { uri: source_uri },
        metadata,
    )
    .map_err(MediaImportError::InvalidMetadata)
}

/// Builds a candidate library item from a previously probed, already-validated
/// source reference. The caller must submit it through `media.add`.
pub fn prepare_media_import_from_probe(
    source: MediaSourceRef,
    probe_output: &[u8],
    file_size_bytes: u64,
) -> Result<MediaItem, MediaImportError> {
    let metadata = crate::parse_media_probe_output(probe_output, file_size_bytes)
        .map_err(MediaImportError::Probe)?;
    MediaItem::new(MediaId::generate(), source, metadata).map_err(MediaImportError::InvalidMetadata)
}

#[cfg(test)]
mod tests {
    use super::{MediaImportError, prepare_media_import, prepare_media_import_from_probe};
    use crate::{MediaId, MediaSourceRef};
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

    #[test]
    fn an_in_process_probe_is_held_to_the_same_import_matrix() {
        let output = br#"{"format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2","duration":"1.5"},"streams":[{"index":0,"codec_type":"video","codec_name":"h264","width":32,"height":24,"pix_fmt":"yuv420p","avg_frame_rate":"30/1","duration":"1.5"},{"index":1,"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2,"channel_layout":"stereo","duration":"1.5"}]}"#;
        let path = std::env::temp_dir().join(format!("or-probe-{}.mp4", MediaId::generate()));
        std::fs::write(&path, b"probe fixture").unwrap();
        let item = super::prepare_media_import_with_probe(&path, output).unwrap();
        assert_eq!(item.metadata().file_size_bytes(), 13);
        assert!(item.source().uri().starts_with("file:///"));
        std::fs::remove_file(&path).unwrap();

        let unsupported = br#"{"format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2","duration":"1.5"},"streams":[{"index":0,"codec_type":"video","codec_name":"hap","width":32,"height":24}]}"#;
        let path = std::env::temp_dir().join(format!("or-probe-{}.mp4", MediaId::generate()));
        std::fs::write(&path, b"probe fixture").unwrap();
        let error = super::prepare_media_import_with_probe(&path, unsupported).unwrap_err();
        assert_eq!(error.code_str(), "UNSUPPORTED_CODEC");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn probed_saf_source_stays_a_document_uri_in_the_candidate_item() {
        let source = MediaSourceRef::android_saf_document_uri(
            "content://com.example.documents/document/video%3A42",
        )
        .unwrap();
        let output = br#"{"format":{"format_name":"matroska,webm","duration":"1.000000"},"streams":[{"index":0,"codec_type":"video","codec_name":"ffv1","width":16,"height":16,"pix_fmt":null,"avg_frame_rate":"24/1","duration":null},{"index":1,"codec_type":"audio","codec_name":"pcm_s16le","sample_rate":48000,"channels":2,"channel_layout":null,"duration":null}]}"#;

        let item = prepare_media_import_from_probe(source, output, 128).unwrap();

        assert!(matches!(
            item.source(),
            MediaSourceRef::AndroidSafDocumentUri { .. }
        ));
        assert!(item.source().to_file_path().is_err());
        assert_eq!(item.metadata().file_size_bytes(), 128);
    }
}
