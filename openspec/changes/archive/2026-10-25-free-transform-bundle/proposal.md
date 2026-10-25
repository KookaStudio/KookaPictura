# Proposal

## Why

Three Free Transform defects are open from the manual pass for #854: Ctrl-drag
does not reshape a corner (#901), switching between the box and Warp discards
one mode's edits (#902), and the reference point stops tracking the box after a
move or scale (#903). They share the same session state and gesture code, so
fixing them together avoids three overlapping passes over `impl_transform/`.

## What Changes

- **Ctrl-drag gestures in a Free session** (#901). `Ctrl` + drag a corner
  distorts that corner freely; `Ctrl+Shift` + drag a side handle skews the
  edge. `Ctrl+Alt+Shift` + corner keeps its existing perspective behavior.
  The gestures use the session's transient projective base, so later plain
  drags scale and rotate the reshaped box and one commit records one
  `Free Transform` state. On editable type the distort shortcut falls back to
  an ordinary scale; the affine skew keeps the text as type.
- **Warp and box edits coexist** (#902). Leaving Warp keeps the mesh in the
  session instead of dropping it, re-entering Warp reuses that mesh, and the
  box stays editable with the warp applied. Leaving also moves the box onto the
  warped preview's bounds without changing the preview pixels, so the box and
  grid follow the warp in both directions. Every box mutation — canvas drags,
  numeric W/H/angle/skew, exact rotate/flip — refreshes the warped preview.
  Cancel, Escape, and Undo/Redo clear the preview and grid; commit still applies
  the mesh then the box frame as one state.
- **The reference point follows the box** (#903). The session holds the
  reference point in the item's source frame and displays it at its mapped
  document position, so it moves with a translation or an off-anchor scale and
  stays at the rotation centre during a rotation. `Transform Again` records
  the source-frame pivot offset, so the automatic carry does not shift the
  repeated result.
- **BREAKING (behavioral):** the Transform Again reference-point offset now
  means the pivot's place in the source frame, not its post-transform document
  position. A transform that automatically carried the pivot no longer shifts
  the repeated transform.

Excluded from this bundle:

- #904 — assigned to another contributor.
- #905 — already covered by the pending-transform lock; the Layer menu and
  Layers panel are gated while a session is open.
- #832 — the Move tool's live preview, not Free Transform.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/free-transform`: adds the transient Ctrl distort and
  `Ctrl+Shift` skew shortcuts, with their type and commit rules.
- `document/transform-reference-point`: redefines the pivot as a source-frame
  point, its document-position readout, the inverse setter, and the carry
  through move/scale.
- `document/transform-session-warp`: keeps the warp across box/Warp switches,
  reuses the mesh on re-entry, and requires box mutations to refresh the
  warped preview.
- `tools/transform-commands`: makes Transform Again repeat the source-frame
  pivot offset.

## Impact

Code:

- `crates/pictura-app/src/cxxqt_object/impl_transform/` — `session.rs`
  (begin/commit/repeat logic), `geometry.rs` (pivot mapping and inverse setter),
  `dispatch.rs` (gesture routing and re-warp), `distort.rs`, `skew.rs`,
  `perspective.rs` (pure quad helpers and base-quad unmapping), `bridge.rs`
  (new begin/read accessors, pivot readouts), `warp.rs` (retain instead of
  drop; `transform_warp_stored`).
- `crates/pictura-app/src/cxxqt_object/document_session/transform.rs` —
  session fields and their documentation.
- `crates/pictura-app/cpp/tools.cpp` — modifier precedence on press.
- `crates/pictura-app/cpp/tool_transform.cpp` — preview selection for a
  stored warp; calls the new warp accessor.
- `crates/pictura-app/cpp/options_bar_transform.cpp` — unchanged behavior;
  verified against the new accessor.

Tests:

- Rust unit tests in `impl_transform/` for the pivot inverse, the base-scale
  pivot carry, and the repeat offset; the existing gesture tests cover the
  factored distort/skew quad helpers.
- Qt Test additions to `tst_transform_distort`, `tst_transform_skew`,
  `tst_transform_options`, `tst_transform_warp`, and `tst_transform_commands`.

No new dependencies. No document or file-format change. The preview re-warp on
box edits inherits the existing full-resolution warp-preview cost.
