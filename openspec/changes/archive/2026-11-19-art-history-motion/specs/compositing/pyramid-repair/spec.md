# pyramid-repair delta

## ADDED Requirements

### Requirement: A repair builds only the levels it can patch

`TilePyramid::repair` SHALL build level 0's straight interleaved rows only when
a current level-0 tile of the repaired source meets the region. It SHALL halve
each level from the previous one without copying its input. After a repair,
every cached tile SHALL equal a tile rebuilt from the edited document.

#### Scenario: A repair with only coarse tiles cached matches a rebuild [pr_coarse_only]

- **WHEN** only levels above 0 are cached and a region of the document is edited and repaired
- **THEN** no level-0 tile appears and every cached tile equals a rebuild

#### Scenario: A repair patches cached level-0 tiles [pr_level0]

- **WHEN** levels 0 to 2 are cached and an edited region is repaired
- **THEN** every cached tile equals a rebuild, and a tile outside the damage keeps its allocation

### Requirement: Interleaving runs on every core

`interleave` SHALL convert straight planar RGBA to interleaved RGBA a row at a
time in parallel, with output byte-identical to a serial copy.

#### Scenario: Parallel interleave matches a serial copy [pr_interleave]

- **WHEN** a composited region is interleaved
- **THEN** the bytes equal a serial plane-to-pixel copy
