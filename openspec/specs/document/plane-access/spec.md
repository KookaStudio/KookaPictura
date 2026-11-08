# document/plane-access Specification

## Purpose
Pixel planes are read and written by row, rectangle, range, or band, so hot paths never materialize a diverged banded plane into a canvas-sized contiguous copy, and a later tiled plane can replace the bands without touching their callers.

## Requirements

### Requirement: Streaming plane access

`Plane` SHALL offer access that never builds a contiguous copy of a tiled
plane:
- a row copied out (`read_row_into`) or written in (`write_row_from`);
- a rectangle copied out (`read_rect_into`) or written in (`write_rect_from`);
- a logical sample range copied out (`read_into`);
- the tiles under a rectangle, visited (`for_each_tile`) or written
  (`tiles_mut`, `for_each_tile_mut`).

Each read SHALL return the same samples as the plane's materialized slice. A
mutable access SHALL fork only the tiles it touches, leaving every other tile
shared with the plane's clones. The parts of a rectangle, row or range that
fall outside the plane SHALL be skipped and leave the caller's buffer
untouched. None of these accesses SHALL increase the materialization count.

#### Scenario: Rows read a diverged plane in place [pa_rows]

- **WHEN** rows crossing a tile edge are copied out of a tiled plane whose
  middle tile has diverged from a clone, and a row past the end is asked for
- **THEN** each row equals the matching row of the materialized slice, the row
  past the end leaves the buffer untouched, and the materialization count is
  unchanged

#### Scenario: A rectangle is copied out and clipped [pa_read_rect]

- **WHEN** a rectangle spanning two tiles, and one that runs past the
  plane's last column and row, are read into a buffer
- **THEN** the in-plane samples equal the materialized slice's, the samples
  outside the plane are left untouched, and an empty rectangle copies nothing

#### Scenario: A logical range is copied out across bands [pa_read_into]

- **WHEN** a range starting inside one tile row and ending in the next is read
  into a buffer, and a range runs past the plane's end
- **THEN** the samples equal the materialized slice's and the part past the
  end is left untouched

#### Scenario: Bands are visited with their first row [pa_bands]

- **WHEN** every tile of a diverged plane is visited
- **THEN** each tile is visited once with its position and size, their samples
  equal the materialized slice's, and the count is unchanged

#### Scenario: A mutable access forks only the bands it touches [pa_mut]

- **WHEN** a row of a diverged plane is written through `write_row_from`, a
  rectangle through `write_rect_from`, or the tiles under a rectangle through
  `for_each_tile_mut`
- **THEN** the plane holds the written samples, the tiles that were not
  written are still shared with the clone, the clone keeps its bytes, and the
  count is unchanged

#### Scenario: The guard catches a whole-plane copy [pa_guard]

- **WHEN** `assert_no_materialization` wraps a closure that reads a tiled
  plane through `as_slice`
- **THEN** the guard panics

### Requirement: Hot paths do not materialize banded planes

Compositing, Move, Free Transform, painting, filling, history capture and
PSD write SHALL read and write a document's tiled planes through streaming
access and SHALL NOT materialize a tiled plane into a contiguous copy. They
SHALL produce the same bytes as the same operation on flat planes. The memory
benchmark's heap guards SHALL report each scenario's materializations, and
SHALL fail any scenario listed as streamed that materializes a plane.

#### Scenario: Compositing reads a diverged layer and mask in place [pa_composite]

- **WHEN** a document whose layer channels and raster mask are diverged tiled
  planes is composited
- **THEN** no plane is materialized and the composite equals the composite of
  the same document with flat planes

#### Scenario: Move reads and merges diverged rows in place [pa_move]

- **WHEN** a layer's content is trimmed and grown back, and selected content
  is cut, moved, and merged back into the layer, all on diverged tiled planes
- **THEN** no plane is materialized and the layers equal the result of the
  same operations on flat planes

#### Scenario: Free Transform resamples a diverged layer in place [pa_transform]

- **WHEN** a layer and its mask with diverged tiled planes are scaled by 150 %
  and rotated by 15° with bicubic interpolation
- **THEN** no plane is materialized and the layer equals the same transform
  of flat planes

#### Scenario: The memory scenarios stay free of materializations [pa_scenarios]

- **WHEN** a heap guard in the streamed list (M1–M12) runs its scenario at
  4250²
- **THEN** its measured step materializes no plane

#### Scenario: The view's pan and zoom read no planes [pa_view]

- **WHEN** the view is fitted, set to 100 %, panned across the document, and
  fitted again (memory scenario M13)
- **THEN** no plane is materialized
