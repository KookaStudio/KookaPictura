//! The paragraph justification: whether a justification spreads a paragraph's
//! lines, how much each word space takes when a line is spread to the column,
//! and the widening of a shaped line's space glyphs.
//!
//! ponytail: the spread divides the leftover width evenly; the word-spacing
//! min/desired/max bounds, the letter spacing and the glyph scaling are
//! authored to the engine data but not laid out (#518).

use pictura_core::{Justify, ShapedGlyph, TextAlign, TextLayout};

use crate::text_render::BundledText;

/// Whether the justification spreads the paragraph's lines at all.
pub(crate) fn spreads(justify: Justify) -> bool {
    matches!(
        justify,
        Justify::JustifyLastLeft
            | Justify::JustifyLastRight
            | Justify::JustifyLastCenter
            | Justify::JustifyAll
    )
}

/// Whether the justification spreads the paragraph's last line too.
pub(crate) fn spreads_last(justify: Justify) -> bool {
    justify == Justify::JustifyAll
}

/// The number of word spaces in a line of text.
pub(crate) fn spaces(line: &str) -> usize {
    line.chars().filter(|c| *c == ' ').count()
}

/// The number of space glyphs in a shaped run.
pub(crate) fn shaped_spaces(bundled: &BundledText, glyphs: &[ShapedGlyph]) -> usize {
    let Some(space) = bundled.space_glyph() else {
        return 0;
    };
    glyphs.iter().filter(|g| g.id == space).count()
}

/// The extra width each word space of a line takes when the line is spread to
/// `column`: the leftover width over the line's word spaces. `None` when the
/// line is not spread: no word space, no leftover width, a face without a
/// space glyph, or a face whose space glyph another character shares - then
/// the shaped run's spaces and the line's spaces disagree, and the renderer
/// and the caret would drift apart.
pub(crate) fn space_extra(
    bundled: &BundledText,
    line: &str,
    shaped: usize,
    advance: f32,
    column: f32,
) -> Option<f32> {
    bundled.space_glyph()?;
    let spaces = spaces(line);
    if spaces == 0 || spaces != shaped {
        return None;
    }
    let leftover = column - advance;
    if leftover <= 0.0 {
        return None;
    }
    Some(leftover / spaces as f32)
}

/// Widen the line's space glyphs by `extra` each.
fn widen_spaces(glyphs: &mut [ShapedGlyph], space: u16, extra: f32) {
    for glyph in glyphs {
        if glyph.id == space {
            glyph.advance += extra;
        }
    }
}

/// Spread the lines the justification spreads: each one's word spaces take the
/// leftover width, so the line's advance reaches the column and the layout
/// shifts it by zero. `lines` carries each line's text and whether it ends
/// its paragraph; `tracking` is the per-glyph tracking step. Returns the
/// lines that were spread.
pub(crate) fn spread_lines(
    bundled: &BundledText,
    lines: &[(String, bool)],
    shaped: &mut [Vec<ShapedGlyph>],
    justify: Justify,
    column: f32,
    tracking: f32,
) -> Vec<bool> {
    let mut spread = vec![false; shaped.len()];
    if !spreads(justify) {
        return spread;
    }
    let Some(space) = bundled.space_glyph() else {
        return spread;
    };
    for (i, ((line, last), glyphs)) in lines.iter().zip(shaped.iter_mut()).enumerate() {
        if *last && !spreads_last(justify) {
            continue;
        }
        let advance: f32 =
            glyphs.iter().map(|g| g.advance).sum::<f32>() + tracking * glyphs.len() as f32;
        let Some(extra) = space_extra(
            bundled,
            line,
            shaped_spaces(bundled, glyphs),
            advance,
            column,
        ) else {
            continue;
        };
        widen_spaces(glyphs, space, extra);
        spread[i] = true;
    }
    spread
}

