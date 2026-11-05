# Spec Delta

## MODIFIED Requirements

### Requirement: Effect passes build for the composite region

A non-banded sub-region composite SHALL reuse an already-ready whole-layer
effect pass whose content key matches the current layer and document, unless
the pass build reads the document pattern library. Reuse SHALL replay only the
intersection with the requested region, without copying the whole pass or
compositing the full document. A region's ready-pass probe SHALL NOT initiate
or wait for a whole-layer build and SHALL NOT create a cache entry on a miss.
An absent, evicted, oversized-and-unretained, or still-building whole-layer
pass SHALL cause a transient region build.

For such a transient build, the compositor SHALL build each enabled effect
pass over the requested region grown by that effect's build reach, and the
resulting pass SHALL equal the same rectangle of a pass built over the whole
layer, byte for byte. The build reach SHALL be the effect's summed stage
support plus its sample offset where the effect reads a shifted sample: drop
shadow and inner shadow grow by `reach + |ox| + |oy|`, satin already includes
its offset in its pad. The build SHALL sample the layer content, the raster
mask, and the vector mask only over that grown region, or over one shared
per-layer region that covers every enabled family's reach, and SHALL clip
every newly built pass output to the requested region. A gradient or pattern
overlay's newly generated source buffer SHALL likewise cover only the region
its pass covers.

A full-canvas composite SHALL keep reusing the content-keyed cached pass.
Newly built non-banded sub-region passes SHALL be transient and SHALL NOT be
retained in the shared cache, so a stroke cannot evict other layers' sources.
Pattern-driven passes SHALL continue rebuilding without whole-layer pass
reuse. A painted channel or edited mask SHALL invalidate reuse of its prior
content's passes; unchanged layers MAY reuse their ready passes during the
same gesture. A banded composite MAY build the whole-layer pass and replay it
into each band, keeping the cached full pass. Both reused and transient region
output SHALL equal the full composite's slice.

#### Scenario: A region pass equals the full pass

- **WHEN** a document with a drop shadow, outer glow, inner shadow, inner glow,
  stroke, satin, bevel, and color, gradient, and pattern overlay is composited
  over a sub-region and whole
- **THEN** every pixel of the region composite equals the same pixel of the full
  composite, for every effect family

#### Scenario: A full composite still reuses its cached pass

- **WHEN** an unchanged styled document is composited whole twice
- **THEN** the second composite reports a pass-cache hit and rebuilds no effect
  pass

#### Scenario: A stroke present rebuilds only the dab and its reach

- **WHEN** a dab is painted on a canvas-sized layer carrying a layer style and
  the dab's non-banded present region is composited without a ready pass for
  the new content
- **THEN** each pass is built over the present region grown by its reach, not
  over the layer, and the composited pixels equal the same slice of a full
  composite

#### Scenario: An overlay source is bounded by the region

- **WHEN** a canvas-sized layer carries a gradient or pattern overlay and a
  non-banded sub-region is composited without a reusable whole-layer pass
- **THEN** the overlay's source buffer covers only the pass's region and the
  result equals the full composite's slice

#### Scenario: A banded sub-region reuses one pass

- **WHEN** a tall sub-region is composited in bands
- **THEN** one whole-layer pass is built and replayed into every band, and the
  output equals the whole composite

#### Scenario: A sub-region pass is not retained

- **WHEN** a non-banded sub-region composite runs
- **THEN** the shared cache does not gain a pass entry for that region

#### Scenario: A warm short region reuses the full pass [rep_warm]

- **WHEN** a whole composite has cached an unchanged non-pattern styled
  layer's passes and a non-banded sub-region is composited
- **THEN** the region reuses those passes, builds no effect passes, and equals
  the corresponding full-composite slice

#### Scenario: A cold short region stays bounded [rep_cold]

- **WHEN** no ready whole-layer pass exists for a non-pattern styled layer
  and a non-banded sub-region is composited
- **THEN** it builds only transient bounded region passes, creates no
  whole-layer pass entry, and equals the full-composite slice

#### Scenario: An in-flight whole-layer pass does not delay a region [rep_inflight]

- **WHEN** a whole-layer pass build is in flight and a non-banded region of
  the same layer is requested
- **THEN** the ready-pass probe returns a miss without waiting for that build,
  and the transient region result equals the full-composite slice

#### Scenario: Paint and mask edits reject warmed prior content [rep_edit]

- **WHEN** a whole composite has warmed a styled layer's passes and its
  channel or raster mask is subsequently edited before a short region present
- **THEN** each edit causes fresh bounded region passes rather than reuse of
  prior-content passes, and the output equals a fresh full-composite slice

#### Scenario: Pattern edits remain fresh [rep_pattern]

- **WHEN** a pattern-driven styled layer is composited whole and over short
  regions before and after editing its referenced document pattern
- **THEN** each region builds transient passes using the current pattern,
  equals its current full-composite slice, and retains no pass entry

## ADDED Requirements

### Requirement: Warm bevel regions avoid effect-build cost

An ignored CPU profile SHALL measure a fixed, generated beveled-layer document
after a full composite warms its retained passes. The acceptance run SHALL use
one rendering worker and no concurrent tests in its process. It SHALL compare
the sum of two adjacent 128-row non-banded CPU composites with a single
256-row banded CPU composite covering exactly their union, at the same width.
After three untimed warm-up rounds, it SHALL measure nine rounds, alternating
which workload runs first, and assert that the median summed short-region time
divided by the median banded time is at most 4.0. The profile SHALL report both
medians, the ratio, a cold 128-row timing, and a warm full-composite timing.
Output equality checks SHALL occur outside the timed intervals and SHALL
compare both workloads byte for byte with the corresponding full-composite
slice.

The warm short-region path SHALL build no effect passes; deterministic unit
tests SHALL verify that directly, counting transient as well as cached builds.
The integration profile SHALL separately assert the fixture's expected cached
pass reuse through observable cache-hit deltas, zero warm source-cache misses,
and zero effect decodes. Generic source-cache metrics alone SHALL NOT be
reported as direct effect-build counters. The timing bound SHALL be asserted
only in the explicitly invoked ignored profile, not ordinary CI tests.

#### Scenario: A warm bevel profile isolates replay [rep_profile]

- **WHEN** the ignored profile is explicitly run on the generated bevel fixture
- **THEN** its single-worker median pair-to-union timing ratio is at most 4.0,
  it reports cold and warm timings, asserts the fixture's expected cache reuse
  signals, and verifies both workloads against the full-composite slice
