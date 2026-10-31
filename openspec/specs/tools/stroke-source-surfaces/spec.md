# tools/stroke-source-surfaces Specification

## Purpose
Defines how the layer, composite, history, and pattern surfaces a paint stroke
reads are built, cropped, sampled, and reused, so a stroke's setup does not pay
for per-pixel plane lookups or redundant full-document passes.

## Requirements

### Requirement: Stroke source surfaces match the serial builder

The layer, composite, and history surfaces a stroke reads SHALL be
byte-identical to building the same surface serially, and each SHALL resolve the
target's plane layout once per surface rather than once per pixel. This SHALL
hold for a layer with an offset or partial rectangle, a Background layer without
an alpha plane, and a source rectangle that hangs off the document.

#### Scenario: Layer surface matches the serial builder [ss_layer_parity]

- **WHEN** the layer surface is built for a layer that covers the document, for
  a layer with an offset and a partial rectangle, and for a Background layer
  without an alpha plane
- **THEN** each surface is byte-identical to the serial builder's surface

#### Scenario: Composite surface matches the serial builder [ss_composite_parity]

- **WHEN** a document composite is turned into a document-space surface
- **THEN** the surface is byte-identical to the serial planar-to-interleaved
  conversion

#### Scenario: History surface matches the serial builder [ss_history_parity]

- **WHEN** a history state's layer has moved since the state was captured and
  its surface is built in document space
- **THEN** the surface places the past pixels at the layer's past rectangle and
  is byte-identical to the serial builder's surface

#### Scenario: Retouch stroke start is reported [ss_profile]

- **WHEN** the ignored retouch profile begins a Brush control, a Dodge, a Smudge
  with Sample All Layers, a Clone Stamp All Layers, a History Brush, and a
  Pattern Stamp stroke on an 8000×6000 document
- **THEN** it reports the stroke-start cost of each

### Requirement: A zero-size source is empty

When a layer, composite, or crop has zero width or height, its builder SHALL
return an empty image and SHALL NOT panic.

#### Scenario: A zero-size source is empty [ss_zero_size]

- **WHEN** a layer, composite, or crop has zero width or height
- **THEN** the builder returns an empty image and does not panic

### Requirement: Sample All Layers crops in the same pass

When Smudge, Blur, Sharpen, or Mixer samples the composite, the layer-local
source SHALL be built in one pass. It SHALL NOT first build a document-sized
surface and then crop it, and the sampled pixels SHALL be identical to the
two-pass result.

#### Scenario: Sampled source matches the two-pass crop [ss_sample_all_parity]

- **WHEN** Sample All Layers builds the layer-local source for a layer with an
  offset and a partial rectangle
- **THEN** the sampled pixels are identical to building a document-sized
  surface and cropping it to the layer rectangle

### Requirement: Pattern Stamp samples its tile per dab

The Pattern Stamp SHALL NOT build a document-sized pattern image at stroke
start. It SHALL read the pattern tile at the offset destination pixel within the
document bounds, and the painted pixels SHALL be identical to painting from a
pre-tiled document-sized image. A zero-size tile SHALL yield no source.

#### Scenario: Pattern origins match the tiled image [ss_pattern_parity]

- **WHEN** a Pattern Stamp stroke runs with the pattern origin on the document
  origin, off it, and negative, and on a layer that hangs off the document, and
  a zero-size tile is requested
- **THEN** every painted pixel is identical to the same stroke painted from a
  pre-tiled document-sized image with that origin, and the zero-size tile
  produces no source

### Requirement: History source surfaces are reused until the resolved source changes

A History Brush, Art History Brush, or Erase to History stroke SHALL reuse the
source surface built for the same resolved history source and layer path until
either changes.
A different source state or snapshot, a source that is no longer resolvable or
whose resolved id changes, a brush-source change, or a different layer path
SHALL invalidate the reused surface, and the next stroke SHALL read the current
source. A source kept as the pinned source when the stack drops it keeps the
reused surface. Adding a history state SHALL NOT invalidate the reused surface.

#### Scenario: A repeated stroke reuses the source [ss_history_reuse]

- **WHEN** a History Brush stroke starts from a history state and commits,
  adding a new state, and a second stroke starts from the same source state and
  layer path
- **THEN** the second stroke reuses the first stroke's source surface instead of
  rebuilding it, and paints the same pixels

#### Scenario: A history change invalidates the source [ss_history_invalidate]

- **WHEN** the resolved source is a different state or snapshot, the source is
  no longer resolvable or its resolved id changes, the brush source changes, or
  the layer path changes
- **THEN** the next stroke builds the surface from the current source