/// Shift the lines a justification could not spread back to flush left: the
/// layout shifted them by the paragraph's last line's alignment, and a line
/// with no word space to spread stays where it was set.
pub(crate) fn flush_left(
    layout: &mut TextLayout,
    lines: &[(String, bool)],
    spread: &[bool],
    justify: Justify,
    column: f32,
) {
    if !spreads(justify) {
        return;
    }
    let align = TextAlign::from_justification(justify.index());
    for (line, ((_, last), was_spread)) in
        layout.lines.iter_mut().zip(lines.iter().zip(spread.iter()))
    {
        if *last || *was_spread {
            continue;
        }
        let shift = -(column - line.advance) * align.factor();
        for glyph in &mut line.glyphs {
            glyph.x += shift;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{
        CharacterAttrs, LayoutLine, ParagraphAttrs, TextLayout, TextStyle, TypeTool,
    };

    use crate::fonts::face_for;

    /// A paragraph of short words, so every wrapped line holds several of
    /// them and each has a word space to spread.
    const TEXT: &str = "aa bb cc dd ee ff gg hh ii jj kk ll mm nn oo pp qq rr";
    const COLUMN: f32 = 200.0;

    fn tool(text: &str, box_bounds: Option<[f64; 4]>, justify: Justify) -> TypeTool {
        TypeTool {
            transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text: text.into(),
            bounds: [0; 4],
            text_desc: Vec::new(),
            warp_desc: Vec::new(),
            fonts: vec!["Liberation Sans".into()],
            style: Some(TextStyle {
                font: Some("Liberation Sans".into()),
                character: CharacterAttrs {
                    size: 20.0,
                    ..CharacterAttrs::default()
                },
                paragraph: ParagraphAttrs {
                    justify,
                    ..ParagraphAttrs::default()
                },
                ..TextStyle::default()
            }),
            vertical: false,
            box_bounds,
        }
    }

    fn layout(text: &str, box_bounds: Option<[f64; 4]>, justify: Justify) -> TextLayout {
        let bundled = face_for("Liberation Sans").expect("bundled face");
        crate::text_render::layout_type(bundled, &tool(text, box_bounds, justify), 200, 100)
            .expect("the layer lays out")
    }

    fn boxed() -> [f64; 4] {
        [0.0, 0.0, f64::from(COLUMN), 200.0]
    }

    fn first_x(line: &LayoutLine) -> f32 {
        line.glyphs.first().map_or(0.0, |g| g.x)
    }

    #[test]
    fn a_justified_line_fills_the_column() {
        let layout = layout(TEXT, Some(boxed()), Justify::JustifyLastLeft);
        assert!(layout.lines.len() >= 2, "the paragraph wraps");
        let first = &layout.lines[0];
        assert!(
            (first.advance - COLUMN).abs() < 0.01,
            "{} is not the column width",
            first.advance
        );
        assert!(first_x(first).abs() < 0.01, "the line starts at the column");
    }

    #[test]
    fn the_last_line_keeps_its_variants_alignment() {
        let right = layout(TEXT, Some(boxed()), Justify::JustifyLastRight);
        let left = layout(TEXT, Some(boxed()), Justify::JustifyLastLeft);
        assert!(
            right.lines.len() >= 2 && left.lines.len() >= 2,
            "the paragraph wraps"
        );
        for layout in [&right, &left] {
            assert!(
                (layout.lines[0].advance - COLUMN).abs() < 0.01,
                "the first line spreads"
            );
        }
        let last_right = right.lines.last().unwrap();
        let last_left = left.lines.last().unwrap();
        assert!(
            last_right.advance < COLUMN && last_left.advance < COLUMN,
            "the last line does not spread"
        );
        assert!(
            first_x(last_left).abs() < 0.01,
            "the last line is flush left"
        );
        assert!(
            first_x(last_right) > first_x(last_left) + 1.0,
            "the last line is pushed right"
        );
    }

    #[test]
    fn justify_all_spreads_the_last_line() {
        let layout = layout(TEXT, Some(boxed()), Justify::JustifyAll);
        assert!(layout.lines.len() >= 2, "the paragraph wraps");
        for line in &layout.lines {
            assert!(
                (line.advance - COLUMN).abs() < 0.01,
                "{} is not the column width",
                line.advance
            );
        }
    }

    #[test]
    fn a_line_with_no_word_space_stays_flush_left() {
        let narrow = Some([0.0, 0.0, 60.0, 200.0]);
        let layout = layout("abcdefghijkl", narrow, Justify::JustifyLastRight);
        assert!(layout.lines.len() >= 2, "the long word breaks");
        assert!(
            first_x(&layout.lines[0]).abs() < 0.01,
            "the line with no word space is flush left"
        );
        // The paragraph's last line takes the variant's alignment, not the
        // flush left of a line that could not be spread.
        let last = &layout.lines[layout.lines.len() - 1];
        assert!(first_x(last) > 0.0, "the last line is right-aligned");
    }

    #[test]
    fn a_point_type_layer_has_no_column() {
        let all = layout("aa\rb", None, Justify::JustifyAll);
        assert_eq!(all.lines.len(), 2);
        for line in &all.lines {
            assert!(line.advance < COLUMN, "the line is not spread");
            assert!(first_x(line).abs() < 0.01, "the line keeps the alignment");
        }
        let right = layout("aa\rb", None, Justify::JustifyLastRight);
        for line in &right.lines {
            assert!(line.advance < COLUMN, "the line is not spread");
            assert!(
                first_x(line) > COLUMN - line.advance - 0.01,
                "the line keeps the variant's alignment"
            );
        }
        // The two lines have different widths, so they sit at different
        // offsets inside the column.
        assert!(first_x(&right.lines[1]) > first_x(&right.lines[0]));
    }

    #[test]
    fn the_caret_stands_where_the_spread_glyphs_are_on_a_wrapped_line() {
        let bundled = face_for("Liberation Sans").expect("bundled face");
        let tool = tool(TEXT, Some(boxed()), Justify::JustifyLastRight);
        let layout = crate::text_render::layout_type(bundled, &tool, 200, 100).expect("lays out");
        let mut spec = pictura_core::TypeSpec::new(TEXT, "Liberation Sans", 20.0);
        spec.origin = (0.0, 0.0);
        spec.box_bounds = Some([0.0, 0.0, f64::from(COLUMN), 200.0]);
        spec.paragraph.justify = Justify::JustifyLastRight;
        let stops = crate::type_caret_stops(&spec);
        // The wrapped line is the one a justification spreads, so its caret
        // stops stand at the spread glyphs' pens.
        let line = &layout.lines[0];
        assert!(
            line.advance > 0.0 && (line.advance - COLUMN).abs() < 0.01,
            "the line spreads"
        );
        // The caret re-shapes each prefix, so it misses the kerning across a
        // prefix boundary: a fraction of a pixel, not a word's width.
        for (glyph, stop) in line.glyphs.iter().zip(stops.iter()) {
            assert!(
                (f64::from(glyph.x) - stop[0]).abs() < 1.0,
                "the caret at {} is not the glyph at {}",
                stop[0],
                glyph.x
            );
        }
    }

    #[test]
    fn a_justify_variant_places_a_vertical_column() {
        let bundled = face_for("Liberation Sans").expect("bundled face");
        let mut top = tool(
            "abc",
            Some([0.0, 0.0, 100.0, 200.0]),
            Justify::JustifyLastLeft,
        );
        top.vertical = true;
        let top = crate::text_render::layout_type(bundled, &top, 100, 200).expect("lays out");
        let mut bottom = tool(
            "abc",
            Some([0.0, 0.0, 100.0, 200.0]),
            Justify::JustifyLastRight,
        );
        bottom.vertical = true;
        let bottom = crate::text_render::layout_type(bundled, &bottom, 100, 200).expect("lays out");
        let y = |layout: &TextLayout| layout.lines[0].glyphs.first().map_or(0.0, |g| g.y);
        assert!(y(&top).abs() < 0.01, "the column is at the top");
        assert!(y(&bottom) > y(&top) + 10.0, "the column is at the bottom");
    }
}
