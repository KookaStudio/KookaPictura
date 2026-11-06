## ADDED Requirements

### Requirement: A banded level-0 source is read row by row

The view SHALL accept level 0 as either contiguous channel slices or a planar
copy-on-write plane, and SHALL read the pixels it needs through whole rows of
that source. Building, coarsening, updating and cropping a view from a banded
plane MUST NOT materialize a contiguous copy of it, and MUST produce the same
bytes as the same pixels supplied as contiguous slices. The view MUST NOT write
to the supplied level-0 source.

#### Scenario: A banded level 0 builds the same levels [vp_banded_rebuild]

- **WHEN** a view is built from a plane whose bands were forked from a shared
  base, and from the same pixels supplied as contiguous slices
- **THEN** every level is byte-identical and the banded plane is not
  materialized

#### Scenario: A banded level 0 updates and crops [vp_banded_update]

- **WHEN** a region update and a level-0 crop run on a banded level-0 plane
- **THEN** both match the same operations on the contiguous source byte for
  byte, and neither builds a contiguous copy
