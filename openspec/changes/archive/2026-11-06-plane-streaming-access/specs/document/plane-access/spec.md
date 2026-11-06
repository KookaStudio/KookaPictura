## ADDED Requirements

### Requirement: Streaming plane access

`Plane` SHALL offer access that never builds a contiguous copy of a diverged
banded plane:
- rows of a given width, read in place (`rows`) or written in place
  (`rows_mut`);
- a rectangle copied out (`read_rect_into`) or written in (`write_rect_from`);
- a logical sample range copied out (`read_into`);
- every band with the row it starts at (`for_each_band`,
  `for_each_band_mut`).

Each read SHALL return the same samples as the plane's materialized slice. A
mutable access SHALL fork only the bands it touches, leaving every other band
shared with the plane's clones. The parts of a rectangle or range that fall
outside the plane SHALL be skipped and leave the caller's buffer untouched.
None of these accesses SHALL increase the materialization count.

#### Scenario: Rows read a diverged plane in place [pa_rows]

- **WHEN** rows spanning a band edge are read from a fractured plane whose
  middle band has diverged from a clone
- **THEN** each row equals the matching row of the materialized slice, rows
  past the end are not yielded, and the materialization count is unchanged

#### Scenario: A rectangle is copied out and clipped [pa_read_rect]

- **WHEN** a rectangle spanning two bands, and one that runs past the
  plane's last column and row, are read into a buffer
- **THEN** the in-plane samples equal the materialized slice's, the samples
  outside the plane are left untouched, and an empty rectangle copies nothing

#### Scenario: A logical range is copied out across bands [pa_read_into]

- **WHEN** a range starting inside one band and ending in the next is read
  into a buffer, and a range runs past the plane's end
- **THEN** the samples equal the materialized slice's and the part past the
  end is left untouched

#### Scenario: Bands are visited with their first row [pa_bands]

- **WHEN** every band of a diverged plane is visited
- **THEN** the bands are visited in order, each with its first row, they
  concatenate to the materialized slice, and the count is unchanged

#### Scenario: A mutable access forks only the bands it touches [pa_mut]

- **WHEN** rows of one band of a diverged plane are written through
  `rows_mut`, a rectangle through `write_rect_from`, or every band through
  `for_each_band_mut`
- **THEN** the plane holds the written samples, the bands that were not
  written are still shared with the clone, the clone keeps its bytes, a
  uniquely held base is written in place, and the count is unchanged

#### Scenario: The guard catches a whole-plane copy [pa_guard]

- **WHEN** `assert_no_materialization` wraps a closure that reads a diverged
  plane through `as_slice`
- **THEN** the guard panics

### Requirement: Hot paths do not materialize banded planes

Compositing, Move, and Free Transform SHALL read and write a layer's
fractured planes through streaming access and SHALL NOT materialize a
diverged banded plane. They SHALL produce the same bytes as the same
operation on flat planes. The memory benchmark's heap guards SHALL report each
scenario's materializations, and SHALL fail any scenario listed as streamed
that materializes a plane.

#### Scenario: Compositing reads a diverged layer and mask in place [pa_composite]

- **WHEN** a document whose layer channels and raster mask are diverged banded
  planes is composited
- **THEN** no plane is materialized and the composite equals the composite of
  the same document with flat planes

#### Scenario: Move reads and merges diverged rows in place [pa_move]

- **WHEN** a layer's content is trimmed and grown back, and selected content
  is cut, moved, and merged back into the layer, all on diverged banded planes
- **THEN** no plane is materialized and the layers equal the result of the
  same operations on flat planes

#### Scenario: Free Transform resamples a diverged layer in place [pa_transform]

- **WHEN** a layer and its mask with diverged banded planes are scaled by
  150 % and rotated by 15° with bicubic interpolation
- **THEN** no plane is materialized and the layer equals the same transform
  of flat planes

#### Scenario: The memory scenarios stay free of materializations [pa_scenarios]

- **WHEN** a heap guard in the streamed list runs its scenario at 4250²
- **THEN** its measured step materializes no plane

#### Scenario: The view's pan and zoom read no planes [pa_view]

- **WHEN** the view is fitted, set to 100 %, panned across the document, and
  fitted again (memory scenario M13)
- **THEN** no plane is materialized
