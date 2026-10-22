//! Paragraph (box) type: the lines a box wraps its text into, the overflow
//! past the box, and the Type menu's point ↔ paragraph conversion.
//!
//! Point type sets its hard lines as they are. A horizontal box breaks each
//! hard line greedily after a run of spaces, before the word that would cross
//! the box width; a word wider than the box breaks between characters. A
//! vertical box breaks each hard line into columns of `⌊height / cell⌋`
//! characters. Lines that do not fit the box are overflow: the layout keeps
//! them for the caret but the render drops them.
//!
//! ponytail: greedy wrap, not Adobe's every-line composer; no hyphenation, and
//! vertical columns break by character count with no kinsoku rules.

use pictura_core::{CharacterAttrs, TypeSpec};

use crate::fonts::face_for;
use crate::text_render::BundledText;
use crate::type_layer::{align_factor, leading_px, map, type_lines, type_placement};
use crate::type_style::{shape_styled, tracking};

/// Float slack: a line measured at exactly the box width still fits it.
const FIT: f32 = 0.01;

/// One laid-out line. `soft` is a wrap inside a hard line, whose end boundary
/// is the next line's start (no character is consumed there).
pub(crate) struct VisualLine {
    pub text: String,
    pub soft: bool,
}

/// The laid-out lines, of which the first `visible` fit the box.
pub(crate) struct Lines {
    pub lines: Vec<VisualLine>,
    pub visible: usize,
}

impl Lines {
    /// The visible lines as the render sets them: a soft-wrapped line drops
    /// its trailing spaces, so alignment sits on its ink.
    pub(crate) fn rendered(&self) -> Vec<String> {
        self.lines[..self.visible]
            .iter()
            .map(|line| match line.soft {
                true => line.text.trim_end().to_owned(),
                false => line.text.clone(),
            })
            .collect()
    }
}

/// `text`'s advance as the layout sets it: the styled run's advance (scale,
/// caps, Faux Bold, kerning mode), plus tracking per glyph.
pub(crate) fn line_advance(bundled: &BundledText, text: &str, c: &CharacterAttrs) -> f32 {
    let size = c.size as f32;
    let glyphs = shape_styled(bundled, text, c);
    glyphs.iter().map(|g| g.advance).sum::<f32>()
        + tracking(c) * size / 1000.0 * glyphs.len() as f32
}

/// The vertical cell one character occupies down a column.
pub(crate) fn vertical_cell(bundled: &BundledText, c: &CharacterAttrs) -> f32 {
    let size = c.size as f32;
    (bundled.ascent(size) + bundled.descent(size)) * (c.vertical_scale / 100.0) as f32
}

/// Break `line` to `width` and append its pieces to `out`.
///
/// ponytail: each candidate break re-shapes its prefix, so a line costs
/// O(n²) shaping in its length (a single long unbroken word most of all).
/// Fine for interactive type; cache per-glyph advances if long paragraphs lag.
fn wrap_line(line: &str, width: f32, measure: &dyn Fn(&str) -> f32, out: &mut Vec<VisualLine>) {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let text = |a: usize, b: usize| chars[a..b].iter().collect::<String>();
    if n == 0 {
        out.push(VisualLine {
            text: String::new(),
            soft: false,
        });
        return;
    }
    let mut start = 0;
    while start < n {
        // The furthest break (after a run of spaces, or the line's end) whose
        // ink fits the width.
        let mut best = None;
        let mut i = start;
        while i < n {
            while i < n && !chars[i].is_whitespace() {
                i += 1;
            }
            let ink_end = i;
            while i < n && chars[i].is_whitespace() {
                i += 1;
            }
            if measure(&text(start, ink_end)) > width + FIT {
                break;
            }
            best = Some(i);
        }
        let end = best.unwrap_or_else(|| {
            let mut end = start + 1;
            while end < n && measure(&text(start, end + 1)) <= width + FIT {
                end += 1;
            }
            end
        });
        out.push(VisualLine {
            text: text(start, end),
            soft: end < n,
        });
        start = end;
    }
}

