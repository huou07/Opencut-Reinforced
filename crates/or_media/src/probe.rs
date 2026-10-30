//! In-process media probing through the linked FFmpeg runtime.
//!
//! The packaged `ffprobe` helper is the normal import backend because it is
//! bounded by a separate process. When that helper cannot start or exits
//! without reading a file, the application already links the same libavformat
//! for playback and export, so it can inspect the file directly and produce the
//! same bounded JSON the helper would have produced.

use ffmpeg_the_third::{Error, format};
use std::path::Path;

const MAX_STREAMS: usize = 64;
const EINVAL: i32 = 22;

/// Builds the probe JSON for an already opened format context.
pub(crate) fn from_input(input: &mut format::context::Input) -> Result<Vec<u8>, Error> {
    if input.nb_streams() as usize > MAX_STREAMS {
        return Err(Error::Other { errno: EINVAL });
    }
    let streams = input
        .streams()
        .map(|stream| {
            let parameters = stream.parameters();
            let codec_name = parameters.id().name();
            let duration = decimal_time(stream.duration(), stream.time_base());
            let mut value = serde_json::json!({
                "index": stream.index(),
                "codec_name": (codec_name != "none").then_some(codec_name),
                "duration": duration,
                "disposition": {
                    "attached_pic": stream.disposition().contains(
                        format::stream::Disposition::ATTACHED_PIC
                    ),
                },
            });
            let fields = value.as_object_mut().expect("JSON object");
            match parameters.medium() {
                ffmpeg_the_third::media::Type::Video => {
                    fields.insert("codec_type".to_owned(), "video".into());
                    fields.insert("width".to_owned(), parameters.width().into());
                    fields.insert("height".to_owned(), parameters.height().into());
                    let rate = stream.avg_frame_rate();
                    fields.insert(
                        "avg_frame_rate".to_owned(),
                        if rate.numerator() > 0 && rate.denominator() > 0 {
                            format!("{}/{}", rate.numerator(), rate.denominator()).into()
                        } else {
                            "0/0".into()
                        },
                    );
                }
                ffmpeg_the_third::media::Type::Audio => {
                    fields.insert("codec_type".to_owned(), "audio".into());
                    fields.insert("sample_rate".to_owned(), parameters.sample_rate().into());
                    fields.insert(
                        "channels".to_owned(),
                        parameters.ch_layout().channels().into(),
                    );
                }
                _ => {
                    fields.insert("codec_type".to_owned(), "other".into());
                }
            }
            value
        })
        .collect::<Vec<_>>();
    let format_names = input.format().name();
    let duration = input.duration();
    let duration =
        (duration >= 0).then(|| format!("{}.{:06}", duration / 1_000_000, duration % 1_000_000));
    serde_json::to_vec(&serde_json::json!({
        "format": { "format_name": format_names, "duration": duration },
        "streams": streams,
    }))
    .map_err(|_| Error::Other { errno: EINVAL })
}

fn decimal_time(value: i64, time_base: ffmpeg_the_third::Rational) -> Option<String> {
    if value < 0 || time_base.numerator() <= 0 || time_base.denominator() <= 0 {
        return None;
    }
    let denominator = i128::from(time_base.denominator());
    let micros = i128::from(value)
        .checked_mul(i128::from(time_base.numerator()))?
        .checked_mul(1_000_000)?
        .checked_add(denominator / 2)?
        .checked_div(denominator)?;
    let micros = u64::try_from(micros).ok()?;
    Some(format!("{}.{:06}", micros / 1_000_000, micros % 1_000_000))
}

/// Probes one local path in-process.
///
/// This is the fallback for a missing or unusable packaged helper, not the
/// primary path: it is bounded by the stream limit and the normal
/// `MAX_MEDIA_STREAMS` validation that runs when the caller parses the output.
pub fn probe_media_path(path: &Path) -> Result<Vec<u8>, Error> {
    ffmpeg_the_third::init()?;
    let mut input = format::input(path)?;
    from_input(&mut input)
}

#[cfg(test)]
mod tests {
    use super::probe_media_path;

    #[test]
    fn in_process_probe_reads_iso_bmff_video_and_audio() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tiny_h264_aac.mp4");
        let bytes = probe_media_path(&path).expect("probe the MP4 fixture in-process");
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let format = value["format"]["format_name"].as_str().unwrap();
        assert!(format.contains("mov") || format.contains("mp4"), "{format}");
        let streams = value["streams"].as_array().unwrap();
        assert!(
            streams.iter().any(|stream| {
                stream["codec_type"] == "video" && stream["codec_name"] == "h264"
            })
        );
        assert!(
            streams
                .iter()
                .any(|stream| { stream["codec_type"] == "audio" && stream["codec_name"] == "aac" })
        );
    }
}
