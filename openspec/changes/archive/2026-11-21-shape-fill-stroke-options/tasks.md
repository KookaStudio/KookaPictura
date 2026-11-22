# Tasks

## 1. Planning

- [x] 1.1 Write `openspec/changes/shape-fill-stroke-options/test-plan.md` with one row per spec scenario and a Coverage cell naming a test or a justified manual check.
- [x] 1.2 Validate the change with `openspec validate shape-fill-stroke-options --strict`.

## 2. Engine

- [x] 2.1 Add `ShapeFill`, `ShapeStrokeFill`, `StrokeLine`, and the extended `ShapeStroke`/`ShapeAttributes` to `pictura-render`'s shape appearance model; keep the solid-only helpers for existing callers.
- [x] 2.2 Encode/decode gradient and pattern fills and strokes through the existing `FillContent` and `FrFX` encoders; make Copy/Paste Shape Attributes transfer the full appearance.
- [x] 2.3 Add `DshP`/`Lntp` decoding and a vector-mask dash mask to the Stroke layer effect; render dashed and dotted bands.
- [x] 2.4 Add `proportion` to `pictura_core::shape::ShapeOptions` and use it in `outline`.
- [x] 2.5 Update shape thumbnails to render gradient and pattern fills.
- [x] 2.6 Add Rust tests for appearance round-trips, copy/paste, dashed rendering, and proportional geometry.

## 3. Bridge

- [x] 3.1 Extend `ShapeSpec` with fill/stroke kinds and payloads, stroke line, path operation, Fill Pixels mode/opacity/anti-alias, Snap To Pixels, proportion, and a flat outline payload.
- [x] 3.2 Build gradient/pattern fills and strokes in `shape_add_layer`; add style get/set bridge calls and `shape_active_style`.
- [x] 3.3 Apply Fill Pixels mode, opacity, and anti-alias in `shape_fill_pixels`.
- [x] 3.4 Add flat outline encode/decode helpers and the active-outline / target-outline replace bridge calls.
- [x] 3.5 Add Rust tests for Fill Pixels options and outline replacement.

## 4. Options bar and tools

- [x] 4.1 Add the appearance kinds/payloads, path operation, Fill Pixels fields, Snap To Pixels, and proportional ratio to `ShapeOptions` and its clamping.
- [x] 4.2 Enable the Gradient/Pattern fill and stroke pickers and the Dashed/Dotted line types in `options_bar_shape.cpp`; add the path operation menu, Fill Pixels controls, Snap To Pixels, and Proportional geometry.
- [x] 4.3 Mirror and restyle the full appearance through the new bridge calls in `tool_shape.cpp`.
- [x] 4.4 Perform the path operation combine in `tool_shape.cpp` using `QPainterPath` and the new outline bridge calls.
- [x] 4.5 Add the Qt tests `tst_shape_tools::fillStrokeOptions`, `::pathOperations`, `::fillPixelsOptions`, and `::geometryOptions`; update `::optionsBar` from disabled stubs to live controls.

## 5. Docs and integration

- [x] 5.1 Update `ROADMAP.md`, `PARITY.md`, and the TOOL-22 row in `docs/dev/doc-code-parity-gaps.md`.
- [x] 5.2 Run `python3 scripts/check-test-plan.py`.
- [x] 5.3 Run `cargo fmt --all`, clippy, nextest, and the Qt suites offscreen under the CPU budget.
- [x] 5.4 Run the fast-gate components (guard with `TASK_ALLOWS_DOCS=1`, parity, test plan, openspec) and the relevant Rust/Qt suites; the full nextest gate has an unrelated ImageMagick `ramp_Divide` oracle failure.
- [x] 5.5 Archive the change with `openspec archive shape-fill-stroke-options`.
