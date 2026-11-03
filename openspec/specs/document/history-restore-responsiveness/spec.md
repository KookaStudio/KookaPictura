# document/history-restore-responsiveness Specification

## Purpose
What a history restore may do on the GUI thread, so that undo, redo and history
jumps keep the pointer responsive.

## Requirements

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

### Requirement: A layer-mask restore reports the composite

A restore that changes layer masks SHALL report the rectangle covering the
composite tiles. A restore in one geometry segment that places no mask
differently SHALL also cover its mask tiles in document coordinates. A soloed
mask SHALL show the restored mask after any restore, including one that moved,
removed or reordered masks.

#### Scenario: A mask edit reports its tiles [hrr_mask_region]

- **WHEN** a mask edit is committed, undone and redone
- **THEN** each restore reports a rectangle of at most two tiles on each side
  covering the edit at its document position, and the document matches byte
  for byte

#### Scenario: A moved mask reports the composite [hrr_mask_moved]

- **WHEN** an undo crosses a state that moved a mask
- **THEN** the restore reports the composite's changed rectangle and that masks
  changed, and the document matches byte for byte

#### Scenario: A jump over a mask move reports the composite [hrr_mask_walk]

- **WHEN** a jump crosses states that move a mask and move it back
- **THEN** the restore reports the composite's changed rectangle and that masks
  changed

#### Scenario: A deleted masked layer stays bounded [hrr_mask_delete]

- **WHEN** the delete of a masked layer is undone
- **THEN** the restore reports the composite's changed rectangle, not the whole
  document

#### Scenario: A soloed mask follows the restore [hrr_mask_solo]

- **WHEN** a restore that reports changed masks lands while a mask is soloed
- **THEN** the solo view shows the restored mask

### Requirement: A document channel does not widen a restore

No canvas view draws a document channel, so a change to one SHALL NOT add to the
reported rectangle. The panels SHALL still refresh on the debounce.

#### Scenario: A document channel edit reports no rectangle [hrr_channel_empty]

- **WHEN** an edit to a document channel is undone
- **THEN** the restore reports an empty rectangle and the document matches byte
  for byte

#### Scenario: A crossing with a changed channel stays bounded [hrr_channel_crossing]

- **WHEN** an undo crosses a layer change that also changed a document channel
- **THEN** the restore reports only the composite's changed rectangle

### Requirement: A restore that changes no pixel leaves the display

A restore whose composite did not change SHALL report an empty rectangle, SHALL
NOT rebuild the frame or the pyramid, and SHALL leave the panels to the debounce.

#### Scenario: A reorder of equal layers changes nothing [hrr_no_pixel_reorder]

- **WHEN** a reorder that leaves the picture unchanged is undone
- **THEN** the restore reports an empty rectangle

#### Scenario: A new layer undo keeps the frame [hrr_no_pixel_frame]

- **WHEN** a neutral New Layer is undone
- **THEN** the frame revision is unchanged and the layer is gone from the panels
  after the debounce

### Requirement: A history jump presents the stored composite

A whole-document history jump SHALL present the target state's stored composite
without compositing its layers, and that composite MUST equal a full composite of
the target state. A later bounded edit's undo SHALL stay bounded.

#### Scenario: A jump's composite is exact [hrr_jump_oracle]

- **WHEN** a jump crosses structural, geometry and mask edits
- **THEN** the stored composite equals a full composite of the target

#### Scenario: A named snapshot taken mid-stroke stores an exact composite [hrr_snapshot_mid_stroke]

- **WHEN** a named snapshot is created while a stroke is live
- **THEN** its stored composite equals a full composite of its layers, and the
  live document keeps its pre-stroke composite

#### Scenario: An undo after a jump stays bounded [hrr_jump_then_undo]

- **WHEN** a stroke is committed and undone after a whole-document jump
- **THEN** the undo reports a rectangle

### Requirement: A large whole-document restore builds its pyramid off the GUI thread

A whole-document restore of a document over 16 MP SHALL install level 0 and a
coarse level at or under 4 MP on the GUI thread, SHALL present the coarse level,
and SHALL build the exact pyramid on a worker. The exact pyramid MUST equal a
synchronous rebuild. A result for a superseded restore SHALL be dropped. A stroke
preview, a display preview or a pyramid update while the build is pending SHALL
first build the exact pyramid synchronously. A document at or under 16 MP SHALL
rebuild synchronously.

#### Scenario: The coarse level matches the exact level's size [hrr_coarse_levels]

- **WHEN** a coarse pyramid is built
- **THEN** it has the exact pyramid's level sizes, and its first filled level
  holds the level-0 pixels at its texel corners

#### Scenario: A large undo lands the exact pyramid [hrr_deferred_lands]

- **WHEN** a resize of a large document is undone
- **THEN** the canvas presents the coarse level, and after `pyramidSettled` it
  presents the zoom-selected level with bytes equal to a synchronous rebuild

#### Scenario: A stale pyramid is dropped [hrr_deferred_stale]

- **WHEN** a worker result lands after a later restore superseded it
- **THEN** the result is dropped and the pending restore stays deferred

#### Scenario: A write settles the pyramid [hrr_deferred_settle]

- **WHEN** a pyramid update arrives while a build is pending
- **THEN** the pyramid is exact afterwards and the pending result is dropped

#### Scenario: A small document stays synchronous [hrr_small_sync]

- **WHEN** a whole-document restore lands on a document at or under 16 MP
- **THEN** nothing is deferred

#### Scenario: The pointer stays responsive during a large restore [hrr_pointer_manual]

- **WHEN** the mouse moves across an undo of a resize on a 16507×16196 document
- **THEN** the brush ring keeps following the pointer
