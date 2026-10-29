# Spec Delta

## ADDED Requirements

### Requirement: Pixel planes are band-granular copy-on-write

A pixel plane SHALL store its samples either as one contiguous allocation or as
one base allocation plus a sequence of 64-row bands. A band SHALL be either a
shared range into the plane's base allocation or an owned allocation holding a
forked copy of that band. Cloning a plane SHALL share every band and SHALL NOT
copy pixel bytes. Fracturing a plane SHALL record the plane's width and create
its band ranges without copying pixel bytes. A write to a band SHALL fork at
most that band's bytes and SHALL leave every other band shared with any other
holder of the plane. A band write SHALL write in place when the band is an owned
allocation with no other owner, or when the band is a shared range and the base
allocation has no other owner; otherwise it SHALL copy that band's range into an
owned allocation.

A whole-plane mutable write, through `as_mut_slice` or `DerefMut`, SHALL
materialize one contiguous allocation, replace the plane's band view with that
allocation, and MAY copy the plane once. Region and row writes SHALL go through
the band writer and SHALL fork only the bands they touch.

A contiguous read of a plane SHALL return the plane's logical bytes: the base
allocation while no band has diverged, and otherwise one materialized contiguous
view, cached until the next write. Per-band and per-row reads SHALL return the
same bytes as the contiguous read. A band's geometry SHALL be 64 rows of the
plane's width, and the last band MAY be shorter.

Fracturing SHALL record the width. A plane of length zero or width zero SHALL
fracture to no bands and SHALL read as empty. When the length is not a multiple
of the width, the final band SHALL hold the remaining samples, including any
trailing partial row; tracked-plane capture covers whole rows, so a trailing
partial row is not part of the tile grid. Re-fracturing an already fractured plane at the same width
SHALL be a no-op; re-fracturing at a different width, or after a band has
diverged, SHALL flatten once and rebuild the bands.

`shares(a, b)` SHALL be true only when every band of both planes is the same
allocation, meaning the same base with no band diverged. `shares_band(i, other)`
SHALL report whether band `i` is the same allocation as the corresponding band
of `other`.

#### Scenario: Fracturing copies no pixels [dm_band_fracture]

- **WHEN** a 4000×2000 plane is fractured so it can be shared
- **THEN** the band handles are created without allocating or copying pixel
  bytes, and together they cover the plane's original allocation

#### Scenario: A band write forks only its band [dm_band_write_forks_one]

- **WHEN** a fractured plane is cloned and one band of the clone is written
- **THEN** only that band's allocation changes, every other band is still one
  allocation shared with the clone, and the clone's bytes are unchanged

#### Scenario: Contiguous and band reads agree [dm_band_reads_agree]

- **WHEN** a plane is fractured, shared, and written, and its bytes are then read
  contiguously and per band
- **THEN** both reads equal the result of the same operations on an unfractured
  plane, and a second contiguous read with no intervening write allocates
  nothing

#### Scenario: Fracture preserves every byte [dm_band_round_trip]

- **WHEN** a plane holding arbitrary bytes is fractured, cloned, written, and
  compared with the same sequence applied to an unfractured plane
- **THEN** every read returns byte-identical samples

#### Scenario: A whole-plane write flattens [dm_band_whole_write_flattens]

- **WHEN** a fractured plane is written through `as_mut_slice` or `DerefMut`
- **THEN** the plane becomes one contiguous allocation, its band view is
  replaced, and any other holder of the previous bands keeps its bytes

#### Scenario: Degenerate planes stay well-defined [dm_band_degenerate_planes]

- **WHEN** an empty plane, a width-zero plane, or a plane whose length is not a
  multiple of its width is fractured, or an already fractured plane is
  fractured again
- **THEN** the empty and width-zero planes have no bands and read as empty, the
  final band holds the remaining whole rows, a trailing partial row is not part
  of the tracked-plane tile grid, and re-fracturing at the same width is a no-op
  while a different width flattens once and rebuilds
