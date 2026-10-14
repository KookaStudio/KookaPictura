# Design: filter-alpha-family-contracts

## Which kernels were missed

`filters_alpha` is the one classification; a kernel on it must process every
plane it is given. The four misses each carried a `.min(3)` (or a fixed RGB
split) that predates the fold. The kernel digest test showed it: every
migrated kernel's digest moved with the fold, these did not.

## Per-kernel alpha

- **Shape Blur, Liquify**: plane-independent, so the cap simply goes.
  Liquify's commit now matches its dialog preview, which already warped
  alpha (premultiplied).
- **Lens Blur**: the alpha plane takes the aperture's weighted average only.
  The specular bloom brightens colour by luma and the noise is grain on
  colour; applied to alpha they would add opacity that was never there.
- **Smart Blur, Normal**: the threshold test stays on the three colour
  channels; alpha accumulates over the taps that pass, so a layer edge that
  coincides with a held colour edge stays sharp and a soft edge inside a
  flat colour region softens. Edge Only and Overlay Edge paint colour
  outlines and leave alpha bit-identical (they are off `filters_alpha`'s
  Normal-only arm).

## Regression guard

`moving_kernels_move_a_soft_layer_edge` (pictura-render) runs one instance of
every `filters_alpha` kind through `apply_filter` on a layer whose alpha is a
soft disc plus a stray speck, and requires alpha to change. Reverting any one
kernel to three planes fails it. The same list feeds
`moving_kernels_map_opaque_to_opaque`.
