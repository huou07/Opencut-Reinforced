use crate::{RationalRate, RationalTime, UuidV4ParseError};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{SeqAccess, Visitor},
};
use std::{fmt, num::NonZeroU32, path::Path, str::FromStr};
use url::Url;
use uuid::{Uuid, Version};

const MAX_DECIMAL_FRACTION_DIGITS: usize = 9;
pub const MAX_MEDIA_SOURCE_URI_BYTES: usize = 8192;
pub const MAX_MEDIA_FORMAT_NAMES: usize = 32;
pub const MAX_MEDIA_FORMAT_NAME_BYTES: usize = 256;
pub const MAX_MEDIA_STREAMS: usize = 4096;
pub const MAX_MEDIA_CODEC_NAME_BYTES: usize = 256;
pub const MAX_MEDIA_CODEC_TYPE_BYTES: usize = 128;
pub const MAX_MEDIA_PIXEL_FORMAT_BYTES: usize = 128;
pub const MAX_MEDIA_CHANNEL_LAYOUT_BYTES: usize = 256;

/// Opaque persistent UUIDv4 identity for an imported project media item.
///
/// Probing a file does not create or assign a `MediaId`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MediaId(Uuid);

impl MediaId {
    /// Generates a new UUID version 4 media identity.
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for MediaId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, formatter)
    }
}

impl FromStr for MediaId {
    type Err = UuidV4ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(value).map_err(UuidV4ParseError::InvalidUuid)?;
        validate_v4(uuid).map(Self)
    }
}

impl<'de> Deserialize<'de> for MediaId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let uuid = Uuid::deserialize(deserializer)?;
        validate_v4(uuid)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

/// A validated file URI for a local project media source.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct MediaSourceUri(String);

impl MediaSourceUri {
    pub fn parse(uri: impl AsRef<str>) -> Result<Self, MediaSourceUriError> {
        let uri = uri.as_ref();
        if uri.len() > MAX_MEDIA_SOURCE_URI_BYTES {
            return Err(MediaSourceUriError::TooLong);
        }
        let parsed = Url::parse(uri).map_err(|_| MediaSourceUriError::Invalid)?;
        if uri != parsed.as_str()
            || !has_valid_percent_escapes(uri)
            || parsed.scheme() != "file"
            || parsed.cannot_be_a_base()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.port().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed
                .host_str()
                .is_some_and(|host| !host.is_empty() && !host.eq_ignore_ascii_case("localhost"))
            || parsed.to_file_path().is_err()
        {
            return Err(MediaSourceUriError::Invalid);
        }
        let normalized = parsed.as_str().to_owned();
        if normalized.len() > MAX_MEDIA_SOURCE_URI_BYTES {
            return Err(MediaSourceUriError::TooLong);
        }
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn from_canonical_path(path: &Path) -> Result<Self, MediaSourceUriError> {
        let uri = Url::from_file_path(path).map_err(|_| MediaSourceUriError::Invalid)?;
        Self::parse(uri.as_str())
    }
}

fn has_valid_percent_escapes(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

impl<'de> Deserialize<'de> for MediaSourceUri {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let uri = String::deserialize(deserializer)?;
        Self::parse(uri).map_err(serde::de::Error::custom)
    }
}

/// A media source reference persisted in a project.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MediaSourceRef {
    LocalFile { uri: MediaSourceUri },
}

impl MediaSourceRef {
    pub fn local_file(uri: impl AsRef<str>) -> Result<Self, MediaSourceUriError> {
        MediaSourceUri::parse(uri).map(|uri| Self::LocalFile { uri })
    }

    pub fn uri(&self) -> &str {
        match self {
            Self::LocalFile { uri } => uri.as_str(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaSourceUriError {
    Invalid,
    TooLong,
}

impl fmt::Display for MediaSourceUriError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "media source must be a valid local file URI",
            Self::TooLong => "media source URI exceeds the configured byte limit",
        })
    }
}

impl std::error::Error for MediaSourceUriError {}

/// One persistent item in the project's ordered media library.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaItem {
    id: MediaId,
    source: MediaSourceRef,
    metadata: MediaMetadata,
}

impl MediaItem {
    pub fn new(
        id: MediaId,
        source: MediaSourceRef,
        metadata: MediaMetadata,
    ) -> Result<Self, MediaMetadataValidationError> {
        metadata.validate()?;
        Ok(Self {
            id,
            source,
            metadata,
        })
    }

