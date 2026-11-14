# compositing/vector-masks Specification

## Purpose
Authoring of a layer vector mask: creating, deleting, enabling/disabling,
linking/unlinking, and rasterizing one, the PSD `vmsk` flag semantics, and the
Qt bridge the Layers panel and menus call.

## Requirements

### Requirement: Vector mask creation

The system SHALL create a vector mask for a target layer from a `VectorMaskKind`
of Reveal All, Hide All, or Current Path. Reveal All SHALL author a closed
rectangle subpath over the layer's content rectangle with no flags; Hide All
SHALL author the same rectangle with the invert flag so the layer is fully
hidden; Current Path SHALL author the document's work path subpaths. Creation
SHALL be refused, leaving the document and history unchanged, when the target
layer already has a vector mask, when the path does not resolve, or when Current
Path is requested and the work path is empty. A created vector mask SHALL be
linked and enabled.

#### Scenario: Reveal All adds a full rectangle mask [vmk_add_reveal]

- **WHEN** Reveal All is applied to a raster layer with no vector mask
- **THEN** the layer gains a `vmsk` block whose path is the layer's content
  rectangle, it reads as a vector mask, and the composite is unchanged

#### Scenario: Hide All hides the layer [vmk_add_hide]

- **WHEN** Hide All is applied to a raster layer with no vector mask
- **THEN** the layer gains a `vmsk` block whose path is the content rectangle
  with the invert flag set, and the layer contributes nothing to the composite

#### Scenario: Current Path authors the work path [vmk_add_current]

- **WHEN** Current Path is applied while the document has a non-empty work path
- **THEN** the layer gains a `vmsk` block whose outline is that work path

#### Scenario: Current Path without a work path is refused [vmk_add_current_empty]

- **WHEN** Current Path is applied and the document's work path is empty
- **THEN** the operation is refused, no vector mask is created, and no history
  state is added

#### Scenario: A second vector mask is refused [vmk_add_duplicate]

- **WHEN** any creation is applied to a layer that already has a vector mask
- **THEN** the operation is refused and the existing mask is unchanged

#### Scenario: An unknown path is refused [vmk_add_unknown]

- **WHEN** a vector-mask operation targets a path that does not resolve
- **THEN** the operation is refused and the document is unchanged

### Requirement: Vector mask flag semantics

The system SHALL interpret the PSD `vmsk` flag word as bit 1 invert, bit 2
not-linked, and bit 3 disabled. A vector mask SHALL report linked when the
not-linked bit is clear and unlinked when it is set; a newly created vector mask
SHALL report linked. Enable/disable and link/unlink SHALL modify only their own
bit in the existing block and SHALL otherwise preserve the encoded block bytes,
so an imported mask's geometry and all unrelated bits round-trip unchanged.

#### Scenario: A new mask is linked and enabled [vmk_flags_default]

- **WHEN** a vector mask is created
- **THEN** it reports linked and not disabled

#### Scenario: Unlinking sets only the not-linked bit [vmk_flags_unlink]

- **WHEN** a linked vector mask is unlinked
- **THEN** the not-linked bit becomes set, the other flag bits and the path
  bytes are unchanged, and the mask reports unlinked

#### Scenario: Disabling sets only the disabled bit [vmk_flags_disable]

- **WHEN** an enabled vector mask is disabled
- **THEN** the disabled bit becomes set, the other flag bits and the path bytes
  are unchanged, and the mask reports disabled

#### Scenario: Re-enabling clears only the disabled bit [vmk_flags_enable]

- **WHEN** a disabled vector mask is enabled
- **THEN** the disabled bit is cleared and the mask reports enabled

#### Scenario: Setting the current state is no change [vmk_flags_noop]

- **WHEN** enable/disable or link/unlink is set to the value it already holds
- **THEN** the operation reports no change and adds no history state

### Requirement: Vector mask deletion

The system SHALL delete the vector mask of a target layer, reporting success
when one was removed and failure when the layer had no vector mask or the path
does not resolve. Deletion SHALL remove the `vmsk` block, leave the layer's
pixels unchanged, and the composite SHALL return to the unmasked result.

#### Scenario: Deleting restores the unmasked composite [vmk_delete]

- **WHEN** a vector mask that hides part of a layer is deleted
- **THEN** the layer has no vector mask and the composite reveals the previously
  hidden pixels

#### Scenario: Deleting without a mask reports failure [vmk_delete_absent]

- **WHEN** deletion targets a layer that has no vector mask
- **THEN** it reports failure and the document is unchanged

### Requirement: Rasterizing a vector mask

Rasterizing SHALL convert a vector mask into a raster layer mask and drop the
vector mask. The resulting mask SHALL carry the product, rounded to nearest, of
the vector mask's effective coverage and any existing layer mask's effective
coverage over the layer's content rectangle, so the composite is unchanged;
where either mask is disabled its coverage SHALL contribute fully permissive
(255). The resulting raster mask SHALL be linked and enabled. Rasterizing SHALL
be refused and the document unchanged when the layer has no vector mask.

