# compositing/layer-masks — delta

## ADDED Requirements

### Requirement: Canvas tools paint the active mask

With a mask edit target active, the canvas Brush and Pencil (`begin_brush`) SHALL
paint the mask's coverage with the foreground colour's Rec.601 luma, and the
Eraser (`begin_eraser`) SHALL paint it with the background colour's luma, each
recording one history state with its normal label ("Brush", "Pencil",
"Eraser") and leaving the layer's pixels unchanged. Erase To History SHALL be
refused on a mask. Painting tools that cannot paint a mask, the Magic Eraser,
the Paint Bucket, the Gradient, and the other fills that share the layer-fill
path SHALL refuse while a mask is the target, recording no history state and
leaving the layer's pixels unchanged.

#### Scenario: The Brush tool hides the layer through its mask

- **WHEN** a mask edit target is active and `begin_brush` paints a black foreground
- **THEN** one "Brush" state is recorded, the composite shows the layer hidden there, and deleting the mask shows the layer's pixels unchanged

#### Scenario: The Eraser reveals with the background colour

- **WHEN** a mask edit target is active and `begin_eraser` paints over hidden coverage with a white background
- **THEN** one "Eraser" state is recorded and the layer shows again

#### Scenario: A tool without mask support refuses

- **WHEN** a mask edit target is active and the Pattern Stamp or the Paint Bucket starts
- **THEN** it is refused and no history state is recorded

### Requirement: Foreground and background follow the edit target

The application SHALL keep one foreground/background pair for editing layer
pixels and one for editing a mask. When the active edit target moves from a
layer's pixels to its mask, the colour state SHALL switch to the mask pair,
initially black foreground and white background; when it moves back, the colour
state SHALL switch to the image pair. Each pair SHALL keep the colours last set
while its target was active.

#### Scenario: Toggling the thumbnails swaps the pairs

- **WHEN** the image colours are set, the mask thumbnail is clicked, the mask foreground is changed, and the layer thumbnail is clicked
- **THEN** the mask shows black/white on activation, the image pair returns on the layer click, and clicking the mask again shows the changed mask foreground
