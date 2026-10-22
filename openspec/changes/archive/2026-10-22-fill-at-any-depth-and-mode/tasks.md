# Tasks

## 1. Tests

- [x] 1.1 Rust (`pictura-paint`): the colour-space tests — `a_gray_layer_takes_the_colours_luma`, `an_rgb_layer_keeps_the_colours_channels`, `a_gray_ramp_lies_on_the_gray_axis`, `an_eight_bit_fill_is_byte_identical`, `a_layer_above_the_canvas_fills_the_rows_on_the_canvas`; verified by the five tests naming the pixels, the ramp and the layer's rows they pin.
- [x] 1.2 Rust (`pictura-paint`): the native-store tests — `a_depth16_fill_writes_the_native_store`, `a_depth32_fill_writes_the_native_store`, `a_depth16_fill_blends_at_the_stores_depth`, `the_depth16_mirror_is_the_stores_narrowing`, `a_store_without_a_transparency_plane_still_fills`, `a_mismatched_store_falls_back_to_the_eight_bit_path`, `a_converted_mode_document_fills_the_eight_bit_path`, `a_native_blend_matches_the_eight_bit_modes`, `a_native_fill_refuses_a_locked_layer`, `a_native_dissolve_picks_the_same_pixels_as_the_eight_bit_path`; verified by the twelve tests naming the store and the working copy they pin.
- [x] 1.3 Rust (`pictura-render`): `a_depth16_fill_survives_a_psd_round_trip` in `crates/pictura-render/tests/native_depth_roundtrip.rs`; verified by the test naming the re-read native samples it pins.
- [x] 1.4 Qt (`tst_fill_tools`): `fillCommandOnADepth16Document`, `fillCommandOnAGrayDocument`, `fillCommandPreservesTransparency`, `paintBucketOnADepth16Document`, `gradientOnADepth16Document` and `strokeCommandOnADepth16Document`, declared in the suite's private slots; verified by the six cases existing in the suite and by 6.2.

## 2. The shared rules

- [x] 2.1 Add `Document::authoritative_plane(&self, layer: &Layer, id: i16) -> Option<&Samples>` to `pictura-core`, carrying the per-channel rule the compositor applies — a recorded source depth of 16 or 32 bits, no converted source mode, a store whose rectangle and depth match the layer's and the document's, and a plane for that channel — and point `pictura-render`'s `native_unit` at it; verified by the `pictura-render` native-depth suite (`depth16_rgb_layer_keeps_native_content`, `converted_mode_document_uses_the_8bit_path`, `depth16_layer_without_matching_store_falls_back`) passing with no golden change.
- [x] 2.2 Add `pictura_core::rec601_luma(r, g, b) -> u8` and have the app's `helpers::luma_u8` delegate to it, so the crate has one Rec. 601 luma; verified by the app's existing tests and the engine's luma test.

## 3. The engine's colour space

- [x] 3.1 Resolve the target layer's colour planes from the layer — channel ids 0, 1 and 2 for an RGB layer, channel id 0 alone for a Grayscale layer — reduce a Grayscale source colour to its Rec. 601 luma, and carry the optional green and blue planes through the row loop; verified by `a_gray_layer_takes_the_colours_luma`, `an_rgb_layer_keeps_the_colours_channels` and the existing `pictura-paint` fill suite.
- [x] 3.2 Reduce a Gradient's end colours to their luma before the ramp is sampled when the document is Grayscale, so the ramp lies on the gray axis; verified by `a_gray_ramp_lies_on_the_gray_axis` and the existing gradient tests.

## 4. The native engine

- [x] 4.1 Add `crates/pictura-paint/src/fill_native.rs`: borrow the store's colour and transparency planes, blend at the store's own depth in the unit domain, write them back, and re-narrow the layer's 8-bit channels from the store; verified by `a_depth16_fill_writes_the_native_store`, `a_depth32_fill_writes_the_native_store`, `a_depth16_fill_blends_at_the_stores_depth` and `the_depth16_mirror_is_the_stores_narrowing`.
- [x] 4.2 Dispatch to the native engine per channel — the store's plane where the store is authoritative for it, the 8-bit channel otherwise — in an immutable pass before the layer is borrowed mutably; verified by `a_store_without_a_transparency_plane_still_fills`, `a_mismatched_store_falls_back_to_the_eight_bit_path` and `a_converted_mode_document_fills_the_eight_bit_path`.
- [x] 4.3 Verify the 8-bit path's bytes do not move: `an_eight_bit_fill_is_byte_identical` and the existing `pictura-paint` fill suite.

## 5. The app level

- [x] 5.1 The command reaches the layer in every colour mode and depth: the six new Qt cases cover it, and the suite's existing cases are unchanged.

## 6. The whole change

- [x] 6.1 `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` pass with no new warning.
- [x] 6.2 `cargo nextest run --workspace` and `cargo test --workspace --doc` pass, and `ctest --test-dir build -R '^tst_fill_tools' --output-on-failure` runs the six new Qt cases.
- [x] 6.3 `bash scripts/verify-fast.sh` passes, including `scripts/check-test-plan.py` against the plan's rows and the file-size ceilings on the touched files.
- [x] 6.4 `openspec validate --all --strict` passes; `docs/` and `ROADMAP.md` are confirmed unchanged — the corpus never restricted `Edit > Fill` to 8-bit RGB, and this fixes a bug rather than shipping a roadmap item — and the Paint Bucket's 32-bit availability, which the corpus puts out of reach, is named on issue #791.
- [x] 6.5 `openspec archive fill-at-any-depth-and-mode` merges the deltas into the main specs, so the change leaves no open change behind.
