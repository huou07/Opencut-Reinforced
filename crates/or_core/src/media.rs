use crate::{RationalRate, RationalTime, UuidV4ParseError};
use serde::{Deserialize, Deserializer, Serialize};
use std::{fmt, num::NonZeroU32, str::FromStr};
use uuid::{Uuid, Version};

const MAX_DECIMAL_FRACTION_DIGITS: usize = 9;

/// Opaque persistent UUIDv4 identity for a future imported media item.
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaMetadata {
    format_names: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nonnegative_duration")]
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "metadata", rename_all = "snake_case")]
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
    codec_name: Option<String>,
    width: NonZeroU32,
    height: NonZeroU32,
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
    codec_name: Option<String>,
    sample_rate: Option<NonZeroU32>,
    channels: Option<NonZeroU32>,
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
    codec_type: Option<String>,
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
        AudioStreamMetadata, DecimalDurationError, MediaId, MediaMetadata, MediaStreamMetadata,
        OtherStreamMetadata, VideoStreamMetadata, parse_decimal_duration,
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
}
