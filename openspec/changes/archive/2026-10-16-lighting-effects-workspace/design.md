# Design: lighting-effects-workspace

## Model

Shading is photorust's (`core/src/filters/render.rs::lighting_effects`). Per
pixel it computes ambient (`colorize · ambience/100`), plus diffuse by `N·L`,
plus a Blinn highlight once Gloss > 0. Metallic blends the highlight from the
lamp's colour to the surface's. The normal comes from the Texture channel's
gradient scaled by Height. The output is `(own · light + spec) · 2^(exposure/50)`.

Kooka's changes:

- **Rig.** Every visible light adds its own diffuse and highlight. The rig
  holds 1..=16 lights (`MAX_LIGHTS`).
- **Intensity.** The strength is `intensity / 50`, so CS6's "about 50 is
  normal" holds. Photorust used `/ 25`.
- **Relative facing.** Diffuse uses `(N·L) / L_z`, capped at 2. A flat pixel
  is then lit by its falloff alone, whatever the lamp height, and a slope
  facing the lamp catches more.
- **Spot.** An ellipse at `center` with semi-axes `size·span` and
  `width·span`, rotated by `angle`. For `h = (hotspot + 100) / 200`, the
  hotspot ellipse has semi-axes `(0.6h²·A, 0.7h·B)` and its centre
  `0.9·(1 − 0.6h²)·A` along the aim (`spot_hotspot`). It stays short of the
  far end, so the light fades there instead of aliasing. Falloff is 1 inside
  the hotspot and 0 outside the outer ellipse. In between it is
  `smoothstep(1 − (dᵢ − 1)/(dᵢ − dₒ))`, where `dᵢ` and `dₒ` are the
  normalised ellipse distances. That form is continuous and closed. A
  bisection over the interpolated ellipses was tried first, but the family is
  not nested near the far end and it speckled.
- **Point.** `(1 − (d/R)²)₊` with `R = size·span`, lamp at `0.7R` height.
- **Infinite.** A direction `(cos a · cos e, sin a · cos e, sin e)` that comes
  from `angle` at `elevation`, with no falloff.

The geometry is tuned by eye against CS6 screenshots (`ponytail:` in the
module). `lighting_rig.cpp::spotHotspot` mirrors `spot_hotspot` so the
on-canvas hotspot outline matches the render.

## Slots

The rig is 9 slots: colorize r, g, b, exposure, gloss, metallic, ambience,
texture, height. Each light is 13 more: type, visible, colour r, g, b,
intensity, hotspot, centre x, y, angle, size, width, elevation.
`FILTER_ARITIES` lists the one-light total (22), which keeps
`filter_param_arity` and the arity test meaningful. `lighting()` in
`filter_map.rs` accepts `9 + 13k` for `1 ≤ k ≤ 16`. The `FilterCommandSpec`
row describes the rig and one light, and stays the source of labels, ranges,
and defaults for the generic dialog's slot count.

## Workspace

`LightingEffectsDialog` is a large `QDialog`: 92 % of the main window, or
1180 × 760 without a parent. It is laid out as CS6's workspace:

- **Options bar.** A glyph, Presets, Lights (three add buttons), Reset, a
  stretch, Preview, Cancel, and OK. Presets lists CS6's 17 styles with the
  colours, intensities, and focus from CS6's Help. The placements are by eye.
  Any edit switches the combo to Custom. Reset loads Default.
- **Canvas** (`LightingCanvas`). The picture is fitted on a #282828
  pasteboard. A proxy of at most 1100 px is rendered through
  `filter_thumbnail` and coalesced on a zero-timer, so a drag renders once
  per event-loop turn. The document is not touched until OK, so Cancel needs
  no restore. The selected light draws its whole widget, and every light
  draws its centre handle and Intensity ring. Hit priority is: the selected
  light's Intensity ring, then another light's centre (to select it), then
  the selected light's handles (Spot axis handles, hotspot edge, Point ring,
  Infinite end handle), then its body (move), then, for a Spot, beyond it
  (rotate).
- **Properties / Lights panels.** These form a fixed 264 px column. Light
  names (`Spot Light 1`, …) are numbered per type in rig order. They are
  derived, not stored, so they never drift from the slots.
