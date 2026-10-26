# compositing/smart-filter-masks Specification

## Purpose
Rendering a smart filter stack through its mask: a masked-out pixel keeps the
unfiltered source.

## Requirements

### Requirement: The smart-filter chain renders through the filter mask

When a smart object carries a filter mask, the compositor SHALL blend the
filtered chain result with the unfiltered source by the mask coverage: a fully
masked-out pixel SHALL equal the unfiltered source, a fully unmasked pixel the
filtered result, and a partial pixel the two mixed in proportion. A pixel
outside the mask rect SHALL take the mask's extend value (`filterMaskExtendWithWhite`
white, otherwise transparent). A smart object with no mask SHALL render exactly
as before.

#### Scenario: A masked-out pixel keeps the source [sfm_render]

- **WHEN** `smart_filter::tests::the_chain_renders_through_the_mask` composites
  a filtered smart object whose mask is black over a region
- **THEN** that region shows the unfiltered source and the unmasked region shows
  the filtered result

#### Scenario: No mask renders as before [sfm_render_none]

- **WHEN** `smart_filter::tests::a_layer_without_a_mask_is_unchanged` composites
  the same object with `filter_mask` `None`
- **THEN** every pixel is the filtered result
