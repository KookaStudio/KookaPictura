# Tasks: native-depth-masks

## 1. Native mask gating

- [x] 1.1 Add `pub(crate) fn mask_alpha_unit(doc: &Document, layer: &Layer, x: i32, y: i32) -> f32` in `composite_native.rs`: mirror `raster_mask_alpha`'s absent/disabled/empty/default-color branches in unit form, read the native `-2` plane (unit) when `doc.source_depth.is_some()` and the plane length matches the mask rect, else fall back to `raster_mask_alpha/255`, and multiply by the vector coverage in unit space.
- [x] 1.2 Change `blend_into` to take `doc: &Document` and use `mask_alpha_unit` for a high-depth document (falling back to `mask_alpha as f32 / 255.0` for 8-bit, preserving the existing u8 rounding).
- [x] 1.3 Update all `blend_into` call sites (`composite.rs`, `composite_native.rs`, `fill.rs`, `text_render.rs`); add `doc` to `composite_solid_fill`/`composite_gradient_fill` and their callers. Keep `composite.rs` at or below its allowlist ceiling (1217).

## 2. Verification

- [x] 2.1 Render test: a depth-16 document with a layer whose native `-2` mask is not the 8-bit widening composites differently from the widened 8-bit mask.
- [x] 2.2 Render test: an absent/disabled mask and an out-of-rect point gate as before (full coverage / default color).
- [x] 2.3 `cargo nextest run --workspace`, `cargo test -p pictura-render --test document document_oracle::`, `cargo test -p pictura-render`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/check-file-size.sh` (report `composite.rs` LOC), `openspec validate native-depth-masks --strict`.
