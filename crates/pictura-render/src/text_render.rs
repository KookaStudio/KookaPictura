//! The bundled-font glyph backend: shape, rasterize, and materialize a type
//! layer into pixels.
//!
//! The engine (`pictura_core`) owns deterministic layout and the `Rasterizer`
//! port; this module is the pure-Rust backend over a bundled Liberation Sans
//! face. No Qt, no C dependency: `rustybuzz` shapes each line (applying the
//! font's default `kern`/GPOS and GSUB), `swash` rasterizes the glyphs, and
//! `ttf-parser` answers the metrics (glyph count, lookup, ascent).

use pictura_core::{
    layout_lines, Channel, Document, GlyphMask, Layer, LayoutParams, PixelBuffer, PsdRect,
    RasterRequest, Rasterizer, ShapedGlyph, TextAlign, TextLayout, TextProvenance, TypeTool,
};

use crate::type_style::{shape_styled, tracking, RunStyle};
use rustybuzz::ttf_parser;
use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::{Transform, Vector};
use swash::FontRef;
const FONT_BYTES: &[u8] = include_bytes!("../assets/LiberationSans-Regular.ttf");
const RESOLVED_FAMILY: &str = "Liberation Sans";
const BACKEND: &str = "rustybuzz/swash";
// Literal pins of the `rustybuzz` shaper and `swash` rasterizer versions in
// Cargo.toml; bump with the dependencies.
const BACKEND_VERSION: &str = "0.20.1/0.2.10";

/// The bundled face, parsed once.
pub struct BundledText {
    bytes: &'static [u8],
    ttf: ttf_parser::Face<'static>,
    face: rustybuzz::Face<'static>,
    swash: FontRef<'static>,
}

impl BundledText {
    /// Parse the bundled font; `None` if the embedded bytes fail to parse.
    pub fn new() -> Option<Self> {
        Self::from_bytes(FONT_BYTES)
    }

    /// Parse any font's bytes (face 0) the same way as the bundled face.
    pub(crate) fn from_bytes(bytes: &'static [u8]) -> Option<Self> {
        let ttf = ttf_parser::Face::parse(bytes, 0).ok()?;
        let face = rustybuzz::Face::from_slice(bytes, 0)?;
        let swash = FontRef::from_index(bytes, 0)?;
        Some(Self {
            bytes,
            ttf,
            face,
            swash,
        })
    }

    /// The font file this face was parsed from.
    pub(crate) fn bytes(&self) -> &'static [u8] {
        self.bytes
    }

    pub(crate) fn ttf(&self) -> &ttf_parser::Face<'static> {
        &self.ttf
    }

    pub(crate) fn shaper(&self) -> &rustybuzz::Face<'static> {
        &self.face
    }

    /// The face's PostScript name (`name` ID 6), when it has one.
    pub(crate) fn postscript_name(&self) -> Option<String> {
        self.ttf
            .names()
            .into_iter()
            .filter(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
            .find_map(|n| n.to_string())
    }

    /// A rasterizer borrowing this face.
    pub fn rasterizer(&self) -> BundledRasterizer<'_> {
        BundledRasterizer {
            ttf: &self.ttf,
            swash: self.swash,
        }
    }

    /// The glyph id for `c`, or 0 (`.notdef`) when the face has no mapping.
    #[cfg(test)]
    fn glyph_for(&self, c: char) -> u16 {
        self.ttf.glyph_index(c).map_or(0, |g| g.0)
    }

    /// Shape one line into glyph ids and **device-pixel** advances. Lay the
    /// result out with `LayoutParams.units_per_em == font_size`.
    ///
    /// `_tracking` is ignored here: spacing is the layout's job (design D2).
    pub fn shape_line(&self, text: &str, font_size: f32, _tracking: f32) -> Vec<ShapedGlyph> {
        self.shape_with(text, font_size, &[])
    }

    /// [`Self::shape_line`] with explicit OpenType feature settings.
    pub(crate) fn shape_with(
        &self,
        text: &str,
        font_size: f32,
        features: &[rustybuzz::Feature],
    ) -> Vec<ShapedGlyph> {
        let upem = self.face.units_per_em() as f32;
        if upem <= 0.0 {
            return Vec::new();
        }
        let cleaned: String = text.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(&cleaned);
        buffer.set_direction(rustybuzz::Direction::LeftToRight);
        buffer.guess_segment_properties();
        let shaped = rustybuzz::shape(&self.face, features, buffer);
        let scale = font_size / upem;
        shaped
            .glyph_infos()
            .iter()
            .zip(shaped.glyph_positions())
            .filter_map(|(info, pos)| {
                let id = u16::try_from(info.glyph_id).ok()?;
                Some(ShapedGlyph {
                    id,
                    advance: pos.x_advance as f32 * scale,
                    x_offset: pos.x_offset as f32 * scale,
                    y_offset: pos.y_offset as f32 * scale,
                    scale: 1.0,
                })
            })
            .collect()
    }

    /// The ascent at `font_size`, used to place the first baseline inside the
    /// layer rect instead of on its top edge.
    pub(crate) fn ascent(&self, font_size: f32) -> f32 {
        let upem = self.ttf.units_per_em();
        if upem == 0 {
            return font_size;
        }
        self.ttf.ascender() as f32 * font_size / upem as f32
    }

    /// The descent below the baseline at `font_size`, as a positive distance.
    pub(crate) fn descent(&self, font_size: f32) -> f32 {
        let upem = self.ttf.units_per_em();
        if upem == 0 {
            return 0.0;
        }
        -(self.ttf.descender() as f32) * font_size / upem as f32
    }

    /// Record the substitution of `requested` by the bundled face.
    pub fn provenance(&self, requested: &str) -> TextProvenance {
        TextProvenance::new(
            requested,
            RESOLVED_FAMILY,
            font_hash(),
            BACKEND,
            BACKEND_VERSION,
        )
    }
}

