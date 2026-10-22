# Proposal: Bound warp values read from files

## Why

Review of PR #843 found that `decode_type_warp` accepted any number from a
PSD's `TySh` warp descriptor, and that warped placement did not check every
bound for finiteness before sizing the layer plane. A corrupt or hostile file
could ask for an enormous allocation on open.

## What changes

- Decoded bend and distortions are clamped to CS6's ±100 %, and non-finite
  values read as 0.
- Warped placement requires all four bounds to be finite. It refuses a warped
  rect more than four times the unwarped text box's span (plus a small
  margin), so the type does not render and nothing is allocated for it.

## Capabilities

- Modified (added requirement): `tools/warp-text`.

## Impact

- `pictura-codec/src/type_tool.rs` and `pictura-render/src/type_warp.rs`, with
  their tests.
