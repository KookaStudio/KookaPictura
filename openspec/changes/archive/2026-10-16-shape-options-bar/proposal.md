# Proposal: shape-options-bar

## Why

Owner request (follow-up on #48, with a CS6 screenshot): the shape tools'
options bar should carry CS6's controls — Mode, Fill, Stroke (width and
type), W / link / H, the path operation / alignment / arrangement menus, the
geometry gear, the Shape picker, and Align Edges.

## What Changes

- `pictura_core::shape::align_edges`: straight horizontal / vertical edges
  snapped to the pixel grid (CS6's Align Edges; curves untouched).
- `pictura_render` `shape_style.rs` (new): `shape_fill` / `set_shape_fill`
  (a solid colour, or None as Fill opacity 0), `shape_stroke` /
  `set_shape_stroke` (an authored solid Stroke layer effect, `lfx2` `FrFX`,
  other effects kept), `shape_bounds`, `resize_shape` (about the top-left;
  a live shape stays live).
- `cxxqt_object/shapes.rs`: `ShapeSpec` gains Align Edges, no-fill, and the
  stroke; `shape_active`, `shape_set_active_fill`, `shape_set_active_stroke`,
  `shape_resize_active` ("Change Shape Fill", "Change Shape Stroke", "Resize
  Shape", labels approximated).
- `ShapeOptions` gains the appearance, the geometry gear (Unconstrained /
  Square, Circle, or Defined Proportions / Fixed Size, From Center; the
  Polygon's star and smoothing), Align Edges, and the mirrored active size;
  `ToolController::shapeOptionsChanged`.
- `tool_shape.cpp`: mirrors the active shape layer into the options and
  applies bar edits back to it; honours the geometry gear.
- `options_bar_shape.cpp`: rebuilt in CS6's order; the Custom Shape picker
  becomes a grid pop-up with a gear menu (Reset Shapes).
- Qt Test `tst_shape_tools::optionsBar`; psd-tools oracle
  `psd_tools_reads_a_shape_stroke_and_no_fill`.

## Capabilities

### New Capabilities

- `tools/shape-options-bar`: the shape tools' options bar.

## Impact

- `pictura-core`, `pictura-render`, `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Layout from the owner's CS6 screenshot and `docs/03-tools/shape-tools.md`.
Approximations (`ponytail:`): CS6 stores a vector shape's fill and stroke in
`vstk`; here Fill None is Fill opacity 0 and the stroke is a Stroke layer
effect (the two mechanisms the doc warns not to conflate — chosen because the
compositor already draws the effect along the vector mask). Not implemented
(listed but disabled): Gradient / Pattern fill and stroke, Dashed / Dotted
lines, path operations / alignment / arrangement, loading / saving shape
sets; widths are pixels, not points; the Proportional geometry option is
absent. The stroke Align default is Center, as the doc lists (unsourced).
