# Tasks: native-depth-edit-preserve

## 1. Sample-typed rebasing kernels

- [x] 1.1 Add `crates/pictura-render/src/document_ops/native_store.rs`: `extend_samples` (offset blit), `resize_samples` (the `pictura_ops::resize` nearest/bilinear/bicubic kernels via `Sample`), and `remap_samples` (the five exact orientation maps), each mapping whole planes and dispatching on `Samples`.
- [x] 1.2 Declare the module in `document_ops/mod.rs`.

## 2. Translate / crop / canvas

- [x] 2.1 In `crop.rs`, offset `source_channels.rect` in `crop_document` and the four `translate_layer*` paths; rebase `source_planes` in `crop_document` via `canvas::rebase_source_planes`.
- [x] 2.2 In `canvas.rs`, offset `source_channels.rect` for every layer and rebase `source_planes` in `resize_canvas_document`.

## 3. Transform family

- [x] 3.1 Add `crates/pictura-render/src/document_ops/layer_ops/transform_native.rs`: `resample_native`, `bilinear_sample`, `resample_store`, `warp_store`, `dest_tuple`; make `PlaneMap`/`resample_plane` `pub(super)` and `warp_plane` generic over a sampler.
- [x] 3.2 Add `Prepared.native` and a `LayerOutput` bundle; `write_layer` derives each 8-bit channel (and mask `data`) from the resampled native narrowing when `source_mode` is `None`, and sets the resampled store on the non-translate path.
- [x] 3.3 Resample the store in `apply_layer_map` (`transform_layer`/`transform_layer_quad`) and in `transform_layer_warp`.

## 4. Resize / orient

- [x] 4.1 In `resize.rs`, resample each layer store (`-2` by the mask) and `source_planes` in `resize_document`.
- [x] 4.2 In `orient.rs`, remap each layer store and `source_planes` exactly, update the rects, swap the recorded dimensions for 90°/270°, and stop clearing the store.

## 5. Tests

- [x] 5.1 Update `transform_tests.rs`: move keeps the store rect and drops only `raw_channels` on scale; add a pure-move save test that re-emits native samples (not the 8-bit widen).
- [x] 5.2 Update `resize.rs`'s native-store test; add warp, orient, canvas, and crop native-store checks.
- [x] 5.3 Add an end-to-end 16-bit fixture round-trip in `tests/document/native_depth_roundtrip.rs` for move, scale, and quad.

## 6. Docs and verification

- [x] 6.1 Update `docs/dev/STATE.md` and `docs/dev/psd-support-roadmap.md` with the shipped change and its ceilings.
- [x] 6.2 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`, `openspec validate --all --strict`, `bash scripts/check-file-size.sh`.
