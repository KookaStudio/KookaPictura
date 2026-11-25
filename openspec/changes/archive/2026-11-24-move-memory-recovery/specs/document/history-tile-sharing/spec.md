# document/history-tile-sharing Specification

## ADDED Requirements

### Requirement: Tiling a large flat plane is memory-gated

Converting a flat plane to tiles for the history SHALL reserve the tiled copy
first and, when it does not fit the room, SHALL leave the plane flat. A flat
plane that stays flat SHALL still compare by shared allocation, and a changed
one SHALL fall back to a full copy-on-write state. The conversion MUST NOT evict
display tiles to make room for itself.

#### Scenario: A plane that does not fit stays flat [hts_flat_plane_gate]
- **WHEN** a flat 16 MiB plane is tiled in a room that cannot hold the tiled copy
- **THEN** the plane stays flat and no display tile is evicted for it

#### Scenario: A plane with room becomes tiled [hts_tiled_when_room]
- **WHEN** a flat plane is tiled with room for the tiled copy
- **THEN** the plane becomes tiled and the history shares its tiles
