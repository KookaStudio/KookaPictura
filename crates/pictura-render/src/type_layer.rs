//! The Type tools' engine: place point type, commit it as a type layer, or
//! render its coverage for the Type Mask tools.
//!
//! Point type is laid out from the click (`TypeSpec::origin`). Horizontal type
//! sets lines `1.2 × size` apart, the click on the first baseline; vertical
//! type stacks each line's characters top to bottom in a column, columns
//! running right to left, the click on the first column's centre axis. A
//! committed layer carries both its rendered pixels and an authored `TySh`, so
//! it composites like any pixel layer and reopens as type. Glyphs come from
//! the face registered for the spec's family (`fonts`), else the bundled face;
//! `TySh` names the face's PostScript name. Ported
//! from photorust's Type tool (`CanvasView` typing / commit).
//!
//! ponytail: vertical type sets every character upright on a fixed
//! `ascent + descent` cell — no `vert` alternates, rotated Latin, or tate-chu-yoko.

use pictura_core::{
    AntiAlias, BitDepth, Channel, CharacterAttrs, ColorMode, Document, LayerBlock, LayoutLine,
    LockFlags, PixelBuffer, PlacedGlyph, PsdRect, RasterRequest, TextAlign, TextLayout, TextStyle,
    TypeSpec, TypeTool,
};

use crate::text_render::{blit_glyph, glyph_matrix, layout_type, render_text_buffer, BundledText};
use crate::type_style::{shape_styled, RunStyle};

/// The lines of a `TySh` string: `\r` separates them (`\n` and `\r\n` are
/// accepted too).
pub(crate) fn type_lines(text: &str) -> Vec<String> {
    text.replace("\r\n", "\r")
        .split(['\r', '\n'])
        .map(str::to_owned)
        .collect()
}

/// 0 left / top, 1 right / bottom, 2 centre, as a fraction of the slack: the
/// alignment the paragraph justification byte selects.
pub(crate) fn align_factor(justification: u8) -> f32 {
    TextAlign::from_justification(justification).factor()
}

/// Lay out vertical type inside a `width × height` box: column `j` is centred
/// `leading × (j + ½)` from the right edge, and its characters fall one cell
/// apart, the column aligned top / bottom / centre by `justification`. Each
/// character shapes as `c` styles it (its advance carries the horizontal
/// scale), the vertical scale stretches the cell, and the baseline shift
/// offsets the column along its axis.
pub(crate) fn vertical_layout(
    bundled: &BundledText,
    lines: &[String],
    c: &CharacterAttrs,
    justification: u8,
    width: f32,
    height: f32,
    leading: f32,
) -> TextLayout {
    let baseline_shift = c.baseline_shift as f32;
    let cell = crate::type_paragraph::vertical_cell(bundled, c);
    let factor = align_factor(justification);
    let columns = lines
        .iter()
        .enumerate()
        .map(|(j, line)| {
            let centre = width - leading * (j as f32 + 0.5);
            let count = line.chars().count() as f32;
            let top = (height - count * cell) * factor - baseline_shift;
            let glyphs = line
                .chars()
                .enumerate()
                .flat_map(|(k, ch)| {
                    let shaped = shape_styled(bundled, &ch.to_string(), c);
                    let advance: f32 = shaped.iter().map(|g| g.advance).sum();
                    let mut x = centre - advance / 2.0;
                    let y = top + k as f32 * cell;
                    shaped.into_iter().map(move |g| {
                        let placed = PlacedGlyph {
                            id: g.id,
                            x: x + g.x_offset,
                            y: y - g.y_offset,
                            scale: g.scale,
                        };
                        x += g.advance;
                        placed
                    })
                })
                .collect();
            LayoutLine {
                glyphs,
                advance: count * cell,
                baseline: centre,
            }
        })
        .collect();
    TextLayout {
        lines: columns,
        width,
        height,
    }
}

/// Baseline-to-baseline distance: `Auto` is the CS6 `1.2 × size`, `Fixed` the
/// authored value.
pub(crate) fn leading_px(size: f32, leading: pictura_core::Leading) -> f32 {
    match leading {
        pictura_core::Leading::Auto => size * 1.2,
        pictura_core::Leading::Fixed(value) => value as f32,
    }
}

/// Where point type lands: its layer rect in document pixels, and its box
/// relative to the origin (left, top, right, bottom) for the `TySh` bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypePlacement {
    pub rect: PsdRect,
    pub bounds: [f64; 4],
}

/// `spec`'s untransformed text box in document pixels: left, top, width,
/// height. `None` for empty text, a non-positive size, or a missing face.
fn text_box(spec: &TypeSpec) -> Option<(f32, f32, f32, f32)> {
    if type_lines(&spec.text)
        .iter()
        .all(|line| line.trim().is_empty())
    {
        return None;
    }
    text_box_any(spec)
}

