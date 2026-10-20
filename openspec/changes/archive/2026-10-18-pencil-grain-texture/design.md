# Design: pencil-grain-texture

## Context

The archived `paintbrush-types-pen-tip` change gave `StrokeConfig::grain` a
fixed per-pixel alpha drop keyed to layer coordinates. At grain 30 the 2B
Pencil still reads as a solid dark stroke; the owner's Photoshop reference is
lighter (~60 % mean coverage) and has 2–3 px grain clumps with fine per-pixel
variation.

## Decisions

### Multiscale noise

`fill::grain_noise(x, y)` mixes the existing `pixel_noise` hash with a
bilinear value noise sampled on a 2 px grid, then expands the contrast by 1.5:

    noise = clamp((0.4 × fine + 0.6 × smooth2 − 0.5) × 1.5 + 0.5, 0, 1)

The fine term keeps the texture from looking like a smooth blotch; the 2 px
term forms the pencil-like clumps, and the contrast expansion restores the
reference's light patches and dark clumps. The result is deterministic, in
`0.0..=1.0`, and keyed to layer coordinates, so overlapping dabs see the same
pattern and the same stroke always renders identically. No threshold holes are
introduced, so the dab core stays painted; the reference's light patches come
from the noise reaching low values, not from dropped pixels.

### Strength

`keep = 1 − grain% × (1 − noise)`. With the mixed noise's mean 0.5, grain 80
leaves a mean coverage of 0.6, close to the reference's interior alpha
(0.60–0.70). The `2B Pencil` preset uses 80 %. `Non-Photo Blue Pencil` keeps
30 %: the issue's screenshot only supplies a 2B reference, and the blue pencil
is a different, brighter tip.

### Tests

- `fill::grain_noise_is_deterministic_and_clumpy`: values stay in range,
  repeat exactly, and a 2 px box average keeps most of the texture's spread
  (unlike single-pixel static, which averages away).
- `stroke::grain_breaks_up_the_dab_alpha`: the existing determinism and
  no-core-holes checks stay, plus the grained coverage is 50–70 % of the plain
  dab at grain 80.
- `tst_brush_tip_preview`: the 2B Pencil expectation becomes grain 80.
