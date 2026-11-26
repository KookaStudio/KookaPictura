# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| An RGB document | Open an 8-bit RGB document and inspect the Mode submenu. | RGB and 8 bits are checked; Multichannel and the other supported conversions are enabled; the unsupported entries are dimmed. | `tst_image_mode::menuChecksTheModeAndDimsUnavailableTargets` |
| Indexed preview then cancel | Preview Indexed Color on a layered document and cancel. | The original mode and layers return with no new history state. | `tst_image_mode::indexedPreviewsCancelsAndFlattens` |
| Grayscale and undo | Convert RGB to Grayscale and undo. | One conversion state is added and undo restores the original RGB pixels. | `tst_image_mode::grayscaleIsOneStateAndUndoRestoresColor` |
| `multi_menu_history` | Open layered RGB, cancel the flatten prompt once, then accept it and undo. | Cancel preserves history; acceptance produces named spot rows in one state; undo restores RGB and the layers. | `tst_image_mode::multichannelPromptsAndUndoes` |
| `multi_menu_gate` | Activate Indexed Color and 32-bit RGB documents in turn. | Both dim Multichannel. | `tst_image_mode::multichannelDimsUnsupportedSources` |
| `multi_rgb_split` | Convert visible RGB artwork with a hidden layer and one saved selection. | Three CMY spot planes from the visible artwork precede the unchanged selection, and no layers remain. | `pictura-render::multichannel_flattens_visible_rgb_and_preserves_extra` |
| `multi_components` | Convert CMYK, Lab, and Grayscale fixtures. | The correct names and source component values remain in channel order. | `pictura-render::multichannel_preserves_source_component_order` |
| `multi_native_round_trip` | Convert a flat native RGB fixture with non-repeated low bytes and a saved channel, and save and read it as PSD and PSB. | Both retain depth, mode, names, and every 16-bit sample. | `pictura-render::multichannel_native_plates_round_trip` |
| `multi_refusal` | Attempt conversion with unsupported modes, depth, or channel capacity. | Images compare equal to their pre-call state after each refusal. | `pictura-render::multichannel_refuses_without_mutation` |
| `multi_read_compatibility` | Read named or DisplayInfo-bearing mode-7 files at both depths and unnamed legacy and unsupported layouts. | Named files retain plates; legacy previews and write-back remain; unsupported unnamed layouts fail. | `pictura-codec::multichannel_channel_owned_and_legacy_read` |
| `multi_plate_polarity` | Convert RGB black, white and primaries at both depths. | Low samples indicate ink and high samples no ink. | `pictura-render::multichannel_rgb_plate_polarity` |
| `multi_layer_refusal` | Insert a layer into a channel-owned Multichannel document. | Insertion fails with document and history unchanged. | `tst_image_mode::multichannelRejectsLayerInsertion` |
| `multi_rgb_exit` | Convert channel-owned Multichannel to RGB, then undo. | Preview becomes RGB, every plate and pre-existing extra remains exactly once as a normal extra channel; fabricated mode-7 rows disappear, undo restores plates. | `tst_image_mode::multichannelReturnsToRgbAndUndoes` |
| `multi_load_selection` | Load each named plate in grayscale and CMYK conversions with an extra selection channel. | Each actual plate appears once and loads its stored sample through spot handling; the extra loads unchanged. | `tst_selection_save_load::multichannelLoadsStoredPlates` |
| An Indexed document opens as RGB with palette colors | Read indexed fixture. | Palette colors become RGB. | `pictura-codec::indexed_depth8_maps_palette_indices` |
| A CMYK document opens as RGB | Read profile-tagged CMYK. | Composite uses its profile. | `pictura-codec::cmyk_profile_read_uses_embedded` |
| An untagged CMYK document opens through the working profile | Read untagged CMYK with working profile. | Composite uses working profile. | `pictura-codec::cmyk_profile_read_uses_working` |
| An untagged CMYK document with no working profile uses the fallback | Read untagged CMYK without profile. | Composite uses fallback. | `pictura-codec::cmyk_depth8_uses_profile_free_formula` |
| A Lab document opens as RGB | Read Lab fixture. | Composite becomes RGB. | `pictura-codec::lab_depth8_neutral_gray_and_white_within_two` |
| A Bitmap document opens as black and white RGB | Read depth-1 Bitmap. | Bits become black and white. | `pictura-codec::bitmap_depth1_raw_expands_black_white` |
| Grayscale and RGB are unchanged | Read Grayscale and RGB fixtures. | Their modes remain unchanged. | `pictura-codec::round_trip_rgb_and_grayscale` |
| Duotone opens as grayscale-normalized RGB | Read Duotone fixture. | RGB and source mode retained. | `pictura-codec::duotone_opens_as_rgb_and_preserves_color_mode_data` |
| One-channel Multichannel opens as grayscale-normalized RGB | Read unnamed single plate. | Legacy grayscale preview preserved. | `pictura-codec::multichannel_one_channel_opens_as_gray_rgb` |
| Three-channel Multichannel opens as CMY-normalized RGB | Read unnamed three plates. | Legacy CMY preview preserved. | `pictura-codec::multichannel_three_channels_maps_cmy_to_rgb` |
| Other Multichannel channel counts stay unsupported | Read unnamed two-plate file. | Unsupported without panic. | `pictura-codec::multichannel_two_channels_is_unsupported` |
| Unknown mode codes stay unsupported | Read unknown mode code. | Unsupported without panic. | `pictura-codec::unsupported_depths_and_color_modes_are_rejected` |
| Depth 16 and 32 are normalized with the mode | Read 16/32-bit CMYK and Lab. | Samples narrow and convert. | `pictura-codec::depth16_cmyk_preserves_source_mode_and_depth`, `pictura-codec::depth32_cmyk_preserves_source_mode_and_depth`, `pictura-codec::high_depth_lab_preserves_source_mode_and_depth` |
| Load inverts before combining | Load with invert and Add. | Coverage combines after inversion. | `tst_selection_save_load::loadDialogInvertAndOps` |
| Load from another document | Load channel across documents. | Coverage and history match. | `tst_selection_save_load::loadDialogCrossDocument` |
| Load lists existing channels | Open Load with saved channels. | Stored names are listed. | `tst_selection_save_load::loadDialogDefaultsToExtraChannel` |
| sm_load_lists_colour | Open RGB Load with mask and extra. | Rows follow specified order. | `tst_selection_save_load::loadDialogListsColourRows` |
| sm_load_colour_plane | Load each RGB plane. | Coverage matches plane. | `tst_selection_save_load::loadColourRowCoverage` |
| sm_load_composite | Load composite RGB row. | Coverage matches luminance. | `tst_selection_save_load::loadColourRowCoverage` |
| sm_load_no_saved | Open Load without saved channels. | Colour rows and OK are available. | `tst_selection_save_load::loadDialogWithNoSavedChannels` |
| sm_load_single_row_mode | Load grayscale row. | One Gray row loads luminance. | `tst_selection_save_load::loadDialogGrayscaleCompositeRow` |
| sm_load_default | Open Load with an extra channel. | First extra is selected. | `tst_selection_save_load::loadDialogDefaultsToExtraChannel` |
| sm_load_multichannel_default | Open legacy Multichannel Load. | First legacy colour row is selected. | `tst_selection_save_load::loadDialogModeRows` |
| sm_load_colour_cross_doc | Load colour from another document. | Coverage and history match. | `tst_selection_save_load::loadColourRowCrossDocument` |
| Wrong-length channel is refused | Load wrong-length channel. | Refused without history. | `tst_selection_save_load::wrongLengthRefused` |
| Round trip is exact | Save and load selection. | Coverage is byte exact. | `pictura-app::save_then_load_round_trip` |
| Zero dimension | Read a zero-dimension header. | Invalid error. | `pictura-codec::psd_header_zero_dimension_is_invalid` |
| Depth 16 and 32 are accepted | Read high-depth ordinary modes. | Header accepted and samples narrowed. | `pictura-codec::depth16_cmyk_preserves_source_mode_and_depth`, `pictura-codec::depth32_cmyk_preserves_source_mode_and_depth`, `pictura-codec::high_depth_lab_preserves_source_mode_and_depth` |
| Unsupported depth | Read invalid depths and high-depth Bitmap and Indexed. | Unsupported error. | `pictura-codec::unsupported_depths_and_color_modes_are_rejected` |
| Bitmap depth 1 is accepted | Read depth-1 Bitmap. | Header accepted and rows decoded. | `pictura-codec::bitmap_depth1_raw_expands_black_white` |
| Channel count out of range | Read zero or over-limit channels. | Invalid error. | `pictura-codec::psd_header_channel_count_is_invalid` |
| A non-RGB color mode is accepted | Read Indexed, CMYK and Lab files. | Headers accepted and normalized. | `pictura-codec::indexed_depth8_maps_palette_indices`, `pictura-codec::cmyk_depth8_uses_profile_free_formula`, `pictura-codec::lab_depth8_neutral_gray_and_white_within_two` |
| multi_header_modes | Read supported and unsupported Multichannel and Duotone layouts. | Supported layouts open and unsupported layouts fail. | `pictura-codec::multichannel_channel_owned_and_legacy_read`, `pictura-codec::duotone_opens_as_rgb_and_preserves_color_mode_data` |
| A Lab document saves as Lab | Read and write Lab fixture. | Lab header and output agree. | `pictura-codec::lab_document_writes_back_as_lab` |
| A CMYK document saves as CMYK | Read and write CMYK fixture. | CMYK header and native planes preserved. | `pictura-codec::cmyk_document_writes_back_as_cmyk` |
| An unedited Lab document saves losslessly | Read and write unedited Lab. | Native planes and RGB survive. | `pictura-codec::lab_document_writes_back_as_lab` |
| An unedited CMYK document saves losslessly | Read and write unedited CMYK. | Native planes and RGB survive. | `pictura-codec::cmyk_document_writes_back_as_cmyk` |
| An edited Lab plane is re-encoded approximately | Edit a Lab plane and save. | Approximate inverse is used. | `pictura-codec::edited_lab_plane_uses_the_approximate_inverse` |
| An edited CMYK plane re-encodes through the document profile | Edit profiled CMYK and save. | Profile conversion preserves RGB within tolerance. | `pictura-codec::cmyk_profile_write_reencodes` |
| An edited CMYK plane uses the exact inverse | Edit untagged CMYK and save. | Profile-free exact inverse used. | `pictura-codec::edited_cmyk_plane_uses_the_exact_inverse` |
| A Lab layer channel is converted | Save a Lab pixel layer. | Layer planes convert and reopen. | `pictura-codec::lab_layer_color_channels_write_back_as_lab` |
| A CMYK layer channel is converted | Save a CMYK pixel layer. | Four layer planes convert and reopen. | `pictura-codec::cmyk_layer_color_channels_write_back_as_cmyk` |
| A grouped Lab layer is converted | Read and write nested Lab pixel layer. | Nested pixels survive. | `pictura-codec::grouped_lab_layers_convert_on_read_and_round_trip` |
| A 16/32-bit Lab or CMYK source saves back in its source mode and depth | Read and write high-depth Lab and CMYK. | Mode, depth, and native planes survive. | `pictura-codec::depth16_cmyk_preserves_source_mode_and_depth`, `pictura-codec::depth32_cmyk_preserves_source_mode_and_depth`, `pictura-codec::high_depth_lab_preserves_source_mode_and_depth` |
| A profile-aware CMYK save carries the profile | Save profiled CMYK. | Profile and native planes survive. | `pictura-codec::cmyk_profile_write_keeps_profile` |
| Source mode is recorded | Read CMYK and Lab. | RGB working mode and source modes recorded. | `pictura-codec::cmyk_depth8_uses_profile_free_formula`, `pictura-codec::lab_depth8_neutral_gray_and_white_within_two` |
| Native modes record no source mode | Read RGB and Grayscale and construct document. | No source mode recorded. | `pictura-codec::native_and_constructed_documents_have_no_source_mode` |
| The Indexed palette is consumed | Read Indexed fixture. | Palette retained in source, not color-mode data. | `pictura-codec::indexed_depth8_maps_palette_indices` |
| A flat unchanged Bitmap document saves as Bitmap | Read and write flat unchanged Bitmap. | Bitmap header and packed bytes preserved. | `pictura-codec::bitmap_document_saves_as_bitmap` |
| An edited or layered Bitmap document saves as the working mode | Save edited, layered, and extra-channel Bitmap. | Output mode is RGB. | `pictura-codec::edited_bitmap_composite_falls_back_to_rgb`, `pictura-codec::layered_bitmap_falls_back_to_rgb`, `pictura-codec::bitmap_with_a_document_channel_falls_back_to_rgb` |
| An unchanged Indexed document saves back as Indexed | Read and write unchanged Indexed. | Palette and index planes survive. | `pictura-codec::indexed_document_writes_back_as_indexed` |
| An edited Indexed document saves as the working mode | Edit Indexed composite or layer and save. | Output mode is RGB. | `pictura-codec::edited_indexed_composite_falls_back_to_rgb`, `pictura-codec::edited_indexed_layer_falls_back_to_rgb` |
| An Indexed layer and its group are retained | Read and write grouped Indexed. | Nested index planes and pixels survive. | `pictura-codec::grouped_indexed_layer_round_trips` |
| An Indexed document without a merged composite saves back as Indexed | Save Indexed without merged composite. | Indexed header, palette and layer index survive. | `pictura-codec::indexed_document_without_a_merged_composite_writes_indexed` |
| An unchanged Duotone document saves as Duotone | Read and write unchanged Duotone. | Mode and source bytes survive. | `pictura-codec::duotone_opens_as_rgb_and_preserves_color_mode_data` |
| An edited Duotone document saves as RGB | Edit Duotone composite and save. | Output mode is RGB. | `pictura-codec::edited_duotone_and_multichannel_write_rgb` |
| An unchanged one-channel Multichannel document saves as Multichannel | Read and write unchanged legacy single plate. | Mode and plate bytes survive. | `pictura-codec::multichannel_one_channel_opens_as_gray_rgb` |
| An unchanged three-channel Multichannel document saves as Multichannel | Read and write unchanged legacy three plates. | Mode and plate bytes survive. | `pictura-codec::multichannel_three_channels_maps_cmy_to_rgb` |
| An edited Multichannel document saves as RGB | Edit legacy Multichannel composite and save. | Output mode is RGB. | `pictura-codec::edited_duotone_and_multichannel_write_rgb` |
| Geometry of a plate document | Crop, resize, rotate by a quarter turn, and expand the canvas at 8 and 16 bits. Save and reopen each result. | The plate samples and preview track the new canvas; uncovered spot areas use no ink and the document remains layerless. | `pictura-render::multichannel_geometry_keeps_plates_layerless_and_savable` |
| Save Selection modifies a spot plate | Replace a spot channel with a selection, then undo and redo. | Plate coverage and the preview change together and follow history. | `pictura-app::save_selection_on_spot_refreshes_plate_preview`, `tst_selection_save_load::multichannelSpotSelectionRefreshesAndUndoes` |
