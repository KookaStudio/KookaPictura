# compositing/layer-masks Specification

## Purpose
Authoring of a raster layer mask: creating, deleting, applying, enabling, and
linking one, plus the PSD link-flag semantics the Layers panel needs.

## Requirements

### Requirement: Layer mask creation

The system SHALL create a raster layer mask for a target layer from a
`LayerMaskKind` of Reveal All, Hide All, Reveal Selection, Hide Selection, or
From Transparency. A mask created for Reveal All, Hide All, or From Transparency
SHALL be sized to the layer's content rectangle; a selection-driven mask SHALL be
sized to the supplied selection's rectangle. The reveal variants SHALL produce
maximal coverage (255 inside the mask rectangle) and the hide variants SHALL
produce minimal coverage (0 inside it), with the region outside the mask
rectangle contributing the mask's default color so the result is reveal-everything
or hide-everything respectively. Creation SHALL be refused, leaving the document
and history unchanged, when the target layer already has a mask or when the path
does not resolve. A refused creation SHALL report failure and a created mask
SHALL report success. A successful creation SHALL make the new mask the view's
mask edit target, so the brush, fill, and filters write its coverage at once; a
refused creation SHALL leave the target unchanged.

#### Scenario: Reveal All creates a full mask

- **WHEN** Reveal All is applied to a pixel layer that has no mask
- **THEN** the layer gains one mask sized to the layer's content rectangle whose
  samples are 255 and whose outside-rectangle default color is 255, and the
  composite is unchanged

#### Scenario: Hide All creates an empty mask

- **WHEN** Hide All is applied to a pixel layer that has no mask
- **THEN** the layer gains one mask whose samples and outside-rectangle default
  color are 0, and the layer contributes nothing to the composite

#### Scenario: A created mask becomes the edit target [lpr_mask_add_target]

- **WHEN** a layer mask is added to the active layer
- **THEN** the view's mask edit target names that layer, so painting and filling
  edit the mask's coverage

#### Scenario: A second mask is refused

- **WHEN** a layer-mask operation is applied to a layer that already has a mask
- **THEN** the operation is refused, the existing mask is unchanged, and no
  history state is added

#### Scenario: An unknown path is refused

- **WHEN** a layer-mask operation targets a path that does not resolve
- **THEN** the operation is refused and the document is unchanged

### Requirement: Selection-driven mask variants

Reveal Selection and Hide Selection SHALL derive their coverage from a selection
coverage value supplied by the caller, which the engine SHALL accept without
depending on any application type. Reveal Selection SHALL use the selection
coverage directly, so selected pixels are revealed; Hide Selection SHALL use the
inverse, so selected pixels are hidden. When no selection coverage is supplied to
a selection-driven variant, the operation SHALL be refused and the document left
unchanged.

#### Scenario: Reveal Selection keeps only the selected pixels

- **WHEN** Reveal Selection runs with a selection covering part of the layer
- **THEN** the mask reveals the selected pixels and hides the unselected pixels,
  and the composite shows the layer only where the selection covered it

#### Scenario: Hide Selection removes the selected pixels

- **WHEN** Hide Selection runs with a selection covering part of the layer
- **THEN** the mask hides the selected pixels and reveals the unselected pixels,
  and the composite omits the layer where the selection covered it

#### Scenario: A selection variant without a selection is refused

- **WHEN** a selection-driven variant runs with no selection coverage supplied
- **THEN** the operation is refused and no mask is created

### Requirement: Mask from transparency

From Transparency SHALL create a mask whose coverage is derived from the target
layer's alpha: fully revealed where the layer is opaque and fully hidden where the
layer is transparent. A layer with no alpha channel SHALL be treated as fully
opaque.

#### Scenario: Coverage follows the alpha channel

- **WHEN** From Transparency runs on a layer with transparent and opaque pixels
- **THEN** the mask is hidden at the transparent pixels and revealed at the
  opaque pixels, and the composite matches the layer's own alpha

### Requirement: Mask deletion

The system SHALL delete the raster layer mask of a target layer, reporting
success when a mask was removed and failure when the layer had no mask or the
path does not resolve. Deletion SHALL leave the layer's pixels unchanged and the
composite SHALL return to the unmasked result.

