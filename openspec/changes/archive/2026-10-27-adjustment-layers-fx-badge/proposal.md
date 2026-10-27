# Proposal: Adjustment layers show no fx badge

## Why

Every adjustment and fill layer row paints the layer-style `fx` badge, because
`LayerRowDelegate::showsFx()` ORs the adjustment role with the layer-style role.
Adjustment content is not a layer style, so the badge is wrong: it marks a row
that carries no effects and it steals the right-edge space the mask thumbnail
and the name need. CS6 draws `fx` only for a layer that actually carries a style
(#983).

## What Changes

- `showsFx()` keys on the layer carrying a style *and* that style carrying at
  least one effect (still excluding shape rows), so an adjustment or fill row —
  or any layer whose effects are all gone — paints no `fx` badge and reserves no
  fx space. The badge and the row's `Effects` group always appear together.
- The badge, its hit target, and the reserved space all come from that one
  predicate, so `fxRect()`, `badgesRight()`, `nameRect()`, and the painted badge
  follow together.
- The Alt-click-all-effects gesture stays on real layer styles: a styled row's
  fx region still toggles every layer's effects, and an adjustment row (no fx
  region) does nothing.
- The smart-filter and layer-effect rows keep their own fx glyph path in the
  thumbnail slot.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: the `fx` badge is drawn only for a layer that carries a
  layer style, and the Alt-click-all-effects gesture targets a styled row's fx
  region.

## Impact

- C++ panel: `crates/pictura-app/cpp/panels/layers_panel_internal.h`.
- Tests: `crates/pictura-app/cpp/tests/tst_layers_effects.cpp` (new case
  `adjustmentRowsShowNoFxBadge`).
- Spec: `openspec/specs/ui/layers-panel/spec.md` (through this change's delta).
