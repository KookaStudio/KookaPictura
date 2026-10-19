# Proposal

## Why

The Background is documented as always bottom-most
(`docs/05-layers/layers-overview.md:55-57`) and is defined as never a clipping
base by Photoshop's behavior and issue #717. The engine enforces only part of
it. Another layer can be moved or arranged beneath the Background, and a layer
directly above it can be clipped to it or receive clipping from the New Layer
dialog. The result is a document whose stacking and clipping model contradicts
both Photoshop and the project's own contract.

## What Changes

- Enforce a Background floor in every within-container reorder:
  `move_path` clamps a delta move at the Background (and refuses a source already
  below it), `move_path_to` / `can_move_path_to` refuse a destination at or
  below it, and `arrange_target` clamps Send Backward / Send to Back to the
  position directly above it.
- Refuse a second Background: `Background From Layer` requires that the document
  has no Background yet.
- Reject a Layers-panel drop below the Background, so the drag shows no drop
  indicator and the document and history stay unchanged.
- Refuse to create a clipping mask when the resolved base is the Background:
  the menu, Alt-click, and Properties-panel toggle all route through the same
  engine predicate.
- Drop a requested clipping flag when the New Layer dialog would place the new
  layer directly above a Background with no other legal base.
- Release a clipped run that an edit (Reverse, ungroup, grouping, Background
  conversion) leaves directly above a Background.
- Align rename with the documented rule: renaming a layer to `Background` does
  not set the Background flag (the codec still derives it from the bottom layer
  on read).
- Keep PSD round-trip behavior unchanged: the codec continues to read and write
  the clipping byte verbatim. No load-time normalization is added.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/layer-management`: the Arrange requirement stops Send to Back at
  the Background floor, and a new requirement states the Background stays
  bottom-most for moves, drops, and Arrange and that only one Background exists
  per document.
- `compositing/clipping-masks`: creating a clipping mask refuses a Background
  base, and stack edits (including grouping and ungrouping) do not leave a layer
  clipped to a Background.
- `ui/layers-panel`: the drag-and-drop requirement names a drop below the
  Background as an invalid target with no drop indicator, and Move Layer
  Up/Down refuses a move below the Background.

## Impact

- `crates/pictura-render/src/document_ops/layer_ops/properties.rs` — background
  floor for `move_path`, `move_path_to_dest`, `arrange_target`; rename no longer
  sets the Background flag; grouping normalizes clipping.
- `crates/pictura-render/src/document_ops/layer_ops/clipping.rs` — base guard
  for `can_create_clipping_mask`, Background-run release in
  `normalize_clipping`.
- `crates/pictura-render/src/document_ops/layer_ops/create.rs` — `insert_node`
  drops a requested clip with no legal base; `group_layer` normalizes; the
  second-Background refusal.
- Rust tests in `tests.rs`, `clipping.rs`, and the existing rename test; Qt
  coverage in `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp` and
  `tst_command_tree.cpp`.
- No new dependencies.
