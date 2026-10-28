# Spec Delta

## MODIFIED Requirements

### Requirement: Unchanged layer uploads stay resident

The GPU compositor SHALL keep a layer's source upload between composites, keyed
by the stamps of the planes it was built from, the layer rect, the document size,
and the grayscale flag. It SHALL keep a sampled mask coverage upload keyed by the
mask plane, its region, and its influence rectangle, and MUST retain it only for
a composite over the whole document. A source or coverage upload built for a
composite over a sub-region SHALL NOT be retained. The constant coverage of a
layer with no enabled data-carrying mask SHALL NOT be retained; it is rebuilt
row-wise from the layer geometry. An upload whose planes carry no stamp SHALL NOT
be kept or reused, so a written plane is always uploaded again. Retention SHALL
be bounded by a byte budget and an entry cap, evicting the oldest retained
uploads first.

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

## ADDED Requirements

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
