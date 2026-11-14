# Spec Delta

## ADDED Requirements

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
