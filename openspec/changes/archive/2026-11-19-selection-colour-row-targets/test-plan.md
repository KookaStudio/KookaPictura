# Test Plan

Every scenario in the change's spec deltas maps to an automated test, except one justified manual row for a fixture limitation.

## tools/select-menu

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Save to a new channel stores the name | Save a rectangle named "Hair" to a new channel | One extra channel holds the coverage and is listed as "Hair" | `tst_selection_save_load::saveDialogNewChannelName` |
| Save combines with an existing channel | Save a soft selection into an existing channel with each operation | Coverage becomes incoming, max, saturating difference, rounded product | `tst_selection_save_load::saveDialogExistingChannelOps` |
| Save into a layer mask reveals the selection | Save a rectangle into an existing raster mask with Replace | Selected pixels white, unselected black, metadata unchanged | `tst_mask_edit::saveSelectionIntoMaskReplaces` |
| Save to a new image | Choose the New document in Save | A same-size grayscale document opens with the selection as its extra channel; it saves and reopens as a PSD | `tst_selection_save_load::saveDialogNewImage` |
| sm_save_excludes_colour | Open an RGB document with one extra channel and one raster mask; open Save Selection | List is New Channel, the extra channel, the mask; no RGB/Red/Green/Blue row | `tst_selection_save_load::saveDialogExcludesColourRows` |
| The document list is same-size only | Open two documents with different sizes | The other document is absent from both lists | `tst_selection_save_load::documentListSameSize` |
| Load inverts before combining | Load coverage 64 inverted with Add into coverage 128 | Result is 191 | `tst_selection_save_load::loadDialogInvertAndOps` |
| Load from another document | Load an extra channel from another same-size document | Active selection becomes the source and one state is recorded | `tst_selection_save_load::loadDialogCrossDocument` |
| Load lists existing channels | Open a document with saved selections, invoke Load | Dialog lists every extra channel by name and loads the chosen one | `tst_selection_save_load::loadDialogDefaults` |
| sm_load_lists_colour | Open an RGB document with one extra channel and one raster mask; open Load Selection | Rows read RGB, Red, Green, Blue, the mask, then the extra channel | `tst_selection_save_load::loadDialogListsColourRows`; `tst_selection_save_load::loadDialogModeRows` |
| sm_load_colour_plane | Select Red in the Load dialog for a flat (100, 150, 200) image, then load the encoded target | Request target is `color:1`; coverage is 100 per pixel (Green 150, Blue 200) | `tst_selection_save_load::loadDialogColourRowTarget`; `tst_selection_save_load::loadColourRowCoverage`; `tst_selection_save_load::loadColourRowThroughFrame`; `pictura-app::channel_row_selection_reads_the_row_plane` |
| sm_load_composite | Select RGB in the Load dialog for a flat (100, 150, 200) image, then load the encoded target | Request target is `color:0`; coverage is 140 per pixel | `tst_selection_save_load::loadDialogColourRowTarget`; `tst_selection_save_load::loadColourRowCoverage`; `pictura-app::channel_row_selection_reads_the_row_plane` |
| sm_load_no_saved | Open Load Selection for an RGB document with no extra channel and no mask | Four colour rows are listed, OK is enabled, and the Select command is enabled | `tst_selection_save_load::loadDialogWithNoSavedChannels`; `tst_selection_save_load::selectLoadEnabledWithoutChannels`; `tst_selection_save_load::loadDialogModeRows` |
| sm_load_single_row_mode | Open Load Selection for a Grayscale document with no saved channels and load the row | One `Gray` row is listed; loading it selects the image's luminance | `tst_selection_save_load::loadDialogGrayscaleCompositeRow` |
| sm_load_default | Open Load Selection for a document with an extra channel and a mask | The request targets the extra channel, not the composite row | `tst_selection_save_load::loadDialogDefaultsToExtraChannel` |
| sm_load_multichannel_default | Open Load Selection for a Multichannel document with no extra channel | The default target is the first colour row (Cyan), `color:0` | `manual:` no Qt fixture builds a Multichannel document (`convert_image_mode` accepts only Grayscale/RGB/CMYK/Lab and the codec writes no Multichannel); defaulting to `color:0` is the same code path as `tst_selection_save_load::loadDialogWithNoSavedChannels` |
| sm_load_colour_cross_doc | Derive a colour row from a flat source document and load it into a same-size active document | Target selection becomes that row's coverage and one state is recorded | `tst_selection_save_load::loadColourRowCrossDocument`; `tst_selection_save_load::loadColourRowThroughFrame` |
| The 56-channel limit refuses a new channel | Save when 56 channels exist | No channel added and no state recorded | `tst_selection_save_load::channelLimitRefusal` |
| Wrong-length channel is refused | Load a channel with a short plane | Command refused, history unchanged | `tst_selection_save_load::wrongLengthRefused`; `pictura-app::wrong_length_source_refused` |
| Round trip is exact | Save then load a soft selection | Byte-identical coverage, one undo state | `pictura-app::save_then_load_round_trip` |

## tools/selection-channels

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Invert is applied before the combine | Coverage 64 inverted with Add into 128 | Result 191 | `tst_selection_save_load::loadDialogInvertAndOps`; `pictura-app::load_selection_inverts_before_combining` |
| No selection with each operation | Load with no current selection | New/Add load the source; Subtract/Intersect are empty | `pictura-app::load_selection_without_existing_uses_op` |
| Load is one state | Load with Add/Subtract/Intersect | One "Load Selection" state | `pictura-app::load_selection_one_state` |
| sc_load_colour_plane | Derive the Red, Green, Blue rows of a flat (100, 150, 200) image and load them | Coverage is 100, 150, 200 per pixel | `tst_selection_save_load::loadColourRowCoverage`; `pictura-app::channel_row_selection_reads_the_row_plane` |
| sc_load_colour_composite | Derive the composite row of a flat (100, 150, 200) image and load it | Coverage is 140 per pixel | `tst_selection_save_load::loadColourRowCoverage`; `pictura-app::channel_row_selection_reads_the_row_plane` |
| sc_load_colour_combine | Load the Red row holding 100 with Subtract into a selection of 200 | Resulting coverage is 100 and one "Load Selection" state is recorded | `tst_selection_save_load::loadColourRowCombines` |
| sc_load_naive_mode_rows | Derive a CMYK or Multichannel plate row | Coverage equals the naive ink derivation the Channels panel uses for that row | `pictura-app::multichannel_rows_derive_naive_inks`; `pictura-app::gray_and_ink_match_the_panel_derivation` |
