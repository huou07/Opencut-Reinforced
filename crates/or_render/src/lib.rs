//! The shared wgpu render spine for the runtime plane.
//!
//! Phase 7B established a platform-neutral preview-surface contract,
//! deterministic synthetic render-graph inputs, an offscreen wgpu target, and
//! an explicit CPU readback path. Phase 7F adds GPU composition of decoded
//! software video layers. Native adapters own viewer presentation; full-rate
//! frame bytes remain outside the Flutter boundary.

use or_core::{Crop, Opacity, Transform};
use or_runtime::{
    FrameAccess, FrameColorInfo, FrameDescriptor, FrameMemoryDomain, FramePixelFormat, FrameTiming,
    RenderSnapshot,
};
use std::{error::Error, fmt, sync::mpsc};
use wgpu::util::DeviceExt;

/// The exact upstream wgpu version used by this render foundation.
pub const WGPU_VERSION: &str = "25.0.2";

/// A validated 2D render extent.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RenderSize {
    width: u32,
    height: u32,
}

impl RenderSize {
    pub const fn new(width: u32, height: u32) -> Result<Self, RenderSizeError> {
        if width == 0 {
            return Err(RenderSizeError::ZeroWidth);
        }
        if height == 0 {
            return Err(RenderSizeError::ZeroHeight);
        }
        Ok(Self { width, height })
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }

    pub const fn pixel_count(self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderSizeError {
    ZeroWidth,
    ZeroHeight,
}

impl fmt::Display for RenderSizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ZeroWidth => "render width must be positive",
            Self::ZeroHeight => "render height must be positive",
        })
    }
}

impl Error for RenderSizeError {}

/// Formats supported by the first offscreen render target.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RenderFormat {
    Rgba8Unorm,
}

impl RenderFormat {
    const fn wgpu(self) -> wgpu::TextureFormat {
        match self {
            Self::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        }
    }
}

/// Opaque presentation identity supplied by the native viewer adapter.
///
/// This is deliberately an application-level identity, not a raw Metal,
/// DX12, Vulkan, or OS handle. Native interop belongs in a runtime adapter.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ViewerSurfaceHandle(u64);

impl ViewerSurfaceHandle {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

/// The presentation destination described by a preview surface.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PreviewSurfaceTarget {
    Offscreen,
    Viewer(ViewerSurfaceHandle),
}

/// A platform-neutral preview-surface abstraction.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PreviewSurface {
    size: RenderSize,
    format: RenderFormat,
    target: PreviewSurfaceTarget,
}

impl PreviewSurface {
    pub const fn offscreen(size: RenderSize, format: RenderFormat) -> Self {
        Self {
            size,
            format,
            target: PreviewSurfaceTarget::Offscreen,
        }
    }

    pub const fn viewer(
        handle: ViewerSurfaceHandle,
        size: RenderSize,
        format: RenderFormat,
    ) -> Self {
        Self {
            size,
            format,
            target: PreviewSurfaceTarget::Viewer(handle),
        }
    }

    pub const fn size(self) -> RenderSize {
        self.size
    }

    pub const fn format(self) -> RenderFormat {
        self.format
    }

    pub const fn target(self) -> PreviewSurfaceTarget {
        self.target
    }
}

/// The viewer-facing contract. It contains only a surface identity and its
/// presentation description; it has no frame byte buffer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ViewerSurfaceContract {
    surface: PreviewSurface,
    handle: ViewerSurfaceHandle,
}

impl ViewerSurfaceContract {
    pub const fn new(handle: ViewerSurfaceHandle, size: RenderSize, format: RenderFormat) -> Self {
        Self {
            surface: PreviewSurface::viewer(handle, size, format),
            handle,
        }
    }

    pub const fn handle(self) -> ViewerSurfaceHandle {
        self.handle
    }

    pub const fn surface(self) -> PreviewSurface {
        self.surface
    }
}

/// A deterministic RGBA color used by synthetic scenes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SyntheticColor([u8; 4]);

