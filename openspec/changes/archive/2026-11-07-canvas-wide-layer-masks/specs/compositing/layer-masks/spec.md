# Spec Delta

## MODIFIED Requirements

### Requirement: Layer mask creation

The system SHALL create a raster layer mask for a target layer from a
`LayerMaskKind` of Reveal All, Hide All, Reveal Selection, Hide Selection, or
From Transparency. A newly created mask SHALL cover the document rectangle and
SHALL retain any source coverage outside that rectangle. Its exterior default
color SHALL be 0. For a nonempty pixel layer, Reveal All SHALL put 255 inside
the layer's content rectangle and 0 elsewhere. Hide All SHALL put 0 throughout.
From Transparency SHALL place the layer's alpha at its document coordinates,
treat a missing alpha channel as 255 inside the content rectangle, and put 0
outside it. Reveal Selection and Hide Selection SHALL sample the supplied
selection's coverage and its inverse respectively over the document and any
retained selection rectangle. For a group or a zero-area layer, Reveal All and
From Transparency SHALL preserve the existing document-wide white initialization
when there is no usable alpha source; Hide All SHALL
remain black. Creation SHALL be refused, leaving the document and history
unchanged, when the target layer already has a mask, the path does not resolve,
or a selection-driven variant has no supplied selection. A refused creation
SHALL report failure and a created mask SHALL report success. A successful
creation SHALL make the new mask the view's mask edit target, so brush, fill,
and filters write coverage at once; a refusal SHALL leave the target unchanged.

#### Scenario: Reveal All creates a full mask

- **WHEN** Reveal All is applied to a nonempty pixel layer smaller than the document
- **THEN** the mask covers the document, is 255 inside the layer's content rectangle and 0 elsewhere, has a black exterior default, and leaves the composite unchanged

#### Scenario: Hide All creates an empty mask

- **WHEN** Hide All is applied to a pixel layer that has no mask
- **THEN** the mask covers the document with zero coverage and a black exterior default, and the layer contributes nothing to the composite

#### Scenario: A created mask becomes the edit target [lpr_mask_add_target]

- **WHEN** a layer mask is added to the active layer
- **THEN** the view's mask edit target names that layer, so painting and filling edit the mask's coverage

#### Scenario: A second mask is refused

- **WHEN** a layer-mask operation is applied to a layer that already has a mask
- **THEN** the operation is refused, the existing mask is unchanged, and no history state is added

#### Scenario: An unknown path is refused

- **WHEN** a layer-mask operation targets a path that does not resolve
- **THEN** the operation is refused and the document is unchanged

#### Scenario: A group mask is document-sized [lm_group_mask_document]

- **WHEN** Reveal All is applied to a group with children and to an empty group
- **THEN** each group gains a document-sized white mask with a non-empty thumbnail, and painting or filling the mask changes its coverage

#### Scenario: An empty layer keeps a usable Reveal All mask [cmd_empty_layer]

- **WHEN** Reveal All or From Transparency is applied to a zero-area pixel layer without usable alpha
- **THEN** the mask is white throughout the document and can be edited throughout the canvas

#### Scenario: Transparency is placed on black at document coordinates [cmd_alpha_creation]

- **WHEN** From Transparency is applied to an offset pixel layer with partial alpha, or an offset layer without alpha
- **THEN** the mask follows alpha, or 255 for missing alpha, inside the layer rectangle and is 0 elsewhere on the canvas

#### Scenario: Selection variants cover the document [cmd_selection_creation]

- **WHEN** Reveal Selection and Hide Selection are created from a selection whose rectangle is smaller than the canvas
- **THEN** each mask covers the canvas and carries the selection coverage or its inverse at the correct document coordinates, with a black exterior default

#### Scenario: Creation retains source coverage outside the canvas [cmd_creation_offcanvas]

- **WHEN** a layer or supplied selection extends outside the canvas when its mask is created
- **THEN** creation retains that source coverage as well as making the entire canvas editable

