# Test Plan — channels-panel-live

One row per spec scenario in `specs/ui/channels-panel/spec.md`.
Coverage names an automated test (`<crate>::<fn>` for Rust, `tst_<suite>::<case>`
for Qt Test) or `manual: <reason>`.

| Scenario | Steps | Coverage |
|---|---|---|
| `cp_rows_four` | Refresh an RGB document with no mask on the active layer and assert the four rows and their Ctrl labels | `tst_channels_panel::rowsAndThumbnails` |
| `cp_rows_mask` | Give the active layer a raster mask and assert an extra `<layer name> Mask` row appears after the colour rows | `tst_channels_panel::maskChannelRow` |
| `cp_rows_mask_change` | Change the active layer to one with no raster mask and assert the mask row is removed | `tst_channels_panel::maskChannelRow` |
| `cp_rows_stable` | Refresh again with the active layer unchanged and assert the row count is unchanged | `tst_channels_panel::refreshKeepsRows` |
| `cp_live_visibility` | Hide Red then Green, try to hide Blue, click the composite eye, and switch documents, reading the canvas pixel each time | `tst_channels_panel::eyesHideChannelsOnTheCanvas` |
| `cp_alpha_footer` | Create and delete a channel, save a rectangle selection as a channel, deselect, select it, and load it | `tst_channels_panel::alphaChannelsAndFooter` |

## Manual rows

None.
