# Proposal: freeform-pen-tool

## Why

The Freeform Pen tool (issue #33) was catalogued but disabled. photorust's
`core/src/path.rs` reduces a freehand drag to anchors with Douglas-Peucker; the
port follows `docs/03-tools/pen-and-path-tools.md`.

## What Changes

- `pictura_core::path`: `simplify_freehand` and `VectorPath::add_freeform`
  (a new subpath of corner anchors within the Curve Fit tolerance, closed when
  asked).
- `cxxqt_object/paths.rs`: `path_add_freeform`; one "Freeform Pen" state.
- `tool_pen.cpp`: the drag's live trail; ending within 8 screen pixels of the
  start closes the subpath.
- `options_bar_pen.cpp`: Curve Fit (0.5-10 px, 2 px).
- Qt Test `tst_pen_tools::freeformPenTool`.

## Capabilities

### New Capabilities

- `tools/freeform-pen-tool`: the Freeform Pen tool.

## Impact

- `pictura-core` (`path.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/path.rs`
(<https://github.com/perfecto25/photorust>). Behavioural parity only: no CS6 oracle exists for path geometry or
history labels. Ceiling (`ponytail:`): every kept point is a corner (CS6 fits curves), no
Magnetic option, and no continuing from an endpoint.
