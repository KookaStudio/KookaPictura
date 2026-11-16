# Test plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `cp_duplicate_menu` | Open the live Channels panel menu with an alpha row current; invoke `Duplicate Channel…`. | The dialog lists same-size documents and `New`; the name defaults to `<source> copy`; the active document is selected. | `tst_channels_panel::duplicateChannelMenu` |
| `cp_duplicate_color_row` | Select the Red row of a white RGB document and accept Duplicate Channel. | The new extra channel holds the Red row's grayscale plane (255 on white). | `tst_channels_panel::duplicateChannelColorRow` |
| `cp_duplicate_same_doc` | Accept the dialog with the source document, a name, and no invert. | A new channel with that name holds the source coverage; exactly one `Duplicate Channel` state is added. | `tst_channels_panel::duplicateChannelSameDocument` |
| `cp_duplicate_cross_doc` | Create two same-size documents; accept the dialog with the other document. | The target gains the named channel and one state; the source channel count and history are unchanged. | `tst_channels_panel::duplicateChannelCrossDocument` |
| `cp_duplicate_new_doc` | Accept the dialog with `New`. | A grayscale document opens with the coverage as its named extra channel. | `tst_channels_panel::duplicateChannelNewDocument` |
| `cp_duplicate_invert` | Duplicate a channel holding 64 with Invert checked. | The new channel holds 191. | `tst_channels_panel::duplicateChannelInvert` |
| `cp_duplicate_button_drag` | Drag an alpha row and drop it on the Create New Channel button. | The document gains one named copy and one `Duplicate Channel` state. | `tst_channels_panel::duplicateChannelOntoNewButton` |
| `cp_duplicate_document_drag` | Drag an alpha row onto another document's tab. | The target gains one named copy and one state; the source is unchanged; a different target size fits and centres the coverage. | `tst_channels_panel::dragChannelOntoAnotherDocument`, `tst_channels_panel::dragChannelToDifferentSizeFits` |
| `cp_duplicate_cap` | Fill a target to 56 channels and duplicate into it. | No channel is added and no history state is recorded. | `tst_channels_panel::duplicateChannelAtCapRefused` |
| `cp_channel_drag_same_size` | Drag an alpha row onto a same-size document. | The target channel bytes equal the source row bytes. | `tst_channels_panel::dragChannelOntoAnotherDocument` |
| `cp_channel_drag_fit` | Drag a centred 8×8 alpha block from 16×16 onto 32×32. | The target holds a 16×16 block at (8, 8); the border is empty. | `tst_channels_panel::dragChannelToDifferentSizeFits` |
| `cp_channel_drag_source` | Compare the source before and after a cross-document channel drag. | Source channel count, bytes, and history are unchanged. | `tst_channels_panel::dragChannelOntoAnotherDocument` |
| `sc_fit_same_size` | Fit a selection to its own dimensions. | Every byte equals the source. | `pictura-select::fit_centered_same_size_is_identity` |
| `sc_fit_letterbox` | Fit an all-selected 8×4 selection to 8×8. | Rows 2–5 are 255; rows 0–1 and 6–7 are 0. | `pictura-select::fit_centered_letterboxes_a_wide_source` |
| `sc_fit_upscale` | Fit an all-selected 2×2 selection to 4×4. | Every byte is 255. | `pictura-select::fit_centered_upscales_a_small_source` |
| `sc_drag_same_size` | Drag a selection outline onto a same-size document's tab. | Target selection bytes equal the source; target history gains one `Load Selection` state. | `tst_selection_save_load::selectionDragSameSize` |
| `sc_drag_different_size` | Drag a centred selection from 16×16 onto 32×32. | The target selection is the fitted, centred coverage. | `tst_selection_save_load::selectionDragDifferentSizeFits` |
| `sc_drag_source_unchanged` | Compare the source selection and history before and after the drag. | Both are unchanged. | `tst_selection_save_load::selectionDragSourceUnchanged` |
| `sc_drag_same_doc_ignored` | Drop a selection drag on the source document's own tab. | The source selection and history are unchanged. | `tst_selection_save_load::selectionDragOntoSourceIgnored` |
| Drag source reaches the platform drag loop | Drag a selection outline from the canvas to another tab in a real window. | The platform drag starts, the source selection is restored, and the target receives the fitted selection. | manual: offscreen Qt cannot complete a platform drag loop; the preparation seam is covered by tst_selection_save_load::selectionDragPreparationRestoresSource and the drop path by the router tests |
