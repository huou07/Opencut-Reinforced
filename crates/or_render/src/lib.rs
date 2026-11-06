//! The shared wgpu render spine for the runtime plane.
//!
//! Phase 7B established a platform-neutral preview-surface contract,
//! deterministic synthetic render-graph inputs, an offscreen wgpu target, and
//! an explicit CPU readback path. Phase 7F adds GPU composition of decoded
//! software video layers. Native adapters own viewer presentation; full-rate
//! frame bytes remain outside the Flutter boundary.

use or_core::{Crop, EffectReference, Opacity, Transform, TransitionKind};
use or_runtime::{
    FrameAccess, FrameColorInfo, FrameDescriptor, FrameMemoryDomain, FramePixelFormat, FrameTiming,
    RenderSnapshot,
};
use std::{
    error::Error,
    fmt,
    sync::{Mutex, mpsc},
};
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
    nv12_bind_group_layout: wgpu::BindGroupLayout,
    nv12_luma_pipeline: wgpu::RenderPipeline,
    nv12_chroma_pipeline: wgpu::RenderPipeline,
    nv12_layer_bind_group_layout: wgpu::BindGroupLayout,
    nv12_layer_pipeline: wgpu::RenderPipeline,
    /// Reusable NV12 targets and readback buffer for the last picture size.
    ///
    /// An export renders hundreds of frames at one size. Creating two textures
    /// and one staging buffer per frame, then waiting on the GPU twice to read
    /// them back, spent more time in bookkeeping than in the colour conversion.
    /// One entry keyed by size keeps a delivery export's steady state to a
    /// single submit and a single wait per frame.
    nv12_scratch: Mutex<Option<Nv12Scratch>>,
    /// Compositor targets and source textures reused across frames.
    ///
    /// A frame used to allocate a fresh target, one texture per layer and a
    /// uniform buffer per layer, then drop them. The pool keeps the steady state
    /// of an export to uploads, one submit and one readback. Entries are
    /// checked out of the pool, so concurrent preview and export renders never
    /// share a texture.
    compositor_pool: Mutex<CompositorPool>,
    adapter_info: RenderAdapterInfo,
    limits: wgpu::Limits,
}

/// The shader that turns a composited RGBA target into NV12.
///
/// BT.709 primaries, limited (MPEG) range, which is what the hardware encoder
/// and the previous swscale path both expect. Luma is written per pixel; chroma
/// is written once per 2x2 block by averaging the four source texels, because
/// NV12 interleaves one chroma pair per block.
const NV12_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let point = positions[index];
    var output: VertexOutput;
    output.position = vec4<f32>(point, 0.0, 1.0);
    output.uv = vec2<f32>((point.x + 1.0) * 0.5, (1.0 - point.y) * 0.5);
    return output;
}

fn bt709_luma(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}

@fragment
fn fs_luma(input: VertexOutput) -> @location(0) vec4<f32> {
    let rgb = textureSample(source, source_sampler, input.uv).rgb;
    let luma = (16.0 + 219.0 * bt709_luma(rgb)) / 255.0;
    return vec4<f32>(luma, 0.0, 0.0, 1.0);
}

@fragment
fn fs_chroma(input: VertexOutput) -> @location(0) vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(source, 0));
    let texel = vec2<f32>(1.0) / dimensions;
    let corner = input.uv - texel * 0.5;
    let rgb = (
        textureSample(source, source_sampler, corner).rgb
        + textureSample(source, source_sampler, corner + vec2<f32>(texel.x, 0.0)).rgb
        + textureSample(source, source_sampler, corner + vec2<f32>(0.0, texel.y)).rgb
        + textureSample(source, source_sampler, corner + texel).rgb
    ) * 0.25;
    let luma = bt709_luma(rgb);
    let u = (128.0 + 224.0 * (rgb.b - luma) / 1.8556) / 255.0;
    let v = (128.0 + 224.0 * (rgb.r - luma) / 1.5748) / 255.0;
    return vec4<f32>(u, v, 0.0, 1.0);
}
"#;

/// Converts an NV12 source layer to RGB while it is composited.
///
/// Decoding straight to NV12 and converting in the layer shader keeps the
/// software decode out of RGBA: a covering 1080p clip uploads 3.1 MB of planes
/// instead of 8.3 MB of pixels, and the encoder receives the composition in the
/// same BT.709 limited-range form it already expects.
/// Composites one NV12 layer the same way the RGBA layer shader composites
/// pixels, so a decoded video plane stack can join the text composition without
/// an RGBA decode on the CPU.
///
/// The decode side uses the BT.601 limited-range matrix. Frames carry no colour
/// metadata (`FrameColorInfo::default()` is unknown), and the software decoder
/// converts with libswscale's default matrix, which is BT.601. Matching it keeps
/// an export's picture identical to the pre-NV12 path instead of shifting every
/// saturated colour; the encode side converts back with BT.709 and tags it, as it
/// already did. A source that spells out BT.709 in its container needs the
/// metadata plumbed through before this inverse can follow it.
const NV12_LAYER_SHADER: &str = r#"
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

