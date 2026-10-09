use crate::{MAX_TIMELINE_CAPTION_BYTES, RationalTime};
use std::{error::Error, fmt};
use subtitler::{Format, SubtitleFormat};

pub const MAX_CAPTION_FILE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_IMPORTED_CAPTIONS: usize = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptionFileFormat {
    Srt,
    WebVtt,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedCaption {
    pub start: RationalTime,
    pub duration: RationalTime,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptionImportPlan {
    pub format: CaptionFileFormat,
    pub captions: Vec<ImportedCaption>,
    pub cues_with_formatting_loss: usize,
    pub empty_cues_skipped: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptionImportError {
    FileTooLarge,
    UnsupportedFormat,
    InvalidFile,
    TooManyCues,
    NoUsableCues,
    CueTooLarge,
    InvalidCueTiming,
    TimeOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptionExportError {
    TooManyCaptions,
    InvalidCueTiming,
    CueTooLarge,
    TimeNotMillisecondExact,
    TimeOverflow,
    OutputTooLarge,
}

impl CaptionExportError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::TooManyCaptions => "CAPTION_LIMIT_EXCEEDED",
            Self::InvalidCueTiming => "CAPTION_TIMING_INVALID",
            Self::CueTooLarge => "CAPTION_TEXT_TOO_LARGE",
            Self::TimeNotMillisecondExact => "CAPTION_TIME_NOT_MILLISECOND_EXACT",
            Self::TimeOverflow => "CAPTION_TIME_OVERFLOW",
            Self::OutputTooLarge => "CAPTION_EXPORT_TOO_LARGE",
        }
    }
}

impl fmt::Display for CaptionExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooManyCaptions => "The project exceeds the 10,000 caption export limit.",
            Self::InvalidCueTiming => "A project caption has an invalid time range.",
            Self::CueTooLarge => "A project caption exceeds the subtitle text limit.",
            Self::TimeNotMillisecondExact => {
                "A caption time cannot be represented exactly in subtitle milliseconds."
            }
            Self::TimeOverflow => "A project caption is outside the subtitle time range.",
            Self::OutputTooLarge => "The subtitle export exceeds the 16 MiB output limit.",
        })
    }
}

impl Error for CaptionExportError {}

pub const MAX_CAPTION_EXPORT_BYTES: usize = 16 * 1024 * 1024;

impl CaptionImportError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::FileTooLarge => "CAPTION_FILE_TOO_LARGE",
            Self::UnsupportedFormat => "CAPTION_FORMAT_UNSUPPORTED",
            Self::InvalidFile => "CAPTION_FILE_INVALID",
            Self::TooManyCues => "CAPTION_LIMIT_EXCEEDED",
            Self::NoUsableCues => "CAPTION_FILE_EMPTY",
            Self::CueTooLarge => "CAPTION_TEXT_TOO_LARGE",
            Self::InvalidCueTiming => "CAPTION_TIMING_INVALID",
            Self::TimeOverflow => "CAPTION_TIME_OVERFLOW",
        }
    }
}

impl fmt::Display for CaptionImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::FileTooLarge => "The subtitle file exceeds the 8 MiB import limit.",
            Self::UnsupportedFormat => "Only SubRip (.srt) and WebVTT (.vtt) are supported.",
            Self::InvalidFile => "The subtitle file could not be parsed.",
            Self::TooManyCues => "The subtitle file exceeds the 10,000 cue import limit.",
            Self::NoUsableCues => "The subtitle file contains no usable caption cues.",
            Self::CueTooLarge => "A caption exceeds the project text limit.",
            Self::InvalidCueTiming => "A caption has an invalid or zero-length time range.",
            Self::TimeOverflow => "A caption time is outside the supported project range.",
        })
    }
}

impl Error for CaptionImportError {}

