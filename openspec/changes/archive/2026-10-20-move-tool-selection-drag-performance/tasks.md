# Tasks

## 1. Tests first

- [x] 1.1 Add `pictura_select::bounds_matches_a_full_scan_with_padding` and `pictura_select::translated_bounds_clip_at_the_canvas` (new `bounds` module) and confirm they fail before the implementation
- [x] 1.2 Add `pictura_render::bounded_mask_move_equals_the_full_plane_move` (fixed-seed randomized equivalence against a test-only reference of the old full-plane loop) and `pictura_render::bounded_mask_move_leaves_outside_pixels_untouched`
- [x] 1.3 Add `pictura_app::preview_keeps_the_origin_plane`, `pictura_app::coverage_follows_the_drag_offset`, and `pictura_app::commit_materialises_the_translated_plane` on `PictureViewRust`
- [x] 1.4 Add `pictura_app::content_move_region_equals_a_full_recomposite` and `pictura_app::content_move_region_keeps_outside_pixels` in `canvas_view_test.rs`
- [x] 1.5 Add `tst_info_panel::cursorMoveDoesNotRestatTheDocument` (open a temp image, remove the file, move the cursor, assert the Doc line keeps its size)

## 2. Dev-profile optimisation

- [x] 2.1 Add `[profile.dev.package.*] opt-level = 2` for the nine engine crates in `Cargo.toml`, mirroring `[profile.test.package.*]`
- [x] 2.2 Record the cold-build cost of the override in the task result (engine crates at opt-level 2: ~2 min extra on a cold dev build, 4-core machine, 3 jobs)

## 3. Bounds and drag offset

- [x] 3.1 Add `Selection::bounds() -> Option<PsdRect>` (row-based, `position`/`rposition`) in `crates/pictura-select` and cover the empty, padded, soft-edged, and single-pixel cases
- [x] 3.2 Move the drag state machine onto `PictureViewRust`: `selection_move_begin`, `selection_move_preview`, `selection_move_commit`, `selection_move_cancel`, `selection_bounds`, `selection_coverage_at`
- [x] 3.3 Make `selection_bounds_extents` use `selection_bounds()`; make `selection_coverage` read through the drag offset; keep `preview_selection_move` from touching the plane; materialise once in `commit_selection_move`
- [x] 3.4 Clear the drag offset in `move_selection_content` and `cancel_selection_move`

## 4. Bounded engine moves

- [x] 4.1 Bound `move_mask_selection_content` to `coverage_bounds` with row-sliced source clear and destination composite
- [x] 4.2 Make `crop_selection` row-sliced (`copy_from_slice` per clipped row)
- [x] 4.3 Make `clear_layer` row-sliced and bounded to the selection's rows

## 5. Regional commit

- [x] 5.1 Add `PictureView::recomposite_region(rect)` (region composite + `apply_refreshed_region` + `changed`)
- [x] 5.2 Use the union of the selection's source and destination bounds for the content-move commit on both the mask and layer paths, falling back to the full recomposite when the union is the whole canvas

## 6. Info panel

- [x] 6.1 Split `InfoPanel::refresh` into a cursor-only path (`refresh(false)`: position, colour, selection size) and the full path (`refresh(true)`: document size, samplers); call `refresh(false)` from `setCursorPosition`

## 7. Verification

- [x] 7.1 Re-run `selection_move_perf` in both profiles and record the before/after table in the task result (dev profile, 4000²/2000²: bounds 334 → 7 ms, crop 359 → 1.6 ms, mask move 727 → 38 ms, contour 4.2 → 1.1 ms)
- [x] 7.2 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`
- [x] 7.3 `cmake --build build` and `ctest --test-dir build -R '^tst_(mask_edit|selection_move|info_panel)$' --output-on-failure` (all three pass; two unrelated environment failures remain in a Debug build: `tst_keyboard_shortcuts` hits a live Qt assertion on a non-printable shortcut key, and the flaky `float_screen_bounds` self-test finds no clamp band offscreen)
- [x] 7.4 `bash scripts/verify-fast.sh` (green up to the two environment failures above) and `openspec validate --all --strict` (279 passed)
- [x] 7.5 Archive the change with `openspec archive move-tool-selection-drag-performance`
