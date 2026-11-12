# Proposal

## Why

Issue #1300: on a 17000 × 17000 document, a Gradient drag is refused with
"Could not fill: not enough memory" while the scratch disks have room. Free
Transform and Move commits on the same document succeed: they spill to
scratch and run behind a cancellable progress window (#1290, #1296).

The four fills (Gradient, Paint Bucket, Edit > Fill, Edit > Stroke) gate on
RAM alone through `memory_gate::admit`. Each charges the tiles it forks from
history and a 6-byte-a-pixel refresh. On a full 17000² layer, the Gradient
asks for about 1.16 GB of forked tiles plus 1.7 GB of refresh. That is more
than the governor's room on a 7.3 GB host. Yet the fill writes those tiles one
by one into tiled planes the evictor can pack or spill. And on a large RGB/8
document, a refresh that big is never composited densely: the region's display
tiles are only marked stale, and the visible ones are rebuilt within the
display budget. Every fill also runs on the GUI thread, with no progress and
no cancel.

## What Changes

- The four fills gate through `memory_gate::admit_spilling`. Their working
  set must fit the governor's room: flood masks for Paint Bucket and Stroke,
  and the dense refresh composite where the refresh builds one. Their forked
  tiles must fit the room plus the free scratch space.
- A dense refresh composite is charged only where the refresh builds one: an
  authoritative composite, a document of at most 16 Mpx, or a region of at
  most 4 Mpx.
- A fill refused because the scratch disks are full says so, as the Move and
  transform refusals do.
- On a document of 4 MP or more, an interactive fill of a pixel layer runs as
  a cancellable document job behind the progress window (busy bar). Cancel
  leaves the document and history unchanged. A small document, a mask or
  Quick Mask target, and a replayed or API call apply at once, as before.

## Capabilities

- **ADDED** `document/out-of-core-fill`: fills are gated on RAM plus scratch,
  and large-document fills run as cancellable jobs.

## Impact

- `pictura-app` Rust: `helpers/memory_gate.rs` (fill cost), `paint_tools/fills/imp.rs`
  (gates, job path), `impl_core/document_job.rs` (outcome rect), `picture.rs`
  (`sample_tiles` takes the stored-composite flag).
- `pictura-app` C++: `tool_fills.cpp`, `frame_menus_edit.cpp` (run the fills
  through `runDocumentCommand`; `fillMemoryRefusalMessage` names full scratch
  disks).
