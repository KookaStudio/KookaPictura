# tools/magnetic-lasso Specification

## Purpose
The Magnetic Lasso: an edge-snapping live wire (Width, Contrast, Frequency) with click-to-fasten, automatic fastening, close, Delete, and Escape, committed through the shared lasso path.

## Requirements

### Requirement: Edge-cost live wire

`pictura_select` SHALL expose `EdgeMap::from_buffer(buffer, contrast)` and
`EdgeMap::trace(from, to, width)`. The field SHALL be built from the buffer's
luma (1–2 planes read as gray, 3+ as RGB) with a Sobel operator, gradients below
`contrast` % (clamped to 1–100) of the image's peak SHALL count as no edge, and
the one-pixel border SHALL be treated as flat. `trace` SHALL return an
8-connected path including both endpoints that minimises edge cost within the
segment's bounding box grown by `width` px, and SHALL return a straight
(Bresenham) line when an endpoint is off-canvas, when no edges exist, or when
the search corridor exceeds 2²⁰ pixels. This approximates Photoshop's closed
cost function and SHALL NOT be claimed as pixel parity.

#### Scenario: A flat image traces straight

- **WHEN** `trace` runs on a uniform image
- **THEN** the path is the straight segment between the endpoints

#### Scenario: The wire snaps to a nearby edge

- **WHEN** both endpoints lie beside a strong vertical edge within `width`
- **THEN** the path reaches the edge column and still starts and ends at the given points

#### Scenario: Width bounds the search

- **WHEN** the edge lies farther from the segment than `width`
- **THEN** the path stays within `width` px of the straight segment

#### Scenario: Contrast ignores weak edges

- **WHEN** a weak edge normalises below a 60 % contrast threshold
- **THEN** the path does not move toward it

#### Scenario: Off-canvas endpoints fall back

- **WHEN** an endpoint lies outside the image
- **THEN** `trace` returns the straight Bresenham line between the endpoints

### Requirement: Magnetic Lasso tool interaction

The Magnetic Lasso SHALL be an implemented tool. The first click SHALL set a
fastening point and build the edge field from the visible composite (refused
with a message on a 32-bit document); while open, the pointer SHALL drag a live
wire from the last fastening point along the cheapest path within Width. A click
SHALL fasten the wire at the pointer. With Frequency `f > 0` a pointer move
SHALL fasten automatically once the wire reaches `max(8, 108 − f)` px; with
Frequency 0 it SHALL NOT. Clicking the first point, double-clicking, or Enter
SHALL close the outline and commit it through the shared lasso path (combine
mode, feather, one history state). Delete / Backspace SHALL remove the last
fastening point and its segment, abandoning the trace when only the first
remains. Escape SHALL discard the outline, leaving the selection and history
unchanged. A press inside a live selection while an outline is open SHALL
extend the outline, not start a selection move.

#### Scenario: Clicks inside an edge close into a snapped selection

- **WHEN** the `magnetic_lasso` self-test clicks the four corners two pixels inside a black square on white and clicks the first point again
- **THEN** exactly one history state is added and the selection includes the square's edge columns and row that a straight outline through the clicks would miss, and nothing outside the square

#### Scenario: Delete peels fastening points back

- **WHEN** three points are fastened and Delete is pressed three times
- **THEN** the outline stays open after two deletes and is abandoned after the third, with no history state and no selection

#### Scenario: Escape leaves the selection untouched

- **WHEN** an outline is open over an existing selection and Escape is pressed
- **THEN** the selection and history equal their pre-trace state

#### Scenario: Frequency controls automatic fastening

- **WHEN** a 15 px wire is drawn from a single fastening point at Frequency 100 and at Frequency 0
- **THEN** Frequency 100 has fastened an extra point and Frequency 0 has not

### Requirement: Magnetic Lasso options

The options bar SHALL show Width (1–256 px, default 10), Contrast (1–100 %,
default 10), and Frequency (0–100, default 57) for the Magnetic Lasso, beside
the shared mode buttons, Feather, and Anti-alias, and a disabled Stylus
Pressure checkbox. With the Magnetic Lasso active, `]` / `[` SHALL increase /
decrease Width by exactly 1 px, clamped to 1–256, and the Width field SHALL
follow. The options bar's minimum width SHALL come from the active tool's page
only.

#### Scenario: Bracket keys step Width

- **WHEN** Width is 10 and `]` then `[` are applied
- **THEN** Width becomes 11 (the options-bar field shows 11) and then 10

### Requirement: Magnetic Lasso live outline preview

While a Magnetic Lasso outline is open, the canvas SHALL preview it as a closed,
dashed outline made of the fastened path, the live wire, and the cursor point,
closed by a straight connector back to the origin (the first fastening point).
The origin SHALL be marked with a hollow square 5 screen pixels wide at any
zoom. Closing, cancelling, or abandoning the outline SHALL clear both the
outline and the marker.

#### Scenario: The open outline closes back to a marked origin

- **WHEN** the first point is placed and the pointer moves away
- **THEN** the preview is a closed outline and the origin marker is shown

#### Scenario: Cancelling clears the marker

- **WHEN** the outline is abandoned by deleting its first fastening point
- **THEN** neither the preview nor the origin marker remains

### Requirement: Magnetic Lasso click deselects

When a selection exists and a Magnetic Lasso outline in New mode closes with
fewer than three points (for example a double-click on one spot), the system
SHALL run Deselect as one "Deselect" history state. In Add, Subtract, or
Intersect mode such an outline SHALL leave the selection and history unchanged.

#### Scenario: A double-click outside the selection deselects

- **WHEN** a selection exists and the Magnetic Lasso double-clicks one spot outside it
- **THEN** the selection is cleared as one "Deselect" state

### Requirement: Anti-aliased magnetic lasso commit
The Magnetic Lasso SHALL commit its fastened outline through the shared lasso
path with the tool-time Anti-alias setting. With Anti-alias on, the committed
polygon boundary SHALL hold fractional coverage so that boundary pixels MAY
have values strictly between 0 and 255. With it off, every coverage value SHALL
be 0 or 255. This SHALL NOT change Width, Contrast, Frequency, automatic
fastening, Delete, Escape, or the close-by-click behavior.

#### Scenario: Magnetic Lasso Anti-alias produces fractional coverage [mag_aa]
- **WHEN** a Magnetic Lasso outline closes with Anti-alias on and then again with it off
- **THEN** the anti-aliased mask contains boundary values strictly between 0 and 255, while the mask with Anti-alias off contains only 0 and 255
