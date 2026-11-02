# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `pressure_buttons_toggle` | Click each pressure button, then round-trip the brush JSON | Flags and buttons agree; flags survive | `tst_brush_airbrush::pressureButtonsToggleAndPersist` |
| `pressure_opacity_lightens` | Paint at pressure 1.0 and 0.5 with the opacity override on | Half pressure is lighter | `tst_brush_airbrush::pressureOpacityLightensTheDab`, `pictura-paint::pressure_opacity_lightens_the_dab` |
| `pressure_size_shrinks` | Paint at pressure 1.0 and 0.5 with the size override on | Half pressure covers less | `tst_brush_airbrush::pressureSizeShrinksTheDab`, `pictura-paint::pressure_size_scales_the_dab_footprint` |
| `pressure_hold_keeps` | Press at half pressure with opacity override, hold 50 dabs | Stops at the half-pressure cap | `pictura-paint::hold_keeps_the_last_pressure` |
| `pressure_scaled_helper` | Call the scaling helper with each override on/off | Only the named fields scale | `pictura-paint::pressure_scaled_applies_the_overrides` |
| `pressure_tablet_routing` | A real tablet press/move over the canvas | The next dab uses the tablet pressure | manual: offscreen Qt has no tablet device |
| Preview pressure | A large pressure stroke previews and commits | The commit keeps the pressure build-up | manual: the preview shares the exact helper; `tst_brush_airbrush::previewHoldReachesTheCommit` covers the preview commit path |
| Icons | The two buttons are built | Both icons load and are distinct | `tst_brush_airbrush::pressureButtonsToggleAndPersist` |
