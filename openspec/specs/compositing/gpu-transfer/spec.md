# compositing/gpu-transfer Specification

## Purpose
Moves composite pixels between host and GPU storage without redundant host
copies.

## Requirements

### Requirement: GPU composites move pixels without host copies

A GPU composite SHALL write each layer's source planes and mask coverage
straight into the queue's staging memory, in parallel runs, without first
assembling them in a host buffer; SHALL read its result back into one
allocation taken from zeroed pages; and SHALL reuse its readback buffer across
composites, so a large readback does not fault a fresh mapping in every time.
The uploaded bytes SHALL equal the per-pixel reference assembly and the
composite SHALL keep matching the CPU oracle within ±1 LSB.

#### Scenario: The parallel writers equal the per-pixel reference [gt_writers]

- **WHEN** the source and mask writers fill a buffer for a covering RGB layer, a
  layer without alpha, a monochrome layer, a gray layer, and masks with and
  without data
- **THEN** the bytes equal the per-pixel reference assembly

#### Scenario: Large documents still match the CPU [gt_parity]

- **WHEN** the `gpu_parity` suite runs with `PICTURA_GPU_BENCH=1` (4000²
  three-layer and over-the-1-D-limit documents)
- **THEN** every composite matches the CPU within ±1 LSB

### Requirement: Unchanged layer uploads stay resident

The GPU compositor SHALL keep a layer's source upload between composites, keyed
by the stamps of the planes it was built from, the layer rect, the document size,
and the grayscale flag. A source built from a cached CPU raster (a proxy-less
type layer, a smart-filtered smart object, a channel-less smart-object source, or
a Solid/Gradient fill) SHALL instead be keyed by that raster's content identity,
the raster rectangle, the document size, and the grayscale flag; it MUST NOT be
keyed by a plane stamp the raster does not carry. A Pattern fill upload SHALL NOT
be retained, because the pattern library is not part of the key. It SHALL keep a
sampled mask coverage upload keyed by the mask plane, its region, and its
influence rectangle, and MUST retain it only for a composite over the whole
document. A source or coverage upload built for a composite over a sub-region
SHALL NOT be retained. The constant coverage of a layer with no enabled
data-carrying mask SHALL NOT be retained; it is rebuilt row-wise from the layer
geometry. An upload whose planes carry no stamp SHALL NOT be kept or reused, so a
written plane is always uploaded again. Retention SHALL be bounded by a byte
budget and an entry cap, evicting the oldest retained uploads first.

#### Scenario: Reused, never stale [gt_resident]

- **WHEN** a stamped document is composited twice on the GPU, then a layer's
  blend mode changes, then a plane is written without and with a fresh stamp
- **THEN** the second composite reuses the uploads and every composite matches
  the CPU within ±1 LSB

#### Scenario: A region composite reuses whole-document sources

- **WHEN** a document's layers are composited over the whole document and then a
  smaller rectangle is composited repeatedly while the layer content is unchanged
- **THEN** each region composite reports a resident hit for every source it
  dispatches instead of re-uploading it

#### Scenario: Moving region composites leave the store flat

- **WHEN** 100 region composites run over moving rectangles on an unchanged
  document
- **THEN** the retained entry count and byte total do not grow, and a following
  whole-document composite reports zero misses for the unchanged layers

#### Scenario: Constant coverage is rebuilt, never retained

- **WHEN** a pixel layer has no enabled mask carrying data, including a disabled
  mask that carries data
- **THEN** its coverage upload is built for the composite and is not offered to
  the resident store

#### Scenario: A sampled mask is reused by a repeated whole-document composite

- **WHEN** a layer with an enabled raster mask is composited over the whole
  document twice with unchanged mask bytes
- **THEN** the second composite reports a resident hit for the mask upload

#### Scenario: A cached raster source is reused

- **WHEN** a document containing a proxy-less type layer, a smart-filtered smart
  object, or a Solid/Gradient fill layer is composited twice on the GPU with
  unchanged content
- **THEN** the second composite reports a resident hit for each cached raster
  source and does not re-upload it

#### Scenario: A pattern fill upload is not retained

- **WHEN** a document containing a Pattern fill layer is composited on the GPU
- **THEN** the fill upload is used for that composite and is not offered to the
  resident store

#### Scenario: A cached raster source changes when its content changes

- **WHEN** the type text, the smart-filter chain, the smart-object payload, or
  the fill parameters change and the document is composited again
- **THEN** the new raster is uploaded and the result matches a
  purge-then-composite

#### Scenario: A cached raster upload is not retained for a sub-region build

- **WHEN** a cached raster source is first uploaded for a composite over a
  sub-region
- **THEN** the upload is used for that composite and is not retained

### Requirement: The resident store is bounded and observable

The resident store SHALL NOT retain an upload larger than the byte budget; the
current composite may still use it. It SHALL enforce both the byte budget and a
maximum entry count, evicting the oldest retained uploads first. A lookup SHALL
find an entry by key in time independent of the number of retained uploads, and
MUST NOT change the retained entries or their byte total. Keeping a key again
SHALL replace its previous entry and accounting, so one key occupies one entry.
The store SHALL maintain a running byte total rather than recomputing it per
eviction. `pictura_render::metrics::Counters` SHALL report the retained entry
count and total bytes, updated on every keep and eviction.

#### Scenario: The byte budget evicts the oldest uploads

- **WHEN** uploads whose combined size exceeds the byte budget, each within it,
  are kept in order
- **THEN** the store drops the oldest entries until it is within the budget and
  the most recent upload is retained

#### Scenario: An upload larger than the budget is not retained

- **WHEN** an upload larger than the byte budget is kept
- **THEN** the composite uses it, the store retains nothing from it, and the
  entry count and byte total are unchanged

#### Scenario: The entry count stays capped

- **WHEN** more distinct uploads than the entry cap are kept
- **THEN** the store retains at most the cap and drops the oldest entries

#### Scenario: Keeping a key again replaces its entry

- **WHEN** the same key is kept a second time
- **THEN** one entry remains for that key, the running byte total counts only
  the newer upload, and an eviction cannot drop the newer upload first

#### Scenario: A retained upload is found by its key

- **WHEN** an upload is kept under a key and read back by that key
- **THEN** the store returns it, and an unknown key returns nothing

#### Scenario: A lookup does not mutate the store

- **WHEN** a retained upload is read by its key
- **THEN** the retained entry count and byte total are unchanged

#### Scenario: The report reflects entries and bytes

- **WHEN** a composite retains uploads and the counters are read
- **THEN** the reported entry count and byte total are non-zero and consistent
  with the uploads the composite retained
