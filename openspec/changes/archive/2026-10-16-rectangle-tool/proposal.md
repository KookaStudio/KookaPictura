# Proposal: rectangle-tool

## Why

The Rectangle tool (issue #43) was catalogued but disabled. photorust's
`core/src/shape.rs` turns a drag into a shape outline and lands it as a shape
layer, a path, or pixels; the port follows `docs/03-tools/shape-tools.md`.
This change also lands the engine and modes the Rounded Rectangle (#44),
Ellipse (#45), and Polygon (#46) changes share.

## What Changes

- `pictura_core::shape` (new): `outline` turns a drag into one closed
  `Subpath` of the Work Path model (curves stay cubic), Shift squaring the
  box by its longer side and Alt growing it from the press point; `coverage`
  rasterises an outline to an anti-aliased, even-odd, document-sized mask.
  `VectorPath::add_subpath` appends a finished outline; `Subpath::flatten`
  becomes public.
- `pictura_codec::encode_vector_mask` / `decode_vector_mask`: author a
  version-3 `vmsk` block from subpaths, and decode one.
- `pictura_render::add_shape_layer`: a solid-color fill layer named
  `"<Tool> N"`, cut to the outline by an authored `vmsk` (the legacy shape
  layer form CS6 reads), inserted above the active layer.
- `cxxqt_object/shapes.rs` (new bridge): `shape_outline` (the live preview),
  `shape_add_layer` (Shape), `shape_add_path` (Path), and
  `shape_fill_pixels` (Pixels, through the Paint Bucket's fill and the
  selection). Each records one `"<Tool> Tool"` state.
- `tool_shape.cpp` (new, shared by the four tools): drag to preview the
  outline (dashed), land it on release in the options bar's Mode; Shift and
  Alt are read live. A click draws nothing.
- `options_bar_shape.cpp` (new): Mode (Shape / Path / Pixels), shared by the
  four tools' pages; catalog row enabled.
- Qt Test `tst_shape_tools`; the self-test guard (98) now probes Line and
  `shift_plain` (117) presses R as the unimplemented key.

## Capabilities

### New Capabilities

- `tools/rectangle-tool`: the Rectangle tool and the shape tools' modes.

## Impact

- `pictura-core` (`shape.rs`, `path.rs`), `pictura-codec`
  (`vector_mask.rs`), `pictura-render` (`create.rs`), `pictura-app`
  (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/shape.rs` and its shell's
`CanvasView` shape drag (<https://github.com/perfecto25/photorust/blob/bab90b3305ec3675e7fce4b2d617905a10739241/core/src/shape.rs>). photorust flattened
outlines to points; here they stay cubic Beziers. Behavioural parity only: no
CS6 oracle exists for the drawn geometry or history labels (`"Rectangle Tool"`
is approximated); the authored `vmsk` is checked against psd-tools.
The click-to-create dialogs and live shapes are the `live-shapes` change.
Ceiling (`ponytail:`): no Stroke, gradient or pattern Fill, geometry pop-up (Fixed Size, Proportional,
Snap To Pixels), path operations (Shift / Alt at the
press do not combine), Align Edges, or Fill Pixels blend mode / opacity /
anti-alias toggle (Pixels always paints Normal, 100 %, anti-aliased).
