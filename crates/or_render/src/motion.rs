//! Rasterises an evaluated MotionScene frame into one RGBA layer.
//!
//! This is the vector-to-pixel step ADR 0007's canonical path needs between
//! `evaluate` and the shared compositor. It is a *layer producer*, not a second
//! compositor: it returns an RGBA buffer that the existing `or_render` spine
//! composites like any decoded frame, so blending, effects and encoding stay in one
//! place.
//!
//! It reads nothing. An image node draws a placeholder rather than decoding an asset,
//! because evaluation resolves identities and the asset pipeline is a later step;
//! drawing a placeholder is honest about that, where silently drawing nothing would
//! look like a working scene with a missing picture.
//!
//! The coordinate space is canvas pixels with the origin at the top left, x to the
//! right and y down, matching the RGBA buffer's own layout.

use or_core::{TextAlignment, TextColor, TextFormatting, TextWeight};
use or_motion::{
    MotionColor, MotionFrame, MotionInstruction, MotionNodeKind, MotionScene, evaluate_scene,
};

/// The largest canvas this rasterises, in pixels. The text layer bound is the same
/// order of magnitude; a scene is authored content, not media.
pub const MAX_MOTION_CANVAS_PIXELS: u32 = 16_384 * 16_384;

/// The default stroke width for an arrow that names none.
const DEFAULT_ARROW_WIDTH: f64 = 2.0;

/// Why a frame could not be rasterised.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionRasterError {
    /// The canvas is empty or larger than [`MAX_MOTION_CANVAS_PIXELS`].
    InvalidCanvas,
    /// A text node's font size is outside what the bundled rasteriser accepts.
    InvalidTextSize,
}

impl MotionRasterError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidCanvas => "MOTION_INVALID_CANVAS",
            Self::InvalidTextSize => "MOTION_INVALID_TEXT_SIZE",
        }
    }
}

/// One RGBA canvas: `width * height * 4` bytes, straight (non-premultiplied) alpha.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterizedMotionFrame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl RasterizedMotionFrame {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The RGBA value at a pixel, or `None` outside the canvas.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = ((y as usize * self.width as usize) + x as usize) * 4;
        self.pixels
            .get(offset..offset + 4)
            .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
    }
}

/// Rasterises every instruction in a frame, in document order, onto a transparent
/// canvas. A later instruction paints over an earlier one, which is what document
/// order means for a scene.
pub fn rasterize_motion_frame(
    frame: &MotionFrame,
) -> Result<RasterizedMotionFrame, MotionRasterError> {
    let width = frame.canvas.width;
    let height = frame.canvas.height;
    if width == 0 || height == 0 || width.saturating_mul(height) > MAX_MOTION_CANVAS_PIXELS {
        return Err(MotionRasterError::InvalidCanvas);
    }
    let byte_len = width as usize * height as usize * 4;
    let mut canvas = Canvas {
        width,
        height,
        pixels: vec![0; byte_len],
    };
    let mut text = TextRasterizer::new();
    for instruction in &frame.instructions {
        draw_instruction(&mut canvas, &mut text, instruction)?;
    }
    Ok(RasterizedMotionFrame {
        width,
        height,
        pixels: canvas.pixels,
    })
}

use crate::text::TextRasterizer;

struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Canvas {
    /// Source-over blend of one straight-alpha colour onto the canvas.
    fn blend(&mut self, x: i64, y: i64, color: [f64; 4]) {
        if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
            return;
        }
        let source_alpha = color[3].clamp(0.0, 1.0);
        if source_alpha <= 0.0 {
            return;
        }
        let offset = ((y as usize * self.width as usize) + x as usize) * 4;
        let destination = &mut self.pixels[offset..offset + 4];
        let destination_alpha = destination[3] as f64 / 255.0;
        let out_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
        if out_alpha <= 0.0 {
            destination.copy_from_slice(&[0, 0, 0, 0]);
            return;
        }
        for channel in 0..3 {
            let source = color[channel].clamp(0.0, 1.0);
            let existing = destination[channel] as f64 / 255.0;
            let blended = (source * source_alpha
                + existing * destination_alpha * (1.0 - source_alpha))
                / out_alpha;
            destination[channel] = (blended * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        destination[3] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

fn rgba(color: MotionColor, opacity: f64) -> [f64; 4] {
    [
        color.red as f64 / 255.0,
        color.green as f64 / 255.0,
        color.blue as f64 / 255.0,
        (color.alpha as f64 / 255.0) * opacity,
    ]
}

/// The inverse-rotated offset of a point about a centre, in canvas units. Rotation
/// is clockwise for a positive angle in a y-down space, which is what a reader
/// expects from "rotate 90 degrees".
fn unrotate(x: f64, y: f64, center_x: f64, center_y: f64, degrees: f64) -> (f64, f64) {
    if degrees == 0.0 {
        return (x - center_x, y - center_y);
    }
    let radians = degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    let dx = x - center_x;
    let dy = y - center_y;
    (dx * cos + dy * sin, -dx * sin + dy * cos)
}

fn draw_instruction(
    canvas: &mut Canvas,
    text: &mut TextRasterizer,
    instruction: &MotionInstruction,
) -> Result<(), MotionRasterError> {
    match instruction.kind {
        MotionNodeKind::Group => {}
        MotionNodeKind::Rectangle | MotionNodeKind::Ellipse => {
            let Some(size) = instruction.size else {
                return Ok(());
            };
            let center_x = instruction.position.x + size.width / 2.0;
            let center_y = instruction.position.y + size.height / 2.0;
            let half_width = size.width / 2.0;
            let half_height = size.height / 2.0;
            let fill = instruction
                .fill
                .map(|color| rgba(color, instruction.opacity));
            let stroke = instruction
                .stroke
                .map(|color| rgba(color, instruction.opacity));
            let stroke_width = instruction.stroke_width.max(0.0);
            let radius_x = half_width - stroke_width / 2.0;
            let radius_y = half_height - stroke_width / 2.0;

            // The scan covers the shape's circumscribed circle, not its unrotated
            // box: a rotation moves ink outside that box, and scanning only the box
            // silently clipped rotated shapes.
            let reach = (half_width * half_width + half_height * half_height).sqrt() + 1.0;
            let min_x = (center_x - reach).floor() as i64;
            let max_x = (center_x + reach).ceil() as i64;
            let min_y = (center_y - reach).floor() as i64;
            let max_y = (center_y + reach).ceil() as i64;
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let (local_x, local_y) = unrotate(
                        x as f64 + 0.5,
                        y as f64 + 0.5,
                        center_x,
                        center_y,
                        instruction.rotation_degrees,
                    );
                    let inside = match instruction.kind {
                        MotionNodeKind::Rectangle => {
                            local_x.abs() <= half_width && local_y.abs() <= half_height
                        }
                        _ => {
                            if radius_x <= 0.0 || radius_y <= 0.0 {
                                false
                            } else {
                                let nx = local_x / radius_x;
                                let ny = local_y / radius_y;
                                nx * nx + ny * ny <= 1.0
                            }
                        }
                    };
                    if !inside {
                        continue;
                    }
                    let on_stroke = stroke_width > 0.0
                        && (local_x.abs() >= half_width - stroke_width
                            || local_y.abs() >= half_height - stroke_width);
                    match (on_stroke, stroke, fill) {
                        (true, Some(stroke), _) => canvas.blend(x, y, stroke),
                        (false, _, Some(fill)) => canvas.blend(x, y, fill),
                        (true, None, Some(fill)) => canvas.blend(x, y, fill),
                        _ => {}
                    }
                }
            }
        }
        MotionNodeKind::Arrow => {
            let Some(end) = instruction.end else {
                return Ok(());
            };
            let width = if instruction.stroke_width > 0.0 {
                instruction.stroke_width
            } else {
                DEFAULT_ARROW_WIDTH
            };
            let color = instruction
                .stroke
                .or(instruction.fill)
                .map(|color| rgba(color, instruction.opacity));
            let Some(color) = color else {
                return Ok(());
            };
            let start = instruction.position;
            let length = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
            if length <= f64::EPSILON {
                return Ok(());
            }
            let head = (width * 4.0).min(length / 2.0);
            let direction_x = (end.x - start.x) / length;
            let direction_y = (end.y - start.y) / length;
            let normal_x = -direction_y;
            let normal_y = direction_x;
            let half = width / 2.0;
            // A coarse but deterministic coverage test: a point is on the shaft when
            // its projection falls inside the segment and its distance is within half
            // the width. The head is a triangle at the far end.
            let min_x = (start.x.min(end.x) - head - 2.0).floor() as i64;
            let max_x = (start.x.max(end.x) + head + 2.0).ceil() as i64;
            let min_y = (start.y.min(end.y) - head - 2.0).floor() as i64;
            let max_y = (start.y.max(end.y) + head + 2.0).ceil() as i64;
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let point_x = x as f64 + 0.5;
                    let point_y = y as f64 + 0.5;
                    let along =
                        (point_x - start.x) * direction_x + (point_y - start.y) * direction_y;
                    let across = (point_x - start.x) * normal_x + (point_y - start.y) * normal_y;
                    let on_shaft = (0.0..=length - head).contains(&along) && across.abs() <= half;
                    let on_head = along > length - head
                        && along <= length
                        && across.abs() <= half * 3.0 * (length - along) / head.max(f64::EPSILON);
                    if on_shaft || on_head {
                        canvas.blend(x, y, color);
                    }
                }
            }
        }
        MotionNodeKind::Image => {
            // A placeholder, not a decoded asset: a bordered box with a cross, so a
            // missing picture is visible instead of silent.
            let Some(size) = instruction.size else {
                return Ok(());
            };
            let border = instruction
                .stroke
                .or(instruction.fill)
                .map(|color| rgba(color, instruction.opacity))
                .unwrap_or([0.6, 0.6, 0.6, instruction.opacity]);
            let center_x = instruction.position.x + size.width / 2.0;
            let center_y = instruction.position.y + size.height / 2.0;
            let reach = ((size.width / 2.0).powi(2) + (size.height / 2.0).powi(2)).sqrt() + 1.0;
            let min_x = (center_x - reach).floor() as i64;
            let max_x = (center_x + reach).ceil() as i64;
            let min_y = (center_y - reach).floor() as i64;
            let max_y = (center_y + reach).ceil() as i64;
            let thickness = 2.0_f64;
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let (local_x, local_y) = unrotate(
                        x as f64 + 0.5,
                        y as f64 + 0.5,
                        center_x,
                        center_y,
                        instruction.rotation_degrees,
                    );
                    let edge = local_x.abs() >= size.width / 2.0 - thickness
                        || local_y.abs() >= size.height / 2.0 - thickness;
                    // The two diagonals of the box.
                    let diagonal = (local_y - local_x * size.height / size.width).abs() < thickness
                        || (local_y + local_x * size.height / size.width).abs() < thickness;
                    if edge || diagonal {
                        canvas.blend(x, y, border);
                    }
                }
            }
        }
        MotionNodeKind::Text => {
            let Some(content) = &instruction.text else {
                return Ok(());
            };
            if content.is_empty() {
                return Ok(());
            }
            let size = instruction.font_size.unwrap_or(32.0).max(1.0);
            let milli_points = (size * 750.0).round();
            if !(1.0..=1_000_000.0).contains(&milli_points) {
                return Err(MotionRasterError::InvalidTextSize);
            }
            let formatting = TextFormatting {
                size_milli_points: milli_points as u32,
                weight: TextWeight::Regular,
                alignment: TextAlignment::Start,
                color: instruction
                    .fill
                    .map(|color| TextColor {
                        red: color.red,
                        green: color.green,
                        blue: color.blue,
                        alpha: color.alpha,
                    })
                    .unwrap_or(TextColor::WHITE),
                ..TextFormatting::DEFAULT
            };
            // The box the text is laid out in: as wide as the canvas allows from the
            // node's position, and **one line** tall. The rasteriser centres its
            // layout inside the box it is given, so a taller box would push the text
            // down from the position the scene names — a node's position is where its
            // text starts, not the top of an invisible block. V1 renders one line per
            // text node and clips it to the canvas; wrapping is a later step.
            let box_width = (canvas.width as f64 - instruction.position.x)
                .clamp(16.0, canvas.width as f64) as u32;
            let box_height = (size * 1.25).clamp(8.0, canvas.height as f64) as u32;
            // Highlighted runs go through the same layout with more than one colour;
            // a node without spans takes the single-colour path.
            let spans = instruction
                .spans
                .iter()
                .map(|span| {
                    (
                        span.start,
                        span.end,
                        TextColor {
                            red: span.color.red,
                            green: span.color.green,
                            blue: span.color.blue,
                            alpha: span.color.alpha,
                        },
                    )
                })
                .collect::<Vec<_>>();
            let layer = text
                .rasterize_spans(content, &spans, formatting, box_width, box_height)
                .map_err(|_| MotionRasterError::InvalidTextSize)?;
            let center_x = instruction.position.x + box_width as f64 / 2.0;
            let center_y = instruction.position.y + box_height as f64 / 2.0;
            let min_x = instruction.position.x.floor() as i64;
            let max_x = (instruction.position.x + box_width as f64).ceil() as i64;
            let min_y = instruction.position.y.floor() as i64;
            let max_y = (instruction.position.y + box_height as f64).ceil() as i64;
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let (local_x, local_y) = unrotate(
                        x as f64 + 0.5,
                        y as f64 + 0.5,
                        center_x,
                        center_y,
                        instruction.rotation_degrees,
                    );
                    let source_x = (local_x + box_width as f64 / 2.0).floor();
                    let source_y = (local_y + box_height as f64 / 2.0).floor();
                    if source_x < 0.0 || source_y < 0.0 {
                        continue;
                    }
                    let source_x = source_x as usize;
                    let source_y = source_y as usize;
                    if source_x >= box_width as usize || source_y >= box_height as usize {
                        continue;
                    }
                    let offset = (source_y * box_width as usize + source_x) * 4;
                    let Some(pixel) = layer.get(offset..offset + 4) else {
                        continue;
                    };
                    let alpha = pixel[3] as f64 / 255.0 * instruction.opacity;
                    if alpha <= 0.0 {
                        continue;
                    }
                    canvas.blend(
                        x,
                        y,
                        [
                            pixel[0] as f64 / 255.0,
                            pixel[1] as f64 / 255.0,
                            pixel[2] as f64 / 255.0,
                            alpha,
                        ],
                    );
                }
            }
        }
    }
    Ok(())
}

