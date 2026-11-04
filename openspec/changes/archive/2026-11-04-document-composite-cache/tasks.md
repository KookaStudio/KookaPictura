# Tasks

## 1. Write the render cache tests first (inline `#[cfg(test)] mod tests` in `document_composite.rs`)

- [x] 1.1 Write `pictura-render::a_full_refresh_stores_the_composite` and verify it asserts document-sized output equal to a full composite.
- [x] 1.2 Write `pictura-render::a_region_refresh_patches_the_composite` and verify it asserts the region is written and the outside pixels are unchanged.
- [x] 1.3 Write `pictura-render::an_rgb_region_patch_matches_the_full_slice` and verify it asserts byte equality against the full-composite slice and that the outside pixels are unchanged.
- [x] 1.4 Write `pictura-render::a_grayscale_patch_keeps_one_plane` and verify it asserts one colour plane and equality with the full grey plane.
- [x] 1.5 Write `pictura-render::a_banded_composite_is_the_patch_reference` and verify it pins the banded full composite against the single-canvas composite and the patched clone.
- [x] 1.6 Write `pictura-render::a_whole_document_region_replaces_the_composite` and verify it asserts the whole-document region equals a full composite and the previously seeded composite's bytes are unchanged (no in-place mutation).
- [x] 1.7 Write `pictura-render::a_region_patch_preserves_a_shared_banded_plane` and verify it fractures/shares the plane with a history snapshot and asserts the plane stays shared or the snapshot bytes are unchanged.
- [x] 1.8 Write `pictura-render::a_rendered_buffer_is_stored_without_recompositing` and verify it stores a distinctive marker buffer and reads it back unchanged.
- [x] 1.9 Write `pictura-render::render_does_not_write_the_cache` and verify the read-only path leaves the composite unchanged.
- [x] 1.10 Write `pictura-render::a_size_mismatch_is_not_shape_valid` and verify a mismatched composite reports not shape-valid.
- [x] 1.11 Write `pictura-render::a_non_rgb_or_non_four_channel_composite_is_not_shape_valid` and verify both cases report not shape-valid.
- [x] 1.12 Write `pictura-render::a_layerless_full_refresh_stores_the_embedded_composite` and verify it asserts the result matches the old `store_composite(current_buffer(doc))` behavior: RGB becomes 4 channels with the colour planes equal to the embedded composite, and a non-RGB composite keeps its plane count.
- [x] 1.13 Write `pictura-render::a_layerless_region_refresh_is_a_noop` and verify the composite is unchanged and no result is returned.
- [x] 1.14 Write `pictura-render::store_expands_planes_like_rgba_frame` and verify the RGB and grayscale plane expansion matches the moved `rgba_frame` helper.

## 2. Add the render cache module

- [x] 2.1 Add `pictura-render/src/document_composite.rs` with `DocumentComposite`, `CompositeScope`, `Refreshed`, `store`, `patch`, `refresh`, `render`, and `is_shape_valid`; export it from `lib.rs` and verify the crate compiles.
- [x] 2.2 Copy the full-store body and `rgba_frame` into the new module, preserving the RGB/grayscale plane-count rule; verify `a_full_refresh_stores_the_composite`, `a_grayscale_patch_keeps_one_plane`, and `store_expands_planes_like_rgba_frame`.
- [x] 2.3 Implement region refresh with clamping, whole-region store, ordered row-wise patch, and duplicate-tolerant `Regions`; verify `a_region_refresh_patches_the_composite`, `an_rgb_region_patch_matches_the_full_slice`, and `a_whole_document_region_replaces_the_composite`.
- [x] 2.4 Implement the read-only `render` path and verify `render_does_not_write_the_cache`.
- [x] 2.5 Implement `is_shape_valid`, the layerless full-refresh store, and the layerless `Regions` no-op; verify `a_size_mismatch_is_not_shape_valid`, `a_non_rgb_or_non_four_channel_composite_is_not_shape_valid`, `a_layerless_full_refresh_stores_the_embedded_composite`, and `a_layerless_region_refresh_is_a_noop`.
- [x] 2.6 Add the banded-reference pin, the shared-plane test, and the fixture matrix (RGB, grayscale, clipping, tall document; knockout and effects stay covered by the existing `pictura-render` region suites); verify `a_banded_composite_is_the_patch_reference`, `a_region_patch_preserves_a_shared_banded_plane`, `a_clipping_region_patch_matches_the_full_slice`, and `a_grayscale_store_keeps_the_red_plane`.
- [x] 2.7 Move `patch_buffer_region` into the render module as a public utility and verify `a_rendered_buffer_is_stored_without_recompositing`.

## 3. Migrate the application to the render cache

- [x] 3.1 Route `composite_full` through `refresh(Full)` and verify the existing edit and full-recomposite tests pass.
- [x] 3.2 Route `refresh_region_buffer` through `refresh(Regions)` while keeping the sRGB conversion in the app; verify `one_commit_refreshes_once_through_the_cache` and the existing region-refresh tests.
- [x] 3.3 Route the open, filter-job, image-adjust-job, paint-cancel, and history-snapshot cache writers through `store`/`patch`/`refresh`; verify `job_writers_store_and_patch_through_the_cache` and their existing tests.
- [x] 3.4 Switch the live mask-stroke present path to `render` and verify `a_mask_present_leaves_the_composite_unchanged`.
- [x] 3.5 Replace the mode, channel-count, and dimension checks in `picture::is_current` with `DocumentComposite::is_shape_valid`, keep the live-state gates, and verify the `picture.rs` tests.
- [x] 3.6 Move `rgba_frame` consumers to the render symbol, delete `store_composite`, `patch_composite_region`, `patch_buffer_region`, and `rgba_frame` from `helpers_composite.rs`, repoint every import, and verify `rg` finds no app-side definition (not merely no import).
- [x] 3.7 Delete the composite-patch-only app oracle tests `paint_commit_region_patch_equals_a_full_recomposite` and `visibility_region_path_matches_full_recomposite`; keep the display/level-0 integration tests such as `region_refresh_keeps_composite_and_rebuilt_image_equal_to_full`.
- [x] 3.8 Write `pictura-app::one_commit_refreshes_once_through_the_cache` with a test-only refresh counter; assert one cache call per commit, a damaged rectangle for the bounded commit, and `None` for the full commit.
- [x] 3.9 Write `pictura-app::an_empty_region_edit_rebuilds_fully` and verify an empty `Edit::Regions` upgrades to a full rebuild.
- [x] 3.10 Write `pictura-app::a_mask_present_leaves_the_composite_unchanged` and verify a live mask present does not write the maintained composite.
- [x] 3.11 Write `pictura-app::job_writers_store_and_patch_through_the_cache` and verify the filter/image-adjust job buffers reach the live composite through `store`/`patch`.
- [x] 3.12 Add the no-second-store/patch static check to the helper guard (or the task 3.6 `rg` check) and verify it fails on an app-side definition.

## 4. Integration checks

- [x] 4.1 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo nextest run --workspace`; verify all pass.
- [x] 4.2 Run `openspec validate --all --strict` and `python3 scripts/check-test-plan.py`; verify both pass.
- [x] 4.3 Verify `git diff --stat` shows no changes under `crates/pictura-codec`, `crates/pictura-core`, or `crates/pictura-render/src/document_ops`, and no new dependency in any `Cargo.toml`.
