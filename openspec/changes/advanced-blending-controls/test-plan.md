# Test plan

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| Values and order | Open dialog with mixed flags and inspect controls. | Each value and order match the layer. | `tst_layer_style::advancedBlendingFlags`, `pictura-render::advanced_blending_flag_values` |
| Live edit and cancel | For each flag, use a layer with an effect and suitable transparency, raster mask, or vector mask; record a distinguishing canvas pixel, toggle its control, and cancel. | The corresponding flag and canvas pixel change live; cancellation restores both the original flag and pixel. | `tst_layer_style::advancedBlendingFlags`, `tst_layer_style::advancedBlendingPreview` |
| Accept and undo | Change flags, accept, reopen, undo. | One history entry; values persist then undo. | `tst_layer_style::advancedBlendingFlags` |
| Save and reopen | Change flags, save PSD and PSB, reopen. | Dialog shows saved flags. | `tst_layer_style::advancedBlendingRoundTrip` |
