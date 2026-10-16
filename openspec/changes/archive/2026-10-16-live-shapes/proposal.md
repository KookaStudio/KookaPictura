# Proposal: live-shapes

## Why

Requested by the project owner (follow-up on issues #43–#45, with Photoshop
CC 2015 screenshots as the reference): a drawn rectangle, rounded rectangle,
or ellipse should show its draggable corners; reshaping one with Direct
Selection should first ask to "turn a live shape into a regular path"; a
click (not a drag) should open the tool's Create dialog; and a shape layer
should carry the shape badge in the Layers panel.

**Scope note.** Live shapes, per-corner radii, and the Create dialogs as shown
are Photoshop CC features. `docs/03-tools/shape-tools.md` places live-shape
editing outside CS6 parity scope; this change adds them deliberately, at the
owner's request, without changing that document. A PSD written here stays
readable by CS6, which keeps the extra `vogk` block opaque.

## What Changes

- `pictura_core::shape`: `ShapeOptions` gains per-corner `radii`, a `star`
  indent, and `smooth_corners` / `smooth_indents`; `outline_in_box` places a
  shape in a given box (the Polygon upright, stretched to the box).
  Smoothing uses Catmull-Rom tangents: an approximation, Adobe's is
  unpublished.
- `pictura_codec`: `encode_live_shape` / `decode_live_shape` author and read a
  `vogk` (vector origination) block, layout inferred from psd-tools and ag-psd
  (`keyOriginType` 1 rectangle, 2 rounded rectangle, 5 ellipse; box, radii,
  corners, identity `Trnf`); `decode_vector_mask_paths` reads a `vmsk` back as
  editable Bezier subpaths.
- `pictura_render` `shape_layer.rs`: `add_shape_layer` takes the live origin;
  `is_shape_layer`, `layer_shape_paths` / `set_layer_shape_paths`,
  `layer_live_shape` / `set_layer_live_shape`, `shape_fill_color`,
  `shape_coverage`.
- `cxxqt_object/paths.rs`: `path_set_layer_target` points the path calls at
  the active shape layer's outline (Path Selection, Direct Selection, the
  shape tools) or the Work Path (the Pen group, the Paths panel's Work Path
  row). An edit there rewrites the `vmsk` and recomposites; a whole-component
  move keeps the shape live (its box moves too), any other edit makes it a
  regular path. `path_target_is_live_shape`, `path_convert_live_shape` (no
  history of its own: undoing the edit that follows restores the live shape).
- `cxxqt_object/shapes.rs`: one shared `ShapeSpec` struct (a drag or a boxed
  placement); Rectangle / Rounded Rectangle / Ellipse layers are live,
  Polygons are not (as in CC 2015); `shape_row_is_shape`,
  `shape_row_is_live`, `shape_row_thumbnail`.
- `shape_dialogs.{h,cpp}` (new): the prompt (Yes / No, "Don't show again"
  saved as `confirmLiveShapeToPath` in the session store) and the Create
  Rectangle / Rounded Rectangle (four radii) / Ellipse / Polygon (sides,
  Smooth Corners, Star, Indent Sides By, Smooth Indents) dialogs, each with
  Width, Height, and From Center.
- `tool_shape.cpp`: a click opens the Create dialog (seeded with the last
  shape's size); outside Path mode the active shape layer's outline is drawn
  with its anchors. `tool_path_selection.cpp`: targets the shape layer and
  prompts before an anchor or handle drag on a live shape.
- Layers panel: shape rows get a rendered thumbnail and the `layers.kindShape`
  badge on its corner (no fx badge). Paths panel: "<Layer> Shape Path" above
  the Work Path when a shape layer is active.
- Qt Test `tst_shape_tools::liveShapes` and `::createDialogs`; psd-tools
  oracle `psd_tools_reads_an_authored_live_shape`.

## Capabilities

### New Capabilities

- `tools/live-shapes`: live shape layers, their outline on the canvas, and the
  conversion prompt.
- `tools/create-shape-dialog`: the click-to-create dialogs.
- `ui/layers-shape-badge`: the Layers panel's shape-layer thumbnail and badge.

## Impact

- `pictura-core`, `pictura-codec`, `pictura-render`, `pictura-app` (bridge,
  C++, session store).
- No new dependency.

## Provenance

Behaviour from the owner's Photoshop CC 2015 screenshots; no photorust source.
The `vogk` layout is inferred from what psd-tools and ag-psd read (both decode
the authored block); no Adobe reader has been checked. Ceiling (`ponytail:`):
no Properties-panel live-shape editing (W / H / radii after drawing), the
prompt does not appear for Path Selection's Alt-drag copy or Delete (they
convert silently), a converted drag ends at the prompt (it is modal), a live
reshape recomposites the whole document on every move, and dragging an anchor
needs Direct Selection (the shape tools only show the anchors).