    pub const fn id(&self) -> MediaId {
        self.id
    }

    pub fn source(&self) -> &MediaSourceRef {
        &self.source
    }

    pub fn metadata(&self) -> &MediaMetadata {
        &self.metadata
    }
}

fn validate_v4(uuid: Uuid) -> Result<Uuid, UuidV4ParseError> {
    if uuid.get_version() == Some(Version::Random) {
        Ok(uuid)
    } else {
        Err(UuidV4ParseError::NotVersion4)
    }
}

/// Control-plane metadata returned by a read-only local media probe.
///
/// The source path and arbitrary container tags are intentionally omitted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MediaMetadata {
    format_names: Vec<String>,
    duration: Option<RationalTime>,
    file_size_bytes: u64,
    streams: Vec<MediaStreamMetadata>,
}

impl MediaMetadata {
    pub(crate) fn from_probe(
        format_names: Vec<String>,
        duration: Option<RationalTime>,
        file_size_bytes: u64,
        streams: Vec<MediaStreamMetadata>,
    ) -> Self {
        Self {
            format_names,
            duration,
            file_size_bytes,
            streams,
        }
    }

    pub fn format_names(&self) -> &[String] {
        &self.format_names
    }

    pub const fn duration(&self) -> Option<RationalTime> {
        self.duration
    }

    pub const fn file_size_bytes(&self) -> u64 {
        self.file_size_bytes
    }

    pub fn streams(&self) -> &[MediaStreamMetadata] {
        &self.streams
    }

    pub fn validate(&self) -> Result<(), MediaMetadataValidationError> {
        if self.format_names.len() > MAX_MEDIA_FORMAT_NAMES
            || self
                .format_names
                .iter()
                .any(|value| value.len() > MAX_MEDIA_FORMAT_NAME_BYTES)
            || self.streams.len() > MAX_MEDIA_STREAMS
            || self.duration.is_some_and(RationalTime::is_negative)
        {
            return Err(MediaMetadataValidationError::Invalid);
        }
        for stream in &self.streams {
            let valid = match stream {
                MediaStreamMetadata::Video(stream) => {
                    bounded_optional(stream.codec_name.as_deref(), MAX_MEDIA_CODEC_NAME_BYTES)
                        && bounded_optional(
                            stream.pixel_format.as_deref(),
                            MAX_MEDIA_PIXEL_FORMAT_BYTES,
                        )
                        && stream.duration.is_none_or(|time| !time.is_negative())
                }
                MediaStreamMetadata::Audio(stream) => {
                    bounded_optional(stream.codec_name.as_deref(), MAX_MEDIA_CODEC_NAME_BYTES)
                        && bounded_optional(
                            stream.channel_layout.as_deref(),
                            MAX_MEDIA_CHANNEL_LAYOUT_BYTES,
                        )
                        && stream.duration.is_none_or(|time| !time.is_negative())
                }
                MediaStreamMetadata::Other(stream) => {
                    bounded_optional(stream.codec_name.as_deref(), MAX_MEDIA_CODEC_NAME_BYTES)
                        && bounded_optional(
                            stream.codec_type.as_deref(),
                            MAX_MEDIA_CODEC_TYPE_BYTES,
                        )
                }
            };
            if !valid {
                return Err(MediaMetadataValidationError::Invalid);
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaMetadataRepr {
    #[serde(deserialize_with = "deserialize_bounded_format_names")]
    format_names: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nonnegative_duration")]
    duration: Option<RationalTime>,
    file_size_bytes: u64,
    #[serde(deserialize_with = "deserialize_bounded_streams")]
    streams: Vec<MediaStreamMetadata>,
}

impl<'de> Deserialize<'de> for MediaMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = MediaMetadataRepr::deserialize(deserializer)?;
        let metadata = Self {
            format_names: repr.format_names,
            duration: repr.duration,
            file_size_bytes: repr.file_size_bytes,
            streams: repr.streams,
        };
        metadata.validate().map_err(serde::de::Error::custom)?;
        Ok(metadata)
    }
}

fn deserialize_bounded_format_names<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let values: Vec<String> =
        deserialize_limited_vec(deserializer, MAX_MEDIA_FORMAT_NAMES, "format names")?;
    if values
        .iter()
        .any(|value| value.len() > MAX_MEDIA_FORMAT_NAME_BYTES)
    {
        return Err(serde::de::Error::custom(
            "format names exceed media metadata limits",
        ));
    }
    Ok(values)
}

fn deserialize_bounded_streams<'de, D>(
    deserializer: D,
) -> Result<Vec<MediaStreamMetadata>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_limited_vec(deserializer, MAX_MEDIA_STREAMS, "media streams")
}

