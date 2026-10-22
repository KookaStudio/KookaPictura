//! Caret geometry for the Type tools: where the caret stands at every
//! character boundary of the text, from the same lines the render sets (the
//! hard lines of point type, the wrapped lines of paragraph type), so a click
//! lands between the letters it is drawn between; and the baseline guide drawn
//! under each line while typing. Ported from photorust's
//! `CanvasView::typeCaretRect` / `typeFlowOffset`, which measured with Qt's
//! font metrics; here the spec's face measures (`fonts::face_for`).

use pictura_core::TypeSpec;

use crate::fonts::face_for;
use crate::type_layer::{align_factor, flat_rect, leading_px, map, text_box_any};
use crate::type_paragraph::{line_advance, vertical_cell, visual_lines};
use crate::type_style::shape_styled;

/// One laid-out line's caret segments, one per character boundary (its end
/// included), mapped to document pixels.
struct CaretLine {
    chars: Vec<char>,
    stops: Vec<[f64; 4]>,
    /// A wrap inside a hard line: its end boundary is the next line's start.
    soft: bool,
    /// Inside the box (always, for point type); overflow lines are not.
    visible: bool,
}

/// Every laid-out line's caret segments, and where the baseline crosses a
/// segment as a fraction from its first point (the centre for vertical type).
fn caret_lines(spec: &TypeSpec) -> Option<(Vec<CaretLine>, f64)> {
    let bundled = face_for(&spec.character.font_family)?;
    let (left, top, width, height) = text_box_any(spec)?;
    let rect = flat_rect(left, top, width, height);
    let (rl, rt) = (rect.left as f32, rect.top as f32);
    // Lines align across the box for paragraph type, else across the rect.
    let (aw, ah) = match spec.box_bounds {
        Some(b) => ((b[2] - b[0]) as f32, (b[3] - b[1]) as f32),
        None => (rect.width() as f32, rect.height() as f32),
    };
    let c = &spec.character;
    let size = c.size as f32;
    let ascent = bundled.ascent(size);
    let descent = bundled.descent(size);
    let leading = leading_px(size, c.leading);
    let cell = vertical_cell(bundled, c);
    let factor = align_factor(spec.justification());
    let justify = spec.paragraph.justify;
    let (ox, oy) = spec.origin;
    let at = |a: (f32, f32), b: (f32, f32)| {
        let rel = |p: (f32, f32)| map(spec, f64::from(rl + p.0) - ox, f64::from(rt + p.1) - oy);
        let (p, q) = (rel(a), rel(b));
        [p.0, p.1, q.0, q.1]
    };
    let wrapped = visual_lines(bundled, &spec.text, spec.vertical, spec.box_bounds, c);
    let lines = wrapped
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let chars: Vec<char> = line.text.chars().collect();
            let stops = if spec.vertical {
                let centre = aw - leading * (i as f32 + 0.5);
                let run = (ah - chars.len() as f32 * cell) * factor;
                (0..=chars.len())
                    .map(|k| {
                        let y = run + k as f32 * cell;
                        at((centre - leading / 2.0, y), (centre + leading / 2.0, y))
                    })
                    .collect()
            } else {
                let prefix = |n: usize| chars[..n].iter().collect::<String>();
                let advance = |n: usize| line_advance(bundled, &prefix(n), c);
                let ink = match line.soft {
                    true => line.text.trim_end().chars().count(),
                    false => chars.len(),
                };
                // A justified line is spread: its word spaces take the
                // leftover width and the line fills the column. A line that
                // cannot be spread stays flush left.
                let trimmed = prefix(ink);
                let ink_advance = advance(ink);
                let spread = spec.box_bounds.is_some()
                    && crate::type_justify::spreads(justify)
                    && (line.soft || crate::type_justify::spreads_last(justify));
                let extra = if spread {
                    let shaped = shape_styled(bundled, &trimmed, c);
                    crate::type_justify::space_extra(
                        bundled,
                        &trimmed,
                        crate::type_justify::shaped_spaces(bundled, &shaped),
                        ink_advance,
                        aw,
                    )
                    .unwrap_or(0.0)
                } else {
                    0.0
                };
                // A spread line fills the column, and a line in a justified
                // paragraph that could not be spread stays flush left; the
                // paragraph's last line, or a plainly aligned one, takes the
                // alignment.
                let flush = crate::type_justify::spreads(justify) && line.soft && extra <= 0.0;
                let shift = if extra > 0.0 || flush {
                    0.0
                } else {
                    (aw - ink_advance) * factor
                };
                let baseline = ascent + i as f32 * leading;
                (0..=chars.len())
                    .map(|k| {
                        let spaces = crate::type_justify::spaces(&prefix(k.min(ink)));
                        let x = shift + advance(k) + spaces as f32 * extra;
                        at((x, baseline - ascent), (x, baseline + descent))
                    })
                    .collect()
            };
            CaretLine {
                chars,
                stops,
                soft: line.soft,
                visible: i < wrapped.visible,
            }
        })
        .collect();
    let cross = if spec.vertical {
        0.5
    } else {
        f64::from(ascent / (ascent + descent))
    };
    Some((lines, cross))
}

