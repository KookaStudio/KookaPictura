# Tasks: move-auto-select-and-refine-enablement

## 1. Move Auto-Select

- [x] 1.1 `MoveOptions::autoSelectOn` (default off) and `ToolServices::selectLayer`
      (ToolController emits `layerSelectionRequested`).
- [x] 1.2 Bridge `auto_select_layer(view, x, y, group)` over
      `helpers_composite::auto_select_path`, with a unit test (topmost wins,
      hidden/zero-opacity/adjustment skipped, group ancestor in Group mode).
- [x] 1.3 Move handler: hit-test and select the layer on press while Auto-Select is on.
- [x] 1.4 Options bar: Auto-Select checkbox (unchecked default) gating the
      Group/Layer combo; frame wiring for `layerSelectionRequested`.

## 2. Ellipsis icon

- [x] 2.1 `assets/icons/ellipsis.svg` + `pictura.qrc` + `lucide-map.json` +
      provenance doc row/counts.
- [x] 2.2 Move menu button uses the icon (icon-only, no text glyph).

## 3. Refine Edge button enablement

- [x] 3.1 `OptionsBar::setRefineEdgeEnabled` over the built buttons.
- [x] 3.2 Frame refresh calls it with `refine_edge_available`.

## 4. Verification

- [x] 4.1 Qt Test: `tst_options_bars_parity` covers the checkbox/combo state
      and the Auto-Select hit test; `tst_refine_edge` covers the button's
      enabled state without and with a selection.
- [x] 4.2 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
