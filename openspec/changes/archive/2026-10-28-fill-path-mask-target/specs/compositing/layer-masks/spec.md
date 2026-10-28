# compositing/layer-masks — delta

## MODIFIED Requirements

### Requirement: Canvas tools paint the active mask

With a mask edit target active, the canvas Brush and Pencil (`begin_brush`) SHALL
paint the mask's coverage with the foreground colour's Rec.601 luma, and the
Eraser (`begin_eraser`) SHALL paint it with the background colour's luma, each
recording one history state with its normal label ("Brush", "Pencil",
"Eraser") and leaving the layer's pixels unchanged. Erase To History SHALL be
refused on a mask. Painting tools that cannot paint a mask and the Magic Eraser
SHALL refuse while a mask is the target, recording no history state and leaving
the layer's pixels unchanged. The Gradient, the Paint Bucket, `Edit ▸ Stroke`,
and the fills that share the layer-fill path SHALL instead fill the mask's
coverage, as the next requirement says.

#### Scenario: The Brush tool hides the layer through its mask

- **WHEN** a mask edit target is active and `begin_brush` paints a black foreground
- **THEN** one "Brush" state is recorded, the composite shows the layer hidden there, and deleting the mask shows the layer's pixels unchanged

#### Scenario: The Eraser reveals with the background colour

- **WHEN** a mask edit target is active and `begin_eraser` paints over hidden coverage with a white background
- **THEN** one "Eraser" state is recorded and the layer shows again

#### Scenario: A tool without mask support refuses

- **WHEN** a mask edit target is active and the Pattern Stamp or the Magic Eraser starts
- **THEN** it is refused and no history state is recorded

## ADDED Requirements

### Requirement: The shared fill path fills the active mask

With a mask edit target active, the Gradient, the Paint Bucket, `Edit ▸ Stroke`,
Fill Path, Stroke Path, and the shape tools' Fill Pixels SHALL fill the mask's
coverage through the selection instead of the layer's pixels. The Gradient's
ramp, the Paint Bucket's colour or pattern, and every fill colour SHALL be
reduced to its Rec. 601 luma; a path's or a shape's coverage SHALL be reduced to
its luma and confined to the mask's rectangle; and the gradient's drag and the
bucket's click SHALL be read in the document's coordinates. Each operation SHALL
record exactly one history state with its normal label ("Gradient", "Paint
Bucket", "Stroke", "Fill Path", "Stroke Path", or the shape tool's label) and
SHALL leave the layer's pixels unchanged, whatever the owning layer's pixel
lock. Without a mask edit target each SHALL edit the layer's pixels exactly as
before.

#### Scenario: The fill tools write the mask [lm_mask_fill_path]

- **WHEN** a mask edit target is active and the Gradient, the Paint Bucket, `Edit ▸ Stroke`, Fill Path, Stroke Path, and a shape's Fill Pixels each run
- **THEN** each writes the mask's coverage, records its own state, and leaves the layer's pixels unchanged

#### Scenario: A pixel-locked layer's mask fills [lm_mask_fill_path_locked]

- **WHEN** a text layer's mask is the edit target and the Gradient drags across it
- **THEN** the mask's coverage takes the ramp and one "Gradient" state is recorded