impl SyntheticColor {
    pub const BLACK: Self = Self([0, 0, 0, 255]);
    pub const GREEN: Self = Self([0, 255, 0, 255]);

    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self([red, green, blue, alpha])
    }

    pub const fn channels(self) -> [u8; 4] {
        self.0
    }

    fn wgpu(self) -> wgpu::Color {
        let [red, green, blue, alpha] = self.0;
        wgpu::Color {
            r: red as f64 / 255.0,
            g: green as f64 / 255.0,
            b: blue as f64 / 255.0,
            a: alpha as f64 / 255.0,
        }
    }
}

/// The first deterministic scene family. Later graph nodes can extend this
/// value without moving presentation or project state into the renderer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SyntheticScene {
    clear_color: SyntheticColor,
}

impl SyntheticScene {
    pub const fn solid(clear_color: SyntheticColor) -> Self {
        Self { clear_color }
    }

    pub const fn clear_color(self) -> SyntheticColor {
        self.clear_color
    }
}

/// Immutable inputs consumed by the render graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderGraphInput {
    snapshot: RenderSnapshot,
    surface: PreviewSurface,
    scene: SyntheticScene,
}

impl RenderGraphInput {
    pub const fn new(
        snapshot: RenderSnapshot,
        surface: PreviewSurface,
        scene: SyntheticScene,
    ) -> Self {
        Self {
            snapshot,
            surface,
            scene,
        }
    }

    pub const fn snapshot(self) -> RenderSnapshot {
        self.snapshot
    }

    pub const fn surface(self) -> PreviewSurface {
        self.surface
    }

    pub const fn scene(self) -> SyntheticScene {
        self.scene
    }
}

/// Copyable adapter metadata kept separate from native interop resources.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderAdapterInfo {
    name: String,
    driver: String,
    driver_info: String,
    backend: wgpu::Backend,
}

impl RenderAdapterInfo {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn driver(&self) -> &str {
        &self.driver
    }

    pub fn driver_info(&self) -> &str {
        &self.driver_info
    }

    pub const fn backend(&self) -> wgpu::Backend {
        self.backend
    }
}

/// A wgpu device/queue pair for the shared render spine.
pub struct RenderDevice {
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    video_bind_group_layout: wgpu::BindGroupLayout,
    video_pipeline: wgpu::RenderPipeline,
    video_sampler: wgpu::Sampler,
    adapter_info: RenderAdapterInfo,
    limits: wgpu::Limits,
}

impl RenderDevice {
    /// Returns the backends compiled into this crate for the current target.
    pub const fn compiled_backends() -> wgpu::Backends {
        wgpu::Instance::enabled_backend_features()
    }

    /// Creates a device using the compiled native backends.
    pub async fn new() -> Result<Self, RenderError> {
        Self::new_with_backends(Self::compiled_backends()).await
    }

