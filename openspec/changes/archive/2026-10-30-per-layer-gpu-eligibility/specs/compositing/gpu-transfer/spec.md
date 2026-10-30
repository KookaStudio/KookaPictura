# Spec Delta

## MODIFIED Requirements

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
