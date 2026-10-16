# Proposal: add-anchor-point-tool

## Why

The Add Anchor Point tool (issue #34) was catalogued but disabled. photorust's
`core/src/path.rs` splits a segment with de Casteljau; the port follows
`docs/03-tools/pen-and-path-tools.md`.

## What Changes

- `pictura_core::path`: `VectorPath::insert_anchor` splits a cubic at `t`
  keeping its shape (a straight segment stays straight), and `hit_segment`
  finds the nearest segment point.
- `cxxqt_object/paths.rs`: `path_insert_anchor`; one "Add Anchor Point" state.
- `tool_pen.cpp`: a click on a segment within 8 screen pixels.
- Qt Test `tst_pen_tools::anchorPointTools`.

## Capabilities

### New Capabilities

- `tools/add-anchor-point-tool`: the Add Anchor Point tool.

## Impact

- `pictura-core` (`path.rs`), `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's `core/src/path.rs`
(<https://github.com/perfecto25/photorust>). Behavioural parity only: no CS6 oracle exists for path geometry or
history labels. Ceiling (`ponytail:`): none beyond the Pen group's.