/// [`text_box`] for any text, blank included (the caret still needs a box).
pub(crate) fn text_box_any(spec: &TypeSpec) -> Option<(f32, f32, f32, f32)> {
    let bundled = crate::fonts::face_for(&spec.character.font_family)?;
    let size = spec.character.size as f32;
    let lines = type_lines(&spec.text);
    if size.is_nan() || size <= 0.0 {
        return None;
    }
    // Paragraph type occupies its box, whatever the text inside.
    if let Some(b) = spec.box_bounds {
        let (x, y) = (spec.origin.0, spec.origin.1);
        return Some((
            (x + b[0]) as f32,
            (y + b[1]) as f32,
            (b[2] - b[0]) as f32,
            (b[3] - b[1]) as f32,
        ));
    }
    let ascent = bundled.ascent(size);
    let descent = bundled.descent(size);
    let leading = leading_px(size, spec.character.leading);
    let v = (spec.character.vertical_scale / 100.0) as f32;
    let factor = align_factor(spec.justification());
    let (x, y) = (spec.origin.0 as f32, spec.origin.1 as f32);
    let (left, top, width, height) = if spec.vertical {
        let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let width = lines.len() as f32 * leading;
        let height = longest as f32 * (ascent + descent) * v;
        (
            x - (width - leading / 2.0),
            y - height * factor,
            width,
            height,
        )
    } else {
        let run = RunStyle::of(bundled, &spec.character);
        // Faux Italic leans the ascenders past the last advance.
        let overhang = ascent * run.skew + run.embolden;
        let width = lines
            .iter()
            .map(|line| crate::type_paragraph::line_advance(bundled, line, &spec.character))
            .fold(0.0f32, f32::max)
            + overhang;
        // A subscript or an underline can hang below the font's descent.
        let drop = (spec.character.baseline_shift as f32 - run.rise).max(0.0);
        let mut below = descent;
        if drop > 0.0 {
            below = below.max(drop + bundled.descent(run.size));
        }
        if let Some((offset, thickness)) = run.underline {
            below = below.max(drop + offset + thickness / 2.0);
        }
        let height = (lines.len() - 1) as f32 * leading + ascent + below;
        (x - width * factor, y - ascent, width, height)
    };
    Some((left, top, width, height))
}

/// The untransformed layer rect of a text box: one spare column on the right
/// keeps a glyph's overhang past its advance.
pub(crate) fn flat_rect(left: f32, top: f32, width: f32, height: f32) -> PsdRect {
    PsdRect {
        top: top.floor() as i32,
        left: left.floor() as i32,
        bottom: (top + height).ceil() as i32,
        right: (left + width).ceil() as i32 + 1,
    }
}

/// `(u, v)` relative to the origin, mapped by `spec.matrix` and moved to it.
pub(crate) fn map(spec: &TypeSpec, u: f64, v: f64) -> (f64, f64) {
    let [xx, xy, yx, yy] = spec.matrix;
    (
        spec.origin.0 + xx * u + yx * v,
        spec.origin.1 + xy * u + yy * v,
    )
}

/// Place `spec`: the layer rect bounds the text box mapped by `spec.matrix`
/// about the origin. `None` for empty text, a non-positive size, or a missing
/// face.
pub fn type_placement(spec: &TypeSpec) -> Option<TypePlacement> {
    if spec.path.is_some() {
        return crate::type_path::placement(spec);
    }
    let (left, top, width, height) = text_box(spec)?;
    let (x, y) = (spec.origin.0 as f32, spec.origin.1 as f32);
    let bounds = [
        f64::from(left - x),
        f64::from(top - y),
        f64::from(left + width - x),
        f64::from(top + height - y),
    ];
    if spec.matrix != TypeSpec::IDENTITY {
        let corners = [
            map(spec, bounds[0], bounds[1]),
            map(spec, bounds[2], bounds[1]),
            map(spec, bounds[2], bounds[3]),
            map(spec, bounds[0], bounds[3]),
        ];
        let lo = |f: fn(&(f64, f64)) -> f64| corners.iter().map(f).fold(f64::MAX, f64::min);
        let hi = |f: fn(&(f64, f64)) -> f64| corners.iter().map(f).fold(f64::MIN, f64::max);
        let rect = PsdRect {
            top: lo(|c| c.1).floor() as i32,
            left: lo(|c| c.0).floor() as i32,
            bottom: hi(|c| c.1).ceil() as i32 + 1,
            right: hi(|c| c.0).ceil() as i32 + 1,
        };
        return Some(TypePlacement { rect, bounds });
    }
    Some(TypePlacement {
        rect: flat_rect(left, top, width, height),
        bounds,
    })
}

/// The unauthored view the renderer needs: `spec`'s style without descriptors.
pub(crate) fn render_view(spec: &TypeSpec) -> TypeTool {
    TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, spec.origin.0, spec.origin.1],
        text: spec.text.clone(),
        bounds: [0; 4],
        text_desc: Vec::new(),
        warp_desc: Vec::new(),
        fonts: Vec::new(),
        style: Some(TextStyle {
            font: Some(spec.character.font_family.clone()),
            character: spec.character.clone(),
            paragraph: spec.paragraph.clone(),
            applied_character_style: None,
            applied_paragraph_style: None,
        }),
        vertical: spec.vertical,
        box_bounds: spec.box_bounds,
        path: spec.path.clone(),
    }
}

/// Render `spec` over its placement rect: planar R, G, B, A. Anti-aliasing off
/// thresholds the coverage at half, so every edge pixel is in or out.
pub fn render_type(spec: &TypeSpec) -> Option<(PsdRect, PixelBuffer)> {
    let placement = type_placement(spec)?;
    let rect = placement.rect;
    let mut buffer = if let Some(path) = spec.path.as_ref() {
        match path.kind {
            pictura_core::TextPathKind::OnPath => crate::type_path::render_path(spec, rect)?,
            pictura_core::TextPathKind::Area => crate::type_path::render_area(spec, rect)?,
        }
    } else if spec.matrix == TypeSpec::IDENTITY {
        render_text_buffer(&render_view(spec), rect.width(), rect.height())?
    } else {
        render_mapped(spec, rect)?
    };
    // ponytail: Crisp/Strong/Smooth rasterize on the Sharp smooth path; only
    // None thresholds, matching the previous None/Sharp split.
    if spec.character.anti_alias == AntiAlias::None {
        let plane = buffer.pixel_count();
        for alpha in &mut buffer.data[3 * plane..] {
            *alpha = if *alpha >= 128 { 255 } else { 0 };
        }
    }
    Some((rect, buffer))
}

