# Spec Delta

## MODIFIED Requirements

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
