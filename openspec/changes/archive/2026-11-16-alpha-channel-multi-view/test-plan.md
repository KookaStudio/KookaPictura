# Test plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `cp_alpha_eye_shows` | Click one alpha eye. | The eye is checked and the canvas shows a red 50 % overlay over the channel's masked area. | `tst_channels_panel::alphaEyeShowsChannel` |
| `cp_alpha_eye_toggles` | Click the checked eye again. | The channel is removed and the eye is unchecked. | `tst_channels_panel::alphaEyeShowsChannel` |
| `cp_alpha_multiple` | Click a second alpha eye while the first is shown. | Both eyes stay checked and the canvas red deepens where both channels are masked. | `tst_channels_panel::alphaEyesShowMultiple`, `tst_channels_panel::alphaEyesAccumulateRedness` |
| `cp_alpha_composite_restores` | Show two channels, then click the composite eye. | Both are cleared and the composite returns. | `tst_channels_panel::alphaEyeShowsChannel`, `tst_channels_panel::alphaEyesShowMultiple` |
| `cp_alpha_delete_clears` | Delete one shown channel. | Its view is removed; any other shown channel stays. | `tst_channels_panel::deletingShownAlphaClearsView` |
| `cp_alpha_view_display_only` | Show channels and compare state. | Channel pixels, selection, and history are unchanged. | `tst_channels_panel::alphaEyeShowsChannel` |
| `cp_alpha_eye_selects` | Click an alpha eye. | Its row becomes current. | `tst_channels_panel::clickAlphaEyeSelectsRow` |
