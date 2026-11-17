# Test plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `cp_alpha_eye_shows` | Create an alpha channel with a rectangle; click its eye. | The eye is checked; the canvas overlay is red at 50 % over the channel's masked (black) area. | `tst_channels_panel::alphaEyeShowsChannel` |
| `cp_alpha_eye_toggles` | Click the checked alpha eye again. | The eye is unchecked and the overlay is cleared. | `tst_channels_panel::alphaEyeShowsChannel` |
| `cp_alpha_composite_restores` | Show an alpha channel, then click the composite eye. | The alpha view is cleared and the composite returns. | `tst_channels_panel::alphaEyeShowsChannel` |
| `cp_alpha_delete_clears` | Show an alpha channel and delete it with the Delete button. | No alpha is shown; the channel is removed. | `tst_channels_panel::deletingShownAlphaClearsView` |
| `cp_alpha_view_display_only` | Show an alpha channel and compare channel count/history before and after. | The document state is unchanged. | `tst_channels_panel::alphaEyeShowsChannel` |
| `cp_alpha_eye_selects` | Click an alpha eye and check the current row. | The alpha row becomes current. | `tst_channels_panel::clickAlphaEyeSelectsRow` |