pub(crate) fn deserialize_limited_vec<'de, D, T>(
    deserializer: D,
    max_items: usize,
    label: &'static str,
) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct LimitedVecVisitor<T> {
        max_items: usize,
        label: &'static str,
        marker: std::marker::PhantomData<T>,
    }

    impl<'de, T: Deserialize<'de>> Visitor<'de> for LimitedVecVisitor<T> {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "at most {} {}", self.max_items, self.label)
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let capacity = sequence.size_hint().unwrap_or(0).min(self.max_items);
            let mut values = Vec::with_capacity(capacity);
            while let Some(value) = sequence.next_element()? {
                if values.len() == self.max_items {
                    return Err(serde::de::Error::custom(format!(
                        "{} exceed configured limit",
                        self.label
                    )));
                }
                values.push(value);
            }
            Ok(values)
        }
    }

    deserializer.deserialize_seq(LimitedVecVisitor {
        max_items,
        label,
        marker: std::marker::PhantomData,
    })
}

pub(crate) fn deserialize_codec_name<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_optional_string(deserializer, MAX_MEDIA_CODEC_NAME_BYTES, "codec name")
}

pub(crate) fn deserialize_codec_type<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_optional_string(deserializer, MAX_MEDIA_CODEC_TYPE_BYTES, "codec type")
}

pub(crate) fn deserialize_pixel_format<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_optional_string(deserializer, MAX_MEDIA_PIXEL_FORMAT_BYTES, "pixel format")
}

pub(crate) fn deserialize_channel_layout<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_bounded_optional_string(
        deserializer,
        MAX_MEDIA_CHANNEL_LAYOUT_BYTES,
        "channel layout",
    )
}

fn deserialize_bounded_optional_string<'de, D>(
    deserializer: D,
    max_bytes: usize,
    label: &'static str,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    if value.as_ref().is_some_and(|value| value.len() > max_bytes) {
        return Err(serde::de::Error::custom(format!(
            "{label} exceeds configured byte limit"
        )));
    }
    Ok(value)
}

fn bounded_optional(value: Option<&str>, max_bytes: usize) -> bool {
    value.is_none_or(|value| value.len() <= max_bytes)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaMetadataValidationError {
    Invalid,
}

impl fmt::Display for MediaMetadataValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("media metadata is invalid or exceeds a configured bound")
    }
}