#### Scenario: Rasterize replaces the vector mask with a layer mask [vmk_rasterize]

- **WHEN** a vector mask that hides part of a layer is rasterized
- **THEN** the layer has no `vmsk` block, it carries a raster layer mask whose
  coverage matches the vector mask, and the composite is unchanged

#### Scenario: Rasterize multiplies an existing layer mask [vmk_rasterize_merge]

- **WHEN** a layer carrying both a layer mask and a vector mask is rasterized
- **THEN** the resulting single layer mask is the product of the two coverages
  and the composite is unchanged

#### Scenario: A disabled vector mask rasterizes permissively [vmk_rasterize_disabled]

- **WHEN** a disabled vector mask is rasterized
- **THEN** the resulting layer mask does not hide any additional pixels and the
  composite is unchanged

#### Scenario: Rasterize without a vector mask is refused [vmk_rasterize_absent]

- **WHEN** rasterize targets a layer with no vector mask
- **THEN** it reports failure and the document is unchanged

### Requirement: Vector mask read predicates

The system SHALL report, without mutating the document, whether a target layer
has a vector mask, whether that mask is linked, and whether it is disabled. A
path that does not resolve or a layer without a vector mask SHALL report no
mask, unlinked, and enabled.

#### Scenario: Reads reflect the model [vmk_reads]

- **WHEN** a vector mask is present, linked, and enabled on a layer
- **THEN** presence reports present, link reports linked, and disabled reports
  enabled; a layer with no vector mask reports absent, unlinked, and enabled

### Requirement: Qt bridge for vector-mask operations

The application bridge SHALL expose the vector-mask operations over the active
layer, taking the creation kind as a string (`reveal-all`, `hide-all`,
`current-path`), and per-row presence/link/disabled reads plus a thumbnail for a
given row. Each mutation that changes the document SHALL recomposite and record
exactly one history state; a mutation that changes nothing SHALL record no
state. When there is no document or no single active layer, every operation
SHALL report failure and leave the document unchanged.

#### Scenario: A bridge mutation is one undo step [vmk_bridge_state]

- **WHEN** the active layer receives a vector mask through the bridge
- **THEN** the canvas is recomposited and exactly one history state is recorded

#### Scenario: A no-op bridge call records nothing [vmk_bridge_noop]

- **WHEN** a bridge vector-mask operation is refused because the layer already
  has a mask or no active layer exists
- **THEN** no history state is recorded and the document is unchanged

#### Scenario: Per-row reads track the mask [vmk_bridge_rows]

- **WHEN** a layer row carries a vector mask
- **THEN** the row reads report a vector mask present, its linked state, and its
  disabled state, and return a non-null thumbnail when thumbnails are enabled

### Requirement: `vsms` vector masks support the same authoring operations

When a layer's vector mask is derived from `'vsms'` and the layer has no
`'vmsk'` block, the system SHALL treat the `'vsms'` block as the layer's
effective vector-mask block for the flag, deletion, rasterization, and
read-predicate operations defined for a `vmsk` mask in this capability. Enable/disable and link/unlink SHALL
patch only the effective block's flag word and SHALL preserve its path bytes and
unrelated bits. Deletion and rasterization SHALL remove every `'vmsk'` and
`'vsms'` block on the layer so the mask cannot return on reload; rasterization
SHALL otherwise keep the composite unchanged and SHALL leave a linked, enabled
layer mask. The linked read SHALL report the effective block's not-linked bit;
presence and disabled reads SHALL report the derived view as for a `'vmsk'`
mask.

#### Scenario: Flags and link state edit the `vsms` block [vmk_vsms_flags]

- **WHEN** a layer whose only vector-mask block is `vsms` is unlinked, then
  disabled, then re-enabled
- **THEN** each change patches only the requested bit in the `vsms` flags word,
  the path bytes and other bits are unchanged, and the read predicates report
  the new state

#### Scenario: Deleting a `vsms` mask removes every vector-mask block [vmk_vsms_delete]

- **WHEN** deletion runs on a layer whose vector mask came from a `vsms` block
- **THEN** the layer has no vector-mask block, the composite is unmasked, and a
  save and reload does not restore the mask

#### Scenario: Rasterizing a `vsms` mask leaves one layer mask [vmk_vsms_rasterize]

- **WHEN** a `vsms`-masked layer is rasterized
- **THEN** no `vmsk` or `vsms` block remains, the layer carries a linked,
  enabled raster mask whose coverage matches the vector mask, and the composite
  is unchanged after a save and reload

#### Scenario: Reads track the effective `vsms` block [vmk_vsms_reads]

- **WHEN** a layer carries a linked, enabled `vsms` mask and no `vmsk`
- **THEN** presence reports present, link reports linked, and disabled reports
  enabled, matching the derived vector-mask view
