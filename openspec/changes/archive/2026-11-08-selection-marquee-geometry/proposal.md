# Proposal: selection-marquee-geometry

## Why

Issue #275. Pure selection geometry still lives in the C++ shell with no Rust
test surface: the marquee rect math (`dragRect`, `marqueeDragRect`), the
polygon close-radius hit test duplicated in the Polygonal and Magnetic Lasso,
and `clipSelectionShapeToCanvas`. The engine already owns the selection model
and `pictura-core::shape::drag_rect`, so the math can move without changing
behaviour.

## What Changes

- Move the marquee rect math into `pictura-select` and return an integer rect.
  Normal, Shift, and mirror reuse `pictura-core::shape::drag_rect`; Fixed Ratio
  keeps the deleted C++ branch so its rounding stays exact; Fixed Size centres
  on the mousedown.
- Move the polygon close-radius hit test into the same module as
  `within_radius(a, b, radius)` and use it from both the Polygonal Lasso and
  the Magnetic Lasso (the Magnetic Lasso passes `radius / zoom`).
- Keep `clipSelectionShapeToCanvas` in C++ for this change: it needs a real
  even-odd polygon boolean, and no boolean-geometry crate is in the workspace.
  File a follow-up for the port.
- Keep the Qt event, modifier, style, and cursor policy in C++.
- Add the missing Rust unit tests: Normal, Shift, mirror, Fixed Ratio
  (portrait, landscape, and half-pixel), Fixed Size centring, the close radius,
  and the style/ratio/negative-radius guards.
- No behaviour change; the Qt tests and `tsc_marquee_geometry` stay as parity
  checks.

## Capabilities

No spec delta: this is an internal refactor with no requirement change
(`skip_specs: true`).

### New Capabilities

None.

### Modified Capabilities

None.

## Impact

- `crates/pictura-select/src/geometry.rs` (new) and its unit tests.
- `crates/pictura-core/src/shape.rs` — `drag_rect` becomes `pub`.
- `crates/pictura-app/src/cxxqt_object/impl_selection/bridge.rs` (the selection bridge).
- `crates/pictura-app/cpp/selection_geometry.{h,cpp}` — thin Qt adapters;
  `dragRect` is deleted (no external callers).
- `crates/pictura-app/cpp/tool_selection.cpp` and
  `crates/pictura-app/cpp/tool_magneticlasso.cpp` — call `within_radius`.
- No new dependency; no public API removal.

## Provenance

Independent refactor; no Adobe behaviour is involved.
