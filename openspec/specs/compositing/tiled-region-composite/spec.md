# compositing/tiled-region-composite Specification

## Purpose
A large region composites into tiles a strip at a time, patches the stored composite without materializing, and makes its display copy only when a display update reads it.

## Requirements

### Requirement: A large region composites into tiles

The banded compositor SHALL write a region of 16 Mpx or more into a tiled
buffer one group of 256 rows at a time, and SHALL keep a flat buffer for a
smaller region. The tiled result SHALL be byte-identical to the flat one.

#### Scenario: The tiled region equals the flat region [trc_exact]

- **WHEN** a bandable stack with offset layers, a mask, a group with a clipped child and several blend modes is composited over a region at the tiling threshold and over a sub-region below it (the threshold lowered in tests so the fixture reaches it)
- **THEN** the large region is tiled and byte-identical to the one-canvas composite, and the small region stays flat

### Requirement: Patching the composite reads a tiled region by tiles

Patching the stored composite with a tiled region SHALL read the region row by
row through its tiles and SHALL NOT materialize it.

#### Scenario: A patch does not materialize [trc_patch_tiled]

- **WHEN** a tiled 512 × 512 region is patched into a tiled composite
- **THEN** the patched pixels equal the region's, and the plane materialization counter is unchanged

### Requirement: The display copy of a region is made on demand

A composited region's display (sRGB) copy SHALL be produced only when a display
update reads it. A deferred whole-document landing on a large document SHALL
NOT produce it.

#### Scenario: A deferred whole landing skips the display conversion [trc_srgb_lazy]

- **WHEN** a whole-document region of a document with a non-sRGB profile is landed by the deferred composite on a document over the synchronous tile limit
- **THEN** the landed region equals the document's composite, the display tiles are invalidated, and no display conversion ran
