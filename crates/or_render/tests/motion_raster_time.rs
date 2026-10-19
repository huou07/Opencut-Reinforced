//! Bounded MotionScene rasterisation time on this named host.
//!
//! `#[ignore]`d by default: `scripts/bench_quality.py` runs it, not the ordinary
//! suite. One machine-readable line (`OR_BENCH_JSON=...`) carries the distribution.
//!
//! What it establishes: the cost of the vector-to-pixel step (evaluate a scene, then
//! rasterise every instruction) and of encoding the result as a PNG, at the canvas
//! size a 1080p explainer uses, on this host in a debug build. It is a regression
//! guard for OR's own code, not a claim about a release build's speed, and it is
//! pure CPU work: no GPU adapter is involved, so nothing here says anything about
//! the wgpu path the render-frame-time row covers.
use or_core::RationalTime;
use or_motion::{MotionScene, evaluate_scene, parse_motion_scene};
use or_render::{encode_png, rasterize_motion_frame};
use std::time::Instant;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const NODES: usize = 40;
const FRAMES: usize = 30;

fn scene() -> MotionScene {
    let mut nodes = Vec::with_capacity(NODES);
    for index in 0..NODES {
        let column = index % 8;
        let row = index / 8;
        nodes.push(serde_json::json!({
            "node_id": format!("node-{index}"),
            "kind": if index % 5 == 0 { "ellipse" } else { "rectangle" },
            "position": {"x": 40 + column * 220, "y": 60 + row * 200},
            "size": {"width": 180, "height": 120},
            "fill": {"red": 30 + index * 5, "green": 60, "blue": 90},
            "stroke": {"red": 200, "green": 200, "blue": 200},
            "stroke_width": 2,
            "opacity": 0.8,
        }));
    }
    let value = serde_json::json!({
        "format": "opencut-reinforced-motion-scene",
        "schema_version": 1,
        "scene_id": "6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f",
        "name": "Raster benchmark",
        "duration": {"numerator": 30, "denominator": 1},
        "canvas": {"width": WIDTH, "height": HEIGHT},
        "assets": [],
        "nodes": nodes,
        "animations": [{
            "node_id": "node-0",
            "property": "opacity",
            "keyframes": [
                {"at": {"numerator": 0, "denominator": 1}, "value": 0.0},
                {"at": {"numerator": 30, "denominator": 1}, "value": 1.0},
            ],
            "easing": "ease_in_out",
        }],
    });
    parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap()
}

fn distribution(mut samples: Vec<f64>) -> (f64, f64) {
    samples.sort_by(|left, right| left.total_cmp(right));
    let p50 = samples[samples.len() / 2];
    let index = ((samples.len() as f64 * 0.95).ceil() as usize).clamp(1, samples.len()) - 1;
    (p50, samples[index])
}

#[test]
#[ignore = "measurement lane: run through scripts/bench_quality.py"]
fn motion_raster_time() {
    let scene = scene();
    // Warm up, so the first allocation and the font database load are not in the
    // distribution.
    let warm = evaluate_scene(&scene, RationalTime::new(0, 1).unwrap()).unwrap();
    let _ = rasterize_motion_frame(&warm).unwrap();

    let mut raster_samples = Vec::with_capacity(FRAMES);
    let mut encode_samples = Vec::with_capacity(FRAMES);
    for frame in 0..FRAMES {
        let time = RationalTime::new(frame as i64, 1).unwrap();
        let started = Instant::now();
        let evaluated = evaluate_scene(&scene, time).unwrap();
        let canvas = rasterize_motion_frame(&evaluated).unwrap();
        raster_samples.push(started.elapsed().as_secs_f64() * 1000.0);
        let started = Instant::now();
        let png = encode_png(canvas.width(), canvas.height(), canvas.pixels()).unwrap();
        encode_samples.push(started.elapsed().as_secs_f64() * 1000.0);
        assert!(!png.is_empty());
    }

    let (raster_p50, raster_p95) = distribution(raster_samples);
    let (encode_p50, encode_p95) = distribution(encode_samples);
    println!(
        "OR_BENCH_JSON={{\"result\":\"MEASURED\",\"row\":\"motion-rasterize-frame\",\"nodes\":{NODES},\
         \"width\":{WIDTH},\"height\":{HEIGHT},\"frames\":{FRAMES},\"build\":\"debug\",\
         \"raster_p50_ms\":{raster_p50:.4},\"raster_p95_ms\":{raster_p95:.4},\
         \"encode_p50_ms\":{encode_p50:.4},\"encode_p95_ms\":{encode_p95:.4},\
         \"includes\":\"evaluate, rasterize every instruction, and PNG encode\"}}"
    );
}
