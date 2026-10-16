# Proposal: vertical-type-mask-tool

## Why

The Vertical Type Mask tool (issue #40) was catalogued but disabled. It is the
Horizontal Type Mask tool with the text set in columns
(`docs/03-tools/type-tools.md`).

## What Changes

- `type_commit_mask` records one "Vertical Type Mask" state for vertical text;
  the coverage comes from the Vertical Type tool's column layout.
- `tool_type.cpp`: the vertical mask session.
- Qt Test `tst_type_tools::typeMaskTools` (the vertical half).

## Capabilities

### New Capabilities

- `tools/vertical-type-mask-tool`: the Vertical Type Mask tool.

## Impact

- `pictura-app` (bridge, C++).
- No new dependency.

## Provenance

Ported from photorust's type entry (<https://github.com/perfecto25/photorust>).
Behavioural parity only. Ceiling (`ponytail:`): the Vertical Type and Horizontal
Type Mask tools' ceilings.
