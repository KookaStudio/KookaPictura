# Design: port-photorust-lens-flare

## Model

Ported from perfecto25/photorust `core/src/filters/render.rs::lens_flare`,
with its constants and idioms kept so the port diffs against upstream.

- **Frame.** `span = ½·√(w² + h²)`. The flare sits at `center · (w, h)`,
  sampled at pixel centres (`x + 0.5`). The ghost axis runs from the flare to
  the middle of the frame. Ghost `at` 0 is the flare, 1 the middle, and 2 the
  opposite point.
- **Light per pixel**, in tinted RGB, before the `brightness / 100` gain:
  - core `1 / (1 + (d / (core·span))²)`, plus a Gaussian glow;
  - a halo ring at `halo·span`, `0.08·span` wide;
  - rays: `exp(−(14·between)²)`, where `between` is the angular distance to
    the nearest spoke in −0.5..0.5, faded by distance;
  - Movie Prime only: a streak `1.3·span` long and `0.012·span` thick;
  - ghosts, as filled hexagonal discs (`aperture` = 0.65·hex + 0.35·round)
    or as rings.
- Each RGB value is `v + light·gain·255`, clamped and truncated. Alpha is
  untouched.
- Per-lens constants (core, glow, rays, halo, streak, ghost table) are
  photorust's verbatim.

## Kooka adaptations

- The planar `PixelBuffer` is written directly: the R, G, and B planes are
  split and zipped row by row under rayon.
- `LensType` keeps Kooka's `Zoom` variant name (photorust: `Zoom50To300`),
  so `Filter`, `filter_map.rs`, and the profiles stay untouched.
- Out-of-range brightness is rejected (`FilterError::InvalidParams`), not
  clamped. That is the existing Kooka contract. The centre is still clamped.

## Dialog

The flare's parameters are one struct: a crosshair that sets two slots, a
brightness, and a lens. The generic `FilterPreviewDialog` lays slots out as
independent rows, so `LensFlareDialog` is its own `QDialog`.
`frame_menus_filter.cpp::runFilterDialog` routes `lens-flare` to it from both
the menu and Last Filter Settings. The `FilterCommandSpec` row stays as the
source of labels, ranges, defaults, and arity.

- **Proxy.** On open, the canvas image is scaled to fit 250 × 250 (aspect
  kept) and cached as RGBA. This happens before any canvas preview, so the
  flare is never drawn twice. Each change re-renders the pad through
  `filter_thumbnail`. That is cheap at proxy size, and honest because the
  model is size-invariant.
- **Canvas preview.** With Preview on, `filter_preview` re-renders the whole
  layer on open (deferred one event-loop turn), on crosshair moves, on lens
  changes, on Brightness edits, and on slider release. During a slider drag
  only the pad updates. Cancel discards the preview.
- **Whole-layer preview.** `lens-flare` joins `filter_preview_needs_whole_layer`,
  because a crop-relative centre and span would misplace the flare (#168).
