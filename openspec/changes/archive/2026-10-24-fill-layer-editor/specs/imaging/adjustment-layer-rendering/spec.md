# Spec Delta — imaging/adjustment-layer-rendering

## ADDED Requirements

### Requirement: Full-parameter gradient fill payloads encode and round-trip

`pictura-render` SHALL expose `encode_gradient_fill_params(params:
&GradientFillParams) -> AdjustmentData`. It builds the same version-16 `GdFl`
descriptor as `encode_gradient_fill`, and in addition SHALL write `Rvrs` (bool)
when `params.reverse` is true and `Scl ` (double) when `params.scale` differs
from 100. `decode_adjustment` on its output SHALL equal
`Adjustment::GradientFill(params)`.

#### Scenario: Reverse and scale survive the round-trip [alf_gradient_full_roundtrip]

- **WHEN** `encode_gradient_fill_params` encodes a Radial gradient at angle 45 with reverse true and scale 60
- **THEN** `decode_adjustment` returns those stops, kind, angle, reverse, and scale

#### Scenario: Defaults stay absent [alf_gradient_defaults]

- **WHEN** the params have reverse false and scale 100
- **THEN** the encoded descriptor carries no `Rvrs` or `Scl ` item and decodes to the decode defaults

### Requirement: Pattern fill payloads encode and round-trip

`pictura-render` SHALL expose `encode_pattern_fill(params:
&PatternFillParams) -> AdjustmentData`. It builds a version-16 `PtFl`
descriptor whose `Ptrn` object carries `Idnt` for `params.pattern_id`, plus
`Scl ` for the scale, `Algn` for `link_with_layer`, and a `phase` point when
`params.origin` is not `(0, 0)`. `decode_adjustment` on its output SHALL equal
`Adjustment::PatternFill(params)`.

#### Scenario: A pattern payload decodes back [alf_pattern_roundtrip]

- **WHEN** `encode_pattern_fill` encodes pattern id `"p1"`, scale 50, link false, and origin `(3, 4)`
- **THEN** `decode_adjustment` returns those exact pattern params

#### Scenario: The default origin is omitted [alf_pattern_default_origin]

- **WHEN** the params have origin `(0, 0)`
- **THEN** the encoded descriptor carries no `phase` object and decodes to origin `(0, 0)`
