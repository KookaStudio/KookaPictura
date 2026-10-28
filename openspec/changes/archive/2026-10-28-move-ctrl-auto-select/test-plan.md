# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mca_ctrl_click_selects` | Auto-Select off, Layer target; active layer is an empty layer on top; Ctrl-press over a lower copy's pixels | The lower copy becomes active | `tst_options_bars_parity::moveCtrlClickAutoSelects` |
| `mca_ctrl_disables` | Auto-Select on; Background active; Ctrl-press over a copy's pixels | Background stays active | `tst_options_bars_parity::moveCtrlClickSuppressesAutoSelect` |
| `mca_checkbox_shows_flip` | Auto-Select off; canvas reports Ctrl down, then up | Checked + dropdown enabled while held; restored on release; stored option stays off | `tst_options_bars_parity::moveCtrlShowsFlippedAutoSelect` |
| `mca_ctrl_follows_mode` | Auto-Select off, Group target; a copy wrapped in a group; Ctrl-press over its pixels | The group becomes active | `tst_options_bars_parity::moveCtrlClickFollowsGroupMode` |
| Real keyboard and pointer | In `./build/pictura` with the issue's file: Move tool, Auto-Select off; hold Ctrl, Ctrl-click the bottom black square, release | The box shows checked while held; `Layer 1` is selected; the box unchecks on release | manual: real key and pointer events through the window system are not observable offscreen |
