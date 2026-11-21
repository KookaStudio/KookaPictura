# Proposal

## Why

Issue #546 closes parity row TOOL-22. The shape tools' options bar still shows
Gradient and Pattern fill/stroke, Dashed and Dotted lines, and the path
operation menu as disabled placeholders; Fill Pixels ignores CS6's Mode,
Opacity, and Anti-alias controls; Snap To Pixels and Proportional geometry are
absent. `docs/dev/doc-code-parity-gaps.md` records the gap as TOOL-22.

The renderer already models gradient and pattern fill content and gradient and
pattern stroke effects, so most of the work is exposing them through the shape
appearance model, the options bar, and the bridge rather than new raster code.
Dashed/dotted strokes and boolean path operations need new engine behaviour.

## What Changes

- Shape layers carry a full appearance: Fill None / Solid / Gradient / Pattern
  and Stroke None / Solid / Gradient / Pattern, with stroke width, Align, and
  line type Solid / Dashed / Dotted.
- The options bar enables the Gradient and Pattern fill/stroke pickers (reusing
  the existing Gradient Fill and Pattern Fill editors) and the Dashed/Dotted
  line types; it mirrors the active shape layer's appearance and restyles it
  with one history state per change.
- Copy/Paste Shape Attributes transfers gradient and pattern fills and strokes,
  and the dashed/dotted line type, not only solid colours.
- The Stroke layer effect decodes and renders a dash pattern from an authored
  descriptor; shape layers with a vector mask cut the stroke band into dashes or
  dots. Raster layers without a vector outline keep a solid stroke (ceiling).
- Path Operations gains New Shape, Combine Shapes, Subtract From Shape Area,
  Intersect Shape Areas, and Exclude Overlapping Shape Areas. Non-zero
  operations combine the newly drawn outline with the active shape layer's
  outline (Shape mode) or the Work Path (Path mode) through Qt's path boolean
  operators; the result becomes the target's outline in one "Path Operation"
  state.
- Fill Pixels gains Mode (the Paint Bucket's supported modes), Opacity, and
  Anti-aliased; the bar shows them only in Pixels mode.
- The geometry gear gains Snap To Pixels for Rectangle and Rounded Rectangle
  and a Proportional mode with a W:H ratio for the box tools.
- `ROADMAP.md`, `PARITY.md`, and the parity register move together.

## Capabilities

### New Capabilities

- `tools/shape-fill-stroke-options`: gradient/pattern fill and stroke, stroke
  line types, appearance mirroring, and attribute copy/paste.
- `tools/shape-path-operations`: the path operation menu and boolean combining
  of shape and path components.
- `tools/shape-fill-pixels-options`: Fill Pixels Mode, Opacity, and Anti-alias.
- `tools/shape-geometry-options`: Snap To Pixels and Proportional geometry.

### Modified Capabilities

- `tools/shape-options-bar`: the bar's disabled placeholders become live
  controls and the mirror/restyle contract extends to the new appearance.

## Impact

- `crates/pictura-core`: shape options gain a proportional ratio.
- `crates/pictura-render`: the shape appearance model (`shape_style`), stroke
  effect decoding/rendering (`layer_effects/strokes`), and shape thumbnails.
- `crates/pictura-app`: the `ShapeSpec`/`ShapeOptions` bridge, the shape and
  path bridge functions for outlines and appearance, `options_bar_shape.cpp`,
  and `tool_shape.cpp`.
- Qt tests: `tst_shape_tools` gains fill/stroke, path-operation, fill-pixels,
  and geometry cases; the existing options-bar case changes from disabled
  stubs to live controls.
- Rust tests: `pictura-core::shape`, `pictura-render::shape_style`, and
  `pictura-render::layer_effects` cover proportional geometry, appearance
  round-trips, attribute copy/paste, and dashed rendering.
- No new dependencies: path booleans use the Qt `QPainterPath` already linked
  into the shell.
