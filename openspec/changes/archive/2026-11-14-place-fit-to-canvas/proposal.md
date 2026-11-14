# Proposal

## Why

`File > Place` lands an oversized source at its native size, so most of it sits
outside the canvas. The Place Free Transform also materializes a channel-less
smart object on commit, discarding the embedded source that the user placed.
#755's multi-file Place shipped in #1118; this change keeps the placed source
intact while fitting it to the canvas.

## What Changes

- **Place creates the smart object first and fits it afterwards.** A PSD/PSB
  source keeps its original bytes; a raster source is converted at its native
  pixel dimensions. Only then does the Place command apply a uniform
  `min(1, canvas/source)` scale, centred on the canvas, with no upscale.
- **The fit is a transform, not a resample.** It changes the placed layer's
  transform/rect only. The embedded source keeps its original bytes and native
  pixel dimensions, so a later scale-up samples the original pixels. The fit is
  part of the same `"Place"` history state.
- **A raster Place releases its raster proxy after conversion.** The layer then
  renders from the embedded native-size source at the fitted rect.
- **A scale or translation in the Place Free Transform is non-destructive.** A
  later scale/translation on the placed smart object is too: the layer rect
  changes and the smart object, payload, and native source dimensions remain.
  Rotation, skew, and warp keep the existing rasterizing path.
- **The session starts fitted.** The Free Transform session begins on the
  fitted rect; an identity commit adds no state, a changed transform adds one
  `"Free Transform"` state.
- Not in scope: arbitrary non-destructive rotation/skew/warp, a stored
  smart-object transform renderer, PDF/AI vector Place, video Place, and
  changing the existing multi-file sequencing/cancel rules.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/smart-object-layer-actions`: oversized placements are fitted
  non-destructively; the fit belongs to the `"Place"` state; the raster route
  keeps its native embedded source.
- `document/free-transform`: the Place session starts from the fitted rect, and
  a scale or translation (during or after placement) preserves the placed smart
  object; rotation/skew/warp stay on the rasterizing path.
- `interop/image-import`: the raster Place route releases the raster proxy,
  embeds the native-size source, and preserves its merged alpha.

## Impact

- `crates/pictura-render/src/document_ops/layer_ops/smart_object.rs` and its
  module re-exports: fit and smart-rect transform helpers.
- `crates/pictura-app/src/cxxqt_object/impl_layers_smart_object.rs`: both Place
  bridges fit; the raster bridge releases the proxy.
- `crates/pictura-app/src/cxxqt_object/impl_transform/session.rs` and
  `impl_transform/smart_rect.rs`: the scale/translation commit path for a
  placed smart object.
- `crates/pictura-render/src/tests/smart_object_actions.rs` and a new
  `crates/pictura-app/cpp/tests/tst_place_fit.cpp` (plus the Qt test list).
- `docs/05-layers/smart-objects.md`, `docs/10-workflow-io/open-and-new.md`,
  `docs/03-tools/move-and-transform.md`, `docs/dev/STATE.md`, `ROADMAP.md`.
