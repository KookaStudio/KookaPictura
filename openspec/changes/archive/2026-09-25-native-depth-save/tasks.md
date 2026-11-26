# Tasks: native-depth-save

## 1. Refresh helper

- [x] 1.1 Add `pub fn refresh_native_composite(doc: &mut Document) -> bool` to `pictura-render` (`composite_native.rs`, exported from `lib.rs`): gate on `source_depth` 16/32, `source_mode` `None`, non-empty `layers`; compute `composite_native`; set `doc.composite` from its narrowing (4-plane RGBA for RGB, 1-plane for Grayscale, mirroring `store_composite`); splice the native color planes into the first `color_channels` planes of `doc.source_planes.samples` via `to_bytes`/`from_bytes`, preserving the rest; return `true`. Return `false` with no mutation otherwise.
- [x] 1.2 Do not touch `doc.channels`, the alpha/extra planes, or an 8-bit result.

## 2. App save wiring

- [x] 2.1 In `crates/pictura-app/src/cxxqt_object/impl_core.rs` `save`, when the view is dirty, call `pictura_render::refresh_native_composite(doc)` on the document before `write_psd` (restructure the borrow as needed); a clean save must not call it.

## 3. Verification

- [x] 3.1 Render/codec test: a depth-16 RGB document with a pixel layer (native samples not the 8-bit widening) is edited/marked so the refresh runs; `write_psd` then `read_psd` yields depth 16 and a retained composite that is not the 8-bit widening.
- [x] 3.2 Render test: `refresh_native_composite` returns `false` and mutates nothing for a no-layer, converted-mode (`Some(Cmyk)`), 8-bit, or `source_depth`-absent document.
- [x] 3.3 App test (if the harness allows): a dirty 16-bit view saves depth 16; an open→save with no edit is byte-identical.
- [x] 3.4 `cargo test -p pictura-codec --test psd depth_oracle::` and `--test psd color_mode_oracle::` still green; `cargo nextest run --workspace`; `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `bash scripts/check-file-size.sh`; `openspec validate native-depth-save --strict`.
