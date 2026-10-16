# Design: diffuse-glow

## Model

Fitted by least squares to four CS6 Filter Gallery renders. Three are of one
photograph at (Graininess, Glow, Clear) = (6, 10, 15), (2, 2, 2), and
(9, 18, 16), previewed at 95 %. The fourth is of another photograph at
(8, 1, 6), previewed at 43 %. Each render was registered to its source, and
the model was rendered at full size and resized to the preview scale before
comparison. In every render, R, G and B moved toward white by the same
fraction, so the model is a single mix fraction `m` per pixel:
`out = src + (255 − src)·m`.

- `lit` = Rec.601 luminance (0..1), Gaussian-blurred with
  σ = 2.37 + 0.54·glow px. The blur makes the halo. CS6's glow reaches about
  6 px at Glow 10 and about 17 px at Glow 18.
- `ramp` = smoothstep of `clamp(0.5 + k·(lit − T))`, where
  `T = 0.56 + 0.016·clear − 0.028·glow` and `k = 0.71 + 0.186·glow`.
- `cover = 0.93·(1 − e^(−glow/0.71))`. Glow 0 is no glow, and Glow 1 is
  already about 0.7 of full cover.
- `veil = 0.98·max(0, 1 − clear/10)^2.5`. CS6 lifts the shadows about 58 %
  at Clear 2 but only about a tenth at Clear 6, so the veil is gone by
  Clear 10. A first fit without the Clear 6 render used a gentle
  `(1 − clear/20)²` falloff, which washed Clear 6 out.
- `m = 1 − (1 − veil)·(1 − cover·ramp)`.
- Grain: `m += s·(u₁ + u₂ − 1)` with `s = 0.0126·g + 0.0016·g²`. The two
  draws come from a position hash salted by the seed. `m` is then clamped to
  0..1, so the filter never darkens.

Smoothed-luma RMSE against CS6 is about 16 / 7 / 21 / 8 levels for the four
renders. Most of the remaining error is in the heavy-grain flower render,
where CS6's grain is coarser than ours.

## Kooka adaptations

- The glow colour is white. CS6 uses the Background swatch, but the dialog
  shows none, and Neon Glow has the same stand-in (`ponytail:`).
- A Seed slider is added, as for Film Grain and Ocean Ripple.
- The preview apron stays at the 128 px default, which covers the halo's 3σ
  (≤ 35 px).