    /// Creates a headless device using a caller-selected subset of compiled
    /// backends. No window or surface handle is required.
    pub async fn new_with_backends(backends: wgpu::Backends) -> Result<Self, RenderError> {
        let backends = backends & Self::compiled_backends();
        if backends.is_empty() {
            return Err(RenderError::NoCompiledBackend);
        }

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .map_err(|error| RenderError::AdapterUnavailable {
                message: error.to_string(),
            })?;
        let adapter_info = adapter.get_info();
        let limits = adapter.limits();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("or_render"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| RenderError::DeviceUnavailable {
                message: error.to_string(),
            })?;
        let video_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("or_render.video_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let video_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("or_render.video_blit"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct LayerSettings {
    translation_scale: vec4<f32>,
    rotation_anchor_aspect: vec4<f32>,
    crop: vec4<f32>,
    opacity: vec4<f32>,
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: LayerSettings;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let point = positions[index];
    let anchor = vec2<f32>(
        settings.rotation_anchor_aspect.y * 2.0 - 1.0,
        1.0 - settings.rotation_anchor_aspect.z * 2.0,
    );
    let aspect = settings.rotation_anchor_aspect.w;
    let aspect_scale = vec2<f32>(aspect, 1.0);
    let delta = (point - anchor) * aspect_scale;
    let scaled = delta * settings.translation_scale.zw;
    let angle = settings.rotation_anchor_aspect.x;
    let cosine = cos(angle);
    let sine = sin(angle);
    let rotated = vec2<f32>(
        cosine * scaled.x + sine * scaled.y,
        -sine * scaled.x + cosine * scaled.y,
    );
    var output: VertexOutput;
    output.position = vec4<f32>(
        anchor + rotated / aspect_scale + settings.translation_scale.xy,
        0.0,
        1.0,
    );
    output.uv = vec2<f32>((point.x + 1.0) * 0.5, (1.0 - point.y) * 0.5);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let crop_size = vec2<f32>(1.0) - settings.crop.xy - settings.crop.zw;
    let source_uv = settings.crop.xy + input.uv * crop_size;
    var color = textureSample(source, source_sampler, source_uv);
    color.a *= settings.opacity.x;
    return color;
}
"#
                .into(),
            ),
        });
        let video_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("or_render.video_pipeline_layout"),
                bind_group_layouts: &[&video_bind_group_layout],
                push_constant_ranges: &[],
            });
        let video_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("or_render.video_pipeline"),
            layout: Some(&video_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &video_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &video_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let video_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("or_render.video_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            video_bind_group_layout,
            video_pipeline,
            video_sampler,
            adapter_info: RenderAdapterInfo {
                name: adapter_info.name,
                driver: adapter_info.driver,
                driver_info: adapter_info.driver_info,
                backend: adapter_info.backend,
            },
            limits,
        })
    }

    pub fn adapter_info(&self) -> &RenderAdapterInfo {
        &self.adapter_info
    }

    pub const fn limits(&self) -> &wgpu::Limits {
        &self.limits
    }

    /// Renders one immutable graph input to an owned offscreen target.
    pub fn render(&self, input: RenderGraphInput) -> Result<RenderedFrame, RenderError> {
        RenderGraph.render(self, input)
    }

    /// Copies an offscreen target into a tightly-packed CPU buffer.
    ///
    /// This is an explicit diagnostic/readback path for headless tests and
    /// future encoders. Viewer presentation uses the surface contract instead
    /// and does not call this method per frame.
    pub fn readback(&self, frame: &RenderedFrame) -> Result<OffscreenReadback, RenderError> {
        if frame.surface.target() != PreviewSurfaceTarget::Offscreen {
            return Err(RenderError::ViewerSurfaceNotImplemented);
        }

        let size = frame.surface.size();
        let unpadded_bytes_per_row = size
            .width()
            .checked_mul(4)
            .ok_or(RenderError::ReadbackSizeOverflow)?;
        let padded_bytes_per_row = align_to_copy_row(unpadded_bytes_per_row)?;
        let buffer_size = u64::from(padded_bytes_per_row)
            .checked_mul(u64::from(size.height()))
            .ok_or(RenderError::ReadbackSizeOverflow)?;
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("or_render.readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("or_render.readback_encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &frame.target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(size.height()),
                },
            },
            wgpu::Extent3d {
                width: size.width(),
                height: size.height(),
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = staging_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result.map_err(|error| error.to_string()));
        });
        self.device
            .poll(wgpu::PollType::Wait)
            .map_err(|error| RenderError::DevicePoll {
                message: error.to_string(),
            })?;
        receiver
            .recv()
            .map_err(|error| RenderError::ReadbackMap {
                message: error.to_string(),
            })?
            .map_err(|message| RenderError::ReadbackMap { message })?;

        let mapped = slice.get_mapped_range();
        let row_bytes = usize::try_from(unpadded_bytes_per_row)
            .map_err(|_| RenderError::ReadbackSizeOverflow)?;
        let padded_row_bytes =
            usize::try_from(padded_bytes_per_row).map_err(|_| RenderError::ReadbackSizeOverflow)?;
        let mut pixels = Vec::with_capacity(
            row_bytes
                .checked_mul(size.height() as usize)
                .ok_or(RenderError::ReadbackSizeOverflow)?,
        );
        for row in mapped.chunks_exact(padded_row_bytes) {
            pixels.extend_from_slice(&row[..row_bytes]);
        }
        drop(mapped);
        staging_buffer.unmap();

        Ok(OffscreenReadback {
            surface: frame.surface,
            pixels,
        })
    }

    /// Composites software-decoded RGBA layers through wgpu and reads back the
    /// bounded CPU presentation frame required by the desktop texture fallback.
    pub fn render_rgba_layers(
        &self,
        snapshot: RenderSnapshot,
        size: RenderSize,
        layers: &[RgbaVideoLayer<'_>],
    ) -> Result<OffscreenReadback, RenderError> {
        let frame = RenderGraph.render_rgba_layers(self, snapshot, size, layers)?;
        self.readback(&frame)
    }
}

