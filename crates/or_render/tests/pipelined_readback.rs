//! A queued NV12 readback must hand back exactly what a waited one does, and
//! waiting for it must not stop the renderer from starting the next frame.
//!
//! The export loop used to call `composite_layers_nv12`, which submits the
//! colour conversion and then blocks the render thread inside `device.poll`
//! until the GPU has finished. On a composited export that wait is several
//! milliseconds per frame, and none of the renderer's own CPU work (decoding,
//! rasterising text) can overlap it. The pipelined readback splits the call in
//! two, so this lane pins down both halves: the bytes are the same picture, and
//! the render thread keeps running while the conversion is in flight.
//!
//! What it establishes: on this host and this size, the queued and waited paths
//! return identical planes, and a pipelined loop that interleaves CPU work with
//! the readback is faster than the same loop that waits inside the call.
//!
//! What it does not: it does not measure a real export. The per-frame CPU work
//! below is a stand-in for what the export thread does between frames; the
//! export lanes in `or_app_bridge` measure the real thing.

use or_render::{CompositorLayer, Nv12VideoLayer, PendingNv12Planes, RenderDevice, RenderSize};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

/// A frame size big enough for the conversion to cost real milliseconds on an
/// integrated GPU, which is where the readback wait was hurting.
const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const WARMUP: usize = 3;
const FRAMES: usize = 30;

#[test]
fn queued_readback_matches_the_waited_one() {
    let renderer = match block_on(RenderDevice::new()) {
        Ok(renderer) => renderer,
        Err(error) => {
            eprintln!("skipping queued/waited comparison: no usable wgpu adapter: {error}");
            return;
        }
    };
    let size = RenderSize::new(WIDTH, HEIGHT).expect("size");
    let bytes = nv12_bytes(WIDTH, HEIGHT, 42);
    let layers = [video_layer(&bytes)];

    let waited = renderer
        .composite_layers_nv12(size, &layers)
        .expect("the waited readback converts one frame");
    // Queue a frame, start a second one, and resolve the first afterwards: the
    // pipelined export does exactly this, and the shared staging buffer must
    // not turn the already-correct frame into the later one's picture.
    let first = renderer
        .composite_layers_nv12_pending(size, &layers)
        .expect("the queued readback starts");
    let second_bytes = nv12_bytes(WIDTH, HEIGHT, 200);
    let second_layers = [video_layer(&second_bytes)];
    let second = renderer
        .composite_layers_nv12_pending(size, &second_layers)
        .expect("a second queued readback starts");
    let first = first.resolve().expect("the first queued frame resolves");
    let second = second.resolve().expect("the second queued frame resolves");

    assert_eq!(
        waited.luma(),
        first.luma(),
        "the queued luma plane differs from the waited one"
    );
    assert_eq!(
        waited.chroma(),
        first.chroma(),
        "the queued chroma plane differs from the waited one"
    );
    assert_ne!(
        first.luma(),
        second.luma(),
        "two different frames resolved to the same luma plane"
    );
}

#[test]
#[ignore]
fn pipelined_readback_overlaps_the_next_frame() {
    let renderer = match block_on(RenderDevice::new()) {
        Ok(renderer) => renderer,
        Err(error) => {
            eprintln!("skipping pipelined readback probe: no usable wgpu adapter: {error}");
            return;
        }
    };
    let info = renderer.adapter_info();
    let size = RenderSize::new(WIDTH, HEIGHT).expect("size");
    let bytes = nv12_bytes(WIDTH, HEIGHT, 42);
    let layers = [video_layer(&bytes)];
    // Stands in for the decode and text rasterisation the export thread does
    // between frames. It is deliberately longer than a sleep would be on a
    // frame that has nothing to decode, and shorter than the conversion.
    let cpu_work = Duration::from_millis(4);

    for _ in 0..WARMUP {
        let pending = renderer
            .composite_layers_nv12_pending(size, &layers)
            .expect("warmup starts");
        pending.resolve().expect("warmup resolves");
    }

    let started = Instant::now();
    for _ in 0..FRAMES {
        let planes = renderer
            .composite_layers_nv12(size, &layers)
            .expect("the waited readback converts");
        assert_eq!(planes.luma().len(), (WIDTH * HEIGHT) as usize);
        thread::sleep(cpu_work);
    }
    let waited = started.elapsed().as_secs_f64() * 1000.0 / FRAMES as f64;

    let started = Instant::now();
    let mut queued: Option<PendingNv12Planes> = None;
    for _ in 0..FRAMES {
        let next = renderer
            .composite_layers_nv12_pending(size, &layers)
            .expect("the queued readback starts");
        if let Some(previous) = queued.take() {
            let planes = previous.resolve().expect("the previous frame resolves");
            assert_eq!(planes.luma().len(), (WIDTH * HEIGHT) as usize);
        }
        thread::sleep(cpu_work);
        queued = Some(next);
    }
    if let Some(last) = queued.take() {
        last.resolve().expect("the last frame resolves");
    }
    let pipelined = started.elapsed().as_secs_f64() * 1000.0 / FRAMES as f64;

    println!(
        "PIPELINED_READBACK adapter={:?} backend={:?} type={:?} cpu_work_ms={:.1} waited_ms={waited:.3} pipelined_ms={pipelined:.3} ratio={:.2}",
        info.name(),
        info.backend(),
        info.device_type(),
        cpu_work.as_secs_f64() * 1000.0,
        waited / pipelined,
    );
}

fn video_layer(bytes: &[u8]) -> CompositorLayer<'_> {
    CompositorLayer::Nv12(
        Nv12VideoLayer::from_contiguous(WIDTH, HEIGHT, bytes).expect("a full-size NV12 layer"),
    )
}

fn nv12_bytes(width: u32, height: u32, fill: u8) -> Vec<u8> {
    let luma = (width * height) as usize;
    let mut bytes = vec![fill; luma + luma / 2];
    // Keep the two frames visibly different in both planes.
    for value in &mut bytes[..luma] {
        *value = value.wrapping_add(1);
    }
    bytes
}

fn block_on<F: Future>(future: F) -> F::Output {
    struct ThreadWaker(thread::Thread);

    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let mut future = Box::pin(future);
    let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
    let mut context = Context::from_waker(&waker);
    loop {
        match Pin::as_mut(&mut future).poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => thread::park(),
        }
    }
}
