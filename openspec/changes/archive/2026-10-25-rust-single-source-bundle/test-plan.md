# Test plan

Rust tests run under `cargo nextest run --workspace`. Qt suites run offscreen
through CTest. Rows whose test does not exist yet are written in the first task of
their unit, before that unit's implementation.

## compositing/blend-modes (#271)

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `blend_list_engine` | Enumerate the bridged blend rows. | 27 rows in `LAYER_MODES` order, no Pass Through. Each key decodes back to its mode, each name equals `BlendMode::name()`, and `has_neutral` equals `neutral_color(mode).is_some()`. | `pictura-app::blend_mode_rows_follow_layer_modes` |
| `blend_list_combos` | Build the Layers panel (combo and Mode filter), the New Layer dialog and the Layer Style dialog, then read each blend combo. | Item texts are the engine names in engine order. The Layers panel, filter and New Layer combos carry the engine keys as data. The Layer Style combo is index-only. | `tst_blend_mode_list::layerCombosFollowEngineList`, `tst_blend_mode_list::layerStyleComboIsEngineIndexed` |
| `blend_list_set` | Pick index *i* in the Layers panel blend combo for several *i*. | The active layer's blend key equals the engine key at *i*. | `tst_blend_mode_list::comboIndexSetsEngineKey` |

