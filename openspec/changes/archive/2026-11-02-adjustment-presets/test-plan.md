# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `preset_menu_lists` | Open a Levels adjustment layer, save a preset named `My Look`, inspect the Properties Preset menu | The menu lists `Default`, the Levels built-ins, `My Look`, and `Custom` | `tst_adjustment_presets::propertiesMenuListsBuiltinsAndSaved` |
| `preset_menu_hidden` | Open a Brightness/Contrast adjustment layer | The Properties panel has no Preset menu | `tst_adjustment_presets::unsupportedKindHasNoMenu` |
| `preset_apply_history` | Choose `Increase Contrast 1` in the Levels Preset menu | The parameters change and one `Modify Levels Layer` state is added | `tst_adjustment_presets::applyingAPresetIsOneHistoryState` |
| `preset_custom` | Apply a preset, then move a Levels slider | The Preset menu reads `Custom` | `tst_adjustment_presets::aHandEditShowsCustom` |
| `preset_save_persists` | Save `My Look` on a Levels layer, then add a fresh Levels layer and reopen the panel | `My Look` is listed and applies the same values | `tst_adjustment_presets::savePresetAppendsAndPersists`, `pictura-render::preset_store_round_trips` |
| `preset_save_rejects_reserved` | Try to save `Default`, `Custom`, and an empty name | No preset is stored; the menu is unchanged | `tst_adjustment_presets::reservedNamesAreRejected`, `pictura-render::preset_store_rejects_reserved_names` |
| `preset_save_whitelist` | Query the save gate for the seven whitelist kinds, Photo Filter, and Vibrance | The seven allow saving; Photo Filter and Vibrance do not | `tst_adjustment_presets::saveIsWhitelisted`, `pictura-render::saveable_and_menu_kinds_match_the_spec` |
| `preset_load_panel` | Save a Levels preset, reset the layer, then run `Load Preset…` and choose it | The layer parameters match the saved block | `tst_adjustment_presets::panelLoadAppliesASavedPreset` |
| `preset_load_dialog` | Save a Levels preset, open the Levels dialog, and choose it in the dialog Preset menu | The dialog fields and preview show the saved values | `tst_adjustment_presets::dialogLoadAppliesASavedPreset` |
| `preset_builtins_established` | Open the Levels, Curves, and Photo Filter Preset menus | Each lists its established names, and the kinds with no list show `Default` only | `tst_adjustment_presets::establishedBuiltinsAreListed` |
| `preset_photo_filter_density` | Set Density to 50 %, then choose `Cooling Filter (80)` | The colour changes and Density stays 50 % | `tst_adjustment_presets::photoFilterPresetKeepsDensity` |
| `preset_survives_restart` | Save a preset, rebuild the panel for the same kind | The preset is listed and applies the same block | `tst_adjustment_presets::presetsSurviveARestart` |
| `preset_malformed_store` | Write malformed lines into the kind's store file and open the Preset menu | The menu reads an empty saved list without crashing | `tst_adjustment_presets::malformedStoreReadsEmpty`, `pictura-render::preset_store_ignores_malformed_lines` |
| `preset_kind_mapping` | Map the default block for every preset kind | Each returns its catalogue key | `pictura-render::preset_kind_mapping_covers_the_catalogue` |
| `preset_decoded_match` | Compare two blocks that decode to the same adjustment | The matcher reports a match | `pictura-render::decoded_blocks_match` |
