# Test Plan

One row per scenario in the delta spec. Every row is automated.

## ui/layers-panel

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| lpr_name_fit | Add a layer named `r1 r2` and read the painted name band; then convert the bottom layer to the Background, add a group, and add a styled layer and read each name. | Each row paints its full name with no ellipsis. | `tst_layers_panel_bundle::shortLayerNameIsNotElided`, `tst_layers_panel_bundle::layerNamesRenderInFullWhenTheyFit` |
| lpr_name_elide | Rename the same layer to `start` + 200 characters + `end` and refresh. Read the name text the row paints. | The painted text is shorter than the name, starts with `start`, ends with `end`, and carries one ellipsis in the middle. | `tst_layers_panel_bundle::shortLayerNameIsNotElided`, `tst_layers_panel_bundle::layerNamesRenderInFullWhenTheyFit` |