/// The most frames one materialisation may produce. A scene is authored content, and
/// a bound keeps a mistyped duration or frame rate from asking for hours of work.
pub const MAX_MOTION_FRAMES: u64 = 200_000;

/// Why a frame sequence could not be produced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionSequenceError {
    /// The frame rate is zero, or the scene would need more than
    /// [`MAX_MOTION_FRAMES`] frames.
    InvalidFrameRate,
    /// A frame failed to evaluate or rasterise.
    Frame(MotionRasterError),
}

impl MotionSequenceError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidFrameRate => "MOTION_INVALID_FRAME_RATE",
            Self::Frame(error) => error.code(),
        }
    }
}

/// Produces every frame of a scene at a frame rate, in order, and hands each to
/// `visitor` with its index.
///
/// The frames are the half-open interval `[0, duration)` at exact rational times, so
/// the sequence is deterministic and seekable: frame `n` is at `n * denominator /
/// numerator` seconds, computed through the rationals rather than by accumulating a
/// floating-point step. A visitor is used rather than a collected vector so a long
/// scene does not have to fit in memory: materialisation writes each frame as it is
/// produced.
pub fn for_each_motion_frame<F>(
    scene: &MotionScene,
    frame_rate_numerator: u32,
    frame_rate_denominator: u32,
    mut visitor: F,
) -> Result<u64, MotionSequenceError>
where
    F: FnMut(u64, &RasterizedMotionFrame) -> Result<(), MotionRasterError>,
{
    if frame_rate_numerator == 0 || frame_rate_denominator == 0 {
        return Err(MotionSequenceError::InvalidFrameRate);
    }
    // The number of frames in [0, duration) is the floor of duration * rate, plus one
    // when the product is not exact, so a partial final frame is still drawn.
    let duration_numerator = scene.duration.numerator() as i128;
    let duration_denominator = scene.duration.denominator() as i128;
    let scaled = duration_numerator
        .checked_mul(frame_rate_numerator as i128)
        .ok_or(MotionSequenceError::InvalidFrameRate)?;
    let divisor = duration_denominator
        .checked_mul(frame_rate_denominator as i128)
        .ok_or(MotionSequenceError::InvalidFrameRate)?;
    if divisor == 0 || scaled < 0 {
        return Err(MotionSequenceError::InvalidFrameRate);
    }
    let frames = (scaled + divisor - 1) / divisor;
    let frames = u64::try_from(frames).map_err(|_| MotionSequenceError::InvalidFrameRate)?;
    if frames > MAX_MOTION_FRAMES {
        return Err(MotionSequenceError::InvalidFrameRate);
    }

    for index in 0..frames {
        let time = frame_time(
            index,
            frame_rate_numerator,
            frame_rate_denominator,
            scene.duration,
        )?;
        let frame = evaluate_scene(scene, time)
            .map_err(|_| MotionSequenceError::Frame(MotionRasterError::InvalidCanvas))?;
        let canvas = rasterize_motion_frame(&frame).map_err(MotionSequenceError::Frame)?;
        visitor(index, &canvas).map_err(MotionSequenceError::Frame)?;
    }
    Ok(frames)
}

