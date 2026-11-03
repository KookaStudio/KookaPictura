# Proposal

## Why

Issue #1149. On a 16507×16196 document a Paint Bucket click that fills an empty
layer and its undo were slow. Besides the paint engine (its own change,
`paint-bucket-fill-responsiveness`), `bucket_fill_profile` at 1/16 of the area
showed two shared costs: the region composite (285 ms) blended a background the
now-opaque layer hides, and the history capture (77 ms) and undo (79 ms) copied
every changed tile twice.

## What Changes

- **A band of the CPU composite starts at the topmost plain opaque Normal
  layer** over it when everything below is provably opaque, and **copies that
  layer's bytes** when nothing is shown above it. The output is byte-identical.
- **A whole-plane change moves through the history by sharing.** A capture with
  no hint shares the live plane as the current state's instead of copying the
  changed tiles in; an undo or redo shares the restored plane with the live
  document instead of writing the tiles into it a second time; the tile swap
  runs one band per core.

## Capabilities

### New Capabilities

- `compositing/occluded-band-skip`

### Modified Capabilities

- `document/in-place-history`: a capture with no hint shares the changed plane
  with the live document, and an in-place undo shares the restored plane.

## Impact

`pictura-render`'s banded compositor (`composite_region.rs`, two `BlendCtx` /
`SourcePlanes` accessors) and the app history (`history/planes.rs`). Every
document whose top layers are opaque composites faster, and every unhinted
capture and in-place undo of a changed plane copies less.