impl fmt::Debug for RenderDevice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RenderDevice")
            .field("adapter_info", &self.adapter_info)
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

/// The shared wgpu graph entry point for this checkpoint.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderGraph;

impl RenderGraph {
    pub fn render(
        &self,
        renderer: &RenderDevice,
        input: RenderGraphInput,
    ) -> Result<RenderedFrame, RenderError> {
        let surface = input.surface();
        if surface.target() != PreviewSurfaceTarget::Offscreen {
            return Err(RenderError::ViewerSurfaceNotImplemented);
        }

        let size = surface.size();
        if size.width() > renderer.limits.max_texture_dimension_2d
            || size.height() > renderer.limits.max_texture_dimension_2d
        {
            return Err(RenderError::SizeExceedsDeviceLimit {
                size,
                limit: renderer.limits.max_texture_dimension_2d,
            });
        }

        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("or_render.synthetic_target"),
            size: wgpu::Extent3d {
                width: size.width(),
                height: size.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface.format().wgpu(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(input.scene().clear_color().wgpu()),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("or_render.synthetic_encoder"),
            });
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("or_render.synthetic_pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        renderer.queue.submit(Some(encoder.finish()));

        let descriptor = FrameDescriptor::new(
            FrameMemoryDomain::HardwareSurface,
            size.width(),
            size.height(),
            FramePixelFormat::Rgba8,
            FrameColorInfo::default(),
            FrameTiming::at(input.snapshot().requested_range().start()),
            FrameAccess::ReadOnly,
        )
        .map_err(|error| RenderError::FrameDescriptor {
            message: error.to_string(),
        })?;

        Ok(RenderedFrame {
            snapshot: input.snapshot(),
            surface,
            descriptor,
            target: RenderTarget { texture },
        })
    }

    fn render_rgba_layers(
        &self,
        renderer: &RenderDevice,
        snapshot: RenderSnapshot,
        size: RenderSize,
        layers: &[RgbaVideoLayer<'_>],
    ) -> Result<RenderedFrame, RenderError> {
        if size.width() > renderer.limits.max_texture_dimension_2d
            || size.height() > renderer.limits.max_texture_dimension_2d
        {
            return Err(RenderError::SizeExceedsDeviceLimit {
                size,
                limit: renderer.limits.max_texture_dimension_2d,
            });
        }
        for layer in layers {
            layer.validate()?;
            if layer.width > renderer.limits.max_texture_dimension_2d
                || layer.height > renderer.limits.max_texture_dimension_2d
            {
                return Err(RenderError::SourceSizeExceedsDeviceLimit {
                    width: layer.width,
                    height: layer.height,
                    limit: renderer.limits.max_texture_dimension_2d,
                });
            }
        }

        let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("or_render.viewer_target"),
            size: wgpu::Extent3d {
                width: size.width(),
                height: size.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut sources = Vec::with_capacity(layers.len());
        for layer in layers {
            let source = renderer.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("or_render.viewer_source"),
                size: wgpu::Extent3d {
                    width: layer.width,
                    height: layer.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            renderer.queue.write_texture(
                source.as_image_copy(),
                layer.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(layer.width * 4),
                    rows_per_image: Some(layer.height),
                },
                wgpu::Extent3d {
                    width: layer.width,
                    height: layer.height,
                    depth_or_array_layers: 1,
                },
            );
            let view = source.create_view(&wgpu::TextureViewDescriptor::default());
            let uniform = renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("or_render.viewer_layer_settings"),
                    contents: &layer.settings_uniform(size),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let bind_group = renderer
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("or_render.viewer_layer"),
                    layout: &renderer.video_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&renderer.video_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: uniform.as_entire_binding(),
                        },
                    ],
                });
            sources.push((source, view, uniform, bind_group));
        }

