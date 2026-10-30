# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cmyk_white` | Convert sRGB white to CMYK | All components 0 | `pictura-app::picker_cmyk_white_has_no_ink` |
| `cmyk_black_profile` | Convert sRGB black to CMYK | Equals the profile's transform; not 0/0/0/100 | `pictura-app::picker_cmyk_black_follows_profile` |
| `cmyk_round_trip` | Convert muted colours to CMYK and back | Each channel within 3 | `pictura-app::picker_cmyk_round_trips_in_gamut` |
| `cmyk_installed_profile` | Install a second CMYK profile; sweep colours through all services | Each equals that profile's one-shot conversion and differs from coated | `pictura-app::picker_services_follow_the_installed_profile` |
| `cmyk_fields_installed_pair` | Open the picker on a colour; read its fields and `rgbToCmyk` | Both equal `picker_cmyk` | `tst_color_picker::cmykFieldsUsePickerService` |
| `cmyk_installed_options` | Install Perceptual with BPC off; sweep colours through readout, edit, gamut and closest | Each equals that option's one-shot conversion and differs from the relative+BPC default | `pictura-app::picker_services_follow_the_installed_options`, `tst_color_settings::pickerFollowsConversionOptions` |
| The defaults match CS6 | Start with no stored preference | Relative Colorimetric, BPC on, dither on | `tst_color_settings::defaults` |
| A stored option survives a restart | Set Perceptual; save and reload the session | Reported intent is Perceptual | `tst_color_settings::preferencesSurviveRestart` |
| Changing an option is not a document edit | Change an option with a document open | History count and dirty state unchanged | `tst_color_settings::notADocumentEdit` |
| `cs_picker_follows_options` | Change the intent with a view; read `picker_cmyk` before and after | The readout changes to the option's conversion | `tst_color_settings::pickerFollowsConversionOptions` |
| Every intent produces a transform | Convert one RGB buffer with each intent | Expected length and more than eight distinct values | `pictura-color::every_intent_produces_output` |
| Black-point compensation reaches the transform | Convert with BPC on and off | Shadow outputs differ; the on case matches lcms2 | `pictura-color::black_point_compensation_changes_shadows` |
| Dither is deterministic and 8-bit only | Convert twice with dither on; convert 16-bit | Byte-identical and different from undithered; 16-bit ignores it | `pictura-color::dither_is_deterministic_and_8bit_only` |
| `picker_shared_options` | Give the shared options an intent and BPC; convert a colour | The picker uses them and ignores dither | `pictura-app::picker_services_follow_the_installed_options`, `tst_color_settings::pickerFollowsConversionOptions` |
