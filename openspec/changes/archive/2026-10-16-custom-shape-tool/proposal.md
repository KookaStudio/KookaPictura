# Proposal: custom-shape-tool

## Why

The Custom Shape tool (issue #48) was catalogued but disabled. photorust's
`core/src/shape.rs` generates a small built-in shape set and stretches the
chosen shape to the dragged box; the port follows
`docs/03-tools/custom-shape.md` and lands on the shape engine, bridge,
handler, and modes of the `rectangle-tool` change.

## What Changes

- `pictura_core::shape::custom` (new submodule): `CUSTOM_SHAPE_NAMES`
  (Star, Heart, Arrow, Cross, Lightning, Check, generated rather than Adobe's
  artwork), each stretched to the box; Shift keeps the shape's designed
  proportions, Alt draws from the centre; `custom_shape_preview` for the
  picker. `ShapeKind::CustomShape` (its layer is named "Shape N", as CS6 does),
  `ShapeOptions::custom`.
- `cxxqt_object/shapes.rs`: `ShapeSpec::custom`; `shape_custom_count`,
  `shape_custom_name`, `shape_custom_preview` (black silhouettes).
- `options_bar_shape.cpp`: the shape picker. A click opens "Create Custom
  Shape" (Width, Height, From Center), as in the `live-shapes` change.
- Qt Test `tst_shape_tools::lineAndCustomShape`.

## Capabilities

### New Capabilities

- `tools/custom-shape-tool`: the Custom Shape tool.

## Impact

- `pictura-core` (`shape/custom.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `CUSTOM_SHAPE_NAMES` / `unit_shape` / `heart`
(<https://github.com/perfecto25/photorust/blob/bab90b3305ec3675e7fce4b2d617905a10739241/core/src/shape.rs>). Ceiling (`ponytail:`): as `rectangle-tool`, plus a fixed six-shape
set: no `.csh` libraries, shape sets, Replace / Append / Reset, Define Custom
Shape, Preset Manager, or Defined Size; the shapes are polygons (the heart is
flattened to 48 corners); a Custom Shape is not a live shape.
