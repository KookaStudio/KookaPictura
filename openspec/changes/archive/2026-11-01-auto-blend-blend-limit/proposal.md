# Proposal

## Why

PR review of #583: the merged blend needs full-resolution pyramids, so a very large
domain implied multi-GB allocations with no guard, unlike the capped mask analysis.

## What Changes

Above 100 MP a run adds its masks and no merged layer, and tells the user.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/auto-blend-layers`: a size limit on the merged layer.

## Impact

`crates/pictura-render/src/document_ops/layer_ops/auto_blend/`,
`crates/pictura-app/src/cxxqt_object/auto_blend/imp.rs`, `frame_menus_align.cpp`.