        let attachments = [Some(wgpu::RenderPassColorAttachment {
            view: &target_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(SyntheticColor::BLACK.wgpu()),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("or_render.viewer_encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("or_render.viewer_pass"),
                color_attachments: &attachments,
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&renderer.video_pipeline);
            for (_, _, _, bind_group) in &sources {
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        renderer.queue.submit(Some(encoder.finish()));

        let descriptor = FrameDescriptor::new(
            FrameMemoryDomain::HardwareSurface,
            size.width(),
            size.height(),
            FramePixelFormat::Rgba8,
            FrameColorInfo::default(),
            FrameTiming::at(snapshot.requested_range().start()),
            FrameAccess::ReadOnly,
        )
        .map_err(|error| RenderError::FrameDescriptor {
            message: error.to_string(),
        })?;
        Ok(RenderedFrame {
            snapshot,
            surface: PreviewSurface::offscreen(size, RenderFormat::Rgba8Unorm),
            descriptor,
            target: RenderTarget { texture: target },
        })
    }
}

/// One software-decoded RGBA source layer passed to the render graph.
#[derive(Clone, Copy, Debug)]
pub struct RgbaVideoLayer<'a> {
    width: u32,
    height: u32,
    pixels: &'a [u8],
    transform: Transform,
    crop: Crop,
    opacity: Opacity,
}

impl<'a> RgbaVideoLayer<'a> {
    pub fn new(width: u32, height: u32, pixels: &'a [u8]) -> Result<Self, RenderError> {
        let layer = Self {
            width,
            height,
            pixels,
            transform: Transform::IDENTITY,
            crop: Crop::NONE,
            opacity: Opacity::OPAQUE,
        };
        layer.validate()?;
        Ok(layer)
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }

    pub fn with_visual_settings(
        mut self,
        transform: Transform,
        crop: Crop,
        opacity: Opacity,
    ) -> Result<Self, RenderError> {
        if !transform.is_valid() || !crop.is_valid() || !opacity.is_valid() {
            return Err(RenderError::InvalidVideoSettings);
        }
        self.transform = transform;
        self.crop = crop;
        self.opacity = opacity;
        Ok(self)
    }

    fn settings_uniform(&self, size: RenderSize) -> [u8; 64] {
        let transform = self.transform;
        let crop = self.crop;
        let opacity = self.opacity;
        let angle = (transform.rotation_milli_degrees as f32 / 1_000.0).to_radians();
        let values = [
            transform.x_milli_canvas as f32 / 500.0,
            -(transform.y_milli_canvas as f32) / 500.0,
            transform.scale_x_milli as f32 / 1_000.0,
            transform.scale_y_milli as f32 / 1_000.0,
            angle,
            transform.anchor_x_basis_points as f32 / 10_000.0,
            transform.anchor_y_basis_points as f32 / 10_000.0,
            size.width() as f32 / size.height() as f32,
            crop.left_basis_points as f32 / 10_000.0,
            crop.top_basis_points as f32 / 10_000.0,
            crop.right_basis_points as f32 / 10_000.0,
            crop.bottom_basis_points as f32 / 10_000.0,
            opacity.basis_points as f32 / 10_000.0,
            0.0,
            0.0,
            0.0,
        ];
        let mut bytes = [0; 64];
        for (index, value) in values.into_iter().enumerate() {
            let offset = index * 4;
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn validate(&self) -> Result<(), RenderError> {
        let length = (self.width as usize)
            .checked_mul(self.height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|length| *length <= 256 * 1024 * 1024)
            .ok_or(RenderError::InvalidSourceFrame)?;
        if self.width == 0 || self.height == 0 || self.pixels.len() != length {
            return Err(RenderError::InvalidSourceFrame);
        }
        Ok(())
    }
}

/// An owned GPU render target and its immutable runtime metadata.
///
/// Dropping this value releases the wgpu texture through normal RAII. No
/// native backend handle is exposed or serialized.
pub struct RenderedFrame {
    snapshot: RenderSnapshot,
    surface: PreviewSurface,
    descriptor: FrameDescriptor,
    target: RenderTarget,
}

impl RenderedFrame {
    pub const fn snapshot(&self) -> RenderSnapshot {
        self.snapshot
    }

    pub const fn surface(&self) -> PreviewSurface {
        self.surface
    }

    pub const fn descriptor(&self) -> &FrameDescriptor {
        &self.descriptor
    }
}

impl fmt::Debug for RenderedFrame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RenderedFrame")
            .field("snapshot", &self.snapshot)
            .field("surface", &self.surface)
            .field("descriptor", &self.descriptor)
            .finish_non_exhaustive()
    }
}

struct RenderTarget {
    texture: wgpu::Texture,
}

/// Tightly-packed CPU pixels produced only by an explicit offscreen readback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OffscreenReadback {
    surface: PreviewSurface,
    pixels: Vec<u8>,
}

impl OffscreenReadback {
    pub const fn surface(&self) -> PreviewSurface {
        self.surface
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

fn align_to_copy_row(bytes_per_row: u32) -> Result<u32, RenderError> {
    const ALIGNMENT: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let aligned = bytes_per_row
        .checked_add(ALIGNMENT - 1)
        .ok_or(RenderError::ReadbackSizeOverflow)?
        / ALIGNMENT
        * ALIGNMENT;
    Ok(aligned)
}

#[derive(Debug)]
pub enum RenderError {
    NoCompiledBackend,
    AdapterUnavailable { message: String },
    DeviceUnavailable { message: String },
    ViewerSurfaceNotImplemented,
    InvalidSourceFrame,
    InvalidVideoSettings,
    SourceSizeExceedsDeviceLimit { width: u32, height: u32, limit: u32 },
    SizeExceedsDeviceLimit { size: RenderSize, limit: u32 },
    ReadbackSizeOverflow,
    DevicePoll { message: String },
    ReadbackMap { message: String },
    FrameDescriptor { message: String },
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCompiledBackend => {
                formatter.write_str("no wgpu backend is compiled for this target")
            }
            Self::AdapterUnavailable { message } => {
                write!(formatter, "no usable wgpu adapter: {message}")
            }
            Self::DeviceUnavailable { message } => {
                write!(formatter, "could not create wgpu device: {message}")
            }
            Self::ViewerSurfaceNotImplemented => formatter
                .write_str("viewer surface rendering is reserved for the viewer checkpoint"),
            Self::InvalidSourceFrame => {
                formatter.write_str("viewer source frame has an invalid size or pixel buffer")
            }
            Self::InvalidVideoSettings => formatter
                .write_str("video transform, crop, or opacity is outside its supported range"),
            Self::SourceSizeExceedsDeviceLimit {
                width,
                height,
                limit,
            } => write!(
                formatter,
                "viewer source size {width}x{height} exceeds the device texture limit {limit}"
            ),
            Self::SizeExceedsDeviceLimit { size, limit } => write!(
                formatter,
                "render size {}x{} exceeds the device texture limit {limit}",
                size.width(),
                size.height()
            ),
            Self::ReadbackSizeOverflow => formatter.write_str("render readback size overflowed"),
            Self::DevicePoll { message } => write!(formatter, "wgpu device poll failed: {message}"),
            Self::ReadbackMap { message } => {
                write!(formatter, "wgpu readback mapping failed: {message}")
            }
            Self::FrameDescriptor { message } => {
                write!(formatter, "could not describe rendered frame: {message}")
            }
        }
    }
}

impl Error for RenderError {}

#[cfg(test)]
mod tests {
    use super::*;
    use or_core::{Crop, Opacity, ProjectDocument, RationalTime, TimeRange, Transform};
    use std::{
        future::Future,
        pin::Pin,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
        thread,
        time::Duration,
    };

