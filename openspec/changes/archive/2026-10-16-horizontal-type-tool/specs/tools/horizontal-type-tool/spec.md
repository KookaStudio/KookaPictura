## ADDED Requirements

### Requirement: Horizontal Type tool

A Horizontal Type click SHALL open a point-type session at the click, the click
on the first baseline. While the session is open, typed characters SHALL be
added to the text, Enter SHALL start a new line, and Backspace SHALL remove the
last character; the window's single-letter tool shortcuts SHALL NOT fire. The
canvas SHALL preview the text as it will commit, and the document and history
SHALL be unchanged until commit. Ctrl+Enter, keypad Enter, the options bar's
Commit, a click away from the session, or switching tools SHALL commit: blank
text SHALL record nothing, otherwise one type layer SHALL be added above the
active layer and made active, recording exactly one "Horizontal Type" state;
the Layers panel SHALL show that row as a type layer — a T on a white
thumbnail — rather than its pixels.
Esc or the options bar's Cancel SHALL discard the text without a state.

#### Scenario: Typing and committing a type layer

- **WHEN** the `tst_type_tools` horizontal test clicks, types "Big", Enter, "type", and presses Ctrl+Enter
- **THEN** typing "B" does not switch tools, nothing is recorded before the commit, and the commit records one "Horizontal Type" state that adds one active type layer two lines tall starting at the click, whose Layers row thumbnail is a dark T on white

#### Scenario: Cancelling and implicit commits

- **WHEN** the `tst_type_tools` keys test types then presses Esc, backspaces to nothing then commits, then types and clicks away, then types and switches tools
- **THEN** the first two record nothing and the last two each record one "Horizontal Type" state

### Requirement: Type sessions edit like a text field

While a Type tool session is open, a click in its text SHALL place the caret
at the nearest character boundary of the engine's layout, a drag or Shift-click
SHALL select, and a double-click SHALL select the word (or run of spaces) under
it. The arrows SHALL move the caret along the text and between lines (by word
with Ctrl, extending the selection with Shift), Home / End SHALL go to the
line's ends (the text's with Ctrl), typing SHALL replace the selection or
insert at the caret, Backspace / Delete SHALL remove the selection or the
character before / after the caret, and Ctrl+A / C / X / V SHALL select all,
copy, cut, and paste the session's text. Reopening a type layer SHALL put the
caret where it was clicked.

#### Scenario: Inserting, selecting, and the clipboard

- **WHEN** the `tst_type_tools` field test types "one two", clicks before "one" and types ">", selects "two" with Ctrl+Shift+Left and types "2", deletes the ">", copies all, pastes "\nthree", double-clicks "three" and types "3", then commits
- **THEN** the clipboard held "one 2", the double-click selected five characters, and the committed layer's text is "one 2\r3"

#### Scenario: The caret model

- **WHEN** `tst_type_tools::textEditModel` drives word steps, line steps, Home with Shift, Backspace, Delete, word selection, and a surrogate pair
- **THEN** each edit leaves the expected text and caret, and the caret never stops inside the surrogate pair

### Requirement: Clicking a type layer reopens it

A Horizontal or Vertical Type click on a visible type layer SHALL reopen that
layer instead of starting new type: the session SHALL start with the layer's
text, in its own orientation, size, and alignment (shown in the options bar),
at its position, and the layer SHALL be hidden while it is retyped, recording
nothing. Commit SHALL re-set the layer in place and record exactly one "Edit
Type Layer" state; Esc, or committing blank text, SHALL show the layer again
unchanged with no state. The Type Mask tools SHALL start new type over a type
layer.

#### Scenario: Editing a type layer in place

- **WHEN** the `tst_type_tools` reopen test commits "Hello", clicks on it, types " there", and presses Ctrl+Enter
- **THEN** the click hides the layer and records nothing, the commit records one "Edit Type Layer" state, the layer count is unchanged, and the layer is visible, named "Hello there", wider, and at the same top-left

#### Scenario: Cancelling an edit and the other tools

- **WHEN** the test reopens it and presses Esc, reopens it and backspaces it blank before committing, reopens it with Vertical Type, then clicks it with Horizontal Type Mask
- **THEN** the first two leave it visible and unchanged with no state, Vertical Type edits it as horizontal text, and the mask tool records a "Horizontal Type Mask" state

### Requirement: Transforming type keeps it type

Free Transform or Skew of a type layer without a layer mask SHALL fold the map
into the layer's `TySh` transform and re-render the text from its outlines,
so the result stays a sharp, editable type layer rather than resampled
pixels, recording the usual one "Free Transform" state. A reopened
transformed layer SHALL keep its transform when edited. A position-locked
type layer SHALL refuse the transform.

#### Scenario: Scaling type stays sharp

- **WHEN** `type_layer::tests::free_transform_re_sets_type_sharp_and_editable` scales a type layer 3× and then turns and skews it
- **THEN** it stays a type layer whose transform is the scale, its edges are no softer per solid pixel than before (unlike a resample), editing keeps the scale, the turn stands it up, and the skew stays type

#### Scenario: Free Transform from the canvas

- **WHEN** the `tst_type_tools` transform test drags a type layer's right handle to twice its width, commits, then reopens it and types a letter
- **THEN** one "Free Transform" state is recorded, the layer is still type and twice as wide, and the edit records one "Edit Type Layer" state at the same stretched height

### Requirement: Authored type layers reopen as type

A committed type layer SHALL carry its rendered pixels and an authored `TySh`
block: the transform at the click, the text, the orientation, and EngineData
naming the font, size, fill colour (alpha first), and justification, with run
lengths covering the text. Reading that block back SHALL give the same view.

#### Scenario: The authored block round-trips

- **WHEN** `type_write::tests::authored_type_tool_reads_back_as_written` encodes and decodes an authored two-line, centred, red type spec
- **THEN** the decoded view equals the authored one, including text, transform, bounds, font, size 36, colour, and justification

### Requirement: Type options bar

The Type tools' options bar SHALL set the font family, size in pixels,
anti-aliasing (None or Sharp), and alignment (left, center, right; top, center,
bottom for vertical type) used by the next commit, SHALL enable Commit and
Cancel only while text is being typed, and its Toggle Text Orientation SHALL
switch to the same tool in the other orientation. Type SHALL render, lay out,
and place its caret in the chosen family's face. With a type layer selected
and no text being typed, changing the family, size, alignment, or
anti-aliasing SHALL re-set that layer — applying only the changed setting — and
record exactly one "Edit Type Layer" state; selecting a type layer SHALL show
its family, size, and alignment in the bar.

#### Scenario: The bar restyles the selected type layer

- **WHEN** the `tst_type_tools` restyle test commits "iiiiiiii" at 24 px, then with the layer selected sets the size to 48 and the family to the system monospace face
- **THEN** each change records one "Edit Type Layer" state, the text is unchanged, the layer doubles in height and then widens in the monospace face, its font reads back as that family, and reselecting it shows that family and size

#### Scenario: Right-aligned type ends at the click

- **WHEN** the `tst_type_tools` options test sets size 40, right alignment, types at x 180, and clicks Commit
- **THEN** one "Horizontal Type" state is recorded, the layer's right edge is at the click, and Toggle Text Orientation then selects the Vertical Type tool
