# Tasks

## 1. Tests first

- [x] Add `tst_layers_effects::adjustmentRowsShowNoFxBadge`: an adjustment row
  and a solid-fill row paint no fx badge and reserve no fx space, the Alt-click
  gesture is inert on them, and adding a color overlay to the adjustment row
  brings the badge back.

## 2. Panel

- [x] Change `LayerRowDelegate::showsFx()` to require a present effect: the
  layer carries a style *and* the row's present-effects list is non-empty
  (shape rows still excluded), so the badge and the `Effects` group agree.

## 3. Spec and verification

- [x] Update the `ui/layers-panel` delta: the Alt-click-all-effects requirement
  targets a styled row's fx region, with a scenario for the adjustment row.
- [x] Run `openspec validate --all --strict`, the focused Qt suite, and archive
  the change.
