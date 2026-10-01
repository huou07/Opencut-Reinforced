use cosmic_text::{
    Align, Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, Weight, Wrap,
    fontdb::Source,
};
use or_core::{
    FontIdentity, MAX_TEXT_SIZE_MILLI_POINTS, MIN_TEXT_SIZE_MILLI_POINTS, TextAlignment,
    TextFormatting, TextWeight,
};
use std::{error::Error, fmt, sync::Arc};

const MAX_RGBA_BYTES: usize = 256 * 1024 * 1024;
const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/inter/Inter-Regular.ttf");
const INTER_MEDIUM: &[u8] = include_bytes!("../assets/fonts/inter/Inter-Medium.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/inter/Inter-SemiBold.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/inter/Inter-Bold.ttf");

/// CPU text shaping and rasterization using only the bundled Inter faces.
pub struct TextRasterizer {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl TextRasterizer {
    pub fn new() -> Self {
        let mut font_db = cosmic_text::fontdb::Database::new();
        for font in [INTER_REGULAR, INTER_MEDIUM, INTER_SEMIBOLD, INTER_BOLD] {
            let _ = font_db.load_font_source(Source::Binary(Arc::new(font.to_vec())));
        }
        font_db.set_monospace_family("Inter");
        font_db.set_sans_serif_family("Inter");
        font_db.set_serif_family("Inter");
        Self {
            font_system: FontSystem::new_with_locale_and_db("en-US".to_owned(), font_db),
            swash_cache: SwashCache::new(),
        }
    }

    /// Rasterizes one bounded text layer to a transparent RGBA canvas.
    pub fn rasterize(
        &mut self,
        text: &str,
        formatting: TextFormatting,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, TextRasterError> {
        if !(MIN_TEXT_SIZE_MILLI_POINTS..=MAX_TEXT_SIZE_MILLI_POINTS)
            .contains(&formatting.size_milli_points)
        {
            return Err(TextRasterError::InvalidFormatting);
        }
        if width == 0 || height == 0 {
            return Err(TextRasterError::InvalidDimensions);
        }
        let width = usize::try_from(width).map_err(|_| TextRasterError::CanvasTooLarge)?;
        let height = usize::try_from(height).map_err(|_| TextRasterError::CanvasTooLarge)?;
        let byte_len = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes <= MAX_RGBA_BYTES)
            .ok_or(TextRasterError::CanvasTooLarge)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(byte_len)
            .map_err(|_| TextRasterError::AllocationFailed)?;
        pixels.resize(byte_len, 0);

        let width_f = width as f32;
        let height_f = height as f32;
        let margin_x = width_f * 0.05;
        let margin_y = height_f * 0.05;
        let font_size = formatting.size_milli_points as f32 / 1000.0 * (4.0 / 3.0);
        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics::new(font_size, font_size * 1.2),
        );
        buffer.set_size(
            Some(width_f - 2.0 * margin_x),
            Some(height_f - 2.0 * margin_y),
        );
        buffer.set_wrap(Wrap::WordOrGlyph);
        let family = match formatting.font {
            FontIdentity::BundledInter => "Inter",
        };
        let attrs = Attrs::new()
            .family(Family::Name(family))
            .weight(match formatting.weight {
                TextWeight::Regular => Weight(400),
                TextWeight::Medium => Weight(500),
                TextWeight::Semibold => Weight(600),
                TextWeight::Bold => Weight(700),
            });
        let alignment = match formatting.alignment {
            TextAlignment::Start => Align::Left,
            TextAlignment::Center => Align::Center,
            TextAlignment::End => Align::End,
        };
        buffer.set_text(text, &attrs, Shaping::Advanced, Some(alignment));
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut layout_bounds: Option<(f32, f32)> = None;
        for run in buffer.layout_runs() {
            let top = run.line_top;
            let bottom = top + run.line_height;
            layout_bounds = Some(match layout_bounds {
                Some((min_top, max_bottom)) => (min_top.min(top), max_bottom.max(bottom)),
                None => (top, bottom),
            });
        }
        let vertical_offset = layout_bounds.map_or(0.0, |(top, bottom)| {
            margin_y + ((height_f - 2.0 * margin_y) - (bottom - top)) / 2.0 - top
        });

