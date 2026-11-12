# Tasks: bristle-brushes

## 1. Engine

- [x] 1.1 Add `pictura-paint/src/bristle.rs`: `BristleShape`, `BristleTip`, `BristleTrack`, `BristleDab`, the six shape profiles, and deterministic track seeds.
- [x] 1.2 Put `bristle: Option<BristleTip>` on `StrokeConfig` and sanitize it; keep every old default exact.
- [x] 1.3 Raster a bristle dab through `DabRaster` (coverage, reach, x-span) and use the bristle reach in the stroke's saved-pixel and dirty bounds.
- [x] 1.4 Add the per-bristle reservoir path to `MixerBrush`; keep the non-bristle path byte-identical.
- [x] 1.5 Decline the GPU stroke path when `cfg.bristle` is set.
- [x] 1.6 Unit tests for every engine scenario in the test plan.

## 2. Bridge and UI

- [x] 2.1 Add the bristle fields to the `PaintTip` bridge and translate them in `tip_config`.
- [x] 2.2 Add the bristle fields to `PaintOptions` (getters, clamping setters, `apply`).
- [x] 2.3 Add the Bristle Tip toggle, Shape combo, and Bristles/Length/Thickness/Stiffness fields to the Brush Tip Shape page; disable Hardness/Roundness while it is on.
- [x] 2.4 Add `Bristle Round 30` and `Bristle Flat 40` to the preset picker and apply the bristle fields.
- [x] 2.5 Persist and restore the bristle fields through `PaintFields` and `tool_options_json`.

## 3. Tests

- [x] 3.1 Add `tst_bristle_brushes.cpp`, register it in `cpp/tests/CMakeLists.txt`, and cover every Qt scenario in the test plan.
- [x] 3.2 Update the preset-count expectation in `tst_brush_tip_preview` for the two new presets.

## 4. Verification and close-out

- [x] 4.1 Run `cargo fmt`, `cargo clippy`, targeted `cargo test`, the Qt suite, file-size, guard, and `openspec validate --all --strict`.
- [x] 4.2 Update `docs/dev/STATE.md` and `docs/dev/doc-code-parity-gaps.md` with `TASK-ALLOWS-DOCS`.
- [x] 4.3 Archive the change with `openspec archive bristle-brushes`.
