## MODIFIED Requirements

### Requirement: Composite a region at a pyramid level

The renderer SHALL provide `composite_region(doc, rect, level)`, which returns
the composite of the document over `rect` at pyramid `level`, without compositing
or storing the rest of the document. At level 0 the result SHALL be straight-alpha
working-space RGBA and SHALL be byte-identical to the same rectangle cropped from
the whole-document composite. At a level k above 0 the rectangle SHALL be given
in level-k pixels and the result SHALL be premultiplied RGBA byte-identical to the
same rectangle from an independent whole-composite reduction following the
Reduced-level pixel semantics requirement, including odd rows and columns.
The whole-composite reduction is a test reference, not a production allocation
requirement.
Pixels of `rect` outside the level SHALL be transparent.

#### Scenario: A level-0 region equals the full crop [rc_level0]

- **WHEN** a region is composited at level 0 for seeded documents covering every blend mode the oracle covers
- **THEN** it equals the same rectangle cropped from the whole composite, byte for byte

#### Scenario: A level-k region equals the pyramid crop [rc_levels]

- **WHEN** regions are composited at levels 1 through 3, including rectangles that touch odd-sized right and bottom edges
- **THEN** each equals the same rectangle cropped from that level of a pyramid rebuilt from the whole composite, byte for byte

#### Scenario: A region past the level is transparent [rc_outside]

- **WHEN** a requested rectangle extends past the level's bounds
- **THEN** the part outside is fully transparent and the part inside matches the crop

## ADDED Requirements

### Requirement: Reduced-level pixel semantics

Each successive reduced level SHALL have dimensions `ceil(width/2)` by
`ceil(height/2)`. Before the first reduction, each straight-alpha color sample
SHALL be premultiplied as `floor((color * alpha + 127) / 255)`; alpha SHALL stay
unchanged. Each output channel SHALL be `floor((sum + floor(n/2)) / n)` over
the `n` samples present in its aligned 2x2 block. An odd last row or column
SHALL average only existing samples, without adding transparent padding or
replicating an edge. Further reductions SHALL average the already premultiplied
previous level with the same rounding at each step. A multi-level reduction
MUST NOT replace those successive rounded operations with one larger average.
A display crop at level 0 SHALL premultiply its straight source; pixels outside
any level SHALL be transparent. These rules SHALL define values independently
of whether pixels are stored, tiled or generated as a region.

#### Scenario: An odd edge averages only its existing samples [rp_odd_edge]

- **WHEN** a region includes a last row or column of an odd-sized level
- **THEN** its reduced pixels equal successive rounded averages of only the existing samples and match the independent whole-rebuild reference

#### Scenario: A soft edge is premultiplied before averaging [rp_soft_edge]

- **WHEN** opaque and partially transparent colored samples are reduced together
- **THEN** their colors are premultiplied before averaging, and the returned channels equal the independent reference byte for byte

#### Scenario: Multiple reductions preserve per-level rounding [rp_rounding]

- **WHEN** a region is requested at levels 1 through 3
- **THEN** its bytes equal a whole reference reduced one level at a time, including odd edges and transparent out-of-level pixels