@group(0) @binding(0) var luma: texture_2d<f32>;
@group(0) @binding(1) var chroma: texture_2d<f32>;
@group(0) @binding(2) var source_sampler: sampler;
@group(0) @binding(3) var<uniform> settings: LayerSettings;

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
    let luma_sample = textureSample(luma, source_sampler, source_uv).r;
    let chroma_sample = textureSample(chroma, source_sampler, source_uv).rg;
    let y = (luma_sample * 255.0 - 16.0) / 219.0;
    let u = (chroma_sample.x * 255.0 - 128.0) / 224.0;
    let v = (chroma_sample.y * 255.0 - 128.0) / 224.0;
    let rgb = vec3<f32>(
        y + 1.4020 * v,
        y - 0.3441 * u - 0.7141 * v,
        y + 1.7720 * u,
    );
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), settings.opacity.x);
}
"#;

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
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(limits.clone()),
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

        let nv12_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("or_render.nv12_layout"),
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
                ],
            });
        let nv12_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("or_render.nv12_shader"),
            source: wgpu::ShaderSource::Wgsl(NV12_SHADER.into()),
        });
        let nv12_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("or_render.nv12_pipeline_layout"),
            bind_group_layouts: &[&nv12_bind_group_layout],
            push_constant_ranges: &[],
        });
        let nv12_pipeline =
            |label: &str, entry_point: &str, format: wgpu::TextureFormat| -> wgpu::RenderPipeline {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(&nv12_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &nv12_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &nv12_shader,
                        entry_point: Some(entry_point),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                })
            };
        let nv12_luma_pipeline = nv12_pipeline(
            "or_render.nv12_luma_pipeline",
            "fs_luma",
            wgpu::TextureFormat::R8Unorm,
        );
        let nv12_chroma_pipeline = nv12_pipeline(
            "or_render.nv12_chroma_pipeline",
            "fs_chroma",
            wgpu::TextureFormat::Rg8Unorm,
        );

        let nv12_layer_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("or_render.nv12_layer_layout"),
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
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
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
        let nv12_layer_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("or_render.nv12_layer_shader"),
            source: wgpu::ShaderSource::Wgsl(NV12_LAYER_SHADER.into()),
        });
        let nv12_layer_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("or_render.nv12_layer_pipeline_layout"),
                bind_group_layouts: &[&nv12_layer_bind_group_layout],
                push_constant_ranges: &[],
            });
        let nv12_layer_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("or_render.nv12_layer_pipeline"),
            layout: Some(&nv12_layer_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &nv12_layer_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &nv12_layer_shader,
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

        Ok(Self {
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            video_bind_group_layout,
            video_pipeline,
            video_sampler,
            nv12_bind_group_layout,
            nv12_luma_pipeline,
            nv12_chroma_pipeline,
            nv12_layer_bind_group_layout,
            nv12_layer_pipeline,
            nv12_scratch: Mutex::new(None),
            compositor_pool: Mutex::new(CompositorPool::default()),
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
        let pixels = self.readback_texture(&frame.target.texture, frame.surface.size())?;
        Ok(OffscreenReadback {
            surface: frame.surface,
            pixels,
        })
    }

    /// Copies one offscreen texture into a tightly-packed CPU buffer.
    fn readback_texture(
        &self,
        texture: &wgpu::Texture,
        size: RenderSize,
    ) -> Result<Vec<u8>, RenderError> {
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
                texture,
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
        Ok(pixels)
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

    /// Composites the same layers and returns an NV12 picture instead of RGBA.
    ///
    /// The conversion happens on the GPU, so an export reads back 3.1 MB of
    /// planes at 1080p instead of 8.3 MB of RGBA and then converts on the CPU.
    /// The pixels are the same composition: one renderer, one graph, two output
    /// layouts.
    pub fn render_rgba_layers_nv12(
        &self,
        snapshot: RenderSnapshot,
        size: RenderSize,
        layers: &[RgbaVideoLayer<'_>],
    ) -> Result<Nv12Planes, RenderError> {
        let frame = RenderGraph.render_rgba_layers(self, snapshot, size, layers)?;
        self.readback_nv12(&frame)
    }

    /// Converts an offscreen target into NV12 and reads back both planes.
    pub fn readback_nv12(&self, frame: &RenderedFrame) -> Result<Nv12Planes, RenderError> {
        if frame.surface.target() != PreviewSurfaceTarget::Offscreen {
            return Err(RenderError::ViewerSurfaceNotImplemented);
        }
        self.readback_texture_nv12(&frame.target.texture, frame.surface.size())
    }

    /// Converts one offscreen RGBA texture into NV12 and reads back both planes.
    fn readback_texture_nv12(
        &self,
        texture: &wgpu::Texture,
        size: RenderSize,
    ) -> Result<Nv12Planes, RenderError> {
        let width = size.width();
        let height = size.height();
        if !width.is_multiple_of(2) || !height.is_multiple_of(2) {
            return Err(RenderError::OddSizeForNv12 { size });
        }
        let (chroma_width, chroma_height) = (width / 2, height / 2);

        let source_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("or_render.nv12_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            // Linear is required by the filtering slot this layout declares. Every
            // sample below lands on a texel centre, so the weights collapse and the
            // reading stays exact: one texel per output pixel, four per chroma pair.
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("or_render.nv12_bind_group"),
            layout: &self.nv12_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let mut slot = self
            .nv12_scratch
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot.as_ref().is_none_or(|scratch| scratch.size != size) {
            // A size change is a new picture, not a new export: replacing the
            // entry is cheaper than carrying a pool nobody asked for.
            *slot = Some(Nv12Scratch::new(&self.device, size)?);
        }
        let scratch = slot.as_ref().expect("the NV12 scratch was just built");

        // One encoder, one submit, one wait. Each of the two planes used to cost
        // its own submit and its own device poll, which put the render thread to
        // sleep twice per frame for work the GPU had already finished.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("or_render.nv12_encoder"),
            });
        for (pipeline, view) in [
            (&self.nv12_luma_pipeline, &scratch.luma_view),
            (&self.nv12_chroma_pipeline, &scratch.chroma_view),
        ] {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("or_render.nv12_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &scratch.luma,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &scratch.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(scratch.luma_row_bytes),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &scratch.chroma,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &scratch.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: scratch.chroma_offset,
                    bytes_per_row: Some(scratch.chroma_row_bytes),
                    rows_per_image: Some(chroma_height),
                },
            },
            wgpu::Extent3d {
                width: chroma_width,
                height: chroma_height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = scratch.buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
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

        let (luma, chroma) = {
            let mapped = slice.get_mapped_range();
            let luma = repack_plane(
                &mapped,
                scratch.luma_row_bytes,
                usize::try_from(width).map_err(|_| RenderError::ReadbackSizeOverflow)?,
                usize::try_from(height).map_err(|_| RenderError::ReadbackSizeOverflow)?,
            )?;
            let chroma = repack_plane(
                &mapped[usize::try_from(scratch.chroma_offset)
                    .map_err(|_| RenderError::ReadbackSizeOverflow)?..],
                scratch.chroma_row_bytes,
                usize::try_from(chroma_width).map_err(|_| RenderError::ReadbackSizeOverflow)? * 2,
                usize::try_from(chroma_height).map_err(|_| RenderError::ReadbackSizeOverflow)?,
            )?;
            (luma, chroma)
        };
        // Unmapping is a property of the buffer, not of this frame's slice, and
        // the next readback maps the same range again.
        scratch.buffer.unmap();
        Ok(Nv12Planes {
            width,
            height,
            luma: PlaneBytes::Owned(luma),
            chroma: PlaneBytes::Owned(chroma),
        })
    }

    /// Composites a mixed RGBA/NV12 layer stack and reads the result back as
    /// RGBA. Targets and source textures come from the pool, so a long export
    /// stops allocating a texture per layer per frame.
    pub fn composite_layers(
        &self,
        size: RenderSize,
        layers: &[CompositorLayer<'_>],
    ) -> Result<OffscreenReadback, RenderError> {
        let target = self.checkout_target(size)?;
        let pixels = self
            .compose_into(size, layers, &target.view)
            .and_then(|()| self.readback_texture(&target.texture, size));
        self.return_target(target);
        Ok(OffscreenReadback {
            surface: PreviewSurface::offscreen(size, RenderFormat::Rgba8Unorm),
            pixels: pixels?,
        })
    }

    /// Composites the same stack and reads it back as NV12 planes.
    pub fn composite_layers_nv12(
        &self,
        size: RenderSize,
        layers: &[CompositorLayer<'_>],
    ) -> Result<Nv12Planes, RenderError> {
        let target = self.checkout_target(size)?;
        let planes = self
            .compose_into(size, layers, &target.view)
            .and_then(|()| self.readback_texture_nv12(&target.texture, size));
        self.return_target(target);
        planes
    }

    /// Renders `layers` into a freshly allocated target owned by the frame.
    fn compose_layers_into_frame(
        &self,
        snapshot: RenderSnapshot,
        size: RenderSize,
        layers: &[CompositorLayer<'_>],
    ) -> Result<RenderedFrame, RenderError> {
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
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
            // TEXTURE_BINDING so an NV12 export can sample the composited target
            // for its colour conversion instead of reading it back to the CPU.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        self.compose_into(size, layers, &view)?;
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

    /// Draws every layer into `target_view`: one pass, one submit.
    fn compose_into(
        &self,
        size: RenderSize,
        layers: &[CompositorLayer<'_>],
        target_view: &wgpu::TextureView,
    ) -> Result<(), RenderError> {
        if size.width() > self.limits.max_texture_dimension_2d
            || size.height() > self.limits.max_texture_dimension_2d
        {
            return Err(RenderError::SizeExceedsDeviceLimit {
                size,
                limit: self.limits.max_texture_dimension_2d,
            });
        }

        let mut sources: Vec<SourceTexture> = Vec::with_capacity(layers.len() * 2);
        let mut uniforms: Vec<wgpu::Buffer> = Vec::with_capacity(layers.len());
        let mut draws: Vec<(bool, wgpu::BindGroup, Option<LayerScissor>)> =
            Vec::with_capacity(layers.len());
        let outcome = (|| -> Result<(), RenderError> {
            for layer in layers {
                layer.validate()?;
                let (width, height) = layer.source_size();
                if width > self.limits.max_texture_dimension_2d
                    || height > self.limits.max_texture_dimension_2d
                {
                    return Err(RenderError::SourceSizeExceedsDeviceLimit {
                        width,
                        height,
                        limit: self.limits.max_texture_dimension_2d,
                    });
                }
                let uniform = self
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("or_render.viewer_layer_settings"),
                        contents: &layer.settings_uniform(size),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                let (nv12, bind_group) = match *layer {
                    CompositorLayer::Rgba(rgba) => {
                        let mut source = self.checkout_source(
                            rgba.width,
                            rgba.height,
                            wgpu::TextureFormat::Rgba8Unorm,
                            rgba.cache_key,
                        );
                        if rgba.cache_key.is_none() || source.cache_key != rgba.cache_key {
                            self.queue.write_texture(
                                source.texture.as_image_copy(),
                                rgba.pixels,
                                wgpu::TexelCopyBufferLayout {
                                    offset: 0,
                                    bytes_per_row: Some(rgba.width * 4),
                                    rows_per_image: Some(rgba.height),
                                },
                                wgpu::Extent3d {
                                    width: rgba.width,
                                    height: rgba.height,
                                    depth_or_array_layers: 1,
                                },
                            );
                        }
                        source.cache_key = rgba.cache_key;
                        let bind_group = self.create_layer_bind_group(
                            &self.video_bind_group_layout,
                            &source.view,
                            &uniform,
                        );
                        sources.push(source);
                        (false, bind_group)
                    }
                    CompositorLayer::Nv12(nv12) => {
                        let (chroma_width, chroma_height) = (nv12.width / 2, nv12.height / 2);
                        let mut luma = self.checkout_source(
                            nv12.width,
                            nv12.height,
                            wgpu::TextureFormat::R8Unorm,
                            None,
                        );
                        let mut chroma = self.checkout_source(
                            chroma_width,
                            chroma_height,
                            wgpu::TextureFormat::Rg8Unorm,
                            None,
                        );
                        self.queue.write_texture(
                            luma.texture.as_image_copy(),
                            nv12.luma,
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(nv12.width),
                                rows_per_image: Some(nv12.height),
                            },
                            wgpu::Extent3d {
                                width: nv12.width,
                                height: nv12.height,
                                depth_or_array_layers: 1,
                            },
                        );
                        self.queue.write_texture(
                            chroma.texture.as_image_copy(),
                            nv12.chroma,
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(chroma_width * 2),
                                rows_per_image: Some(chroma_height),
                            },
                            wgpu::Extent3d {
                                width: chroma_width,
                                height: chroma_height,
                                depth_or_array_layers: 1,
                            },
                        );
                        luma.cache_key = None;
                        chroma.cache_key = None;
                        let bind_group =
                            self.create_nv12_layer_bind_group(&luma.view, &chroma.view, &uniform);
                        sources.push(luma);
                        sources.push(chroma);
                        (true, bind_group)
                    }
                };
                uniforms.push(uniform);
                let scissor = match *layer {
                    CompositorLayer::Rgba(rgba) => rgba.scissor,
                    CompositorLayer::Nv12(_) => None,
                };
                draws.push((nv12, bind_group, scissor));
            }

            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(SyntheticColor::BLACK.wgpu()),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut encoder = self
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
                for (nv12, bind_group, scissor) in &draws {
                    pass.set_pipeline(if *nv12 {
                        &self.nv12_layer_pipeline
                    } else {
                        &self.video_pipeline
                    });
                    pass.set_bind_group(0, bind_group, &[]);
                    let (x, y, width, height) = scissor
                        .map(|(x, y, width, height)| {
                            (
                                x.min(size.width()),
                                y.min(size.height()),
                                width.min(size.width().saturating_sub(x)),
                                height.min(size.height().saturating_sub(y)),
                            )
                        })
                        .unwrap_or((0, 0, size.width(), size.height()));
                    if width == 0 || height == 0 {
                        continue;
                    }
                    pass.set_scissor_rect(x, y, width, height);
                    pass.draw(0..3, 0..1);
                }
            }
            self.queue.submit(Some(encoder.finish()));
            Ok(())
        })();
        for source in sources.drain(..) {
            self.return_source(source);
        }
        outcome
    }

    fn create_layer_bind_group(
        &self,
        layout: &wgpu::BindGroupLayout,
        source: &wgpu::TextureView,
        uniform: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("or_render.viewer_layer"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.video_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform.as_entire_binding(),
                },
            ],
        })
    }

    fn create_nv12_layer_bind_group(
        &self,
        luma: &wgpu::TextureView,
        chroma: &wgpu::TextureView,
        uniform: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("or_render.nv12_layer"),
            layout: &self.nv12_layer_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(luma),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(chroma),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.video_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: uniform.as_entire_binding(),
                },
            ],
        })
    }

    fn checkout_target(&self, size: RenderSize) -> Result<TargetTexture, RenderError> {
        {
            let mut pool = self
                .compositor_pool
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(index) = pool
                .targets
                .iter()
                .position(|target| target.width == size.width() && target.height == size.height())
            {
                return Ok(pool.targets.remove(index));
            }
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("or_render.compositor_target"),
            size: wgpu::Extent3d {
                width: size.width(),
                height: size.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(TargetTexture {
            width: size.width(),
            height: size.height(),
            texture,
            view,
        })
    }

    fn return_target(&self, target: TargetTexture) {
        let mut pool = self
            .compositor_pool
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if pool.targets.len() < CompositorPool::MAX_TARGETS {
            pool.targets.push(target);
        }
    }

    /// Takes a matching texture, preferring one that already holds `cache_key`
    /// so a static layer keeps its upload, and one that holds nothing so an
    /// unkeyed layer does not evict it.
    fn checkout_source(
        &self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        cache_key: Option<u64>,
    ) -> SourceTexture {
        {
            let mut pool = self
                .compositor_pool
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let matching = |source: &SourceTexture| {
                source.width == width && source.height == height && source.format == format
            };
            if let Some(key) = cache_key
                && let Some(index) = pool
                    .sources
                    .iter()
                    .position(|source| matching(source) && source.cache_key == Some(key))
            {
                return pool.sources.remove(index);
            }
            if let Some(index) = pool
                .sources
                .iter()
                .position(|source| matching(source) && source.cache_key.is_none())
            {
                return pool.sources.remove(index);
            }
            if let Some(index) = pool.sources.iter().position(matching) {
                return pool.sources.remove(index);
            }
        }
        SourceTexture::new(&self.device, width, height, format)
    }

    fn return_source(&self, source: SourceTexture) {
        let mut pool = self
            .compositor_pool
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if pool.sources.len() < CompositorPool::MAX_SOURCES {
            pool.sources.push(source);
        }
    }
}

/// The reusable NV12 render targets and readback buffer for one picture size.
///
/// Textures and the staging buffer outlive a frame: an export renders hundreds
/// of frames at one size, and re-creating three GPU allocations per frame cost
/// more than the colour conversion they served.
struct Nv12Scratch {
    size: RenderSize,
    luma: wgpu::Texture,
    luma_view: wgpu::TextureView,
    chroma: wgpu::Texture,
    chroma_view: wgpu::TextureView,
    /// Luma rows, padded to the copy row alignment the backend requires.
    luma_row_bytes: u32,
    /// Chroma rows, padded the same way.
    chroma_row_bytes: u32,
    /// Where the chroma plane starts in `buffer`; copy offsets must be aligned.
    chroma_offset: u64,
    buffer: wgpu::Buffer,
}

impl Nv12Scratch {
    fn new(device: &wgpu::Device, size: RenderSize) -> Result<Self, RenderError> {
        let width = size.width();
        let height = size.height();
        let (chroma_width, chroma_height) = (width / 2, height / 2);
        let luma_row_bytes = align_to_copy_row(
            width
                .checked_mul(1)
                .ok_or(RenderError::ReadbackSizeOverflow)?,
        )?;
        let chroma_row_bytes = align_to_copy_row(
            chroma_width
                .checked_mul(2)
                .ok_or(RenderError::ReadbackSizeOverflow)?,
        )?;
        let luma_bytes = u64::from(luma_row_bytes)
            .checked_mul(u64::from(height))
            .ok_or(RenderError::ReadbackSizeOverflow)?;
        let chroma_bytes = u64::from(chroma_row_bytes)
            .checked_mul(u64::from(chroma_height))
            .ok_or(RenderError::ReadbackSizeOverflow)?;
        // A texture-to-buffer copy may only start at a 256-byte boundary, so the
        // chroma plane begins on the next one after the luma plane.
        let chroma_offset = luma_bytes.div_ceil(256) * 256;
        let buffer_size = chroma_offset
            .checked_add(chroma_bytes)
            .ok_or(RenderError::ReadbackSizeOverflow)?;

        let luma = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("or_render.nv12_luma"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let chroma = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("or_render.nv12_chroma"),
            size: wgpu::Extent3d {
                width: chroma_width,
                height: chroma_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let luma_view = luma.create_view(&wgpu::TextureViewDescriptor::default());
        let chroma_view = chroma.create_view(&wgpu::TextureViewDescriptor::default());
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("or_render.nv12_readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Ok(Self {
            size,
            luma,
            luma_view,
            chroma,
            chroma_view,
            luma_row_bytes,
            chroma_row_bytes,
            chroma_offset,
            buffer,
        })
    }
}

/// Copies one padded plane out of a mapped readback buffer.
///
/// The backend aligns every copy row, so a tight plane is the leading
/// `usable_row_bytes` of each `row_bytes`-wide row.
fn repack_plane(
    mapped: &[u8],
    row_bytes: u32,
    usable_row_bytes: usize,
    rows: usize,
) -> Result<Vec<u8>, RenderError> {
    let row_bytes = usize::try_from(row_bytes).map_err(|_| RenderError::ReadbackSizeOverflow)?;
    let plane_bytes = usable_row_bytes
        .checked_mul(rows)
        .ok_or(RenderError::ReadbackSizeOverflow)?;
    let mut pixels = Vec::with_capacity(plane_bytes);
    for row in mapped.chunks_exact(row_bytes).take(rows) {
        pixels.extend_from_slice(&row[..usable_row_bytes]);
    }
    Ok(pixels)
}

/// One NV12 picture: tightly packed luma at full resolution and interleaved
/// chroma at half resolution.
///
/// This is the layout the hardware H.264 encoder takes, so an export can hand the
/// media engine planes it already has instead of a CPU colour conversion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Nv12Planes {
    width: u32,
    height: u32,
    luma: PlaneBytes,
    chroma: PlaneBytes,
}

/// One plane's storage: either bytes this crate owns, or a window into a buffer
/// the decoder still holds. The shared form lets a decoded frame reach the
/// encoder without a 3 MiB copy per frame at 1080p.
#[derive(Clone, Debug, Eq, PartialEq)]
enum PlaneBytes {
    Owned(Vec<u8>),
    Shared {
        bytes: std::sync::Arc<[u8]>,
        start: usize,
        end: usize,
    },
}

impl PlaneBytes {
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes,
            Self::Shared { bytes, start, end } => &bytes[*start..*end],
        }
    }
}

