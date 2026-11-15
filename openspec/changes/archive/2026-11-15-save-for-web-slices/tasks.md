# Tasks

## 1. Test scaffolding

- [x] 1.1 Write `test-plan.md` with one row per scenario and verify every coverage ref names a test in the tree or is created below.
- [x] 1.2 Add the new Rust unit-test stubs in `crates/pictura-codec` and `crates/pictura-render` for `locked_palette_survives_reduction`, `locked_palette_over_budget`, `locked_palette_reserves_the_clear_slot`, `locked_palette_full_drops_transparency`, `custom_palette_maps`, `custom_clear_entry_does_not_catch_opaque_white`, `act_roundtrip`, `act_invalid_count_falls_back_to_the_full_table`, `palette_edit_changes_one_entry`, `palette_delete_remaps_nearest`, `palette_map_transparency_moves_entry`, `metadata_levels`, `none_keeps_exif_copyright`, `png_xmp_chunk`, `html_table_grid`, `html_css_grid`, `css_id_prefixes_a_leading_digit`, `html_escaping`, and `overlapping_slices_export_disjoint`; verify `cargo test -p pictura-codec -p pictura-render --no-run` compiles.
- [x] 1.3 Add the new Qt Test slots to `crates/pictura-app/cpp/tests/tst_save_for_web.cpp` for `dialogSurfaces`, `fourUp`, `repopulate`, `colorTableEdit`, `colorTableTransparency`, `colorTableUnmap`, `colorBudget`, `colorTableSortAct`, `metadataAllNone`, `metadataFormats`, `metadataRemember`, `sliceImages`, `sliceImageSize`, `htmlTable`, `htmlOnly`, `cssLayout`, `saveFormats`, `saveCancel`, and `unslicedSave`, plus the `addSwatchForTest` hook for the colour budget; verify `cmake --build build --target tst_save_for_web` builds and the new slots fail.
- [x] 1.4 Verify `python3 scripts/check-test-plan.py` resolves every plan row to a named test.

## 2. Slice HTML/CSS emitter

- [x] 2.1 Add `crates/pictura-codec/src/web/html.rs` with `SliceCell`, `SliceLayout`, and `slice_html`; implement the table grid (`rowspan`/`colspan`), the CSS layout with a relative wrapper, and the digit-leading `s_` id prefix; verify `cargo test -p pictura-codec html_` and `css_id_prefixes_a_leading_digit` pass.
- [x] 2.2 Export the module from `crates/pictura-codec/src/web/mod.rs`; verify `cargo clippy -p pictura-codec --all-targets -- -D warnings` is clean.
- [x] 2.3 Confirm the golden strings cover the 2x2 grid, the L-shaped arrangement of rectangular slices, the PS-shaped head/body attributes, escaped text and paths, and the image paths; verify the named tests pass.

## 3. Palettes and `.act`

- [x] 3.1 Add `crates/pictura-codec/src/web/act.rs` with 768/772 parsing and writing: 772 with the big-endian count and transparent index for short or transparent tables, 768 only for a full 256-entry opaque table, and an invalid 772 trailer falling back to the first 768 bytes; make `act_roundtrip` cover a 16-colour opaque table and a 16-colour table with transparency and `act_invalid_count_falls_back_to_the_full_table` cover the fallback; verify `cargo test -p pictura-codec act_` passes.
- [x] 3.2 Add `quantize_locked` and `quantize_custom` to `web/quantize.rs`, including the `max(Colors, locked.len() + clear)` budget capped at 256 (a full locked table drops transparency, not a colour) and the custom-table `mapped_transparency` mapping; verify `cargo test -p pictura-codec locked_palette_survives_reduction`, `locked_palette_over_budget`, `locked_palette_reserves_the_clear_slot`, `locked_palette_full_drops_transparency`, `custom_palette_maps`, and `custom_clear_entry_does_not_catch_opaque_white` pass.
- [x] 3.3 Add palette operations (sort, remap indices, map/unmap transparency, delete-to-nearest) with unit coverage `palette_edit_changes_one_entry`, `palette_delete_remaps_nearest`, and `palette_map_transparency_moves_entry`; verify `cargo test -p pictura-codec web::` and clippy pass.

## 4. Metadata levels and PNG XMP

- [x] 4.1 Add `sfw_metadata(raw, level)` with the five levels and the minimal EXIF copyright for None; verify `cargo test -p pictura-codec metadata_levels` and `none_keeps_exif_copyright` pass.
- [x] 4.2 Add `png::attach_xmp` (iTXt `XML:com.adobe.xmp` before IEND, existing CRC); verify `cargo test -p pictura-codec png_xmp_chunk` passes and `decode_rgba` still reads the file.
- [x] 4.3 Verify `cargo clippy -p pictura-codec --all-targets -- -D warnings`.

## 5. Bridge

