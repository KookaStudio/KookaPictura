# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cmyk_white` | Convert sRGB white to CMYK | All components 0 | `pictura-app::picker_cmyk_white_has_no_ink` |
| `cmyk_black_profile` | Convert sRGB black to CMYK | Equals the profile's transform; not 0/0/0/100 | `pictura-app::picker_cmyk_black_follows_profile` |
| `cmyk_round_trip` | Convert muted colours to CMYK and back | Each channel within 3 | `pictura-app::picker_cmyk_round_trips_in_gamut` |
| `cmyk_installed_profile` | Install a second CMYK profile; sweep colours through all services | Each equals that profile's one-shot conversion and differs from coated | `pictura-app::picker_services_follow_the_installed_profile` |
| `cmyk_fields_installed_pair` | Open the picker on a colour; read its fields and `rgbToCmyk` | Both equal `picker_cmyk` | `tst_color_picker::cmykFieldsUsePickerService` |