/// One caret segment per UTF-16 boundary of `spec.text` (`len + 1` of them,
/// the boundary inside a surrogate pair repeating the one before it), each
/// `[x0, y0, x1, y1]` in document pixels after `spec.matrix`. Horizontal type's
/// caret runs ascent to descent across the line at the boundary's pen position
/// (the line shifted by its alignment); vertical type's runs across the column
/// above the boundary's cell. A soft wrap's boundary stands at the start of the
/// next line. Empty for a non-positive size or missing face.
pub fn type_caret_stops(spec: &TypeSpec) -> Vec<[f64; 4]> {
    if spec
        .path
        .as_ref()
        .is_some_and(|path| path.kind == pictura_core::TextPathKind::OnPath)
    {
        return crate::type_path::caret_stops(spec);
    }
    let Some((lines, _)) = caret_lines(spec) else {
        return Vec::new();
    };
    let mut stops = Vec::new();
    for line in &lines {
        let count = if line.soft {
            line.chars.len()
        } else {
            line.chars.len() + 1
        };
        for (k, stop) in line.stops.iter().take(count).enumerate() {
            for _ in 0..line.chars.get(k).map_or(1, |c| c.len_utf16()) {
                stops.push(*stop);
            }
        }
    }
    stops
}

/// The baseline guide under each non-blank visible line, `[x0, y0, x1, y1]` in
/// document pixels after `spec.matrix`, from the line's first caret stop to its
/// last: the baseline for horizontal type, the column's centre axis for
/// vertical type. Empty for blank text, a non-positive size, or a missing face.
pub fn type_baselines(spec: &TypeSpec) -> Vec<[f64; 4]> {
    if spec
        .path
        .as_ref()
        .is_some_and(|path| path.kind == pictura_core::TextPathKind::OnPath)
    {
        return crate::type_path::baselines(spec);
    }
    let Some((lines, t)) = caret_lines(spec) else {
        return Vec::new();
    };
    let cross = |s: &[f64; 4]| (s[0] + (s[2] - s[0]) * t, s[1] + (s[3] - s[1]) * t);
    lines
        .iter()
        .filter(|line| line.visible && line.chars.iter().any(|c| !c.is_whitespace()))
        .filter_map(|line| {
            let (a, b) = (cross(line.stops.first()?), cross(line.stops.last()?));
            Some([a.0, a.1, b.0, b.1])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(text: &str, vertical: bool, justification: u8) -> TypeSpec {
        let mut spec = TypeSpec::new(text, "Liberation Sans", 32.0);
        spec.vertical = vertical;
        spec.origin = (100.0, 80.0);
        spec.paragraph.justify =
            pictura_core::Justify::from_index(justification).unwrap_or_default();
        spec
    }

    #[test]
    fn stops_step_along_lines_and_down_the_columns() {
        let empty = type_caret_stops(&spec("", false, 0));
        assert_eq!(empty.len(), 1);
        assert!((empty[0][0] - 100.0).abs() <= 1.0 && empty[0][1] < 80.0 && empty[0][3] > 80.0);

        // "ab\rc": stops 0..=2 on line one, rising; 3 and 4 on line two.
        let stops = type_caret_stops(&spec("ab\rc", false, 0));
        assert_eq!(stops.len(), 5);
        assert!(stops[0][0] < stops[1][0] && stops[1][0] < stops[2][0]);
        assert!((stops[3][0] - stops[0][0]).abs() < 0.01);
        assert!((stops[3][1] - stops[0][1] - 38.4).abs() < 0.01);

        // Right alignment ends the line at the click.
        let right = type_caret_stops(&spec("ab", false, 1));
        assert!((right[2][0] - 100.0).abs() <= 1.5, "{:?}", right[2]);

        // Vertical: each boundary one cell further down, flat across the column.
        let column = type_caret_stops(&spec("ab", true, 0));
        assert_eq!(column.len(), 3);
        assert_eq!(column[0][1], column[0][3]);
        assert!(column[1][1] > column[0][1] && column[0][0] < 100.0 && column[0][2] > 100.0);

        // A surrogate pair repeats its leading stop; a turn maps every stop.
        assert_eq!(type_caret_stops(&spec("a\u{1F600}", false, 0)).len(), 4);
        let mut turned = spec("ab", false, 0);
        turned.matrix = [0.0, 1.0, -1.0, 0.0];
        let t = type_caret_stops(&turned);
        assert!((t[0][1] - t[0][3]).abs() < 0.01 && t[2][1] > t[0][1]);
    }

    #[test]
    fn the_caret_stands_where_the_spread_glyphs_are() {
        // A justified paragraph spreads its last line too, so the caret's
        // last stop stands at the box's right edge.
        let mut s = spec("aa bb cc", false, 6);
        s.box_bounds = Some([0.0, 0.0, 200.0, 200.0]);
        let stops = type_caret_stops(&s);
        let last = stops.last().expect("stops");
        let right = s.origin.0 + 200.0;
        assert!(
            (last[0] - right).abs() < 0.5,
            "{} is not the box's right edge {right}",
            last[0]
        );
    }

    #[test]
    fn baselines_follow_the_layout() {
        let s = spec("ab\r\rc", false, 0);
        let lines = type_baselines(&s);
        assert_eq!(lines.len(), 2, "the blank line has no guide");
        let stops = type_caret_stops(&s);
        for line in &lines {
            assert!((line[1] - line[3]).abs() < 1e-6, "horizontal");
        }
        assert!((lines[0][0] - 100.0).abs() <= 1.0 && (lines[0][1] - 80.0).abs() <= 1.0);
        assert!(
            (lines[0][2] - stops[2][0]).abs() < 1e-6,
            "ends at the line's last stop"
        );
        assert!(
            (lines[1][1] - lines[0][1] - 2.0 * 38.4).abs() < 0.01,
            "two leadings down"
        );
        assert!((lines[1][2] - stops[5][0]).abs() < 1e-6);
        assert!(type_baselines(&spec("", false, 0)).is_empty());

        let column = type_baselines(&spec("ab", true, 0));
        assert_eq!(column.len(), 1);
        let c = column[0];
        assert!((c[0] - c[2]).abs() < 1e-6 && c[3] > c[1], "down the column");
        let stops = type_caret_stops(&spec("ab", true, 0));
        assert!(
            (c[0] - (stops[0][0] + stops[0][2]) / 2.0).abs() < 1e-6,
            "on the axis"
        );
    }
}
