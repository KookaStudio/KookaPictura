# Spec Delta

## MODIFIED Requirements

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
