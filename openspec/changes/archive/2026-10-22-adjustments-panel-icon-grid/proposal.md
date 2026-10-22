# Proposal

## Why

The Adjustments panel is a `PlaceholderPanel` (`crates/pictura-app/cpp/frame_build.cpp:77`), so the CS6 launch-grid surface is absent even though the underlying machinery — adjustment-layer creation for all 16 kinds, the panel menu, and the Properties-panel editor — already ships. The parity register tracks this as PAN-1 (MISSING) against `docs/02-ui-ux/panels/adjustments-panel.md`, issue #509.

## What Changes

- Add an `AdjustmentsPanel` content widget with an always-visible icon grid: the three fill-layer icons (Solid Color, Gradient, Pattern) first, then the 16 adjustment icons in the CS6 order already encoded by the panel menu (`panel_group_menu.cpp:223-244`).
- Clicking an adjustment icon creates the adjustment layer through the existing `adjustment:<kind>` dispatch (same path as the panel menu and `Layer > New Adjustment Layer`) and then selects the new layer so the Properties panel opens its controls (the create path itself does not select; the panel adds the selection step). Clicking a fill icon runs the existing `Layer > New Fill Layer` commands (Solid Color / Gradient; Pattern stays inert, matching the menu).
- Each icon carries a tooltip naming the adjustment; the grid wraps/scrolls when the dock is narrow, and is disabled while no document is open (matching the existing menu enablement).
- Add 19 icons under `assets/icons/` (`adjustments.<kind>`), 14 vendored from the pinned Lucide release and 5 hand-authored Lucide-style, registered in `assets/pictura.qrc`, `assets/icons/lucide-map.json`, and `docs/dev/icon-provenance.md`; mirror them into `site/icons/` and the site icon gallery in `site/index.html`.
- Keep the existing panel menu rows unchanged (the menu is the secondary create path).

## Capabilities

### New Capabilities
- `ui/adjustments-panel`: the Adjustments panel icon-grid surface — the 19-icon ordered grid, per-icon create/select behavior, fill-vs-adjustment routing, tooltips, enablement with no document, and narrow-panel layout.

### Modified Capabilities
- `ui/icon-assets`: extend the icon-id naming rule to sanction panel-item glyph ids (`<panel>.<item>`, e.g. `adjustments.levels`) in addition to the existing command-id and `tool.<tool>` forms, so the new assets satisfy the icon-assets contract.

## Impact

- `crates/pictura-app/cpp/panels/adjustments_panel.{h,cpp}` (new), registered in the top-level `CMakeLists.txt`; `frame.h` / `frame_build.cpp` swap `PlaceholderPanel` for `AdjustmentsPanel` (empty-state placeholder for the Adjustments panel is removed; other placeholders stay). The panel selects the created adjustment layer through the existing Layers-panel selection API.
- `assets/icons/` +19 SVGs (14 Lucide-vendored from v1.50.0, 5 custom/hand-authored), `assets/pictura.qrc`, `assets/icons/lucide-map.json`, `docs/dev/icon-provenance.md`; `scripts/sync-lucide-icons.py` and `scripts/check-icon-provenance.py` unchanged (they consume the map).
- `site/icons/` +19 SVGs and `site/index.html` icon gallery.
- Tests: new Qt Test suite `tst_adjustments_panel.cpp`; existing `tst_command_tree::layerAdjustmentMenu` and selftest rows remain green (menu texts unchanged).
- Docs: `ROADMAP.md` (#509 tick, placeholders count), `docs/dev/doc-code-parity-gaps.md` PAN-1 row (docs commit carries `TASK-ALLOWS-DOCS`).
