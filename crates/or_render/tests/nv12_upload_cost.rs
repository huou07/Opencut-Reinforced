//! What one NV12 frame upload actually costs on this host.
//!
//! `write_texture` with a tight 1920-byte row is not aligned to the 256-byte
//! copy pitch Vulkan and D3D12 require, so wgpu repacks it row by row into a
//! staging buffer before the copy. This lane measures that path against a
//! pre-padded 2048-byte row (which takes wgpu's single-memcpy fast path) and
//! against a persistent staging buffer copied by hand, so the choice can be
//! made on numbers instead of on the hypothesis.
//!
//! `#[ignore]`d: a measurement lane, not a pass/fail test.
use std::future::Future;
use std::task::{Context, Poll, Wake, Waker};
use std::time::Instant;

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const PAD: u32 = 2048;
const RUNS: usize = 20;
const WARMUP: usize = 3;

struct NoopWake;
impl Wake for NoopWake {
    fn wake(self: std::sync::Arc<Self>) {}
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let waker = Waker::from(std::sync::Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park_timeout(std::time::Duration::from_millis(1)),
        }
    }
}

fn measure(label: &str, mut run: impl FnMut() -> usize) {
    for _ in 0..WARMUP {
        run();
    }
    let mut samples = Vec::with_capacity(RUNS);
    for _ in 0..RUNS {
        let started = Instant::now();
        let _ = run();
        samples.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "UPLOAD_COST {label} median_ms={:.3} min_ms={:.3}",
        samples[samples.len() / 2],
        samples[0]
    );
}

fn texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("upload_probe"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

#[test]
#[ignore]
fn nv12_upload_cost() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = match block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    })) {
        Ok(adapter) => adapter,
        Err(error) => {
            println!("UPLOAD_COST UNVERIFIED no adapter: {error}");
            return;
        }
    };
    let info = adapter.get_info();
    println!(
        "UPLOAD_COST adapter name={:?} backend={:?} type={:?}",
        info.name, info.backend, info.device_type,
    );
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("upload_probe"),
        ..Default::default()
    }))
    .expect("a device");

    let luma = vec![32_u8; (WIDTH as usize) * (HEIGHT as usize)];
    let chroma = vec![128_u8; (WIDTH as usize) * (HEIGHT as usize) / 2];
    let mut luma_padded = vec![0_u8; (PAD as usize) * (HEIGHT as usize)];
    for row in 0..HEIGHT as usize {
        luma_padded[row * PAD as usize..row * PAD as usize + WIDTH as usize]
            .copy_from_slice(&luma[row * WIDTH as usize..(row + 1) * WIDTH as usize]);
    }
    let mut chroma_padded = vec![0_u8; (PAD as usize) * (HEIGHT as usize) / 2];
    for row in 0..HEIGHT as usize / 2 {
        chroma_padded[row * PAD as usize..row * PAD as usize + WIDTH as usize]
            .copy_from_slice(&chroma[row * WIDTH as usize..(row + 1) * WIDTH as usize]);
    }

    let tight_luma = texture(&device, WIDTH, HEIGHT, wgpu::TextureFormat::R8Unorm);
    let tight_chroma = texture(
        &device,
        WIDTH / 2,
        HEIGHT / 2,
        wgpu::TextureFormat::Rg8Unorm,
    );
    let padded_luma = texture(&device, WIDTH, HEIGHT, wgpu::TextureFormat::R8Unorm);
    let padded_chroma = texture(
        &device,
        WIDTH / 2,
        HEIGHT / 2,
        wgpu::TextureFormat::Rg8Unorm,
    );

    let mut memcpy_copy = vec![0_u8; luma.len() + chroma.len()];
    measure("memcpy_3_1mb", || {
        memcpy_copy[..luma.len()].copy_from_slice(&luma);
        memcpy_copy[luma.len()..].copy_from_slice(&chroma);
        memcpy_copy.len()
    });

    fn repack(luma: &[u8], chroma: &[u8], luma_padded: &mut [u8], chroma_padded: &mut [u8]) {
        for (target, source) in luma_padded
            .chunks_exact_mut(PAD as usize)
            .zip(luma.chunks_exact(WIDTH as usize))
        {
            target[..WIDTH as usize].copy_from_slice(source);
        }
        for (target, source) in chroma_padded
            .chunks_exact_mut(PAD as usize)
            .zip(chroma.chunks_exact(WIDTH as usize))
        {
            target[..WIDTH as usize].copy_from_slice(source);
        }
    }

    measure("write_tight", || {
        queue.write_texture(
            tight_luma.as_image_copy(),
            &luma,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH),
                rows_per_image: Some(HEIGHT),
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        queue.write_texture(
            tight_chroma.as_image_copy(),
            &chroma,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH),
                rows_per_image: Some(HEIGHT / 2),
            },
            wgpu::Extent3d {
                width: WIDTH / 2,
                height: HEIGHT / 2,
                depth_or_array_layers: 1,
            },
        );
        device.poll(wgpu::PollType::Wait).expect("poll");
        luma.len() + chroma.len()
    });

    measure("write_padded", || {
        repack(&luma, &chroma, &mut luma_padded, &mut chroma_padded);
        queue.write_texture(
            padded_luma.as_image_copy(),
            &luma_padded,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(PAD),
                rows_per_image: Some(HEIGHT),
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        queue.write_texture(
            padded_chroma.as_image_copy(),
            &chroma_padded,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(PAD),
                rows_per_image: Some(HEIGHT / 2),
            },
            wgpu::Extent3d {
                width: WIDTH / 2,
                height: HEIGHT / 2,
                depth_or_array_layers: 1,
            },
        );
        device.poll(wgpu::PollType::Wait).expect("poll");
        luma.len() + chroma.len()
    });

    // One persistent staging buffer, one submit for both planes: the shape a
    // single-copy upload would take.
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("upload_probe.staging"),
        size: (PAD as u64) * (HEIGHT as u64) * 3 / 2,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    measure("copy_persistent", || {
        repack(&luma, &chroma, &mut luma_padded, &mut chroma_padded);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("upload_probe.encoder"),
        });
        // upload_probe writes through a queue write into the same staging
        // buffer, then copies both planes with one submit.
        queue.write_buffer(&staging, 0, &luma_padded);
        queue.write_buffer(&staging, (PAD as u64) * (HEIGHT as u64), &chroma_padded);
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(PAD),
                    rows_per_image: Some(HEIGHT),
                },
            },
            padded_luma.as_image_copy(),
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: (PAD as u64) * (HEIGHT as u64),
                    bytes_per_row: Some(PAD),
                    rows_per_image: Some(HEIGHT / 2),
                },
            },
            padded_chroma.as_image_copy(),
            wgpu::Extent3d {
                width: WIDTH / 2,
                height: HEIGHT / 2,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        device.poll(wgpu::PollType::Wait).expect("poll");
        luma.len() + chroma.len()
    });
}