impl std::error::Error for MediaMetadataValidationError {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "metadata",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MediaStreamMetadata {
    Video(VideoStreamMetadata),
    Audio(AudioStreamMetadata),
    Other(OtherStreamMetadata),
}

impl MediaStreamMetadata {
    pub const fn index(&self) -> u32 {
        match self {
            Self::Video(stream) => stream.index,
            Self::Audio(stream) => stream.index,
            Self::Other(stream) => stream.index,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoStreamMetadata {
    index: u32,
    #[serde(deserialize_with = "deserialize_codec_name")]
    codec_name: Option<String>,
    width: NonZeroU32,
    height: NonZeroU32,
    #[serde(deserialize_with = "deserialize_pixel_format")]
    pixel_format: Option<String>,
    average_frame_rate: Option<RationalRate>,
    #[serde(default, deserialize_with = "deserialize_nonnegative_duration")]
    duration: Option<RationalTime>,
}

impl VideoStreamMetadata {
    pub(crate) fn from_probe(
        index: u32,
        codec_name: Option<String>,
        width: NonZeroU32,
        height: NonZeroU32,
        pixel_format: Option<String>,
        average_frame_rate: Option<RationalRate>,
        duration: Option<RationalTime>,
    ) -> Self {
        Self {
            index,
            codec_name,
            width,
            height,
            pixel_format,
            average_frame_rate,
            duration,
        }
    }

    pub const fn index(&self) -> u32 {
        self.index
    }

    pub fn codec_name(&self) -> Option<&str> {
        self.codec_name.as_deref()
    }

    pub const fn width(&self) -> u32 {
        self.width.get()
    }

    pub const fn height(&self) -> u32 {
        self.height.get()
    }

    pub fn pixel_format(&self) -> Option<&str> {
        self.pixel_format.as_deref()
    }

    pub const fn average_frame_rate(&self) -> Option<RationalRate> {
        self.average_frame_rate
    }

    pub const fn duration(&self) -> Option<RationalTime> {
        self.duration
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioStreamMetadata {
    index: u32,
    #[serde(deserialize_with = "deserialize_codec_name")]
    codec_name: Option<String>,
    sample_rate: Option<NonZeroU32>,
    channels: Option<NonZeroU32>,
    #[serde(deserialize_with = "deserialize_channel_layout")]
    channel_layout: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nonnegative_duration")]
    duration: Option<RationalTime>,
}

impl AudioStreamMetadata {
    pub(crate) fn from_probe(
        index: u32,
        codec_name: Option<String>,
        sample_rate: Option<NonZeroU32>,
        channels: Option<NonZeroU32>,
        channel_layout: Option<String>,
        duration: Option<RationalTime>,
    ) -> Self {
        Self {
            index,
            codec_name,
            sample_rate,
            channels,
            channel_layout,
            duration,
        }
    }

    pub const fn index(&self) -> u32 {
        self.index
    }

    pub fn codec_name(&self) -> Option<&str> {
        self.codec_name.as_deref()
    }

    pub fn sample_rate(&self) -> Option<u32> {
        self.sample_rate.map(NonZeroU32::get)
    }

    pub fn channels(&self) -> Option<u32> {
        self.channels.map(NonZeroU32::get)
    }

    pub fn channel_layout(&self) -> Option<&str> {
        self.channel_layout.as_deref()
    }

    pub const fn duration(&self) -> Option<RationalTime> {
        self.duration
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OtherStreamMetadata {
    index: u32,
    #[serde(deserialize_with = "deserialize_codec_type")]
    codec_type: Option<String>,
    #[serde(deserialize_with = "deserialize_codec_name")]
    codec_name: Option<String>,
}

impl OtherStreamMetadata {
    pub(crate) fn from_probe(
        index: u32,
        codec_type: Option<String>,
        codec_name: Option<String>,
    ) -> Self {
        Self {
            index,
            codec_type,
            codec_name,
        }
    }

    pub const fn index(&self) -> u32 {
        self.index
    }

    pub fn codec_type(&self) -> Option<&str> {
        self.codec_type.as_deref()
    }

    pub fn codec_name(&self) -> Option<&str> {
        self.codec_name.as_deref()
    }
}

fn deserialize_nonnegative_duration<'de, D>(
    deserializer: D,
) -> Result<Option<RationalTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let duration = Option::<RationalTime>::deserialize(deserializer)?;
    if duration.is_some_and(|value| value.numerator() < 0) {
        return Err(serde::de::Error::custom("duration must be nonnegative"));
    }
    Ok(duration)
}

/// Parses nonnegative decimal seconds without a floating-point conversion.
pub(crate) fn parse_decimal_duration(value: &str) -> Result<RationalTime, DecimalDurationError> {
    let (whole, fractional) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || (!fractional.is_empty() && !fractional.bytes().all(|byte| byte.is_ascii_digit()))
        || value.starts_with('+')
        || value.starts_with('-')
        || (value.contains('.') && fractional.is_empty())
    {
        return Err(DecimalDurationError::Invalid);
    }
    if fractional.len() > MAX_DECIMAL_FRACTION_DIGITS {
        return Err(DecimalDurationError::TooPrecise);
    }

    let scale = 10_u32
        .checked_pow(fractional.len() as u32)
        .ok_or(DecimalDurationError::Overflow)?;
    let whole = whole
        .parse::<i128>()
        .map_err(|_| DecimalDurationError::Overflow)?;
    let fraction = if fractional.is_empty() {
        0
    } else {
        fractional
            .parse::<i128>()
            .map_err(|_| DecimalDurationError::Overflow)?
    };
    let numerator = whole
        .checked_mul(i128::from(scale))
        .and_then(|whole| whole.checked_add(fraction))
        .ok_or(DecimalDurationError::Overflow)?;
    let numerator = i64::try_from(numerator).map_err(|_| DecimalDurationError::Overflow)?;
    RationalTime::new(numerator, scale).map_err(|_| DecimalDurationError::Overflow)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DecimalDurationError {
    Invalid,
    TooPrecise,
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::{
        AudioStreamMetadata, DecimalDurationError, MAX_MEDIA_CHANNEL_LAYOUT_BYTES,
        MAX_MEDIA_CODEC_NAME_BYTES, MAX_MEDIA_CODEC_TYPE_BYTES, MAX_MEDIA_FORMAT_NAME_BYTES,
        MAX_MEDIA_FORMAT_NAMES, MAX_MEDIA_PIXEL_FORMAT_BYTES, MAX_MEDIA_SOURCE_URI_BYTES,
        MAX_MEDIA_STREAMS, MediaId, MediaMetadata, MediaSourceRef, MediaSourceUri,
        MediaSourceUriError, MediaStreamMetadata, OtherStreamMetadata, VideoStreamMetadata,
        parse_decimal_duration,
    };
    use crate::{RationalRate, RationalTime};
    use std::{num::NonZeroU32, str::FromStr};
    use uuid::Uuid;

    #[test]
    fn media_id_generates_formats_parses_and_round_trips() {
        let id = MediaId::generate();
        let text = id.to_string();
        assert_eq!(
            Uuid::parse_str(&text).unwrap().get_version(),
            Some(uuid::Version::Random)
        );
        assert_eq!(MediaId::from_str(&text).unwrap(), id);
        assert_eq!(
            serde_json::from_str::<MediaId>(&serde_json::to_string(&id).unwrap()).unwrap(),
            id
        );
    }

    #[test]
    fn media_id_rejects_malformed_and_non_v4_values() {
        assert!(MediaId::from_str("not-a-uuid").is_err());
        assert!(
            serde_json::from_str::<MediaId>("\"00000000-0000-0000-0000-000000000001\"").is_err()
        );
    }

    #[test]
    fn source_uri_accepts_local_file_uris_and_rejects_other_or_unbounded_sources() {
        let source = MediaSourceRef::local_file("file:///tmp/a%20clip.mkv").unwrap();
        assert_eq!(source.uri(), "file:///tmp/a%20clip.mkv");
        for uri in [
            "https://example.com/video.mkv",
            "file://remote.example/video.mkv",
            "file:///tmp/video.mkv?download=1",
            "file:///tmp/video.mkv#fragment",
            "file:///tmp/%ZZ.mkv",
        ] {
            assert_eq!(
                MediaSourceUri::parse(uri),
                Err(MediaSourceUriError::Invalid),
                "{uri}"
            );
        }
        let long = format!("file:///{}", "a".repeat(MAX_MEDIA_SOURCE_URI_BYTES));
        assert_eq!(
            MediaSourceUri::parse(long),
            Err(MediaSourceUriError::TooLong)
        );
    }

    #[test]
    fn decimal_seconds_parse_exactly_without_float_round_trips() {
        for (input, expected) in [
            ("0", (0, 1)),
            ("1", (1, 1)),
            ("1.5", (3, 2)),
            ("0.001", (1, 1000)),
            ("10.125", (81, 8)),
        ] {
            assert_eq!(
                parse_decimal_duration(input).unwrap(),
                RationalTime::new(expected.0, expected.1).unwrap(),
                "{input}"
            );
        }
    }

    #[test]
    fn decimal_seconds_reject_invalid_signs_precision_and_overflow() {
        for input in ["", "NaN", "inf", "1e3", "1.", ".5", "-0.5", "+1"] {
            assert_eq!(
                parse_decimal_duration(input),
                Err(DecimalDurationError::Invalid)
            );
        }
        assert_eq!(
            parse_decimal_duration("0.1234567890"),
            Err(DecimalDurationError::TooPrecise)
        );
        assert_eq!(
            parse_decimal_duration("9223372036854775808"),
            Err(DecimalDurationError::Overflow)
        );
    }

    #[test]
    fn media_metadata_serde_round_trips_the_or_contract() {
        let metadata = MediaMetadata::from_probe(
            vec!["matroska".to_owned(), "webm".to_owned()],
            Some(RationalTime::new(3, 2).unwrap()),
            1024,
            vec![
                MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                    0,
                    Some("ffv1".to_owned()),
                    NonZeroU32::new(16).unwrap(),
                    NonZeroU32::new(16).unwrap(),
                    Some("yuv420p".to_owned()),
                    Some(RationalRate::new(24, 1).unwrap()),
                    Some(RationalTime::new(3, 2).unwrap()),
                )),
                MediaStreamMetadata::Audio(AudioStreamMetadata::from_probe(
                    1,
                    Some("pcm_s16le".to_owned()),
                    NonZeroU32::new(48_000),
                    NonZeroU32::new(2),
                    Some("stereo".to_owned()),
                    Some(RationalTime::new(3, 2).unwrap()),
                )),
                MediaStreamMetadata::Other(OtherStreamMetadata::from_probe(
                    2,
                    Some("subtitle".to_owned()),
                    Some("subrip".to_owned()),
                )),
            ],
        );

        let encoded = serde_json::to_vec(&metadata).unwrap();
        let decoded: MediaMetadata = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, metadata);
        assert!(!String::from_utf8(encoded).unwrap().contains("/Users/"));
    }

    #[test]
    fn media_metadata_serde_rejects_zero_dimensions_and_negative_duration() {
        let zero_dimensions = r#"{"format_names":[],"duration":null,"file_size_bytes":0,"streams":[{"kind":"video","metadata":{"index":0,"codec_name":null,"width":0,"height":1,"pixel_format":null,"average_frame_rate":null,"duration":null}}]}"#;
        assert!(serde_json::from_str::<MediaMetadata>(zero_dimensions).is_err());

        let negative_duration = r#"{"format_names":[],"duration":{"numerator":-1,"denominator":1},"file_size_bytes":0,"streams":[]}"#;
        assert!(serde_json::from_str::<MediaMetadata>(negative_duration).is_err());
    }

    #[test]
    fn media_metadata_serde_enforces_all_persisted_string_and_collection_bounds() {
        let metadata_with =
            |format_names, streams| MediaMetadata::from_probe(format_names, None, 0, streams);
        assert!(
            metadata_with(vec!["x".to_owned(); MAX_MEDIA_FORMAT_NAMES + 1], Vec::new(),)
                .validate()
                .is_err()
        );
        assert!(
            metadata_with(
                vec!["x".repeat(MAX_MEDIA_FORMAT_NAME_BYTES + 1)],
                Vec::new(),
            )
            .validate()
            .is_err()
        );
        assert!(
            metadata_with(
                Vec::new(),
                vec![MediaStreamMetadata::Other(OtherStreamMetadata::from_probe(
                    0,
                    None,
                    Some("x".repeat(MAX_MEDIA_CODEC_NAME_BYTES + 1)),
                ))],
            )
            .validate()
            .is_err()
        );
        assert!(
            metadata_with(
                Vec::new(),
                vec![MediaStreamMetadata::Other(OtherStreamMetadata::from_probe(
                    0,
                    Some("x".repeat(MAX_MEDIA_CODEC_TYPE_BYTES + 1)),
                    None,
                ))],
            )
            .validate()
            .is_err()
        );
        assert!(
            metadata_with(
                Vec::new(),
                vec![MediaStreamMetadata::Video(VideoStreamMetadata::from_probe(
                    0,
                    None,
                    NonZeroU32::new(1).unwrap(),
                    NonZeroU32::new(1).unwrap(),
                    Some("x".repeat(MAX_MEDIA_PIXEL_FORMAT_BYTES + 1)),
                    None,
                    None,
                ))],
            )
            .validate()
            .is_err()
        );
        assert!(
            metadata_with(
                Vec::new(),
                vec![MediaStreamMetadata::Audio(AudioStreamMetadata::from_probe(
                    0,
                    None,
                    None,
                    None,
                    Some("x".repeat(MAX_MEDIA_CHANNEL_LAYOUT_BYTES + 1)),
                    None,
                ))],
            )
            .validate()
            .is_err()
        );

        let stream = MediaStreamMetadata::Other(OtherStreamMetadata::from_probe(0, None, None));
        assert!(
            metadata_with(Vec::new(), vec![stream; MAX_MEDIA_STREAMS + 1])
                .validate()
                .is_err()
        );
        let invalid_wire = format!(
            r#"{{"format_names":["{}"],"duration":null,"file_size_bytes":0,"streams":[]}}"#,
            "x".repeat(MAX_MEDIA_FORMAT_NAME_BYTES + 1)
        );
        assert!(serde_json::from_str::<MediaMetadata>(&invalid_wire).is_err());
    }
}
