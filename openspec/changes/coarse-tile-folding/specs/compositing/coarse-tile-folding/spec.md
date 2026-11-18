## ADDED Requirements

### Requirement: A coarse tile folds from the finer tiles at hand

A missing tile above level 0 SHALL be built from the current finer tiles of
the same source that lie under it, cached or built earlier in the same build,
folding one level at a time through as many levels as needed. Only the parts
of its footprint under which no finer current tile lies SHALL be built from
the level-0 source. A stale tile MUST NOT be folded from. A folded tile SHALL
equal the same tile built from the level-0 source, byte for byte. The worker
and the synchronous path SHALL follow this rule alike.

#### Scenario: The coarsest level folds from the cached fit level [ctf_fold_cached]

- **WHEN** every tile of a finer level is cached and a tile two or more levels coarser is built
- **THEN** it equals the tile built from the level-0 source, and the level-0 source is not read

#### Scenario: Only the uncovered parts read level 0 [ctf_partial]

- **WHEN** finer tiles are cached under part of a coarse tile's footprint
- **THEN** the coarse tile equals the tile built from the level-0 source, and the level-0 source is read only outside the covered parts

#### Scenario: A build of several levels folds the coarser from the finer [ctf_same_job]

- **WHEN** one build is asked for every tile of a fine level and a tile of a coarser level, in any order, with nothing cached
- **THEN** the level-0 source is read only for the fine level's tiles, and every tile equals its level-0 build

#### Scenario: Stale tiles are not folded from [ctf_stale]

- **WHEN** the finer tiles under a coarse tile are cached and then damaged before the coarse tile is built
- **THEN** the coarse tile is built from the current level-0 source

#### Scenario: A worker folds from the cache and lands its intermediate tiles [ctf_worker]

- **WHEN** a large document's fit level is cached and the canvas asks a worker for the coarsest level
- **THEN** the landed coarsest tiles equal a whole rebuild, the composite is not read, and the levels between are cached as well

### Requirement: Halving takes an interior fast path with the same bytes

The 2×2 halving of a premultiplied region SHALL produce the same bytes
whether a block lies wholly inside the level and the region or is clipped by
either.

#### Scenario: The fast path matches the general rule [ctf_halve]

- **WHEN** regions of odd and even sizes, at the level's edge and inside it, are halved
- **THEN** every output pixel equals the rounded average of its block's in-bounds pixels

### Requirement: The first fit-to-screen picture is measured

The time to the first fit-to-screen picture of the M12 fixture at 17000²
SHALL be measured before and after this change on the same binary and
reported against the eager baseline.

#### Scenario: M12 first paint is reported [ctf_m12]

- **WHEN** the M12 fixture is opened and fitted to screen
- **THEN** the time until the exact fit-level and coarsest tiles have landed is recorded with the eager baseline and the #1218 result
