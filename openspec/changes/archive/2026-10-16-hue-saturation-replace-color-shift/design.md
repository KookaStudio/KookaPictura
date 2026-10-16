# Design: hue-saturation-replace-color-shift

## Hue/Saturation lightness

`hue_saturation_rgb` keeps the HSL hue rotation and the `S · (1 + sat)` scale,
converts back to RGB at the original `L`, then applies per channel:

- `dl ≥ 0`: `c + dl · (1 − c)`
- `dl < 0`: `c · (1 + dl)`

Each channel moves the same fraction toward white or black, so the colour's hue
and relative chroma are kept and a near-neutral pixel stays near-neutral. The
WGSL `adj_hs` mirrors this exactly, so GPU-to-CPU parity stays within the
existing tolerance.

## Replace Color shift

`replace_color::shift(rgb, hue, sat, light)`:

1. Rotate the hue in HSL, keeping `S` and `L`.
2. Scale each channel's distance from `L` by the gain: `1 / (1 − sat)` when
   `sat ≥ 0` (bounded at `1e3`), `1 + sat` when `sat < 0`. `L` is unchanged and
   a gray has no distance to scale.
3. Apply the Hue/Saturation lightness blend above.

The weighted blend toward the shifted colour and the sample weighting are
unchanged.

## Dialog inverse

`replace_color_shift_for(sample, result)` inverts the shift for the Result
picker:

- **Lightness** comes from `L`, which step 3 moves linearly:
  `(l1 − l0) / (1 − l0)` lighter, `l1 / l0 − 1` darker.
- **Saturation** comes from the chroma `max − min`, which step 2 scales by the
  gain and step 3 by `1 − |light|`. So `gain = c1 / (c0 · (1 − |light|))`,
  mapped back through the two gain branches.
- **Hue** is the wrapped hue difference.
- **Gray sample:** a gray sample (`c0 = 0`) gets hue 0 and saturation 0, so only
  Lightness moves. This matches CS6 Help's note.

The swatch calls the same Rust `shift`, so the swatch and the canvas cannot
drift apart.
