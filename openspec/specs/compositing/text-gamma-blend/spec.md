# compositing/text-gamma-blend Specification

## Purpose
Blend Text Colors Using Gamma: the approximated CS6 curve on a type layer's coverage and opacity at composite time, and how rasterizing and the GPU respect it.

## Requirements

### Requirement: Type layers composite through the text gamma curve

While Blend Text Colors Using Gamma is on with value γ, the compositor SHALL
replace a type layer's source alpha `a` (coverage × layer opacity × Fill) with
`1 − (1 − a)^(γ / 2.2)` before the layer mask, Blend If, and blend mode apply.
A type layer is a layer carrying decoded type data, whether it composites its
stored pixels or re-renders its text. Other layers SHALL composite unchanged.
While the setting is off the curve SHALL NOT apply. The setting SHALL be
application-wide, default on at 1.45, and SHALL accept 1.00–2.20. The curve is
an approximation of CS6's closed blend.

#### Scenario: The curve maps coverage [tg_curve]

- **WHEN** the curve is evaluated at γ 1.45
- **THEN** 0 maps to 0, 1 maps to 1, 0.5 maps into (0.36, 0.38), and every value maps at or below itself; at γ 2.2 every value maps to itself

#### Scenario: Type edges thin by default [tg_default_on]

- **WHEN** black type over white is composited with the default setting
- **THEN** every partially covered pixel is lighter than or equal to the plain blend and at least one is strictly lighter, while fully covered and uncovered pixels are unchanged

#### Scenario: Opacity passes through the curve [tg_opacity]

- **WHEN** a fully covered black type pixel at 50 % opacity is composited over white with the default setting
- **THEN** the result is the white blended with `1 − 0.5^(1.45/2.2)` of black, not with 0.5

#### Scenario: A pixel layer is unaffected [tg_pixel_layer]

- **WHEN** a pixel layer with partial alpha is composited with the default setting
- **THEN** the result equals the plain blend

#### Scenario: Off reproduces the plain blend [tg_off]

- **WHEN** the setting is turned off and the same type layer is composited
- **THEN** the result equals the plain blend of its coverage

### Requirement: Rasterizing and the GPU respect the text gamma

Rasterizing a type layer while the gamma is on SHALL store the curved coverage
as the new pixel layer's alpha, so the composite does not visibly change. While
the gamma is on, the GPU compositor SHALL refuse a document with a visible type
layer, so the CPU compositor renders it.

#### Scenario: Rasterize bakes the curve [tg_rasterize]

- **WHEN** a type layer is materialized into pixels with the default setting
- **THEN** each stored alpha equals the curve applied to the rendered coverage

#### Scenario: The GPU defers type to the CPU [tg_gpu]

- **WHEN** the GPU compositor is asked to composite a document whose type layer has pixels, with the default setting
- **THEN** it reports that text is unsupported
