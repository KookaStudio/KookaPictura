## ADDED Requirements

### Requirement: Large dabs raster in parallel row bands

A Paint-kind stroke (Brush, Pencil, Eraser, and the source strokes: Clone
Stamp, Pattern Stamp, History Brush) SHALL raster a dab whose clipped bounding
box covers at least 128 × 128 px as bands of whole rows run in parallel, each
band writing only its own rows of the coverage buffers and the layer's planes.
The result SHALL be byte-identical to rastering the same dab on one thread: the
same pixels, the same dirty rectangle, and the same changed flag, for every
paint mode, tip shape, dynamics, stamp source, and transparency lock. Dissolve,
whose random rolls follow raster order, SHALL raster on one thread.

#### Scenario: Banded and serial rasters agree [bdr_exact]

- **WHEN** a stroke of 260 px dabs is painted over a textured, offset layer in
  each of Normal, Behind, Clear, and Dissolve, with a flattened and rotated tip,
  a square tip, jitter, an aliased tip, a transparency lock, and a stamp source,
  once banded and once forced onto one thread
- **THEN** every sample reports the same changed flag and dirty rectangle, and
  the two documents are byte-identical

### Requirement: Fills run their rows in parallel

The shared fill pass behind the Gradient, Paint Bucket, Fill, Stroke, Fill
Path, Stroke Path, and the shape tools' Pixels mode SHALL process the layer's
rows in parallel. Its colour source SHALL be a pure function of the pixel, so
the result, including Dissolve's per-pixel noise, SHALL repeat exactly from run
to run.

#### Scenario: A Dissolve fill repeats [bdr_fill_repeat]

- **WHEN** a 64 × 64 layer is filled twice with a half-opaque colour at half
  strength in Dissolve
- **THEN** about a quarter of the pixels are painted, each wholly, and the two
  fills are identical

### Requirement: Shape coverage sweeps only the path's columns

The anti-aliased shape coverage the Pixels mode fills through SHALL clear and
write, per row, only the columns between the path's leftmost and rightmost
points (clamped to the document), and SHALL give the same coverage as a sweep
of the whole row.

#### Scenario: Coverage ignores the columns beside the path [bdr_coverage_cols]

- **WHEN** an ellipse inside the document, and one hanging off its left edge,
  are rasterized on a 30 px and on a 90 px wide document
- **THEN** the first 30 columns of every row agree and the rest of the wide
  document is empty
