# tools/text-render-seam Specification

## MODIFIED Requirements

### Requirement: A justified paragraph spreads to the column width

Each hard line of a text is a paragraph, and the lines a paragraph box wraps
from it belong to that paragraph. A paragraph of horizontal type whose
justification spreads it SHALL set every line but the paragraph's last line
across the column width, by widening the line's word spaces evenly, so that
the line's advance equals the column width. Justify All SHALL spread the
paragraph's last line as well. The column SHALL be the paragraph box's width;
a layer without a box has no column and SHALL NOT spread its lines, the
justification variant still selecting the line's alignment. A line with no
word space SHALL NOT be spread: it SHALL stay flush left, unless it is the
paragraph's last line under a justify-last variant, which takes the variant's
alignment. A line SHALL NOT be spread unless its shaped run carries one space
glyph per space character. A paragraph whose overflow cuts it short SHALL
still spread its last visible line, because the paragraph continues past the
box. The spread SHALL equalise the line's advance, not its ink: with a
non-zero tracking the ink ends one tracking step short of the column. The
paragraph's word-spacing bounds and desired value SHALL NOT be applied, and
the spread SHALL be computed for left-to-right text. A vertical paragraph's
lines run down a column and SHALL NOT be spread; its justification selects
the column's placement - top, bottom or centre - as its variant names.

#### Scenario: A justified line fills the column [justify_spread]

- **WHEN** a paragraph in a 200 px box is set to Justify Last Left and wraps
  into two lines
- **THEN** the first line's advance is 200 px, within a fraction of a pixel,
  and the line starts at the box's left edge

#### Scenario: The last line keeps its variant's alignment [justify_last_line]

- **WHEN** the same paragraph is set to Justify Last Right
- **THEN** the first line is spread to the column width and the last line is
  right-aligned inside it

#### Scenario: Justify All spreads the last line too [justify_all]

- **WHEN** the same paragraph is set to Justify All
- **THEN** both lines are spread to the column width

#### Scenario: A line with no word space stays flush left [justify_no_space]

- **WHEN** a paragraph set to Justify Last Right has a line with no word
  space, and that line is not the paragraph's last
- **THEN** that line is not spread and stays flush left

#### Scenario: A layer without a box has no column [justify_no_column]

- **WHEN** a point-type layer with two lines of different lengths is set to
  Justify All
- **THEN** neither line is spread and each line keeps the variant's alignment

#### Scenario: The caret stands between the spread glyphs [justify_caret]

- **WHEN** the caret stops of a justified paragraph are computed
- **THEN** each stop stands where the spread glyphs are, within a fraction of
  a pixel; a wrapped line's stops stop short of its end, whose boundary is
  the next line's start
