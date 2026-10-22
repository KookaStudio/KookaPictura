## ADDED Requirements

### Requirement: Warp values from a file are bounded

Decoding a `TySh` warp descriptor SHALL clamp bend and both distortions to
−100…+100 % and SHALL read a non-finite value as 0. Warped placement SHALL
refuse non-finite bounds, and a warped extent more than four times the
unwarped text box's span, before allocating the layer's pixels.

#### Scenario: Out-of-range values clamp [wt_bounded_decode]

- **WHEN** `pictura_codec::type_tool::tests::decodes_the_warp_descriptor`
  decodes a bend of 1e308 and a NaN bend
- **THEN** they read as 100 and 0

#### Scenario: Hostile values never size a huge plane [wt_bounded_render]

- **WHEN** `pictura_render::type_warp::tests::hostile_warp_values_stay_bounded`
  places Arc type with infinite, NaN, and 1e308 values
- **THEN** each placement is refused or stays within four times the unwarped
  span, and out-of-range values place exactly like ±100 %