- [x] 5.1 Add `WebSliceCell`, `web_slice_html`, and the `resolve_export_slices` bridge to `crates/pictura-app/src/cxxqt_object/web_export.rs`; verify `cargo clippy -p pictura-app --all-targets -- -D warnings`.
- [x] 5.2 Add the `.act` pack/unpack bridge, the locked/custom quantize entry points, and `WebMetadata` with `web_metadata(view)` (raw, no level) and `web_embed_metadata(bytes, suffix, raw, level)`; `encodeWebImage` stays metadata-free; verify the crate builds.
- [x] 5.3 Verify the cxx bridge codegen compiles by building `tst_save_for_web` (`cmake --build build --target tst_save_for_web`).

## 6. Settings extraction

- [x] 6.1 Add `save_for_web_settings.{h,cpp}` with `WebSettings`, widget read/write, presets, remembered state, and `encodeWebImage`; register it in `cmake/sources/documents.cmake`; verify the build reconstitutes.
- [x] 6.2 Refactor `save_for_web_dialog.{h,cpp}` to hold `WebSettings panes_[4]` and call `encodeWebImage`; verify `tst_save_for_web::formats` and `sizePresetsSaveAndDone` still pass.
- [x] 6.3 Implement `tst_save_for_web::dialogSurfaces` and verify `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_save_for_web$' --output-on-failure`.

## 7. 4-Up

- [x] 7.1 Implement the four preview panes, mouse selection, porthole annotations, and Save-from-active-pane; verify `tst_save_for_web::fourUp` passes.
- [x] 7.2 Implement Repopulate Views with the deterministic lower-budget variants; verify `tst_save_for_web::repopulate` passes.
- [x] 7.3 Verify the full `tst_save_for_web` suite still passes.

## 8. Colour table

- [x] 8.1 Add `save_for_web_color_table.{h,cpp}` with swatch display, selection, edit, add-locked, delete, sort, map/unmap transparency, and `.act` load/save; register it in `cmake/sources/documents.cmake`.
- [x] 8.2 Wire the table to `WebSettings` and the locked/custom quantize paths; verify `tst_save_for_web::colorTableEdit`, `colorTableTransparency`, `colorTableUnmap`, and `colorTableSortAct` pass.
- [x] 8.3 Verify the full `tst_save_for_web` suite still passes.

## 9. Metadata menu

- [x] 9.1 Add the dialog-level Metadata combo (one remembered choice, not per pane), fetch raw metadata once at construction, and filter/embed it at save through `web_embed_metadata`; verify `tst_save_for_web::metadataAllNone`, `metadataFormats`, and `metadataRemember` pass.
- [x] 9.2 Verify `tst_save_for_web` still passes and the existing GIF/PNG/JPEG/WBMP assertions are unchanged.

## 10. Slice export and Save Optimized As

- [x] 10.1 Add `save_optimized_dialog.{h,cpp}` with file name, Format, All Slices, and Table/CSS; register it in `cmake/sources/documents.cmake`; verify `tst_save_for_web::saveCancel` passes.
- [x] 10.2 Implement the slice plan: `resolve_export_slices` (overlaps clipped in `Document::slices` order, auto slices filling the rest), the user-slice guard, edge scaling, zero-area omission, crop, `encodeWebImage`, `<base>_NN.<ext>` names, and `images/` writes; verify `cargo test -p pictura-render overlapping_slices_export_disjoint` and `tst_save_for_web::sliceImages` and `sliceImageSize` pass.
- [x] 10.3 Generate HTML through `web_slice_html` for both layouts and all three formats; verify `tst_save_for_web::htmlTable`, `htmlOnly`, `cssLayout`, and `saveFormats` pass.
- [x] 10.4 Keep the unsliced single-image path and verify `tst_save_for_web::unslicedSave` passes.

## 11. Documentation

- [x] 11.1 Tick the Save for Web slice item in `ROADMAP.md` and verify the remaining `[ ]` items still link their issues.
- [x] 11.2 Update WF-5 in `docs/dev/doc-code-parity-gaps.md` with the shipped behavior and the remaining gaps (#543, `.aco`, Web Snap, saved settings); verify the table renders.
- [x] 11.3 Align `docs/10-workflow-io/web-export-and-slices.md` with the shipped Metadata levels, Save Optimized As formats, and Table/CSS layout; verify `openspec validate --all --strict`.
- [x] 11.4 Note the milestone in `docs/dev/STATE.md`; commit the docs changes separately with `TASK-ALLOWS-DOCS` after verifying `bash scripts/verify-fast.sh`.

## 12. Integration and archive

- [x] 12.1 Run `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`; fix any findings.
- [x] 12.2 Run `cargo nextest run -p pictura-codec -p pictura-render -p pictura-app`; fix any failures.
- [x] 12.3 Run `QT_QPA_PLATFORM=offscreen ctest --test-dir build -R '^tst_save_for_web$' --output-on-failure`.
- [x] 12.4 Run `bash scripts/verify-fast.sh` and confirm every gate passes.
- [x] 12.5 Run `openspec archive save-for-web-slices` and `bash scripts/check-openspec-archived.sh`; verify only `archive/` remains under `openspec/changes/`.
