## ADDED Requirements

### Requirement: Move preview keeps the layer stack order

While the Move tool drags layers, the canvas SHALL draw the layers above the
moving group over the moving image, so a layer that sits below other layers does
not hide them for the length of the drag. The above-layer image SHALL cover the
visible document rectangle at the presented level, SHALL be built only when the
moving group has layers above it, and MUST be built with the moving group and
every layer below it hidden. A drag of the topmost layer MUST NOT build it.

#### Scenario: A layer below others stays under them [lct_move_preview_stack]

- **WHEN** a layer below an opaque layer is dragged so the moving content
  overlaps the opaque layer
- **THEN** the opaque layer still shows over the moving content at the overlap,
  the moving content shows outside it, and the layer's old place shows the
  backdrop

#### Scenario: The topmost layer builds no above-layer image [lct_move_preview_top]

- **WHEN** the topmost layer of a document is dragged
- **THEN** no above-layer image is prepared and the preview is unchanged

#### Scenario: A linked set keeps the layers above it [lct_move_preview_link_stack]

- **WHEN** a linked set below an opaque layer is dragged over it
- **THEN** the opaque layer still shows over the moved set at the overlap