/// A glyph rasterizer over the bundled face.
pub struct BundledRasterizer<'a> {
    ttf: &'a ttf_parser::Face<'static>,
    swash: FontRef<'static>,
}

impl BundledRasterizer<'_> {
    /// As [`Rasterizer::rasterize`], the outline first mapped by the y-down
    /// linear part `m` (`xx, xy, yx, yy`, the `TySh` transform's order) and
    /// emboldened by `embolden` pixels (Faux Bold).
    pub(crate) fn rasterize_styled(
        &self,
        request: &RasterRequest,
        m: [f32; 4],
        embolden: f32,
    ) -> Option<GlyphMask> {
        // swash's outline is y-up: conjugating by the y flip negates the
        // off-diagonal terms.
        self.render(
            request,
            Some(Transform::new(m[0], -m[1], -m[2], m[3], 0.0, 0.0)),
            embolden,
        )
    }

    fn render(
        &self,
        request: &RasterRequest,
        transform: Option<Transform>,
        embolden: f32,
    ) -> Option<GlyphMask> {
        // Guard the glyph count so a bogus id cannot reach a panicking lookup.
        if request.glyph >= self.ttf.number_of_glyphs() {
            return None;
        }
        // ponytail: fresh ScaleContext per glyph under `rasterize(&self)`; cache
        // one per thread if profiling shows scaling cost.
        let mut context = ScaleContext::new();
        let mut scaler = context
            .builder(self.swash)
            .size(request.px_size)
            .hint(false)
            .build();
        let mut render = Render::new(&[Source::Outline]);
        render.transform(transform).embolden(embolden);
        render.offset(Vector::new(request.subpixel_x, request.subpixel_y));
        let image = render.render(&mut scaler, request.glyph)?;
        if image.placement.width == 0 || image.placement.height == 0 {
            return None;
        }
        if image.data.len() != (image.placement.width * image.placement.height) as usize {
            return None;
        }
        Some(GlyphMask {
            width: image.placement.width,
            height: image.placement.height,
            left: image.placement.left,
            // swash `placement.top` is the glyph's *top* edge (y-up); `GlyphMask.top`
            // is the bottom edge (`ymin`), matching the painter's
            // `baseline - (height + top)` row formula.
            top: image.placement.top - image.placement.height as i32,
            coverage: image.data,
        })
    }
}

impl Rasterizer for BundledRasterizer<'_> {
    fn rasterize(&self, request: &RasterRequest) -> Option<GlyphMask> {
        self.render(request, None, 0.0)
    }
}

