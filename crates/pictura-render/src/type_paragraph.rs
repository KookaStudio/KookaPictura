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

use pictura_core::{CharacterAttrs, ParagraphAttrs, TypeSpec};

use crate::fonts::face_for;
use crate::text_render::BundledText;
use crate::type_box::type_layout_box;
use crate::type_layer::{align_factor, leading_px, map, type_lines};
use crate::type_style::shape_styled;

/// Float slack: a line measured at exactly the box width still fits it.
const FIT: f32 = 0.01;

/// One laid-out line. `soft` is a wrap inside a hard line, whose end boundary
/// is the next line's start (no character is consumed there).
pub(crate) struct VisualLine {
    pub text: String,
    pub soft: bool,
    /// The first visual line of its paragraph.
    pub first: bool,
}

/// Where a visual line sits inside its paragraph box. Horizontal type uses the
/// inline axis for `start`/`extent` and the block axis for `block`; vertical
/// type swaps them, with `block` measured from the box's right edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LineBox {
    /// Left (horizontal) or top (vertical) edge.
    pub start: f32,
    /// Width (horizontal) or height (vertical) available to the line.
    pub extent: f32,
    /// Baseline from the box top (horizontal) or the column centre's distance
    /// from the box right edge (vertical).
    pub block: f32,
}

/// The laid-out lines, of which the first `visible` fit the box. `inline_size`
/// is the natural width (horizontal) or height (vertical) before indents;
/// `inline_low`/`inline_high` are the ink's inline extent and `block_size` the
/// block extent including paragraph spacing.
pub(crate) struct Lines {
    pub lines: Vec<VisualLine>,
    pub boxes: Vec<LineBox>,
    pub visible: usize,
    pub inline_size: f32,
    pub inline_low: f32,
    pub inline_high: f32,
    pub block_size: f32,
}

impl Lines {
    /// The visible lines as the render sets them: a soft-wrapped line drops
    /// its trailing spaces, so alignment sits on its ink.
    pub(crate) fn rendered(&self) -> Vec<String> {
        self.rendered_paragraphs()
            .into_iter()
            .map(|(text, _)| text)
            .collect()
    }

    /// The visible lines as the render sets them, each with the flag that
    /// marks the last line of its paragraph. The paragraph structure comes
    /// from the whole text, so a paragraph the overflow cuts short has no
    /// visible last line and its last visible line still spreads.
    pub(crate) fn rendered_paragraphs(&self) -> Vec<(String, bool)> {
        self.lines[..self.visible]
            .iter()
            .map(|line| {
                let text = match line.soft {
                    true => line.text.trim_end().to_owned(),
                    false => line.text.clone(),
                };
                (text, !line.soft)
            })
            .collect()
    }
}

/// `text`'s advance as the layout sets it: the styled run's advance (scale,
/// caps, Faux Bold, kerning mode), plus tracking per glyph, with the paragraph's
/// desired word, letter, and glyph spacing applied in the order `type_justify`
/// spreads a line.
pub(crate) fn line_advance_with(
    bundled: &BundledText,
    text: &str,
    c: &CharacterAttrs,
    paragraph: &ParagraphAttrs,
) -> f32 {
    let size = c.size as f32;
    let glyphs = shape_styled(bundled, text, c);
    let mut advance = glyphs.iter().map(|g| g.advance).sum::<f32>();
    if let Some(space) = bundled.space_glyph() {
        let spaces = glyphs.iter().filter(|g| g.id == space).count() as f32;
        if spaces > 0.0 {
            let normal = glyphs
                .iter()
                .find(|g| g.id == space)
                .map_or(0.0, |g| g.advance);
            let desired = paragraph.word_spacing[1]
                .clamp(paragraph.word_spacing[0], paragraph.word_spacing[2])
                as f32;
            advance += normal * (desired - 1.0) * spaces;
        }
    }
    let desired = paragraph.letter_spacing[1]
        .clamp(paragraph.letter_spacing[0], paragraph.letter_spacing[2]) as f32
        * size;
    advance += desired * glyphs.len() as f32;
    let desired = paragraph.glyph_spacing[1]
        .clamp(paragraph.glyph_spacing[0], paragraph.glyph_spacing[2]) as f32;
    advance * desired + crate::type_style::tracking_step(c, size) * glyphs.len() as f32
}

