//! Deterministic text layout over already-shaped glyph runs, plus the
//! font-agnostic glyph rasterizer port and font-policy/provenance records.
//!
//! The engine never shapes text or parses a font: the host supplies glyph ids
//! and advances in font units, and [`layout_lines`] turns them into device
//! positions with pure `f32` arithmetic.

/// One shaped glyph: a font glyph id, its advance, and its x/y offsets, all in
/// font units. `scale` sizes the outline against the run's size (1.0 normally;
/// synthesized small caps draw capitals smaller).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph {
    pub id: u16,
    pub advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub scale: f32,
}

impl ShapedGlyph {
    /// A glyph with no offsets; the shaper's marks override the offset fields.
    pub fn new(id: u16, advance: f32) -> Self {
        Self {
            id,
            advance,
            x_offset: 0.0,
            y_offset: 0.0,
            scale: 1.0,
        }
    }
}

impl Default for ShapedGlyph {
    fn default() -> Self {
        Self::new(0, 0.0)
    }
}

/// Paragraph alignment, mapped from the EngineData justification byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl TextAlign {
    /// The alignment the EngineData paragraph justification byte selects: 0
    /// left, 1 right, 2 center, 3 (Justify Last Left) left, 4 (Justify Last
    /// Right) right, 5 (Justify Last Center) center, 6 (Justify All) left -
    /// every line of a justified-all paragraph is spread, so its alignment
    /// never shows. A byte of 7 or above names no modelled justification and
    /// falls back to left.
    pub fn from_justification(byte: u8) -> TextAlign {
        match byte {
            1 | 4 => TextAlign::Right,
            2 | 5 => TextAlign::Center,
            // 0 left, 3 (Justify Last Left), 6 (Justify All), and the bytes
            // that name no modelled justification.
            _ => TextAlign::Left,
        }
    }

    /// The share of the leftover width a line shifts by: 0 left, 0.5 center,
    /// 1 right.
    pub fn factor(self) -> f32 {
        match self {
            TextAlign::Left => 0.0,
            TextAlign::Center => 0.5,
            TextAlign::Right => 1.0,
        }
    }
}

/// How a shaped run is laid out into device pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutParams {
    pub font_size: f32,
    pub units_per_em: f32,
    pub tracking: f32,
    pub leading: f32,
    pub align: TextAlign,
    pub wrap_width: Option<f32>,
}

/// A glyph positioned in device pixels; `y` is the line baseline adjusted by
/// the shaper's y-offset. `scale` is the shaped glyph's outline scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedGlyph {
    pub id: u16,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
}

/// One laid-out line: its glyphs, total advance, and baseline.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutLine {
    pub glyphs: Vec<PlacedGlyph>,
    pub advance: f32,
    pub baseline: f32,
}

/// The whole layout. `width` is the widest line advance; `height` is
/// `lines.len() * leading`.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    pub lines: Vec<LayoutLine>,
    pub width: f32,
    pub height: f32,
}

fn device_advance(glyph: &ShapedGlyph, params: &LayoutParams) -> f32 {
    device(glyph.advance, params) + params.tracking * params.font_size / 1000.0
}

fn device(value: f32, params: &LayoutParams) -> f32 {
    value * params.font_size / params.units_per_em
}

/// Lay out shaped glyph runs, one inner slice per explicit line.
pub fn layout_lines(lines: &[Vec<ShapedGlyph>], params: &LayoutParams) -> TextLayout {
    let mut out = Vec::with_capacity(lines.len());
    let mut width = 0.0f32;
    for (i, line) in lines.iter().enumerate() {
        let baseline = i as f32 * params.leading;
        let advance: f32 = line.iter().map(|g| device_advance(g, params)).sum();
        if advance > width {
            width = advance;
        }
        let shift = match params.wrap_width {
            Some(wrap) => (wrap - advance) * params.align.factor(),
            None => 0.0,
        };
        let mut glyphs = Vec::with_capacity(line.len());
        let mut x = shift;
        for glyph in line {
            glyphs.push(PlacedGlyph {
                id: glyph.id,
                x: x + device(glyph.x_offset, params),
                y: baseline - device(glyph.y_offset, params),
                scale: glyph.scale,
            });
            x += device_advance(glyph, params);
        }
        out.push(LayoutLine {
            glyphs,
            advance,
            baseline,
        });
    }
    TextLayout {
        lines: out,
        width,
        height: lines.len() as f32 * params.leading,
    }
}

/// A rasterization request: no font handle, only ids and geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterRequest {
    pub glyph: u16,
    pub px_size: f32,
    pub subpixel_x: f32,
    pub subpixel_y: f32,
}

