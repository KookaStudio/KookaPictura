# Tasks

## 1. Tests

- [x] 1.1 Add `tst_layers_panel_bundle::renameEditorStaysInsideLabelBand`: on a
  styled row with a short name, the inline editor starts at the rendered label,
  is no wider than the label plus the name gap, and stops left of the fx badge.
- [x] 1.2 Add `renameEditorHugsTheNameOnPlainRows` and
  `renameEditorStopsBeforeLockBadge` for a plain row and a locked row.
- [x] 1.3 Add `renameEditorStopsAtTheBandForLongNames`: a name that fills the
  band caps the editor at the badge edge.

## 2. App

- [x] 2.1 Extract `LayerRowDelegate::nameBandRect` and have `nameRect` elide
  inside it.
- [x] 2.2 Override `updateEditorGeometry` to size the editor to the rendered
  label plus the name gap, capped at the label band.

## 3. Gates

- [x] 3.1 Build and run `tst_layers_panel_bundle`.
- [x] 3.2 `python3 scripts/check-test-plan.py`, `openspec validate --all --strict`.
- [x] 3.3 Archive with `openspec archive rename-editor-label-bounds`.