/// [`line_advance_with`] at the paragraph defaults.
#[cfg(test)]
pub(crate) fn line_advance(bundled: &BundledText, text: &str, c: &CharacterAttrs) -> f32 {
    line_advance_with(bundled, text, c, &ParagraphAttrs::default())
}

/// The vertical cell one character occupies down a column.
pub(crate) fn vertical_cell(bundled: &BundledText, c: &CharacterAttrs) -> f32 {
    let size = c.size as f32;
    (bundled.ascent(size) + bundled.descent(size)) * (c.vertical_scale / 100.0) as f32
}

/// Break `line` to the first piece's width and then the remaining width, and
/// append its pieces to `out`.
///
/// ponytail: each candidate break re-shapes its prefix, so a line costs
/// O(n²) shaping in its length (a single long unbroken word most of all).
/// Fine for interactive type; cache per-glyph advances if long paragraphs lag.
fn wrap_line(
    line: &str,
    first_width: f32,
    rest_width: f32,
    measure: &dyn Fn(&str) -> f32,
    out: &mut Vec<VisualLine>,
) {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let text = |a: usize, b: usize| chars[a..b].iter().collect::<String>();
    if n == 0 {
        out.push(VisualLine {
            text: String::new(),
            soft: false,
            first: true,
        });
        return;
    }
    let mut start = 0;
    let mut first = true;
    while start < n {
        let width = if first { first_width } else { rest_width };
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
            first,
        });
        start = end;
        first = false;
    }
}

/// The lines `text` sets in: its hard lines for point type (`box_bounds`
/// `None`), else wrapped to the box with the fitting count in `visible`.
///
/// Each line carries the column its paragraph's indents leave it, and the block
/// offset its paragraph spacing leaves it. The first line of a paragraph adds
/// `first_line_indent` to the left (top) indent; a negative value hangs it into
/// the margin. Space before/after offsets baselines (horizontal) or the gaps
/// between columns (vertical).
pub(crate) fn visual_lines(
    bundled: &BundledText,
    text: &str,
    vertical: bool,
    box_bounds: Option<[f64; 4]>,
    c: &CharacterAttrs,
    paragraph: &ParagraphAttrs,
) -> Lines {
    let hard = type_lines(text);
    let size = c.size as f32;
    let leading = leading_px(size, c.leading, paragraph.auto_leading);
    let ascent = bundled.ascent(size);
    let descent = bundled.descent(size);
    let factor = align_factor(paragraph.justify.index());
    let start_indent = paragraph.start_indent as f32;
    let end_indent = paragraph.end_indent as f32;
    let first_indent = paragraph.first_line_indent as f32;
    let space_before = paragraph.space_before as f32;
    let space_after = paragraph.space_after as f32;

    if vertical {
        let cell = vertical_cell(bundled, c);
        let inline_size = match box_bounds {
            Some(b) => (b[3] - b[1]) as f32,
            None => {
                hard.iter()
                    .map(|line| line.chars().count())
                    .max()
                    .unwrap_or(0) as f32
                    * cell
            }
        };
        let mut lines = Vec::new();
        let mut boxes = Vec::new();
        let mut block = leading / 2.0 + space_before;
        for (p, line) in hard.iter().enumerate() {
            let extent = inline_size - start_indent - end_indent - first_indent;
            if box_bounds.is_none() {
                lines.push(VisualLine {
                    text: line.clone(),
                    soft: false,
                    first: true,
                });
                boxes.push(LineBox {
                    start: start_indent + first_indent,
                    extent,
                    block,
                });
                block += leading + space_after;
                if p + 1 < hard.len() {
                    block += space_before;
                }
                continue;
            }
            let chars: Vec<char> = line.chars().collect();
            if chars.is_empty() {
                lines.push(VisualLine {
                    text: String::new(),
                    soft: false,
                    first: true,
                });
                boxes.push(LineBox {
                    start: start_indent + first_indent,
                    extent,
                    block,
                });
                block += leading + space_after;
                if p + 1 < hard.len() {
                    block += space_before;
                }
                continue;
            }
            let mut index = 0;
            let mut first = true;
            while index < chars.len() {
                let extra = if first { first_indent } else { 0.0 };
                let extent = inline_size - start_indent - end_indent - extra;
                let per = (((extent + FIT) / cell).floor() as usize).max(1);
                let end = (index + per).min(chars.len());
                lines.push(VisualLine {
                    text: chars[index..end].iter().collect(),
                    soft: end < chars.len(),
                    first,
                });
                boxes.push(LineBox {
                    start: start_indent + extra,
                    extent,
                    block,
                });
                block += leading;
                index = end;
                first = false;
            }
            block += space_after;
            if p + 1 < hard.len() {
                block += space_before;
            }
        }
        let block_size = block - leading / 2.0;
        let visible = match box_bounds {
            Some(b) => {
                let width = (b[2] - b[0]) as f32;
                boxes
                    .iter()
                    .take_while(|line| line.block + leading / 2.0 <= width + FIT)
                    .count()
            }
            None => lines.len(),
        };
        let (inline_low, inline_high) = inline_bounds(&lines, &boxes, factor, |text| {
            text.chars().count() as f32 * cell
        });
        return Lines {
            lines,
            boxes,
            visible,
            inline_size,
            inline_low,
            inline_high,
            block_size,
        };
    }

    let run = crate::type_style::RunStyle::of(bundled, c);
    let overhang = ascent * run.skew + run.embolden;
    let inline_size = match box_bounds {
        Some(b) => (b[2] - b[0]) as f32,
        None => {
            hard.iter()
                .map(|line| line_advance_with(bundled, line, c, paragraph))
                .fold(0.0f32, f32::max)
                + overhang
        }
    };
    let line_height = ascent + descent;
    let measure = |s: &str| line_advance_with(bundled, s, c, paragraph);
    let mut lines = Vec::new();
    let mut boxes = Vec::new();
    let mut block = space_before;
    let mut last_baseline = 0.0f32;
    for (p, line) in hard.iter().enumerate() {
        let before = lines.len();
        if box_bounds.is_some() {
            wrap_line(
                line,
                inline_size - start_indent - end_indent - first_indent,
                inline_size - start_indent - end_indent,
                &measure,
                &mut lines,
            );
        } else {
            lines.push(VisualLine {
                text: line.clone(),
                soft: false,
                first: true,
            });
        }
        for (k, line) in lines[before..].iter().enumerate() {
            let extra = if line.first { first_indent } else { 0.0 };
            let line_block = block + k as f32 * leading;
            boxes.push(LineBox {
                start: start_indent + extra,
                extent: inline_size - start_indent - end_indent - extra,
                block: line_block,
            });
            last_baseline = line_block;
        }
        block += lines[before..].len() as f32 * leading + space_after;
        if p + 1 < hard.len() {
            block += space_before;
        }
    }
    let block_size = last_baseline + line_height + space_after;
    let visible = match box_bounds {
        Some(b) => {
            let height = (b[3] - b[1]) as f32;
            boxes
                .iter()
                .take_while(|line| line.block + line_height <= height + FIT)
                .count()
        }
        None => lines.len(),
    };
    let (inline_low, inline_high) = inline_bounds(&lines, &boxes, factor, |text| {
        line_advance_with(bundled, text, c, paragraph)
    });
    Lines {
        lines,
        boxes,
        visible,
        inline_size,
        inline_low,
        inline_high,
        block_size,
    }
}

