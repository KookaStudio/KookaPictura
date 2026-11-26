## ADDED Requirements

### Requirement: A layer grow copies only the tiles that hold pixels

Growing a tiled pixel plane into a larger rect SHALL yield the same samples as a
row-by-row copy into a transparent plane. The grow SHALL NOT allocate or write
any destination tile for an empty source tile. A source tile that lands exactly
on a destination tile of the same size SHALL be shared, not copied. A layer
growing right or down, or by any whole-tile offset, therefore keeps its full
tiles without copying a pixel.

#### Scenario: A grow equals a row copy [lgc_pixels]

- **WHEN** a tiled plane with empty, uniform and dense tiles, one or two
  channels, is padded into a larger plane at a tile-aligned or unaligned offset
- **THEN** every sample equals a row-by-row copy of the plane into a zero
  plane at that offset

#### Scenario: A grow right and down shares the full tiles [lgc_share]

- **WHEN** a tiled layer grows right and down
- **THEN** each of its planes holds exactly the allocations it held before, and
  the new area reads transparent

#### Scenario: Empty tiles are not copied [lgc_empty]

- **WHEN** a plane with one painted tile among empty ones is padded at an
  unaligned offset
- **THEN** only the destination tiles under that painted tile are written, and
  no contiguous copy of the plane is made

#### Scenario: A long stroke on a large document spends little on growing [lgc_long_stroke]

- **WHEN** the long-stroke profile paints its 400 px strokes of about 15,500 px
  on a 21,000² document
- **THEN** the layer grows take a small part of each stroke's time, not most
  of it
