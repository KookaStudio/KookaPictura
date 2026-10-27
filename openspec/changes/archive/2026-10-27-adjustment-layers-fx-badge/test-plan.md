# Test plan

Every scenario in the delta spec has a row. CI runs the Qt suites; no local
build is required for this change.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A double-click opens Blending Options | Double-click a pixel row outside the name | Layer Style opens on Blending Options | `tst_layers_panel::layerStylePanelIntegration` |
| A background double-click does not open a style | Double-click the Background outside the name | The conversion path runs and no style opens | `tst_fill_layer_editor::backgroundDoubleClickConverts` |
| The style rows report their enablement | Open the menu on a styled row with a copied style | All four style rows enabled | `tst_layers_panel::layerStylePanelIntegration` |
| An unstyled pixel row disables Copy and Clear | Open the menu on an unstyled pixel row | Blending Options enabled, Copy/Paste/Clear disabled | `tst_layers_panel::layerStylePanelIntegration` |
| An adjustment row omits the style rows | Open an adjustment row's menu | No style rows | `tst_layers_panel::layerStylePanelIntegration` |
| The row menu clears a row's style | Clear a styled row's style | Style removed, fx badge gone, one state | `tst_layers_panel::layerStylePanelIntegration` |
| Alt-clicking the fx region toggles all effects | Alt-click a styled row's fx region | Effects hide, then show, one state each | `tst_layers_panel::layerStylePanelIntegration` |
| The Effect dimension matches styled rows | Filter by an effect | Only rows carrying it remain | `tst_layers_panel::layerStylePanelIntegration` |
| The Effect dimension lists the effect keys | Open the Effect dimension choices | The ten effect keys | `tst_layers_panel::layerStylePanelIntegration` |
| `lpr_adjustment_thumb_opens` | Double-click an adjustment content thumbnail | Properties shown, no Layer Style | `tst_fill_layer_editor::adjustmentThumbOpensProperties` |
| `lpr_fill_thumb_opens` | Double-click a fill content thumbnail | The fill editor opens, no Layer Style | `tst_fill_layer_editor::thumbDoubleClickOpensEditor` |
| `lpr_fx_absent_adjustment` | Add an adjustment and a solid-fill layer, then add a color overlay to the adjustment | Adjustment and fill rows paint no fx badge and reserve no fx space; the styled adjustment row paints one | `tst_layers_effects::adjustmentRowsShowNoFxBadge` |