/// The laid-out ink's inline extent: the first edge and the last edge of every
/// line's advance after alignment.
fn inline_bounds(
    lines: &[VisualLine],
    boxes: &[LineBox],
    factor: f32,
    advance: impl Fn(&str) -> f32,
) -> (f32, f32) {
    let mut low = f32::MAX;
    let mut high = f32::MIN;
    for (line, line_box) in lines.iter().zip(boxes) {
        let advance = advance(&line.text);
        let start = line_box.start + (line_box.extent - advance) * factor;
        low = low.min(start);
        high = high.max(start + advance);
    }
    if low == f32::MAX {
        (0.0, 0.0)
    } else {
        (low, high)
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
    let Some(bundled) = face_for(&spec.character.font_family, &spec.character.font_style) else {
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
        &spec.paragraph,
    );
    lines.visible < lines.lines.len()
}

/// `Type > Convert To Paragraph Text`: point type boxed at its own text
/// bounds, so no glyph moves. `None` for paragraph type or blank text.
pub fn type_to_paragraph(spec: &TypeSpec) -> Option<TypeSpec> {
    if spec.path.is_some() || spec.box_bounds.is_some() {
        return None;
    }
    let bounds = type_layout_box(spec)?;
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
    let bundled = face_for(&spec.character.font_family, &spec.character.font_style)?;
    let c = &spec.character;
    let lines = visual_lines(
        bundled,
        &spec.text,
        spec.vertical,
        Some(b),
        c,
        &spec.paragraph,
    );
    let size = c.size as f32;
    let factor = f64::from(align_factor(spec.justification()));
    let space_before = spec.paragraph.space_before;
    let (u, v) = if spec.vertical {
        let leading = f64::from(leading_px(size, c.leading, spec.paragraph.auto_leading));
        (
            b[2] - leading / 2.0 - space_before,
            b[1] + (b[3] - b[1]) * factor,
        )
    } else {
        (
            b[0] + (b[2] - b[0]) * factor,
            b[1] + f64::from(bundled.ascent(size)) + space_before,
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
        let bundled = face_for(&spec.character.font_family, &spec.character.font_style).unwrap();
        visual_lines(
            bundled,
            &spec.text,
            spec.vertical,
            spec.box_bounds,
            &spec.character,
            &spec.paragraph,
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
    fn rtl_wraps_at_same_words() {
        let mut ltr = boxed("aa bb cc dd ee ff gg hh", 80.0, 200.0);
        let mut rtl = ltr.clone();
        ltr.paragraph.direction = pictura_core::TextDirection::Ltr;
        rtl.paragraph.direction = pictura_core::TextDirection::Rtl;
        let ltr_lines = lines_of(&ltr);
        let rtl_lines = lines_of(&rtl);
        assert_eq!(ltr_lines.lines.len(), rtl_lines.lines.len());
        for (a, b) in ltr_lines.lines.iter().zip(&rtl_lines.lines) {
            assert_eq!(a.text, b.text);
        }
    }

    #[test]
    fn rtl_overflow_counts_logical_lines() {
        let mut ltr = boxed("aa bb cc dd ee ff gg hh", 80.0, 30.0);
        let mut rtl = ltr.clone();
        ltr.paragraph.direction = pictura_core::TextDirection::Ltr;
        rtl.paragraph.direction = pictura_core::TextDirection::Rtl;
        assert_eq!(type_overflows(&ltr), type_overflows(&rtl));
    }

    #[test]
    fn rtl_alignment_keeps_named_edge() {
        let bundled = face_for("Liberation Sans", "").unwrap();
        let c = CharacterAttrs {
            size: 20.0,
            ..CharacterAttrs::default()
        };
        let faces = crate::fonts::faces_in_order(bundled);
        let glyphs = crate::text_render::shape_line_directed(
            &faces,
            "\u{5D0}\u{5D1}\u{5D2}",
            &c,
            pictura_core::TextDirection::Rtl,
        );
        let params = pictura_core::LayoutParams {
            font_size: 20.0,
            units_per_em: 20.0,
            tracking: 0.0,
            leading: 20.0,
            align: pictura_core::TextAlign::Center,
            wrap_width: Some(200.0),
        };
        let layout = pictura_core::layout_lines(&[glyphs], &params);
        let line = &layout.lines[0];
        let left = line.glyphs.first().unwrap().x;
        let right = left + line.advance;
        assert!((left - (200.0 - right)).abs() < 1.0, "centred edge margins");
    }

    #[test]
    fn mixed_line_wrap_uses_fallback_advance() {
        let _guard = crate::fonts::FALLBACK_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let noto = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/NotoSans-Regular.ttf"
        ));
        assert!(crate::fonts::register_font(
            "Fallback Wrap",
            "",
            noto.to_vec()
        ));
        let spec = boxed("A\u{370}", 200.0, 200.0);
        let on = crate::type_caret::type_caret_stops(&spec);
        crate::fonts::set_missing_glyph_protection(false);
        let off = crate::type_caret::type_caret_stops(&spec);
        crate::fonts::set_missing_glyph_protection(true);
        assert_ne!(
            on.last().map(|s| s[0]),
            off.last().map(|s| s[0]),
            "the fallback advance changes the line's last caret stop"
        );
    }

    #[test]
    fn paragraph_wraps_words_and_long_words() {
        let bundled = face_for("Liberation Sans", "").unwrap();
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
        let bundled = face_for("Liberation Sans", "").unwrap();
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
    fn a_justified_paragraph_loses_its_spread_when_converted() {
        let bundled = face_for("Liberation Sans", "").unwrap();
        let mut s = boxed("aa bb cc", 200.0, 200.0);
        s.paragraph.justify = pictura_core::Justify::JustifyLastRight;
        let point = type_to_point(&s).expect("converts");
        assert!(point.box_bounds.is_none());
        // The lines set at their natural widths, right-aligned, so the first
        // line ends where the paragraph's spread first line ended: at the
        // box's right edge.
        let natural = line_advance(bundled, "aa bb cc", &s.character);
        assert!(natural < 200.0, "the line is not spread");
        assert!(
            (point.origin.0 - (s.origin.0 + 200.0)).abs() < 0.01,
            "{} is not the box's right edge",
            point.origin.0
        );
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

        let bundled = face_for("Liberation Sans", "").unwrap();
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

    fn layout_of(spec: &TypeSpec) -> pictura_core::TextLayout {
        let bundled = face_for(&spec.character.font_family, &spec.character.font_style).unwrap();
        let placement = crate::type_layer::type_placement(spec).unwrap();
        crate::text_render::layout_type(
            bundled,
            &crate::type_layer::render_view(spec),
            placement.rect.width(),
            placement.rect.height(),
        )
        .unwrap()
    }

    #[test]
    fn paragraph_indents_shape_the_box_lines() {
        let text = "aaaa bbbb cccc dddd eeee ffff gggg hhhh";
        let plain = layout_of(&boxed(text, 200.0, 400.0));
        let mut s = boxed(text, 200.0, 400.0);
        s.paragraph.start_indent = 20.0;
        s.paragraph.end_indent = 10.0;
        s.paragraph.first_line_indent = 15.0;
        let indented = layout_of(&s);
        assert!(
            indented.lines.len() > plain.lines.len(),
            "the indent narrows the wrap"
        );
        assert!((indented.lines[0].glyphs[0].x - 35.0).abs() < 0.01);
        assert!((indented.lines[1].glyphs[0].x - 20.0).abs() < 0.01);
        for line in &indented.lines {
            let right = line.glyphs[0].x + line.advance;
            assert!(right <= 190.0 + FIT, "line ends at {right}");
        }

        // A negative first-line indent hangs into the left margin.
        s.paragraph.first_line_indent = -15.0;
        let hanging = layout_of(&s);
        assert!((hanging.lines[0].glyphs[0].x - 5.0).abs() < 0.01);
    }

    #[test]
    fn vertical_indents_shape_the_columns() {
        let mut s = boxed("ABCDEFGHIJ", 200.0, 100.0);
        s.vertical = true;
        s.paragraph.start_indent = 20.0;
        s.paragraph.end_indent = 10.0;
        s.paragraph.first_line_indent = 15.0;
        let layout = layout_of(&s);
        assert!(layout.lines.len() > 1, "chunks into columns");
        assert!((layout.lines[0].glyphs[0].y - 35.0).abs() < 0.01);
        assert!((layout.lines[1].glyphs[0].y - 20.0).abs() < 0.01);
        for line in &layout.lines {
            let bottom = line.glyphs[0].y + line.advance;
            assert!(bottom <= 90.0 + FIT, "column bottom {bottom}");
        }

        // Point type has no box to break into: the indent moves its single
        // column but never chunks it.
        let mut point = spec("ABCDEFGHIJ");
        point.vertical = true;
        point.paragraph.start_indent = 20.0;
        point.paragraph.end_indent = 10.0;
        point.paragraph.first_line_indent = 15.0;
        let layout = layout_of(&point);
        assert_eq!(layout.lines.len(), 1, "point type does not chunk");
        assert!((layout.lines[0].glyphs[0].y - 35.0).abs() < 0.01);
    }

    #[test]
    fn point_type_indents_move_the_glyphs() {
        let mut s = spec("aa\rbb");
        s.paragraph.start_indent = 20.0;
        s.paragraph.first_line_indent = 15.0;
        let (_, pens) = crate::type_layer::type_pens(&s).unwrap();
        let first_y = pens[0].2;
        let second_x = pens
            .iter()
            .find(|(_, _, y)| (*y - first_y).abs() > 1.0)
            .unwrap()
            .1;
        assert!((pens[0].1 - 35.0).abs() < 0.5, "{}", pens[0].1);
        assert!((second_x - 35.0).abs() < 0.5, "{second_x}");
        let placement = crate::type_layer::type_placement(&s).unwrap();
        assert!(placement.rect.left <= s.origin.0 as i32 + 20);
        assert!(placement.rect.right >= s.origin.0 as i32 + 35);
    }

    #[test]
    fn space_before_and_after_separate_paragraphs() {
        let bundled = face_for("Liberation Sans", "").unwrap();
        let leading = 20.0 * 1.2;
        let line_height = f64::from(bundled.ascent(20.0) + bundled.descent(20.0));
        let short = leading + line_height + 1.0;
        let plain = layout_of(&boxed("aa\rbb", 200.0, short));
        assert_eq!(plain.lines.len(), 2);

        let mut s = boxed("aa\rbb", 200.0, short);
        s.paragraph.space_before = 8.0;
        s.paragraph.space_after = 12.0;
        assert_eq!(
            layout_of(&s).lines.len(),
            1,
            "the second paragraph overflows"
        );
        assert!(type_overflows(&s));

        let plain_tall = layout_of(&boxed("aa\rbb", 200.0, 400.0));
        s.box_bounds = Some([0.0, 0.0, 200.0, 400.0]);
        let tall = layout_of(&s);
        let gap = tall.lines[1].baseline - tall.lines[0].baseline;
        let plain_gap = plain_tall.lines[1].baseline - plain_tall.lines[0].baseline;
        assert!((gap - plain_gap - 20.0).abs() < 0.01, "{gap}");
    }

    #[test]
    fn vertical_space_before_and_after_separate_columns() {
        let mut s = boxed("A\rB", 200.0, 200.0);
        s.vertical = true;
        let plain = layout_of(&s);
        let plain_gap = plain.lines[0].baseline - plain.lines[1].baseline;
        s.paragraph.space_before = 8.0;
        s.paragraph.space_after = 12.0;
        let spaced = layout_of(&s);
        let gap = spaced.lines[0].baseline - spaced.lines[1].baseline;
        assert!((gap - plain_gap - 20.0).abs() < 0.01, "{gap}");

        let mut p = spec("A\rB");
        p.vertical = true;
        let plain_rect = crate::type_layer::type_placement(&p).unwrap().rect;
        p.paragraph.space_before = 8.0;
        p.paragraph.space_after = 12.0;
        let spaced_rect = crate::type_layer::type_placement(&p).unwrap().rect;
        assert_eq!(spaced_rect.width() - plain_rect.width(), 40);
    }

    #[test]
    fn caret_follows_indented_lines() {
        let mut s = boxed("aa bb cc dd", 120.0, 400.0);
        s.paragraph.start_indent = 10.0;
        s.paragraph.first_line_indent = 20.0;
        let stops = crate::type_caret::type_caret_stops(&s);
        let origin = s.origin.0;
        let layout = layout_of(&s);
        let first_pen = origin + f64::from(layout.lines[0].glyphs[0].x);
        assert!(
            (stops[0][0] - (origin + 30.0)).abs() < 0.5,
            "{:?}",
            stops[0]
        );
        assert!(
            (stops[0][0] - first_pen).abs() < 0.5,
            "the stop is not on the first pen"
        );
        let second = stops
            .iter()
            .find(|stop| (stop[1] - stops[0][1]).abs() > 1.0)
            .expect("the paragraph wraps");
        assert!((second[0] - (origin + 10.0)).abs() < 0.5, "{:?}", second);
        let wrapped_pen = origin + f64::from(layout.lines[1].glyphs[0].x);
        assert!(
            (second[0] - wrapped_pen).abs() < 0.5,
            "the wrapped stop is not on its pen"
        );
    }

    #[test]
    fn point_bounds_include_indents_and_spacing() {
        let mut s = spec("aa");
        s.paragraph.first_line_indent = -15.0;
        let plain = crate::type_layer::type_placement(&s).unwrap();
        s.paragraph.space_after = 12.0;
        let spaced = crate::type_layer::type_placement(&s).unwrap();
        assert_eq!(plain.rect.left, s.origin.0 as i32 - 15);
        assert_eq!(spaced.rect.bottom - plain.rect.bottom, 12);
    }

    #[test]
    fn unsupported_paragraph_attributes_do_not_move_glyphs() {
        let mut s = boxed("aa bb cc dd", 120.0, 200.0);
        let plain = layout_of(&s);
        s.paragraph.hanging = true;
        s.paragraph.hyphenate = true;
        s.paragraph.composer = pictura_core::Composer::EveryLine;
        let styled = layout_of(&s);
        assert_eq!(plain.lines.len(), styled.lines.len());
        for (a, b) in plain.lines.iter().zip(&styled.lines) {
            assert!((a.advance - b.advance).abs() < 1e-6);
            for (ga, gb) in a.glyphs.iter().zip(&b.glyphs) {
                assert!((ga.x - gb.x).abs() < 1e-6 && (ga.y - gb.y).abs() < 1e-6);
            }
        }
    }
}