/// A deterministic non-cryptographic FNV-1a digest of the font bytes, padded to
/// a 32-byte identity.
//
// ponytail: not a content/security digest; swap for a real hash if documents
// ever pin a font by digest.
fn font_hash() -> [u8; 32] {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in FONT_BYTES {
        h ^= byte as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut out = [0u8; 32];
    for (i, chunk) in out.chunks_mut(8).enumerate() {
        chunk.copy_from_slice(&h.wrapping_mul(i as u64 + 1).to_le_bytes());
    }
    out
}

/// The bundled face parsed once per process: compositing a proxy-less type
/// layer must not reparse the ~400 KB font every frame.
pub(crate) fn bundled() -> Option<&'static BundledText> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Option<BundledText>> = OnceLock::new();
    CACHE.get_or_init(BundledText::new).as_ref()
}

/// Replace the layer at `path`'s `0/1/2/-1` channels from a packed straight-alpha
/// RGBA buffer of the layer-rect size, drop `TySh`, and clear the type tool.
/// Returns false without mutating for a missing layer, a non-positive rect, or a
/// buffer that is not `width * height * 4` bytes.
pub fn materialize_text_rgba(doc: &mut Document, path: &str, rgba: &[u8]) -> bool {
    let Some(layer) = crate::resolve_path(doc, path) else {
        return false;
    };
    let width = layer.rect.width();
    let height = layer.rect.height();
    if width <= 0 || height <= 0 {
        return false;
    }
    let Some(expected) = (width as usize * height as usize).checked_mul(4) else {
        return false;
    };
    if rgba.len() != expected {
        return false;
    }
    let plane = expected / 4;
    let Some(layer) = crate::resolve_path_mut(doc, path) else {
        return false;
    };
    let mut r = Vec::with_capacity(plane);
    let mut g = Vec::with_capacity(plane);
    let mut b = Vec::with_capacity(plane);
    let mut a = Vec::with_capacity(plane);
    for px in rgba.as_chunks::<4>().0 {
        r.push(px[0]);
        g.push(px[1]);
        b.push(px[2]);
        a.push(px[3]);
    }
    layer.channels = vec![
        Channel {
            id: 0,
            data: r.into(),
        },
        Channel {
            id: 1,
            data: g.into(),
        },
        Channel {
            id: 2,
            data: b.into(),
        },
        Channel {
            id: -1,
            data: a.into(),
        },
    ];
    layer.extra_blocks.retain(|block| &block.key != b"TySh");
    layer.type_tool = None;
    true
}

/// Shape, lay out, and paint a type layer over its rect, then drop the type
/// tool: the layer becomes raster. Returns false without mutating when there is
/// no type tool or style, the rect has no area, or no glyph rendered.
pub fn render_text_layer(doc: &mut Document, path: &str) -> bool {
    let Some(layer) = crate::resolve_path(doc, path) else {
        return false;
    };
    let Some(type_tool) = layer.type_tool.as_ref() else {
        return false;
    };
    let Some(buffer) = render_text_buffer(type_tool, layer.rect.width(), layer.rect.height())
    else {
        return false;
    };
    let plane = buffer.pixel_count();
    let mut rgba = vec![0u8; plane * 4];
    for i in 0..plane {
        rgba[4 * i] = buffer.data[i];
        rgba[4 * i + 1] = buffer.data[plane + i];
        rgba[4 * i + 2] = buffer.data[2 * plane + i];
        rgba[4 * i + 3] = buffer.data[3 * plane + i];
    }
    materialize_text_rgba(doc, path, &rgba)
}

/// Shape, lay out, and paint a type tool into a straight-alpha RGBA buffer of
/// `width × height` (layer-rect-local). `None` when the style is absent, the
/// size is non-positive, or no glyph painted a pixel.
pub(crate) fn render_text_buffer(
    type_tool: &TypeTool,
    width: i32,
    height: i32,
) -> Option<PixelBuffer> {
    let style = type_tool.style.as_ref()?;
    if width <= 0 || height <= 0 {
        return None;
    }
    let font_size = style.character.size as f32;
    let color = style.rgba().map(|c| f64::from(c) / 255.0);
    let bundled = crate::fonts::face_for(style.font.as_deref().unwrap_or_default())?;
    let layout = layout_type(bundled, type_tool, width, height)?;
    let run = RunStyle::of(bundled, &style.character);
    let ascent = bundled.ascent(font_size);
    paint_run(
        bundled,
        &layout,
        [width, height],
        ascent,
        &run,
        color,
        type_tool.vertical,
    )
}

