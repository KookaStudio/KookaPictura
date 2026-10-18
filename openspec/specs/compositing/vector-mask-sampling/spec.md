# compositing/vector-mask-sampling Specification

## Purpose
Builds a bounded row table for rasterizing a vector mask so sampling stays
within the mask's coverage.

## Requirements

### Requirement: Bounded vector-mask row table

Vector-mask coverage sampling SHALL index the mask's closed subpaths into a
per-row edge-crossing table, and SHALL bound that table to the sampled rows:
the caller's clip grown by one row each side, intersected with the path's own
row span. Rows outside the clip SHALL never be queried, and an edge spanning
past the clip SHALL contribute crossings only to the in-range rows. The table's
size MUST depend on the clip, not on the file's path points, so a crafted `vmsk`
whose 8.24 points span the whole `i32` range (about ±128× the document height)
MUST NOT allocate or iterate more rows than the clip covers.

#### Scenario: A crafted path is bounded by the clip [vms_bound]

- **WHEN** a vector mask whose points span `i32::MIN` to `i32::MAX` is sampled
  over a document-height clip
- **THEN** its row table covers only the clip's rows plus one each side, not the
  path's millions of rows

#### Scenario: Coverage inside the clip is unchanged [vms_inside]

- **WHEN** a rectangle mask is sampled at pixels over a clip that contains them
- **THEN** each sample reports inside or outside exactly as the even-odd (or
  non-zero) scanline of the full path would

#### Scenario: A sample outside the clip is outside [vms_outside]

- **WHEN** a pixel row outside the clip is queried
- **THEN** the sample is outside and no row beyond the clip is built

### Requirement: Bounded row-table cache

The clip SHALL be supplied by the caller and SHALL be the exact region the
caller samples. The compositor SHALL pass the document canvas; a feathered mask
SHALL pass the plane it rasterizes and blurs (the canvas grown by the feather's
reach); a rasterizing or thumbnail caller SHALL pass the region it walks. The
row table SHALL be a cache bounded by the first query's clip; a row outside the
cache SHALL be answered by scanning the mask's edges directly, so the clip
bounds only the cache and never the sample's correctness. Changing the mask's
feather radius SHALL retire the table so the grown feather plane stays on the
indexed path.

#### Scenario: A row outside the cached clip stays correct [vms_fallback]

- **WHEN** a mask is sampled over one clip and then over a later clip whose rows
  fall outside the cached table (a partial-region composite)
- **THEN** every sample still reports inside or outside exactly as the full
  scanline would

#### Scenario: A feather change rebuilds the table [vms_feather_rebuild]

- **WHEN** a mask is sampled with no feather and then its feather radius is set
- **THEN** its row table is retired and rebuilt over the feather plane's grown
  clip

#### Scenario: The feather ramp crosses the canvas edge [vms_feather_edge]

- **WHEN** a feathered path extends past the canvas edge and a pixel on the edge
  is sampled
- **THEN** the blur sees the coverage just outside the canvas, not zero

### Requirement: Rows beyond the path answer without an edge scan

A vector-mask sample in a row above the path's first crossing row or at or
below its last SHALL report outside at once, without walking the path's edges,
whatever clip built the row table. Only a row inside the path's row span but
outside the cached table SHALL fall back to scanning the edges.

#### Scenario: Rows past the path are outside [vms_beyond]

- **WHEN** a rectangle mask whose table was built for a two-row clip is sampled
  in rows above, below, and inside the path, under even-odd and non-zero
- **THEN** rows inside the path (cached or not) are inside and every row beyond
  it is outside
