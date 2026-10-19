//! Materialising a declarative MotionScene into a video file.
//!
//! The CLI renders a still frame only, because the release CLI archives ship `or`
//! without the FFmpeg shared libraries; the application layer links and ships them, so
//! materialisation belongs here. The frame sequence comes from
//! `or_render::for_each_motion_frame`, which yields frames in order over the half-open
//! `[0, duration)` at exact rational times, and the encoder is the editor's own
//! software export writer — Matroska with FFV1 video and PCM S16LE audio — rather than a
//! second encoding path.

use std::path::{Path, PathBuf};

use or_motion::{parse_motion_scene, validate_scene};

/// A bounded request to materialise one scene.
const MAX_MOTION_OUTPUT_FRAMES: u64 = 200_000;

/// What a materialisation produced, or why it produced nothing.
#[derive(Debug, Clone)]
pub struct MotionMaterializationView {
    pub succeeded: bool,
    pub code: String,
    pub message: String,
    pub output_path: String,
    pub scene_id: String,
    pub scene_name: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate_numerator: u32,
    pub frame_rate_denominator: u32,
    pub frame_count: u64,
    pub duration_numerator: i64,
    pub duration_denominator: i64,
    pub bytes_written: u64,
}

fn failure(code: &str, message: impl Into<String>, output: &Path) -> MotionMaterializationView {
    MotionMaterializationView {
        succeeded: false,
        code: code.to_owned(),
        message: message.into(),
        output_path: output.display().to_string(),
        scene_id: String::new(),
        scene_name: String::new(),
        width: 0,
        height: 0,
        frame_rate_numerator: 0,
        frame_rate_denominator: 0,
        frame_count: 0,
        duration_numerator: 0,
        duration_denominator: 0,
        bytes_written: 0,
    }
}

/// Materialises the scene at `scene_path` into `output_path` as a video.
///
/// `frame_rate` and the canvas size are explicit so a caller cannot inherit an
/// accidental default, and the frame count follows from the scene's duration and the
/// rate rather than from the caller. A scene whose duration is zero, whose rate is
/// invalid, or whose canvas exceeds the rasteriser's bound is refused before any frame
/// is encoded.
#[allow(clippy::too_many_arguments)]
pub fn materialize_motion_scene(
    scene_path: String,
    output_path: String,
    frame_rate_numerator: u32,
    frame_rate_denominator: u32,
    width: u32,
    height: u32,
) -> MotionMaterializationView {
    let output = PathBuf::from(&output_path);
    let source = match std::fs::read_to_string(&scene_path) {
        Ok(source) => source,
        Err(error) => {
            return failure(
                "MOTION_SCENE_UNREADABLE",
                format!("The scene could not be read: {error}"),
                &output,
            );
        }
    };
    let scene = match parse_motion_scene(source.as_bytes()) {
        Ok(scene) => scene,
        Err(error) => return failure(error.code(), error.code().to_owned(), &output),
    };
    materialize_parsed_scene(
        &scene,
        &output,
        frame_rate_numerator,
        frame_rate_denominator,
        width,
        height,
    )
}

/// Builds a scene from a code artifact and materialises it in one call.
///
/// This is the path the application surface uses: a user chooses a source file, not a
/// scene, and the CLI offers the same construction as `or motion from-code`. The scene
/// construction is `or_motion::scene_from_source`, so the app and the CLI cannot drift.
#[allow(clippy::too_many_arguments)]
pub fn materialize_motion_scene_from_code(
    source_path: String,
    output_path: String,
    frame_rate_numerator: u32,
    frame_rate_denominator: u32,
    width: u32,
    height: u32,
    title: String,
) -> MotionMaterializationView {
    let output = PathBuf::from(&output_path);
    let source = match std::fs::read(&source_path) {
        Ok(source) => source,
        Err(error) => {
            return failure(
                "MOTION_SOURCE_UNREADABLE",
                format!("The code source could not be read: {error}"),
                &output,
            );
        }
    };
    let options = or_motion::CodeSceneOptions {
        title: if title.trim().is_empty() {
            "Illustration".to_owned()
        } else {
            title
        },
        canvas: or_motion::MotionCanvas { width, height },
        ..or_motion::CodeSceneOptions::default()
    };
    // The language follows the file, as it does in the CLI, so a Python artifact is
    // tokenised as Python rather than as plain text.
    let language = or_motion::CodeLanguage::from_path(&source_path);
    let scene = match or_motion::scene_from_source(&source, language, &options) {
        Ok(scene) => scene,
        // `CodeSceneError` carries its code rather than a Display message.
        Err(error) => return failure(error.code(), error.code().to_owned(), &output),
    };
    materialize_parsed_scene(
        &scene,
        &output,
        frame_rate_numerator,
        frame_rate_denominator,
        width,
        height,
    )
}