/// Lay a type tool's text out in a `width × height` box (horizontal lines
/// aligned across it, or vertical columns); glyph `y` is the baseline less the
/// ascent, as [`paint_layout`] expects. `None` without a style or any glyph.
///
/// Renders the character attributes: auto/fixed leading, tracking, the kerning
/// modes, horizontal/vertical scale, baseline shift, and the styling
/// `type_style` shapes (OpenType features, caps, super/subscript, Faux Bold);
/// and the left / centre / right alignment.
///
/// Paragraph type (`box_bounds`) wraps to its box and aligns across its
/// width; lines past the box are not laid out (see `type_paragraph`).
///
/// ponytail: paragraph indents, space before/after, hanging punctuation,
/// hyphenation, word/letter/glyph spacing, the composer, and the full
/// justification variants 3-6 are authored to the engine data but not laid out;
/// render them when a paragraph-layout engine exists.
pub(crate) fn layout_type(
    bundled: &BundledText,
    type_tool: &TypeTool,
    width: i32,
    height: i32,
) -> Option<TextLayout> {
    let style = type_tool.style.as_ref()?;
    let c = &style.character;
    let font_size = c.size as f32;
    let leading = crate::type_layer::leading_px(font_size, c.leading);
    let lines = crate::type_paragraph::visual_lines(
        bundled,
        &type_tool.text,
        type_tool.vertical,
        type_tool.box_bounds,
        c,
    )
    .rendered();
    let (width, height) = match type_tool.box_bounds {
        Some(b) => ((b[2] - b[0]) as f32, (b[3] - b[1]) as f32),
        None => (width as f32, height as f32),
    };
    let layout = if type_tool.vertical {
        crate::type_layer::vertical_layout(
            bundled,
            &lines,
            c,
            style.paragraph.justify.index(),
            width,
            height,
            leading,
        )
    } else {
        let shaped: Vec<Vec<ShapedGlyph>> = lines
            .iter()
            .map(|line| shape_styled(bundled, line, c))
            .collect();
        let mut layout = layout_lines(
            &shaped,
            &LayoutParams {
                font_size,
                units_per_em: font_size,
                tracking: tracking(c),
                leading,
                align: TextAlign::from_justification(style.paragraph.justify.index()),
                wrap_width: Some(width),
            },
        );
        // Baseline shift and super/subscript move the run off its baseline
        // (positive is up).
        let rise = RunStyle::of(bundled, c).rise;
        for line in &mut layout.lines {
            for glyph in &mut line.glyphs {
                glyph.y -= rise;
            }
        }
        layout
    };
    if layout.lines.iter().all(|line| line.glyphs.is_empty()) {
        return None;
    }
    Some(layout)
}

/// Paint a laid-out run into a `width × height` buffer: each glyph's baseline
/// is its own (offset-adjusted) `y`, so GPOS marks land on the right row.
#[cfg(test)]
pub(crate) fn paint_layout(
    bundled: &BundledText,
    layout: &TextLayout,
    width: i32,
    height: i32,
    font_size: f32,
    color: [f64; 4],
) -> Option<PixelBuffer> {
    let ascent = bundled.ascent(font_size);
    let run = RunStyle::plain(font_size);
    paint_run(bundled, layout, [width, height], ascent, &run, color, false)
}

/// A glyph's outline transform under `run` (y-down `xx, xy, yx, yy`): the H/V
/// scale times the glyph's own scale, then Faux Italic's slant.
pub(crate) fn glyph_matrix(run: &RunStyle, scale: f32) -> [f32; 4] {
    let (h, v) = (run.h * scale, run.v * scale);
    [h, 0.0, -run.skew * v, v]
}

