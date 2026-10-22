# Design

## Context

The adjustment-layer machinery already ships: `PictureView::add_adjustment(kind)` creates the layer (16 kinds), `LayersPanel::performPanelMenuAction("adjustment:<kind>")` is the working create path used by the panel menu (`panel_group_menu.cpp:402-408`), and the Properties panel opens the selected layer's controls (`properties_panel.cpp:718`, covered by `tst_properties_panel`). `Layer > New Fill Layer` handles Solid Color and Gradient. The only missing piece is the panel body: `PlaceholderPanel` at `frame_build.cpp:77`. See proposal.md for the why.

## Goals / Non-Goals

**Goals:**
- Replace the Adjustments placeholder with an `AdjustmentsPanel` icon-grid widget.
- Reuse the existing `adjustment:<kind>` dispatch and fill-layer commands; add no new engine/bridge behavior.
- Ship 19 icons under the established Lucide-vendoring + provenance rules.
- Keep the panel menu untouched (it is a tested, working secondary create path).

**Non-Goals:**
- No adjustment-layer model changes, no gating by color mode/bit depth (the menu does not gate; tracked separately).
- No preset or Properties-panel work (already specified by `ui/properties-panel`).
- No fill-layer creation behavior beyond what the existing commands do (Pattern stays inert, matching the Layer menu).

## Decisions

**1. Widget shape: a static button grid, no model/delegate.** The spec's `QAbstractListModel`/delegate design is overkill for a fixed 19-entry launcher. `AdjustmentsPanel` is a `QWidget` hosting a `QGridLayout` of `QToolButton`s in `IconOnly` style with tooltips (compact CS6 launcher look; the labels live in tooltips only). A static table of `{kind, label, iconId, fill?}` rows drives button construction; the table is the single source for both order and tooltip text.
   - *Alternative considered:* `QListView` icon mode — rejected: more code, no benefit for a fixed grid.
   - *Alternative considered:* text-under-icon buttons — rejected: the user asked for a compact, label-free grid.

**2. Dispatch through the existing panel-menu path, then select.** Adjustment buttons send `"adjustment:<kind>"` to the same handler as the panel menu (which routes to `LayersPanel::performPanelMenuAction`). That path only creates the layer — `add_adjustment` never touches `layer_target`, so the grid must add a select step after create: look up the newly-created layer path (the top of the active document's stack) and select it through the existing Layers-panel selection API. This is what makes the Properties panel open (the properties panel binds on selection; `tst_properties_panel.cpp:153` has to select manually today). Fill buttons dispatch `command_ids::LayerNewFillSolidColor` / `LayerNewFillGradient`; Pattern stays inert (its menu leaf has no handler).
   - *Alternative considered:* calling `add_adjustment` directly on the view — rejected: same selection gap, plus it would bypass the menu's routing.
   - *Alternative considered:* teaching `add_adjustment` to select — rejected: changes a tested bridge contract used by the menus; selection is a UI concern.

**3. Icon ids and assets.** New ids follow the `adjustments.<kind>` scheme, mirroring `tool.<tool>`. The 19 assets split as: 14 Lucide v1.50.0 slugs vendored by `scripts/sync-lucide-icons.py` into `assets/icons/` — `circle-dot` (Solid Color), `grid-3x3` (Pattern), `chart-column` (Levels), `chart-spline` (Curves), `sun-moon` (Exposure), `palette` (Hue/Saturation), `contrast` (Black & White), `camera` (Photo Filter), `sliders-horizontal` (Channel Mixer), `box` (Color Lookup), `image-minus` (Invert), `rows-4` (Posterize), `square-split-horizontal` (Threshold), `pipette` (Selective Color) — plus 5 hand-authored Lucide-style files: `adjustments.brightness-contrast`, `adjustments.vibrance`, `adjustments.color-balance`, `adjustments.gradient`, and `adjustments.gradient-map` (the gradient pair derive from the existing `tool.gradient.svg`). Every file is registered in `assets/pictura.qrc` and mapped in `assets/icons/lucide-map.json` with provenance entries in `docs/dev/icon-provenance.md` (the guard `scripts/check-icon-provenance.py` fails otherwise). Because the icon-assets spec names ids verbatim from command ids or `tool.<tool>`, this change adds a delta requirement to `ui/icon-assets` sanctioning `<panel>.<item>` ids. Site copies go to `site/icons/`.

**4. Enablement.** The grid enables/disables as a whole based on whether a document is open, mirroring the panel menu's enablement. Handled by a `setDocumentOpen(bool)` style refresh called from the frame's existing document-change notification points.

**5. File placement and build.** New `panels/adjustments_panel.{h,cpp}`; add to the top-level `CMakeLists.txt` (no globbing). `frame.h:760` member becomes `AdjustmentsPanel*`; `frame_build.cpp:77` constructs it with the same `objectName` (`adjustmentsPanel`) so panel registration, Window menu, and existing tests keep working.

## Risks / Trade-offs

- [Icon fidelity vs. Lucide sourcing] Hand-drawn Vibrance/Color Balance/Brightness–Contrast (and the B&W/Brightness split) may look off vs. CS6 → deliberate; refine later without spec change (art-only iteration is allowed by the icon-assets spec). The provenance map records `custom` for these.
- [Selection added in UI layer] The grid's select-after-create step is new surface; keep it to the existing Layers-panel selection API and cover it in the `clickAdjustmentCreatesAndSelectsLayer` test.
- [Fill commands] Solid Color/Gradient handlers add fills directly (no modal dialog), so tests assert dispatch without blocking; Pattern is inert like the menu — verify at build time and match the button's enabled state to the menu's (no spec change).
- [New asset files] 19 SVG files + map/provenance/qrc edits are mechanical but guard-gated; run `check-icon-provenance.py` early in implementation.
