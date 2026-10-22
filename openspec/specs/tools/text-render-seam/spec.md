# text-render-seam Specification

## Purpose
Deterministic layout over shaped glyph runs, paragraph alignment, a POD-only rasterizer port, and font provenance.

## Requirements

### Requirement: Deterministic layout over shaped glyph runs

`pictura-core` SHALL expose a Qt-free `layout_lines` that positions already
shaped glyph runs — advance accumulation scaled from font units to device
pixels by `font_size / units_per_em`, tracking in 1/1000 em, per-line leading,
and alignment — without parsing a font or shaping text. The layout SHALL be a
pure function of its inputs and SHALL NOT depend on the host, the clock, or any
random source.

#### Scenario: Advances accumulate left to right

- **WHEN** a line of glyphs with known advances is laid out
- **THEN** each glyph's x equals the sum of the preceding device advances and the line advance is their total

#### Scenario: Tracking and leading apply

- **WHEN** tracking and leading are non-zero
- **THEN** each device advance includes `tracking * font_size / 1000` and each successive line's baseline is offset by `leading`

#### Scenario: Empty input yields an empty layout

- **WHEN** no lines or empty lines are laid out
- **THEN** the layout has no placed glyphs and zero width

### Requirement: Alignment maps the paragraph justification

The system SHALL provide `TextAlign` with `Left`, `Center`, and `Right` and
SHALL map the EngineData paragraph `Justification` byte to the alignment the
justification selects: 0 (left) to `Left`, 1 (right) to `Right`, 2 (center) to
`Center`, 3 (Justify Last Left) to `Left`, 4 (Justify Last Right) to `Right`,
5 (Justify Last Center) to `Center`, and 6 (Justify All) to `Left`. A byte of
7 or above SHALL map to `Left`. When a line is aligned inside a `wrap_width`,
every glyph SHALL be shifted by `(wrap_width - line_advance)` times `0.0`,
`0.5`, or `1.0`; without a `wrap_width` no shift SHALL be applied.

#### Scenario: Right alignment shifts by the leftover width

- **WHEN** a line shorter than the wrap width is laid out right-aligned
- **THEN** its last glyph ends at the wrap width

#### Scenario: A justify variant keeps its last line's alignment [justify_align]

- **WHEN** the justification byte is 3, 4 or 5
- **THEN** the alignment is `Left`, `Right` or `Center` respectively

#### Scenario: A justify variant places a vertical column [justify_vertical]

- **WHEN** a vertical type layer is set to Justify Last Right
- **THEN** its column sits at the bottom of its box

#### Scenario: Unknown justification falls back to left

- **WHEN** the justification byte is 7 or above
- **THEN** alignment is `Left`

### Requirement: The glyph rasterizer port is POD-only

The system SHALL define a `Rasterizer` port that takes a `RasterRequest`
(`glyph` id, pixel size, and subpixel position) and returns an optional
`GlyphMask` (`width`, `height`, integer `left`/`top` offsets, and a coverage
buffer), carrying no font bytes and no Qt type. A rasterizer SHALL be usable
without mutable state.

#### Scenario: A backend produces a coverage mask

- **WHEN** a `RasterRequest` for a glyph is passed to a rasterizer backend
- **THEN** it returns a mask whose coverage length equals `width * height`, or `None` when the glyph cannot be rendered

#### Scenario: The request carries no resource handle

- **WHEN** a `RasterRequest` is inspected
- **THEN** it contains only a glyph id, a pixel size, and a subpixel position

### Requirement: Font policy and provenance are recorded

The system SHALL define `FontPolicy` (`BundledOnly`, `HostAllowed`,
`ExactOrRefuse`) and `TextProvenance` recording the requested family, the
resolved family, the resolved font's content hash, and the rasterizer backend
and version, so that a substitution or refusal is an auditable fact.

#### Scenario: Provenance records a substitution

- **WHEN** provenance is constructed for a requested family that resolved to a different face
- **THEN** the record exposes both the requested and the resolved family and the resolved hash

#### Scenario: Policies are distinct

- **WHEN** the three font policies are compared
- **THEN** each is a distinct value that can be persisted and replayed

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
