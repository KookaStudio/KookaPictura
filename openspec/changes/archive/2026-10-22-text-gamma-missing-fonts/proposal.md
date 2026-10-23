# Proposal: Blend Text Colors Using Gamma and missing fonts

## Why

CS6 composites type layers through a global **Blend Text Colors Using Gamma**
curve (`Edit > Color Settings > More Options`, on at 1.45), so its type edges
and the effective opacity of translucent type differ from a plain layer of the
same colour. Pictura composites type like any pixel layer. A document whose
type layers name fonts that are not installed opens silently and renders in the
bundled face, and `Type > Replace All Missing Fonts` is not on the menu. Issue
#685 (a sub-issue of #541) asks for the gamma blend (on by default at 1.45, a
curve on the text coverage at composite time) and the missing-font alert and
command (`docs/03-tools/type-tools.md`). Missing Glyph Protection was split out
to #887.

## What changes

- **Text gamma.** While Blend Text Colors Using Gamma is on, a type layer's
  coverage × opacity × Fill passes through `1 − (1 − a)^(γ / 2.2)` before its
  mask and blend mode apply. γ defaults to 1.45; off (or γ 2.20) is the plain
  blend. The setting is application-wide, not document or history state. The
  merged composite that saves and exports use carries it, and Rasterize Type
  bakes the curve into the layer's alpha. The GPU compositor hands type layers
  back to the CPU while the gamma is on.
- **Color Settings.** A More Options button reveals Advanced Controls with a
  Blend Text Colors Using Gamma checkbox and a 1.00–2.20 value. Both persist in
  the session; changing them recomposites every open document and records no
  history.
- **Missing fonts.** A font is missing when Qt has no installed family for it
  and it is not the bundled Liberation Sans. Opening a document whose type
  layers use missing fonts shows one non-blocking alert listing them. Those
  layers keep their stored pixels and their Layers rows carry a warning badge.
  `Type > Replace All Missing Fonts` re-sets every such layer in Liberation
  Sans Regular as one "Replace All Missing Fonts" state, keeping its text.

## Capabilities

- Added: `compositing/text-gamma-blend`, `tools/missing-fonts`.
- Modified (added requirement): `color/color-settings`.

## Impact

- `pictura-render`: `text_gamma.rs` (the setting and the curve), the curve in
  `blend_into_row` for type layers, the GPU refusal, the baked curve in
  `materialize_text_rgba`; `missing_fonts.rs` (`replace_type_fonts`).
- `pictura-app` bridge: a `type_tools/fonts.rs` bridge module (set the gamma,
  recomposite a view, replace fonts), so `type_tools.rs` stays under its cap.
- `pictura-app` C++: the Color Settings advanced controls, two session keys,
  `fontIsInstalled` in `type_fonts`, the open-time alert, the Type menu entry,
  and the Layers-row badge. Tests: Rust units in `pictura-render`, a new
  `tst_text_gamma_fonts` Qt suite.

## Out of scope

- Missing Glyph Protection (per-character font fallback, #887).
- Blend RGB Colors Using Gamma and the rest of Color Settings' advanced
  controls.
- A font picker for the replacement: CS6 substitutes its default font.
- Exact parity with CS6's gamma curve: no CS6 reference renders exist, so the
  curve is an approximation (see `design.md`).
