# Tasks

## 1. The current-picture accessor

- [x] 1.1 Add `PictureViewRust::is_painting` (a paint or mask stroke is live) next to the session accessors, and make the QObject's `is_painting` delegate to it; verify `cargo nextest run -p pictura_app paint`, `cargo nextest run -p pictura_app mask`, and `ctest --test-dir build -R '^tst_(document_session|mask_edit)$' --output-on-failure` pass unchanged.
- [x] 1.2 Add `crates/pictura-app/src/cxxqt_object/picture.rs` with `picture<'a>(doc: &'a Document, view: &PictureViewRust) -> Cow<'a, PixelBuffer>`, the five freshness guards from `design.md` (no live paint/mask stroke, no lifted transform, RGB working mode, four composite channels, matching dimensions) and a `current_buffer` fallback, and the module doc comment stating the freshness contract; register the module in `cxxqt_object.rs`; verify `cargo check -p pictura_app` and `bash scripts/check-file-size.sh`.
- [x] 1.3 Write `picture_is_served_after_a_committed_edit`, `picture_is_served_after_a_preview`, and `picture_is_served_after_history_navigation`: each asserts the accessor returns `Cow::Borrowed` after the corresponding state change; drive at least one direct preview path (opacity or fill) and one `preview_edit` path, and include a history jump, not only undo/redo; verify `cargo nextest run -p pictura_app picture`.
- [x] 1.4 Write `a_live_layer_stroke_is_composited` and `a_live_mask_stroke_is_composited`: each starts the stroke and samples a dab while skipping the present; the layer stroke asserts `Cow::Owned` and that the served buffer contains the dab while the maintained composite does not, and the mask stroke asserts `Cow::Owned` and equality with the full composite; verify `cargo nextest run -p pictura_app picture`.
- [x] 1.5 Write `a_lifted_transform_is_composited`, `a_grayscale_document_is_composited`, `a_mismatched_composite_is_composited`, and `a_layerless_document_keeps_its_embedded_composite`; each asserts the `Cow` variant and the observable guard condition (not a guard name, which the accessor does not expose); verify `cargo nextest run -p pictura_app picture`.
- [x] 1.6 Write `the_served_picture_equals_a_full_composite`, comparing the served buffer with `current_buffer(doc, false)` sample for sample for a fresh, a previewed, a live-stroke, a lifted-transform, a non-RGB, a layerless, and a mismatched-composite document; verify `cargo nextest run -p pictura_app picture`.

## 2. Selection and paint-tool sampling

- [x] 2.1 Move `impl_selection.rs` (Grow, Similar, Magic Wand, channel row), `impl_selection/quick.rs`, and `impl_selection/refine.rs` to the accessor; make `shrink` in `refine.rs` take `&PixelBuffer`; verify `cargo nextest run -p pictura_app selection` and `cargo nextest run -p pictura_app wand`.
- [x] 2.2 Move `magnetic.rs` and `color_range.rs` to the accessor; make `shrink` in `color_range.rs` take `&PixelBuffer`; verify `cargo nextest run -p pictura_app magnetic` and `cargo nextest run -p pictura_app color_range`.
- [x] 2.3 Move `paint_tools.rs` (`composite_surface`, `magic_erase_at`) and `paint_tools/fills.rs` (Paint Bucket with All Layers) to the accessor; verify `cargo nextest run -p pictura_app fill`, `cargo nextest run -p pictura_app erase`, and `cargo nextest run -p pictura_app smudge`.
- [x] 2.4 Add the test-only composite counter to `picture.rs` and write `a_read_only_consumer_composites_nothing` in `paint_tools.rs` (where `composite_surface` is in scope), asserting the counter delta is zero and the surface equals the document's composite; build the document with a stored 4-channel composite so the fast path applies; verify the test fails if the accessor is forced to composite, then passes.

## 3. Clipboard, adjustments and exports

- [x] 3.1 Move `clipboard.rs` (Copy Merged), `image_match_color.rs`, `image_replace_color.rs`, and `image_adjust/image_ops.rs` (Duplicate Merged, Trim) to the accessor; verify `cargo nextest run -p pictura_app clipboard`, `cargo nextest run -p pictura_app match_color`, `cargo nextest run -p pictura_app replace_color`, and `cargo nextest run -p pictura_app trim`.
- [x] 3.2 Move `export.rs`, `web_export.rs`, and `save_format.rs` to the accessor; keep `impl_core/save.rs` and `pdf_export.rs` on `current_buffer` (they refresh the retained native composite first) and keep the PDF live-type hidden-layer background on `current_buffer`; verify `cargo nextest run -p pictura_app export`, `cargo nextest run -p pictura_app save`, and `cargo nextest run -p pictura_app pdf`.
- [x] 3.3 Confirm no read-only consumer still reaches the compositor: list every remaining `current_buffer` call in `crates/pictura-app/src` and check it against the keep list in `design.md`; verify the list contains only `composite_full`, `new_document`, `store_opened_composite`, the async apply jobs, the transform previews, the PDF hidden-layer background, `impl_core/save.rs`, `pdf_export.rs`, `displace_maps`, `level0_composited`, `document_to_image`, and the test-only `rebuild_display`.

## 4. Integration checks

- [x] 4.1 Run the app suite and fix any call site that needed an owned buffer; verify `cargo nextest run -p pictura_app`.
- [x] 4.2 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, and `cargo test --workspace --doc`; verify all pass.
- [x] 4.3 Run the GUI suites touched by the migrated consumers; verify `ctest --test-dir build -R '^tst_(selection_api|selection_alt|selection_move|selection_preview|color_range|refine_edge|match_color|replace_color|channels_panel|paint_api|fill_tools|eraser_tools|retouch_tools|edit_clipboard|export_as|save_for_web|save_format_options|image_ops|open_save)$' --output-on-failure` passes.
- [ ] 4.4 Manual check on a large layered document: Magic Wand, Quick Selection drag, Sample All Layers stroke start, Copy Merged, and Save; verify each responds without the multi-hundred-millisecond stall and produces the same result as before.

## 5. Specs, archive

- [x] 5.1 Before archiving, run `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`; verify both pass.
- [x] 5.2 Archive with `openspec archive serve-current-composite`; verify `scripts/check-openspec-archived.sh` passes and only the archive remains under `openspec/changes/`.
- [x] 5.3 After archiving, run `bash scripts/verify-fast.sh`; verify it passes with the change already archived.
- [x] 5.4 When the user asks, commit with `perf(app): serve the current picture from doc.composite (#1010)`, open the PR, and list the test plan's manual rows as a reviewer checklist.

## 6. Measurement

- [x] 6.1 Add `picture_serve_profile_4000` to `picture.rs`'s tests: an `#[ignore]`d profile on a 4000² eight-layer document that times `current_buffer` (before) and `picture` (after) three times each and prints the medians; verify `cargo test -p pictura_app picture_serve_profile_4000 -- --ignored --nocapture` prints both numbers.
- [x] 6.2 Run the profile and record the before/after medians and the ratio in the task result and the PR body; verify the served path avoids the composite.