        let text_color = Color::rgb(
            formatting.color.red,
            formatting.color.green,
            formatting.color.blue,
        );
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical = glyph.physical((margin_x, run.line_y + vertical_offset), 1.0);
                let origin_x = physical.x;
                let origin_y = physical.y;
                let pixels = &mut pixels;
                self.swash_cache.with_pixels(
                    &mut self.font_system,
                    physical.cache_key,
                    text_color,
                    |dx, dy, color| {
                        let x = origin_x + dx;
                        let y = origin_y + dy;
                        if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                            return;
                        }
                        let [red, green, blue, coverage] = color.as_rgba();
                        let source_alpha =
                            u16::from(coverage) * u16::from(formatting.color.alpha) / 255;
                        if source_alpha == 0 {
                            return;
                        }
                        let index = (y as usize * width + x as usize) * 4;
                        let destination_alpha = u16::from(pixels[index + 3]);
                        let alpha = source_alpha + destination_alpha * (255 - source_alpha) / 255;
                        pixels[index] = red;
                        pixels[index + 1] = green;
                        pixels[index + 2] = blue;
                        pixels[index + 3] = alpha as u8;
                    },
                );
            }
        }
        Ok(pixels)
    }
}

impl Default for TextRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextRasterError {
    InvalidDimensions,
    InvalidFormatting,
    CanvasTooLarge,
    AllocationFailed,
}

impl fmt::Display for TextRasterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions => {
                formatter.write_str("text canvas dimensions must be positive")
            }
            Self::InvalidFormatting => {
                formatter.write_str("text formatting is outside the supported bounds")
            }
            Self::CanvasTooLarge => {
                formatter.write_str("text canvas exceeds the render memory limit")
            }
            Self::AllocationFailed => formatter.write_str("text canvas allocation failed"),
        }
    }
}

impl Error for TextRasterError {}

#[cfg(test)]
mod tests {
    use super::{TextRasterError, TextRasterizer};
    use or_core::{TextAlignment, TextColor, TextFormatting, TextWeight};

    #[test]
    fn bundled_inter_rasterizes_deterministic_text_with_project_color() {
        let mut rasterizer = TextRasterizer::new();
        assert_eq!(rasterizer.font_system.db().faces().count(), 4);
        let formatting = TextFormatting {
            size_milli_points: 24_000,
            weight: TextWeight::Semibold,
            alignment: TextAlignment::Center,
            color: TextColor {
                red: 220,
                green: 40,
                blue: 10,
                alpha: 128,
            },
            ..TextFormatting::default()
        };
        let first = rasterizer
            .rasterize("Title\nCaption", formatting, 320, 180)
            .unwrap();
        let second = rasterizer
            .rasterize("Title\nCaption", formatting, 320, 180)
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 320 * 180 * 4);
        let all_pixels = first.as_chunks::<4>().0;
        assert!(all_pixels.iter().any(|pixel| pixel[3] == 0));
        let drawn = all_pixels.iter().filter(|pixel| pixel[3] > 0);
        let drawn: Vec<_> = drawn.collect();
        assert!(!drawn.is_empty());
        assert!(drawn.iter().all(|pixel| pixel[0] == 220));
        assert!(drawn.iter().all(|pixel| pixel[1] == 40));
        assert!(drawn.iter().all(|pixel| pixel[2] == 10));
        assert!(drawn.iter().all(|pixel| pixel[3] <= 128));
    }

    #[test]
    fn rasterizer_rejects_invalid_and_unbounded_canvases() {
        let mut rasterizer = TextRasterizer::new();
        assert_eq!(
            rasterizer.rasterize("Title", TextFormatting::default(), 0, 10),
            Err(TextRasterError::InvalidDimensions)
        );
        assert_eq!(
            rasterizer.rasterize("Title", TextFormatting::default(), 8193, 8192),
            Err(TextRasterError::CanvasTooLarge)
        );
    }
}
