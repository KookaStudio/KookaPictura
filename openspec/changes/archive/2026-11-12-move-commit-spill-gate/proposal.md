# Proposal

## Why

Issue #1290: on a large document (about 16000² px), a few Move tool drags in
a row end with "Could not move: not enough memory", while a Free Transform of
the same layer still commits. The Move tool's commit gate still charges
12 bytes a pixel of the changed region against RAM: a dense composite, its
sRGB copy and a display patch. Phases 3–4 (#1295) made that composite banded
and tiled and the sRGB copy lazy, and moved the Free Transform gate to RAM for
the working set plus scratch for the spillable output. The Move commit lands
through the same deferred region composite (`build_move_composite`), but its
gate was not updated. At 13000² the old charge is about 2 GB of RAM per move,
which runs out as history holds the moved tiles.

## What Changes

- The Move tool's layer commit is gated like a whole-pixel Free Transform:
  the composite's strip buffers must fit the governor's room, and the 4-byte
  tiled composite of the changed region must fit the room plus the free
  scratch-disk space.
- A Move refused because the scratch disks are full says so, as the
  transform refusal does.

The drag-start preview gate (16 bytes a pixel of the moving content) is
unchanged: its images are dense and held in RAM for the drag. The layer-mask
move gate is unchanged.

## Capabilities

- **MODIFIED** `document/out-of-core-transform`: ADDED requirement, the Move
  tool commit is gated on RAM plus scratch.

## Impact

- `pictura-app`: `impl_transform/group.rs` (`move_commit_cost`),
  `impl_transform/session.rs` (`commit_move`), `tool_move.cpp`
  (`moveMemoryRefusalMessage`).
