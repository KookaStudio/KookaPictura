# Tasks: brush-dynamics-option-sets

## 1. Engine

- [x] 1.1 Add `pictura-paint/src/dynamics.rs`: `Control`, `DynamicsInput`, `Dynamics`, the five set blocks, and the evaluation helpers.
- [x] 1.2 Put `Dynamics` on `StrokeConfig` and sanitize it; keep every old default exact.
- [x] 1.3 Add `Stroke::sample_with`, per-stroke/per-dab color, transfer factors, pose overrides, tangents, and dab indices; pass per-dab paint/opacity/flow into the raster.
- [x] 1.4 Unit tests for every engine scenario in the test plan.

## 2. Bridge and input

- [x] 2.1 Add the `PaintDynamics` bridge struct and put it on `PaintTip`; translate it in `tip_config`.
- [x] 2.2 Forward `QTabletEvent` tilt and rotation through `ImageView`, `ToolController`, `ToolServices`, and `paint_api::Stroke`.
- [x] 2.3 Add the `paint_dab_pose` bridge and route the Brush's dabs through it.

## 3. Options, persistence, and padlocks

- [x] 3.1 Extend C++ `BrushDynamics` with the new fields, controls, enable flags, and padlocks; clamp them in `setDynamics`.
- [x] 3.2 Serialize and restore every new field in `tool_options_json.cpp`; keep old sessions loading.
- [x] 3.3 Make the brush preset picker preserve locked sets and update unlocked ones.

## 4. Brush panel

- [x] 4.1 Replace the single tip page with a `QStackedWidget` and enable Shape Dynamics, Scattering, Color Dynamics, Transfer, and Brush Pose.
- [x] 4.2 Add the shared Control row (combo + Fade steps), enable checkboxes, per-set padlock, and the pages' controls.
- [x] 4.3 Follow external changes and restore the selected page.

## 5. Verification

- [x] 5.1 Add `tst_brush_dynamics.cpp`, register it in `cpp/tests/CMakeLists.txt`, and cover every Qt scenario in the test plan.
- [x] 5.2 Run `cargo fmt`, `cargo clippy`, targeted `cargo test`, the Qt suite, file-size, guard, and `openspec validate --all --strict`.
- [x] 5.3 Update `docs/dev/STATE.md` and `docs/dev/doc-code-parity-gaps.md` with `TASK-ALLOWS-DOCS`.
