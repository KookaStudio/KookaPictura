## 1. Model

- [x] 1.1 Add `pub source_depth: Option<BitDepth>` to `pictura_core::Document`; set it to `None` in `Document::new`/`Document::from_rgba` and in every full struct literal (`cargo check --workspace --all-targets` enumerates them). Leave the field unset for depth-1 Bitmap (its 1-bit expansion stays recorded by `source_mode`).
- [x] 1.2 Unit-test that a constructed document and an 8-bit read have `source_depth == None`.

## 2. Codec: depth gate and row stride

- [x] 2.1 In `read.rs`, replace the depth check with the `psd-bit-depth` mapping: accept depth 1 (Bitmap only) and 8 (all accepted modes) as today, and accept 16/32 for Grayscale/RGB/CMYK/Lab; keep Bitmap/Indexed at 16/32 and every other depth as `PsdError::Unsupported`.
- [x] 2.2 Add `row_bytes(width, depth) = (width * depth).div_ceil(8)` and thread it through the raw composite branch, `read_rle`, `planar_len`, and `decode_bitmap_channel`; replace the `if bits == 1` special case.
- [x] 2.3 Make `decode_prediction` depth-aware: byte-wise at 8 (unchanged), per-`u16` big-endian running sum mod 2^16 at 16, and 4-plane unshuffle + byte-wise delta at 32. Keep ZIP at depth 1 `Unsupported`.

## 3. Codec: sample narrowing

- [x] 3.1 Add `narrow_u16_to_8(v: u16) -> u8 = (v >> 8) as u8` and `narrow_f32_to_8(f: f32) -> u8 = (f * 256.0).trunc().clamp(0.0, 255.0) as u8` (std-only), each with a `ponytail:` comment naming the `psd-tools`-compatible convention and the Photoshop/HDR ceiling.
- [x] 3.2 Narrow the composite's decoded byte buffer in place from 16/32 to 8-bit before `split_planes`, so `split_planes`/`normalize`/the color-mode conversions see the 8-bit layout.
- [x] 3.3 Thread `depth: u16` (replacing the `bits` flag) through `read_layer_section` → `read_layer_info` → `read_channel_data`; decode a 16/32 layer channel at the document depth and narrow it to an 8-bit plane (color, `-1`, `-2`, and unmodeled channels alike), mirroring `decode_bitmap_channel`. Keep `patterns.rs` on the 8-bit `decode_channel_data`.
- [x] 3.4 On a normalized document set `depth = Eight` and `source_depth = Some(Sixteen | ThirtyTwo)`; leave depth 8 and depth-1 Bitmap exactly as shipped.

## 4. Codec unit tests

- [x] 4.1 16-bit: a hand-built raw RGB composite narrows as `v >> 8` (values `0, 1, 255, 256, 257, 32768, 65534, 65535` → `0, 0, 0, 1, 1, 128, 255, 255`); a non-multiple-of-16-bit width; a 16-bit RLE row stride (`2 * width`).
- [x] 4.2 32-bit: a hand-built raw RGB composite narrows as `clamp(trunc(f * 256))` (`0.0, 0.001, 0.5, 1.0, 1.5, -0.5, 0.99609375, 255.0` → `0, 0, 128, 255, 255, 0, 255, 255`).
- [x] 4.3 ZIP-with-prediction at 16 (per-`u16` delta) and 32 (shuffle + byte delta), using a hand-built stream computed by the psd-tools algorithm (the P1 ZIP hand-built precedent); ZIP at depth 1 still `Unsupported`.
- [x] 4.4 A hand-built 16-bit layered file (`layered_psd_depth` style) proves the layer color channel is narrowed to 8-bit and a `-2` mask is narrowed too.
- [x] 4.5 A 16-bit CMYK file reads as RGB with `source_depth == Some(Sixteen)` and the narrowed-then-converted pixels.
- [x] 4.6 Malformed inputs are typed errors, never panics: a truncated 16-bit raw composite, a 16-bit Bitmap header, and a 16-bit Indexed header.
- [x] 4.7 Update `unsupported_depths_and_color_modes_are_rejected` to drop the now-accepted depth 16/32 cases and keep an invalid depth (e.g. 4) plus modes 7/8.

## 5. Fixtures and oracle

- [x] 5.1 In `scripts/generate-fixtures.py`, add `rgb16()` and `rgb32()` builders: a flat RGB document with `header.depth = 16`/`32` and hand-built big-endian planes (`struct.pack(">H"…)` / `struct.pack(">f"…)`) via the existing `_set_composite`; register both in `FIXTURES`. Confirm regeneration is byte-stable and additive.
- [x] 5.2 Commit `crates/pictura-codec/tests/fixtures/{rgb16,rgb32}.psd`.
- [x] 5.3 Add `crates/pictura-codec/tests/psd/depth_oracle.rs` (mirroring `color_mode_oracle.rs`): decode each fixture with `read_psd`, compare to `psd-tools`' `.convert("RGB")` composite with **tolerance 0**, and self-skip with a clear message when `psd-tools` is absent.
- [x] 5.4 Add a round-trip test: read each fixture, assert `depth == Eight`, `source_depth == Some(..)`, `mode == Rgb`; write with `write_psd`, re-read, assert `source_depth == None` and stable pixels, and confirm `psd-tools` opens the output as an 8-bit RGB document.
- [x] 5.5 Confirm no existing fixture or golden changes.

## 6. App

- [x] 6.1 In `crates/pictura-app/src/cxxqt_object.rs` and `impl_core.rs`, add `#[qinvokable] fn depth_notice(&self) -> QString` returning `"Converted from 16-bit"` / `"Converted from 32-bit"` from `doc.source_depth` (empty otherwise), mirroring `mode_notice`.
- [x] 6.2 In `crates/pictura-app/cpp/frame.cpp::openPath`, show the mode and depth notices together after a successful open (join non-empty ones); rebase onto the concurrent session's changes to this call site rather than overwriting.
- [x] 6.3 Add one C++ self-test check taking **exit code 298** (295/296/297 in use): write a minimal flat 16-bit RGB PSD to a `QTemporaryDir`, `view->open` it, and assert the open succeeds, `depth_notice()` names 16-bit, the document mode reads RGB, and a composite pixel equals `v >> 8`. Add it to the existing `selftest_layers_adjustments.cpp` sub-runner to avoid a `CMakeLists.txt` edit.
- [x] 6.4 Confirm `new_document` and `decode_image`/`open_image` are unchanged (creation is still 8-bit RGB/Grayscale).

## 7. Gates

- [x] 7.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, `cargo test --workspace --doc`.
- [x] 7.2 `bash scripts/verify-full.sh` and a headless self-test; record counts and confirm the `psd-tools` oracle ran rather than self-skipped.
- [x] 7.3 `openspec validate depth-read --strict` and `openspec validate --all --strict`.
- [x] 7.4 Commit code + openspec + fixtures. Any `docs/` update (roadmap G4/P4, `STATE.md`) is a separate `TASK-ALLOWS-DOCS` commit.