/// Paint a laid-out styled run into a `size[0] × size[1]` buffer: each glyph
/// at the run's size, mapped by [`glyph_matrix`] and emboldened by Faux Bold,
/// its baseline at its `y + ascent` (the authored size's ascent the layout
/// used); then the underline and strikethrough.
pub(crate) fn paint_run(
    bundled: &BundledText,
    layout: &TextLayout,
    size: [i32; 2],
    ascent: f32,
    run: &RunStyle,
    color: [f64; 4],
    vertical: bool,
) -> Option<PixelBuffer> {
    let mut out = PixelBuffer::new(size[0] as u32, size[1] as u32, 4);
    let rasterizer = bundled.rasterizer();
    let mut painted = false;
    for line in &layout.lines {
        for glyph in &line.glyphs {
            let fx = glyph.x;
            let px = fx.floor();
            let fy = glyph.y + ascent;
            let py = fy.floor();
            let request = RasterRequest {
                glyph: glyph.id,
                px_size: run.size,
                subpixel_x: fx - px,
                // swash offsets the outline in its y-up frame, so a baseline
                // `frac` below `py` needs the opposite-signed offset.
                subpixel_y: py - fy,
            };
            let m = glyph_matrix(run, glyph.scale);
            let mask = if m == [1.0, 0.0, 0.0, 1.0] && run.embolden == 0.0 {
                rasterizer.rasterize(&request)
            } else {
                rasterizer.rasterize_styled(&request, m, run.embolden)
            };
            let Some(mask) = mask else {
                continue;
            };
            painted |= blit_glyph(&mut out, &mask, px as i32, py as i32, color);
        }
    }
    if run.decorated() {
        for rect in crate::type_decor::decorations(run, layout, vertical, ascent) {
            painted |= crate::type_decor::fill_rect(&mut out, rect, color);
        }
    }
    painted.then_some(out)
}

/// Lay one glyph `mask` into planar RGBA `out` with its pen at `(px, py)`:
/// colour where it covers, alpha the max coverage. Whether any pixel painted.
pub(crate) fn blit_glyph(
    out: &mut PixelBuffer,
    mask: &GlyphMask,
    px: i32,
    py: i32,
    color: [f64; 4],
) -> bool {
    let (width, height) = (out.width as i32, out.height as i32);
    let n = out.pixel_count();
    let pen_x = px + mask.left;
    let top = py - (mask.height as i32 + mask.top);
    let mut painted = false;
    for row in 0..mask.height {
        let y = top + row as i32;
        if y < 0 || y >= height {
            continue;
        }
        for col in 0..mask.width {
            let x = pen_x + col as i32;
            if x < 0 || x >= width {
                continue;
            }
            let coverage = mask.coverage[(row * mask.width + col) as usize];
            if coverage == 0 {
                continue;
            }
            let index = y as usize * width as usize + x as usize;
            out.data[index] = tint(coverage, color[0]);
            out.data[n + index] = tint(coverage, color[1]);
            out.data[2 * n + index] = tint(coverage, color[2]);
            out.data[3 * n + index] = out.data[3 * n + index].max(coverage);
            painted = true;
        }
    }
    painted
}

/// Composite a proxy-less type layer live: render its text buffer and blend it
/// over the canvas-clipped region with the layer's opacity/mask/blend. `false`
/// when there is no type tool, a colour channel is present, or nothing painted,
/// so the caller's normal path runs.
///
/// ponytail: axis-aligned placement at the layer rect, first-run style — the
/// bundled renderer; add transform/rotation when the type model carries one.
pub(crate) fn composite_type_source(
    canvas: &mut crate::composite::Canvas,
    layer: &Layer,
    doc: &Document,
) -> bool {
    let Some(type_tool) = layer.type_tool.as_ref() else {
        return false;
    };
    if crate::channel(layer, 0).is_some() {
        return false;
    }
    let region = PsdRect {
        top: layer.rect.top.max(canvas.y0()),
        left: layer.rect.left.max(canvas.x0()),
        bottom: layer.rect.bottom.min(canvas.y1()),
        right: layer.rect.right.min(canvas.x1()),
    };
    if region.width() <= 0 || region.height() <= 0 {
        return false;
    }
    let Some(buf) = render_text_buffer(type_tool, layer.rect.width(), layer.rect.height()) else {
        return false;
    };
    let lw = layer.rect.width() as usize;
    let plane = buf.pixel_count();
    let rw = region.width() as usize;
    let rh = region.height() as usize;
    for by in 0..rh {
        for bx in 0..rw {
            let lx = (region.left - layer.rect.left) as usize + bx;
            let ly = (region.top - layer.rect.top) as usize + by;
            let i = ly * lw + lx;
            crate::composite::blend_into(
                canvas,
                layer,
                doc,
                region.left as usize + bx,
                region.top as usize + by,
                [
                    buf.data[i] as f32 / 255.0,
                    buf.data[plane + i] as f32 / 255.0,
                    buf.data[2 * plane + i] as f32 / 255.0,
                ],
                buf.data[3 * plane + i] as f32 / 255.0,
            );
        }
    }
    true
}

