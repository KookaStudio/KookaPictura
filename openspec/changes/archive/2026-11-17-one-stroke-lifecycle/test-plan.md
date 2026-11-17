# Test Plan: one stroke lifecycle

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| `osl_layer_begin` | Call the one constructor with a layer plan; inspect the installed session. | One session with the supplied label, pad, fade capture, empty pending present, reset tiles, and one recording begin. | `pictura-app::one_constructor_installs_a_layer_stroke` |
| `osl_mask_begin` | Call the one constructor with a mask plan; inspect the installed session. | One mask session with the mask pad, no fade capture, empty pending present, reset tiles, and one recording begin. | `pictura-app::one_constructor_installs_a_mask_stroke` |
| `osl_refused` | Begin a stroke while another session owns the slot, then inspect the live slot and pending state. | The begin returns false and the existing session, pending present, tiles, and recording are unchanged. | `pictura-app::a_refused_stroke_begin_leaves_the_live_state_untouched` |
| `osl_layer_commit` | Paint and commit a layer stroke through the existing Qt path. | Exactly one history state with the stroke label; the pending region is committed and the tile hint is preserved. | `tst_tool_commit::aStrokeCommitsOnlyWhatIsPending`, `pictura-app::paint_commit_hints_all_stroke_tiles` |
| `osl_mask_commit` | Paint and commit a mask stroke through the existing Qt path. | The mask coverage is written back and exactly one history state is recorded; the composite rule is unchanged. | `tst_mask_edit::paintAndFillWriteCoverage`, `tst_tool_commit::aSelectionClipsAMaskStroke` |
| `osl_empty` | End a stroke that changed no pixels. | No history state is added; a structural base or preview state is restored as before. | `tst_tool_commit::aCancelledStrokePutsTheCompositeBack`, `tst_brush_airbrush::airbrushOffStaysStill` |
| `osl_preview_replay` | Run a large-brush layer and mask preview to commit. | The exact stroke replays the preview log and the committed pixels match the full-resolution stroke. | `tst_brush_airbrush::previewHoldReachesTheCommit`, `tst_brush_airbrush::maskPreviewHoldReachesTheCommit`, `tst_mask_edit::previewOnlyPixelsRestoreAtCommit` |