impl Nv12Planes {
    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Full-resolution luma, `width * height` bytes.
    pub fn luma(&self) -> &[u8] {
        self.luma.as_slice()
    }

    /// Half-resolution interleaved chroma, `(width / 2) * (height / 2) * 2` bytes.
    pub fn chroma(&self) -> &[u8] {
        self.chroma.as_slice()
    }

    /// Builds planes from one contiguous NV12 buffer: `width * height` luma
    /// bytes followed by `width * height / 2` interleaved chroma bytes.
    ///
    /// This is the layout the decoder produces so a CPU decode can reach the
    /// hardware encoder without an intermediate RGBA copy.
    pub fn from_contiguous(width: u32, height: u32, bytes: &[u8]) -> Option<Self> {
        let luma_len = nv12_luma_length(width, height)?;
        if bytes.len() != luma_len + luma_len / 2 {
            return None;
        }
        Some(Self {
            width,
            height,
            luma: PlaneBytes::Owned(bytes[..luma_len].to_vec()),
            chroma: PlaneBytes::Owned(bytes[luma_len..].to_vec()),
        })
    }

    /// Builds planes over a buffer the caller already owns, without copying it.
    ///
    /// The layout is the same as [`Self::from_contiguous`]; the buffer stays
    /// alive for as long as any plane of this picture is held.
    pub fn from_shared(width: u32, height: u32, bytes: std::sync::Arc<[u8]>) -> Option<Self> {
        let luma_len = nv12_luma_length(width, height)?;
        if bytes.len() != luma_len + luma_len / 2 {
            return None;
        }
        Some(Self {
            width,
            height,
            luma: PlaneBytes::Shared {
                bytes: std::sync::Arc::clone(&bytes),
                start: 0,
                end: luma_len,
            },
            chroma: PlaneBytes::Shared {
                bytes,
                start: luma_len,
                end: luma_len + luma_len / 2,
            },
        })
    }
}

