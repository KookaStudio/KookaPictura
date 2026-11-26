# Tasks: bit-depth-sample-model

## 1. Core representation

- [x] 1.1 Make `PixelBuffer<T = u8>` generic in `crates/pictura-core/src/lib.rs` (`data: Vec<T>`), with `new` bounded `T: Clone + Default` and `pixel_count` unbounded; fix any turbofish sites the compiler names.
- [x] 1.2 Add `Samples` (enum `U8(U8)/U16(Vec<u16>)/F32(Vec<f32>)`) with `depth()`, `len()`, `is_empty()`, and the depth conversion helpers `narrow_to_u8` / `widen_from_u8` used by the codec.
- [x] 1.3 Change `SourcePlanes` to hold `Samples` + dims + depth, and `SourceChannels` to hold `Samples` + rect + depth + channel ids, keeping `new`'s sort-and-equality behavior.
- [x] 1.4 Unit tests in `pictura-core` for the converters (`0/255/256/32768/65535`, `1.5/-0.5`, `0/1/255`) and for `Samples` length/depth.

## 2. Codec decode/encode

- [x] 2.1 `crates/pictura-codec/src/read.rs`: decode the retained composite/document-extra and per-layer channel byte images into `Samples` (u16/f32 decode per `psd-bit-depth`); keep the 8-bit Lab store as `U8`.
- [x] 2.2 `crates/pictura-codec/src/write.rs` and `color_mode.rs`: encode `Samples` back to the PSD byte image with the depth-specific rule; route narrowing/widening through the shared converters.
- [x] 2.3 Update the render sites that construct/inspect the retained store (`document_ops/{resize,orient,transform}.rs`).

## 3. Verification

- [x] 3.1 `cargo nextest run -p pictura-core -p pictura-codec -p pictura-render` green with no golden change.
- [x] 3.2 `cargo test -p pictura-codec --test psd depth_oracle::` and `--test psd color_mode_oracle::` green (self-skip if `psd-tools` absent).
- [x] 3.3 Add the byte-exact encoded-equals-decoded unit test for depth 16 and 32.
- [x] 3.4 `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `scripts/check-file-size.sh`.
