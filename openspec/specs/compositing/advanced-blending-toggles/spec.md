# compositing/advanced-blending-toggles Specification

## Purpose
Advanced Blending options that reach the compositor: clipped-layer grouping, interior-effect folding, transparency shaping, the two mask-hide switches, and per-channel Include. The capability fixes the render contract for the five toggles without changing any document whose fields are at their defaults.

## Requirements

### Requirement: Blend Clipped Layers as Group off keeps each member's blending

When a clipping base's `blend_clipping` flag is `false`, the CPU compositor
SHALL composite the base with its own blend mode and opacity, and each visible
clipped member with its own blend mode and opacity, still bounded by the base's
content coverage. The base's mode and opacity SHALL NOT be inherited by the
members. When the flag is `true` (the default), the existing clipping-group
behaviour SHALL be unchanged. A hidden base SHALL still hide its run, and a
clipped member with no base SHALL still composite unclipped.

#### Scenario: A member keeps its own mode and opacity when off [abt_clbl_off]

- **WHEN** a 50 %-opacity Normal base carries a clipped Multiply member at 40 % opacity with `blend_clipping` false
- **THEN** the member blends with Multiply at its own 40 % opacity and the base's 50 % opacity does not fade it

#### Scenario: The default on still groups the stack [abt_clbl_on_default]

- **WHEN** the same stack is composited with `blend_clipping` true
- **THEN** the result is byte-identical to the existing clipping-group composite

#### Scenario: Members stay bounded by the base when off [abt_clbl_off_bounded]

- **WHEN** a clipped member extends past the base's content with `blend_clipping` false
- **THEN** the member appears only where the base has content

### Requirement: Blend Interior Effects as Group folds the interior effect set

When a layer's `blend_interior` flag is `true`, the CPU compositor SHALL
composite the layer's content together with its enabled Pattern Overlay,
Gradient Overlay, Color Overlay, Satin and Inner Glow as one group, applying the
layer's blend mode, opacity and Blend If once to that group; the layer and
vector masks SHALL keep the role defined by the mask-hide requirement. The
grouping SHALL cover pixel, fill, shape, type and smart-object content; a group
or a destructive adjustment layer SHALL keep its existing composition. Inner
Shadow, Stroke and Bevel SHALL remain separate and composite onto the blended
group result with their own modes. This requirement SHALL take precedence over
the above-content ordering in the existing per-effect requirements when the flag
is on. When the flag is `false` (the default), every effect SHALL keep its own
pass and the existing per-effect blending SHALL be unchanged. A layer with no
enabled fold effect SHALL composite byte-identically whether the flag is on or
off.

#### Scenario: On folds the interior effects [abt_infx_on_folds]

- **WHEN** a Multiply layer with a 100 % Normal Color Overlay and `blend_interior` true composites over an opaque backdrop
- **THEN** the overlay folds into the layer colour and the layer's Multiply mode applies once to the combined result

#### Scenario: Off keeps the per-effect modes [abt_infx_off_default]

- **WHEN** the same layer composites with `blend_interior` false
- **THEN** the overlay applies with its own Normal mode over the layer-over-backdrop result

#### Scenario: On with no fold effect is byte-identical to off [abt_infx_no_fold_identity]

- **WHEN** a styled layer has no enabled overlay, Satin or Inner Glow
- **THEN** its composite is byte-identical with `blend_interior` on and off

#### Scenario: Inner Shadow, Stroke and Bevel stay after the group blend [abt_infx_after_blend]

- **WHEN** an enabled Inner Shadow, Stroke and Bevel accompany the folded overlays with `blend_interior` true
- **THEN** they composite onto the final group result with their own modes rather than folding into the layer colour

#### Scenario: A fill layer folds as content [abt_infx_fill_layer]

- **WHEN** a Multiply solid-fill layer with an enabled Color Overlay composites with `blend_interior` true
- **THEN** the fill and the overlay form one group and the layer's Multiply mode applies once

#### Scenario: Groups and destructive adjustments stay inert [abt_infx_inert_on_non_content]

- **WHEN** a group or a destructive adjustment layer has `blend_interior` true
- **THEN** its composite is byte-identical to the same layer with the flag false

### Requirement: Transparency Shapes Layer off uses the whole canvas

When a layer's `transparency_shapes` flag is `false`, the CPU compositor SHALL
use the document canvas as the effect and knockout shape regardless of the
layer's pixel alpha: enabled overlays SHALL fill the canvas, a non-`None`
knockout SHALL cover the whole canvas, and the effects that need an interior or
exterior edge (Drop Shadow, Outer Glow, Stroke, Inner Shadow, Satin, Inner Glow
and Bevel) SHALL contribute no pixels. The layer and vector masks SHALL keep the
role defined by the mask-hide requirement, so the canvas fill is bounded by a
mask whose hide flag is off. When the flag is `true` (the default), the layer's
content alpha SHALL shape effects and knockouts as before. The flag SHALL NOT
change the layer's own content blending.

#### Scenario: An overlay fills the canvas when off [abt_tsly_off_overlay]

- **WHEN** a small square layer with no mask and an enabled Color Overlay composites with `transparency_shapes` false
- **THEN** the overlay colour covers the whole canvas, not only the square

#### Scenario: A knockout cuts the whole canvas when off [abt_tsly_off_knockout]

- **WHEN** a half-fill knockout layer composites with `transparency_shapes` false
- **THEN** the knockout replaces the whole canvas with the knockout base, including pixels where the layer has no content

#### Scenario: Edge-derived effects disappear when off [abt_tsly_off_edge_effects]