/// Validates NV12 geometry and returns the luma plane length in bytes.
fn nv12_luma_length(width: u32, height: u32) -> Option<usize> {
    if width == 0 || height == 0 || !width.is_multiple_of(2) || !height.is_multiple_of(2) {
        return None;
    }
    (width as usize).checked_mul(height as usize)
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
            // TEXTURE_BINDING lets a later NV12 conversion sample this target
            // instead of reading it back to the CPU first.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
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
        let layers: Vec<CompositorLayer<'_>> =
            layers.iter().copied().map(CompositorLayer::Rgba).collect();
        renderer.compose_layers_into_frame(snapshot, size, &layers)
    }
}

/// One covering video layer decoded straight to NV12 planes.
///
/// A clip that fills the canvas at the output size loses nothing by being
/// decoded as NV12: the planes are already what the encoder wants, and the
/// layer shader converts to RGB while it composites. Partial-canvas clips stay
/// on [`RgbaVideoLayer`] so their source resolution is not thrown away.
#[derive(Clone, Copy, Debug)]
pub struct Nv12VideoLayer<'a> {
    width: u32,
    height: u32,
    luma: &'a [u8],
    chroma: &'a [u8],
    transform: Transform,
    crop: Crop,
    opacity: Opacity,
}

impl<'a> Nv12VideoLayer<'a> {
    pub fn new(
        width: u32,
        height: u32,
        luma: &'a [u8],
        chroma: &'a [u8],
    ) -> Result<Self, RenderError> {
        let layer = Self {
            width,
            height,
            luma,
            chroma,
            transform: Transform::IDENTITY,
            crop: Crop::NONE,
            opacity: Opacity::OPAQUE,
        };
        layer.validate()?;
        Ok(layer)
    }

