//! PNG image assets for the MotionScene rasteriser.
//!
//! A scene refers to an image by identity (`assets[].asset_id`) and a
//! scene-relative path. This module decodes those files into straight-alpha RGBA
//! and bounds them, so the rasteriser can draw a real picture instead of the
//! placeholder it used before the asset pipeline existed. It reads only the files a
//! validated scene names; it never fetches a URL and never runs a decoder on
//! untrusted dimensions without a bound.

use crate::motion::MotionRasterError;
use or_motion::MotionScene;
use std::{
    collections::BTreeMap,
    fmt,
    io::Cursor,
    path::{Path, PathBuf},
};

/// The largest encoded image this decodes.
pub const MAX_MOTION_IMAGE_BYTES: usize = 16 * 1024 * 1024;
/// The largest decoded image, in pixels. 64 megapixels is far beyond a 4K still.
pub const MAX_MOTION_IMAGE_PIXELS: u64 = 64 * 1024 * 1024;

/// A decoded image: straight-alpha RGBA, row-major, no padding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MotionImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl MotionImage {
    /// Builds an image from validated RGBA bytes.
    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, MotionImageError> {
        if width == 0 || height == 0 {
            return Err(MotionImageError::InvalidDimensions);
        }
        let expected = width as usize * height as usize * 4;
        if u64::from(width) * u64::from(height) > MAX_MOTION_IMAGE_PIXELS {
            return Err(MotionImageError::TooManyPixels);
        }
        if pixels.len() != expected {
            return Err(MotionImageError::InvalidDimensions);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Bilinear sample at normalised coordinates, clamped at the edges.
    fn sample(&self, u: f64, v: f64) -> [u8; 4] {
        // Pixel centres sit at (n + 0.5) / size; subtracting half a texel makes a
        // 1:1 blit sample exact texels instead of blending neighbours.
        let x =
            (u.clamp(0.0, 1.0) * f64::from(self.width) - 0.5).clamp(0.0, f64::from(self.width - 1));
        let y = (v.clamp(0.0, 1.0) * f64::from(self.height) - 0.5)
            .clamp(0.0, f64::from(self.height - 1));
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let fx = x - f64::from(x0);
        let fy = y - f64::from(y0);
        let mut out = [0_u8; 4];
        for (channel, value) in out.iter_mut().enumerate() {
            let p00 = f64::from(self.pixel(x0, y0)[channel]);
            let p10 = f64::from(self.pixel(x1, y0)[channel]);
            let p01 = f64::from(self.pixel(x0, y1)[channel]);
            let p11 = f64::from(self.pixel(x1, y1)[channel]);
            let top = p00 + (p10 - p00) * fx;
            let bottom = p01 + (p11 - p01) * fx;
            *value = (top + (bottom - top) * fy).round().clamp(0.0, 255.0) as u8;
        }
        out
    }

    fn pixel(&self, x: u32, y: u32) -> &[u8] {
        let offset = ((y as usize * self.width as usize) + x as usize) * 4;
        &self.pixels[offset..offset + 4]
    }
}

/// Why an image could not be decoded or resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MotionImageError {
    /// The encoded file is larger than [`MAX_MOTION_IMAGE_BYTES`].
    TooLarge,
    /// The bytes are not a PNG this decoder accepts.
    InvalidPng,
    /// The decoded size is zero or too large.
    InvalidDimensions,
    /// The decoded image exceeds [`MAX_MOTION_IMAGE_PIXELS`].
    TooManyPixels,
    /// A scene asset path is remote, absolute or leaves the scene directory.
    UnsafePath(String),
    /// A scene names an asset whose file is missing.
    MissingAsset(String),
    /// A file could not be read.
    Unreadable(String),
}

impl MotionImageError {
    /// The stable wire spelling.
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooLarge => "MOTION_IMAGE_TOO_LARGE",
            Self::InvalidPng => "MOTION_IMAGE_INVALID_PNG",
            Self::InvalidDimensions => "MOTION_IMAGE_INVALID_DIMENSIONS",
            Self::TooManyPixels => "MOTION_IMAGE_TOO_MANY_PIXELS",
            Self::UnsafePath(_) => "MOTION_IMAGE_UNSAFE_PATH",
            Self::MissingAsset(_) => "MOTION_IMAGE_MISSING_ASSET",
            Self::Unreadable(_) => "MOTION_IMAGE_UNREADABLE",
        }
    }
}