/// Parses common subtitle files into bounded, exact-time caption values.
///
/// Formatting and cue placement are retained only as a warning count because
/// the canonical caption model currently stores plain text and basic styling.
pub fn parse_caption_file(bytes: &[u8]) -> Result<CaptionImportPlan, CaptionImportError> {
    if bytes.len() > MAX_CAPTION_FILE_BYTES {
        return Err(CaptionImportError::FileTooLarge);
    }
    let file = subtitler::parse_bytes(bytes).map_err(|_| CaptionImportError::InvalidFile)?;
    let format = match file.format() {
        Format::Srt => CaptionFileFormat::Srt,
        Format::Vtt => CaptionFileFormat::WebVtt,
    };
    let subtitles = file.subtitles();
    if subtitles.len() > MAX_IMPORTED_CAPTIONS {
        return Err(CaptionImportError::TooManyCues);
    }

    let mut captions = Vec::with_capacity(subtitles.len());
    let mut cues_with_formatting_loss = 0;
    let mut empty_cues_skipped = 0;
    for subtitle in subtitles {
        let text = subtitle.plaintext();
        if text.trim().is_empty() {
            empty_cues_skipped += 1;
            continue;
        }
        if text.len() > MAX_TIMELINE_CAPTION_BYTES {
            return Err(CaptionImportError::CueTooLarge);
        }
        if subtitle.start >= subtitle.end {
            return Err(CaptionImportError::InvalidCueTiming);
        }
        let start_ms =
            i64::try_from(subtitle.start).map_err(|_| CaptionImportError::TimeOverflow)?;
        let duration_ms = i64::try_from(subtitle.end - subtitle.start)
            .map_err(|_| CaptionImportError::TimeOverflow)?;
        let start =
            RationalTime::new(start_ms, 1000).map_err(|_| CaptionImportError::TimeOverflow)?;
        let duration =
            RationalTime::new(duration_ms, 1000).map_err(|_| CaptionImportError::TimeOverflow)?;
        if subtitle.text != text
            || !subtitle.text_parts.is_empty()
            || subtitle
                .settings
                .as_ref()
                .is_some_and(|settings| !settings.is_empty())
            || subtitle.style.is_some()
            || subtitle.style_props.is_some()
            || subtitle.position.is_some()
        {
            cues_with_formatting_loss += 1;
        }
        captions.push(ImportedCaption {
            start,
            duration,
            text,
        });
    }
    if captions.is_empty() {
        return Err(CaptionImportError::NoUsableCues);
    }

    Ok(CaptionImportPlan {
        format,
        captions,
        cues_with_formatting_loss,
        empty_cues_skipped,
    })
}

/// Encodes project captions without rounding exact project times.
pub fn encode_caption_file(
    format: CaptionFileFormat,
    captions: &[ImportedCaption],
) -> Result<Vec<u8>, CaptionExportError> {
    if captions.len() > MAX_IMPORTED_CAPTIONS {
        return Err(CaptionExportError::TooManyCaptions);
    }
    let mut subtitles = Vec::with_capacity(captions.len());
    for caption in captions {
        if caption.start.is_negative() || !caption.duration.is_positive() {
            return Err(CaptionExportError::InvalidCueTiming);
        }
        if caption.text.trim().is_empty() {
            return Err(CaptionExportError::InvalidCueTiming);
        }
        if caption.text.len() > MAX_TIMELINE_CAPTION_BYTES {
            return Err(CaptionExportError::CueTooLarge);
        }
        let end = caption
            .start
            .checked_add(caption.duration)
            .map_err(|_| CaptionExportError::TimeOverflow)?;
        let start_ms = exact_milliseconds(caption.start)?;
        let end_ms = exact_milliseconds(end)?;
        if end_ms <= start_ms {
            return Err(CaptionExportError::InvalidCueTiming);
        }
        subtitles.push(subtitler::Subtitle::new(start_ms, end_ms, &caption.text));
    }
    let text = match format {
        CaptionFileFormat::Srt => subtitler::srt::to_string(&subtitles),
        CaptionFileFormat::WebVtt => subtitler::vtt::to_string(&subtitles, None),
    };
    if text.len() > MAX_CAPTION_EXPORT_BYTES {
        return Err(CaptionExportError::OutputTooLarge);
    }
    Ok(text.into_bytes())
}

