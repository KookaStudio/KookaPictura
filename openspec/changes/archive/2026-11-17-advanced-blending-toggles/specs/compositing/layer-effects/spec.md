# Spec Delta

## ADDED Requirements

### Requirement: Mask-hide switches move a mask from source shaping to output clipping

For every effect family in this capability, a layer's
`layer_mask_hides_effects` flag SHALL select the role of the raster layer mask.
When the flag is `false` (the default), the mask SHALL multiply the effect's
source matte, as the per-effect requirements state. When the flag is `true`, the
mask SHALL NOT multiply the source matte; it SHALL instead multiply the effect
pass output coverage over the pass's rectangle. The
`vector_mask_hides_effects` flag SHALL do the same for the vector mask. The two
flags SHALL act independently, and a disabled or absent mask SHALL be inert
whatever its flag says. When the flag is `true`, this requirement SHALL take
precedence over the per-effect source-matte wording in the existing requirements
of this capability. The effect source box SHALL also ignore a mask whose hide
flag is on, so a fill or shape layer is not narrowed to that mask's box.

#### Scenario: Off keeps the mask on the effect source [lef_lmgm_off_source]

- **WHEN** a drop shadow is built for a layer whose mask hides half the canvas and `layer_mask_hides_effects` is false
- **THEN** the shadow's source matte is zero in the masked-out half, and the shadow cast by visible content can still extend into that half

#### Scenario: On clips the effect output [lef_lmgm_on_output]

- **WHEN** the same layer has `layer_mask_hides_effects` true
- **THEN** the source matte is not multiplied by the mask, and the shadow is absent wherever the mask is zero

#### Scenario: The vector mask switches independently [lef_vmgm_independent]

- **WHEN** a layer has `vector_mask_hides_effects` true and `layer_mask_hides_effects` false
- **THEN** the vector mask clips the effect output while the raster mask continues to shape the effect source

#### Scenario: A fill or shape source is not narrowed to a hiding mask [lef_lmgm_on_fill_source]

- **WHEN** a fill or shape layer with a mask has `layer_mask_hides_effects` true
- **THEN** its effect source box is the layer's whole box rather than the mask's confining box, and the effect output is clipped by the mask