/// An 8-bit coverage mask and its placement relative to the glyph origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphMask {
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
    pub coverage: Vec<u8>,
}

/// A glyph rasterizer backend. Takes no mutable state; `None` when the glyph
/// cannot be rendered.
pub trait Rasterizer {
    fn rasterize(&self, request: &RasterRequest) -> Option<GlyphMask>;
}

/// How missing or substituted fonts are handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontPolicy {
    BundledOnly,
    HostAllowed,
    ExactOrRefuse,
}

/// An auditable record of which font and backend actually rendered text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextProvenance {
    pub requested_family: String,
    pub resolved_family: String,
    pub font_hash: [u8; 32],
    pub backend: String,
    pub backend_version: String,
}

impl TextProvenance {
    pub fn new(
        requested_family: &str,
        resolved_family: &str,
        font_hash: [u8; 32],
        backend: &str,
        backend_version: &str,
    ) -> Self {
        Self {
            requested_family: requested_family.to_string(),
            resolved_family: resolved_family.to_string(),
            font_hash,
            backend: backend.to_string(),
            backend_version: backend_version.to_string(),
        }
    }
}

impl std::fmt::Display for TextProvenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "requested={};resolved={};hash=",
            self.requested_family, self.resolved_family
        )?;
        for byte in &self.font_hash {
            write!(f, "{byte:02x}")?;
        }
        write!(
            f,
            ";backend={};version={}",
            self.backend, self.backend_version
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> LayoutParams {
        LayoutParams {
            font_size: 1000.0,
            units_per_em: 1000.0,
            tracking: 0.0,
            leading: 0.0,
            align: TextAlign::Left,
            wrap_width: None,
        }
    }

    #[test]
    fn advances_accumulate_left_to_right() {
        let lines = vec![vec![ShapedGlyph::new(1, 500.0), ShapedGlyph::new(2, 300.0)]];
        let out = layout_lines(&lines, &params());
        let line = &out.lines[0];
        assert_eq!(line.advance, 800.0);
        assert_eq!(line.glyphs[0].x, 0.0);
        assert_eq!(line.glyphs[1].x, 500.0);
        assert_eq!(
            line.glyphs.iter().map(|g| g.y).collect::<Vec<_>>(),
            [0.0, 0.0]
        );
        assert_eq!(out.width, 800.0);
    }

    #[test]
    fn font_units_scale_to_device_pixels() {
        let mut p = params();
        p.units_per_em = 2000.0;
        let lines = vec![vec![ShapedGlyph::new(1, 500.0)]];
        let out = layout_lines(&lines, &p);
        assert_eq!(out.lines[0].advance, 250.0);
    }

    #[test]
    fn offsets_shift_a_glyph_without_moving_the_pen() {
        let mut first = ShapedGlyph::new(1, 500.0);
        first.x_offset = 100.0;
        first.y_offset = 200.0;
        let lines = vec![vec![first, ShapedGlyph::new(2, 300.0)]];
        let out = layout_lines(&lines, &params());
        let line = &out.lines[0];
        assert_eq!(line.glyphs[0].x, 100.0);
        assert_eq!(line.glyphs[0].y, -200.0);
        assert_eq!(line.glyphs[1].x, 500.0);
        assert_eq!(line.glyphs[1].y, 0.0);
        assert_eq!(line.advance, 800.0);
        assert_eq!(out.width, 800.0);
    }

    #[test]
    fn tracking_includes_per_glyph_tracking() {
        let mut p = params();
        p.tracking = 100.0;
        let lines = vec![vec![ShapedGlyph::new(1, 500.0), ShapedGlyph::new(2, 500.0)]];
        let out = layout_lines(&lines, &p);
        assert_eq!(out.lines[0].advance, 1200.0);
        assert_eq!(out.lines[0].glyphs[1].x, 600.0);
    }

    #[test]
    fn leading_steps_baselines_and_height() {
        let mut p = params();
        p.leading = 20.0;
        let lines = vec![
            vec![ShapedGlyph::new(1, 1000.0)],
            vec![ShapedGlyph::new(1, 1000.0)],
        ];
        let out = layout_lines(&lines, &p);
        assert_eq!(out.lines[0].baseline, 0.0);
        assert_eq!(out.lines[1].baseline, 20.0);
        assert_eq!(out.lines[1].glyphs[0].y, 20.0);
        assert_eq!(out.height, 40.0);
    }

    #[test]
    fn center_alignment_shifts_by_half_the_leftover() {
        let mut p = params();
        p.align = TextAlign::Center;
        p.wrap_width = Some(1000.0);
        let lines = vec![vec![ShapedGlyph::new(1, 600.0)]];
        let out = layout_lines(&lines, &p);
        assert_eq!(out.lines[0].glyphs[0].x, 200.0);
    }

    #[test]
    fn right_alignment_puts_last_glyph_at_wrap_width() {
        let mut p = params();
        p.align = TextAlign::Right;
        p.wrap_width = Some(1000.0);
        let lines = vec![vec![ShapedGlyph::new(1, 400.0), ShapedGlyph::new(2, 200.0)]];
        let out = layout_lines(&lines, &p);
        let last = out.lines[0].glyphs[1];
        assert_eq!(last.x + 200.0, 1000.0);
    }

    #[test]
    fn no_wrap_width_means_no_alignment_shift() {
        let mut p = params();
        p.align = TextAlign::Right;
        let lines = vec![vec![ShapedGlyph::new(1, 600.0)]];
        let out = layout_lines(&lines, &p);
        assert_eq!(out.lines[0].glyphs[0].x, 0.0);
    }

    #[test]
    fn justification_maps_and_unknown_falls_back_to_left() {
        assert_eq!(TextAlign::from_justification(0), TextAlign::Left);
        assert_eq!(TextAlign::from_justification(1), TextAlign::Right);
        assert_eq!(TextAlign::from_justification(2), TextAlign::Center);
        for byte in 7u8..=255 {
            assert_eq!(TextAlign::from_justification(byte), TextAlign::Left);
        }
    }

    #[test]
    fn a_justify_variant_maps_to_its_last_line_alignment() {
        assert_eq!(TextAlign::from_justification(3), TextAlign::Left);
        assert_eq!(TextAlign::from_justification(4), TextAlign::Right);
        assert_eq!(TextAlign::from_justification(5), TextAlign::Center);
        assert_eq!(TextAlign::from_justification(6), TextAlign::Left);
    }

    #[test]
    fn empty_and_blank_inputs_yield_no_glyphs_and_zero_width() {
        let out = layout_lines(&[], &params());
        assert!(out.lines.is_empty());
        assert_eq!(out.width, 0.0);
        assert_eq!(out.height, 0.0);

        let blank = vec![Vec::new()];
        let out = layout_lines(&blank, &params());
        assert_eq!(out.lines.len(), 1);
        assert!(out.lines[0].glyphs.is_empty());
        assert_eq!(out.lines[0].advance, 0.0);
        assert_eq!(out.width, 0.0);
    }

    struct TestRasterizer;

    impl Rasterizer for TestRasterizer {
        fn rasterize(&self, request: &RasterRequest) -> Option<GlyphMask> {
            if request.glyph == 0 {
                return None;
            }
            Some(GlyphMask {
                width: 2,
                height: 3,
                left: -1,
                top: 2,
                coverage: vec![1, 2, 3, 4, 5, 6],
            })
        }
    }

    #[test]
    fn test_rasterizer_returns_coverage_mask_and_none_for_unknown() {
        let rasterizer = TestRasterizer;
        let request = RasterRequest {
            glyph: 5,
            px_size: 16.0,
            subpixel_x: 0.25,
            subpixel_y: -0.5,
        };
        let mask = rasterizer.rasterize(&request).expect("known glyph renders");
        assert_eq!(mask.coverage.len(), (mask.width * mask.height) as usize);
        assert_eq!(mask.coverage.len(), 6);

        let missing = RasterRequest {
            glyph: 0,
            ..request
        };
        assert!(rasterizer.rasterize(&missing).is_none());
    }

    #[test]
    fn provenance_records_substitution_and_hash() {
        let provenance =
            TextProvenance::new("Helvetica", "Nimbus Sans", [0xAB; 32], "fontdue", "1.2.3");
        assert_eq!(provenance.requested_family, "Helvetica");
        assert_eq!(provenance.resolved_family, "Nimbus Sans");
        assert_ne!(provenance.requested_family, provenance.resolved_family);

        let record = provenance.to_string();
        assert_eq!(
            record,
            format!(
                "requested=Helvetica;resolved=Nimbus Sans;hash={};backend=fontdue;version=1.2.3",
                "ab".repeat(32)
            )
        );
    }

    #[test]
    fn the_three_font_policies_are_distinct() {
        assert_ne!(FontPolicy::BundledOnly, FontPolicy::HostAllowed);
        assert_ne!(FontPolicy::HostAllowed, FontPolicy::ExactOrRefuse);
        assert_ne!(FontPolicy::BundledOnly, FontPolicy::ExactOrRefuse);
    }
}
