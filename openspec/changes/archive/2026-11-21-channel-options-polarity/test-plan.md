# Test plan

Covers every scenario in the `tools/selection-channels` and `ui/channels-panel`
deltas. The sense is stored in PSD DisplayInfo resource 1077; the codec tests
cover storage, PDF defaults, and round-trip, the app/render tests cover
canonical-coverage conversion and channel clearing, and the Qt Test cases cover
the Channel Options affordance.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `chan_sense_default` | Create a document, add an extra channel with New Channel, inspect its plane and stored mode | Plane is all zero (nothing selected) and the channel loads as `Masked Areas` with no mode entry required | `pictura-app::new_channel_defaults_to_masked`, `pictura-codec::channel_sense_missing_entry_is_masked` |
| `chan_sense_add_aligned` | Add a channel to a document that already has a `Selected Areas` channel and a DisplayInfo resource | The Selected channel still loads the same selection and the new channel is `Masked Areas` | `pictura-app::new_channel_keeps_other_senses` |
| `chan_sense_clear` | Convert a document with extra channels to Bitmap, then add a new channel | The new channel is `Masked Areas`, with no stale kind from the cleared channels | `pictura-render::bitmap_conversion_clears_display_info` |
| `chan_sense_pdf_default` | Save a layered document as a Photoshop PDF that preserves editing data, retain a channel whose DisplayInfo entry is missing, and reopen the payload | The retained channel is `Masked Areas`, not `Selected Areas` | `pictura-codec::pdf_preserve_editing_defaults_to_masked` |
| `chan_sense_selected_plane` | Save canonical coverage 200 into a channel whose sense is `Selected Areas`, read the plane, then load it | Every plane byte is 55 and the loaded coverage is 200 | `pictura-app::selected_channel_stores_protection` |
| `chan_sense_inverse` | Interpret the same plane bytes under `Masked Areas` and `Selected Areas` | Loaded coverage is the plane and `255 - plane` respectively | `pictura-app::same_plane_reads_opposite_under_each_sense` |
| `chan_sense_missing` | Read a document channel that has no DisplayInfo entry | Sense is `Masked Areas` and the plane is read as coverage | `pictura-codec::channel_sense_missing_entry_is_masked` |
| `chan_sense_roundtrip` | Write a document with one Masked and one Selected channel, read it back | Both senses and both planes survive unchanged | `pictura-codec::channel_sense_round_trips_through_psd` |
| `chan_sense_psd_tools` | Write the same document and open it with psd-tools, read resource 1077 | psd-tools reports kind 1 then kind 0 in channel order | `pictura-codec::psd_tools_reports_channel_sense_modes` |
| `chan_sense_delete_shift` | Delete the first of three extra channels that carry distinct senses and display fields | The remaining two keep their senses and every other DisplayInfo field at their new indices | `pictura-app::delete_channel_shifts_display_info`, `pictura-codec::deleting_a_channel_shifts_display_info` |
| `chan_sense_toggle` | Toggle a channel from `Masked Areas` to `Selected Areas` through the bridge | Plane is complemented, kind becomes 0, and the loaded selection is byte-identical | `pictura-app::sense_toggle_preserves_the_selection` |
| `chan_convert_save_replace` | Save coverage 64 into an existing `Selected Areas` channel with Replace, then load it | Plane holds 191 and loading returns 64 | `pictura-app::save_into_selected_channel_writes_protection` |
| `chan_convert_combine` | Combine a `Selected Areas` channel whose plane is 127 with incoming coverage 64 using Add | Canonical result is 128, stored plane stays 127, and one state is recorded | `pictura-app::combine_under_selected_channel_uses_canonical_coverage` |
| `chan_convert_load_invert` | Load a `Selected Areas` channel whose plane is 64 with `Invert` checked and New | Sense coverage 191 is inverted to 64 and the selection is 64 | `pictura-app::load_selected_channel_applies_sense_before_invert` |
| `chan_convert_load` | Load a `Selected Areas` channel whose plane is 64 with no invert | Selection coverage is 191 and one `Load Selection` state is recorded | `pictura-app::load_selected_channel_is_sense_aware` |
| `chan_convert_duplicate` | Duplicate a `Selected Areas` channel within the document and drag it to another same-size document | Each destination gains a `Masked Areas` channel with the same canonical coverage, the source is unchanged, and the destination records one state | `tst_channel_options::duplicateSelectedChannelNormalizesToMasked` |
| `chan_convert_soft` | Load a `Selected Areas` channel whose plane is 127 | Coverage is 128 exactly, with no thresholding | `pictura-app::selected_channel_soft_coverage_is_not_quantized` |
| `cp_options_menu_row` | Move the current row across an alpha row, a colour row, a layer-mask row, the Quick Mask row, and a spot channel row | `Channel Options…` is enabled only for the alpha row | `tst_channel_options::channelOptionsMenuRow` |
| `cp_options_double_click` | Double-click an alpha channel row | The Channel Options dialog opens with that channel's sense selected | `tst_channel_options::channelOptionsDoubleClick` |
| `cp_options_cancel` | Open the dialog and cancel it | Plane, sense, selection, and history are unchanged | `tst_channel_options::channelOptionsCancel` |
| `cp_options_same_sense` | Accept the dialog without changing the sense | Channel and history are unchanged | `tst_channel_options::channelOptionsSameSense` |
| `cp_options_apply` | Switch a `Masked Areas` channel to `Selected Areas` and accept | Every plane byte is complemented, the loaded selection is unchanged, and history gains one `Channel Options` state | `tst_channel_options::channelOptionsApply` |
| `cp_options_thumbnail` | Switch an alpha channel's sense from `Masked Areas` to `Selected Areas` | The row thumbnail shows the complemented plane | `tst_channel_options::channelOptionsThumbnail` |
| `cp_options_overlay` | Show a `Selected Areas` alpha channel on the canvas | The red overlay covers the pixels the plane stores as black, which are its selected pixels | `tst_channel_options::channelOptionsOverlay` |
| `cp_options_overlay_refresh` | Show an alpha channel, then change its sense through Channel Options | The rubylith follows the complemented plane without another eye click | `tst_channel_options::channelOptionsOverlayRefresh` |

