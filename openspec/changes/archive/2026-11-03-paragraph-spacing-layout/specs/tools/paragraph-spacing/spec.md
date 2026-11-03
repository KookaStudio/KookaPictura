## ADDED Requirements

### Requirement: Paragraph indents shape each line's column

Horizontal type SHALL lay every visual line in a column inset by the
paragraph's left and right indents. The first line of each paragraph SHALL add
the first-line indent relative to the left indent; a negative first-line indent
SHALL extend that line into the left margin. Vertical type SHALL map the left
and right indents to top and bottom. Wrapping SHALL use the indented column
width (horizontal) or height (vertical), so a line SHALL NOT wrap outside its
column.

#### Scenario: A horizontal box wraps inside its indents [indent_box_horizontal]

- **WHEN** `pictura-render::paragraph_indents_shape_the_box_lines` lays out a
  horizontal paragraph box with left indent 20, right indent 10, and first-line
  indent 15
- **THEN** the first line starts 35 px from the box's left edge, every line ends
  no farther than 10 px from the box's right edge, later lines start 20 px from
  the left edge, and text that fits only without the indent wraps instead

#### Scenario: A vertical box indents its columns [indent_box_vertical]

- **WHEN** `pictura-render::vertical_indents_shape_the_columns` lays out a
  vertical paragraph box with top indent 20, bottom indent 10, and first-line
  indent 15
- **THEN** the first column starts 35 px below the box's top edge, later columns
  start 20 px below it, and each column's characters stay inside the indented
  height

#### Scenario: Point type moves with its indents [indent_point]

- **WHEN** `pictura-render::point_type_indents_move_the_glyphs` renders a point
  type layer with left indent 20 and first-line indent 15
- **THEN** every hard line (each is a paragraph) sits 35 px right of the click,
  and the placement rect covers the shifted ink

### Requirement: Space before and after separate paragraphs

Space before SHALL offset a paragraph's first line from the preceding
paragraph; space after SHALL offset the following paragraph by the same block
axis. Horizontal type SHALL apply the offsets to baselines and vertical type to
inter-column gaps. The first paragraph's space before and the last paragraph's
space after SHALL count toward point-type bounds. A line or column pushed
outside a paragraph box by paragraph spacing SHALL count as overflow.

#### Scenario: Horizontal paragraphs separate [space_before_after]

- **WHEN** `pictura-render::space_before_and_after_separate_paragraphs` lays out
  two horizontal paragraphs with space after 12 and space before 8
- **THEN** their baselines are 20 px farther apart than the same text without
  spacing, and a box tall enough only without the spacing hides the second
  paragraph

#### Scenario: Vertical paragraphs separate [space_vertical]

- **WHEN** `pictura-render::vertical_space_before_and_after_separate_columns`
  lays out two vertical paragraphs with space before 8 and space after 12
- **THEN** their columns are 20 px farther apart, and a point-type layer's box
  grows to include the trailing space

### Requirement: The caret and bounds follow the laid-out lines

The caret stops SHALL stand on the laid-out glyphs, including their indent and
paragraph-spacing offsets. A point-type placement rect SHALL include the
indented, hanging, and spaced ink, so a click on the first line lands on that
line and the layer rect covers what renders.

#### Scenario: The caret follows an indented first line [caret_indent]

- **WHEN** `pictura-render::caret_follows_indented_lines` asks for the caret
  stops of a paragraph box with left indent 10 and first-line indent 20 whose
  text wraps
- **THEN** the first line's start stop stands 30 px right of the box's left
  edge and the wrapped line's start stop stands 10 px right, each at its glyph's
  pen

#### Scenario: Point-type bounds include the spacing [bounds_indent]

- **WHEN** `pictura-render::point_bounds_include_indents_and_spacing` places a
  point type layer with a negative first-line indent and a positive space after
- **THEN** its rect reaches left of the click for the hanging line and below the
  last baseline for the trailing space

### Requirement: Unsupported paragraph attributes stay authored-only

Hanging punctuation, hyphenation, and the composer SHALL remain stored,
edited, and round-tripped, but SHALL NOT change the laid-out line breaks or
glyph pens in this change.

#### Scenario: The ceilings do not move glyphs [spacing_ceilings]

- **WHEN** `pictura-render::unsupported_paragraph_attributes_do_not_move_glyphs`
  lays out the same text with hanging punctuation, hyphenation, and the
  every-line composer toggled
- **THEN** every line's text and every glyph pen match the default layout
