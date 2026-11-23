# Test plan

One row per scenario in the delta spec. Every row is automated.

## ui/layers-panel

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lpr_rename_editor_bounds` | Add a styled layer with the short name `A`, show and size the window, open the inline editor, and read its geometry, the row rect, the rendered-label rect, and the fx badge rect. | The editor starts at the rendered label's left edge, is no wider than the label plus the name gap, and its right edge is left of the fx badge. | `tst_layers_panel_bundle::renameEditorStaysInsideLabelBand`, `tst_layers_panel_bundle::renameEditorHugsTheNameOnPlainRows`, `tst_layers_panel_bundle::renameEditorStopsBeforeLockBadge` |
| `lpr_rename_editor_band_cap` | Add a styled layer with a name wider than the label band, open the inline editor, and read its geometry, the rendered-label rect, and the fx badge rect. | The editor covers the rendered label and stops left of the fx badge. | `tst_layers_panel_bundle::renameEditorStopsAtTheBandForLongNames` |
