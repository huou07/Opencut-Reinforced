//! Stage breakdown of the composited NV12 path on this named host.
//!
//! `#[ignore]`d by default: this is a measurement lane, not a pass/fail test.
//! It answers one question the export rows cannot: of the ~19 ms a composited
//! 1080p frame costs on an Intel iGPU, how much is plane upload, how much is
//! the conversion pass, and how much is the host readback?
//!
//! Every row uses the same renderer, size and layer stack the export uses, and
//! every row is a full `composite_layers_nv12` call, so the numbers add up the
//! way the product pays them. A software adapter is reported as such and never
//! as device performance.
use or_render::{CompositorLayer, Nv12VideoLayer, RenderDevice, RenderSize, RgbaVideoLayer};
use std::future::Future;

use std::task::{Context, Poll, Waker};
use std::time::Instant;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const WARMUP: usize = 3;
const RUNS: usize = 20;

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park_timeout(std::time::Duration::from_millis(1)),
        }
    }
}

fn measure(label: &str, mut run: impl FnMut() -> usize) {
    for _ in 0..WARMUP {
        let _ = run();
    }
    let mut samples = Vec::with_capacity(RUNS);
    let mut bytes = 0;
    for _ in 0..RUNS {
        let started = Instant::now();
        bytes = run();
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = samples[samples.len() / 2];
    println!(
        "COMPOSE_COST {label} median_ms={median:.3} min_ms={:.3} bytes={bytes}",
        samples[0]
    );
}

#[test]
#[ignore]
fn composited_nv12_stage_cost() {
    let renderer = match block_on(RenderDevice::new()) {
        Ok(renderer) => renderer,
        Err(error) => {
            println!("COMPOSE_COST UNVERIFIED no usable wgpu adapter: {error}");
            return;
        }
    };
    let info = renderer.adapter_info();
    println!(
        "COMPOSE_COST adapter name={:?} backend={:?} type={:?}",
        info.name(),
        info.backend(),
        info.device_type()
    );
    let size = RenderSize::new(WIDTH, HEIGHT).expect("a non-zero size");
    let luma = vec![32_u8; (WIDTH as usize) * (HEIGHT as usize)];
    let chroma = vec![128_u8; (WIDTH as usize) * (HEIGHT as usize) / 2];
    let rgba = vec![0_u8; (WIDTH as usize) * (HEIGHT as usize) * 4];

    measure("empty_scene", || {
        let planes = renderer
            .composite_layers_nv12(size, &[])
            .expect("an empty composite");
        planes.luma().len() + planes.chroma().len()
    });

    measure("one_video_layer", || {
        let nv12 = Nv12VideoLayer::new(WIDTH, HEIGHT, &luma, &chroma).expect("a video layer");
        let planes = renderer
            .composite_layers_nv12(size, &[CompositorLayer::Nv12(nv12)])
            .expect("a video composite");
        planes.luma().len() + planes.chroma().len()
    });

    measure("video_plus_cached_text", || {
        let nv12 = Nv12VideoLayer::new(WIDTH, HEIGHT, &luma, &chroma).expect("a video layer");
        let text = RgbaVideoLayer::new(WIDTH, HEIGHT, &rgba)
            .expect("a text layer")
            .with_cache_key(7);
        let planes = renderer
            .composite_layers_nv12(
                size,
                &[CompositorLayer::Nv12(nv12), CompositorLayer::Rgba(text)],
            )
            .expect("a text composite");
        planes.luma().len() + planes.chroma().len()
    });

    measure("video_plus_uncached_text", || {
        let nv12 = Nv12VideoLayer::new(WIDTH, HEIGHT, &luma, &chroma).expect("a video layer");
        let text = RgbaVideoLayer::new(WIDTH, HEIGHT, &rgba).expect("a text layer");
        let planes = renderer
            .composite_layers_nv12(
                size,
                &[CompositorLayer::Nv12(nv12), CompositorLayer::Rgba(text)],
            )
            .expect("a text composite");
        planes.luma().len() + planes.chroma().len()
    });

    measure("two_video_layers", || {
        let first = Nv12VideoLayer::new(WIDTH, HEIGHT, &luma, &chroma).expect("a video layer");
        let second = Nv12VideoLayer::new(WIDTH, HEIGHT, &luma, &chroma).expect("a video layer");
        let planes = renderer
            .composite_layers_nv12(
                size,
                &[CompositorLayer::Nv12(first), CompositorLayer::Nv12(second)],
            )
            .expect("a two-layer composite");
        planes.luma().len() + planes.chroma().len()
    });

    measure("one_rgba_layer", || {
        let text = RgbaVideoLayer::new(WIDTH, HEIGHT, &rgba).expect("a text layer");
        let planes = renderer
            .composite_layers_nv12(size, &[CompositorLayer::Rgba(text)])
            .expect("an rgba composite");
        planes.luma().len() + planes.chroma().len()
    });

    measure("empty_rgba_readback", || {
        let rendered = renderer
            .composite_layers(size, &[])
            .expect("an empty rgba composite");
        rendered.pixels().len()
    });

    measure("video_rgba_readback", || {
        let nv12 = Nv12VideoLayer::new(WIDTH, HEIGHT, &luma, &chroma).expect("a video layer");
        let rendered = renderer
            .composite_layers(size, &[CompositorLayer::Nv12(nv12)])
            .expect("a video composite");
        rendered.pixels().len()
    });
}
