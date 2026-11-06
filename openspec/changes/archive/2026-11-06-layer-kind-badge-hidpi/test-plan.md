# Test Plan

Covers the single scenario in the `ui/layers-panel` delta for
`layer-kind-badge-hidpi`. The change makes the shape and smart-object corner
badges render at the painter's device pixel ratio; no manual rows are expected.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `lpr_kind_badge_hidpi` | Add a pixel layer and convert it to a smart object, add a rectangle shape layer, refresh the panel and clear the tree selection, then paint each row into a 2x image via `rowImageForTest(path, 2.0)` and crop the row's `kindBadgeRect` | Each badge's pixels are closer to the badge rendered from a 2x pixmap than to the badge rendered from a 1x pixmap drawn into the same 2x device | `tst_panels_parity::kindBadgesHidpi` |
