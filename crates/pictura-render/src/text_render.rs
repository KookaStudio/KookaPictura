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
    RasterRequest, Rasterizer, ShapedGlyph, TextAlign, TextDirection, TextLayout, TextProvenance,
    TypeTool,
};

use crate::type_style::{shape_styled_faces, tracking_step, RunStyle};
use rustybuzz::ttf_parser;
use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::{Transform, Vector};
use swash::FontRef;
use unicode_bidi::{BidiInfo, Level};
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

    pub(crate) fn swash(&self) -> FontRef<'static> {
        self.swash
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

    /// Whether the face has a glyph for `c`.
    pub(crate) fn has_glyph(&self, c: char) -> bool {
        self.ttf.glyph_index(c).is_some()
    }

    /// The face's space glyph, when it has one: the glyph a word space
    /// shapes to, and the one a justified line widens.
    pub(crate) fn space_glyph(&self) -> Option<u16> {
        self.ttf.glyph_index(' ').map(|g| g.0)
    }

    /// Shape one line into glyph ids and **device-pixel** advances. Lay the
    /// result out with `LayoutParams.units_per_em == font_size`.
    ///
    /// `_tracking` is ignored here: spacing is the layout's job (design D2).
    pub fn shape_line(&self, text: &str, font_size: f32, _tracking: f32) -> Vec<ShapedGlyph> {
        self.shape_with(text, font_size, &[], rustybuzz::Direction::LeftToRight)
    }

    /// [`Self::shape_line`] with explicit OpenType feature settings.
    pub(crate) fn shape_with(
        &self,
        text: &str,
        font_size: f32,
        features: &[rustybuzz::Feature],
        direction: rustybuzz::Direction,
    ) -> Vec<ShapedGlyph> {
        let upem = self.face.units_per_em() as f32;
        if upem <= 0.0 {
            return Vec::new();
        }
        let cleaned: String = text.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(&cleaned);
        buffer.set_direction(direction);
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
                    x_scale: 1.0,
                    face: 0,
                    cluster: info.cluster,
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
/// The alpha bakes the text gamma curve: live type curves coverage × opacity ×
/// Fill, so each sample stores `curve(a·k) / k` for the layer's `k` = opacity ×
/// Fill, and the raster composites at that opacity as the type did. A later
/// opacity change then scales the baked alpha linearly, as for any pixel layer.
/// Returns false without mutating for a missing layer, a non-positive rect, or
/// a buffer that is not `width * height * 4` bytes.
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
    let k = f32::from(layer.opacity) / 255.0 * f32::from(layer.fill) / 255.0;
    let baked: [u8; 256] = std::array::from_fn(|v| {
        let coverage = v as f32 / 255.0;
        match crate::text_gamma::current_curve() {
            Some(curve) if k > 0.0 => crate::composite::to_u8(curve.apply(coverage * k) / k),
            Some(curve) => crate::composite::to_u8(curve.apply(coverage)),
            None => v as u8,
        }
    });
    for px in rgba.as_chunks::<4>().0 {
        r.push(px[0]);
        g.push(px[1]);
        b.push(px[2]);
        a.push(baked[usize::from(px[3])]);
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
    let bundled = crate::fonts::face_for(
        style.font.as_deref().unwrap_or_default(),
        &style.character.font_style,
    )?;
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
/// width; lines past the box are not laid out (see `type_paragraph`). A
/// justified paragraph spreads its word spaces to the box's width, except for
/// the paragraph's last line, which its justification aligns (see
/// `type_justify`).
///
/// ponytail: paragraph indents, space before/after, hanging punctuation,
/// hyphenation, and the composer are authored to the engine data but not laid
/// out; render them when a paragraph-layout engine exists.
pub(crate) fn layout_type(
    bundled: &BundledText,
    type_tool: &TypeTool,
    width: i32,
    height: i32,
) -> Option<TextLayout> {
    let style = type_tool.style.as_ref()?;
    let c = &style.character;
    let font_size = c.size as f32;
    let leading = crate::type_layer::leading_px(font_size, c.leading, style.paragraph.auto_leading);
    let justify = style.paragraph.justify;
    let lines = crate::type_paragraph::visual_lines(
        bundled,
        &type_tool.text,
        type_tool.vertical,
        type_tool.box_bounds,
        c,
        &style.paragraph,
    )
    .rendered_paragraphs();
    let (width, height) = match type_tool.box_bounds {
        Some(b) => ((b[2] - b[0]) as f32, (b[3] - b[1]) as f32),
        None => (width as f32, height as f32),
    };
    let layout = if type_tool.vertical {
        crate::type_layer::vertical_layout(
            bundled,
            &lines
                .iter()
                .map(|(text, _)| text.clone())
                .collect::<Vec<_>>(),
            c,
            justify.index(),
            width,
            height,
            leading,
        )
    } else {
        let faces = crate::fonts::faces_in_order(bundled);
        let mut shaped: Vec<Vec<ShapedGlyph>> = lines
            .iter()
            .map(|(line, _)| shape_line_directed(&faces, line, c, style.paragraph.direction))
            .collect();
        // A justified line's word spaces take the leftover width, so the
        // line's advance reaches the column and the layout shifts it by zero.
        let column = if type_tool.box_bounds.is_some() {
            width
        } else {
            0.0
        };
        let spread = crate::type_justify::spread_lines(
            bundled,
            &lines,
            &mut shaped,
            justify,
            column,
            tracking_step(c, font_size),
            &style.paragraph,
            font_size,
        );
        let align = TextAlign::from_justification(justify.index());
        let mut layout = layout_lines(
            &shaped,
            &LayoutParams {
                font_size,
                units_per_em: font_size,
                tracking: tracking_step(c, font_size) * 1000.0 / font_size,
                leading,
                align,
                wrap_width: Some(width),
            },
        );
        // A line that could not be spread stays flush left: the layout
        // shifted it by the paragraph's last line's alignment.
        crate::type_justify::flush_left(&mut layout, &lines, &spread, justify, width);
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

/// Shape one visual line in the paragraph's base direction. An LTR line with
/// no RTL characters takes the plain path; otherwise the Unicode Bidi
/// Algorithm splits it into level runs, each run is shaped in its own
/// direction, and the runs are concatenated in visual order.
pub(crate) fn shape_line_directed(
    faces: &[&BundledText],
    text: &str,
    c: &pictura_core::CharacterAttrs,
    base: TextDirection,
) -> Vec<ShapedGlyph> {
    if faces.is_empty() {
        return Vec::new();
    }
    let base_level = if base == TextDirection::Rtl {
        Level::rtl()
    } else {
        Level::ltr()
    };
    let info = BidiInfo::new(text, Some(base_level));
    let Some(paragraph) = info.paragraphs.first() else {
        return shape_styled_faces(faces, text, c, rustybuzz::Direction::LeftToRight);
    };
    if base == TextDirection::Ltr && !info.has_rtl() {
        return shape_styled_faces(faces, text, c, rustybuzz::Direction::LeftToRight);
    }
    let (levels, runs) = info.visual_runs(paragraph, paragraph.range.clone());
    let mut out = Vec::new();
    for run in runs {
        let level = levels.get(run.start).copied().unwrap_or(base_level);
        let direction = if level.is_rtl() {
            rustybuzz::Direction::RightToLeft
        } else {
            rustybuzz::Direction::LeftToRight
        };
        let mut shaped = shape_styled_faces(faces, &text[run.clone()], c, direction);
        for glyph in &mut shaped {
            glyph.cluster = glyph.cluster.saturating_add(run.start as u32);
        }
        out.extend(shaped);
    }
    out
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
    let faces = crate::fonts::faces_in_order(bundled);
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
            let face = faces.get(glyph.face as usize).copied().unwrap_or(bundled);
            let rasterizer = face.rasterizer();
            let mut m = glyph_matrix(run, glyph.scale);
            m[0] *= glyph.x_scale;
            m[2] *= glyph.x_scale;
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
mod tests;
