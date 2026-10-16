# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cp_ctrl_click_channel` | Open a flat (100, 150, 200) RGB image; Ctrl-click the Red, Green, Blue, then composite rows | Selection coverage 100, 150, 200, then 140 | `tst_channels_panel::ctrlClickLoadsChannelSelection` |
| `cp_ctrl_click_visible` | Hide the Red channel, then Ctrl-click the composite row | Coverage is qGray(0, 150, 200) = 106 | `tst_channels_panel::ctrlClickLoadsChannelSelection` |
| `cp_ctrl_click_plain` | Click a row without Ctrl | The current row moves and the selection is unchanged | `tst_channels_panel::ctrlClickLoadsChannelSelection` |
