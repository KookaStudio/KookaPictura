# Proposal

## Why

`Document::clone` is documented as a refcount bump, and the history is documented
as retaining memory proportional to the edited area, but only `Plane<u8>` fields
actually share. Native 16/32-bit sample stores, smart-object payloads, the raw
`lnk2`/`lnk3` linked-section records in `Document.layer_section_extra` (which
carry the same embedded PSB bytes as `SmartObject.payload`), preserved byte
blocks, the layer tree, and the selection are plain `Vec`s, so every history
capture, every clipping band, and every smart-object transform check deep-copies
document-sized buffers. On 16/32-bit or smart-object documents this can reach
gigabytes of retained history and hundreds of milliseconds per commit on the GUI
thread.

## What Changes

- Store `Samples::{U8,U16,F32}` in `Plane<T>` so a clone shares the sample
  allocation and a write forks only the touched store.
- Store `SmartObject.payload` and the raw linked-section bytes in
  `Document.layer_section_extra` in refcounted planes, so embedding a PSB-sized
  source no longer forces a full in-memory copy of either the typed payload or
  its raw `lnk2`/`lnk3` record on clone.
- Store `Selection.data` in a refcounted plane so the roughly three clones per
  history capture share one mask allocation until the selection is written.
- Back `LayerStack` with `Arc<Vec<Layer>>` (copy-on-write) so cloning a document
  or a clipping base does not copy the layer tree. This is the chosen
  alternative to the issue's proposal to thread compositor overrides through
  `composite_layer_inner`; the per-band `Layer::clone` still copies the small
  container vectors (`channels`, `extra_blocks`, `raw_channels`,
  `smart_filters`) while their bulk data is shared.
- Store every preserved byte block in refcounted planes: `RawChannel.data`,
  `LayerBlock.data`, `LayerMask.extra`, `AdjustmentData.data`,
  `SmartObject.config_descriptor`, `SmartObject.crs_xmp`,
  `Document.source_icc`, `Document.document_icc`,
  `Document.color_mode_data`, `Document.image_resources`,
  `Document.global_layer_mask`, `Document.layer_section_extra`,
  `Layer.blending_ranges`, `SmartFilter.options`, `TypeTool.text_desc`, and
  `TypeTool.warp_desc`.
- Answer `can_free_transform` from a read-only smart-source predicate instead of
  cloning and rasterizing a whole document. The predicate still decodes and
  rasterizes the layer source; it only avoids the document clone and the
  channel split.
- **BREAKING (workspace-internal):** the `Samples` variant payload type,
  `Selection.data`, `SmartObject::payload_mut`, the byte-block fields
  (including `Document.layer_section_extra`, `Layer.blending_ranges`,
  `SmartFilter.options`, and the `TypeTool` descriptors), and the `LayerStack`
  internals change type. Behavior and serialized bytes are unchanged; no public
  file format or dependency changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `color/bit-depth-sample-model`: sample stores become copy-on-write; cloning
  shares the allocation and a write forks only the touched store.
- `document/document-model`: cloning a `Document` or `Layer` shares its pixel,
  sample, payload, raw linked-section, byte-block, and layer-tree storage;
  writes fork only the touched store.
- `document/history-retention`: native samples, smart-object payloads and raw
  linked-section bytes, preserved byte blocks, and the selection are shared
  across history states while unchanged, so retention stays proportional to the
  edited data. The layer-tree container may be forked while a capture builds its
  materialized states; its nodes' bulk stores stay shared.

## Impact

- `pictura-core`: `Samples`, `Plane`, `LayerStack`, `Layer`, `SmartObject`,
  `SmartFilter`, `TypeTool`, `LayerBlock`, `RawChannel`, `LayerMask`,
  `AdjustmentData`, `SourcePlanes`, `SourceChannels`, `Document`.
- `pictura-select`: `Selection` and its boolean/modify operations.
- `pictura-render`: native store helpers, compositor clone sites, smart-object
  transform predicate.
- `pictura-codec`, `pictura-paint`, `pictura-adjust`: sample and byte-block
  construction/access sites.
- `pictura-app`: history capture/restore, transform session, selection lift,
  clipboard and image-match coverage helpers.
- No new dependencies. Golden images, oracles, and serialized PSD bytes are
  unchanged.
