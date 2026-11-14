# Spec Delta

## ADDED Requirements

### Requirement: A `vsms` vector mask clips its layer

This requirement extends "A vector fill composites and is clipped by the vector
mask" to masks derived from `'vsms'`. A layer whose vector mask is derived from
the `'vsms'` key SHALL be clipped by that mask exactly like a layer whose mask
is derived from `'vmsk'`: the sampled coverage SHALL multiply the layer's source
alpha, the mask's `invert` and `disabled` flags SHALL apply, and an absent or
fill-less `'vsms'` mask SHALL leave the layer unclipped by it. A `'vscg'` fill or
other generative shape content SHALL therefore render inside the path, not
across the layer rectangle.

#### Scenario: A `vsms` shape fill renders inside its path [lc_vsms_shape_mask]

- **WHEN** a document composites a `vscg` solid-fill shape layer whose only
  vector mask is a closed-rectangle `vsms` block over an opaque base
- **THEN** the pixels inside the rectangle carry the fill and the pixels
  outside it carry the base

#### Scenario: An inverted `vsms` mask hides its inside [lc_vsms_inverted]

- **WHEN** the same shape layer's `vsms` flags set the invert bit
- **THEN** the pixels inside the path carry the base and the pixels outside it
  carry the fill
