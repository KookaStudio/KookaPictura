# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ter_commit` | Change size, family, colour and a paragraph option of a small type layer | Region inside old and new text bounds; one state | `tst_type_edit_damage::styleChangesStayInsideTheText` |
| `ter_moved` | Centre a type layer | Document equals a fresh document with that layer | `tst_type_edit_damage::alignmentMatchesAFreshDocument` |
| `ter_undo` | Resize, align, undo, redo | Each step shows its earlier pixels | `tst_type_edit_damage::undoAndRedoRestoreThePixels` |
| `ter_knockout` | Change a layer under a knockout | Own reach; region equals the full composite | `pictura-render::a_content_edit_under_a_knockout_stays_bounded_and_exact` |
| `ter_box` | Recolour words in a huge box | Region is the words | `tst_type_edit_damage::aHugeBoxRefreshesItsInk`, `pictura-render::a_nearly_canvas_sized_type_box_trims_to_its_ink`, `pictura-render::channels_hold_every_inked_row` |
| `ter_box_effect` | Add an effect to type in a huge box | No trim | `pictura-render::a_nearly_canvas_sized_type_box_trims_to_its_ink` |
| `ter_undo_region` | Resize, undo, redo | Region around the text; pixels restored | `tst_type_edit_damage::undoAcrossAResizeStaysInsideTheText` |
| `ter_crossing_region` | Cross a segment with one changed tile | That tile's region | `pictura_app::crossing_a_segment_reports_the_changed_tile` |
| `ter_crossing_full` | Cross a segment with a changed document channel | Whole document | `pictura_app::crossing_a_segment_with_a_changed_channel_reports_everything` |
| `ter_warp_preview` | Preview a warp, cancel | Region; nothing recorded; pixels restored | `tst_type_edit_damage::warpPreviewIsBoundedAndCancelRestores` |
| `ter_warp_commit` | Preview a strong warp, commit a slight one, undo | Original pixels | `tst_type_edit_damage::warpCommitUndoesPastEveryPreview` |
| `ter_style_preview` | Preview an applied paragraph style | Region inside the text; history unmoved | `tst_type_edit_damage::stylePreviewStaysInsideTheText` |
| `ter_type_op` | Drive `type_op` | Each call succeeds; unknown refused | `tst_control_metrics::typeOpAddsAndResetsAType` |
| `ter_preview` | Preview another colour | Region inside the text; one change; history unmoved | `tst_type_edit_damage::previewIsBoundedAndUnrecorded` |
| `ter_session` | Open a session, cancel it | Text-sized regions; pixels restored | `tst_type_edit_damage::cancelledSessionRestoresThePixels` |
| `ter_session_commit` | Open a session, retype, commit | Document equals a fresh document with that layer | `tst_type_edit_damage::committedSessionMatchesAFreshDocument` |
| `ter_display_owner` | Preview while a mask is soloed | No region; a whole refresh | `tst_type_edit_damage::soloedMaskKeepsTheFullPath` |
| Feel on the 16507×16196 world map | Add text, change font, size, alignment, colour and paragraph options, drag Warp Text sliders, undo | The text updates at once | manual: perceived lag needs a real display; `scripts/profile-tools.py` records the timings |
