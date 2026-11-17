# Test Plan

This plan covers `tools/tiled-selection-readers` for `tiled-selection-readers`
(#1268).

**Fixtures.** Each parity test builds a seeded soft selection that crosses
tile edges, with some uniform tiles, on a document whose size is not a
multiple of 256. It holds the selection twice: tiled, and flat (the pre-change
dense path, since a flat plane derefs for free). The ported call runs on the
tiled copy inside `assert_no_materialization`, and its result must equal the
flat run byte for byte. "No materialization" checks run under nextest, one
process per test.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `tsr_clipboard` | Copy, Copy Merged (region), Clear and Paste Into/Outside a seeded layer through the tiled selection, and through the flat copy | No materialization; clips, layers and masks equal | `pictura-render::clipboard_reads_a_tiled_selection` |
| `tsr_layer_ops` | Lift, Layer via Copy, Layer via Cut and a mask selection move with the tiled and the flat selection | No materialization; documents equal | `pictura-render::layer_ops_read_a_tiled_selection` |
| `tsr_healing` | Patch (source, destination, transparent) and Content-Aware Move with the tiled and the flat selection | No selection materialization; layers equal | `pictura-paint::healing_reads_a_tiled_selection` |
| `tsr_filter` | Apply Invert and Auto Tone to a seeded layer through a tiled soft selection mask and through the flat copy | No materialization; layers equal | `pictura-render::selection_filters_read_a_tiled_selection` |
| `tsr_select` | Fit a seeded tiled selection into a larger and a smaller canvas, and the same selection held flat | No materialization; results tiled and equal | `pictura-select::fit_centered_reads_a_tiled_selection` |
| `tsr_parity` | Run the existing clipboard, mask-move, healing, filter and Auto tests | They pass with their assertions unchanged | `pictura-render::copy_keeps_only_the_selection_bounding_box`, `pictura-render::paste_into_and_outside_mask_by_the_selection`, `pictura-render::cut_then_paste_in_place_restores_the_pixels`, `pictura-render::move_mask_selection_blends_partial_coverage`, `pictura-render::mask_confines_change_to_left_half`, `pictura-render::auto_tone_uses_selection_histogram`, `pictura-paint::patch_in_source_mode_repairs_the_selection` |
| `tsr_exports` | Export the live selection's coverage and an alpha channel's coverage (no materialization), and a colour row's coverage, from a view holding a tiled selection | Bytes equal the source; the row covers the document; selection and channel still tiled | `pictura-app::coverage_exports_leave_no_mirror` |
| `tsr_writers` | Drag a tiled selection by its outline and commit (the selection-move tests' fixture now writes through `write_rect_from`) | Still tiled; no materialization; bytes equal the dense shift | `pictura-app::selection_move_writes_stay_tiled` |
| `tsr_guard` | Under nextest, on a view with a tiled selection: Copy, Clear, Lift, a filter through the selection, Patch, a coverage export, an in-place move | Materialization counter unchanged | `pictura-app::selection_readers_do_not_materialize` |