    /// Splits one contiguous NV12 buffer: luma first, then interleaved chroma.
    pub fn from_contiguous(width: u32, height: u32, bytes: &'a [u8]) -> Result<Self, RenderError> {
        if !width.is_multiple_of(2) || !height.is_multiple_of(2) {
            return Err(RenderError::InvalidSourceFrame);
        }
        let luma_length = usize::try_from(
            width
                .checked_mul(height)
                .ok_or(RenderError::InvalidSourceFrame)?,
        )
        .map_err(|_| RenderError::InvalidSourceFrame)?;
        if bytes.len() < luma_length {
            return Err(RenderError::InvalidSourceFrame);
        }
        let (luma, chroma) = bytes.split_at(luma_length);
        Self::new(width, height, luma, chroma)
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

    fn validate(&self) -> Result<(), RenderError> {
        if !self.width.is_multiple_of(2) || !self.height.is_multiple_of(2) {
            return Err(RenderError::InvalidSourceFrame);
        }
        let luma_length = (self.width as usize) * (self.height as usize);
        if self.luma.len() != luma_length || self.chroma.len() != luma_length / 2 {
            return Err(RenderError::InvalidSourceFrame);
        }
        Ok(())
    }
}

/// One layer of a composition, whatever the source pixel layout is.
#[derive(Clone, Copy, Debug)]
pub enum CompositorLayer<'a> {
    Rgba(RgbaVideoLayer<'a>),
    Nv12(Nv12VideoLayer<'a>),
}

impl CompositorLayer<'_> {
    fn validate(self) -> Result<(), RenderError> {
        match self {
            Self::Rgba(layer) => layer.validate(),
            Self::Nv12(layer) => layer.validate(),
        }
    }

    fn settings_uniform(self, size: RenderSize) -> [u8; 64] {
        match self {
            Self::Rgba(layer) => {
                layer_settings_uniform(layer.transform, layer.crop, layer.opacity, size)
            }
            Self::Nv12(layer) => {
                layer_settings_uniform(layer.transform, layer.crop, layer.opacity, size)
            }
        }
    }

    fn source_size(self) -> (u32, u32) {
        match self {
            Self::Rgba(layer) => (layer.width, layer.height),
            Self::Nv12(layer) => (layer.width, layer.height),
        }
    }
}

/// A compositor target texture kept alive between frames of the same size.
struct TargetTexture {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// One uploaded layer texture, reused while its size, format and key agree.
struct SourceTexture {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    cache_key: Option<u64>,
}

impl SourceTexture {
    fn new(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("or_render.compositor_source"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            width,
            height,
            format,
            texture,
            view,
            cache_key: None,
        }
    }
}

/// Compositor resources reused across frames.
///
/// Entries are checked out, never lent: two concurrent renders get different
/// textures, and the pool only ever holds what those renders returned.
#[derive(Default)]
struct CompositorPool {
    targets: Vec<TargetTexture>,
    sources: Vec<SourceTexture>,
}

impl CompositorPool {
    /// Bounds the memory a stalled or many-layer export can pin.
    const MAX_TARGETS: usize = 2;
    const MAX_SOURCES: usize = 8;
}

/// A scissor rectangle in canvas pixels, `(x, y, width, height)`.
type LayerScissor = (u32, u32, u32, u32);

/// One software-decoded RGBA source layer passed to the render graph.
#[derive(Clone, Copy, Debug)]
pub struct RgbaVideoLayer<'a> {
    width: u32,
    height: u32,
    pixels: &'a [u8],
    transform: Transform,
    crop: Crop,
    opacity: Opacity,
    /// Identifies pixels the caller already knows are unchanged since the last
    /// frame, so the compositor can skip re-uploading them (a static title, for
    /// example, is 8 MB of RGBA at 1080p that would otherwise cross the bus
    /// every frame).
    cache_key: Option<u64>,
    /// The part of the canvas this layer can paint, in target pixels.
    ///
    /// A text canvas is mostly transparent, so scissoring the blend to the ink
    /// turns a full-frame blend into one over the glyph rectangle. `None` paints
    /// the whole layer, which is what a video layer needs.
    scissor: Option<LayerScissor>,
}

/// A typed transition stage applied to one RGBA layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VisualTransition {
    pub kind: TransitionKind,
    /// Visible progress in basis points, from 0 through 10,000.
    pub visibility_basis_points: u16,
    pub entering: bool,
}

