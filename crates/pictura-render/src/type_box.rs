//! A type layer's untransformed box: the layout box the glyphs are set in, the
//! expanded layer rect that covers a hanging first line, and the placement rect.

use pictura_core::{PsdRect, TypeSpec};

use crate::type_layer::{align_factor, leading_px, type_lines};
use crate::type_style::RunStyle;

/// Where point type lands: its layer rect in document pixels, and its box
/// relative to the origin (left, top, right, bottom) for the `TySh` bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypePlacement {
    pub rect: PsdRect,
    pub bounds: [f64; 4],
}

/// `spec`'s untransformed text box in document pixels: left, top, width,
/// height. `None` for empty text, a non-positive size, or a missing face.
pub(crate) fn text_box(spec: &TypeSpec) -> Option<TextBox> {
    if type_lines(&spec.text)
        .iter()
        .all(|line| line.trim().is_empty())
    {
        return None;
    }
    text_box_any(spec)
}

/// [`text_box`] for any text, blank included (the caret still needs a box).
pub(crate) fn text_box_any(spec: &TypeSpec) -> Option<TextBox> {
    text_box_parts(spec).map(|(_, expanded)| expanded)
}

/// The point type's layout box relative to the origin (left, top, right,
/// bottom), before ink overhang and hanging indents expand it. Conversion to
/// paragraph type uses this box so the glyphs stay where point type set them.
pub(crate) fn type_layout_box(spec: &TypeSpec) -> Option<[f64; 4]> {
    let (layout, _) = text_box_parts(spec)?;
    let (x, y) = (spec.origin.0, spec.origin.1);
    Some([
        f64::from(layout.0) - x,
        f64::from(layout.1) - y,
        f64::from(layout.0 + layout.2) - x,
        f64::from(layout.1 + layout.3) - y,
    ])
}

/// A text box as `(left, top, width, height)` in document pixels.
pub(crate) type TextBox = (f32, f32, f32, f32);

/// `spec`'s text box as `(layout, expanded)`: the layout box the glyphs are set
/// in, and the layer rect that also covers a hanging first line.
fn text_box_parts(spec: &TypeSpec) -> Option<(TextBox, TextBox)> {
    let bundled = crate::fonts::face_for(&spec.character.font_family, &spec.character.font_style)?;
    let size = spec.character.size as f32;
    if size.is_nan() || size <= 0.0 {
        return None;
    }
    // Paragraph type occupies its box, whatever the text inside.
    if let Some(b) = spec.box_bounds {
        let (x, y) = (spec.origin.0, spec.origin.1);
        let box_ = (
            (x + b[0]) as f32,
            (y + b[1]) as f32,
            (b[2] - b[0]) as f32,
            (b[3] - b[1]) as f32,
        );
        return Some((box_, box_));
    }
    let ascent = bundled.ascent(size);
    let descent = bundled.descent(size);
    let leading = leading_px(size, spec.character.leading, spec.paragraph.auto_leading);
    let factor = align_factor(spec.justification());
    let (x, y) = (spec.origin.0 as f32, spec.origin.1 as f32);
    let space_before = spec.paragraph.space_before as f32;
    let wrapped = crate::type_paragraph::visual_lines(
        bundled,
        &spec.text,
        spec.vertical,
        None,
        &spec.character,
        &spec.paragraph,
    );
    if spec.vertical {
        let inline_size = wrapped.inline_size;
        let natural_top = y - inline_size * factor;
        let right = x + leading / 2.0 + space_before;
        let layout = (
            right - wrapped.block_size,
            natural_top,
            wrapped.block_size,
            inline_size,
        );
        let top = natural_top + wrapped.inline_low.min(0.0);
        let bottom = natural_top + wrapped.inline_high.max(inline_size);
        return Some((layout, (layout.0, top, layout.2, bottom - top)));
    }
    let run = RunStyle::of(bundled, &spec.character);
    let inline_size = wrapped.inline_size;
    let natural_left = x - inline_size * factor;
    // A subscript or an underline can hang below the font's descent.
    let drop = (spec.character.baseline_shift as f32 - run.rise).max(0.0);
    let mut below = descent;
    if drop > 0.0 {
        below = below.max(drop + bundled.descent(run.size));
    }
    if let Some((offset, thickness)) = run.underline {
        below = below.max(drop + offset + thickness / 2.0);
    }
    let top = y - ascent - space_before;
    let height = wrapped.block_size - descent + below;
    let layout = (natural_left, top, inline_size, height);
    let left = natural_left + wrapped.inline_low.min(0.0);
    let right = natural_left + wrapped.inline_high.max(inline_size);
    Some((layout, (left, top, right - left, height)))
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
