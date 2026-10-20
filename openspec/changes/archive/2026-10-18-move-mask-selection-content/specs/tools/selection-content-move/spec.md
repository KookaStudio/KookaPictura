# Spec Delta

## ADDED Requirements

### Requirement: Mask edit target receives the content move

When a raster layer mask is the mask edit target and a selection is active, the
Move tool's selection drag and the selection tools' Ctrl content move SHALL move
the mask's coverage instead of the layer's pixels. The coverage under the
selection origin SHALL be translated by the drag offset inside the mask
rectangle; the vacated source coverage SHALL be moved toward the luma of the mask
background colour by the source selection coverage, and the moved coverage SHALL
be composited at the destination using the source selection coverage as its
alpha. With Alt (duplicate), the source coverage SHALL remain and the moved
coverage SHALL be composited at the destination. While a raster layer mask is
the edit target, this mask-target behavior supersedes the pixel content-move
requirements (Move selected pixels, Temporary content move with a selection
tool, and the Alt duplicate/preview requirements): a mask-target Alt move SHALL
NOT insert a layer, SHALL NOT change the active layer, and SHALL NOT show the
pixel-clone preview. A move SHALL leave the layer's pixels, the mask attachment,
and the mask rectangle unchanged. A committed move SHALL record exactly one
`"Move Selection"` history state and the selection SHALL sit at the drag offset;
a zero offset SHALL record nothing.

#### Scenario: The selection moves mask coverage [mask_content_move]

- **WHEN** a raster layer mask is the mask edit target, a selection exists, and the Move tool drags the selection by `(dx, dy)`
- **THEN** the mask coverage under the selection origin is translated by `(dx, dy)`, the layer's pixels are unchanged, and the mask remains attached with the same rectangle

#### Scenario: The vacated area takes the mask background [mask_content_move_background]

- **WHEN** the mask background colour is black and a fully selected white mask patch is dragged away
- **THEN** the patch's former area holds black coverage and the layer is hidden there

#### Scenario: Alt copies mask coverage [mask_content_move_duplicate]

- **WHEN** the mask content move is made with Alt held
- **THEN** the original coverage remains, a copy of it is composited at the drag offset, no layer is inserted, and the active layer and the drag preview stay unchanged

#### Scenario: Ctrl moves mask coverage with a selection tool [mask_content_move_ctrl]

- **WHEN** a mask edit target is active, a selection tool is active, and Ctrl is held while dragging inside the selection
- **THEN** the mask coverage moves by the drag offset and no new selection is started

#### Scenario: One history state per mask move [mask_content_move_history]

- **WHEN** a mask content move commits by a non-zero offset
- **THEN** exactly one `"Move Selection"` history state is recorded and the active selection sits at the offset

#### Scenario: A zero offset records nothing [mask_content_move_noop]

- **WHEN** the mask content move ends with a zero offset
- **THEN** the mask is unchanged and no history state is recorded