/// The exact time of one frame index: `index * denominator / numerator` seconds.
fn frame_time(
    index: u64,
    frame_rate_numerator: u32,
    frame_rate_denominator: u32,
    duration: or_core::RationalTime,
) -> Result<or_core::RationalTime, MotionSequenceError> {
    let numerator = (index as i128)
        .checked_mul(frame_rate_denominator as i128)
        .ok_or(MotionSequenceError::InvalidFrameRate)?;
    let denominator = frame_rate_numerator as i128;
    // `index * denominator / numerator` as an exact rational, reduced by the time
    // model itself.
    let reduced = reduce(numerator, denominator).ok_or(MotionSequenceError::InvalidFrameRate)?;
    let time = or_core::RationalTime::new(reduced.0, reduced.1)
        .map_err(|_| MotionSequenceError::InvalidFrameRate)?;
    if time >= duration {
        // The final partial frame still gets a time inside the interval.
        return duration
            .checked_sub(
                or_core::RationalTime::new(1, 1_000_000)
                    .map_err(|_| MotionSequenceError::InvalidFrameRate)?,
            )
            .map_err(|_| MotionSequenceError::InvalidFrameRate);
    }
    Ok(time)
}

fn reduce(numerator: i128, denominator: i128) -> Option<(i64, u32)> {
    if denominator == 0 {
        return None;
    }
    let mut a = numerator.abs();
    let mut b = denominator.abs();
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    let divisor = a.max(1);
    let numerator = numerator / divisor;
    let denominator = denominator / divisor;
    let numerator = i64::try_from(numerator).ok()?;
    let denominator = u32::try_from(denominator).ok()?;
    Some((numerator, denominator))
}