    fn time(numerator: i64, denominator: u32) -> RationalTime {
        RationalTime::new(numerator, denominator).unwrap()
    }

    fn snapshot() -> RenderSnapshot {
        let project = ProjectDocument::new("synthetic");
        RenderSnapshot::new(
            project.id(),
            project.revision(),
            TimeRange::new(time(1, 24), time(1, 24)).unwrap(),
        )
    }

    fn render_size(width: u32, height: u32) -> RenderSize {
        RenderSize::new(width, height).unwrap()
    }

    #[test]
    fn render_graph_inputs_and_viewer_contract_are_handle_only() {
        let size = render_size(3, 2);
        let handle = ViewerSurfaceHandle::new(7);
        let contract = ViewerSurfaceContract::new(handle, size, RenderFormat::Rgba8Unorm);
        assert_eq!(contract.handle().value(), 7);
        assert_eq!(
            contract.surface().target(),
            PreviewSurfaceTarget::Viewer(handle)
        );

        let input = RenderGraphInput::new(
            snapshot(),
            PreviewSurface::offscreen(size, RenderFormat::Rgba8Unorm),
            SyntheticScene::solid(SyntheticColor::GREEN),
        );
        assert_eq!(input.surface().size(), size);
        assert_eq!(input.scene().clear_color(), SyntheticColor::GREEN);
        assert_eq!(input.snapshot().requested_range().start(), time(1, 24));
    }