#### Scenario: Deleting restores the unmasked composite

- **WHEN** a mask that hides part of a layer is deleted
- **THEN** the layer has no mask and the composite reveals the previously hidden
  pixels

#### Scenario: Deleting without a mask reports failure

- **WHEN** deletion targets a layer that has no mask
- **THEN** it reports failure and the document is unchanged

### Requirement: Applying a mask permanently

Applying a mask SHALL fold the mask coverage permanently into the layer's pixel
alpha, multiply the layer's existing alpha by the mask's coverage at each pixel,
and then clear the layer's mask. The composite after applying SHALL match the
composite before applying within the compositor's one-unit-per-channel tolerance.
Applying SHALL be refused on a smart-object layer, since CS6 cannot apply a mask
permanently to a smart object; a refused apply SHALL leave the mask and the pixels
unchanged.

#### Scenario: Applying folds coverage into alpha and clears the mask

- **WHEN** a mask that hides part of a layer is applied
- **THEN** the layer has no mask, its alpha is the previous alpha multiplied by
  the mask coverage, and the composite is unchanged within tolerance

#### Scenario: Applying a disabled mask does not change pixels

- **WHEN** a disabled mask is applied
- **THEN** the layer's alpha is unchanged and the mask is cleared

#### Scenario: Applying to a smart object is refused

- **WHEN** a mask on a smart-object layer is applied
- **THEN** the operation is refused and both the mask and the layer pixels remain
  unchanged

### Requirement: Enabling and disabling a mask

The system SHALL set a mask's enabled state. Enabling SHALL clear the mask's
disabled state and disabling SHALL set it. A disabled mask SHALL have no effect on
the composite, and setting the state to its current value SHALL be reported as no
change.

#### Scenario: Disabling reveals the whole layer

- **WHEN** a mask that hides part of a layer is disabled
- **THEN** the composite reveals the whole layer and the mask is retained but
  marked disabled

#### Scenario: Re-enabling re-applies the mask

- **WHEN** a disabled mask is re-enabled
- **THEN** the composite hides the same pixels it hid before being disabled

### Requirement: Mask link flag

The system SHALL expose the PSD layer-mask "position relative to layer" flag as
the mask's linked state. The link flag SHALL be bit 0 of the mask's flags byte;
setting the linked state SHALL set or clear that bit while preserving every other
bit, so the raw flags byte keeps round-tripping through PSD save and read. The
linked state SHALL default to linked for a newly created mask, matching CS6.

#### Scenario: Linking sets only bit 0

- **WHEN** a mask whose flags byte has other bits set is linked
- **THEN** bit 0 becomes set, every other bit is preserved, and the mask reads as
  linked

#### Scenario: Unlinking clears only bit 0

- **WHEN** a linked mask is unlinked
- **THEN** bit 0 is cleared, every other bit is preserved, and the mask reads as
  unlinked

#### Scenario: A new mask is linked

- **WHEN** a mask is created
- **THEN** it reads as linked

### Requirement: Mask read predicates

The system SHALL report whether a target layer currently has a raster layer mask
and whether that mask is linked, without mutating the document. A path that does
not resolve, or a layer with no mask, SHALL report no mask and unlinked.

#### Scenario: Reads reflect the model

- **WHEN** a mask is present and linked on a layer
- **THEN** the presence and link reads report present and linked, and a layer
  without a mask reports absent and unlinked

### Requirement: Qt bridge for layer-mask operations

The application bridge SHALL expose the mask operations over the active layer,
taking the operation kind as a string for creation and the selection coverage from
the current selection. Each mutation that changes the document SHALL recomposite
and record exactly one history state; a mutation that changes nothing SHALL record
no state. When there is no document or no single active layer, every operation
SHALL report failure and leave the document unchanged.

#### Scenario: A bridge mutation is one undo step

- **WHEN** the active layer receives a mask through the bridge
- **THEN** the canvas is recomposited and exactly one history state is recorded

#### Scenario: A no-op bridge call records nothing

- **WHEN** a bridge mask operation is refused because the layer already has a mask
  or no active layer exists