/// Transformed type: lay the text out in its untransformed box, then map each
/// glyph's pen through `spec.matrix` and rasterize its outline mapped the same
/// way, so scaled or rotated type stays as sharp as unscaled type.
fn render_mapped(spec: &TypeSpec, rect: PsdRect) -> Option<PixelBuffer> {
    let (bundled, layout, unmapped) = flat_layout(spec)?;
    let run = RunStyle::of(bundled, &spec.character);
    let ascent = bundled.ascent(spec.character.size as f32);
    let color = spec.rgba().map(|c| f64::from(c) / 255.0);
    let m = spec.matrix.map(|v| v as f32);
    // A point of the untransformed layout (in the frame glyphs paint in) as a
    // pixel of `rect`.
    let to_rect = |x: f32, y: f32| {
        let u = f64::from(unmapped.left) + f64::from(x) - spec.origin.0;
        let v = f64::from(unmapped.top) + f64::from(y) - spec.origin.1;
        let (dx, dy) = map(spec, u, v);
        (
            (dx - f64::from(rect.left)) as f32,
            (dy - f64::from(rect.top)) as f32,
        )
    };
    let rasterizer = bundled.rasterizer();
    let mut out = PixelBuffer::new(rect.width() as u32, rect.height() as u32, 4);
    let mut painted = false;
    for glyph in layout.lines.iter().flat_map(|line| &line.glyphs) {
        let (fx, fy) = to_rect(glyph.x, glyph.y + ascent);
        let (px, py) = (fx.floor(), fy.floor());
        let request = RasterRequest {
            glyph: glyph.id,
            px_size: run.size,
            subpixel_x: fx - px,
            subpixel_y: py - fy,
        };
        // The pens already carry the character H/V scale (layout_type applies
        // it); compose the same outline transform so glyphs stretch with them.
        let outline = compose(m, glyph_matrix(&run, glyph.scale));
        if let Some(mask) = rasterizer.rasterize_styled(&request, outline, run.embolden) {
            painted |= blit_glyph(&mut out, &mask, px as i32, py as i32, color);
        }
    }
    if run.decorated() {
        for line in crate::type_decor::decorations(&run, &layout, spec.vertical, ascent) {
            painted |= crate::type_decor::fill_mapped(&mut out, line, to_rect, color);
        }
    }
    painted.then_some(out)
}

/// `a · b` for linear parts in the `TySh` order (`xx, xy, yx, yy`).
fn compose(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
    ]
}

/// `spec`'s face, its layout in the untransformed box, and that box's rect.
fn flat_layout(spec: &TypeSpec) -> Option<(&'static BundledText, TextLayout, PsdRect)> {
    let bundled = crate::fonts::face_for(&spec.character.font_family)?;
    let flat = TypeSpec {
        matrix: TypeSpec::IDENTITY,
        ..spec.clone()
    };
    let unmapped = type_placement(&flat)?.rect;
    let view = render_view(&flat);
    let layout = layout_type(bundled, &view, unmapped.width(), unmapped.height())?;
    Some((bundled, layout, unmapped))
}

/// Glyph ids with their pens `(u, v)`, as [`type_pens`] lays them out.
pub(crate) type Pens = Vec<(u16, f64, f64)>;

/// `spec`'s face and glyphs laid out in its untransformed box: each glyph id
/// with its pen (baseline start) relative to the origin, before `spec.matrix`.
/// The pens carry the horizontal scale, kerning, tracking, and alignment.
pub(crate) fn type_pens(spec: &TypeSpec) -> Option<(&'static BundledText, Pens)> {
    let (bundled, layout, unmapped) = flat_layout(spec)?;
    let ascent = bundled.ascent(spec.character.size as f32);
    let pens = layout
        .lines
        .iter()
        .flat_map(|line| &line.glyphs)
        .map(|glyph| {
            (
                glyph.id,
                f64::from(unmapped.left) + f64::from(glyph.x) - spec.origin.0,
                f64::from(unmapped.top) + f64::from(glyph.y + ascent) - spec.origin.1,
            )
        })
        .collect();
    Some((bundled, pens))
}

/// The Type Mask tools' selection: `spec`'s coverage on a `width × height`
/// document plane, clipped to the canvas. `None` when nothing renders.
pub fn type_mask(width: u32, height: u32, spec: &TypeSpec) -> Option<Vec<u8>> {
    let (rect, buffer) = render_type(spec)?;
    let (dw, dh) = (width as i32, height as i32);
    let plane = buffer.pixel_count();
    let mut mask = vec![0u8; width as usize * height as usize];
    for ly in 0..rect.height() {
        let y = rect.top + ly;
        if !(0..dh).contains(&y) {
            continue;
        }
        for lx in 0..rect.width() {
            let x = rect.left + lx;
            if (0..dw).contains(&x) {
                let i = (ly * rect.width() + lx) as usize;
                mask[(y * dw + x) as usize] = buffer.data[3 * plane + i];
            }
        }
    }
    Some(mask)
}

/// A type layer's name: its first line, as CS6 names one, cut at 30 characters.
fn layer_name(text: &str) -> String {
    let first = type_lines(text).into_iter().find(|l| !l.trim().is_empty());
    first.unwrap_or_default().trim().chars().take(30).collect()
}

