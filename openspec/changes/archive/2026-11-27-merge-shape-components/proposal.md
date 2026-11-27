## Why

Path Selection can move components individually, but users cannot consolidate overlapping contours in the active path (#532).

## What Changes

- Add **Merge Shape Components** under **Path Operations** for Path Selection.
- Normalize overlapping contours in the current editable path while preserving its effective fill, holes, and independent contours.
- Store a successful merge as one undoable path edit.

## Capabilities

### Modified Capabilities

- `tools/path-selection-tool` gains the Merge Shape Components behavior.

## Impact

The Path Selection options bar, active path bridge, reusable vector geometry, and Qt and Rust test sources change. The existing polygon overlay dependency is reused.
