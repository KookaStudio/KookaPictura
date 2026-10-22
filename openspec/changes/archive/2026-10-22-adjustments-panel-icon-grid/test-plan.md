# Test Plan — adjustments-panel-icon-grid

One row per scenario in `specs/ui/adjustments-panel/spec.md`. Coverage names the Qt Test case in `tst_adjustments_panel.cpp` (all new except the last, which exists).

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Grid shows all nineteen entries in order | Open the app, create a document, inspect the Adjustments panel grid buttons | 19 icon-only buttons, in order: Solid Color, Gradient, Pattern, Brightness/Contrast, Levels, Curves, Exposure, Vibrance, Hue/Saturation, Color Balance, Black & White, Photo Filter, Channel Mixer, Color Lookup, Invert, Posterize, Threshold, Gradient Map, Selective Color (identified by tooltip; no text labels) | `tst_adjustments_panel::gridShowsAllNineteenInOrder` |
| Grid entries carry tooltips | Hover each grid entry | Tooltip names the adjustment/fill type | `tst_adjustments_panel::gridEntriesCarryTooltips` |
| Clicking an adjustment icon creates the layer | With a document open, click each of the 16 adjustment icons | A new adjustment layer of that kind appears above the active layer, is selected, and Properties shows its controls | `tst_adjustments_panel::clickAdjustmentCreatesAndSelectsLayer` |
| Creating an adjustment is undoable | Click an adjustment icon, then Undo | The adjustment layer is removed; the stack returns to its prior state | `tst_adjustments_panel::adjustmentCreateIsUndoable` |
| Solid Color icon opens the solid-color flow | Click the Solid Color icon | The Solid Color fill-layer command is dispatched | `tst_adjustments_panel::solidColorIconDispatchesFillCommand` |
| Pattern icon is inert | Click the Pattern icon | No layer created, no command dispatched | `tst_adjustments_panel::patternIconIsInert` |
| No document disables the grid | Open the app with no document; inspect the panel | Every grid entry disabled; clicking does nothing | `tst_adjustments_panel::noDocumentDisablesGrid` |
| A document enables the grid | Create a document; inspect the panel | Grid entries enabled, except the inert Pattern entry | `tst_adjustments_panel::documentEnablesGrid` |
| Narrow panel wraps the grid | Shrink the panel below one row's width | Entries flow onto additional rows; all 19 remain visible and clickable | `tst_adjustments_panel::narrowPanelWrapsGrid` |
| Panel menu still lists the adjustment kinds | Open the Adjustments panel menu | Lists the 16 kinds plus Add Mask by Default and Clip to Layer in order | `tst_command_tree::layerAdjustmentMenu` (existing) |
| Adjustments panel item glyphs resolve | Call `icon(id)` for each `adjustments.*` id | Non-null icon at 16 px | `tst_icon_assets::adjustmentsGlyphsResolve` |
| Panel item glyphs are mapped | Run the provenance guard | Every `adjustments.*` file and map entry pair up | manual: repo guard script scripts/check-icon-provenance.py (no C++/Rust test covers it) |
