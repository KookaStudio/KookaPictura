# Test Plan

Every scenario in the change's spec deltas maps to an automated test. No manual rows.

## tools/select-menu

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Save to a new channel stores the name | Save a rectangle named "Hair" to a new channel | One channel holds the coverage and is listed as "Hair" | `tst_selection_save_load::saveDialogNewChannelName` |
| Save combines with an existing channel | Save a soft selection into an existing channel with each operation | Coverage becomes incoming, max, saturating difference, rounded product | `tst_selection_save_load::saveDialogExistingChannelOps` |
| Save into a layer mask reveals the selection | Save a rectangle into an existing raster mask with Replace | Selected pixels white, unselected black, metadata unchanged | `tst_mask_edit::saveSelectionIntoMaskReplaces` |
| Save to a new image | Choose the New document in Save | A same-size grayscale document opens with the selection as its extra channel; it saves and reopens as a PSD with one named channel | `tst_selection_save_load::saveDialogNewImage` |
| The document list is same-size only | Open two documents with different sizes | The other document is absent from both lists | `tst_selection_save_load::documentListSameSize` |
| Load inverts before combining | Load coverage 64 inverted with Add into coverage 128 | Result is 191 | `tst_selection_save_load::loadDialogInvertAndOps` |
| Load from another document | Load a channel from another same-size document | Active selection becomes the source and records one state | `tst_selection_save_load::loadDialogCrossDocument` |
| Load lists existing channels | Open a document with saved selections, invoke Load | Dialog lists every source by name and loads the chosen one | `tst_selection_save_load::loadDialogDefaults` |
| Load is disabled with no channels | Open a document with no channels or masks | Load is disabled and changes nothing | `tst_selection_save_load::loadDialogDisabledWithoutSources` |
| The 56-channel limit refuses a new channel | Save when 56 channels exist | No channel added and no state recorded | `tst_selection_save_load::channelLimitRefusal` |
| Wrong-length channel is refused | Load a channel with a short plane | Command refused, history unchanged | `tst_selection_save_load::wrongLengthRefused`; `pictura-app::wrong_length_source_refused` |
| Round trip is exact | Save then load a soft selection | Byte-identical coverage, one undo state | `pictura-app::save_then_load_round_trip` |

## tools/selection-channels

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Replace overwrites the channel | Save with Replace into an existing channel | Channel equals the incoming coverage | `pictura-app::save_selection_replaces_existing_channel` |
| Boolean operations combine soft coverage | Existing 128, incoming 64, each operation | Add 128, Subtract 64, Intersect 32 | `pictura-app::save_selection_combines_existing_channel` |
| Saving into a channel is one state | Save into an existing channel, then undo | One state; undo restores the previous bytes | `pictura-app::save_selection_into_channel_one_state` |
| Invert is applied before the combine | Coverage 64 inverted with Add into 128 | Result 191 | `pictura-app::load_selection_inverts_before_combining` |
| No selection with each operation | Load with no current selection | New/Add load the source; Subtract/Intersect are empty | `pictura-app::load_selection_without_existing_uses_op` |
| Load is one state | Load with Add/Subtract/Intersect | One "Load Selection" state | `pictura-app::load_selection_one_state` |
| ASCII and non-ASCII names round-trip | Write and read "Hair" and "Ciel étoilé" | Both names restored in order | `pictura-codec::alpha_names_round_trip_ascii_and_unicode` |
| Missing names fall back | Open a channel with no stored name | Panel and dialogs show `Alpha N` | `pictura-app::channel_name_falls_back_to_alpha_n`; `tst_channels_panel::alphaRowShowsStoredName` |
| Deleting a channel keeps later names aligned | Delete the second of three named channels | First and third keep their names | `pictura-app::channel_name_positions_follow_add_and_delete` |
| Unedited names preserve the resource bytes | Save without changing names | 1006/1045 bytes unchanged | `pictura-codec::alpha_names_unchanged_keeps_bytes` |
| psd-tools reports the names | Open the written PSD with psd-tools | Same names in order | `pictura-codec::psd_tools_reports_alpha_names` |

