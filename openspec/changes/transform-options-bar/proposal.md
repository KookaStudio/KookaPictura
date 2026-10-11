# Proposal: transform-options-bar

## Why

Issue #296 (extracted from the #252 UX pass): CS6's Free Transform options bar
is missing. The transform session holds only the gesture scalars and quad; it
has no reference point, no numeric entry, no interpolation choice, and no
in-session warp state, and the options bar is keyed to the active tool, so
nothing changes when `Ctrl+T` opens a session. A follow-up UX finding (#296
comment) adds that the pivot must also be draggable on the canvas with snapping
to the nine anchors.

## What Changes

- **Reference point**: the session gains a document-space pivot (default: the
  source rect centre), a show flag, the 3×3 preset anchors of the live quad,
  canvas dragging with snap to those anchors, and numeric X/Y (absolute or
  relative to the session origin). Gestures and numeric setters anchor on the
  pivot: rotate about it, W/H/angle/H/V about it.
- **H/V skew**: numeric horizontal/vertical skew (shear) on the similarity
  state, committed as the equivalent affine quad; readouts derive from the quad
  in the projective modes.
- **Interpolation**: the six CS6 resample methods (Nearest Neighbor, Bilinear,
  Bicubic, Bicubic Smoother, Bicubic Sharper, Bicubic Automatic) as a new
  `pictura_render::Interpolation`, with `transform_layer_with`,
  `transform_layer_quad_with`, and `transform_layer_warp_with` variants; the
  existing ops keep bilinear so nothing else changes.
- **In-session warp**: a preset warp (reusing `WarpPresetDialog`) becomes
  session state; the commit applies the transform then the warp as one
  `"Free Transform"` state, with a warped live preview; box gestures are
  suspended while a warp is set (the interactive cage is out of scope).
- **Options bar**: a transform page (tool icon, show-pivot toggle, 3×3 locator,
  X / relative toggle / Y, W / link / H, angle, H/V, interpolation, warp,
  Cancel, Apply) shown while a session is active regardless of the active tool,
  driven by a new `ToolController::transformChanged` signal.

## Capabilities

### New Capabilities

- `document/transform-reference-point`: pivot state, presets, drag+snap, X/Y
  semantics, and pivot-anchored gestures and numeric setters.
- `document/transform-interpolation`: the resample method set and the
  interpolation-parameterised transform ops.
- `document/transform-session-warp`: the in-session preset warp state, preview,
  and commit contract.
- `ui/transform-options-bar`: the session-keyed options page and its signal.

### Modified Capabilities

- `document/free-transform`: the reference point is the movable pivot (rotate
  about the pivot, pivot hit-testing), replacing the fixed layer centre.

## Impact

- `pictura-render` (`layer_ops/kernel.rs`, `transform.rs`, `warp.rs`,
  `transform_native.rs`), `pictura-app` (transform session, `ToolController`,
  `OptionsBar`, `ImageView`), and `crates/pictura-app/cpp/tests/`. No new
  dependency.
- Ceilings (`ponytail:`): the bicubic kernels are inferred (Adobe's are
  closed); the H/V readouts in the projective modes are derived, not stored;
  W/H setters are Free-mode only; the interactive warp cage and Warp Text stay
  out of scope.
