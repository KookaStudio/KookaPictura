# Tasks

## 1. Core rectangle algebra

- [x] 1.1 Write the edge-case tests in `crates/pictura-core/src/tests.rs`: `psd_rect_is_empty_covers_zero_inverted_and_huge`, `psd_rect_intersect_returns_none_when_disjoint_or_touching`, `psd_rect_clip_to_clamps_and_reports_a_miss`, `psd_rect_union_is_the_bounding_box_without_an_empty_identity`, and `psd_rect_grow_saturates_at_the_i32_limits`; verify `cargo nextest run -p pictura-core` fails before 1.2.
- [x] 1.2 Add `Default` to `PsdRect` and implement `is_empty`, `intersect`, `union`, `clip_to`, and `grow` in `crates/pictura-core/src/lib.rs`; verify `cargo nextest run -p pictura-core` passes and `crates/pictura-core/src/lib.rs` stays under the 1200-line hard cap.
- [x] 1.3 Document the five methods in `crates/pictura-core/MODEL.md`; verify each signature matches the implementation and no line-number reference is introduced.
- [x] 1.4 Keep `openspec/changes/psdrect-algebra/test-plan.md` in sync with the test names; verify `python3 scripts/check-test-plan.py` passes.

## 2. Render helper removal

- [x] 2.1 Replace the intersect/clip helpers in `canvas_damage.rs`, `damage.rs`, `filter.rs`, `filter_proxy.rs`, `document_ops/crop.rs`, `document_ops/slices.rs`, `document_ops/layer_ops/clipboard.rs`, `document_ops/layer_ops/move_content.rs`, `document_ops/layer_ops/align.rs`, `document_ops/layer_ops/merge.rs`, `gpu/raster.rs`, and `layer_source/mod.rs` with the methods and delete the private functions; verify `cargo nextest run -p pictura-render` passes.
- [x] 2.2 Replace the union/grow/is_empty helpers in `layer_effects/mod.rs`, `gpu/clip.rs`, `damage.rs` (including `damage::union_rect`), `document_ops/layer_ops/move_content.rs`, `document_ops/layer_ops/align.rs`, and `document_ops/layer_ops/merge.rs`; update the `layer_source::clip_to_document` caller in `fill.rs` to use `clip_to`. Keep the `Option` union combinators: `damage::union` is None-wins (`Some(a?.union(b?))`), and `gpu/clip::union`, `healing/stroke::union`, and `liquify::union` keep their `match` with `a.union(b)` in the `(Some, Some)` arm. Verify `cargo nextest run -p pictura-render` passes.
- [x] 2.3 Run the damage, clipping, and compositing suites and confirm no golden changes; verify `cargo test -p pictura-render --test document_oracle` and the existing region/damage tests pass unchanged.

## 3. App helper removal

- [x] 3.1 Replace the rect helpers in `cxxqt_object/helpers.rs`, `impl_transform/session/view_preview.rs`, `path_list/imp.rs`, `crop_group/imp.rs`, `layer_style/imp.rs`, `display_preview.rs`, `history.rs`, and `impl_filters.rs` (`expand` is the fifth grow helper) with the methods and delete the private functions. Update the production call sites in `impl_paint.rs`, `impl_paint/mask.rs`, `impl_paint/mask_gpu.rs`, `impl_filters.rs`, `impl_transform/mod.rs`, `impl_transform/session.rs`, `impl_transform/session/preview.rs`, `impl_layers.rs`, `impl_core.rs`, `filter_tools/jobs/imp.rs`, `image_adjust/jobs/imp.rs`, and `impl_history.rs`. Keep `cxxqt_object::helpers::clamp_region` as the `(x0, y0, w, h)` tuple adapter and rewrite its body to delegate to `PsdRect::clip_to`. Verify `cargo nextest run -p pictura-app` passes.
- [x] 3.2 Update the tests that call the deleted helpers (`cxxqt_object/damage_policy_tests.rs`, `canvas_view_test.rs`, `tests.rs`, `tests_profiles.rs`, `tests_impl.rs`) to call the methods; verify the same `pictura-app` run passes.

## 4. Paint and filters

- [x] 4.1 Replace `healing::intersect`/`healing::inflate` and `stroke::union_rect` with the methods and delete them; update the call sites in `healing/content_move.rs`, `healing/red_eye.rs`, `healing/proximity.rs`, `healing/tests.rs`, and `stroke/raster.rs`; verify `cargo nextest run -p pictura-paint` passes.
- [x] 4.2 Replace the inner rect union in `distort/liquify.rs` with `PsdRect::union` while keeping the `None`-identity combinator, replace `liquify::clip` with `PsdRect::clip_to`, and replace `liquify::overlap` with `PsdRect::intersect`; verify `cargo nextest run -p pictura-filters` passes.

## 5. Integration

- [x] 5.1 Grep the workspace for the deleted private helper names; verify no deleted helper function remains and no new private copy is introduced. Inline min/max expressions named in design.md are out of scope.
- [x] 5.2 Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`; verify both pass.
- [x] 5.3 Run `openspec validate --all --strict`; verify the change validates.
- [x] 5.4 Run `openspec archive psdrect-algebra`; verify the change moves under `openspec/changes/archive/`.
- [x] 5.5 Run `bash scripts/verify-fast.sh`; verify it passes now that the change is archived.