## ui/channels-panel

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| cp_ctrl_click_channel | Ctrl-click Red, Green, Blue, composite | Coverage 100, 150, 200, 140 | `tst_channels_panel::ctrlClickLoadsChannelSelection` |
| cp_ctrl_click_visible | Hide Red, Ctrl-click composite | Coverage 106 | `tst_channels_panel::ctrlClickLoadsChannelSelection` |
| cp_ctrl_click_plain | Click a row without Ctrl | Row moves, selection unchanged | `tst_channels_panel::ctrlClickLoadsChannelSelection` |
| cp_ctrl_shift_add | Ctrl+Shift-click a row with a selection | Add and one state | `tst_channels_panel::ctrlShiftAdds` |
| cp_ctrl_alt_subtract | Ctrl+Alt-click a row with a selection | Subtract and one state | `tst_channels_panel::ctrlAltSubtracts` |
| cp_ctrl_shift_alt_intersect | Ctrl+Shift+Alt-click a row with a selection | Intersect and one state | `tst_channels_panel::ctrlShiftAltIntersects` |
| cp_drag_load | Drag a row onto the Load button | New selection, one state | `tst_channels_panel::dragLoadsChannel` |
| cp_alt_save_dialog | Alt-click the Save button | Dialog opens, no channel yet | `tst_channels_panel::altClickSaveOpensDialog` |
| cp_alpha_name | Save a channel named "Hair" | Row shows "Hair", shortcut stays positional | `tst_channels_panel::alphaRowShowsStoredName` |
| cp_modifier_one_state | Apply each modifier click | History grows by one each time | `tst_channels_panel::modifierClickRecordsOneState` |

## compositing/layer-masks

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Replace reveals the selected area | Save a rectangle into an existing mask | Selected white, unselected black | `tst_mask_edit::saveSelectionIntoMaskReplaces` |
| Boolean operations combine with the mask | Save with Add/Subtract/Intersect | Mask combines with incoming coverage | `tst_mask_edit::saveSelectionIntoMaskCombines` |
| Mask metadata is preserved | Save into a mask with density, feather, flags | Settings unchanged | `tst_mask_edit::saveSelectionIntoMaskPreservesMetadata` |
| A disabled mask stays disabled | Save into a disabled mask | Coverage changes, mask stays disabled | `tst_mask_edit::saveSelectionIntoDisabledMask` |
| Saving into a mask is one state | Save into a mask, then undo | One state; restore bytes and metadata | `tst_mask_edit::saveSelectionIntoMaskOneState` |
| A missing mask target is refused | Save to a stale path | Refused, no state | `tst_mask_edit::saveSelectionIntoMissingMaskRefuses` |
| A non-active row's mask becomes the target | Click another row's mask thumbnail | Row active, mask edit target | `tst_mask_edit::maskThumbnailOnNonActiveRowKeepsTarget` |
| Ctrl-click loads the mask selection | Ctrl-click a mask thumbnail | White coverage selected, row membership unchanged | `tst_mask_edit::ctrlClickMaskLoadsSelection` |
| Ctrl+Shift adds the mask coverage | Ctrl+Shift-click with a selection | Add and one state | `tst_mask_edit::ctrlShiftAddsMask` |
| Ctrl+Alt subtracts the mask coverage | Ctrl+Alt-click with a selection | Subtract and one state | `tst_mask_edit::ctrlAltSubtractsMask` |
| Ctrl+Shift+Alt intersects the mask coverage | Ctrl+Shift+Alt-click with a selection | Intersect and one state | `tst_mask_edit::ctrlShiftAltIntersectsMask` |
| Ctrl-I inverts a fill mask | Press Ctrl-I on a fill mask target | One `Invert` state, fill pixels untouched | `tst_mask_edit::invertOnMaskTarget` |
