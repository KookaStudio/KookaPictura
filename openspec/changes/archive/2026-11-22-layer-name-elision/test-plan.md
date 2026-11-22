# Test Plan

One row per scenario in the delta spec. Every row is automated.

## ui/layers-panel

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| lpr_name_fit | Add a layer named `r1 r2`, refresh, then rename it to `f2-1` and refresh. Read the name text the row paints. | The painted text is the full `r1 r2` / `f2-1`, with no ellipsis. | `tst_layers_panel_bundle::shortLayerNameIsNotElided` |
| lpr_name_elide | Rename the same layer to a 200-character name and refresh. Read the name text the row paints. | The painted text is shorter than the name and ends in a single ellipsis. | `tst_layers_panel_bundle::shortLayerNameIsNotElided` |
