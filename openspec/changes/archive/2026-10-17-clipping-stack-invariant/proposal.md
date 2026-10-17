# Proposal

## Why

Issue #658: a clipping mask is a per-layer flag whose meaning is positional — a
clipped layer is only valid when it sits in a contiguous run above a non-clipped
base. Reordering the stack never touched the flag, so a clipped layer dragged out
of its run kept the clipping arrow and either lost its base or silently attached
to another layer below it.

## What Changes

- Treat clipping membership as a stack invariant, not a sticky flag:
  - a layer dropped inside a clip run joins it;
  - a clipped layer that leaves its run (below the base, above a non-member, or
    at the bottom of its container) is released;
  - a run loses its base when that base is moved, deleted, or converted to
    Background — the run is released, not silently re-based;
  - released layers stay released when reordered back; they re-join only by being
    dropped inside a run again.
- Apply the normalization to drag/reparent, Layer > Arrange and keyboard reorder,
  delete, Reverse, ungroup, and Background conversion.
- Clear a requested clipping flag when a node is inserted at the bottom of a
  container (for example, New Layer with "Use Previous Layer to Create Clipping
  Mask" targeting an empty group), so the same invariant holds on that path.
- No new UI: the Layers panel already derives the indent, arrow glyph, and base
  underline from the flag, so correcting the state corrects the display.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/clipping-masks`: add requirements that clipping membership follows
  stack position after a reorder and that stack edits never leave a baseless
  clipped layer.

## Impact

- `crates/pictura-render/src/document_ops/layer_ops/clipping.rs` — normalization
  helpers and unit tests.
- `crates/pictura-render/src/document_ops/layer_ops/properties.rs` — reorder,
  delete, reverse, and ungroup call sites.
- `crates/pictura-render/src/document_ops/layer_ops/create.rs` — Background
  conversion, group ungrouping, and the bottom-insertion guard.
- `crates/pictura-app/src/cxxqt_object/impl_layers.rs` — route the legacy
  top-level `move_layer` bridge through `move_path` so it cannot bypass the
  invariant.
- `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp` — Qt Test cases for
  the drag-out repro and the legacy bridge.
- No dependency changes. Undo/history is unchanged: normalization rides the
  existing move/delete history state.
- PSD I/O is unchanged; the flag already round-trips as the PSD clipping byte.
