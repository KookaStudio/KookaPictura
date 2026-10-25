# Spec Delta — compositing/layer-masks

## MODIFIED Requirements

### Requirement: Layer mask creation

The system SHALL create a raster layer mask for a target layer from a
`LayerMaskKind` of Reveal All, Hide All, Reveal Selection, Hide Selection, or
From Transparency. A mask created for Reveal All, Hide All, or From Transparency
SHALL be sized to the layer's content rectangle, except that a mask created for
a group target SHALL be sized to the document rectangle; a selection-driven mask
SHALL be sized to the supplied selection's rectangle. The reveal variants SHALL
produce maximal coverage (255 inside the mask rectangle) and the hide variants
SHALL produce minimal coverage (0 inside it), with the region outside the mask
rectangle contributing the mask's default color so the result is
reveal-everything or hide-everything respectively. Creation SHALL be refused,
leaving the document and history unchanged, when the target layer already has a
mask or when the path does not resolve. A refused creation SHALL report failure
and a created mask SHALL report success. A successful creation SHALL make the
new mask the view's mask edit target, so the brush, fill, and filters write its
coverage at once; a refused creation SHALL leave the target unchanged.

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

#### Scenario: A group mask is document-sized [lm_group_mask_document]

- **WHEN** Reveal All is applied to a group with children and to an empty group
- **THEN** each group gains a document-sized mask with a non-empty thumbnail, and painting or filling the mask changes its coverage

### Requirement: Mask edit target

The bridge SHALL expose a per-view mask edit target naming the layer whose raster
mask is the current edit target, empty when no mask is active. Setting the target
SHALL require a path that resolves to a layer carrying a raster mask, including
a group, and SHALL otherwise refuse; clearing SHALL always succeed. The target
SHALL be independent of the document's pixels, SHALL record no history, and
SHALL read back empty once the layer or its mask no longer resolves.
`pictura_render` SHALL expose `mask_document(doc, path)` returning the mask as a
mask-sized grayscale document (its coverage replicated across the layer's colour
channels, opacity locked) and `write_mask_back(doc, path, edited, region)`
copying the edited document's colour plane into the real mask over `region`,
materialising a data-less mask from its default colour first. A missing mask, a
zero-sized mask rectangle, or a malformed edited document SHALL refuse both.

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

#### Scenario: A group mask round-trips through the adapter [lm_group_mask_edit]

- **WHEN** a group's document-sized mask is targeted, its mask document is edited, and the edit is written back
- **THEN** the group's mask coverage carries the edit and the composite follows it
