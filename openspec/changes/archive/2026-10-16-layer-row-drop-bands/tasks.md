# Tasks: layer-row-drop-bands

## 1. App

- [x] 1.1 `LayersTreeView::dropPositionFor`: half-row bands on a non-group row, quarter edges around a centre drop-into band on a group row.

## 2. Verification

- [x] 2.1 `tst_layers_panel::dropAnywhereOnLayerRowReorders` (fails before the fix).
- [x] 2.2 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
