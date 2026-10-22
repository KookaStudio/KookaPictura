# Tasks

## 1. The engine's alpha policy

- [x] 1.1 Add `Alpha` to `pictura-paint` — `Blend`, `Keep` and `Replace` — and thread it through `fill_planes`, where the pixel's alpha is written, and through `fill_layer`, where a layer's transparency lock forces `Keep` over the caller's policy; verified by `a_keep_policy_keeps_the_destination_alpha`, `a_replace_policy_takes_the_sources_alpha` and `a_transparency_lock_outranks_the_policy`.
- [x] 1.2 Pass the policy on to the native pass in `fill_native.rs`, so a 16- or 32-bit document's pixels take the same treatment an 8-bit document's do; verified by `a_keep_policy_keeps_the_destination_alpha_at_depth_16` and `a_replace_policy_takes_the_sources_alpha_at_depth_16`.
- [x] 1.3 Give `bucket::fill` the policy, and have the Paint Bucket, the shape and path fills, the mask fill and the stroke ask for `Blend`; verified by the existing `pictura-paint` bucket suite and the app's shape and path suites.

## 2. The stroke's band

- [x] 2.1 Give `dilate` and `erode` a horizontal pass that reads the source mask and writes a buffer of its own, as their vertical pass already does, so the filter no longer cascades along the row; verified by `the_outside_band_stops_at_the_selections_edge`, `the_inside_band_stops_at_the_selections_edge` and `the_band_does_not_reach_the_canvas_edge`.

## 3. The callers

- [x] 3.1 Have `Edit > Fill` ask for `Keep` when the dialog's Preserve Transparency is set and `Blend` otherwise, and delete the app's coverage-mask workaround; verified by `tst_fill_tools::fillCommandPreservesTransparency` and the new `tst_fill_tools::fillCommandPreservesTransparencyAtDepth16`.
- [x] 3.2 Have the Gradient ask for `Replace` when its Transparency option is on and its mode is Normal, and `Blend` otherwise; verified by `the_ramps_alpha_reaches_the_pixels`, `transparency_off_draws_the_ramp_solid` and the new depth-16 counterpart.

## 4. The tests

- [x] 4.1 Rust (`pictura-paint`): the band's far end — the 8×1 row's last pixel and a 24×24 canvas whose selection has room to its right.
- [x] 4.2 Rust (`pictura-paint`): the three policies, at 8 bits and at 16 bits, over a half transparent and an empty pixel.
- [x] 4.3 Rust (`pictura-paint`): the ramp's transparent end, at 8 bits and at 16 bits.
- [x] 4.4 Qt (`tst_fill_tools`): the band with room to the right of the selection, and Preserve Transparency on a 16-bit document with a half transparent pixel.

## 5. The whole change

- [x] 5.1 `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean.
- [x] 5.2 `cargo nextest run --workspace`, `cargo test --workspace --doc` and the Qt suites pass, with the golden baselines unchanged.
- [x] 5.3 `bash scripts/verify-fast.sh` is green, including the archive gate.
- [x] 5.4 `docs/` and `ROADMAP.md` are unchanged: the corpus already says Preserve Transparency keeps the transparent areas, that a stroke lays a band along the selection edge, and that the Gradient's Transparency uses the ramp's mask.
- [x] 5.5 `openspec archive fix-stroke-band-and-transparency` merges the deltas into the main specs, so the change leaves no open change behind.
