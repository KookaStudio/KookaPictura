## ADDED Requirements

### Requirement: Shear dialog

`Filter ▸ Distort ▸ Shear…` and `Filter ▸ Last Filter Settings` (when the last filter was Shear) SHALL open CS6's Shear dialog, not the six-slider form. The dialog SHALL show a square curve box ruled by a dotted 4×4 guide, with a smooth line running from the top edge to the bottom edge. Clicking the line SHALL insert a control point at the click; dragging a point SHALL move it; dragging an interior point out of the box SHALL remove it; the two end points SHALL stay on the top and bottom edges and SHALL slide horizontally only, and interior points SHALL stay between their neighbours. The box SHALL show one handle per control point the user placed, never a sampled lattice, and SHALL refuse points past `SHEAR_MAX_POINTS` (8). The line SHALL be a cubic Hermite through the points with Catmull-Rom tangents, so a two-point curve is a straight line and a multi-point curve bends without kinks. Below the box SHALL be an `Undefined Areas:` group of two radio buttons, `Wrap Around` first and `Repeat Edge Pixels` second, with `Wrap Around` checked. OK and Cancel SHALL stand to the right of the box. A live preview of the picture SHALL sit below, with no Preview checkbox and no zoom controls, as CS6's Shear has none; the pane SHALL composite the filtered active layer over the unchanged layers, so the other layers are never displaced with it. The dialog SHALL produce the `shear` slots in order: a control-point count (`2..=SHEAR_MAX_POINTS`), then that many `(position, offset)` pairs top to bottom with unused pairs zero, then the fill index (`0` Wrap Around, `1` Repeat Edge Pixels). OK SHALL commit one history state and Cancel SHALL restore the canvas bit-identically. A Shear canvas preview SHALL always render against the whole layer, never the visible crop.

#### Scenario: The dialog follows CS6's layout and defaults

- **WHEN** the Shear dialog opens on a document
- **THEN** its values are a count of 2, the two end points `(-1.0, 0.0)` and `(1.0, 0.0)`, zero padding, and fill `0`, the curve is a straight line down the middle of the box with exactly two handles, the two `Undefined Areas` radios read `Wrap Around` and `Repeat Edge Pixels` with `Wrap Around` checked, the preview sits below the curve box, and no Preview checkbox or zoom control is present

#### Scenario: The curve box edits the control points

- **WHEN** a point is dragged to the right at the bottom of the curve box, and then a point is added midway and dragged out of the box
- **THEN** the bottom point's stored offset becomes positive, the top point's stays zero, the count grows by one and returns, and the removed point's effect disappears from the points

#### Scenario: The curve keeps the points the user placed

- **WHEN** a point is clicked onto the line and not dragged
- **THEN** the stored count grows by exactly one, the new `(position, offset)` pair is the click, and the other slots stay zero; clicking past the cap leaves the count at `SHEAR_MAX_POINTS`

#### Scenario: The control points round-trip through the slots

- **WHEN** the dialog is opened with a previously committed 18-slot value list
- **THEN** the curve box reconstructs exactly the stored count and `(position, offset)` pairs, and the radios show the stored fill

#### Scenario: Commit, reopen, and cancel

- **WHEN** the dialog is opened from the Filter menu, the curve is bent, and OK is pressed, and Last Filter Settings is then reopened, and a separate dialog is cancelled after a preview
- **THEN** one history state is added carrying the 18 committed slots, the reopened dialog reads the same slots, and the cancelled canvas equals its prior pixels

#### Scenario: The preview is rendered against the whole layer

- **WHEN** Shear is previewed with a visible viewport smaller than the layer
- **THEN** the preview equals the full-layer commit inside the viewport

#### Scenario: The pane filters only the active layer

- **WHEN** the dialog's bottom pane previews Shear on a document with more than one visible layer
- **THEN** the pane shows the filtered active layer composited over the unchanged layers, so the other layers are not displaced with it
