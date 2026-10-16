# Tasks: filter-alpha-family-contracts

## 1. Engine

- [x] 1.1 Shape Blur and Liquify process every plane.
- [x] 1.2 Lens Blur defocuses alpha without bloom or noise.
- [x] 1.3 Smart Blur Normal averages alpha over the colour-admitted taps.

## 2. Verification

- [x] 2.1 Every `filters_alpha` kind moves a soft layer edge through `apply_filter`.
- [x] 2.2 Kernel unit tests pin the per-kernel alpha behaviour.
- [x] 2.3 Kernel digests updated (alpha plane only).

## 3. Specs

- [x] 3.1 The six family alpha requirements follow `filters_alpha`.
- [x] 3.2 `filter-application` names Liquify and the Smart Blur mode split.
