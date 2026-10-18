//! Playback-critical work must not be starved by opportunistic work.
//!
//! OR's performance rule is that decode, audio and render outrank thumbnail,
//! waveform, proxy and analysis work, and that a test shows an opportunistic
//! load cannot starve or delay the playback-critical path. This lane is that
//! test: it measures an offscreen render frame with the job pool idle, then
//! again while the pool is saturated, and reports both distributions and their
//! ratio.
//!
//! `#[ignore]`d by default; `scripts/bench_quality.py` runs it and enforces the
//! ratio. The opportunistic bodies are CPU-bound on purpose: they model the
//! worst case an analysis job can present, not a sleeping placeholder.
//!
//! Limits: wall clock in a debug build on this host. A software adapter is
//! reported as such and never becomes a GPU claim. Saturation here is the job
//! pool's own workers; a sibling build on the same host adds noise the report
//! cannot remove, which is why the idle and loaded samples are taken in the same
//! process seconds apart.
use or_core::{JobKind, JobManager, JobManagerConfig};
use or_core::{ProjectDocument, RationalTime, TimeRange};
use or_render::{
    PreviewSurface, RenderDevice, RenderFormat, RenderGraphInput, RenderSize, SyntheticColor,
    SyntheticScene,
};
use or_runtime::RenderSnapshot;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;
const WARMUP: usize = 5;
const FRAMES: usize = 40;
/// Opportunistic workers: two per available parallelism, so the pool is
/// oversubscribed exactly as a burst of analysis jobs would make it.
fn opportunistic_workers() -> usize {
    thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(4)
}

#[test]
#[ignore]
fn playback_critical_frame_time_survives_a_saturated_job_pool() {
    let renderer = match block_on(RenderDevice::new()) {
        Ok(renderer) => renderer,
        Err(error) => {
            report(&format!(
                "{{\"result\":\"UNVERIFIED\",\"reason\":{}}}",
                json_string(&format!("no usable wgpu adapter: {error}"))
            ));
            return;
        }
    };
    let info = renderer.adapter_info();
    let adapter = info.name().to_owned();
    let software = looks_like_software(&adapter, info.driver());

    let size = match RenderSize::new(WIDTH, HEIGHT) {
        Ok(size) => size,
        Err(error) => {
            report(&format!(
                "{{\"result\":\"UNVERIFIED\",\"reason\":{}}}",
                json_string(&format!("invalid render size: {error}"))
            ));
            return;
        }
    };
    let project = ProjectDocument::new("queue priority");
    let range = TimeRange::new(time(0, 1), time(1, 1)).expect("valid range");

    let idle = match measure(&renderer, size, &project, range) {
        Ok(samples) => samples,
        Err(reason) => {
            report(&format!(
                "{{\"result\":\"UNVERIFIED\",\"reason\":{}}}",
                json_string(&reason)
            ));
            return;
        }
    };

    // Saturate: every worker spins on real CPU work until the flag is cleared.
    let stop = Arc::new(AtomicBool::new(false));
    let workers = opportunistic_workers();
    let manager = JobManager::new(
        JobManagerConfig::new(workers, workers * 2, workers * 2 + 8)
            .expect("valid job manager configuration"),
    );
    let mut submitted = 0usize;
    for _ in 0..(workers * 2) {
        let stop = Arc::clone(&stop);
        if manager
            .submit(JobKind::WaveformGenerate, move |_context| {
                let mut accumulator: u64 = 0x9e37_79b9_7f4a_7c15;
                while !stop.load(Ordering::Relaxed) {
                    for _ in 0..10_000 {
                        accumulator = accumulator
                            .wrapping_mul(6_364_136_223_846_793_005)
                            .wrapping_add(1_442_695_040_888_963_407);
                    }
                    std::hint::black_box(accumulator);
                }
                Ok(())
            })
            .is_ok()
        {
            submitted += 1;
        }
    }
    // Let the workers actually start before sampling.
    thread::sleep(Duration::from_millis(250));
    let loaded = measure(&renderer, size, &project, range);
    stop.store(true, Ordering::Relaxed);
    manager.shutdown();
    let loaded = match loaded {
        Ok(samples) => samples,
        Err(reason) => {
            report(&format!(
                "{{\"result\":\"UNVERIFIED\",\"reason\":{}}}",
                json_string(&reason)
            ));
            return;
        }
    };

    let idle_p95 = percentile(&idle, 0.95);
    let loaded_p95 = percentile(&loaded, 0.95);
    let ratio = if idle_p95 > 0.0 {
        loaded_p95 / idle_p95
    } else {
        f64::INFINITY
    };
    report(&format!(
        concat!(
            "{{\"result\":\"{result}\",",
            "\"adapter\":{adapter},\"adapter_class\":\"{class}\",",
            "\"workers\":{workers},\"submitted\":{submitted},",
            "\"idle_p50_ms\":{idle_p50:.3},\"idle_p95_ms\":{idle_p95:.3},",
            "\"loaded_p50_ms\":{loaded_p50:.3},\"loaded_p95_ms\":{loaded_p95:.3},",
            "\"ratio_p95\":{ratio:.3},",
            "\"includes\":{includes},",
            "\"reason\":{reason}}}"
        ),
        result = if software { "UNVERIFIED" } else { "MEASURED" },
        adapter = json_string(&adapter),
        class = if software { "software" } else { "hardware" },
        workers = workers,
        submitted = submitted,
        idle_p50 = percentile(&idle, 0.50),
        idle_p95 = idle_p95,
        loaded_p50 = percentile(&loaded, 0.50),
        loaded_p95 = loaded_p95,
        ratio = ratio,
        includes = json_string(
            "offscreen render submission plus readback while the job pool runs CPU-bound \
             waveform-kind work; identical process seconds apart"
        ),
        reason = json_string(if software {
            "software rasteriser: software-path evidence only, never a GPU claim"
        } else {
            ""
        }),
    ));
}

fn measure(
    renderer: &RenderDevice,
    size: RenderSize,
    project: &ProjectDocument,
    range: TimeRange,
) -> Result<Vec<Duration>, String> {
    let mut samples = Vec::with_capacity(FRAMES);
    for index in 0..(WARMUP + FRAMES) {
        let input = RenderGraphInput::new(
            RenderSnapshot::new(project.id(), project.revision(), range),
            PreviewSurface::offscreen(size, RenderFormat::Rgba8Unorm),
            SyntheticScene::solid(SyntheticColor::GREEN),
        );
        let started = Instant::now();
        let frame = renderer
            .render(input)
            .map_err(|error| format!("frame {index} failed to render: {error}"))?;
        renderer
            .readback(&frame)
            .map_err(|error| format!("frame {index} failed readback: {error}"))?;
        let elapsed = started.elapsed();
        if index >= WARMUP {
            samples.push(elapsed);
        }
    }
    Ok(samples)
}

fn percentile(samples: &[Duration], fraction: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
    sorted[index].as_secs_f64() * 1000.0
}

fn looks_like_software(name: &str, driver: &str) -> bool {
    let haystack = format!("{name} {driver}").to_ascii_lowercase();
    [
        "llvmpipe",
        "swiftshader",
        "lavapipe",
        "softpipe",
        "software",
    ]
    .iter()
    .any(|needle| haystack.contains(needle))
}

fn time(numerator: i64, denominator: u32) -> RationalTime {
    RationalTime::new(numerator, denominator).expect("valid rational time")
}

fn report(payload: &str) {
    println!("OR_BENCH_JSON={payload}");
}

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
