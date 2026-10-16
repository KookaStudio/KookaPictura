## ADDED Requirements

### Requirement: The canvas reuses an unchanged frame

The view SHALL expose a frame revision that changes only when the level-0 frame
is rebuilt whole, never for a region it presented as a `region_blitted`. A full
canvas refresh SHALL NOT rebuild the canvas image when the canvas already holds
the frame at the current revision, and opening a document SHALL build that image
once. A bounded undo or redo SHALL present its region as a blit and then emit
`changed` so panels and overlays update without a full image.

#### Scenario: A bounded undo keeps the frame [cfr_bounded_undo]

- **WHEN** the `tst_large_document` suite paints a stroke, then runs Edit ▸ Undo
  and Step Forward
- **THEN** the document and the canvas show the restored pixels and the frame
  revision is unchanged

#### Scenario: A metadata-only undo refreshes the overlays [cfr_metadata_undo]

- **WHEN** the self-test's crop-group check undoes a slice
- **THEN** the slice overlay shows the restored slices

### Requirement: The Move tool warms only a movable layer

The Move tool SHALL NOT build its drag preview for a layer it may not move (a
Background), since the preview composites the whole document.

#### Scenario: A Background is not warmed [cfr_move_warm]

- **WHEN** the Move tool prepares a preview on a document whose active layer is
  the Background, and then on an added layer
- **THEN** the first prepare refuses without building a base image and the
  second builds one

### Requirement: Layer properties repaint only their layer

Changing the lock or colour label of layers SHALL record history and notify
without compositing. Changing their blend mode, opacity, or fill SHALL repaint
only the union of the changed layers' bounds — a pixel layer's or a type layer's
rect, a masked adjustment's mask rect — and SHALL fall back to a full
recomposite only for a layer whose reach is not its bounds (a group, layer
effects, an unmasked adjustment).

#### Scenario: A lock repaints nothing [cfr_lock]

- **WHEN** the `tst_large_document` suite locks a painted layer's pixels
- **THEN** the lock is set and neither the canvas revision nor the frame
  revision changes

#### Scenario: A blend change repaints a region [cfr_blend]

- **WHEN** the same layer's blend mode is set to Multiply
- **THEN** the canvas revision changes, the frame revision does not, and the
  canvas shows the layer's pixels

### Requirement: A document canvas holds no full-resolution image

A canvas presenting a document SHALL know the document by its size and present
it from view-pyramid crops alone, holding no full-resolution image; a region
the view repaints SHALL only invalidate the crops. The canvas SHALL still yield
the document as one image on request, cropped whole from level 0, and its
single-channel view SHALL be built from the presented crop.

#### Scenario: The canvas keeps no copy of the document [cfr_no_image]

- **WHEN** the `tst_large_document` suite paints on a document
- **THEN** the canvas reports the document's size, holds no image, and the image
  it yields on request shows the painted pixels
