# Test Plan

One row per scenario in the delta spec. Every row is automated.

## ui/layers-panel

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| m39_rename | Double-click a short-named row's rendered label; then edit a name and press Tab / Shift+Tab at the first row. | The editor opens inside the label; Tab commits and moves down; Shift+Tab commits without wrapping. | `tst_layers_panel_bundle::renameHitAreaMatchesRenderedLabel`, `tst_layers_panel::renameTabNavigation` |
| lpr_rename_band | Double-click a short-named row inside its rendered label, including with thumbnails off. | The inline editor opens for that row. | `tst_layers_panel_bundle::renameHitAreaMatchesRenderedLabel` |
| lpr_rename_outside_label | Double-click a styled row just right of the rendered label, outside the controls. | No inline editor opens; the Layer Style dialog opens on Blending Options. | `tst_layers_panel_bundle::renameHitAreaMatchesRenderedLabel` |
| lpr_rename_name_only | Double-click the eye, fx badge, or mask thumbnail. | No inline editor opens; the control's own action runs. | `tst_layers_panel_bundle::renameHitAreaMatchesRenderedLabel`, `tst_layers_effects::doubleClickBadgeOpensBlendingOptions`, `tst_fill_layer_editor::maskThumbKeepsTarget` |
| lpr_background_dblclick | Double-click the Background row outside the controls. | The Background is converted through the name-and-color dialog, not renamed. | `tst_fill_layer_editor::backgroundDoubleClickConverts` |
