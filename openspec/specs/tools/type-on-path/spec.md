# tools/type-on-path Specification

## Purpose
Type set along an open or closed path, area type inside a closed path, and
their persistence through the PSD `Txt2` text-frame data.

## Requirements

### Requirement: A Type click on a path starts path type

A Horizontal or Vertical Type or Type Mask click that lands on the Work Path or
a saved path SHALL start a type session on that path, unless it lands on an
existing type layer (which reopens as before). The text SHALL start at the
path point nearest the click and flow in anchor order by arc length. The
session SHALL preview the text on the path and SHALL record nothing until
commit.

#### Scenario: Clicking an open path starts on-path type [path_click]

- **WHEN** `tst_type_on_path::clickOnOpenPathStartsOnPath` creates a horizontal
  two-anchor Work Path, clicks its middle with Horizontal Type, and types
- **THEN** the session is active, its caret stops follow the path, the preview
  has ink on the path, and no history state is recorded before commit

#### Scenario: Text starts at the clicked arc length [path_start_click]

- **WHEN** `pictura-render::path_text_starts_at_click_arc` lays text out from a
  start offset half way along a horizontal path
- **THEN** the first glyph's pen is at the half-way point and later glyphs
  advance along the path

#### Scenario: Path hit-testing prefers the edge over the interior [path_hit]

- **WHEN** `pictura-render::path_hit_prefers_edge_then_inside` probes a closed
  path near its edge and well inside it
- **THEN** the edge probe reports on-path type with the nearest arc, and the
  interior probe reports area type

### Requirement: Path text overflows at the path end

On-path text SHALL paint only glyphs whose start lies within the path. An open
path SHALL end at its last anchor; a closed path SHALL wrap once from the
click. When the text is longer than the available path, the session SHALL
report overflow and draw a `+` at the path end point.

#### Scenario: A plus marks overflow at the path end [path_overflow]

- **WHEN** `tst_type_on_path::overflowPlusAtPathEnd` types a long word on a
  short path
- **THEN** the overlay reports overflow with its marker at the path end, and
  the committed layer's rect stays within the path's ink bounds

#### Scenario: A closed path wraps once [path_closed_wrap]

- **WHEN** `pictura-render::path_text_wraps_closed_path` lays text longer than
  the remaining arc but shorter than the full circumference on a closed path
- **THEN** the glyphs continue past the path start, and only text longer than
  the circumference reports overflow

### Requirement: Clicking inside a closed path sets area type

A Type or Type Mask click inside a closed path SHALL start area type. The text
SHALL wrap to the path's bounding box and SHALL be clipped to the path
interior. The box SHALL show the path outline and SHALL report paragraph
overflow. Area type SHALL be an approximation of Photoshop's line-to-shape
reflow.

#### Scenario: Clicking inside a closed path makes area type [area_click]

- **WHEN** `tst_type_on_path::clickInsideClosedPathMakesAreaType` creates a
  closed rectangular Work Path, clicks its centre, types text wider than the
  path, and commits
- **THEN** one type layer is added, its text path kind is area, its text wraps
  within the path's bounding box, and no ink is painted outside the path

#### Scenario: Area ink is clipped to the path [area_clip]

- **WHEN** `pictura-render::area_text_clips_to_closed_path` renders a word
  inside a narrow triangular closed path
- **THEN** every painted pixel lies inside the path, while the same text in a
  rectangular path paints the corners the triangle excludes

### Requirement: The Type tool keeps the path visible and signals path entry

While a Type or Type Mask tool is active, the Work Path SHALL stay visible on
the canvas, and hovering its edge SHALL show the type-on-path cursor instead of
the plain I-beam.

#### Scenario: The path stays visible and shows the path cursor [path_visible]

- **WHEN** `tst_type_on_path::clickOnOpenPathStartsOnPath` draws a Work Path,
  selects Horizontal Type, and hovers its middle
- **THEN** the path overlay still lists the path's anchors, and the pointer zone
  is the type-on-path cursor

### Requirement: Path type moves and transforms with the layer

Moving a path type layer SHALL move its stored path with the layer. Free
Transform SHALL transform the path and scale the type size, keeping the layer
editable path type; reopening and saving the layer SHALL keep the transformed
path.

#### Scenario: Moving and transforming a path type layer [path_transform]

- **WHEN** `tst_type_on_path::moveAndTransformOnPath` commits text on a path,
  moves the layer with the Move tool, then scales it with Free Transform
- **THEN** the stored path points follow the move, the path stays on-path type,
  and the type size grows with the path

### Requirement: Path type commits, reopens, and masks

Commit SHALL add a type layer whose text path is the session's path, recording
one "Horizontal Type" / "Vertical Type" state. Reopening that layer with a Type
tool SHALL edit it on the same path and record one "Edit Type Layer" state on
commit. The Type Mask tools SHALL use the same path coverage for the selection.

#### Scenario: Commit and reopen an on-path layer [path_commit]

- **WHEN** `tst_type_on_path::commitAndReopenOnPath` types on a path, commits,
  clicks the layer, types more, and commits
- **THEN** each commit records one state, the layer remains a type layer with
  the same path, and the second commit changes its text

#### Scenario: Type Mask follows the path [path_mask]

- **WHEN** `tst_type_on_path::typeMaskFollowsPath` commits a Horizontal Type
  Mask on a path
- **THEN** the selection contains the path glyphs and no pixels away from the
  path

### Requirement: Path type round-trips through PSD

A path type layer's path SHALL be written to the document-level `Txt2`
`ResourceDict.TextFrameSet` entry for its text layer (`Type` 2 for on-path,
`Type` 1 for area, cubic control points, start offset). Opening the file SHALL
restore the path so the layer re-renders and reopens on it.

#### Scenario: The text frame set survives encode and decode [path_psd]

- **WHEN** `pictura-codec::text_path_round_trips` writes a document with one
  on-path and one area type layer, reads it back, and compares the layers
- **THEN** each layer's path kind, closed flag, control points, and start
  offset match the authored values

#### Scenario: Reading an authored TextFrameSet [path_psd_read]

- **WHEN** `pictura-codec::reads_text_frame_set_entries` decodes a `Txt2`
  `TextFrameSet` with an on-path and an area entry
- **THEN** the first is on-path with its start fraction and control points and
  the second is area type with its closed control points

### Requirement: Baseline Shift moves path type

Path type SHALL apply the character run's Baseline Shift opposite the path
normal: a positive shift moves the glyphs and caret stops opposite the normal
by the run's rise (up for a horizontal path), and a negative shift moves them
along the normal. The rendered alpha bounds and the layer's text bounds SHALL
follow the shifted glyph positions.

#### Scenario: Baseline Shift moves path glyphs and the caret [tp_baseline_shift]

- **WHEN** the same path text is rendered with Baseline Shift 0 and with a
  positive shift
- **THEN** every glyph and caret stop moves by that distance opposite the path
  normal, and the alpha bounds move with them
