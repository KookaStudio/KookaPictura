# tools/tone-stroke-responsiveness Specification

## Purpose
Keeps Dodge, Burn, and Sponge strokes dab-sized on large documents, byte-identical to the serial raster, and bounds the per-dab write-back every retouch engine shares.

## Requirements

### Requirement: Toning strokes stay dab-sized on a large document

A Dodge, Burn, or Sponge stroke SHALL start without reading the whole layer, and
each dab SHALL load, tone, and write back only the pixels it reaches. The stroke
SHALL copy no whole layer plane, and its dabs SHALL run on every core. The result
SHALL be byte-identical to toning the whole layer's pixels on one thread: the same
pixels, the same dirty rectangle, and the same changed flag.

#### Scenario: The parallel dab equals the serial raster [tsr_exact]

- **WHEN** Dodge, Burn, and Sponge dabs, at every Range, with Protect Tones and
  Vibrance on and off, are toned inside and across the edges of a textured image
  with a soft, hard, flattened, rotated, aliased, square, and small tip, once in
  parallel and once on one thread over the whole bounding square
- **THEN** every dab reports the same dirty rectangle and the images are
  byte-identical

#### Scenario: A stroke reads only the tiles it reaches [tsr_start]

- **WHEN** a toning stroke begins and paints across part of a layer, and the same
  stroke runs from a copy of the whole layer
- **THEN** nothing is read when it begins, the two documents are byte-identical,
  and a tile no dab reached is never loaded

#### Scenario: A dab forks only the bands it touches [tsr_first_dab]

- **WHEN** a toning dab lands on a banded plane that the history shares
- **THEN** the plane stays banded, the touched band forks, and an untouched band
  stays shared

#### Scenario: A layer changed under the stroke is refused [tsr_stale_layer]

- **WHEN** the target layer's bounds change while a toning stroke is live
- **THEN** the next sample changes nothing and reports no change

### Requirement: Per-dab engines write back only what each dab changed

A per-dab stroke SHALL write each dab's changed rectangle into the layer's bands,
not the union of every dab in a sample, and SHALL give the same pixels and dirty
rectangle in parallel as on one thread. Cancelling SHALL restore the document.

#### Scenario: Banded and serial write-back agree [tsr_write_back]

- **WHEN** Dodge, Blur, and Smudge strokes place far-apart dabs on a textured,
  offset layer, once banded and once on one thread
- **THEN** every sample reports the same changed flag and dirty rectangle, the
  documents are byte-identical, and cancelling restores the original
