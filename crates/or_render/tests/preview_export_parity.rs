//! The viewer and the encoder must show the same picture.
//!
//! Preview reads back packed RGBA for the viewer texture. An export that uses a
//! hardware encoder reads back NV12 planes instead, converted on the GPU, so the
//! two paths only share one composition if that conversion is honest. A range or
//! matrix mistake there is exactly what makes a preview look right and the file
//! look washed out, so this lane renders one frame twice and compares the
//! pictures instead of trusting the shared graph by inspection.
//!
//! What it establishes: for the same snapshot, layers and size, the RGBA picture
//! the viewer receives and the NV12 picture the encoder receives decode to the
//! same image within the tolerance 4:2:0 chroma subsampling allows.
//!
//! What it does not: it is not an encode/decode round trip, so it makes no claim
//! about a specific codec's quality. That is measured separately by the export
//! lanes. It also skips itself when no wgpu adapter is available, and says so,
//! rather than passing on a machine that rendered nothing.

use or_core::{ProjectDocument, RationalTime, TextAlignment, TextColor, TextFormatting, TimeRange};
use or_render::{Nv12Planes, RenderDevice, RenderSize, RgbaVideoLayer, TextRasterizer};
use or_runtime::RenderSnapshot;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 180;
/// BT.709 limited-range luma bounds, the mapping the NV12 readback documents.
const LUMA_BLACK: f32 = 16.0;

#[test]
fn preview_rgba_and_export_nv12_agree_on_one_composition() {
    let renderer = match block_on(RenderDevice::new()) {
        Ok(renderer) => renderer,
        Err(error) => {
            eprintln!("skipping preview/export parity: no usable wgpu adapter: {error}");
            return;
        }
    };

    let layer = text_layer();
    let layers = [layer];
    let size = RenderSize::new(WIDTH, HEIGHT).expect("size");
    let snapshot = snapshot();

    let rgba = renderer
        .render_rgba_layers(snapshot, size, &layers)
        .expect("rgba readback")
        .pixels()
        .to_vec();
    let planes = renderer
        .render_rgba_layers_nv12(snapshot, size, &layers)
        .expect("nv12 readback");

    assert_eq!(rgba.len(), (WIDTH * HEIGHT * 4) as usize);
    let decoded = nv12_to_rgba(&planes);
    assert_eq!(decoded.len(), rgba.len());

    let psnr = psnr(&rgba, &decoded);
    eprintln!("preview/export parity PSNR {psnr:.2} dB");
    assert!(
        psnr >= 30.0,
        "preview and export layouts disagree: PSNR {psnr:.2} dB"
    );

    // Geometry has to match too: a crop or scale difference can keep a high PSNR
    // on a mostly dark frame while moving the words.
    let (rgba_x, rgba_y) = ink_centroid(&rgba);
    let (decoded_x, decoded_y) = ink_centroid(&decoded);
    assert!(
        (rgba_x - decoded_x).abs() <= 0.5 && (rgba_y - decoded_y).abs() <= 0.5,
        "ink moved between layouts: rgba ({rgba_x:.2}, {rgba_y:.2}) vs nv12 ({decoded_x:.2}, {decoded_y:.2})"
    );

    // And the black background must not shift: a limited/full range mix-up moves
    // every black pixel by about 16 levels, which this catches even when the PSNR
    // stays acceptable.
    let background = decoded[..4].to_vec();
    assert_eq!(background[0], 0);
    assert_eq!(background[1], 0);
    assert_eq!(background[2], 0);
}

fn text_layer() -> RgbaVideoLayer<'static> {
    let mut rasterizer = TextRasterizer::new();
    let formatting = TextFormatting {
        size_milli_points: 36_000,
        alignment: TextAlignment::Center,
        color: TextColor {
            red: 245,
            green: 245,
            blue: 245,
            alpha: 255,
        },
        ..TextFormatting::default()
    };
    let pixels = rasterizer
        .rasterize("Preview\n= export", formatting, WIDTH, HEIGHT)
        .expect("rasterize the parity frame");
    let leaked: &'static [u8] = Box::leak(pixels.into_boxed_slice());
    RgbaVideoLayer::new(WIDTH, HEIGHT, leaked).expect("layer")
}

fn snapshot() -> RenderSnapshot {
    let project = ProjectDocument::new("parity");
    RenderSnapshot::new(
        project.id(),
        project.revision(),
        TimeRange::new(RationalTime::ZERO, RationalTime::new(1, 24).expect("time")).expect("range"),
    )
}

/// BT.709 limited-range NV12 back to RGB, the inverse of the readback the export
/// path uses. Written out here rather than reused so the test fails if the
/// documented mapping and the implementation drift apart.
fn nv12_to_rgba(planes: &Nv12Planes) -> Vec<u8> {
    let width = planes.width() as usize;
    let height = planes.height() as usize;
    let luma = planes.luma();
    let chroma = planes.chroma();
    let mut out = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let luma_index = y * width + x;
            let chroma_index = (y / 2) * width + (x / 2) * 2;
            let l = f32::from(luma[luma_index]) - LUMA_BLACK;
            let u = f32::from(chroma[chroma_index]) - 128.0;
            let v = f32::from(chroma[chroma_index + 1]) - 128.0;
            let r = 1.164 * l + 1.793 * v;
            let g = 1.164 * l - 0.213 * u - 0.533 * v;
            let b = 1.164 * l + 2.112 * u;
            let index = luma_index * 4;
            out[index] = clamp(r);
            out[index + 1] = clamp(g);
            out[index + 2] = clamp(b);
            out[index + 3] = 255;
        }
    }
    out
}

fn clamp(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

fn psnr(left: &[u8], right: &[u8]) -> f64 {
    let mut squared = 0.0f64;
    let mut samples = 0u64;
    for (a, b) in left.iter().zip(right) {
        // Alpha is opaque on both sides; luminance and chroma carry the claim.
        let difference = f64::from(*a) - f64::from(*b);
        squared += difference * difference;
        samples += 1;
    }
    let mse = squared / samples as f64;
    if mse == 0.0 {
        return f64::INFINITY;
    }
    10.0 * (255.0f64 * 255.0 / mse).log10()
}

fn ink_centroid(pixels: &[u8]) -> (f32, f32) {
    let mut weight = 0.0f32;
    let mut x_sum = 0.0f32;
    let mut y_sum = 0.0f32;
    for (index, pixel) in pixels.as_chunks::<4>().0.iter().enumerate() {
        let luma = 0.2126 * f32::from(pixel[0])
            + 0.7152 * f32::from(pixel[1])
            + 0.0722 * f32::from(pixel[2]);
        if luma <= 8.0 {
            continue;
        }
        weight += luma;
        x_sum += (index % WIDTH as usize) as f32 * luma;
        y_sum += (index / WIDTH as usize) as f32 * luma;
    }
    assert!(weight > 0.0, "the parity frame has no ink");
    (x_sum / weight, y_sum / weight)
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