/// `coverage/255 * channel` as an 8-bit straight colour sample.
fn tint(coverage: u8, channel: f64) -> u8 {
    (coverage as f64 / 255.0 * channel * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        BitDepth, CharacterAttrs, ColorMode, Layer, LayerBlock, LayoutLine, ParagraphAttrs,
        PlacedGlyph, PsdRect, TextStyle, TypeTool,
    };

    fn bundled() -> BundledText {
        BundledText::new().expect("bundled font parses")
    }

    fn type_layer(style: Option<TextStyle>) -> Layer {
        Layer {
            name: "text".into(),
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 80,
                right: 200,
            },
            extra_blocks: vec![LayerBlock {
                key: *b"TySh",
                data: vec![1, 2, 3],
            }],
            type_tool: Some(TypeTool {
                transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                text: "Hi".into(),
                bounds: [0, 0, 80, 200],
                text_desc: Vec::new(),
                warp_desc: Vec::new(),
                fonts: vec!["Arial".into()],
                style,
                vertical: false,
                box_bounds: None,
                path: None,
            }),
            ..Default::default()
        }
    }

    fn style() -> TextStyle {
        TextStyle {
            font: Some("Arial".into()),
            character: CharacterAttrs {
                size: 48.0,
                fill_color: [1.0, 0.0, 0.0, 0.0],
                ..CharacterAttrs::default()
            },
            paragraph: ParagraphAttrs::default(),
            ..TextStyle::default()
        }
    }

    fn doc_with(layer: Layer) -> Document {
        let mut doc = Document::new(200, 80, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
        doc
    }

    #[test]
    fn rasterizer_returns_coverage_of_width_times_height() {
        let bundled = bundled();
        let index = bundled.glyph_for('A');
        assert_ne!(index, 0, "'A' is in Liberation Sans");
        let mask = bundled
            .rasterizer()
            .rasterize(&RasterRequest {
                glyph: index,
                px_size: 48.0,
                subpixel_x: 0.0,
                subpixel_y: 0.0,
            })
            .expect("'A' rasterizes");
        assert_seeded(&mask);
    }

    fn assert_seeded(mask: &GlyphMask) {
        assert_eq!(mask.coverage.len(), (mask.width * mask.height) as usize);
        assert!(
            mask.coverage.iter().any(|&c| c > 0),
            "coverage is non-empty"
        );
    }

    #[test]
    fn rasterizer_refuses_an_absent_index_without_panicking() {
        let bundled = bundled();
        let mask = bundled.rasterizer().rasterize(&RasterRequest {
            glyph: u16::MAX,
            px_size: 48.0,
            subpixel_x: 0.0,
            subpixel_y: 0.0,
        });
        assert!(mask.is_none());
    }

    #[test]
    fn rasterizer_subpixel_offset_changes_the_mask() {
        let bundled = bundled();
        let index = bundled.glyph_for('A');
        let render = |subpixel_x: f32| {
            bundled
                .rasterizer()
                .rasterize(&RasterRequest {
                    glyph: index,
                    px_size: 48.0,
                    subpixel_x,
                    subpixel_y: 0.0,
                })
                .expect("'A' rasterizes")
        };
        assert_ne!(
            render(0.0),
            render(0.5),
            "the half-phase mask differs from the integer phase"
        );
    }

    fn paint_glyph_at(x: f32) -> PixelBuffer {
        let bundled = bundled();
        let id = bundled.glyph_for('A');
        let layout = TextLayout {
            lines: vec![LayoutLine {
                glyphs: vec![PlacedGlyph {
                    id,
                    x,
                    y: 0.0,
                    scale: 1.0,
                }],
                advance: 0.0,
                baseline: 0.0,
            }],
            width: 0.0,
            height: 0.0,
        };
        paint_layout(&bundled, &layout, 200, 120, 48.0, [0.0, 0.0, 0.0, 1.0]).expect("paints")
    }

    #[test]
    fn fractional_pen_is_not_rounded_away() {
        let half = paint_glyph_at(12.5);
        assert_ne!(half, paint_glyph_at(12.0), "12.5 differs from 12.0");
        assert_ne!(half, paint_glyph_at(13.0), "12.5 differs from 13.0");
    }

    #[test]
    fn shape_line_maps_two_glyphs_with_nonzero_advances() {
        let glyphs = bundled().shape_line("AB", 48.0, 0.0);
        assert_eq!(glyphs.len(), 2);
        assert!(glyphs.iter().all(|g| g.advance > 0.0));
    }

    #[test]
    fn shape_line_plain_latin_has_zero_offsets() {
        let glyphs = bundled().shape_line("AB", 48.0, 0.0);
        assert!(glyphs
            .iter()
            .all(|g| g.x_offset == 0.0 && g.y_offset == 0.0));
    }

    #[test]
    fn y_offset_paints_a_shifted_row() {
        let bundled = bundled();
        let font_size = 48.0;
        let id = bundled.glyph_for('A');
        assert_ne!(id, 0);
        let params = LayoutParams {
            font_size,
            units_per_em: font_size,
            tracking: 0.0,
            leading: font_size * 1.2,
            align: TextAlign::Left,
            wrap_width: None,
        };
        let plain = layout_lines(&[vec![ShapedGlyph::new(id, 0.0)]], &params);
        let mut raised = ShapedGlyph::new(id, 0.0);
        raised.y_offset = 5.0;
        let marked = layout_lines(&[vec![raised]], &params);

        let width = 200;
        let height = 120;
        let color = [0.0, 0.0, 0.0, 1.0];
        let plain_buf =
            paint_layout(&bundled, &plain, width, height, font_size, color).expect("plain paints");
        let marked_buf =
            paint_layout(&bundled, &marked, width, height, font_size, color).expect("mark paints");

        let top_row = |buf: &PixelBuffer| {
            let plane = buf.pixel_count();
            let w = width as usize;
            (0..height as usize)
                .find(|&row| (0..w).any(|col| buf.data[3 * plane + row * w + col] > 0))
                .expect("row painted")
        };
        assert_eq!(top_row(&plain_buf), top_row(&marked_buf) + 5);
    }

    #[test]
    fn kerning_reduces_a_kerned_pair_but_not_an_unkerned_control() {
        let bundled = bundled();
        let size = 48.0;
        let total = |glyphs: &[ShapedGlyph]| glyphs.iter().map(|g| g.advance).sum::<f32>();
        let pair_total = |pair: &str| total(&bundled.shape_line(pair, size, 0.0));
        let alone = |pair: &str| {
            pair.chars()
                .map(|c| total(&bundled.shape_line(&c.to_string(), size, 0.0)))
                .sum::<f32>()
        };

        let candidates = ["AV", "AW", "To", "Va", "Ya", "We", "LT", "Ty"];
        let pair = candidates
            .into_iter()
            .find(|pair| pair_total(pair) < alone(pair))
            .expect("Liberation Sans kerns at least one known pair");
        assert!(
            pair_total(pair) < alone(pair),
            "{pair} kerns below the glyphs shaped alone"
        );

        let control = "AA";
        assert!(
            pair_total(control) >= alone(control) - 1e-4,
            "unkerned {control} is not reduced"
        );
    }

    #[test]
    fn render_text_layer_paints_and_clears_the_type_tool() {
        let mut doc = doc_with(type_layer(Some(style())));
        assert!(render_text_layer(&mut doc, "0"));
        let layer = crate::resolve_path(&doc, "0").unwrap();
        assert!(layer.type_tool.is_none(), "type tool is cleared");
        assert!(
            layer.extra_blocks.iter().all(|b| &b.key != b"TySh"),
            "TySh block is gone"
        );
        let alpha = crate::channel(layer, -1).expect("alpha channel");
        assert!(alpha.iter().any(|&a| a > 0), "non-empty alpha coverage");
        assert_eq!(alpha.len(), 200 * 80);
    }

    #[test]
    fn render_text_layer_refuses_a_missing_style_without_mutating() {
        let mut doc = doc_with(type_layer(None));
        let before = doc.clone();
        assert!(!render_text_layer(&mut doc, "0"));
        assert_eq!(doc, before, "refusal leaves the document unchanged");
    }

    #[test]
    fn materialize_text_rgba_refuses_a_wrong_size_buffer_without_mutating() {
        let mut doc = doc_with(type_layer(Some(style())));
        let before = doc.clone();
        assert!(!materialize_text_rgba(&mut doc, "0", &[0u8; 10]));
        assert_eq!(doc, before, "refusal leaves the document unchanged");
    }

    #[test]
    fn materialize_text_rgba_refuses_a_missing_layer() {
        let mut doc = doc_with(type_layer(Some(style())));
        assert!(!materialize_text_rgba(&mut doc, "9", &[]));
    }

    #[test]
    fn materialize_text_rgba_sets_channels_and_clears_the_type_tool() {
        let mut doc = doc_with(type_layer(Some(style())));
        let n = 200 * 80;
        let mut rgba = vec![0u8; n * 4];
        rgba[0] = 10;
        rgba[1] = 20;
        rgba[2] = 30;
        rgba[3] = 40;
        assert!(materialize_text_rgba(&mut doc, "0", &rgba));
        let layer = crate::resolve_path(&doc, "0").unwrap();
        assert!(layer.type_tool.is_none(), "type tool is cleared");
        assert!(
            layer.extra_blocks.iter().all(|b| &b.key != b"TySh"),
            "TySh block is gone"
        );
        assert_eq!(crate::channel(layer, 0).unwrap()[0], 10);
        assert_eq!(crate::channel(layer, 1).unwrap()[0], 20);
        assert_eq!(crate::channel(layer, 2).unwrap()[0], 30);
        assert_eq!(crate::channel(layer, -1).unwrap()[0], 40);
    }

    #[test]
    fn provenance_records_the_bundled_substitution() {
        let provenance = bundled().provenance("Arial");
        assert_eq!(provenance.requested_family, "Arial");
        assert_eq!(provenance.resolved_family, "Liberation Sans");
        assert_eq!(provenance.backend, "rustybuzz/swash");
        assert_ne!(provenance.font_hash, [0u8; 32]);
    }

    #[test]
    fn vertical_subpixel_tracks_the_fractional_baseline() {
        let bundled = bundled();
        let id = bundled.glyph_for('H');
        let paint = |dy: f32| {
            let layout = TextLayout {
                lines: vec![LayoutLine {
                    glyphs: vec![PlacedGlyph {
                        id,
                        x: 20.0,
                        y: dy,
                        scale: 1.0,
                    }],
                    advance: 0.0,
                    baseline: 0.0,
                }],
                width: 0.0,
                height: 0.0,
            };
            paint_layout(&bundled, &layout, 200, 120, 48.0, [0.0, 0.0, 0.0, 1.0]).expect("paints")
        };
        let centroid = |buf: &PixelBuffer| {
            let plane = buf.pixel_count();
            let (w, h) = (200usize, 120usize);
            let (mut sum, mut weight) = (0.0f64, 0.0f64);
            for row in 0..h {
                for col in 0..w {
                    let a = buf.data[3 * plane + row * w + col] as f64;
                    sum += a * row as f64;
                    weight += a;
                }
            }
            sum / weight
        };
        let base = centroid(&paint(0.0));
        let half = centroid(&paint(0.5));
        let one = centroid(&paint(1.0));
        assert!(
            (one - base - 1.0).abs() < 0.02,
            "a full baseline shift moves the centroid one row: {base} -> {one}"
        );
        assert!(
            (half - base - 0.5).abs() < 0.05,
            "a half baseline shift moves the centroid half a row: {base} -> {half}"
        );
    }
}
