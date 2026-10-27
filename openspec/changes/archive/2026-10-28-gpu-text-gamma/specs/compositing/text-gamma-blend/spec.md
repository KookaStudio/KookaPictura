# Spec Delta

## MODIFIED Requirements

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