### Requirement: Mask edit target

The bridge SHALL expose a per-view mask edit target naming the layer whose raster
mask is the current edit target, empty when no mask is active. Setting the target
SHALL require a path that resolves to a layer carrying a raster mask, including
a group, and SHALL otherwise refuse; clearing SHALL always succeed. The target
SHALL be independent of the document's pixels, SHALL record no history, and
SHALL read back empty once the layer or its mask no longer resolves.
`pictura_render` SHALL expose `mask_document(doc, path)` returning a
document-sized grayscale editing view of the mask, with coverage replicated
across the layer's colour channels and opacity locked, and
`write_mask_back(doc, path, edited, region)` copying the edited coverage over the
requested document-space region. Stored masks smaller than, offset from, or
larger than the canvas SHALL be mapped by their document coordinates. An
imported mask with no decoded coverage data SHALL start its canvas editing view
at coverage 255 regardless of its stored default color, matching the existing
composite and solo/Channels missing-data semantics. For a mask with decoded
data, the editing view SHALL use its explicit samples inside the stored rect
and its existing default coverage outside it. Building the editing view alone
SHALL NOT rewrite stored bounds, default color, or missing-data state.
Write-back SHALL preserve stored coverage outside the edited canvas region,
including off-canvas samples. A missing mask, an empty document, or a malformed
edited document SHALL refuse both; an empty stored mask rectangle alone SHALL
NOT prevent an otherwise valid mask from being edited on a nonempty canvas.

#### Scenario: Activating and clearing the target

- **WHEN** the target is set to a layer with a raster mask and then cleared
- **THEN** the first call reports success and the target reads back that path, and the second leaves the target empty; a path without a mask refuses and records no state

#### Scenario: A mask round-trips through the adapter

- **WHEN** the adapter builds the editing document for a left-black/right-white mask, its colour plane is edited, and write-back runs
- **THEN** the real mask carries the edit at the same document coordinates and the layer's pixels are untouched

#### Scenario: A deleted mask empties the target

- **WHEN** the target names a layer whose mask is then deleted
- **THEN** the target reads back empty

#### Scenario: A group mask round-trips through the adapter [lm_group_mask_edit]

- **WHEN** a group's document-sized mask is targeted, its mask document is edited, and the edit is written back
- **THEN** the group's mask coverage carries the edit and the composite follows it

#### Scenario: An empty stored rectangle can be edited [cmd_empty_stored_rect]

- **WHEN** a valid mask with an empty stored rectangle is edited on a nonempty canvas
- **THEN** a data-less mask's editing view starts at coverage 255 independently of its stored default, and an edit can write coverage anywhere on the canvas

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
the layer's pixels exactly as before. The editable area SHALL include the whole
canvas independently of the layer's or stored mask's bounds. A mask stroke or
fill SHALL be refused when the document rectangle is empty. Cancelled previews
and strokes SHALL restore coverage and stored bounds exactly.

#### Scenario: Painting white and black on the mask

- **WHEN** the mask target is active and the Brush paints a white foreground over a masked pixel and a black foreground over a revealed pixel
- **THEN** the first pixel's coverage rises toward 255, the second falls toward 0, the composite follows, and one "Brush" state is recorded

#### Scenario: Filling the mask

- **WHEN** the mask target is active and `Edit ▸ Fill` runs with a black foreground
- **THEN** the selection's coverage becomes the black luma and one "Fill" state is recorded, while the layer's pixels are unchanged

#### Scenario: Filtering the mask

- **WHEN** the mask target is active and a destructive filter commits
- **THEN** coverage is filtered over the canvas in one "Filter" state; cancelling a preview restores original coverage and stored bounds bit-identically

#### Scenario: No target keeps layer editing

- **WHEN** no mask edit target is set and the Brush paints
- **THEN** the layer's pixels change and the mask is untouched
