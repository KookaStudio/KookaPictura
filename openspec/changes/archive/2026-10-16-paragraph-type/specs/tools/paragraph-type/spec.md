## ADDED Requirements

### Requirement: Dragging with a Type tool makes paragraph type

A drag of more than four screen pixels with a Type or Type Mask tool, starting
off any type layer, SHALL open a paragraph-type session in the dragged box. A
plain click SHALL still make point type. The text SHALL wrap to the box: a
horizontal line SHALL break after a run of spaces before the word that would
cross the box width, a word wider than the box SHALL break between characters,
and a vertical column SHALL break at the box height. Commit SHALL add one type
layer the size of the box, recording one "Horizontal Type" / "Vertical Type"
state, and reopening it SHALL edit in the same box.

#### Scenario: Drag a box, type, and commit [para_drag_box]

- **WHEN** `tst_paragraph_type::dragMakesParagraphType` drags a 120 × 80 box with Horizontal Type, types a sentence wider than the box, commits, and reopens the layer
- **THEN** the session shows the box, the preview wraps to more than one line within the box width, the commit records one "Horizontal Type" state adding a paragraph layer whose box is the dragged box and whose rect lies within it, and the reopened session shows the same box

#### Scenario: Wrapping in the engine [para_wrap_engine]

- **WHEN** `pictura-render::paragraph_wraps_words_and_long_words` lays out horizontal text in a narrow box and a word wider than the box, and vertical text in a short box
- **THEN** lines break after spaces, never exceed the box width, keep their trailing spaces for the caret, the long word breaks between characters, the vertical text breaks into columns of the box height, and the caret stops still number one per UTF-16 boundary

### Requirement: The Paragraph Text Size dialog

Alt-clicking with the Horizontal or Vertical Type tool SHALL open a Paragraph
Text Size dialog with Width and Height in pixels. OK SHALL open a
paragraph-type session with a box of that size whose top-left is the click.
Cancel SHALL open nothing.

#### Scenario: Alt-click sizes the box [para_alt_click]

- **WHEN** `tst_paragraph_type::altClickAsksForTheBoxSize` Alt-clicks with Horizontal Type and cancels, then Alt-clicks again and enters 90 × 40
- **THEN** the cancel leaves no session, and the second opens a session whose box is 90 × 40 with its top-left at the click

### Requirement: The box while editing

A paragraph-type session SHALL draw its box with eight handles. Dragging a
handle SHALL resize the box (it stays at least one pixel a side) and reflow the
text. A press inside the box SHALL place the caret instead of committing. Text
that does not fit the box SHALL be overflow: its lines SHALL NOT render, and the
box's bottom-right handle SHALL show a plus sign while any overflow exists.

#### Scenario: Resizing reflows, and overflow shows a plus [para_resize_overflow]

- **WHEN** `tst_paragraph_type::handlesResizeAndOverflow` types a long sentence into a short box, then drags the bottom-right handle down and right
- **THEN** the short box reports overflow; after the drag, the box is larger, the text reflows, the overflow clears, and nothing is recorded until commit

#### Scenario: Overflow lines do not render [para_overflow_engine]

- **WHEN** `pictura-render::paragraph_overflow_is_hidden_and_reported` renders text that needs four lines in a box with room for two
- **THEN** only two lines' worth of rows have ink, `type_overflows` is true, and a box tall enough for all four renders four with no overflow

### Requirement: Paragraph type round-trips through PSD

A paragraph type layer's PSD SHALL carry its box in the EngineData shape
(`/Rendered /Shapes /Children` with `/ShapeType 1` and `/BoxBounds`), and point
type SHALL carry `/ShapeType 0`. Opening the file SHALL read the box back, so
the layer re-renders wrapped and reopens as paragraph type.

#### Scenario: The box survives encode and decode [para_psd_roundtrip]

- **WHEN** `pictura-codec::paragraph_box_round_trips` authors a paragraph spec and a point spec, encodes, and decodes them, including a merge over an existing EngineData tree
- **THEN** the paragraph tool decodes with the same box bounds and `ShapeType 1`, and the point tool decodes with no box

### Requirement: Convert between point and paragraph type

`Type > Convert To Paragraph Text` SHALL be enabled for a point type layer and
SHALL give it a box at its text bounds without moving its glyphs.
`Type > Convert To Point Text` SHALL be enabled for a paragraph type layer and
SHALL end each wrapped line with a return and drop the overflow text, keeping
the first line where it was. Each SHALL record one "Convert To Paragraph Text" /
"Convert To Point Text" state, and each SHALL be disabled for any other layer.

#### Scenario: Converting from the Type menu [para_convert]

- **WHEN** `tst_paragraph_type::convertCommands` converts a point layer to paragraph text, then that layer back to point text
- **THEN** each command is enabled only for its kind, each records one state, the paragraph layer has a box, and the point layer has no box and the same text

#### Scenario: Conversion keeps the glyphs in place [para_convert_engine]

- **WHEN** `pictura-render::conversion_keeps_glyphs_in_place` converts two-line point type to paragraph and back, and converts wrapped, overflowing paragraph type to point
- **THEN** the paragraph render's ink matches the point render's pixel for pixel, the round trip restores the text and origin, and the wrapped text becomes return-separated lines without the overflow
