# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cl_prompt_choices` | Pending box; choose Move; answer Cancel, Don't Crop, then Crop | Stays; switches uncropped; switches cropped to the box | `tst_crop_settings::toolSwitchAsksToCrop` |
| `cl_prompt_default_box` | Crop tool with its default box; choose Move | Nothing asked | `tst_crop_settings::toolSwitchAsksToCrop` |
| `cl_prompt_perspective` | Complete quad; choose Move; Cancel then Don't Crop | Stays; switches | `tst_crop_settings::perspectiveSwitchAsksToCrop` |
| `cl_perspective_reset` | Complete quad; press Reset | Quad and sizes cleared; buttons hidden; tool stays | `tst_perspective_crop::optionsBarApplyCancelFollowQuad` |
| `cl_menu_spacing` | Open both menus and compare | Same row and separator spacing | manual: visual spacing cannot be asserted offscreen |