/// `spec` as a type layer's content: its rect, rendered `0/1/2/-1` channels,
/// and the authored `TySh` view. `existing` is the layer's current text
/// descriptor bytes, whose EngineData the author merges over. `None` when
/// nothing renders.
fn type_content(
    spec: &TypeSpec,
    existing: Option<&[u8]>,
    font_name: Option<&str>,
) -> Option<(PsdRect, Vec<Channel>, TypeTool)> {
    let placement = type_placement(spec)?;
    let (rect, buffer) = render_type(spec)?;
    let tool = pictura_codec::author_type_tool(spec, placement.bounds, existing, font_name);
    let plane = buffer.pixel_count();
    let channels = [0, 1, 2, -1]
        .into_iter()
        .enumerate()
        .map(|(i, id)| Channel {
            id,
            data: buffer.data[i * plane..(i + 1) * plane].to_vec().into(),
        })
        .collect();
    Some((rect, channels, tool))
}

fn rgb8(doc: &Document) -> bool {
    doc.mode == ColorMode::Rgb && doc.depth == BitDepth::Eight
}

/// Commit `spec` as a type layer above `selection_path` (the [`insert_node`]
/// rule): rendered pixels, an authored `TySh`, and CS6's type locks
/// (transparency and pixels). Returns the new path; empty when nothing renders
/// or the document is not 8-bit RGB.
///
/// ponytail: 8-bit RGB only; Grayscale / CMYK / Lab and 16/32-bit documents
/// need the layer's channels in the document's mode and depth.
///
/// [`insert_node`]: crate::document_ops::insert_node
pub fn add_type_layer(doc: &mut Document, selection_path: &str, spec: &TypeSpec) -> String {
    if !rgb8(doc) {
        return String::new();
    }
    let Some((rect, channels, tool)) = type_content(spec, None, None) else {
        return String::new();
    };
    let layer = pictura_core::Layer {
        name: layer_name(&spec.text),
        rect,
        lock: LockFlags::default()
            .with(LockFlags::TRANSPARENCY, true)
            .with(LockFlags::PIXELS, true),
        channels,
        extra_blocks: vec![LayerBlock {
            key: *b"TySh",
            data: pictura_codec::encode_type_tool(&tool),
        }],
        type_tool: Some(tool),
        ..Default::default()
    };
    crate::document_ops::insert_node(doc, selection_path, layer)
}

/// The topmost visible type layer whose rect contains `(x, y)`, searching into
/// visible groups; `None` when the click lands on no type.
pub fn type_layer_at(doc: &Document, x: f64, y: f64) -> Option<String> {
    fn walk(layers: &[pictura_core::Layer], prefix: &str, x: f64, y: f64) -> Option<String> {
        layers.iter().enumerate().rev().find_map(|(i, layer)| {
            if !layer.visible {
                return None;
            }
            let path = if prefix.is_empty() {
                i.to_string()
            } else {
                format!("{prefix}/{i}")
            };
            if layer.is_group {
                return walk(&layer.children, &path, x, y);
            }
            let r = layer.rect;
            let inside = x >= f64::from(r.left)
                && x < f64::from(r.right)
                && y >= f64::from(r.top)
                && y < f64::from(r.bottom);
            (inside && layer.type_tool.as_ref().is_some_and(|t| t.style.is_some())).then_some(path)
        })
    }
    walk(&doc.layers, "", x, y)
}

/// The spec a type layer reopens with: its text, the full character and
/// paragraph attributes, and its orientation, the origin following the layer if
/// it moved since its `TySh` transform was written, and the transform's linear
/// part as `matrix`.
///
/// ponytail: one style for the whole text (the first run's).
pub fn type_layer_spec(layer: &pictura_core::Layer) -> Option<TypeSpec> {
    let tool = layer.type_tool.as_ref()?;
    let style = tool.style.as_ref()?;
    let mut character = style.character.clone();
    let postscript = style.font.as_deref();
    // The EngineData's own `/FontFamily` is authoritative; the registry only
    // normalizes the PostScript name decode falls back to when the family key is
    // absent, in which case `character.font_family` equals `style.font`.
    if character.font_family.is_empty() || Some(character.font_family.as_str()) == postscript {
        character.font_family = postscript
            .and_then(crate::fonts::font_family)
            .or_else(|| style.font.clone())
            .unwrap_or_default();
    }
    let mut spec = TypeSpec {
        text: type_lines(&tool.text).join("\r"),
        vertical: tool.vertical,
        origin: (tool.transform[4], tool.transform[5]),
        matrix: [
            tool.transform[0],
            tool.transform[1],
            tool.transform[2],
            tool.transform[3],
        ],
        box_bounds: tool.box_bounds,
        path: tool.path.clone(),
        character,
        paragraph: style.paragraph.clone(),
        applied_character_style: layer.applied_character_style.clone(),
        applied_paragraph_style: layer.applied_paragraph_style.clone(),
        overrides: layer.type_overrides.clone(),
    };
    if let Some(placed) = type_placement(&spec) {
        let dx = f64::from(layer.rect.left - placed.rect.left);
        let dy = f64::from(layer.rect.top - placed.rect.top);
        spec.origin.0 += dx;
        spec.origin.1 += dy;
        if let Some(path) = spec.path.as_mut() {
            path.subpath.translate(dx, dy);
        }
    }
    Some(spec)
}