/// The shared materialiser: a validated scene, an output path and an explicit shape.
fn materialize_parsed_scene(
    scene: &or_motion::MotionScene,
    output: &Path,
    frame_rate_numerator: u32,
    frame_rate_denominator: u32,
    width: u32,
    height: u32,
) -> MotionMaterializationView {
    if frame_rate_numerator == 0 || frame_rate_denominator == 0 {
        return failure(
            "MOTION_INVALID_FRAME_RATE",
            "A materialised scene needs a positive frame rate.",
            output,
        );
    }
    let validation = validate_scene(scene);
    if !validation.ok {
        let first = validation
            .errors
            .first()
            .map(|diagnostic| diagnostic.code.as_str())
            .unwrap_or("MOTION_INVALID_SCENE");
        return failure(
            first,
            validation
                .errors
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
            output,
        );
    }
    let duration = scene.duration;
    let seconds = duration.numerator() as f64 / duration.denominator() as f64;
    let frame_count = (seconds * f64::from(frame_rate_numerator)
        / f64::from(frame_rate_denominator))
    .ceil() as u64;
    if frame_count == 0 {
        return failure(
            "MOTION_EMPTY_SEQUENCE",
            "A scene with no frames at this rate has nothing to materialise.",
            output,
        );
    }
    if frame_count > MAX_MOTION_OUTPUT_FRAMES {
        return failure(
            "MOTION_SEQUENCE_TOO_LONG",
            format!("A materialisation is bounded at {MAX_MOTION_OUTPUT_FRAMES} frames."),
            output,
        );
    }
    let mut writer = match or_media::FfmpegSoftwareExportWriter::create_with_profile(
        output,
        or_media::SoftwareExportProfile::MatroskaFfv1PcmS16le,
        width,
        height,
        frame_rate_numerator,
        frame_rate_denominator,
    ) {
        Ok(writer) => writer,
        Err(error) => {
            return failure(
                "MOTION_ENCODER_UNAVAILABLE",
                format!("The software encoder is unavailable: {error}"),
                output,
            );
        }
    };
    // `for_each_motion_frame` yields frames in order over the half-open
    // `[0, duration)` at exact rational times, and the visitor writes each one straight
    // to the encoder, so a long scene never has to fit in memory. The visitor's error
    // type describes rasterisation only, so an encoder failure is carried out in a
    // local and reported as itself rather than being disguised as a canvas problem.
    let mut encoder_failure: Option<String> = None;
    let encoded = or_render::for_each_motion_frame(
        scene,
        frame_rate_numerator,
        frame_rate_denominator,
        |index, frame| {
            if encoder_failure.is_some() {
                return Err(or_render::MotionRasterError::InvalidCanvas);
            }
            match writer.write_video_frame(frame.pixels(), frame.width(), frame.height(), index) {
                Ok(()) => Ok(()),
                Err(error) => {
                    encoder_failure = Some(error.to_string());
                    Err(or_render::MotionRasterError::InvalidCanvas)
                }
            }
        },
    );
    if let Some(message) = encoder_failure {
        return failure("MOTION_ENCODER_FAILED", message, output);
    }
    let encoded = match encoded {
        Ok(encoded) => encoded,
        Err(error) => return failure(error.code(), error.code().to_owned(), output),
    };
    if let Err(error) = writer.finish() {
        return failure(
            "MOTION_ENCODER_FAILED",
            format!("The encoder could not finish the file: {error}"),
            output,
        );
    }
    let bytes_written = std::fs::metadata(output)
        .map(|meta| meta.len())
        .unwrap_or(0);
    MotionMaterializationView {
        succeeded: true,
        code: "OK".to_owned(),
        message: format!("Materialised {encoded} frames."),
        output_path: output.display().to_string(),
        scene_id: scene.scene_id.clone(),
        scene_name: scene.name.clone(),
        width,
        height,
        frame_rate_numerator,
        frame_rate_denominator,
        frame_count: encoded,
        duration_numerator: duration.numerator(),
        duration_denominator: i64::from(duration.denominator()),
        bytes_written,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A valid scene, produced by the real code-to-scene path rather than hand-written
    /// JSON: the test then covers "source to scene to video" instead of a fixture that
    /// can drift from the schema.
    fn scene_source() -> String {
        let scene = or_motion::scene_from_source(
            b"fn main() {\n    let total = 1 + 1;\n}\n",
            or_motion::CodeLanguage::Rust,
            &or_motion::CodeSceneOptions {
                title: "Materialisation test".to_owned(),
                canvas: or_motion::MotionCanvas {
                    width: 64,
                    height: 64,
                },
                font_size: 12.0,
                theme: or_motion::CodeTheme::OR_DARK,
                line_milliseconds: 500,
                hold_milliseconds: 0,
            },
        )
        .expect("the fixture source builds a scene");
        serde_json::to_string(&scene).expect("a scene serialises")
    }

    fn scene_file(name: &str, source: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "or-motion-bridge-{}-{name}.json",
            std::process::id()
        ));
        std::fs::write(&path, source).unwrap();
        path
    }

    #[test]
    fn refuses_code_it_cannot_read_or_build() {
        let missing = std::env::temp_dir().join("or-motion-bridge-no-source.rs");
        let output = std::env::temp_dir().join("or-motion-bridge-no-source.mkv");
        let view = materialize_motion_scene_from_code(
            missing.display().to_string(),
            output.display().to_string(),
            10,
            1,
            64,
            64,
            String::new(),
        );
        assert_eq!(view.code, "MOTION_SOURCE_UNREADABLE");
        assert!(!view.succeeded);
        assert!(!output.exists());
    }

    #[test]
    #[ignore = "encodes with the codec-enabled FFmpeg profile; run with the lane's helper"]
    fn materialises_a_scene_built_from_code() {
        let source = scene_file("source.rs", "fn main() {\n    let total = 1 + 1;\n}\n");
        let output =
            std::env::temp_dir().join(format!("or-motion-from-code-{}.mkv", std::process::id()));
        let _ = std::fs::remove_file(&output);
        let view = materialize_motion_scene_from_code(
            source.display().to_string(),
            output.display().to_string(),
            10,
            1,
            128,
            96,
            "From code".to_owned(),
        );
        assert!(view.succeeded, "{}: {}", view.code, view.message);
        assert!(view.frame_count > 0);
        assert_eq!(view.width, 128);
        assert_eq!(view.height, 96);
        assert!(!view.scene_id.is_empty());
        assert!(view.bytes_written > 0);
        let _ = std::fs::remove_file(&output);
    }

    #[test]
    fn refuses_a_frame_rate_it_cannot_materialise() {
        let scene = scene_file("rate", &scene_source());
        let output = std::env::temp_dir().join("or-motion-bridge-rate.mkv");
        let view = materialize_motion_scene(
            scene.display().to_string(),
            output.display().to_string(),
            30,
            0,
            64,
            64,
        );
        assert!(!view.succeeded);
        assert_eq!(view.code, "MOTION_INVALID_FRAME_RATE");
        assert_eq!(view.frame_count, 0);
        assert!(!output.exists());
    }

    #[test]
    fn refuses_a_scene_it_cannot_read_or_validate() {
        let missing = std::env::temp_dir().join("or-motion-bridge-missing.json");
        let output = std::env::temp_dir().join("or-motion-bridge-missing.mkv");
        let view = materialize_motion_scene(
            missing.display().to_string(),
            output.display().to_string(),
            30,
            1,
            64,
            64,
        );
        assert_eq!(view.code, "MOTION_SCENE_UNREADABLE");

        let invalid = scene_file("invalid", "{\"format\":\"not-a-scene\"}");
        let view = materialize_motion_scene(
            invalid.display().to_string(),
            output.display().to_string(),
            30,
            1,
            64,
            64,
        );
        assert!(!view.succeeded);
        assert_ne!(view.code, "OK");
        assert!(!output.exists());
    }

    /// The materialisation-throughput row docs/QUALITY.md owed: a named canvas, a named
    /// rate and a scene built from real source, measured through the client the product
    /// actually calls.
    #[test]
    #[ignore = "measures throughput; run with the lane's helper and --release"]
    fn measures_materialisation_throughput_at_a_named_canvas() {
        let source = (0..24)
            .map(|index| format!("    let value_{index} = {index} * 2; // line {index}\n"))
            .collect::<String>();
        let scene = or_motion::scene_from_source(
            format!("fn main() {{\n{source}}}\n").as_bytes(),
            or_motion::CodeLanguage::Rust,
            &or_motion::CodeSceneOptions {
                title: "Throughput".to_owned(),
                canvas: or_motion::MotionCanvas {
                    width: 640,
                    height: 360,
                },
                font_size: 18.0,
                theme: or_motion::CodeTheme::OR_DARK,
                line_milliseconds: 300,
                hold_milliseconds: 1000,
            },
        )
        .expect("the throughput source builds a scene");
        let path = scene_file("throughput", &serde_json::to_string(&scene).unwrap());
        let output =
            std::env::temp_dir().join(format!("or-motion-throughput-{}.mkv", std::process::id()));
        let _ = std::fs::remove_file(&output);
        let started = std::time::Instant::now();
        let view = materialize_motion_scene(
            path.display().to_string(),
            output.display().to_string(),
            10,
            1,
            640,
            360,
        );
        let elapsed = started.elapsed();
        assert!(view.succeeded, "{}: {}", view.code, view.message);
        println!(
            "OR_MOTION_MATERIALISATION frames={} seconds={:.3} frames_per_second={:.1} bytes={} canvas={}x{} rate={}/{}",
            view.frame_count,
            elapsed.as_secs_f64(),
            view.frame_count as f64 / elapsed.as_secs_f64().max(f64::EPSILON),
            view.bytes_written,
            view.width,
            view.height,
            view.frame_rate_numerator,
            view.frame_rate_denominator,
        );
        let _ = std::fs::remove_file(&output);
    }

    #[test]
    #[ignore = "encodes with the codec-enabled FFmpeg profile; run with the lane's helper"]
    fn materialises_a_scene_into_a_video_through_the_software_encoder() {
        let scene = scene_file("encode", &scene_source());
        let output =
            std::env::temp_dir().join(format!("or-motion-bridge-{}.mkv", std::process::id()));
        let _ = std::fs::remove_file(&output);
        let view = materialize_motion_scene(
            scene.display().to_string(),
            output.display().to_string(),
            10,
            1,
            64,
            64,
        );
        assert!(view.succeeded, "{}: {}", view.code, view.message);
        // Three source lines at half a second each: the frame count follows from the
        // scene's own duration and the requested rate, not from the caller.
        let expected = (view.duration_numerator as f64 / view.duration_denominator as f64 * 10.0)
            .ceil() as u64;
        assert_eq!(view.frame_count, expected);
        assert!(view.frame_count > 0);
        assert_eq!(view.width, 64);
        assert_eq!(view.height, 64);
        assert_eq!(view.frame_rate_numerator, 10);
        assert!(view.duration_numerator > 0);
        assert!(view.bytes_written > 0);
        assert_eq!(
            std::fs::metadata(&output).unwrap().len(),
            view.bytes_written
        );
        let _ = std::fs::remove_file(&output);
    }
}
