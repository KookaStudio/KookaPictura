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
application-wide, default on at 1.45, and SHALL accept 1.00–2.20 in steps of
0.01. The compositor SHALL read the curve from a lookup table built once per
gamma value rather than evaluating it per pixel. The curve is an approximation
of CS6's closed blend.

#### Scenario: The curve maps coverage [tg_curve]

- **WHEN** the curve is evaluated at γ 1.45
- **THEN** 0 maps to 0, 1 maps to 1, 0.5 maps into (0.36, 0.38), and every value maps at or below itself; at γ 2.2 every value maps to itself

#### Scenario: The table samples the curve [tg_table]

- **WHEN** the lookup table for γ 1.00, 1.45, 1.80, and 2.20 is read across 0…1
- **THEN** it is within half an 8-bit level of the curve, except within 0.001 of full coverage at γ 1.00, where it is within 0.01

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

Rasterizing a type layer while the gamma is on SHALL store `curve(a·k) / k` as
the new pixel layer's alpha, where `a` is the coverage and `k` the layer's
opacity × Fill (the curved coverage when `k` is 0), so compositing the raster at
that opacity does not visibly change the result. While the gamma is on, the GPU
compositor SHALL apply the same curve to a type layer that carries a raster
proxy, replacing `coverage × opacity × Fill` with the table's value before the
layer mask and blend mode. The GPU result MUST match the CPU oracle within ±1
LSB per channel. The curve SHALL come from the same lookup table the CPU uses,
so the GPU does not approximate the table by evaluating the formula. A type
layer with no raster proxy SHALL continue to decline to the CPU (see
`tools/type-live-composite`).

#### Scenario: Rasterize bakes the curve [tg_rasterize]

- **WHEN** a type layer at 100 % and at 50 % opacity is rasterized with the default setting
- **THEN** at 100 % each stored alpha is the curve of its coverage within one level, and at both opacities the raster's composite alpha is within two levels of the live type's

#### Scenario: A proxied type layer composites on the GPU [tg_gpu_proxied]

- **WHEN** the GPU compositor is asked to composite a document whose type layer has a raster proxy, with the default setting
- **THEN** it returns a GPU composite that matches the CPU oracle within ±1 LSB and does not count a type-source decline

#### Scenario: The GPU curve matches at every supported gamma [tg_gpu_gammas]

- **WHEN** a proxied type layer is composited on the GPU at γ 1.00, 1.45, 2.20, and with the setting off
- **THEN** every instance matches the CPU oracle within ±1 LSB per channel

#### Scenario: Opacity, Fill, and a mask pass through the GPU curve [tg_gpu_gates]

- **WHEN** a proxied type layer has opacity below 255, Fill below 255, and an enabled raster mask
- **THEN** the GPU composite matches the CPU oracle within ±1 LSB, so the curve is applied before the mask and includes opacity and Fill

#### Scenario: The GPU defers type to the CPU [tg_gpu]

- **WHEN** the GPU compositor is asked to composite a document whose type layer has no raster proxy, with the default setting
- **THEN** it reports that text is unsupported and the CPU compositor renders it