fn exact_milliseconds(time: RationalTime) -> Result<u64, CaptionExportError> {
    if time.is_negative() {
        return Err(CaptionExportError::InvalidCueTiming);
    }
    let scaled = i128::from(time.numerator()) * 1000;
    if scaled % i128::from(time.denominator()) != 0 {
        return Err(CaptionExportError::TimeNotMillisecondExact);
    }
    u64::try_from(scaled / i128::from(time.denominator()))
        .map_err(|_| CaptionExportError::TimeOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_srt_as_exact_rational_times_and_reports_inline_style_loss() {
        let input = b"1\n00:00:00,125 --> 00:00:02,375\n<i>Hello</i>\n";
        let plan = parse_caption_file(input).unwrap();
        assert_eq!(plan.format, CaptionFileFormat::Srt);
        assert_eq!(plan.cues_with_formatting_loss, 1);
        assert_eq!(plan.captions.len(), 1);
        assert_eq!(plan.captions[0].start, RationalTime::new(1, 8).unwrap());
        assert_eq!(plan.captions[0].duration, RationalTime::new(9, 4).unwrap());
        assert_eq!(plan.captions[0].text, "Hello");
    }

    #[test]
    fn imports_webvtt_and_reports_placement_settings_loss() {
        let input = b"WEBVTT\n\n00:00:01.250 --> 00:00:02.500 line:90%\nHello\n";
        let plan = parse_caption_file(input).unwrap();
        assert_eq!(plan.format, CaptionFileFormat::WebVtt);
        assert_eq!(plan.cues_with_formatting_loss, 1);
        assert_eq!(plan.captions[0].start, RationalTime::new(5, 4).unwrap());
        assert_eq!(plan.captions[0].duration, RationalTime::new(5, 4).unwrap());
    }

    #[test]
    fn rejects_invalid_timing_and_bounds_file_text_and_cue_count() {
        let zero = b"1\n00:00:01,000 --> 00:00:01,000\nHello\n";
        assert_eq!(
            parse_caption_file(zero),
            Err(CaptionImportError::InvalidCueTiming)
        );
        assert_eq!(
            parse_caption_file(&vec![0; MAX_CAPTION_FILE_BYTES + 1]),
            Err(CaptionImportError::FileTooLarge)
        );
        let too_many = (0..=MAX_IMPORTED_CAPTIONS)
            .map(|index| format!("{index}\n00:00:00,000 --> 00:00:01,000\nX\n\n"))
            .collect::<String>();
        assert_eq!(
            parse_caption_file(too_many.as_bytes()),
            Err(CaptionImportError::TooManyCues)
        );
    }

    #[test]
    fn exports_srt_and_webvtt_without_rounding() {
        let captions = [ImportedCaption {
            start: RationalTime::new(1, 8).unwrap(),
            duration: RationalTime::new(9, 4).unwrap(),
            text: "Hello".to_owned(),
        }];
        let srt = encode_caption_file(CaptionFileFormat::Srt, &captions).unwrap();
        assert_eq!(parse_caption_file(&srt).unwrap().captions, captions,);
        let vtt = encode_caption_file(CaptionFileFormat::WebVtt, &captions).unwrap();
        assert_eq!(parse_caption_file(&vtt).unwrap().captions, captions);
    }

    #[test]
    fn refuses_to_round_project_times_for_subtitle_export() {
        let captions = [ImportedCaption {
            start: RationalTime::new(1, 3).unwrap(),
            duration: RationalTime::new(2, 3).unwrap(),
            text: "Hello".to_owned(),
        }];
        assert_eq!(
            encode_caption_file(CaptionFileFormat::Srt, &captions),
            Err(CaptionExportError::TimeNotMillisecondExact)
        );
    }
}