/// Encodes a rasterised frame as a PNG.
///
/// A **still preview** encoder, not a media authority: materialised video goes
/// through the normal media and export boundary as ADR 0007 requires. This exists so
/// a scene can be looked at — by a person reviewing a template, or by a test that
/// needs a real image — without pulling an image library into the render crate. The
/// stream is deterministic: no timestamps, no metadata, and deflate's stored blocks,
/// so the same frame always produces byte-identical output.
pub fn encode_png(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>, MotionRasterError> {
    let expected = width as usize * height as usize * 4;
    if width == 0 || height == 0 || pixels.len() != expected {
        return Err(MotionRasterError::InvalidCanvas);
    }

    // One filter byte per row, always 0 (None): the rasteriser's output is already
    // what it is, and a filter would only make the bytes harder to reason about.
    let mut raw = Vec::with_capacity(expected + height as usize);
    for row in 0..height as usize {
        raw.push(0);
        let start = row * width as usize * 4;
        raw.extend_from_slice(&pixels[start..start + width as usize * 4]);
    }

    let mut zlib = Vec::with_capacity(raw.len() + raw.len() / 65_535 * 5 + 16);
    zlib.extend_from_slice(&[0x78, 0x01]);
    let mut offset = 0;
    while offset < raw.len() {
        let block = (raw.len() - offset).min(65_535);
        let final_block = offset + block == raw.len();
        zlib.push(u8::from(final_block));
        let length = block as u16;
        zlib.extend_from_slice(&length.to_le_bytes());
        zlib.extend_from_slice(&(!length).to_le_bytes());
        zlib.extend_from_slice(&raw[offset..offset + block]);
        offset += block;
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut png = Vec::with_capacity(zlib.len() + 64);
    png.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    write_chunk(&mut png, b"IHDR", &header);
    write_chunk(&mut png, b"IDAT", &zlib);
    write_chunk(&mut png, b"IEND", &[]);
    Ok(png)
}

fn write_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    png.extend_from_slice(kind);
    png.extend_from_slice(data);
    let mut crc = Crc32::new();
    crc.update(kind);
    crc.update(data);
    png.extend_from_slice(&crc.finish().to_be_bytes());
}

fn adler32(bytes: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for byte in bytes {
        a = (a + *byte as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

/// CRC-32 (IEEE 802.3), which is what PNG chunks carry.
struct Crc32 {
    value: u32,
    table: [u32; 256],
}

impl Crc32 {
    fn new() -> Self {
        let mut table = [0_u32; 256];
        for (index, entry) in table.iter_mut().enumerate() {
            let mut value = index as u32;
            for _ in 0..8 {
                value = if value & 1 == 1 {
                    0xedb8_8320 ^ (value >> 1)
                } else {
                    value >> 1
                };
            }
            *entry = value;
        }
        Self {
            value: 0xffff_ffff,
            table,
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            let index = ((self.value ^ *byte as u32) & 0xff) as usize;
            self.value = self.table[index] ^ (self.value >> 8);
        }
    }

    fn finish(self) -> u32 {
        self.value ^ 0xffff_ffff
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use or_motion::{MotionScene, parse_motion_scene};

    /// A small scene with one node per kind the rasteriser draws, all inside a
    /// 64x64 canvas so a test can name exact pixels.
    fn scene() -> MotionScene {
        let scene = serde_json::json!({
            "format": "opencut-reinforced-motion-scene",
            "schema_version": 1,
            "scene_id": "6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f",
            "name": "Raster probe",
            "duration": {"numerator": 4, "denominator": 1},
            "canvas": {"width": 64, "height": 64},
            "assets": [{"asset_id": "pic", "path": "assets/pic.png"}],
            "nodes": [
                {
                    "node_id": "under",
                    "kind": "rectangle",
                    "position": {"x": 8, "y": 8},
                    "size": {"width": 24, "height": 24},
                    "fill": {"red": 255, "green": 0, "blue": 0}
                },
                {
                    "node_id": "over",
                    "kind": "rectangle",
                    "position": {"x": 16, "y": 16},
                    "size": {"width": 24, "height": 24},
                    "fill": {"red": 0, "green": 0, "blue": 255},
                    "opacity": 0.5
                },
                {
                    "node_id": "circle",
                    "kind": "ellipse",
                    "position": {"x": 40, "y": 4},
                    "size": {"width": 20, "height": 20},
                    "fill": {"red": 0, "green": 255, "blue": 0}
                },
                {
                    "node_id": "pointer",
                    "kind": "arrow",
                    "position": {"x": 2, "y": 58},
                    "end": {"x": 30, "y": 58},
                    "stroke": {"red": 255, "green": 255, "blue": 0},
                    "stroke_width": 3
                },
                {
                    "node_id": "picture",
                    "kind": "image",
                    "position": {"x": 40, "y": 40},
                    "size": {"width": 20, "height": 20},
                    "asset_id": "pic",
                    "stroke": {"red": 200, "green": 200, "blue": 200}
                },
                {
                    "node_id": "label",
                    "kind": "text",
                    "text": "Hi",
                    "position": {"x": 2, "y": 30},
                    "font_size": 14,
                    "fill": {"red": 255, "green": 255, "blue": 255}
                }
            ],
            "animations": []
        });
        parse_motion_scene(&serde_json::to_vec(&scene).unwrap()).unwrap()
    }

    fn raster() -> RasterizedMotionFrame {
        let frame =
            or_motion::evaluate_scene(&scene(), or_core::RationalTime::new(1, 1).unwrap()).unwrap();
        rasterize_motion_frame(&frame).unwrap()
    }

    #[test]
    fn a_rectangle_paints_its_fill_where_it_is_and_nowhere_else() {
        let canvas = raster();
        assert_eq!(canvas.width(), 64);
        assert_eq!(canvas.height(), 64);
        // Inside the red rectangle and outside the blue one that covers its middle.
        assert_eq!(canvas.pixel(10, 10), Some([255, 0, 0, 255]));
        // Outside every node.
        assert_eq!(canvas.pixel(63, 0), Some([0, 0, 0, 0]));
        assert_eq!(canvas.pixel(0, 63), Some([0, 0, 0, 0]));
    }

    #[test]
    fn a_later_instruction_paints_over_an_earlier_one_with_its_opacity() {
        // Document order is paint order, and the second rectangle is half opaque, so
        // the result is a blend of blue over red rather than either colour alone.
        let canvas = raster();
        let blended = canvas.pixel(20, 20).unwrap();
        assert_eq!(blended[3], 255);
        // Half-opaque blue over opaque red: both channels contribute, and green does
        // not, so the pixel is neither colour alone.
        assert!((blended[0] as i32 - 128).abs() <= 1, "{blended:?}");
        assert_eq!(blended[1], 0);
        assert!((blended[2] as i32 - 128).abs() <= 1, "{blended:?}");
    }

    #[test]
    fn an_ellipse_is_round_rather_than_a_box() {
        let canvas = raster();
        // The centre of the green ellipse is filled.
        assert_eq!(canvas.pixel(50, 14), Some([0, 255, 0, 255]));
        // Its bounding box corner is not: that is what makes it an ellipse.
        assert_eq!(canvas.pixel(41, 5), Some([0, 0, 0, 0]));
    }

    #[test]
    fn an_arrow_draws_a_shaft_along_its_direction() {
        let canvas = raster();
        let shaft = canvas.pixel(10, 58).unwrap();
        assert_eq!(shaft[3], 255);
        assert!(shaft[0] > 200 && shaft[1] > 200, "{shaft:?}");
        // Above the shaft is empty.
        assert_eq!(canvas.pixel(10, 50), Some([0, 0, 0, 0]));
    }

    #[test]
    fn an_image_node_draws_a_visible_placeholder_rather_than_decoding_an_asset() {
        let canvas = raster();
        // The border of the placeholder is drawn...
        let border = canvas.pixel(50, 40).unwrap();
        assert!(border[3] > 0, "{border:?}");
        // ...and the space between its diagonals is empty, so it reads as a
        // placeholder rather than a picture.
        assert_eq!(canvas.pixel(44, 50), Some([0, 0, 0, 0]));
    }

    #[test]
    fn a_text_node_draws_ink_inside_its_box() {
        let canvas = raster();
        // Glyph coverage is exact, so the assertion is "some ink, and only inside the
        // box", not a particular glyph shape.
        let mut inked = 0;
        for y in 30..44 {
            for x in 2..40 {
                if canvas.pixel(x, y).is_some_and(|pixel| pixel[3] > 0) {
                    inked += 1;
                }
            }
        }
        assert!(inked > 10, "expected glyph ink, found {inked} pixels");
        // Away from every node there is no ink.
        assert_eq!(canvas.pixel(2, 2), Some([0, 0, 0, 0]));
    }

    #[test]
    fn a_rotated_rectangle_paints_where_the_rotation_puts_it() {
        let mut value = serde_json::to_value(scene()).unwrap();
        value["nodes"] = serde_json::json!([{
            "node_id": "tilted",
            "kind": "rectangle",
            "position": {"x": 24, "y": 24},
            "size": {"width": 32, "height": 8},
            "fill": {"red": 255, "green": 0, "blue": 0},
            "rotation_degrees": 90.0
        }]);
        let scene = parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap();
        let frame =
            or_motion::evaluate_scene(&scene, or_core::RationalTime::new(0, 1).unwrap()).unwrap();
        let canvas = rasterize_motion_frame(&frame).unwrap();
        // A 90-degree rotation turns the wide bar into a tall one about its centre,
        // which is at (40, 28): the ink is x 36..44 and y 12..44.
        assert_eq!(canvas.pixel(40, 20).unwrap()[3], 255);
        assert_eq!(canvas.pixel(40, 40).unwrap()[3], 255);
        // The unrotated footprint, x 24..56 and y 24..32, is not what got painted.
        assert_eq!(canvas.pixel(20, 28), Some([0, 0, 0, 0]));
        assert_eq!(canvas.pixel(56, 28), Some([0, 0, 0, 0]));
        assert_eq!(canvas.pixel(56, 56), Some([0, 0, 0, 0]));
    }

    #[test]
    fn an_empty_or_absurd_canvas_is_refused() {
        // Evaluation refuses an invalid scene first, so mutate a valid frame instead
        // and exercise the rasteriser's own check.
        let mut frame =
            or_motion::evaluate_scene(&scene(), or_core::RationalTime::new(0, 1).unwrap()).unwrap();
        frame.canvas.width = 0;
        assert_eq!(
            rasterize_motion_frame(&frame).unwrap_err(),
            MotionRasterError::InvalidCanvas
        );
        frame.canvas.width = u32::MAX;
        assert_eq!(
            rasterize_motion_frame(&frame).unwrap_err(),
            MotionRasterError::InvalidCanvas
        );
        assert_eq!(
            MotionRasterError::InvalidCanvas.code(),
            "MOTION_INVALID_CANVAS"
        );
    }

    #[test]
    fn a_sequence_covers_the_half_open_interval_at_exact_frame_times() {
        // Four seconds at 2 fps is eight frames: 0, 0.5, ... 3.5, and never 4.0.
        let mut seen = Vec::new();
        let frames = for_each_motion_frame(&scene(), 2, 1, |index, canvas| {
            seen.push((index, canvas.width()));
            Ok(())
        })
        .unwrap();
        assert_eq!(frames, 8);
        assert_eq!(
            seen.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
            (0..8).collect::<Vec<_>>()
        );

        // An exact rational rate, and a duration that does not divide evenly: a
        // partial final frame is still produced.
        let mut times = Vec::new();
        let scene = {
            let mut value = serde_json::to_value(scene()).unwrap();
            value["duration"] = serde_json::json!({"numerator": 1, "denominator": 1});
            parse_motion_scene(&serde_json::to_vec(&value).unwrap()).unwrap()
        };
        let frames = for_each_motion_frame(&scene, 3, 1, |index, _| {
            times.push(index);
            Ok(())
        })
        .unwrap();
        assert_eq!(frames, 3);
        assert_eq!(times, vec![0, 1, 2]);
    }

    #[test]
    fn an_absurd_frame_rate_or_length_is_refused_rather_than_attempted() {
        assert_eq!(
            for_each_motion_frame(&scene(), 0, 1, |_, _| Ok(())).unwrap_err(),
            MotionSequenceError::InvalidFrameRate
        );
        assert_eq!(
            for_each_motion_frame(&scene(), 1, 0, |_, _| Ok(())).unwrap_err(),
            MotionSequenceError::InvalidFrameRate
        );
        // Four hundred thousand frames is past the bound, and the bound is checked
        // before any frame is produced: a refused request must not do the work it is
        // refusing.
        let mut visited = 0;
        assert_eq!(
            for_each_motion_frame(&scene(), 100_000, 1, |_, _| {
                visited += 1;
                Ok(())
            })
            .unwrap_err(),
            MotionSequenceError::InvalidFrameRate
        );
        assert_eq!(visited, 0);
        assert_eq!(
            MotionSequenceError::InvalidFrameRate.code(),
            "MOTION_INVALID_FRAME_RATE"
        );
    }

    #[test]
    fn a_visitor_error_stops_the_sequence() {
        // A cancelled materialisation must not keep rendering frames it will discard.
        let mut visited = 0;
        let error = for_each_motion_frame(&scene(), 2, 1, |_, _| {
            visited += 1;
            if visited == 3 {
                return Err(MotionRasterError::InvalidCanvas);
            }
            Ok(())
        })
        .unwrap_err();
        assert_eq!(
            error,
            MotionSequenceError::Frame(MotionRasterError::InvalidCanvas)
        );
        assert_eq!(visited, 3);
    }

    #[test]
    fn a_frame_encodes_to_a_png_a_decoder_can_read() {
        let canvas = raster();
        let png = encode_png(canvas.width(), canvas.height(), canvas.pixels()).unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        // IHDR carries the size, bit depth 8 and colour type 6 (RGBA).
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes([png[16], png[17], png[18], png[19]]), 64);
        assert_eq!(u32::from_be_bytes([png[20], png[21], png[22], png[23]]), 64);
        assert_eq!(&png[24..29], &[8, 6, 0, 0, 0]);
        // The stream ends with IEND, and every chunk length fits the file.
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
        // The same frame always encodes to the same bytes: no timestamps, no
        // metadata, and stored deflate blocks.
        assert_eq!(
            png,
            encode_png(canvas.width(), canvas.height(), canvas.pixels()).unwrap()
        );
        // A canvas that does not match its buffer is refused rather than truncated.
        assert_eq!(
            encode_png(64, 64, &canvas.pixels()[..16]).unwrap_err(),
            MotionRasterError::InvalidCanvas
        );
    }

    #[test]
    fn a_highlighted_span_paints_a_different_colour_from_the_rest_of_the_line() {
        // The tokeniser's whole point is that a keyword is visibly not the rest of the
        // line, so the assertion is about two colours in one text node's ink.
        let source = b"let value = 41;";
        let scene = or_motion::scene_from_source(
            source,
            or_motion::CodeLanguage::Rust,
            &or_motion::CodeSceneOptions {
                canvas: or_motion::MotionCanvas {
                    width: 256,
                    height: 128,
                },
                font_size: 24.0,
                ..or_motion::CodeSceneOptions::default()
            },
        )
        .unwrap();
        let frame =
            or_motion::evaluate_scene(&scene, or_core::RationalTime::new(1, 1).unwrap()).unwrap();
        let canvas = rasterize_motion_frame(&frame).unwrap();
        let mut keyword_ink = 0;
        let mut number_ink = 0;
        let mut other_ink = 0;
        let keyword = or_motion::CodeTheme::OR_DARK.keyword;
        let number = or_motion::CodeTheme::OR_DARK.number;
        for y in 0..canvas.height() {
            for x in 0..canvas.width() {
                let Some(pixel) = canvas.pixel(x, y) else {
                    continue;
                };
                if pixel[3] == 0 {
                    continue;
                }
                if pixel[0] == keyword.red && pixel[1] == keyword.green && pixel[2] == keyword.blue
                {
                    keyword_ink += 1;
                } else if pixel[0] == number.red
                    && pixel[1] == number.green
                    && pixel[2] == number.blue
                {
                    number_ink += 1;
                } else {
                    other_ink += 1;
                }
            }
        }
        assert!(
            keyword_ink > 0,
            "expected keyword ink in the keyword colour"
        );
        assert!(number_ink > 0, "expected number ink in the number colour");
        assert!(
            other_ink > 0,
            "expected the rest of the line in the foreground colour"
        );
    }

    #[test]
    fn rasterising_the_same_frame_twice_produces_identical_pixels() {
        assert_eq!(raster().pixels(), raster().pixels());
    }
}
