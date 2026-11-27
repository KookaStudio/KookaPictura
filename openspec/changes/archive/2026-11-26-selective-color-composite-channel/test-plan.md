# Test plan

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| `selective_color_component` | Select Red. Inspect each launcher, try direct dispatch, then restore RGB. | Selective Color is disabled everywhere. No dialog, layer, or history entry appears. Another adjustment stays enabled. Restoring RGB re-enables the launchers. | `tst_adjustments_panel::selectiveColorRequiresComposite` |
| `selective_color_display` | Hide a working colour channel without changing the selected row, show an alpha, select a layer mask, and enter Quick Mask, restoring the composite between targets. | Each target disables Selective Color until the normal composite is restored. | `tst_adjustments_panel::selectiveColorRequiresCompositeDisplay` |
| `selective_color_external_mask_solo` | With the composite selected and a layer mask present, enter and exit mask solo using Alt-click on the mask thumbnail, then repeat using Ctrl+\. | Every launcher disables on entry and returns to its prior availability on exit, even when the Channels selection and mask name do not change. | `tst_adjustments_panel::selectiveColorFollowsExternalMaskSolo` |
| `selective_color_document` | Create two RGB documents, select a component in the active document, then switch to the other document. | Selective Color follows the composite selected by the active document's Channels panel. | `tst_adjustments_panel::selectiveColorFollowsDocument` |
| `selective_color_multichannel` | Open an existing Multichannel document and inspect the launchers. | Selective Color is unavailable. | manual: New Document cannot create Multichannel and no deterministic fixture is specified. |
| `preset_load_panel` | Save and load a Levels preset through Properties. | Saved parameters return. | `tst_adjustment_presets::panelLoadAppliesASavedPreset` |
| `preset_load_dialog` | Choose a saved Levels preset in the Levels dialog. | Dialog parameters show the preset values. | `tst_adjustment_presets::dialogLoadAppliesASavedPreset` |
| `selective_color_preset_round_trip` | Save a named Selective Color preset with nondefault method and family values. Reset and load through Properties, then select it in a fresh dialog. | Both surfaces restore the saved settings. | `tst_adjustment_presets::selectiveColorPresetRoundTrips` |
