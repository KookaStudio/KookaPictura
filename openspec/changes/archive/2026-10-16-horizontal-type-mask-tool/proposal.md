# Proposal: horizontal-type-mask-tool

## Why

The Horizontal Type Mask tool (issue #39) was catalogued but disabled. In CS6 it
types into a red quick-mask and commits a selection in the shape of the type
instead of a type layer (`docs/03-tools/type-tools.md`).

## What Changes

- `pictura_render::type_mask`: the type's coverage on a document-sized plane,
  clipped to the canvas; anti-aliasing None thresholds it.
- `cxxqt_object/type_tools.rs`: `type_commit_mask` merges it into the selection
  by the mode captured at the click (Shift adds, Alt subtracts) and records one
  "Horizontal Type Mask" state (`apply_selection_labeled`).
- `tool_type.cpp`: the mask session — the canvas is tinted red with the letters
  cut out while typing.
- Qt Test `tst_type_tools::typeMaskTools`.

## Capabilities

### New Capabilities

- `tools/horizontal-type-mask-tool`: the Horizontal Type Mask tool.

## Impact

- `pictura-render`, `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's type entry (<https://github.com/perfecto25/photorust>).
Behavioural parity only: no CS6 oracle exists for the mask's coverage or the
history label. Ceiling (`ponytail:`): the tint covers the canvas rather than only
the active layer, plus the Horizontal Type tool's ceilings.