/// Re-set the type layer at `path` from `spec` in place: new rect, pixels, and
/// authored `TySh`. A name that was the old text's default
/// follows the new text; a renamed layer keeps its name. False (unchanged) for
/// a missing or non-type layer, blank text, or a document that is not 8-bit RGB.
///
/// ponytail: an opened layer's own `TySh` (several runs, warp, paragraph type)
/// is replaced by the single-run authored one.
pub fn replace_type_layer(doc: &mut Document, path: &str, spec: &TypeSpec) -> bool {
    if !rgb8(doc) {
        return false;
    }
    let Some((existing, font_name)) = crate::resolve_path(doc, path)
        .and_then(|layer| layer.type_tool.as_ref())
        .map(|tool| {
            // Only reuse the layer's PostScript name while its family is the
            // spec's; a family change must author the new face, not rename the
            // run back onto the old entry.
            let name = tool.style.as_ref().and_then(|style| {
                let postscript = style.font.as_deref()?;
                let family = crate::fonts::font_family(postscript)
                    .unwrap_or_else(|| style.character.font_family.clone());
                (family == spec.character.font_family).then(|| postscript.to_string())
            });
            (tool.text_desc.clone(), name)
        })
    else {
        return false;
    };
    let Some((rect, channels, tool)) = type_content(spec, Some(&existing), font_name.as_deref())
    else {
        return false;
    };
    let Some(layer) = crate::resolve_path_mut(doc, path) else {
        return false;
    };
    let Some(old) = layer.type_tool.as_ref() else {
        return false;
    };
    if layer.name == layer_name(&old.text) {
        layer.name = layer_name(&spec.text);
    }
    let applied_character = layer.applied_character_style.clone();
    let applied_paragraph = layer.applied_paragraph_style.clone();
    // `spec.overrides` is authoritative: the bridge records a manual edit as the
    // fields that changed, and every re-authoring caller carries the layer's
    // existing overrides through the spec it built with `type_layer_spec`.
    let overrides = spec.overrides.clone();
    layer.rect = rect;
    layer.channels = channels;
    layer.raw_channels.clear();
    layer.source_channels = None;
    let block = LayerBlock {
        key: *b"TySh",
        data: pictura_codec::encode_type_tool(&tool),
    };
    match layer.extra_blocks.iter_mut().find(|b| &b.key == b"TySh") {
        Some(existing) => *existing = block,
        None => layer.extra_blocks.push(block),
    }
    layer.type_tool = Some(tool);
    layer.applied_character_style = applied_character;
    layer.applied_paragraph_style = applied_paragraph;
    layer.type_overrides = overrides;
    true
}

/// Apply the named saved character (`paragraph` false) or paragraph
/// (`paragraph` true) style to the type layer at `path`: resolve it against the
/// document's stylesheet, keeping the run's manual overrides, and re-set the
/// layer in place. Returns false (unchanged) for a missing or non-type layer or
/// an unknown style name.
pub fn apply_type_style(doc: &mut Document, path: &str, name: &str, paragraph: bool) -> bool {
    let Some(mut spec) = crate::resolve_path(doc, path).and_then(type_layer_spec) else {
        return false;
    };
    let styles = doc.text_styles.clone();
    let known = if paragraph {
        styles.paragraph_style(name).is_some()
    } else {
        styles.character_style(name).is_some()
    };
    if !known {
        return false;
    }
    if paragraph {
        spec.applied_paragraph_style = Some(name.to_string());
    } else {
        spec.applied_character_style = Some(name.to_string());
    }
    let resolved = styles.resolve(
        &spec.overrides,
        spec.applied_character_style.as_deref(),
        spec.applied_paragraph_style.as_deref(),
    );
    spec.character = resolved.character;
    spec.paragraph = resolved.paragraph;
    if !replace_type_layer(doc, path, &spec) {
        return false;
    }
    if let Some(layer) = crate::resolve_path_mut(doc, path) {
        layer.applied_character_style = spec.applied_character_style.clone();
        layer.applied_paragraph_style = spec.applied_paragraph_style.clone();
        layer.type_overrides = spec.overrides;
    }
    true
}