/// Applies the closed Phase 8 visual-effect set and transitions to a bounded
/// RGBA frame before it enters the shared compositor.
pub fn process_visual_rgba(
    width: u32,
    height: u32,
    pixels: &[u8],
    effects: &[EffectReference],
    transitions: &[VisualTransition],
) -> Result<Vec<u8>, RenderError> {
    let length = rgba_length(width, height)?;
    if pixels.len() != length {
        return Err(RenderError::InvalidSourceFrame);
    }
    let mut output = allocate_rgba(length)?;
    output.copy_from_slice(pixels);
    let mut scratch = None;
    for effect in effects {
        let valid = match *effect {
            EffectReference::Brightness { amount_milli } => {
                (-1_000..=1_000).contains(&amount_milli)
            }
            EffectReference::Contrast { amount_milli }
            | EffectReference::Saturation { amount_milli } => amount_milli <= 4_000,
            EffectReference::GaussianBlur { radius_milli } => radius_milli <= 128_000,
        };
        if !valid {
            return Err(RenderError::InvalidVideoSettings);
        }
        match *effect {
            EffectReference::Brightness { amount_milli } => {
                let offset = f32::from(amount_milli) * (255.0 / 1_000.0);
                for pixel in output.as_chunks_mut::<4>().0.iter_mut() {
                    for channel in &mut pixel[..3] {
                        *channel = (f32::from(*channel) + offset).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
            EffectReference::Contrast { amount_milli } => {
                let factor = f32::from(amount_milli) / 1_000.0;
                for pixel in output.as_chunks_mut::<4>().0.iter_mut() {
                    for channel in &mut pixel[..3] {
                        *channel = ((f32::from(*channel) - 128.0) * factor + 128.0)
                            .round()
                            .clamp(0.0, 255.0) as u8;
                    }
                }
            }
            EffectReference::Saturation { amount_milli } => {
                let factor = f32::from(amount_milli) / 1_000.0;
                for pixel in output.as_chunks_mut::<4>().0.iter_mut() {
                    let red = f32::from(pixel[0]);
                    let green = f32::from(pixel[1]);
                    let blue = f32::from(pixel[2]);
                    let luma = red * 0.2126 + green * 0.7152 + blue * 0.0722;
                    for (channel, value) in pixel[..3].iter_mut().zip([red, green, blue]) {
                        *channel = (luma + (value - luma) * factor).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
            EffectReference::GaussianBlur { radius_milli } => {
                let radius = usize::try_from(radius_milli.div_ceil(1_000))
                    .map_err(|_| RenderError::InvalidVideoSettings)?;
                if radius != 0 {
                    if scratch.is_none() {
                        scratch = Some(allocate_rgba(length)?);
                    }
                    let scratch = scratch.as_mut().expect("blur scratch was initialized");
                    box_blur_axis(
                        &output,
                        scratch,
                        width as usize,
                        height as usize,
                        radius,
                        true,
                    )?;
                    box_blur_axis(
                        scratch,
                        &mut output,
                        width as usize,
                        height as usize,
                        radius,
                        false,
                    )?;
                }
            }
        }
    }
    let width = width as usize;
    let height = height as usize;
    for transition in transitions {
        if transition.visibility_basis_points > 10_000 {
            return Err(RenderError::InvalidVideoSettings);
        }
        match transition.kind {
            TransitionKind::CrossDissolve => {
                let alpha_scale = f32::from(transition.visibility_basis_points) / 10_000.0;
                for pixel in output.as_chunks_mut::<4>().0.iter_mut() {
                    pixel[3] = (f32::from(pixel[3]) * alpha_scale).round() as u8;
                }
            }
            TransitionKind::FadeThroughBlack => {
                let color_scale = f32::from(transition.visibility_basis_points) / 10_000.0;
                for pixel in output.as_chunks_mut::<4>().0.iter_mut() {
                    for channel in &mut pixel[..3] {
                        *channel = (f32::from(*channel) * color_scale).round() as u8;
                    }
                }
            }
            TransitionKind::Wipe => {
                let visible =
                    (transition.visibility_basis_points as u128 * width as u128 / 10_000) as usize;
                let wipe_start = if transition.entering {
                    0
                } else {
                    width.saturating_sub(visible)
                };
                let wipe_end = if transition.entering { visible } else { width };
                for row in 0..height {
                    for column in 0..width {
                        if column < wipe_start || column >= wipe_end {
                            let offset = (row * width + column) * 4;
                            output[offset + 3] = 0;
                        }
                    }
                }
            }
        }
    }
    Ok(output)
}

fn rgba_length(width: u32, height: u32) -> Result<usize, RenderError> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|length| width != 0 && height != 0 && *length <= 256 * 1024 * 1024)
        .ok_or(RenderError::InvalidSourceFrame)
}

fn allocate_rgba(length: usize) -> Result<Vec<u8>, RenderError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| RenderError::AllocationFailed)?;
    bytes.resize(length, 0);
    Ok(bytes)
}

fn box_blur_axis(
    source: &[u8],
    destination: &mut [u8],
    width: usize,
    height: usize,
    radius: usize,
    horizontal: bool,
) -> Result<(), RenderError> {
    let line_length = if horizontal { width } else { height };
    let line_count = if horizontal { height } else { width };
    let mut prefix = Vec::<[u64; 4]>::new();
    prefix
        .try_reserve_exact(line_length.saturating_add(1))
        .map_err(|_| RenderError::AllocationFailed)?;
    for line in 0..line_count {
        prefix.clear();
        prefix.push([0; 4]);
        for position in 0..line_length {
            let pixel_offset = if horizontal {
                (line * width + position) * 4
            } else {
                (position * width + line) * 4
            };
            let mut sum = *prefix.last().expect("prefix contains the initial zero");
            for channel in 0..4 {
                sum[channel] += u64::from(source[pixel_offset + channel]);
            }
            prefix.push(sum);
        }
        for position in 0..line_length {
            let left = position.saturating_sub(radius);
            let right = position.saturating_add(radius).min(line_length - 1);
            let count = (right - left + 1) as u64;
            let pixel_offset = if horizontal {
                (line * width + position) * 4
            } else {
                (position * width + line) * 4
            };
            for channel in 0..4 {
                let sum = prefix[right + 1][channel] - prefix[left][channel];
                destination[pixel_offset + channel] = ((sum + count / 2) / count) as u8;
            }
        }
    }
    Ok(())
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
            cache_key: None,
            scissor: None,
        };
        layer.validate()?;
        Ok(layer)
    }

    /// Restricts painting to `x, y, width, height` in target pixels.
    ///
    /// Only meaningful when the layer keeps the identity transform, because a
    /// moved or scaled layer paints outside the rectangle its canvas covers.
    #[must_use]
    pub const fn with_scissor(mut self, x: u32, y: u32, width: u32, height: u32) -> Self {
        self.scissor = Some((x, y, width, height));
        self
    }

    /// Marks these pixels as reusable while `key` stays unchanged.
    #[must_use]
    pub const fn with_cache_key(mut self, key: u64) -> Self {
        self.cache_key = Some(key);
        self
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

    fn validate(&self) -> Result<(), RenderError> {
        if self.pixels.len() != rgba_length(self.width, self.height)? {
            return Err(RenderError::InvalidSourceFrame);
        }
        Ok(())
    }
}

/// Packs the per-layer uniforms shared by the RGBA and NV12 pipelines.
fn layer_settings_uniform(
    transform: Transform,
    crop: Crop,
    opacity: Opacity,
    size: RenderSize,
) -> [u8; 64] {
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
    AllocationFailed,
    SourceSizeExceedsDeviceLimit { width: u32, height: u32, limit: u32 },
    SizeExceedsDeviceLimit { size: RenderSize, limit: u32 },
    ReadbackSizeOverflow,
    OddSizeForNv12 { size: RenderSize },
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
            Self::InvalidVideoSettings => {
                formatter.write_str("video settings are outside their supported range")
            }
            Self::AllocationFailed => {
                formatter.write_str("could not allocate bounded RGBA processing memory")
            }
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
            Self::OddSizeForNv12 { size } => write!(
                formatter,
                "NV12 needs even dimensions, got {}x{}",
                size.width(),
                size.height()
            ),
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
    fn viewer_render_accepts_2560x1440_sources_when_adapter_allows_them() {
        let renderer = match block_on(RenderDevice::new()) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping wgpu large-source test: {error}");
                return;
            }
        };
        if renderer.limits().max_texture_dimension_2d < 2560 {
            eprintln!("skipping wgpu large-source test: adapter limit is below 2560");
            return;
        }

        let source_pixels = vec![0_u8; 2560 * 1440 * 4];
        let layer = RgbaVideoLayer::new(2560, 1440, &source_pixels).unwrap();
        let rendered = renderer
            .render_rgba_layers(snapshot(), render_size(16, 9), &[layer])
            .unwrap();

        assert_eq!(rendered.pixels().len(), 16 * 9 * 4);
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
    fn typed_brightness_contrast_saturation_and_blur_are_deterministic() {
        let pixels = [20, 40, 60, 255, 0, 0, 0, 255, 0, 0, 0, 255];
        let bright = process_visual_rgba(
            3,
            1,
            &pixels,
            &[EffectReference::Brightness { amount_milli: 500 }],
            &[],
        )
        .unwrap();
        assert_eq!(&bright[..4], &[148, 168, 188, 255]);

        let contrast = process_visual_rgba(
            1,
            1,
            &[20, 40, 60, 255],
            &[EffectReference::Contrast { amount_milli: 0 }],
            &[],
        )
        .unwrap();
        assert_eq!(contrast, [128, 128, 128, 255]);

        let saturation = process_visual_rgba(
            1,
            1,
            &[255, 0, 0, 255],
            &[EffectReference::Saturation { amount_milli: 0 }],
            &[],
        )
        .unwrap();
        assert_eq!(saturation, [54, 54, 54, 255]);

        let blur = process_visual_rgba(
            3,
            1,
            &[255, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255],
            &[EffectReference::GaussianBlur {
                radius_milli: 1_000,
            }],
            &[],
        )
        .unwrap();
        assert_eq!([blur[0], blur[4], blur[8]], [128, 85, 0]);
    }

    #[test]
    fn transitions_apply_typed_opacity_black_fade_and_left_to_right_wipe() {
        let pixels = [200, 100, 50, 255, 100, 50, 25, 255];
        let dissolve = process_visual_rgba(
            2,
            1,
            &pixels,
            &[],
            &[VisualTransition {
                kind: TransitionKind::CrossDissolve,
                visibility_basis_points: 5_000,
                entering: true,
            }],
        )
        .unwrap();
        assert_eq!([dissolve[3], dissolve[7]], [128, 128]);

        let black = process_visual_rgba(
            1,
            1,
            &pixels[..4],
            &[],
            &[VisualTransition {
                kind: TransitionKind::FadeThroughBlack,
                visibility_basis_points: 5_000,
                entering: false,
            }],
        )
        .unwrap();
        assert_eq!(black, [100, 50, 25, 255]);

        let wipe = process_visual_rgba(
            2,
            1,
            &pixels,
            &[],
            &[VisualTransition {
                kind: TransitionKind::Wipe,
                visibility_basis_points: 5_000,
                entering: true,
            }],
        )
        .unwrap();
        assert_eq!([wipe[3], wipe[7]], [255, 0]);
    }

    #[test]
    fn nv12_readback_converts_bt709_limited_range_when_adapter_is_available() {
        let renderer = match block_on(RenderDevice::new()) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping wgpu nv12 test: {error}");
                return;
            }
        };
        // White is the fixed point of the limited-range mapping: luma 235,
        // chroma 128. A red field then exercises the BT.709 coefficients.
        for (color, luma, u, v) in [
            (
                SyntheticColor::rgba(255, 255, 255, 255),
                235_i32,
                128_i32,
                128_i32,
            ),
            (SyntheticColor::rgba(255, 0, 0, 255), 63, 102, 240),
        ] {
            let size = render_size(4, 4);
            let input = RenderGraphInput::new(
                snapshot(),
                PreviewSurface::offscreen(size, RenderFormat::Rgba8Unorm),
                SyntheticScene::solid(color),
            );
            let frame = renderer.render(input).unwrap();
            let planes = renderer.readback_nv12(&frame).unwrap();
            assert_eq!(planes.width(), 4);
            assert_eq!(planes.height(), 4);
            assert_eq!(planes.luma().len(), 16);
            assert_eq!(planes.chroma().len(), 8);
            for sample in planes.luma() {
                assert!(
                    (*sample as i32 - luma).abs() <= 2,
                    "luma {sample} for {color:?}, expected {luma}"
                );
            }
            for pair in planes.chroma().as_chunks::<2>().0 {
                assert!(
                    (pair[0] as i32 - u).abs() <= 3,
                    "u {} for {color:?}, expected {u}",
                    pair[0]
                );
                assert!(
                    (pair[1] as i32 - v).abs() <= 3,
                    "v {} for {color:?}, expected {v}",
                    pair[1]
                );
            }
        }
    }

    #[test]
    fn shared_nv12_planes_read_the_same_picture_without_copying_it() {
        let width = 4_u32;
        let height = 4_u32;
        let mut contiguous = vec![0_u8; (width * height) as usize * 3 / 2];
        for (index, sample) in contiguous.iter_mut().enumerate() {
            *sample = u8::try_from(index % 251).unwrap();
        }
        let copied = Nv12Planes::from_contiguous(width, height, &contiguous).unwrap();
        let shared = Nv12Planes::from_shared(width, height, std::sync::Arc::from(&contiguous[..]))
            .expect("a contiguous buffer of the right length is NV12");
        assert_eq!(copied.luma(), shared.luma());
        assert_eq!(copied.chroma(), shared.chroma());
        assert_eq!(shared.luma().len(), 16);
        assert_eq!(shared.chroma().len(), 8);
        assert!(
            Nv12Planes::from_shared(width, height, std::sync::Arc::from(&contiguous[..1]))
                .is_none(),
            "a short buffer is not a picture"
        );
        assert!(
            Nv12Planes::from_shared(3, height, std::sync::Arc::from(&contiguous[..])).is_none(),
            "an odd width has no chroma pairing"
        );
    }

    #[test]
    fn a_second_rasterizer_never_reuses_the_first_ones_cached_upload() {
        let renderer = match block_on(RenderDevice::new()) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping wgpu text identity test: {error}");
                return;
            }
        };
        let size = render_size(256, 256);
        let formatting = or_core::TextFormatting::DEFAULT;
        let mut first = TextRasterizer::new();
        let mut second = TextRasterizer::new();
        let left = first
            .rasterize_cached("Left", formatting, 256, 256)
            .unwrap();
        let right = second
            .rasterize_cached("Right", formatting, 256, 256)
            .unwrap();
        assert_ne!(
            left.id, right.id,
            "the pool outlives a rasterizer, so ids must be process-wide"
        );
        let drawn = |readback: &OffscreenReadback| -> Vec<u8> {
            readback
                .pixels()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| pixel[0])
                .collect()
        };
        let left_frame = renderer
            .composite_layers(
                size,
                &[CompositorLayer::Rgba(
                    RgbaVideoLayer::new(256, 256, &left.pixels)
                        .unwrap()
                        .with_cache_key(left.id),
                )],
            )
            .unwrap();
        let right_frame = renderer
            .composite_layers(
                size,
                &[CompositorLayer::Rgba(
                    RgbaVideoLayer::new(256, 256, &right.pixels)
                        .unwrap()
                        .with_cache_key(right.id),
                )],
            )
            .unwrap();
        assert_ne!(
            drawn(&left_frame),
            drawn(&right_frame),
            "the second session's title must be uploaded, not read from the first's cache"
        );
    }

    #[test]
    #[test]
    fn a_scissored_layer_paints_only_inside_its_ink_rectangle_when_adapter_is_available() {
        let renderer = match block_on(RenderDevice::new()) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping wgpu scissor test: {error}");
                return;
            }
        };
        let size = render_size(4, 4);
        let pixels = [255_u8, 0, 0, 255].repeat(16);
        let layer = RgbaVideoLayer::new(4, 4, &pixels)
            .expect("layer should validate")
            .with_scissor(0, 0, 2, 2);
        let readback = renderer
            .composite_layers(size, &[CompositorLayer::Rgba(layer)])
            .unwrap();
        for (index, pixel) in readback.pixels().as_chunks::<4>().0.iter().enumerate() {
            let inside = index == 0 || index == 1 || index == 4 || index == 5;
            let wanted = if inside {
                [255_u8, 0, 0, 255]
            } else {
                [0, 0, 0, 255]
            };
            for (channel, wanted) in pixel.iter().zip(wanted) {
                assert!(
                    (*channel as i32 - wanted as i32).abs() <= 3,
                    "pixel {index} was {pixel:?}, expected {wanted:?}"
                );
            }
        }
    }

    fn nv12_layers_composite_through_the_shared_pipeline_when_adapter_is_available() {
        let renderer = match block_on(RenderDevice::new()) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping wgpu nv12 layer test: {error}");
                return;
            }
        };
        let size = render_size(4, 4);
        // The inverse matrix matches the BT.601 limited-range conversion the
        // software decoder applies by default, so white stays the fixed point
        // (luma 235, chroma 128) and red is (81, 90, 240) in that space.
        for (luma, u, v, expected) in [
            (235_u8, 128_u8, 128_u8, [255_u8, 255, 255, 255]),
            (81, 90, 240, [255, 0, 0, 255]),
        ] {
            let luma_plane = [luma; 16];
            let chroma_plane = [u, v].repeat(4);
            let layer = Nv12VideoLayer::new(4, 4, &luma_plane, &chroma_plane).unwrap();
            let readback = renderer
                .composite_layers(size, &[CompositorLayer::Nv12(layer)])
                .unwrap();
            for pixel in readback.pixels().as_chunks::<4>().0 {
                for (channel, wanted) in pixel.iter().zip(expected) {
                    assert!(
                        (*channel as i32 - wanted as i32).abs() <= 3,
                        "pixel {pixel:?} for luma {luma}/u {u}/v {v}, expected {expected:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_cached_layer_keeps_its_upload_until_the_key_changes_when_adapter_is_available() {
        let renderer = match block_on(RenderDevice::new()) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping wgpu layer cache test: {error}");
                return;
            }
        };
        let size = render_size(4, 4);
        let white = [235_u8; 16];
        let white_chroma = [128_u8; 8];
        let red = [81_u8; 16];
        let mut red_chroma = [0_u8; 8];
        for pair in red_chroma.as_chunks_mut::<2>().0 {
            pair[0] = 90;
            pair[1] = 240;
        }
        let first = |layer: RgbaVideoLayer<'_>| {
            renderer
                .composite_layers(size, &[CompositorLayer::Rgba(layer)])
                .unwrap()
        };
        let white_rgba = [255_u8, 255, 255, 255].repeat(16);
        let red_rgba = [255_u8, 0, 0, 255].repeat(16);
        let read = |readback: &OffscreenReadback| -> [i32; 4] {
            let pixel = &readback.pixels()[..4];
            [
                i32::from(pixel[0]),
                i32::from(pixel[1]),
                i32::from(pixel[2]),
                i32::from(pixel[3]),
            ]
        };
        let assert_near = |actual: [i32; 4], expected: [i32; 4]| {
            for (channel, wanted) in actual.iter().zip(expected) {
                assert!(
                    (channel - wanted).abs() <= 3,
                    "pixel {actual:?}, expected {expected:?}"
                );
            }
        };

        // A key promises the pixels behind it are unchanged, so the second
        // composite with the same key must not re-upload the new buffer.
        let cached = first(
            RgbaVideoLayer::new(4, 4, &white_rgba)
                .unwrap()
                .with_cache_key(7),
        );
        assert_near(read(&cached), [255, 255, 255, 255]);
        let still_white = first(
            RgbaVideoLayer::new(4, 4, &red_rgba)
                .unwrap()
                .with_cache_key(7),
        );
        assert_near(read(&still_white), [255, 255, 255, 255]);
        // A new key invalidates the upload.
        let redrawn = first(
            RgbaVideoLayer::new(4, 4, &red_rgba)
                .unwrap()
                .with_cache_key(8),
        );
        assert_near(read(&redrawn), [255, 0, 0, 255]);
        // NV12 layers never cache: their planes are rebuilt by the decoder.
        let nv12 = Nv12VideoLayer::new(4, 4, &white, &white_chroma).unwrap();
        let readback = renderer
            .composite_layers(size, &[CompositorLayer::Nv12(nv12)])
            .unwrap();
        assert_near(read(&readback), [255, 255, 255, 255]);
        let nv12_red = Nv12VideoLayer::new(4, 4, &red, &red_chroma).unwrap();
        let readback = renderer
            .composite_layers(size, &[CompositorLayer::Nv12(nv12_red)])
            .unwrap();
        assert_near(read(&readback), [255, 0, 0, 255]);
    }

    #[test]
    fn nv12_readback_refuses_an_odd_size_when_adapter_is_available() {
        let renderer = match block_on(RenderDevice::new()) {
            Ok(renderer) => renderer,
            Err(error) => {
                eprintln!("skipping wgpu nv12 test: {error}");
                return;
            }
        };
        let size = render_size(3, 2);
        let input = RenderGraphInput::new(
            snapshot(),
            PreviewSurface::offscreen(size, RenderFormat::Rgba8Unorm),
            SyntheticScene::solid(SyntheticColor::GREEN),
        );
        let frame = renderer.render(input).unwrap();
        assert!(matches!(
            renderer.readback_nv12(&frame),
            Err(RenderError::OddSizeForNv12 { .. })
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
        for pixel in first_readback.pixels().as_chunks::<4>().0 {
            assert_eq!(*pixel, SyntheticColor::GREEN.channels());
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
pub mod motion;
mod motion_image;
mod text;
pub use motion::{
    MAX_MOTION_CANVAS_PIXELS, MAX_MOTION_FRAMES, MotionRasterError, MotionSequenceError,
    RasterizedMotionFrame, encode_png, for_each_motion_frame, for_each_motion_frame_with_images,
    rasterize_motion_frame, rasterize_motion_frame_with_images,
};
pub use motion_image::{
    MAX_MOTION_IMAGE_BYTES, MAX_MOTION_IMAGE_PIXELS, MotionImage, MotionImageError,
    decode_motion_image, load_motion_scene_images,
};
pub use text::{TextInkBounds, TextRasterError, TextRasterizer};