    #[test]
    fn copy_row_alignment_is_deterministic() {
        assert_eq!(align_to_copy_row(4).unwrap(), 256);
        assert_eq!(align_to_copy_row(256).unwrap(), 256);
        assert_eq!(align_to_copy_row(257).unwrap(), 512);
    }

    #[test]
    fn viewer_render_composites_video_pixels_through_wgpu_when_adapter_is_available() {
        let layer_pixels = [255, 16, 8, 255].repeat(4);
        let layer = RgbaVideoLayer::new(2, 2, &layer_pixels).unwrap();
        let renderer = match block_on(RenderDevice::new()) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping wgpu viewer test: {error}");
                return;
            }
        };

        let rendered = renderer
            .render_rgba_layers(snapshot(), render_size(2, 2), &[layer])
            .unwrap();

        assert_eq!(rendered.pixels(), &layer_pixels);
    }

    #[test]
    fn viewer_render_applies_crop_opacity_and_transform_when_adapter_is_available() {
        let renderer = match block_on(RenderDevice::new()) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping wgpu visual settings test: {error}");
                return;
            }
        };

        let split_pixels = [
            255, 0, 0, 255, 0, 255, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255,
        ];
        let cropped_layer = RgbaVideoLayer::new(2, 2, &split_pixels)
            .unwrap()
            .with_visual_settings(
                Transform::IDENTITY,
                Crop {
                    left_basis_points: 5_000,
                    ..Crop::NONE
                },
                Opacity {
                    basis_points: 5_000,
                },
            )
            .unwrap();
        let cropped = renderer
            .render_rgba_layers(snapshot(), render_size(2, 2), &[cropped_layer])
            .unwrap();
        let first = &cropped.pixels()[..4];
        let second = &cropped.pixels()[4..8];
        assert!(first[0].abs_diff(32) <= 2, "first pixel: {first:?}");
        assert!(first[1].abs_diff(96) <= 2, "first pixel: {first:?}");
        assert!(second[0] <= 1, "second pixel: {second:?}");
        assert!(second[1].abs_diff(128) <= 2, "second pixel: {second:?}");
        assert_eq!(first[3], 255);
        assert_eq!(second[3], 255);

        let rotated_layer = RgbaVideoLayer::new(2, 2, &split_pixels)
            .unwrap()
            .with_visual_settings(
                Transform {
                    rotation_milli_degrees: 180_000,
                    ..Transform::IDENTITY
                },
                Crop::NONE,
                Opacity::OPAQUE,
            )
            .unwrap();
        let rotated = renderer
            .render_rgba_layers(snapshot(), render_size(2, 2), &[rotated_layer])
            .unwrap();
        assert_eq!(&rotated.pixels()[..4], &[0, 255, 0, 255]);
        assert_eq!(&rotated.pixels()[4..8], &[255, 0, 0, 255]);

        let solid_pixels = [255, 0, 0, 255].repeat(4);
        let translated_layer = RgbaVideoLayer::new(2, 2, &solid_pixels)
            .unwrap()
            .with_visual_settings(
                Transform {
                    x_milli_canvas: 500,
                    ..Transform::IDENTITY
                },
                Crop::NONE,
                Opacity::OPAQUE,
            )
            .unwrap();
        let translated = renderer
            .render_rgba_layers(snapshot(), render_size(2, 2), &[translated_layer])
            .unwrap();
        assert_eq!(&translated.pixels()[..4], &[0, 0, 0, 255]);
        assert_eq!(&translated.pixels()[4..8], &[255, 0, 0, 255]);

        let scaled_layer = RgbaVideoLayer::new(2, 2, &solid_pixels)
            .unwrap()
            .with_visual_settings(
                Transform {
                    scale_x_milli: 500,
                    ..Transform::IDENTITY
                },
                Crop::NONE,
                Opacity::OPAQUE,
            )
            .unwrap();
        let scaled = renderer
            .render_rgba_layers(snapshot(), render_size(4, 2), &[scaled_layer])
            .unwrap();
        assert_eq!(&scaled.pixels()[..4], &[0, 0, 0, 255]);
        assert_eq!(&scaled.pixels()[4..8], &[255, 0, 0, 255]);

        let anchored_layer = RgbaVideoLayer::new(2, 2, &solid_pixels)
            .unwrap()
            .with_visual_settings(
                Transform {
                    scale_x_milli: 500,
                    anchor_x_basis_points: 0,
                    ..Transform::IDENTITY
                },
                Crop::NONE,
                Opacity::OPAQUE,
            )
            .unwrap();
        let anchored = renderer
            .render_rgba_layers(snapshot(), render_size(4, 2), &[anchored_layer])
            .unwrap();
        assert_eq!(&anchored.pixels()[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn viewer_render_rejects_invalid_rgba_layers_before_gpu_use() {
        assert!(matches!(
            RgbaVideoLayer::new(2, 2, &[0; 15]),
            Err(RenderError::InvalidSourceFrame)
        ));
        let pixels = [0; 16];
        assert!(matches!(
            RgbaVideoLayer::new(2, 2, &pixels)
                .unwrap()
                .with_visual_settings(
                    Transform {
                        scale_x_milli: 0,
                        ..Transform::IDENTITY
                    },
                    Crop::NONE,
                    Opacity::OPAQUE,
                ),
            Err(RenderError::InvalidVideoSettings)
        ));
        assert!(matches!(
            RgbaVideoLayer::new(2, 2, &pixels)
                .unwrap()
                .with_visual_settings(
                    Transform::IDENTITY,
                    Crop {
                        left_basis_points: 5_000,
                        right_basis_points: 5_000,
                        ..Crop::NONE
                    },
                    Opacity::OPAQUE,
                ),
            Err(RenderError::InvalidVideoSettings)
        ));
        assert!(matches!(
            RgbaVideoLayer::new(2, 2, &pixels)
                .unwrap()
                .with_visual_settings(
                    Transform::IDENTITY,
                    Crop::NONE,
                    Opacity {
                        basis_points: 10_001
                    },
                ),
            Err(RenderError::InvalidVideoSettings)
        ));
    }

    #[test]
    fn offscreen_render_readback_is_deterministic_when_adapter_is_available() {
        let renderer = match block_on(RenderDevice::new()) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping wgpu readback test: {error}");
                return;
            }
        };
        let size = render_size(3, 2);
        let input = RenderGraphInput::new(
            snapshot(),
            PreviewSurface::offscreen(size, RenderFormat::Rgba8Unorm),
            SyntheticScene::solid(SyntheticColor::GREEN),
        );
        let first = renderer.render(input).unwrap();
        let first_readback = renderer.readback(&first).unwrap();
        let second = renderer.render(input).unwrap();
        let second_readback = renderer.readback(&second).unwrap();

        assert_eq!(first_readback, second_readback);
        assert_eq!(first_readback.pixels().len(), 3 * 2 * 4);
        for pixel in first_readback.pixels().chunks_exact(4) {
            assert_eq!(pixel, SyntheticColor::GREEN.channels());
        }
        assert_eq!(
            first.descriptor().memory_domain(),
            FrameMemoryDomain::HardwareSurface
        );
        assert_eq!(first.descriptor().width(), 3);
        assert_eq!(first.descriptor().height(), 2);
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
        let waker: Waker = Waker::from(Arc::new(ThreadWaker(current.clone())));
        let mut context = Context::from_waker(&waker);
        loop {
            match Pin::as_mut(&mut future).poll(&mut context) {
                Poll::Ready(value) => return value,
                Poll::Pending => thread::park_timeout(Duration::from_millis(1)),
            }
        }
    }
}