- **THEN** no history state is recorded and the document is unchanged

### Requirement: Mask edit target

The bridge SHALL expose a per-view mask edit target naming the layer whose raster
mask is the current edit target, empty when no mask is active. Setting the target
SHALL require a path that resolves to a layer carrying a raster mask and SHALL
otherwise refuse; clearing SHALL always succeed. The target SHALL be independent
of the document's pixels, SHALL record no history, and SHALL read back empty once
the layer or its mask no longer resolves. `pictura_render` SHALL expose
`mask_document(doc, path)` returning the mask as a mask-sized grayscale document
(its coverage replicated across the layer's colour channels, opacity locked) and
`write_mask_back(doc, path, edited, region)` copying the edited document's colour
plane into the real mask over `region`, materialising a data-less mask from its
default colour first. A missing mask, a zero-sized mask rectangle, or a malformed
edited document SHALL refuse both.

#### Scenario: Activating and clearing the target

- **WHEN** the target is set to a layer with a raster mask and then cleared
- **THEN** the first call reports success and the target reads back that path, and
  the second leaves the target empty; a path without a mask refuses and records no
  state

#### Scenario: A mask round-trips through the adapter

- **WHEN** `mask_document` builds the document for a mask whose coverage is a
  left-black/right-white step, the document's layer colour plane is edited, and
  `write_mask_back` runs
- **THEN** the real mask's coverage carries the edited plane and the layer's
  pixels are untouched

#### Scenario: A deleted mask empties the target

- **WHEN** the target names a layer whose mask is then deleted
- **THEN** the target reads back empty

### Requirement: Painting, filling, and filtering the active mask

With a mask edit target active, the Brush (and Pencil) SHALL paint the mask's
coverage instead of the layer's pixels: the foreground colour's Rec.601 luma is
the painted value, so a black foreground hides the layer, white reveals it, and
gray is partial; opacity, flow, hardness, spacing, and the tip shape SHALL behave
as for layer painting. `Edit ▸ Fill` SHALL fill the mask's coverage with the fill
colour's luma through the selection, and a destructive filter SHALL filter the
coverage. Each operation SHALL record exactly one history state with its normal
label ("Brush" / "Pencil", "Fill", "Filter") and SHALL leave the layer's pixels
unchanged. Without an active mask target the three operations SHALL keep editing
the layer's pixels exactly as before. A mask stroke or fill SHALL be refused when
the mask rectangle is empty.

#### Scenario: Painting white and black on the mask

- **WHEN** the mask target is active and the Brush paints a white foreground over
  a masked pixel and a black foreground over a revealed pixel
- **THEN** the first pixel's coverage rises toward 255, the second falls toward 0,
  the composite follows (revealing and hiding the layer), and one "Brush" state is
  recorded

#### Scenario: Filling the mask

- **WHEN** the mask target is active and `Edit ▸ Fill` runs with a black
  foreground
- **THEN** the selection's coverage becomes the black luma and one "Fill" state is
  recorded, while the layer's pixels are unchanged

#### Scenario: Filtering the mask

- **WHEN** the mask target is active and a destructive filter commits
- **THEN** the mask's coverage is filtered over the mask rectangle and one
  "Filter" state is recorded; a preview cancelled before commit restores the
  original coverage bit-identically

#### Scenario: No target keeps layer editing

- **WHEN** no mask edit target is set and the Brush paints
- **THEN** the layer's pixels change and the mask is untouched

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
initially white foreground and black background as CS6 uses while a mask is
active; when it moves back, the colour state SHALL switch to the image pair.
Each pair SHALL keep the colours last set while its target was active. Resetting
the colours (`D` or the toolbox reset) SHALL restore the active target's
defaults: white foreground / black background for a mask, black foreground /
white background for layer pixels.

#### Scenario: Toggling the thumbnails swaps the pairs

- **WHEN** the image colours are set, the mask thumbnail is clicked, the mask foreground is changed, and the layer thumbnail is clicked
- **THEN** the mask shows white/black on activation, the image pair returns on the layer click, and clicking the mask again shows the changed mask foreground

#### Scenario: Resetting colours follows the edit target [lpr_mask_reset_colors]

- **WHEN** a mask is the edit target and `D` or the toolbox reset is used
- **THEN** the mask pair becomes white foreground / black background, and switching back to the layer restores the image pair

### Requirement: Loading a layer mask as a selection

The system SHALL build a document-sized pixel selection from a raster layer mask's
coverage, following the mask's own coverage rule: the mask's stored coverage
inside its rectangle, its `default_color` outside it, and fully selected where the
mask is disabled or carries no data. White (non-zero coverage) SHALL mean selected,
black unselected, grey partial, matching `docs/08-selection/selection-model.md`.
A path that does not resolve, or a layer with no raster mask, SHALL report no
selection. A selection built from a mask SHALL be combined into the current
selection (`Add`, `Subtract`, `Intersect`) or replace it (`New`) in one undoable
step.

#### Scenario: The mask coverage becomes the selection

- **WHEN** a mask that is white in its left half and black in its right half is
  loaded as a selection
- **THEN** the left half of the document is selected and the right half is not

#### Scenario: A disabled mask loads as fully selected

- **WHEN** a disabled mask is loaded as a selection
- **THEN** the whole document is selected

#### Scenario: A layer without a mask reports no selection

- **WHEN** a layer with no raster mask is asked for its mask selection
- **THEN** no selection is produced and the current selection is unchanged

#### Scenario: Loading combines with the current selection

- **WHEN** a mask is loaded as a selection with the `Subtract` mode
- **THEN** the mask's white areas are removed from the current pixel selection

### Requirement: Mask editing is grayscale-native and accelerated

The system SHALL edit a raster layer mask's single coverage plane. The
document through which a mask is painted SHALL carry the coverage as one
grayscale channel, so a dab writes one byte per pixel and no per-dab channel
copy-back is required for correctness. A mask stroke SHALL honour the same
interactive-acceleration policy as a pixel-layer stroke: a dab within the raster
budget SHALL use the exact CPU raster, and a larger brush SHALL use a
reduced-level preview and/or a GPU stroke, selected by the same budget rule.
The committed coverage SHALL be byte-identical, within the compositor's
one-unit-per-channel tolerance, to the exact CPU raster of the same sample log.
A mask stroke SHALL present the changed region only; it SHALL NOT rebuild the
whole mask document, the whole pyramid, or recomposite the whole document on a
per-dab basis. Fills and filters on the mask SHALL keep using the same
single-channel document.

#### Scenario: A dab writes the coverage plane directly

- **WHEN** a mask stroke paints a dab
- **THEN** only the mask's coverage byte changes and no RGBA mask-sized
  intermediate is allocated or copied back

#### Scenario: A large mask brush uses the accelerated path

- **WHEN** the mask target is active and the brush diameter exceeds the raster
  budget
- **THEN** the stroke uses the reduced-level preview and/or the GPU path in place
  of the exact full-resolution CPU raster per dab

#### Scenario: The committed mask matches the exact raster

- **WHEN** a mask stroke is committed after a previewed or GPU stroke
- **THEN** the committed coverage matches a full-resolution exact CPU raster of
  the same sample log within one unit per channel

#### Scenario: The per-dab present stays region-bounded

- **WHEN** a mask dab changes coverage
- **THEN** only the dab's region is recomposited and presented, and the whole
  document is not recomposited

#### Scenario: A live mask stroke renders while dragging

- **WHEN** a mask stroke is dragged across several dabs in one frame
- **THEN** the canvas presents the accumulated region on that frame (a live mask
  stroke counts as painting, so the shell's deferred present runs) instead of
  showing only the first dab and the rest on release

### Requirement: Healing refuses a mask target

With a mask edit target active, the Spot Healing Brush and Healing Brush SHALL
refuse to start with a visible message, recording no history state and leaving
both the layer's pixels and the mask's coverage unchanged.

#### Scenario: Healing refuses a mask target [lm_healing_refuses_mask]

- **WHEN** a mask edit target is active and a Spot Healing or Healing Brush
  press starts
- **THEN** the press is refused with a visible message, no history state is
  recorded, and neither the layer's RGB nor the mask coverage changes
