# selection-content-move Specification

## Purpose
Moves and duplicates selected pixels or mask coverage, with a temporary move and single-undo records.

## Requirements

### Requirement: Move selected pixels
When a selection is active, the Move tool SHALL drag the selected pixels rather
than the whole layer: the covered pixels are cleared from the source layer and
re-created at the dragged offset inside that same layer, and the selection mask
moves with them by the same offset. With no active selection, the Move tool
SHALL continue to move the whole layer.

#### Scenario: Drag moves the selected pixels
- **WHEN** a selection is active and the Move tool is dragged by `(dx, dy)`
- **THEN** the selected pixels appear offset by `(dx, dy)`, the source area is cleared, and the layer count is unchanged

#### Scenario: The selection follows the pixels
- **WHEN** the Move tool commits a selected-pixel drag by `(dx, dy)`
- **THEN** the active selection mask is translated by `(dx, dy)`

#### Scenario: No selection still moves the layer
- **WHEN** no selection is active and the Move tool is dragged
- **THEN** the whole layer moves, as before

### Requirement: Duplicate selected pixels to a new layer

Holding Alt while the Move tool drags a selection SHALL copy the selected pixels
into a new layer directly above the source and move that new layer, leaving the
source layer intact. A selection tool with Ctrl+Alt held SHALL perform the same
duplicate. The duplicate SHALL be a single layer inserted above the source.
**The duplicate SHALL be previewed live during the drag** (the dragged content
moves under the cursor before release) **and SHALL become the active layer on
commit**, so the next edit targets the clone. The duplicate SHALL record exactly
one history state, on commit, and no state during the drag; a zero-offset
Alt-drag SHALL record nothing.

#### Scenario: Move tool Alt-drag duplicates

- **WHEN** a selection is active and the Move tool is dragged with Alt held
- **THEN** the selected pixels are copied to a new layer that moves with the drag and the source layer keeps its pixels

#### Scenario: The clone previews during the drag [lcm_alt_preview]

- **WHEN** a selection is active and the Move tool is dragged with Alt held before release
- **THEN** the duplicate content follows the cursor in the live preview and the source layer is unchanged until commit

#### Scenario: The clone becomes active [lcm_alt_active]

- **WHEN** the Alt-drag commits
- **THEN** the duplicated layer is the active layer and exactly one history state is recorded

#### Scenario: Select tool Ctrl+Alt duplicates

- **WHEN** a selection exists and a selection tool is dragged inside it with Ctrl+Alt held
- **THEN** the selected pixels are copied to a new layer that moves with the drag

### Requirement: Temporary content move with a selection tool
Holding Ctrl while a selection tool drags inside the active selection SHALL move
the selected pixels exactly as the Move tool does, without changing the active
tool. Without Ctrl (and without Shift or Alt) a press inside the selection SHALL
keep the existing behaviour of moving only the selection outline.

#### Scenario: Ctrl moves the pixels
- **WHEN** a selection is active, a selection tool is active, and Ctrl is held while dragging inside the selection
- **THEN** the selected pixels move by the drag offset and no new selection is started

#### Scenario: Plain drag still moves only the outline
- **WHEN** a selection is active, a selection tool is active, and no modifier is held during a drag inside the selection
- **THEN** only the selection outline moves and the pixel content is unchanged

#### Scenario: A quick-mode modifier does not move
- **WHEN** a selection is active and a selection tool drags inside it with Shift or Alt held but not Ctrl
- **THEN** no content move occurs and the gesture combines a new selection per the quick mode

### Requirement: Content move records one undo state
A completed content move (move or duplicate) SHALL record exactly one history
state labelled `Move Selection`; a zero-offset drag, a missing selection, or a
missing document SHALL record nothing.

#### Scenario: One state per move
- **WHEN** a selected-pixel move or duplicate completes
- **THEN** exactly one `Move Selection` state is added to the history

#### Scenario: No-op records nothing
- **WHEN** a content-move gesture ends with a zero offset, or there is no selection
- **THEN** no history state is recorded and the document is unchanged

### Requirement: Alt content-move previews the duplicated pixels

The Move tool SHALL show the duplicated pixels in the live preview during an Alt
content move with a selection, and they SHALL follow the pointer during the drag,
not only the selection outline. Releasing SHALL copy the
selected pixels to a new layer offset by the drag, record exactly one
`"Move Selection"` undo state, and leave the new layer active. Cancelling SHALL
restore the document bit-identically with no history state.