/// The lines `text` sets in: its hard lines for point type (`box_bounds`
/// `None`), else wrapped to the box with the fitting count in `visible`.
pub(crate) fn visual_lines(
    bundled: &BundledText,
    text: &str,
    vertical: bool,
    box_bounds: Option<[f64; 4]>,
    c: &CharacterAttrs,
) -> Lines {
    let hard = type_lines(text);
    let Some(b) = box_bounds else {
        return Lines {
            visible: hard.len(),
            lines: hard
                .into_iter()
                .map(|text| VisualLine { text, soft: false })
                .collect(),
        };
    };
    let size = c.size as f32;
    let leading = leading_px(size, c.leading);
    let (width, height) = ((b[2] - b[0]) as f32, (b[3] - b[1]) as f32);
    let mut lines = Vec::new();
    let fitting = if vertical {
        let cell = vertical_cell(bundled, c);
        let per = (((height + FIT) / cell).floor() as usize).max(1);
        for line in &hard {
            let chars: Vec<char> = line.chars().collect();
            if chars.is_empty() {
                lines.push(VisualLine {
                    text: String::new(),
                    soft: false,
                });
            }
            let count = chars.chunks(per).count();
            for (k, column) in chars.chunks(per).enumerate() {
                lines.push(VisualLine {
                    text: column.iter().collect(),
                    soft: k + 1 < count,
                });
            }
        }
        ((width + FIT) / leading).floor() as usize
    } else {
        let measure = |s: &str| line_advance(bundled, s, c);
        for line in &hard {
            wrap_line(line, width, &measure, &mut lines);
        }
        let line_height = bundled.ascent(size) + bundled.descent(size);
        if height + FIT < line_height {
            0
        } else {
            ((height + FIT - line_height) / leading).floor() as usize + 1
        }
    };
    Lines {
        visible: fitting.min(lines.len()),
        lines,
    }
}

/// Whether paragraph type has text past its box (the `+` on the box handle).
/// False for point type, a missing face, or a non-positive size.
pub fn type_overflows(spec: &TypeSpec) -> bool {
    if spec
        .path
        .as_ref()
        .is_some_and(|path| path.kind == pictura_core::TextPathKind::OnPath)
    {
        return crate::type_path::layout_path(spec).is_some_and(|layout| layout.overflow);
    }
    let Some(bundled) = face_for(&spec.character.font_family) else {
        return false;
    };
    if spec.box_bounds.is_none() || spec.character.size.is_nan() || spec.character.size <= 0.0 {
        return false;
    }
    let lines = visual_lines(
        bundled,
        &spec.text,
        spec.vertical,
        spec.box_bounds,
        &spec.character,
    );
    lines.visible < lines.lines.len()
}

/// `Type > Convert To Paragraph Text`: point type boxed at its own text
/// bounds, so no glyph moves. `None` for paragraph type or blank text.
pub fn type_to_paragraph(spec: &TypeSpec) -> Option<TypeSpec> {
    if spec.path.is_some() || spec.box_bounds.is_some() {
        return None;
    }
    let bounds = type_placement(spec)?.bounds;
    Some(TypeSpec {
        box_bounds: Some(bounds),
        ..spec.clone()
    })
}

