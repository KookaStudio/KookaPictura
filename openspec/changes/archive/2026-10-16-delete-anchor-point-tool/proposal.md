# Proposal: delete-anchor-point-tool

## Why

The Delete Anchor Point tool (issue #35) was catalogued but disabled.
photorust's `core/src/path.rs` removes an anchor; the port follows
`docs/03-tools/pen-and-path-tools.md`.

## What Changes

- `pictura_core::path`: `VectorPath::delete_anchor`; a subpath left with one
  point reopens, and an emptied one is removed.
- `cxxqt_object/paths.rs`: `path_delete_anchor`; one "Delete Anchor Point" state.
- `tool_pen.cpp`: a click on an anchor within 8 screen pixels.
- Qt Test `tst_pen_tools::anchorPointTools`.

## Capabilities

### New Capabilities

- `tools/delete-anchor-point-tool`: the Delete Anchor Point tool.

## Impact

- `pictura-core` (`path.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/path.rs`
(<https://github.com/perfecto25/photorust>). Behavioural parity only: no CS6 oracle exists for path geometry or
history labels. Ceiling (`ponytail:`): the neighbouring segments are not refitted to the
removed anchor's curve.
