# Design: transform-options-bar

## Context

The transform session (`cxxqt_object/impl_transform/`) is a similarity map
`p' = c + R(θ)·S·(p − c) + (dx, dy)` about the source rect centre `c`, with a
projective quad mode. The engine resamples bilinearly. The options bar builds
one stacked page per `ToolId`; no page exists for a session, and the session
begin/end path emits nothing.

## Goals / non-goals

- Goal: a CS6-shaped options bar over the live session, with a movable pivot and
  numeric entry that commit through the engine.
- Goal: an interpolation choice that actually changes committed pixels.
- Non-goal: the interactive warp cage, Warp Text, Puppet Warp, and
  Content-Aware Scale (existing ceilings).

## Key decisions

### Interpolation lives in the engine, defaults stay bilinear

`pictura_render::Interpolation` (`nearest`, `bilinear`, `bicubic`,
`bicubic-smoother`, `bicubic-sharper`, `bicubic-automatic`) is a new
`layer_ops/kernel.rs`. The kernels are Mitchell–Netravali cubics:
Bicubic = Catmull–Rom (B=0, C=1/2, interpolating, so integer translations stay
byte-exact), Bicubic Smoother = B-spline (B=1, C=0), Bicubic Sharper =
(B=0, C=3/4). Bicubic Automatic resolves per op from the destination/source
area ratio: smaller → Sharper, larger → Smoother. Adobe's exact kernels are
closed, so these are `(inferred)`.

The existing `transform_layer`, `transform_layer_quad`, and
`transform_layer_warp` keep bilinear (their canonical requirements do not
change); new `_with` variants take the interpolation. The session defaults to
Bicubic (CS6's default), so the app gains the choice without any engine
behavior change elsewhere.

### The pivot is a parameter anchor, not a fixed point of the map

The session stores `pivot` (document space), `show_pivot`, and `skew_x/skew_y`
(radians). The map is `p' = P + A·(p − P) + t` with `A = R(θ)·S·K`,
`K = [[1, tan(skew_x)], [tan(skew_y), 1]]`, and `t` the stored `(dx, dy)`.

- Moving the pivot keeps the image still: `t` is compensated by
  `(P − A·P) − (P' − A·P')`.
- Numeric setters are absolute on the scalars and keep the pivot's *current
  image point* `M(P) = P + t` fixed (so the pixels under the pivot do not
  move): scale multiplies `A` and `t`; rotation pre-multiplies `A` and `t`;
  skew replaces `K` and leaves `t`.
- The rotate gesture rotates about `M(P)` (same formula).
- Handle drags keep the opposite-handle anchor (unchanged) and never move the
  pivot.
- A pivot drag never changes the map (compensation), only `P`.

`LayerTransform` is unchanged. A sheared similarity is committed through
`transform_layer_quad_with` (the affine map's source corners), so no engine
struct change is needed; the type-layer affine path already handles affine
quads. The C++ preview always uses `transform_preview_matrix` (now returned for
every mode, not just projective), so the preview never re-derives the map.

### Readouts in the projective modes

`scale`/`angle`/`skew` readouts come from the stored scalars in `Free` and are
derived from the live quad in a projective mode: `W%`/`H%` from the top/left
edge lengths over the source rect, `angle` from the mean edge rotation, `H`
from the left edge's deviation from vertical, `V` from the top edge's deviation
from horizontal (minus the rotation). W/H setters are disabled in the
projective modes (angle/H/V setters rotate/shear the quad about the pivot
instead). `(inferred)`; the derivation is a readout, not state.

### In-session warp

The session stores an optional preset warp (`style_id`, `bend`, `distort_x`,
`distort_y`, `vertical`) plus the mesh built for the current rect. The warp
button opens the existing `WarpPresetDialog`; `None`/`Custom` clears the warp.
With a warp set, the Rust side computes the preview on a document clone
(apply the live transform, then the mesh) and hands the warped image to the
canvas as the preview layer; box gestures are suspended (`transform_press`
returns -1) so the preview cannot go stale. The commit applies the similarity/
projective op first and the warp second, recording one `"Free Transform"` state;
a clone-based rollback keeps Escape bit-identical. The mesh is rebuilt for the
post-transform rect at commit.

### Options bar

`OptionsBar` gains a transform page appended after the per-tool pages
(`showTransform()` / `showTool()`), built in `options_bar_transform.cpp`. The
page reads the session through new `PictureView` getters and writes through
setters; the relative-positioning toggle and the W/H link are page-local state.
`ToolController::transformChanged` fires on begin/commit/cancel, after each
gesture move, and after a pivot press; the frame switches the bar to the
transform page while `transformSessionActive()` and back to the active tool's
page when it ends.

The 3×3 locator is nine named `QToolButton`s in a grid (testable by object
name); Cancel/Apply reuse `crop.cancel`/`crop.apply`; the show-pivot toggle uses
`info.crosshair`; the warp button gets a new custom Lucide-style glyph
(`options.warp`).

## Risks

- Existing app tests that commit transforms may compare pixels; the session
  default changes bilinear → Bicubic. Catmull–Rom is interpolating, so integer
  moves stay exact; any golden that fails must be reviewed, not weakened.
- The pivot is document-space and never follows the content, unlike CS6's
  bbox-relative marker. Documented as inferred.