## color/color-model-conversions (#269)

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lab_white` | Convert sRGB white to Lab. | Rounds to `100, 0, 0`. | `pictura-color::lab_white_is_neutral` |
| `lab_known_values` | Convert red, green, blue, mid-grey and a 15-step sweep of the sRGB cube, and compare each with an lcms2 sRGB→Lab (D50) transform. | Within 1 unit per component. | `pictura-color::lab_matches_lcms_connection_space` |
| `lab_round_trip` | Round-trip a stepped grid of 8-bit sRGB through unrounded Lab. | Each channel is within 1 level. | `pictura-color::lab_round_trips_srgb_grid` |
| `lab_clamps` | Convert `L 50, a 127, b −128` and other out-of-gamut Lab to sRGB. | Channels are in 0–255, with no wrap. | `pictura-color::lab_out_of_gamut_clamps` |
| `cmyk_white` | Convert sRGB white to CMYK. | All components round to 0. | `pictura-color::cmyk_white_has_no_ink` |
| `cmyk_black_profile` | Convert sRGB black to CMYK, and compare with a direct lcms2 transform through the working profile. | Equal to the profile result, and not `0, 0, 0, 100`. | `pictura-color::cmyk_black_follows_profile` |
| `cmyk_round_trip` | Round-trip in-gamut sRGB samples through CMYK. | Within the stated profile tolerance. | `pictura-color::cmyk_round_trips_in_gamut` |
| `lab_picker_info_agree` | Set the picker to a colour, sample a pixel of that colour in the Info panel's Lab mode, and compare. | Same rounded L, a and b. | `tst_info_panel::labMatchesColorPicker` |
| `picker_lab_edit` | Type Lab values into the picker. | RGB and hex equal the bridge's `lab_to_srgb` result. | `tst_color_picker::labEditUpdatesRgb` |
| `cmyk_picker_info_agree` | Set the picker to a colour, sample the same colour in the Info panel's CMYK mode, and compare. | Same rounded C, M, Y and K. | `tst_info_panel::cmykMatchesColorPicker` |
| `channels_plate_source` | Feed `channel_plane` RGBA samples for CMYK and Multichannel rows, and compare with `channel_value` and with literal values from the pre-move C++ formula. | Equal to both. | `pictura-app::channel_plane_matches_selection_values` |

## ui/info-histogram-panel (#269)

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `info_cmyk_white` | Hover a white pixel with the default blocks. | Each of `C : 0`, `M : 0`, `Y : 0`, `K : 0` is shown. | `tst_info_panel::colorBlockReadsRgbAndSwitchesModes` |
| `info_cmyk_offcanvas` | Move the cursor off the canvas. | The colour blocks are blank. | `tst_info_panel::offCanvasBlanksColorReadouts` |
| `info_cmyk_profile` | Hover a black pixel in CMYK mode. | Components equal the bridge's CMYK for black, not the naive `0/0/0/100`. | `tst_info_panel::cmykReadoutFollowsProfile` |

## imaging/adjustment-catalogue (#272)

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `adjust_catalogue_complete` | Enumerate the catalogue. | 25 unique kinds. The dialog rows are the 16 kinds in CS6 order (Color Lookup after Channel Mixer), and each has a default block. | `pictura-render::adjustment_catalogue_covers_every_kind_once` |
| `adjust_catalogue_commands` | Resolve each row's Image path and layer path against the command catalogue. | Every path has a `CommandSpec`, and the Auto rows resolve at the Image root. | `pictura-app::adjustment_catalogue_rows_resolve_commands` |
| `adjust_catalogue_layer_rows` | Filter the rows that have a layer entry. | The 16 CS6 layer adjustments in order, without Shadows/Highlights, and only Invert has no ellipsis. | `pictura-render::adjustment_catalogue_layer_entries` |
| `adjust_menu_unchanged` | Wire the main window, then check that each catalogue Image command is implemented with a handler and that the Image > Adjustments leaves keep their order. | Every row is wired, and the order is unchanged. | `tst_command_tree::imageAdjustmentsMenuFollowsCatalogue` |
| `adjust_layer_menu_unchanged` | Build Layer > New Adjustment Layer and the Layers panel's New Fill/Adjustment menu, then trigger one entry. | The leaves equal the catalogue layer rows, and the triggered entry adds that layer. | `tst_command_tree::layerAdjustmentMenu`, `tst_adjustments_panel::layersPanelAdjustmentMenuFollowsCatalogue` |
| `adjust_panel_tiles` | Build the Adjustments panel and its flyout menu, click an adjustment tile, then click Pattern. | Tile ids and icon ids (`adjustments.<kind>`) follow the catalogue (19, fills first), and the menu rows equal the catalogue menu labels, starting with "Brightness-Contrast". The click adds the kind, and Pattern stays inert. | `tst_adjustments_panel::tilesAndMenuFollowCatalogue` |
| `adjust_dialog_ranges_generic` | Open Brightness/Contrast and Vibrance and read each slider. | Range and default equal the page rows for the kind. | `tst_image_adjustments::genericDialogRangesMatchLayout` |
| `adjust_dialog_ranges_handlaid` | Open Exposure, Hue/Saturation, Color Balance, Black & White, Photo Filter, Channel Mixer and Levels on a fresh document and read every numeric control. Curves and Gradient Map have no layout ranges, and the Levels input points are coupled, so they are exempt. | Range, default and decimals equal the page rows for its key, with the spec's exempt controls skipped. | `tst_image_adjustments::handLaidDialogRangesMatchLayout` |

## tools/tool-catalogue (#274)

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tool_catalogue_rows` | Enumerate the Rust catalogue. | 71 rows with unique names. Within a group, every member that has a letter has the same one. | `pictura-script::tool_catalogue_rows_unique_and_grouped` |
| `tool_catalogue_implemented` | Filter the implemented rows. | Exactly the flagged rows, in order. | `pictura-script::tool_catalogue_implemented_set` |
| `tool_catalogue_order` | Compare a test-local 71-entry `{ToolId, name}` table with the bridged rows by index. | Count equals `kToolCount`, and every enumerator's name matches its row. | `tst_keyboard_shortcuts::toolIdOrderMatchesCatalogue` |
| `tool_catalogue_consumers` | Compare every tool's label, group, shortcut, hint, modifier hints (with command ids), cursor and hotspot with a golden captured from the pre-move table. The existing toolbox, hint-bar and search suites stay green. | Identical. | `tst_keyboard_shortcuts::toolCatalogueMatchesGolden` |
| `tool_catalogue_selftest` | Run both headless self-test invocations through `scripts/test-report.sh`. | 98, 116, 117 and 536 (`healing_tools`) pass with the same codes. | manual: the headless self-test is a hand-rolled oracle with no `tst_` or crate test owner; `scripts/test-report.sh` runs it |

## ui/view-seam-adapters (#403)

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `type_session_commit` | Begin an edit session on a type layer, then commit new text. | The text changes, one history state is added, and the layer is visible. | `tst_type_tools::editSessionCommitsOnce` |
| `type_session_cancel` | Begin a session and let it go out of scope. | The layer is unchanged and visible, and the history count is unchanged. | `tst_type_tools::droppedEditSessionCancels` |
| `type_warp_session` | Preview a warp, then drop the session. Repeat and commit instead. | The drop restores the layer with no state. The commit records one "Warp Text" state. | `tst_warp_text::warpSessionCancelsOrCommitsOnce` |
| `type_style_library` | Create three character styles and two paragraph styles, then read both libraries. | One call per library returns every name, font and setting in order. | `tst_character_styles_panel::styleLibraryReadsInOneCall`, `tst_paragraph_styles_panel::styleLibraryReadsInOneCall` |
| `type_header_confined` | Scan `crates/pictura-app/cpp`, except `api/type_api.{h,cpp}` and `tests/`, for includes of `type_tools.cxxqt.h` or `type_tools/*.cxxqt.h`. | None. | `pictura-app::type_bridge_headers_stay_in_adapter` |