#### Scenario: The clone follows the cursor [lcm_alt_preview_pixels]

- **WHEN** a selection exists and an Alt Move drag is in progress before release
- **THEN** the duplicated pixels are drawn at the dragged offset during the drag,
  not only the selection outline

#### Scenario: Commit makes the clone active with one state [lcm_alt_preview_commit]

- **WHEN** the Alt content-move drag is released after a non-zero offset
- **THEN** the copy is a new active layer and exactly one `"Move Selection"`
  state was recorded

#### Scenario: Cancel is side-effect free [lcm_alt_preview_cancel]

- **WHEN** the Alt content-move drag is cancelled before release
- **THEN** the document is bit-identical to the pre-drag state and no history
  state was added

### Requirement: Mask edit target receives the content move

When a raster layer mask is the mask edit target and a selection is active, the
Move tool's selection drag and the selection tools' Ctrl content move SHALL move
the mask's coverage instead of the layer's pixels. Coverage under the selection
origin SHALL be translated by the drag offset without clipping the destination
to the old mask rectangle or the canvas. The vacated source coverage SHALL be
blended toward 0 by the source selection coverage, independently of the color
swatches, and moved coverage SHALL be composited at the destination using the
source selection coverage as its alpha. With Alt (duplicate), the source
coverage SHALL remain and moved coverage SHALL be composited at the destination.
The stored rectangle SHALL grow when needed to retain destination coverage,
including nonzero partial values outside the canvas; unrelated off-canvas
samples SHALL remain unchanged and newly uncovered exterior SHALL be black.
While a raster layer mask is the edit target, this behavior SHALL supersede the
pixel content-move requirements (Move selected pixels, Temporary content move
with a selection tool, and the Alt duplicate/preview requirements): a mask-target
Alt move SHALL NOT insert a layer, change the active layer, or show the
pixel-clone preview. A move SHALL leave layer pixels and the mask attachment
unchanged. A committed move SHALL record exactly one `"Move Selection"` history
state and the visible selection SHALL sit at the offset, subject to its existing
canvas clipping. A zero offset SHALL record nothing and SHALL NOT change bounds.

#### Scenario: The selection moves mask coverage [mask_content_move]

- **WHEN** a raster layer mask is the mask edit target, a selection exists, and the Move tool drags the selection by `(dx, dy)`
- **THEN** mask coverage under the selection origin is translated by `(dx, dy)`, layer pixels are unchanged, and the mask remains attached with bounds expanded only when needed

#### Scenario: The vacated area takes the mask background [mask_content_move_background]

- **WHEN** the mask background swatch is white and a fully selected white mask patch is dragged away
- **THEN** the patch's former area holds black coverage and the layer is hidden there

#### Scenario: Alt copies mask coverage [mask_content_move_duplicate]

- **WHEN** the mask content move is made with Alt held
- **THEN** original coverage remains, a copy is composited at the drag offset including beyond the canvas, no layer is inserted, and the active layer and pixel-clone preview stay unchanged

#### Scenario: Ctrl moves mask coverage with a selection tool [mask_content_move_ctrl]

- **WHEN** a mask edit target is active, a selection tool is active, and Ctrl is held while dragging inside the selection
- **THEN** mask coverage moves by the drag offset and no new selection is started

#### Scenario: One history state per mask move [mask_content_move_history]

- **WHEN** a mask content move commits by a non-zero offset
- **THEN** exactly one `"Move Selection"` history state is recorded and the visible selection sits at the offset subject to canvas clipping

#### Scenario: A zero offset records nothing [mask_content_move_noop]

- **WHEN** the mask content move ends with a zero offset
- **THEN** mask samples and bounds are unchanged and no history state is recorded

#### Scenario: Selected coverage survives an off-canvas destination [cmd_selected_offcanvas]

- **WHEN** selected gray and white mask coverage is moved completely beyond a canvas edge and the mask is subsequently translated to bring that retained region back
- **THEN** the destination samples still exist with their moved values and the move's vacated source region is black

#### Scenario: Partial selections clear toward black [cmd_selected_partial]

- **WHEN** mask coverage of 200 under selection coverage of 128 is moved to a disjoint black destination with a white background swatch
- **THEN** source coverage is 100 and destination coverage is 100 under the existing rounded 8-bit blending rule
