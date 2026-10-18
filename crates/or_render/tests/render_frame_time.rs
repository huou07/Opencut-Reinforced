//! Bounded offscreen render frame time on this named host.
//!
//! `#[ignore]`d by default: this is a measurement lane, run by
//! `scripts/bench_quality.py`, not by the ordinary test suite. It escapes the
//! process as one machine-readable line (`OR_BENCH_JSON=...`) so the budget
//! script reads a number instead of scraping text.
//!
//! What it does and does not establish:
//!
//! * It renders a real offscreen frame through the shared `or_render` wgpu path
//!   on whatever adapter the host exposes, and reports the adapter name, driver,
//!   backend and the frame-time distribution.
//! * A software adapter (llvmpipe, SwiftShader, Lavapipe) is reported as
//!   `software` and never as GPU or device performance. Only a hardware adapter
//!   on this named host class is a local GPU number, and even that is a debug
//!   build on one machine.
//! * It measures frame *submission plus readback*, which is deliberately heavier
//!   than presentation: the readback is included so a silently dropped frame
//!   cannot look fast. The number is therefore an upper bound on the presentation
//!   path, not a claim that the viewer presents at that interval.
use or_core::{ProjectDocument, RationalTime, TimeRange};
use or_render::{
    PreviewSurface, RenderDevice, RenderFormat, RenderGraphInput, RenderSize, SyntheticColor,
    SyntheticScene,
};
use or_runtime::RenderSnapshot;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

/// The viewer's documented preview size, so the number describes the product's
/// own surface rather than a conveniently tiny target.
const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;
/// Warm-up frames keep first-use pipeline compilation out of the distribution.
const WARMUP: usize = 5;
const FRAMES: usize = 60;

#[test]
#[ignore]
fn offscreen_frame_time_is_measurable_on_this_host() {
    let renderer = match block_on(RenderDevice::new()) {
        Ok(renderer) => renderer,
        Err(error) => {
            report(&format!(
                "{{\"result\":\"UNVERIFIED\",\"reason\":{},\"settle\":{}}}",
                json_string(&format!("no usable wgpu adapter: {error}")),
                json_string("install or enable a Vulkan/GL driver, then rerun this lane"),
            ));
            return;
        }
    };

    let info = renderer.adapter_info();
    let name = info.name().to_owned();
    let driver = format!("{} {}", info.driver(), info.driver_info()).trim().to_owned();
    let backend = format!("{:?}", info.backend());
    let software = looks_like_software(&name, &driver);

    let size = match RenderSize::new(WIDTH, HEIGHT) {
        Ok(size) => size,
        Err(error) => {
            report(&format!(
                "{{\"result\":\"UNVERIFIED\",\"reason\":{}}}",
                json_string(&format!("invalid render size: {error}")),
            ));
            return;
        }
    };
    let project = ProjectDocument::new("render frame time");
    let range = match TimeRange::new(time(0, 1), time(1, 1)) {
        Ok(range) => range,
        Err(error) => {
            report(&format!(
                "{{\"result\":\"UNVERIFIED\",\"reason\":{}}}",
                json_string(&format!("invalid time range: {error}")),
            ));
            return;
        }
    };

    let mut samples = Vec::with_capacity(FRAMES);
    for index in 0..(WARMUP + FRAMES) {
        let input = RenderGraphInput::new(
            RenderSnapshot::new(project.id(), project.revision(), range),
            PreviewSurface::offscreen(size, RenderFormat::Rgba8Unorm),
            SyntheticScene::solid(SyntheticColor::GREEN),
        );
        let started = Instant::now();
        let frame = match renderer.render(input) {
            Ok(frame) => frame,
            Err(error) => {
                report(&format!(
                    "{{\"result\":\"UNVERIFIED\",\"reason\":{}}}",
                    json_string(&format!("frame {index} failed to render: {error}")),
                ));
                return;
            }
        };
        if let Err(error) = renderer.readback(&frame) {
            report(&format!(
                "{{\"result\":\"UNVERIFIED\",\"reason\":{}}}",
                json_string(&format!("frame {index} failed readback: {error}")),
            ));
            return;
        }
        let elapsed = started.elapsed();
        if index >= WARMUP {
            samples.push(elapsed);
        }
    }

    samples.sort_unstable();
    let ms = |value: Duration| value.as_secs_f64() * 1000.0;
    let percentile = |fraction: f64| {
        let index = ((samples.len() - 1) as f64 * fraction).round() as usize;
        samples[index]
    };
    report(&format!(
        concat!(
            "{{\"result\":\"{result}\",",
            "\"adapter\":{adapter},\"driver\":{driver},\"backend\":{backend},",
            "\"adapter_class\":\"{class}\",",
            "\"width\":{width},\"height\":{height},\"frames\":{frames},",
            "\"min_ms\":{min:.3},\"p50_ms\":{p50:.3},\"p95_ms\":{p95:.3},\"max_ms\":{max:.3},",
            "\"includes\":{includes},",
            "\"reason\":{reason}}}"
        ),
        result = if software { "UNVERIFIED" } else { "MEASURED" },
        adapter = json_string(&name),
        driver = json_string(&driver),
        backend = json_string(&backend),
        class = if software { "software" } else { "hardware" },
        width = WIDTH,
        height = HEIGHT,
        frames = samples.len(),
        min = ms(samples[0]),
        p50 = ms(percentile(0.50)),
        p95 = ms(percentile(0.95)),
        max = ms(samples[samples.len() - 1]),
        includes = json_string("render submission plus offscreen readback, not presentation"),
        reason = json_string(if software {
            "the only usable adapter is a software rasteriser, so this is software-path evidence and never a GPU claim"
        } else {
            ""
        }),
    ));
}

/// Minimal JSON string escaping, so this lane needs no new dependency to report
/// one line of machine-readable output.
fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control < ' ' => out.push_str(&format!("\\u{:04x}", control as u32)),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn looks_like_software(name: &str, driver: &str) -> bool {
    let haystack = format!("{name} {driver}").to_ascii_lowercase();
    ["llvmpipe", "swiftshader", "lavapipe", "softpipe", "software"]
        .iter()
        .any(|needle| haystack.contains(needle))
}

fn time(numerator: i64, denominator: u32) -> RationalTime {
    RationalTime::new(numerator, denominator).expect("valid rational time")
}

fn report(payload: &str) {
    println!("OR_BENCH_JSON={payload}");
}

struct ThreadWaker(thread::Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    let current = thread::current();
    let waker: Waker = Waker::from(Arc::new(ThreadWaker(current)));
    let mut context = Context::from_waker(&waker);
    loop {
        match Pin::as_mut(&mut future).poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => thread::park_timeout(Duration::from_millis(1)),
        }
    }
}
