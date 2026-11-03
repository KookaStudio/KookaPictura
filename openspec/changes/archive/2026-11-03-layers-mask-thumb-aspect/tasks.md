# Tasks

## 1. Split the mask thumbnail helpers

- [x] 1.1 Move `mask_thumbnail_image` and `vector_mask_thumbnail_image` from `helpers_composite.rs` to a new `cxxqt_object/helpers_masks.rs`, declare `mod helpers_masks;` in `cxxqt_object.rs` (the file is 1202/1204 in the size allowlist), and update `impl_layers.rs` plus the inline `bundle_tests` import; verify `cargo check -p pictura_app` and `bash scripts/check-file-size.sh` pass with `helpers_composite.rs` under 1200 lines.

## 2. Regression tests

- [x] 2.1 Add `tst_layers_snapshot::subCanvasLayerPlacement`: 20x10 document, `view->select_rect(6, 2, 4, 4, QStringLiteral("new"), 0.0)`, `pictura::clipboard_copy(*view, view->active_layer_path(), false)`, `pictura::clipboard_paste(*view, path, pictura::PasteKind::Plain, 0.0, 0.0)`, then `pictura::layer_mask_add_path(*view, pasted, QStringLiteral("reveal-all"))` (add the `clipboard.cxxqt.h` include). Assert `snapshot(*view, 24, true)` gives image and mask (24,12) with content at (7,2) and transparent (0,0), and `snapshot(*view, 24, false)` gives image and mask (24,24); verify the Entire Document image case passes today and the mask case fails today (mask is 24x24).
- [x] 2.2 Add `tst_layers_panel_fixes::maskThumbnailKeepsDocumentAspect`: 400x100 document, paste a 50x50 selection at (50,25), add a layer mask, refresh; assert `row.thumbnail` and `row.maskThumbnail` are (24,6), the mask image is transparent at (0,0) and opaque at (3,2), `maskThumbRect` is (24,6), and the painted margin of both cards at +1,+1 is the checker white (255). Keep the function within that file's 44-line headroom (1400-line cap); verify it fails on the current tree (mask thumbnail is 24x24 and no mask checker is painted).
- [x] 2.3 Add `mask_thumbnail_positioned(mask, doc_w, doc_h, size)` in `helpers_masks.rs` (document aspect, mask rect at its document offset clipped to the canvas, transparent outside, `None` on zero/absent/mismatched data) and put `positioned_mask_thumbnail_follows_document_aspect` in a `#[cfg(test)] mod tests` inside that file (not `cxxqt_object/tests.rs`, which is 1399/1400). Cover the document offset, the transparent margin, a mask wider than the canvas with a negative offset (clipped, no panic), and the zero-rect / missing-data refusals; verify `cargo nextest run -p pictura_app positioned_mask_thumbnail_follows_document_aspect` passes.

## 3. Wire the positioned mask thumbnail

- [x] 3.1 In `impl_layers.rs`, call `mask_thumbnail_positioned` when `entire_document` is true and keep `mask_thumbnail_image` otherwise; verify the Qt tests from 2.1 and 2.2 pass.
- [x] 3.2 Draw `checkerTile()` under the mask card in `layers_panel_internal.h` before `drawImageFit`; verify 2.2's painted-margin assertions pass.

## 4. Verification

- [ ] 4.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo nextest run -p pictura_app`; confirm all pass.
- [ ] 4.2 Configure and build with CMake, then run `ctest --test-dir build -R '^tst_layers_(panel_fixes|snapshot)' --output-on-failure`; confirm both suites pass.
- [ ] 4.3 Run `openspec validate --all --strict` and `bash scripts/verify-fast.sh`; confirm both pass, including the file-size gate (`helpers_composite.rs` under 1200, `tst_layers_panel_fixes.cpp` under 1400, `cxxqt_object.rs` at or under 1204).

## 5. Ship

- [x] 5.1 Archive the change with `openspec archive layers-mask-thumb-aspect`, commit `fix(layers): place raster mask thumbnails in document space (#1102)`, and open the PR titled `fix(layers): raster mask thumbnails use the canvas aspect (#1102)` with the manual rows from `test-plan.md` (none expected).