| `cp_ctrl_click_channel` | Ctrl-click the Red, Green, Blue, and composite rows of a flat (100, 150, 200) image | Coverage is 100, 150, 200, and 140 | `tst_channels_panel::ctrlClickLoadsChannelSelection` |
| `cp_ctrl_click_visible` | Hide Red and Ctrl-click the composite row | Coverage is the luminance of (0, 150, 200), 106 | `tst_channels_panel::ctrlClickLoadsChannelSelection` |
| `cp_ctrl_click_plain` | Click a row without Ctrl | The row moves and the selection is unchanged | `tst_channels_panel::clickChannelRowSelects` |
| `cp_ctrl_shift_add` | Ctrl+Shift-click a colour or alpha row with an existing selection | The row is added and one `Load Selection` state is recorded | `tst_channels_panel::ctrlShiftAdds` |
| `cp_ctrl_alt_subtract` | Ctrl+Alt-click a row with an existing selection | The row is subtracted and one state is recorded | `tst_channels_panel::ctrlAltSubtracts` |
| `cp_ctrl_shift_alt_intersect` | Ctrl+Shift+Alt-click a row with an existing selection | The selection is intersected and one state is recorded | `tst_channels_panel::ctrlShiftAltIntersects` |
| `cp_drag_load` | Drag a channel row onto the Load Selection button | The row's coverage replaces the selection and one state is recorded | `tst_channels_panel::dragLoadsChannel` |
| `cp_alt_save_dialog` | Alt-click the Save Selection button | The Save Selection dialog opens; nothing is created until accepted | `tst_channels_panel::altClickSaveOpensDialog` |
| `cp_alpha_name` | Give an extra channel a stored name and refresh the panel | The row shows the stored name while the Ctrl+number label stays positional | `tst_channels_panel::alphaRowShowsStoredName` |
| `cp_modifier_one_state` | Modifier-click a row | History gains exactly one state | `tst_channels_panel::modifierClickRecordsOneState` |
| `cp_alpha_eye_shows` | Click an alpha eye | The red overlay appears over the plane-black area and the eye is checked | `tst_channels_panel::alphaEyeShowsChannel` |
| `cp_alpha_eye_toggles` | Click the same alpha eye again | The overlay is removed and the eye is unchecked | `tst_channels_panel::alphaEyeShowsChannel` |
| `cp_alpha_multiple` | Show two alpha eyes | Both are checked and the red deepens where both are masked | `tst_channels_panel::alphaEyesShowMultiple` |
| `cp_alpha_composite_restores` | Show alpha channels, then click the composite eye | The composite returns and every alpha view clears | `tst_channels_panel::alphaEyeShowsChannel` |
| `cp_alpha_delete_clears` | Delete a shown alpha channel | It leaves the view and the remaining shown channels stay visible | `tst_channels_panel::deletingShownAlphaClearsView` |
| `cp_alpha_view_display_only` | Show alpha channels on the canvas | Channel pixels, selection, and history are unchanged | `tst_channels_panel::alphaEyeShowsChannel` |
| `cp_alpha_eye_selects` | Click an alpha eye | Its row becomes the current Channels row | `tst_channels_panel::clickAlphaEyeSelectsRow` |

## Manual verification

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Photoshop kind labels | In Photoshop CC or CS6, create an alpha channel in each `Color Indicates` mode, save as PSD, and inspect the DisplayInfo kind; load the channel as a selection | Record the kind byte and plane orientation for each mode. The planning mapping is `Masked Areas` = kind 1 with white = selected and `Selected Areas` = kind 0 with the plane inverted; if the observed bytes differ, swap the two mode constants and update the spec, design, and this row before implementation | manual: needs Photoshop; psd-tools exposes the kind byte but not the Photoshop UI label |