/// Free Transform / Skew of a type layer as type: fold the document-space
/// affine `a` (`xx, xy, yx, yy, tx, ty`, the `TySh` order) into its matrix and
/// origin and re-set it, so the text stays sharp and editable. `None` when the
/// layer is not type this can re-set (no style, a layer mask, or not 8-bit RGB),
/// leaving the caller to resample its pixels; `Some(false)` for a
/// position-locked layer.
///
/// ponytail: a type layer with a layer mask is resampled as pixels.
pub fn transform_type_layer(doc: &mut Document, path: &str, a: [f64; 6]) -> Option<bool> {
    let layer = crate::resolve_path(doc, path)?;
    if layer.mask.is_some() || !rgb8(doc) {
        return None;
    }
    if layer.lock.contains(LockFlags::POSITION) {
        return Some(false);
    }
    let mut spec = type_layer_spec(layer)?;
    let (ox, oy) = spec.origin;
    spec.origin = (a[0] * ox + a[2] * oy + a[4], a[1] * ox + a[3] * oy + a[5]);
    if let Some(text_path) = spec.path.as_mut() {
        // Path text transforms its path geometry; the tangent layout follows
        // the transformed curve. Uniform scale also scales the type size and
        // the arc-length start, so the text keeps its place on the path.
        let scale = (a[0] * a[3] - a[1] * a[2]).abs().sqrt();
        text_path.subpath.transform(a);
        text_path.start *= scale;
        spec.character.size *= scale;
        spec.box_bounds = if text_path.kind == pictura_core::TextPathKind::Area {
            let points = text_path.subpath.flattened(0.25).points;
            let first = *points.first()?;
            let (mut left, mut top, mut right, mut bottom) = (first.0, first.1, first.0, first.1);
            for &(x, y) in &points {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
            spec.origin = (left, top);
            Some([0.0, 0.0, right - left, bottom - top])
        } else {
            None
        };
    } else {
        let [xx, xy, yx, yy] = spec.matrix;
        spec.matrix = [
            a[0] * xx + a[2] * xy,
            a[1] * xx + a[3] * xy,
            a[0] * yx + a[2] * yy,
            a[1] * yx + a[3] * yy,
        ];
    }
    Some(replace_type_layer(doc, path, &spec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        CharacterOverrides, Composer, Justify, KerningMode, Leading, ParagraphOverrides,
        StyleOverrides,
    };

    fn spec(text: &str, vertical: bool, justification: u8) -> TypeSpec {
        let mut spec = TypeSpec::new(text, "Liberation Sans", 32.0);
        spec.vertical = vertical;
        spec.origin = (100.0, 80.0);
        spec.character.fill_color = [1.0, 0.0, 0.0, 1.0];
        spec.paragraph.justify = Justify::from_index(justification).unwrap_or_default();
        spec
    }

    fn alpha_bounds(width: u32, mask: &[u8]) -> (u32, u32, u32, u32) {
        let mut b = (u32::MAX, u32::MAX, 0, 0);
        for (i, _) in mask.iter().enumerate().filter(|(_, &a)| a > 0) {
            let (x, y) = (i as u32 % width, i as u32 / width);
            b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        }
        b
    }

    #[test]
    fn horizontal_type_sits_on_its_baseline_and_follows_alignment() {
        let left = type_placement(&spec("Type", false, 0)).unwrap();
        assert_eq!(left.rect.left, 100);
        assert!(left.rect.top < 80 && left.rect.bottom > 80, "{left:?}");
        let right = type_placement(&spec("Type", false, 1)).unwrap();
        assert!(right.rect.right <= 102 && right.rect.left < 100);
        let centre = type_placement(&spec("Type", false, 2)).unwrap();
        assert!(centre.rect.left < 100 && centre.rect.right > 100);
        let two = type_placement(&spec("Type\rTwo", false, 0)).unwrap();
        assert!(two.rect.height() >= left.rect.height() + 38);
    }

    #[test]
    fn vertical_type_is_taller_than_wide_and_columns_run_leftward() {
        let one = type_placement(&spec("ABCD", true, 0)).unwrap();
        assert!(one.rect.height() > 3 * one.rect.width(), "{one:?}");
        assert!(one.rect.left < 100 && one.rect.right > 100);
        assert_eq!(one.rect.top, 80);
        let two = type_placement(&spec("AB\rCD", true, 0)).unwrap();
        assert_eq!(two.rect.right, one.rect.right);
        assert!(two.rect.left < one.rect.left);
        let mask = type_mask(200, 400, &spec("ABCD", true, 0)).unwrap();
        let (x0, y0, x1, y1) = alpha_bounds(200, &mask);
        assert!(y1 - y0 > 3 * (x1 - x0), "glyphs stack downward");
    }

    #[test]
    fn type_mask_is_coverage_clipped_to_the_canvas() {
        let mask = type_mask(120, 100, &spec("Wide text", false, 0)).unwrap();
        assert_eq!(mask.len(), 120 * 100);
        let (x0, _, x1, y1) = alpha_bounds(120, &mask);
        assert!(x0 >= 100 && x1 == 119 && y1 < 100);
        let mut hard = spec("O", false, 0);
        hard.character.anti_alias = AntiAlias::None;
        let mask = type_mask(200, 200, &hard).unwrap();
        assert!(mask.iter().all(|&a| a == 0 || a == 255));
        assert!(mask.contains(&255));
        assert!(type_mask(200, 200, &spec(" \r ", false, 0)).is_none());
    }

    #[test]
    fn add_type_layer_inserts_a_locked_pixel_backed_type_layer() {
        let mut doc = Document::new(300, 200, ColorMode::Rgb, BitDepth::Eight);
        let path = add_type_layer(&mut doc, "", &spec("Hello\rworld", false, 0));
        assert_eq!(path, "0");
        let layer = &doc.layers[0];
        assert_eq!(layer.name, "Hello");
        assert!(layer.is_type());
        assert_eq!(layer.lock.bits(), 0x03);
        let tool = layer.type_tool.as_ref().unwrap();
        assert_eq!(tool.text, "Hello\rworld");
        assert_eq!(tool.style.as_ref().unwrap().rgba(), [0, 0, 255, 255]);
        let alpha = crate::channel(layer, -1).unwrap();
        let blue = crate::channel(layer, 2).unwrap();
        let painted = alpha.iter().position(|&a| a == 255).expect("opaque ink");
        assert_eq!(blue[painted], 255);
        assert_eq!(crate::channel(layer, 0).unwrap()[painted], 0);

        let mut gray = Document::new(300, 200, ColorMode::Grayscale, BitDepth::Eight);
        assert!(add_type_layer(&mut gray, "", &spec("x", false, 0)).is_empty());
    }

    #[test]
    fn every_text_colour_byte_reopens_unchanged() {
        let mut doc = Document::new(300, 200, ColorMode::Rgb, BitDepth::Eight);
        for v in 0..=255u8 {
            let mut set = spec("I", false, 0);
            set.character.fill_color = [
                1.0,
                f64::from(v) / 255.0,
                f64::from(255 - v) / 255.0,
                f64::from(v / 2) / 255.0,
            ];
            add_type_layer(&mut doc, "", &set);
            let reopened = type_layer_spec(doc.layers.last().unwrap()).unwrap();
            assert_eq!(reopened.rgba(), set.rgba(), "channel value {v}");
        }
    }

    #[test]
    fn a_reopened_layer_re_authors_every_modelled_attribute() {
        let mut doc = Document::new(300, 300, ColorMode::Rgb, BitDepth::Eight);
        let mut s = spec("Attributes", false, 1);
        s.character.tracking = 120.0;
        s.character.horizontal_scale = 50.0;
        s.character.vertical_scale = 200.0;
        s.character.leading = Leading::Fixed(20.0);
        s.character.kerning = KerningMode::Manual(-30);
        s.character.baseline_shift = 3.0;
        s.character.all_caps = true;
        s.character.superscript = true;
        s.character.font_style = "Bold".into();
        s.character.underline = true;
        s.character.strikethrough = true;
        s.character.fractional_widths = false;
        s.paragraph.first_line_indent = 10.0;
        s.paragraph.start_indent = 2.0;
        s.paragraph.end_indent = 1.0;
        s.paragraph.space_before = 4.0;
        s.paragraph.space_after = 8.0;
        s.paragraph.hanging = true;
        s.paragraph.hyphenate = true;
        s.paragraph.composer = Composer::EveryLine;
        s.paragraph.word_spacing = [0.8, 1.1, 1.33];
        s.paragraph.letter_spacing = [-0.1, 0.0, 0.2];
        s.paragraph.glyph_spacing = [0.9, 1.0, 1.1];

        let path = add_type_layer(&mut doc, "", &s);
        assert_eq!(path, "0");
        let reopened = type_layer_spec(&doc.layers[0]).expect("reopens");
        assert_eq!(reopened.character, s.character);
        assert_eq!(reopened.paragraph, s.paragraph);

        // And a re-set keeps them: the merge path must author the full set.
        assert!(replace_type_layer(&mut doc, &path, &reopened));
        let again = type_layer_spec(&doc.layers[0]).expect("reopens");
        assert_eq!(again.character, s.character);
        assert_eq!(again.paragraph, s.paragraph);

        // The other halves of the exclusive caps and baseline pairs, and a
        // different style, round-trip too.
        let mut alt = spec("Alt", false, 0);
        alt.character.small_caps = true;
        alt.character.subscript = true;
        alt.character.font_style = "Italic".into();
        add_type_layer(&mut doc, "", &alt);
        let alt_back = type_layer_spec(doc.layers.last().unwrap()).expect("reopens");
        assert_eq!(alt_back.character, alt.character);
    }

    #[test]
    fn horizontal_scale_narrows_the_rendered_run() {
        let wide = spec("Scale", false, 0);
        let mut narrow = wide.clone();
        narrow.character.horizontal_scale = 50.0;
        let (wide_rect, wide_buf) = render_type(&wide).unwrap();
        let (narrow_rect, narrow_buf) = render_type(&narrow).unwrap();
        assert!(
            narrow_rect.width() < wide_rect.width(),
            "half-scale is narrower"
        );
        assert_ne!(narrow_buf.data, wide_buf.data, "the pixels differ");
    }

    #[test]
    fn fixed_leading_sets_the_line_baselines() {
        let bundled = crate::fonts::face_for("Liberation Sans").unwrap();
        let mut fixed = spec("One\rTwo", false, 0);
        fixed.character.leading = Leading::Fixed(10.0);
        let layout = layout_type(bundled, &render_view(&fixed), 300, 300).unwrap();
        assert_eq!(layout.lines[1].baseline, 10.0);
        let auto = spec("One\rTwo", false, 0);
        let auto_layout = layout_type(bundled, &render_view(&auto), 300, 300).unwrap();
        assert!((auto_layout.lines[1].baseline - 32.0 * 1.2).abs() < 1e-4);
    }

    #[test]
    fn manual_kerning_and_baseline_shift_move_the_run() {
        let plain = spec("Wide", false, 0);
        let mut tight = plain.clone();
        tight.character.kerning = KerningMode::Manual(-500);
        assert!(
            type_placement(&tight).unwrap().rect.width()
                < type_placement(&plain).unwrap().rect.width()
        );

        let mut raised = spec("H", false, 0);
        raised.character.baseline_shift = 5.0;
        let (_, base_buf) = render_type(&spec("H", false, 0)).unwrap();
        let (_, raised_buf) = render_type(&raised).unwrap();
        assert_ne!(
            base_buf.data, raised_buf.data,
            "baseline shift moves the ink"
        );
    }

    #[test]
    fn a_type_layer_reopens_where_it_is_and_edits_in_place() {
        let mut doc = Document::new(300, 200, ColorMode::Rgb, BitDepth::Eight);
        let original = spec("Hello", false, 2);
        let path = add_type_layer(&mut doc, "", &original);
        let r = doc.layers[0].rect;
        assert_eq!(
            type_layer_at(&doc, f64::from(r.left + 2), 75.0).as_deref(),
            Some("0")
        );
        assert_eq!(type_layer_at(&doc, 5.0, 5.0), None);

        // Moved after it was set: the reopened origin follows the pixels.
        doc.layers[0].rect = PsdRect {
            top: r.top + 10,
            left: r.left + 20,
            bottom: r.bottom + 10,
            right: r.right + 20,
        };
        let reopened = type_layer_spec(&doc.layers[0]).unwrap();
        assert_eq!(reopened.text, "Hello");
        assert_eq!(reopened.character.size, 32.0);
        assert_eq!(reopened.rgba(), [0, 0, 255, 255]);
        assert_eq!(reopened.paragraph.justify.index(), 2);
        assert_eq!(reopened.origin, (120.0, 90.0));

        let mut edited = reopened.clone();
        edited.text = "Hello there".into();
        doc.layers[0].visible = false;
        assert!(replace_type_layer(&mut doc, &path, &edited));
        let layer = &doc.layers[0];
        assert_eq!(doc.layers.len(), 1);
        assert!(!layer.visible, "visibility is the caller's");
        assert_eq!(layer.name, "Hello there");
        assert_eq!(layer.type_tool.as_ref().unwrap().text, "Hello there");
        assert_eq!(
            layer
                .extra_blocks
                .iter()
                .filter(|b| &b.key == b"TySh")
                .count(),
            1
        );
        assert!(layer.rect.width() > r.width());
        assert!(!replace_type_layer(&mut doc, "5", &edited));
    }

    /// Soft edge pixels per solid pixel: resampling blurs it, re-setting does not.
    fn softness(layer: &pictura_core::Layer) -> f64 {
        let alpha = crate::channel(layer, -1).unwrap();
        let soft = alpha.iter().filter(|&&a| a > 0 && a < 255).count();
        let solid = alpha.iter().filter(|&&a| a == 255).count();
        soft as f64 / solid.max(1) as f64
    }

    #[test]
    fn free_transform_re_sets_type_sharp_and_editable() {
        use crate::document_ops::{transform_layer, transform_layer_quad, LayerTransform};
        let mut doc = Document::new(600, 600, ColorMode::Rgb, BitDepth::Eight);
        let path = add_type_layer(&mut doc, "", &spec("Sharp", false, 0));
        let before = doc.layers[0].clone();
        let scale = LayerTransform {
            scale_x: 3.0,
            scale_y: 3.0,
            angle_radians: 0.0,
            dx: 0.0,
            dy: 0.0,
        };
        assert!(transform_layer(&mut doc, &path, scale));
        let after = &doc.layers[0];
        assert!(after.is_type());
        let tool = after.type_tool.as_ref().unwrap();
        assert_eq!(tool.transform[..4], [3.0, 0.0, 0.0, 3.0]);
        assert!((after.rect.width() - 3 * before.rect.width()).abs() <= 4);
        assert!(
            softness(after) < softness(&before) * 0.6,
            "{} vs {}",
            softness(after),
            softness(&before)
        );

        // Editing keeps the transform; a quarter turn stands the text up.
        let mut edited = type_layer_spec(after).unwrap();
        assert_eq!(edited.matrix, [3.0, 0.0, 0.0, 3.0]);
        edited.text = "Sharper".into();
        assert!(replace_type_layer(&mut doc, &path, &edited));
        assert_eq!(doc.layers[0].type_tool.as_ref().unwrap().transform[0], 3.0);
        let turn = LayerTransform {
            scale_x: 1.0,
            scale_y: 1.0,
            angle_radians: std::f64::consts::FRAC_PI_2,
            dx: 0.0,
            dy: 0.0,
        };
        let wide = doc.layers[0].rect;
        assert!(transform_layer(&mut doc, &path, turn));
        let tall = doc.layers[0].rect;
        assert!(tall.height() > tall.width() && (tall.height() - wide.width()).abs() <= 4);

        // Skew is affine and stays type; a position lock refuses.
        let r = doc.layers[0].rect;
        let (l, t, rr, b) = (
            f64::from(r.left),
            f64::from(r.top),
            f64::from(r.right),
            f64::from(r.bottom),
        );
        let skew = [(l + 20.0, t), (rr + 20.0, t), (rr, b), (l, b)];
        assert!(transform_layer_quad(&mut doc, &path, skew));
        let tool = doc.layers[0].type_tool.as_ref().unwrap();
        assert!(doc.layers[0].is_type() && tool.transform[2].abs() > 0.01);
        doc.layers[0].lock = doc.layers[0].lock.with(LockFlags::POSITION, true);
        assert!(!transform_layer(&mut doc, &path, scale));
    }

    #[test]
    fn applying_a_style_keeps_a_manual_override() {
        let mut doc = Document::new(300, 200, ColorMode::Rgb, BitDepth::Eight);
        let mut s = spec("Style", false, 0);
        s.character.tracking = 120.0;
        let path = add_type_layer(&mut doc, "", &s);
        doc.layers[0].type_overrides = StyleOverrides {
            character: CharacterOverrides {
                tracking: Some(120.0),
                ..Default::default()
            },
            ..Default::default()
        };
        doc.text_styles
            .create_character_style(
                "Heading",
                CharacterOverrides {
                    size: Some(40.0),
                    ..Default::default()
                },
            )
            .unwrap();

        assert!(apply_type_style(&mut doc, &path, "Heading", false));
        let reopened = type_layer_spec(&doc.layers[0]).unwrap();
        assert_eq!(
            reopened.character.tracking, 120.0,
            "manual tracking survives"
        );
        assert_eq!(
            reopened.character.size, 40.0,
            "the applied style's size wins"
        );
        assert_eq!(reopened.applied_character_style.as_deref(), Some("Heading"));

        // A paragraph style leaves the manual override alone too and acts as the
        // character default for the fields it sets.
        doc.text_styles
            .create_paragraph_style(
                "Body",
                CharacterOverrides {
                    size: Some(22.0),
                    tracking: Some(30.0),
                    ..Default::default()
                },
                ParagraphOverrides::default(),
            )
            .unwrap();
        assert!(apply_type_style(&mut doc, &path, "Body", true));
        let reopened = type_layer_spec(&doc.layers[0]).unwrap();
        assert_eq!(
            reopened.character.tracking, 120.0,
            "manual tracking still wins"
        );
        assert_eq!(
            reopened.character.size, 40.0,
            "the applied character style outranks the paragraph default"
        );
    }
}
