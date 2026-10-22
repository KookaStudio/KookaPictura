# tools/paragraph-type Specification

## MODIFIED Requirements

### Requirement: Convert between point and paragraph type

`Type > Convert To Paragraph Text` SHALL be enabled for a point type layer and
SHALL give it a box at its text bounds without moving its glyphs.
`Type > Convert To Point Text` SHALL be enabled for a paragraph type layer and
SHALL end each wrapped line with a return and drop the overflow text, keeping
the first line's anchor where it was. A justified paragraph SHALL lose its
spread when it becomes point text: its lines set at their natural widths,
anchored as the paragraph's last line was, so the first line's ink no longer
fills the column. Each SHALL record one "Convert To Paragraph Text" /
"Convert To Point Text" state, and each SHALL be disabled for any other layer.

#### Scenario: Converting from the Type menu [para_convert]

- **WHEN** `tst_paragraph_type::convertCommands` converts a point layer to paragraph text, then that layer back to point text
- **THEN** each command is enabled only for its kind, each records one state, the paragraph layer has a box, and the point layer has no box and the same text

#### Scenario: Conversion keeps the glyphs in place [para_convert_engine]

- **WHEN** `pictura-render::conversion_keeps_glyphs_in_place` converts two-line point type to paragraph and back, and converts wrapped, overflowing paragraph type to point
- **THEN** the paragraph render's ink matches the point render's pixel for pixel, the round trip restores the text and origin, and the wrapped text becomes return-separated lines without the overflow

#### Scenario: A justified paragraph loses its spread when converted [para_convert_justify]

- **WHEN** a paragraph box set to Justify Last Right is converted to point text
- **THEN** the lines set at their natural widths, right-aligned, the first line ending where the paragraph's first line ended
