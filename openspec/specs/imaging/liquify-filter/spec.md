# imaging/liquify-filter Specification

## Purpose
The Liquify mesh and its resampling: warp, twirl, pucker, bloat, and
reconstruct.

## Requirements

### Requirement: Liquify mesh and resampling

The system SHALL implement Liquify as a `LiquifyMesh` holding an inverse displacement per node of a regular grid over the layer, plus a freeze mask, and a `Filter::Liquify { mesh }` variant applied through `pictura_filters::apply`. Resampling SHALL show, at each output pixel `p`, the source at `p + d(p)`, with edge pixels repeating past the layer. An identity mesh, and a mesh whose distortion was fully removed, SHALL leave the layer bit-identical. A mesh built for another layer size SHALL be stretched to fit, scaling its offsets with it.

#### Scenario: An identity mesh is a no-op

- **WHEN** `apply` receives `Filter::Liquify` with an identity mesh
- **THEN** every sample of the buffer is unchanged

#### Scenario: A uniform offset translates exactly

- **WHEN** every node of a mesh holds the offset `(3, 0)`
- **THEN** each output pixel equals the source pixel three columns to its right

### Requirement: Liquify distortion tools

Each brush tool SHALL distort only nodes inside the brush, most strongly at its centre. Forward Warp SHALL move pixels along the drag. Twirl SHALL rotate pixels clockwise about the brush centre, further with each Brush Rate tick, and counterclockwise with Alt. Pucker SHALL move pixels toward the centre and Bloat away from it, each reversed by Alt. Push Left SHALL move pixels left on an upward drag and right on a downward one, reversed by Alt. Mirror SHALL reflect the far side of the stroke onto the side left of it (right with Alt) and leave the far side untouched. Turbulence SHALL scramble pixels smoothly and deterministically. Reconstruct SHALL ease an existing distortion back toward none.

#### Scenario: Forward Warp follows the drag and stays inside the brush

- **WHEN** a 30 px Forward Warp brush drags 12 px to the right
- **THEN** pixels at the end of the drag read from the left, less so toward the brush edge, and nodes beyond the brush keep a zero offset

#### Scenario: Pucker and Bloat move pixels radially

- **WHEN** Pucker is held still over a point
- **THEN** pixels inside the brush move toward that point, and Bloat (or Pucker with Alt) moves them away

### Requirement: Liquify freeze mask and reconstruction

Frozen nodes SHALL take no distortion from any tool. Freeze Mask and Thaw Mask SHALL paint the mask with the brush; Mask All SHALL freeze every node, Mask None thaw every node, and Invert All swap frozen and thawed. Reconstruct with an amount SHALL scale unfrozen offsets toward zero by that percentage, so 0 is a no-op and 100 removes the unfrozen distortion; Restore All SHALL remove every distortion, frozen areas included.

#### Scenario: A frozen layer is protected and Invert All releases it

- **WHEN** Mask All is applied and a Forward Warp stroke is drawn
- **THEN** the mesh stays identity, and after Invert All the same stroke distorts it

#### Scenario: Restore All returns the original

- **WHEN** a distorted, fully frozen mesh is reconstructed at 100 and then restored
- **THEN** Reconstruct leaves the frozen offsets and Restore All leaves an identity mesh whose resample is bit-identical to the source

### Requirement: Liquify processes only an active selection's bounding rectangle

When a selection is active, the system SHALL confine the Liquify session, its
preview, and its commit to the selection's bounding rectangle intersected with
the active layer, expressed in layer coordinates. The dialog SHALL show only
that rectangle, opened at 100 % and centred on it. Every pixel outside it MUST
stay bit-identical in the preview and after the commit. A brush stroke or mesh
node outside it SHALL change nothing. The selection's shape MUST NOT restrict
processing inside the rectangle: a pixel inside it but outside a lasso or
ellipse SHALL be processed. Without a selection the whole layer SHALL be
processed. An empty selection, or one whose bounding rectangle does not
intersect the layer, SHALL refuse to open the dialog. A mesh SHALL carry its
rectangle, so a kept mesh (Load Last Mesh, Last Filter) keeps the confinement
and a mesh stretched to another layer size scales it with the mesh.

#### Scenario: A rectangular selection confines the preview and the commit [lq_selection_rect]

- **WHEN** a rectangular selection is active and a stroke is drawn outside its
  bounding rectangle
- **THEN** the preview shows only that rectangle, the stroke changes no pixel,
  and OK records no history state

#### Scenario: A stroke inside the rectangle changes only inside it [lq_selection_rect_inside]

- **WHEN** a stroke inside the bounding rectangle is committed
- **THEN** pixels inside the rectangle change and every pixel outside it is
  unchanged

#### Scenario: A non-rectangular selection uses its bounding rectangle [lq_selection_bbox]

- **WHEN** an elliptical selection is active and a stroke inside its bounding
  rectangle but outside the ellipse is committed
- **THEN** the pixels under the stroke change and every pixel outside the
  rectangle is unchanged

#### Scenario: A selection that misses the layer refuses the dialog [lq_selection_miss]

- **WHEN** the selection's bounding rectangle does not intersect the active
  layer, or the selection is empty
- **THEN** Liquify does not open and records no history state
