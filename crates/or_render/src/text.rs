use cosmic_text::{
    Align, Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, Weight, Wrap,
    fontdb::Source,
};
use or_core::{
    FontIdentity, MAX_TEXT_SIZE_MILLI_POINTS, MIN_TEXT_SIZE_MILLI_POINTS, TextAlignment, TextColor,
    TextFormatting, TextVerticalAnchor, TextWeight,
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
        self.rasterize_spans(text, &[], formatting, width, height)
    }

    /// Rasterizes one bounded text layer with per-span colours.
    ///
    /// `spans` are byte ranges into `text` with their own colour; anything they do
    /// not cover takes `formatting.color`. Layout, shaping and glyph placement are
    /// identical to [`Self::rasterize`] — this is the same engine with more than one
    /// colour, not a second text path.
    pub fn rasterize_spans(
        &mut self,
        text: &str,
        spans: &[(usize, usize, TextColor)],
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
        let shift_x = width_f * f32::from(formatting.offset_x_basis_points) / 10_000.0;
        let shift_y = height_f * f32::from(formatting.offset_y_basis_points) / 10_000.0;
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
        if spans.is_empty() {
            buffer.set_text(text, &attrs, Shaping::Advanced, Some(alignment));
        } else {
            // Split the text into runs at every span boundary, so each run carries one
            // colour and cosmic-text shapes the whole string in one pass.
            let mut boundaries = vec![0, text.len()];
            for (start, end, _) in spans {
                if *start <= text.len() && *end <= text.len() && start < end {
                    boundaries.push(*start);
                    boundaries.push(*end);
                }
            }
            boundaries.sort_unstable();
            boundaries.dedup();
            let mut runs: Vec<(&str, Attrs)> = Vec::new();
            for pair in boundaries.windows(2) {
                let [start, end] = pair else { continue };
                if start == end || !text.is_char_boundary(*start) || !text.is_char_boundary(*end) {
                    continue;
                }
                let color = spans
                    .iter()
                    .find(|(span_start, span_end, _)| span_start <= start && end <= span_end)
                    .map(|(_, _, color)| *color)
                    .unwrap_or(formatting.color);
                let run_attrs = attrs
                    .clone()
                    .color(Color::rgb(color.red, color.green, color.blue));
                runs.push((&text[*start..*end], run_attrs));
            }
            buffer.set_rich_text(runs, &attrs, Shaping::Advanced, Some(alignment));
        }
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
            let anchored = match formatting.anchor {
                TextVerticalAnchor::Top => margin_y - top,
                TextVerticalAnchor::Center => {
                    margin_y + ((height_f - 2.0 * margin_y) - (bottom - top)) / 2.0 - top
                }
                TextVerticalAnchor::Bottom => height_f - margin_y - bottom,
            };
            anchored + shift_y
        });

        // The background is one box behind the whole block, sized from the real
        // shaped glyph bounds rather than from a character-count estimate.
        if let Some(background) = formatting.background {
            let mut bounds: Option<(f32, f32, f32, f32)> = None;
            for run in buffer.layout_runs() {
                for glyph in run.glyphs {
                    let physical =
                        glyph.physical((margin_x + shift_x, run.line_y + vertical_offset), 1.0);
                    let left = physical.x as f32;
                    let top = physical.y as f32;
                    let right = left + glyph.w;
                    let bottom = top + run.line_height;
                    bounds = Some(match bounds {
                        Some((min_x, min_y, max_x, max_y)) => (
                            min_x.min(left),
                            min_y.min(top),
                            max_x.max(right),
                            max_y.max(bottom),
                        ),
                        None => (left, top, right, bottom),
                    });
                }
            }
            if let Some((min_x, min_y, max_x, max_y)) = bounds {
                let padding = background.padding_milli_points as f32 / 1_000.0 * (4.0 / 3.0);
                let left = (min_x - padding).floor().clamp(0.0, width_f) as usize;
                let top = (min_y - padding).floor().clamp(0.0, height_f) as usize;
                let right = (max_x + padding).ceil().clamp(0.0, width_f) as usize;
                let bottom = (max_y + padding).ceil().clamp(0.0, height_f) as usize;
                let background_alpha = (u32::from(background.color.alpha) * u32::from(formatting.opacity_basis_points)
                    / 10_000) as u8;
                if background_alpha > 0 {
                    for y in top..bottom {
                        for x in left..right {
                            let index = (y * width + x) * 4;
                            pixels[index] = background.color.red;
                            pixels[index + 1] = background.color.green;
                            pixels[index + 2] = background.color.blue;
                            pixels[index + 3] = background_alpha;
                        }
                    }
                }
            }
        }

        let text_color = Color::rgb(
            formatting.color.red,
            formatting.color.green,
            formatting.color.blue,
        );
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical = glyph.physical(
                    (margin_x + shift_x, run.line_y + vertical_offset),
                    1.0,
                );
                let origin_x = physical.x;
                let origin_y = physical.y;
                let pixels = &mut pixels;
                // A glyph rendered from rich text carries its own colour; the layer's
                // colour is the fallback for the single-colour path. The glyph image
                // is a mask, so the base colour is the only thing that colours it.
                let base_color = glyph.color_opt.unwrap_or(text_color);
                self.swash_cache.with_pixels(
                    &mut self.font_system,
                    physical.cache_key,
                    base_color,
                    |dx, dy, color| {
                        let x = origin_x + dx;
                        let y = origin_y + dy;
                        if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                            return;
                        }
                        let [red, green, blue, coverage] = color.as_rgba();
                        let source_alpha = (u32::from(coverage)
                            * u32::from(formatting.color.alpha)
                            * u32::from(formatting.opacity_basis_points)
                            / (255 * 10_000)) as u16;
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
    use or_core::{
        TextAlignment, TextColor, TextFormatting, TextVerticalAnchor, TextWeight,
    };

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

    fn alpha_centroid(pixels: &[u8], width: usize) -> (f32, f32) {
        let mut weight = 0.0f32;
        let mut x_sum = 0.0f32;
        let mut y_sum = 0.0f32;
        for (index, pixel) in pixels.as_chunks::<4>().0.iter().enumerate() {
            let alpha = f32::from(pixel[3]);
            if alpha == 0.0 {
                continue;
            }
            weight += alpha;
            x_sum += (index % width) as f32 * alpha;
            y_sum += (index / width) as f32 * alpha;
        }
        (x_sum / weight, y_sum / weight)
    }

    #[test]
    fn the_vertical_anchor_and_offset_move_the_block() {
        let mut rasterizer = TextRasterizer::new();
        let centered = TextFormatting::default();
        let bottom = TextFormatting {
            anchor: TextVerticalAnchor::Bottom,
            ..TextFormatting::default()
        };
        let shifted = TextFormatting {
            anchor: TextVerticalAnchor::Bottom,
            offset_y_basis_points: -2_000,
            ..TextFormatting::default()
        };
        let center = rasterizer.rasterize("A", centered, 160, 120).unwrap();
        let low = rasterizer.rasterize("A", bottom, 160, 120).unwrap();
        let raised = rasterizer.rasterize("A", shifted, 160, 120).unwrap();
        let center_y = alpha_centroid(&center, 160).1;
        let low_y = alpha_centroid(&low, 160).1;
        let raised_y = alpha_centroid(&raised, 160).1;
        assert!(low_y > center_y + 10.0, "{low_y} vs {center_y}");
        assert!(raised_y < low_y - 10.0, "{raised_y} vs {low_y}");
    }

    #[test]
    fn a_background_box_paints_behind_the_glyphs_and_opacity_scales_alpha() {
        let mut rasterizer = TextRasterizer::new();
        let formatting = TextFormatting {
            anchor: TextVerticalAnchor::Bottom,
            background: Some(or_core::TextBackground {
                color: TextColor {
                    red: 0,
                    green: 0,
                    blue: 0,
                    alpha: 255,
                },
                padding_milli_points: 12_000,
            }),
            opacity_basis_points: 5_000,
            ..TextFormatting::default()
        };
        let pixels = rasterizer.rasterize("A", formatting, 160, 120).unwrap();
        let all = pixels.as_chunks::<4>().0;
        let background = all
            .iter()
            .find(|pixel| pixel[3] > 0 && pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0)
            .expect("the background box paints");
        assert_eq!(background[3], 127);
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