/// `Type > Convert To Point Text`: each visible line ends in a return and the
/// overflow is dropped, as CS6 does; the origin moves to where point type
/// sets the same first line. `None` for point type or a missing face.
pub fn type_to_point(spec: &TypeSpec) -> Option<TypeSpec> {
    if spec.path.is_some() {
        return None;
    }
    let b = spec.box_bounds?;
    let bundled = face_for(&spec.character.font_family)?;
    let c = &spec.character;
    let lines = visual_lines(bundled, &spec.text, spec.vertical, Some(b), c);
    let size = c.size as f32;
    let factor = f64::from(align_factor(spec.justification()));
    let (u, v) = if spec.vertical {
        let leading = f64::from(leading_px(size, c.leading));
        (b[2] - leading / 2.0, b[1] + (b[3] - b[1]) * factor)
    } else {
        (
            b[0] + (b[2] - b[0]) * factor,
            b[1] + f64::from(bundled.ascent(size)),
        )
    };
    Some(TypeSpec {
        text: lines.rendered().join("\r"),
        origin: map(spec, u, v),
        box_bounds: None,
        ..spec.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_type;

    fn spec(text: &str) -> TypeSpec {
        let mut spec = TypeSpec::new(text, "Liberation Sans", 20.0);
        spec.origin = (30.0, 40.0);
        spec.character.fill_color = [1.0, 0.0, 0.0, 1.0];
        spec
    }

    fn boxed(text: &str, w: f64, h: f64) -> TypeSpec {
        TypeSpec {
            box_bounds: Some([0.0, 0.0, w, h]),
            ..spec(text)
        }
    }

    fn lines_of(spec: &TypeSpec) -> Lines {
        let bundled = face_for(&spec.character.font_family).unwrap();
        visual_lines(
            bundled,
            &spec.text,
            spec.vertical,
            spec.box_bounds,
            &spec.character,
        )
    }

    /// The rows of a render's alpha plane that have ink.
    fn inked_rows(spec: &TypeSpec) -> Vec<u32> {
        let (_, buffer) = render_type(spec).unwrap();
        let (w, plane) = (buffer.width, buffer.pixel_count());
        let alpha = &buffer.data[3 * plane..];
        (0..buffer.height)
            .filter(|y| (0..w).any(|x| alpha[(y * w + x) as usize] > 0))
            .collect()
    }

    #[test]
    fn paragraph_wraps_words_and_long_words() {
        let bundled = face_for("Liberation Sans").unwrap();
        let s = boxed("the quick brown fox jumps", 70.0, 400.0);
        let lines = lines_of(&s);
        assert!(lines.lines.len() >= 3, "wraps");
        for line in &lines.lines {
            let ink = line.text.trim_end();
            assert!(
                line_advance(bundled, ink, &s.character) <= 70.0 + FIT,
                "{ink:?}"
            );
            assert!(!line.text.starts_with(' '), "breaks after the spaces");
        }
        assert!(lines.lines[0].soft && lines.lines[0].text.ends_with(' '));
        assert!(!lines.lines.last().unwrap().soft);
        let joined: String = lines.lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(joined, s.text, "the wrap consumes no character");

        let long = lines_of(&boxed("abcdefghijklmnop", 40.0, 400.0));
        assert!(
            long.lines.len() > 1,
            "a long word breaks between characters"
        );
        assert!(long.lines.iter().all(|l| !l.text.is_empty()));

        let mut column = boxed("ABCDEFG", 200.0, 60.0);
        column.vertical = true;
        let per = (60.0 / vertical_cell(bundled, &column.character)).floor() as usize;
        let columns = lines_of(&column);
        assert_eq!(columns.lines[0].text.chars().count(), per);
        assert_eq!(columns.lines.len(), 7usize.div_ceil(per));

        // One caret stop per UTF-16 boundary, wraps and hard returns alike.
        let text = "one two three\rfour five six";
        let stops = crate::type_caret_stops(&boxed(text, 50.0, 400.0));
        assert_eq!(stops.len(), text.encode_utf16().count() + 1);
    }

    #[test]
    fn paragraph_overflow_is_hidden_and_reported() {
        let text = "aaaa bbbb cccc dddd";
        let bundled = face_for("Liberation Sans").unwrap();
        let width = f64::from(line_advance(bundled, "aaaa", &spec(text).character)) + 4.0;
        let tall = boxed(text, width, 200.0);
        assert_eq!(lines_of(&tall).visible, 4);
        assert!(!type_overflows(&tall));
        // Room for two lines: first baseline at the ascent, then one leading.
        let two = 20.0 * 1.2 + f64::from(bundled.ascent(20.0) + bundled.descent(20.0)) + 1.0;
        let short = boxed(text, width, two);
        assert_eq!(lines_of(&short).visible, 2);
        assert!(type_overflows(&short));
        assert!(!type_overflows(&spec(text)), "point type never overflows");
        let rows = inked_rows(&short);
        assert!(
            *rows.last().unwrap() < two as u32,
            "no ink past the second line"
        );
        let all = inked_rows(&tall);
        assert!(*all.last().unwrap() > 3 * 24, "four lines render");
    }

    #[test]
    fn conversion_keeps_glyphs_in_place() {
        let point = spec("Hello type\rsecond");
        let paragraph = type_to_paragraph(&point).unwrap();
        assert!(paragraph.box_bounds.is_some());
        assert!(type_to_paragraph(&paragraph).is_none());
        let (a_rect, a) = render_type(&point).unwrap();
        let (b_rect, b) = render_type(&paragraph).unwrap();
        // The box rect may differ by the spare column; the ink must not move.
        let ink = |rect: pictura_core::PsdRect, buffer: &pictura_core::PixelBuffer| {
            let plane = buffer.pixel_count();
            let w = buffer.width as i32;
            let mut out = Vec::new();
            for (i, alpha) in buffer.data[3 * plane..].iter().enumerate() {
                if *alpha > 0 {
                    out.push((rect.left + i as i32 % w, rect.top + i as i32 / w, *alpha));
                }
            }
            out
        };
        assert_eq!(ink(a_rect, &a), ink(b_rect, &b), "same pixels");

        let back = type_to_point(&paragraph).unwrap();
        assert_eq!(back.text, point.text);
        assert!(back.box_bounds.is_none());
        assert!((back.origin.0 - point.origin.0).abs() < 1e-3);
        assert!((back.origin.1 - point.origin.1).abs() < 1e-3);

        let bundled = face_for("Liberation Sans").unwrap();
        let width = f64::from(line_advance(bundled, "aaaa", &point.character)) + 4.0;
        let two = 20.0 * 1.2 + f64::from(bundled.ascent(20.0) + bundled.descent(20.0)) + 1.0;
        let wrapped = boxed("aaaa bbbb cccc dddd", width, two);
        let flat = type_to_point(&wrapped).unwrap();
        assert_eq!(
            flat.text, "aaaa\rbbbb",
            "returns at the wraps, overflow dropped"
        );
        assert!(type_to_point(&point).is_none());
    }
}