- **WHEN** a layer with enabled Drop Shadow, Outer Glow, Stroke, Inner Shadow, Satin, Inner Glow and Bevel composites with `transparency_shapes` false
- **THEN** none of those seven effects changes a pixel inside the canvas

#### Scenario: A mask keeps its role when off [abt_tsly_off_masked]

- **WHEN** a half-masked layer with an enabled Color Overlay composites with `transparency_shapes` false
- **THEN** the overlay follows the mask when its hide flag is off, and the overlay output is clipped by the mask when its hide flag is on

#### Scenario: The default on keeps content-shaped effects [abt_tsly_on_default]

- **WHEN** the same effects composite with `transparency_shapes` true
- **THEN** the result is byte-identical to the existing content-shaped effect composite

### Requirement: Layer and vector masks hide effects independently

When a layer's `layer_mask_hides_effects` flag is `true`, the raster layer mask
SHALL clip the layer's effect pass output instead of shaping the effect source
mattes; when the flag is `false` (the default), the mask SHALL keep shaping the
sources. `vector_mask_hides_effects` SHALL do the same for the vector mask. The
two flags SHALL act independently, and a disabled mask SHALL be inert. The
layer's own content SHALL remain gated by both masks as today.

#### Scenario: Off lets an exterior effect spill past the mask [abt_lmgm_off_source]

- **WHEN** a drop shadow is enabled on a layer whose mask hides half the canvas and `layer_mask_hides_effects` is false
- **THEN** the shadow from the visible content can appear in the masked-out half, and the shadow is absent where the source content is masked out

#### Scenario: On clips the effect output to the mask [abt_lmgm_on_output]

- **WHEN** the same layer has `layer_mask_hides_effects` true
- **THEN** the shadow is absent wherever the mask is zero

#### Scenario: The vector mask switches independently [abt_vmgm_on_output]

- **WHEN** a layer with a vector mask has `vector_mask_hides_effects` true and `layer_mask_hides_effects` false
- **THEN** effect output is clipped by the vector mask while the raster mask still shapes the effect source

#### Scenario: A disabled mask changes nothing [abt_mask_disabled]

- **WHEN** a mask is disabled and either hide-effects flag is true
- **THEN** the composite is byte-identical to the same layer without that mask

### Requirement: Channel Include excludes channels from compositing

When a layer's `restricted_channels` lists a colour channel, the CPU compositor
SHALL leave that channel untouched by the layer's whole contribution, restoring
the pre-layer premultiplied backdrop value `dst * dst_alpha / out_alpha` after
the layer's content and effects have composited; alpha SHALL composite normally.
When `out_alpha` is zero the layer has no contribution and the pixel SHALL be
left unchanged. Excluding every rendered colour channel of the document SHALL
remove the layer entirely, effects and alpha included. An adjustment layer SHALL
leave excluded channels unadjusted. In an RGB document channel indices 0, 1 and
2 map to R, G and B; in a Grayscale document index 0 maps to the gray channel. A
document in any other colour mode SHALL keep the restricted list for round-trip
but SHALL NOT apply an exclusion. Other indices SHALL be preserved in the model
but SHALL NOT be rendered.

#### Scenario: An excluded channel keeps the backdrop [abt_brst_exclude_one]

- **WHEN** a fully opaque green layer over a red backdrop excludes channel 0 (red)
- **THEN** red stays the backdrop's 255, green and blue blend normally, and alpha is unchanged

#### Scenario: Effects obey the exclusion [abt_brst_effects]

- **WHEN** a white square layer with an enabled red Color Overlay over a green backdrop excludes channel 0
- **THEN** neither the layer content nor the overlay changes the red channel

#### Scenario: Excluding every colour channel removes the layer [abt_brst_all]

- **WHEN** an RGB layer excludes channels 0, 1 and 2
- **THEN** the composite is byte-identical to the same document without that layer, including its alpha

#### Scenario: An adjustment leaves excluded channels unadjusted [abt_brst_adjustment]

- **WHEN** an Invert adjustment on a non-zero red backdrop excludes channel 0
- **THEN** red equals the pre-adjustment backdrop

#### Scenario: A transparent backdrop stays transparent [abt_brst_transparent]

- **WHEN** a restricted layer composites over a fully transparent backdrop
- **THEN** the composite stays transparent and no excluded channel is introduced

#### Scenario: The document mode selects the rendered channels [abt_brst_modes]

- **WHEN** a Grayscale document excludes channel 0 and a CMYK document carries a restricted channel
- **THEN** the gray exclusion keeps the backdrop's gray and the CMYK list round-trips without changing the composite

### Requirement: Defaults are a no-op and the GPU declines non-defaults

A document whose advanced-blending fields are all at their defaults SHALL
composite byte-identically to the same document before this capability. The GPU
compositor SHALL report its advanced-blending error and leave the CPU composite
authoritative for a stack containing, at any depth, a visible layer with any
non-default value of `blend_clipping`, `blend_interior`, `transparency_shapes`,
either mask-hide flag, or a non-empty `restricted_channels`.

#### Scenario: Default fields preserve the composite [abt_defaults_identity]

- **WHEN** a document with clipping, effects and masks is composited with every new field at its default
- **THEN** the result is byte-identical to the pre-change composite of the same document

#### Scenario: Each non-default value forces the CPU path [abt_gpu_declines]

- **WHEN** a stack contains a visible layer with `blend_clipping` false, `blend_interior` true, `transparency_shapes` false, a mask-hide flag true, or a restricted channel
- **THEN** the GPU compositor declines with its advanced-blending error
