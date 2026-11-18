# Proposal

## Why

The control server can export a flattened PNG but not a WebP. That makes the
new lossy/lossless WebP path (#807) untestable through the real CLI/control
endpoint, and leaves agents unable to produce the format Export As now offers.

## What Changes

- Add `export_webp` to the control server: `path` is required, `quality`
  defaults to 90, and `lossless` defaults to false.
- It writes the flattened composite atomically, never changes the document's
  path, dirty state, or history, and is gated exactly like a save.
- Cover it with a Qt Test case and extend the live `scripts/verify-control.sh`
  recipe to check the `VP8 ` and `VP8L` chunks.

## Capabilities

### Modified Capabilities

- `interop/agentic-control`: gains the WebP export method and its save gating.

## Impact

- `control_server.{h,cpp}`, `control_server_commands.cpp`,
  `control_server_gate.cpp`, the Rust export bridge, `tst_control_editing`, and
  `scripts/verify-control.sh`.
