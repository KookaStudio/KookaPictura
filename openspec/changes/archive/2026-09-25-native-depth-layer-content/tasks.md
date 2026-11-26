# Tasks: native-depth-layer-content

## 1. Native layer content

- [x] 1.1 Add a helper (in `composite_native.rs`, or `composite.rs` if the cap allows) that returns a pixel layer's unit-domain color/alpha for a high-depth RGB/Grayscale document when `layer.source_channels` matches the layer rect and the document source depth, else `None`.
- [x] 1.2 In `composite_pixels`, take the native path when the helper applies (color ids `0..color_channels`, alpha `-1`, missing channels fall back to the 8-bit `sample` and alpha default `255`); otherwise keep the existing 8-bit path. Gate on `doc.source_depth.is_some() && doc.source_mode.is_none()`.
- [x] 1.3 If `composite.rs` would exceed its allowlist ceiling, move `composite_pixels` into `composite_native.rs`, publish the needed items as `pub(crate)`, and lower the `composite.rs` allowlist entry if it shrinks.

## 2. Verification

- [x] 2.1 Render test: a depth-16 RGB document with a pixel layer whose `source_channels` native samples are not the 8-bit widening yields `composite_native` samples that differ from the widening of `composite_rgba`.
- [x] 2.2 Render test: a `source_mode = Some(Cmyk)` depth-16 document takes the 8-bit path (native composite equals the widening of the composited color).
- [x] 2.3 Render test: a depth-16 layer without a matching store falls back to 8-bit.
- [x] 2.4 `cargo nextest run --workspace`, `cargo test -p pictura-render --test document document_oracle::`, `cargo test -p pictura-render`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/check-file-size.sh` (report `composite.rs` LOC), `openspec validate native-depth-layer-content --strict`.