impl fmt::Display for MotionImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => formatter.write_str("the image is larger than this version accepts"),
            Self::InvalidPng => formatter.write_str("the image is not a PNG this version decodes"),
            Self::InvalidDimensions => formatter.write_str("the image has invalid dimensions"),
            Self::TooManyPixels => formatter.write_str("the image is larger than the pixel bound"),
            Self::UnsafePath(path) => {
                write!(
                    formatter,
                    "the image path leaves the scene directory: {path}"
                )
            }
            Self::MissingAsset(path) => write!(formatter, "the image is missing: {path}"),
            Self::Unreadable(message) => {
                write!(formatter, "the image could not be read: {message}")
            }
        }
    }
}

impl std::error::Error for MotionImageError {}

/// Decodes one PNG into straight-alpha RGBA.
pub fn decode_motion_image(bytes: &[u8]) -> Result<MotionImage, MotionImageError> {
    if bytes.len() > MAX_MOTION_IMAGE_BYTES {
        return Err(MotionImageError::TooLarge);
    }
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .map_err(|_| MotionImageError::InvalidPng)?;
    let info = reader.info();
    if info.width == 0 || info.height == 0 {
        return Err(MotionImageError::InvalidDimensions);
    }
    if u64::from(info.width) * u64::from(info.height) > MAX_MOTION_IMAGE_PIXELS {
        return Err(MotionImageError::TooManyPixels);
    }
    let mut buffer = vec![0; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut buffer)
        .map_err(|_| MotionImageError::InvalidPng)?;
    let raw = &buffer[..frame.buffer_size()];
    let width = frame.width;
    let height = frame.height;
    let pixels = match frame.color_type {
        png::ColorType::Rgba => raw.to_vec(),
        png::ColorType::Rgb => raw
            .chunks_exact(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        png::ColorType::Grayscale => raw
            .iter()
            .flat_map(|value| [*value, *value, *value, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => raw
            .chunks_exact(2)
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        png::ColorType::Indexed => return Err(MotionImageError::InvalidPng),
    };
    MotionImage::from_rgba(width, height, pixels)
}

/// Loads every image a scene names, relative to the scene file's directory.
///
/// The map is keyed by asset id, which is what a rasterised instruction carries.
/// A remote or escaping path is refused rather than resolved.
pub fn load_motion_scene_images(
    scene: &MotionScene,
    base_directory: &Path,
) -> Result<BTreeMap<String, MotionImage>, MotionImageError> {
    let mut images = BTreeMap::new();
    for asset in &scene.assets {
        let relative = Path::new(&asset.path);
        if relative.is_absolute()
            || asset.path.contains("://")
            || relative
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(MotionImageError::UnsafePath(asset.path.clone()));
        }
        let path: PathBuf = base_directory.join(relative);
        let bytes = std::fs::read(&path)
            .map_err(|_| MotionImageError::MissingAsset(path.display().to_string()))?;
        let image = decode_motion_image(&bytes)?;
        images.insert(asset.asset_id.clone(), image);
    }
    Ok(images)
}

/// Draws one image instruction, reusing the rasteriser's blend and rotation rules.
pub(crate) fn draw_motion_image(
    canvas: &mut crate::motion::Canvas,
    instruction: &or_motion::MotionInstruction,
    image: &MotionImage,
) -> Result<(), MotionRasterError> {
    let size = instruction.size.unwrap_or(or_motion::MotionSize {
        width: f64::from(image.width()),
        height: f64::from(image.height()),
    });
    if size.width <= 0.0 || size.height <= 0.0 {
        return Ok(());
    }
    if !size.width.is_finite() || !size.height.is_finite() {
        return Err(MotionRasterError::InvalidCanvas);
    }
    let center_x = instruction.position.x + size.width / 2.0;
    let center_y = instruction.position.y + size.height / 2.0;
    let half_width = size.width / 2.0;
    let half_height = size.height / 2.0;
    let reach = (half_width * half_width + half_height * half_height).sqrt() + 1.0;
    let min_x = (center_x - reach).floor() as i64;
    let max_x = (center_x + reach).ceil() as i64;
    let min_y = (center_y - reach).floor() as i64;
    let max_y = (center_y + reach).ceil() as i64;
    let opacity = instruction.opacity.clamp(0.0, 1.0);
    if opacity <= 0.0 {
        return Ok(());
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (local_x, local_y) = crate::motion::unrotate(
                x as f64 + 0.5,
                y as f64 + 0.5,
                center_x,
                center_y,
                instruction.rotation_degrees,
            );
            if local_x.abs() > half_width || local_y.abs() > half_height {
                continue;
            }
            let u = (local_x + half_width) / size.width;
            let v = (local_y + half_height) / size.height;
            let [red, green, blue, alpha] = image.sample(u, v);
            let alpha = f64::from(alpha) / 255.0 * opacity;
            canvas.blend(
                x,
                y,
                [
                    f64::from(red) / 255.0,
                    f64::from(green) / 255.0,
                    f64::from(blue) / 255.0,
                    alpha,
                ],
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::{encode_png, rasterize_motion_frame_with_images};
    use or_core::RationalTime;
    use or_motion::{
        MotionCanvas, MotionNode, MotionNodeKind, MotionPoint, MotionScene, MotionSize,
    };

    fn scene_with_image(asset_path: &str) -> MotionScene {
        MotionScene {
            format: or_motion::MOTION_SCENE_FORMAT.to_owned(),
            schema_version: or_motion::MOTION_SCENE_SCHEMA_VERSION,
            scene_id: "6f1c2f5e-3a4b-4c5d-8e9f-0a1b2c3d4e5f".to_owned(),
            name: "Image".to_owned(),
            duration: RationalTime::new(1, 1).unwrap(),
            canvas: MotionCanvas {
                width: 2,
                height: 2,
            },
            assets: vec![or_motion::MotionAsset {
                asset_id: "pic".to_owned(),
                sha256: None,
                path: asset_path.to_owned(),
            }],
            nodes: vec![MotionNode {
                node_id: "pic".to_owned(),
                kind: MotionNodeKind::Image,
                children: None,
                text: None,
                position: Some(MotionPoint { x: 0.0, y: 0.0 }),
                size: Some(MotionSize {
                    width: 2.0,
                    height: 2.0,
                }),
                end: None,
                fill: None,
                stroke: None,
                stroke_width: None,
                font_size: None,
                asset_id: Some("pic".to_owned()),
                opacity: None,
                rotation_degrees: None,
                spans: None,
            }],
            animations: vec![],
            camera: None,
        }
        }

    #[test]
    fn a_produced_png_round_trips_through_the_decoder() {
        let pixels = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 128,
        ];
        let png = encode_png(2, 2, &pixels).unwrap();
        let image = decode_motion_image(&png).unwrap();
        assert_eq!(image.width(), 2);
        assert_eq!(image.height(), 2);
        assert_eq!(image.pixels(), pixels);
    }

    #[test]
    fn a_truncated_or_foreign_file_is_refused() {
        assert_eq!(
            decode_motion_image(b"not a png").unwrap_err(),
            MotionImageError::InvalidPng
        );
        let png = encode_png(2, 2, &vec![0; 16]).unwrap();
        assert!(decode_motion_image(&png[..png.len() / 2]).is_err());
    }

    #[test]
    fn scene_images_resolve_relative_to_the_scene_and_refuse_escape() {
        let directory = std::env::temp_dir().join(format!(
            "or-motion-image-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let png = encode_png(2, 2, &vec![10, 20, 30, 255].repeat(4)).unwrap();
        std::fs::write(directory.join("pic.png"), &png).unwrap();

        let scene = scene_with_image("pic.png");
        let images = load_motion_scene_images(&scene, &directory).unwrap();
        assert_eq!(images.len(), 1);

        let escaped = scene_with_image("../pic.png");
        assert!(matches!(
            load_motion_scene_images(&escaped, &directory),
            Err(MotionImageError::UnsafePath(_))
        ));

        let absolute = scene_with_image("/etc/passwd");
        assert!(matches!(
            load_motion_scene_images(&absolute, &directory),
            Err(MotionImageError::UnsafePath(_))
        ));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn an_image_node_draws_the_decoded_picture() {
        let png = encode_png(
            2,
            2,
            &[
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )
        .unwrap();
        let image = decode_motion_image(&png).unwrap();
        let scene = scene_with_image("pic.png");
        let frame = or_motion::evaluate_scene(&scene, RationalTime::new(0, 1).unwrap()).unwrap();
        let mut images = BTreeMap::new();
        images.insert("pic".to_owned(), image);
        let canvas = rasterize_motion_frame_with_images(&frame, &images).unwrap();
        assert_eq!(canvas.pixel(0, 0), Some([255, 0, 0, 255]));
        assert_eq!(canvas.pixel(1, 0), Some([0, 255, 0, 255]));
        assert_eq!(canvas.pixel(0, 1), Some([0, 0, 255, 255]));
    }

    #[test]
    fn a_missing_image_keeps_the_visible_placeholder() {
        let scene = scene_with_image("missing.png");
        let frame = or_motion::evaluate_scene(&scene, RationalTime::new(0, 1).unwrap()).unwrap();
        let canvas = rasterize_motion_frame_with_images(&frame, &BTreeMap::new()).unwrap();
        // The bordered placeholder leaves ink on the canvas rather than transparency.
        assert!(canvas.pixels().iter().any(|value| *value != 0));
    }
}
