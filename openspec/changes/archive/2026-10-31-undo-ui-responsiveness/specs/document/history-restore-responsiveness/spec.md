## ADDED Requirements

### Requirement: A history restore leaves the panels to the debounce

A history restore that ended no live session SHALL NOT refresh the docked panels
synchronously. The frame SHALL refresh once, and the panels SHALL refresh on the
same debounce a paint commit uses. Tools MUST still drop their local gesture
state. A restore that ended a live session SHALL refresh the panels before it
returns.

#### Scenario: An undo returns before the panels refresh [hrr_undo_debounced]

- **WHEN** a committed stroke is undone with no live session
- **THEN** the undo returns with one command refresh and the Histogram panel
  still showing the painted state, and after the debounce the panel shows the
  restored state

#### Scenario: An undo that cancels a stroke refreshes at once [hrr_cancelled_session]

- **WHEN** an undo cancels a stroke in progress
- **THEN** `sessionCancelled` is reported and the Histogram panel shows the
  restored state when the undo returns

#### Scenario: A restore drops tool gestures [hrr_tool_gesture]

- **WHEN** a restore is reported while a tool holds a gesture preview
- **THEN** the tool drops the preview

### Requirement: Thumbnails do not materialize planes

Layer and mask thumbnails SHALL read only the samples they show. Reading them
from a plane with diverged bands MUST NOT materialize the plane, and their pixels
MUST equal those of the same data in one contiguous plane.

#### Scenario: A diverged plane is sampled in place [hrr_plane_sample]

- **WHEN** every sample of a plane with a forked band is read through `Plane::sample`
- **THEN** each value is the plane's logical sample and no contiguous view is built

#### Scenario: Thumbnails after an undo stay on the bands [hrr_thumbnail_no_materialize]

- **WHEN** thumbnails are built from a layer and mask with diverged bands
- **THEN** no plane is materialized

#### Scenario: Thumbnail pixels do not change [hrr_thumbnail_pixels]

- **WHEN** thumbnails are built from diverged and contiguous copies of a layer,
  including a grayscale layer and a document-positioned thumbnail
- **THEN** the images are identical

### Requirement: A layer-mask restore is bounded

A restore whose only non-composite, non-layer-channel change is in layer masks
SHALL report the rectangle covering the composite tiles and the mask tiles in
document coordinates. A restore that crosses a state placing a mask differently,
or that changes a document channel, SHALL report the whole document.

#### Scenario: A mask edit reports its tiles [hrr_mask_region]

- **WHEN** a mask edit is committed, undone and redone
- **THEN** each restore reports a rectangle of at most two tiles on each side
  covering the edit at its document position, and the document matches byte
  for byte

#### Scenario: A moved mask reports the whole document [hrr_mask_moved]

- **WHEN** an undo crosses a state that moved a mask
- **THEN** the restore reports the whole document

#### Scenario: A jump over a mask move reports the whole document [hrr_mask_walk]

- **WHEN** a jump crosses states that move a mask and move it back
- **THEN** the restore reports the whole document

#### Scenario: A document channel edit reports the whole document [hrr_channel_everywhere]

- **WHEN** an edit to a document channel is undone
- **THEN** the restore reports the whole document
