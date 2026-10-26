# Design

## Context

`SvgIconEngine::render()` draws the SVG, then replaces the rendered alpha with a
single tint via `CompositionMode_SourceIn` at 80 % opacity. `colorFor()` chose
that tint from `QPalette::ButtonText` (disabled: `QPalette::Disabled::ButtonText`),
so icon colour was coupled to the shared control text colour.

The theme has a `Ramp` role per family and one token contract in
`Theme::tokens()`. Tokens not referenced by the stylesheet are already
acknowledged as paint-only (`graph-fill`, `graph-axis`).

## Decisions

- Add `Ramp::icon` next to the content roles. The dark family uses `#FFFFFF`;
  the light family uses `#2B2B2B`, the same value as its content roles, so a
  literal white tint never disappears on a light surface.
- Expose it as the `icon` token through `Theme::tokens()`. `tokenColor("icon")`
  then follows the applied family and level like every other token, and the
  design-system dialog and generated style guide pick it up automatically.
- Read the token in `SvgIconEngine::colorFor()` for normal and active modes.
  Keep the palette's `Disabled::ButtonText` for `QIcon::Disabled`, preserving the
  existing dimmed disabled treatment (`#686868` on the dark family).
- Leave the 80 % opacity, explicit-colour override, cache key, and HiDPI paths
  untouched.
- Mark `icon` as paint-only in `tst_design_system`, because the global
  stylesheet never substitutes it.
