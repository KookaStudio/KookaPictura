# Design

## Context

See proposal.md for motivation. `composite_banded` currently calls
`CompositePlan::build(doc, clip, None)` before rendering band 0. `plan_layers`
uses a sequential iterator; `plan_node` resolves content, effects, cached
passes, blend state, and source planes, then recursively resolves children.
Bands already borrow the resolved nodes and retain their `Arc<LayerPasses>`.
The comment that band 0 warms effect passes predates the plan and is stale.

The existing plan contract requires one resolution per layer per composite.
The cache builds outside its global mutex, never blocks Rayon workers on an
in-flight key, and can evict or decline large artifacts. These contracts still
apply. Pattern-driven effects already exclude a stack from banding.

## Goals / Non-Goals

**Goals:** Allow independent styled layers to prepare concurrently before band replay;
keep deterministic replay order, visible-tree traversal, and exact pixels;
retain passes through the composite even if the cache evicts them.

**Non-Goals:** Change effect math, source-cache synchronization or budgets, the banding
eligibility test, or non-banded scheduling. The expanded latency goal includes integrating the exact per-effect blur
and geometry work from #1178 / PR #1221, then optimizing measured residuals. No new dependency or public configuration is needed.

## Decisions

### 1. Give banding an explicit parallel plan entry point

Add an internal banded builder to `CompositePlan` and use it only from
`composite_banded`. Share node resolution with existing builders, threading the
parallel traversal choice through visible groups. Use indexed Rayon traversal
and ordered collection for sibling lists; complete the plan before any band
runs. All other builders keep sequential traversal, even when their effect
region is `None`: that value alone does not identify a banded call.

Parallel traversal makes independent nodes eligible to overlap, without
guaranteeing overlapping build intervals for every workload. Short builds,
imbalanced preceding source preparation, or busy workers can serialize actual
build intervals while preserving this scheduling contract.

Resolve whole nodes in parallel rather than performing a separate pass-cache
warm-up. The latter repeats routing, decoding, or cache-key work and can lose
warmed artifacts to eviction before the sequential plan reaches them. Retaining
each resolved node immediately keeps one resolution per layer and bounds the
number of pass builds per composite independently of retention budget. Other
per-layer state resolved by these banded nodes naturally shares that scheduling.

The parallel traversal must reach styled descendants as well as root layers.
Keep the early hidden-node return ahead of child traversal so a hidden group
cannot schedule its descendants. Collect nodes in input order regardless of
completion order; compositing itself remains ordered. Reuse the current pool
rather than creating threads or a second pool. A one-worker pool runs the same
traversal to completion serially.

### 2. Keep the existing cache and retain every result in the plan

Continue using `effect_passes_with` with whole-layer passes (`region = None`).
Do not change keys, cache budgets, or `Slot::resolve`. Cache mutexes must never
span effect construction or a parallel join. The plan owns the returned Arcs
until all bands finish; bands perform no pass-cache lookups or builds.

Use distinct pass keys for tests asserting one cold physical build per styled
layer. Identical keys retain the cache's
existing worker non-waiting semantics, which can duplicate an in-flight build;
this change does not promise global single-flight behavior across layers or
concurrent composite calls. The once-per-layer contract concerns resolution
per composite, not physical builds attributed to a layer. Existing build
counters aggregate by key and cannot distinguish individual shared-key layers.
Add a separate shared-key fixture that permits either a cache hit or duplicate
builds, checks one resolution per layer, bounded completion, the common published
artifact while resident, no per-band lookup/build, and exact pixels.

### 3. Preserve the band loop while updating its explanation

Keep the first-band execution and remaining band scheduling as they are for
this change; the bottleneck is the preceding plan preparation. Replace the
obsolete first-band cache-warming comment with the plan's actual ownership and
reuse behavior. Removing the separate first-band path is unnecessary to meet
#1179 and can be considered independently.

### 4. Test overlap directly and measure speed separately

Use generated, deterministic fixtures in Rust tests. Add a test-only observer
at actual effect-pass build boundaries, scoped to the test document's keys.
In a dedicated two-worker pool with distinct pass keys, use a bounded rendezvous
to synchronize actual build entry and prove both controlled builds
have entered before either finishes, and assert no band executes until they
finish. A deadline must fail cleanly rather than hang if execution serializes;
do not use a wall-time speed ratio as the concurrency assertion. This demonstrates
that the scheduling permits overlap; production profiles measure whether their
own workload overlaps without the test synchronization. Keep observers test-only
and scoped to the fixture, with guards that restore hook state.

Choose serial or concurrent layer preparation through a test-only per-call
selector passed explicitly through the banded composite's plan preparation and
visible descendants. It must not read or modify the process-global
`composite_rows::FORCE_SERIAL` flag or alter per-effect internal parallelism.
Production calls always use the normal banded scheduling choice. This isolates
overlap tests and preparation-only timing comparisons from unrelated tests
that toggle the row-kernel flag; do not add another global mode flag.

Exercise two root layers and visible nested descendants, hidden subtrees, one
worker, distinct-key cache reuse/eviction/invalidation, shared-key completion,
and exact full/offset-region output. Reuse `cache_test_guard` around purge/count
tests and budget changes. The new tests select preparation scheduling per call
rather than toggling `FORCE_SERIAL`; their concurrency assertions must remain
valid when unrelated row-kernel tests run concurrently. Extend coverage in
`tests/composite_plan.rs`, splitting new tests into a sibling module if needed
for file limits. Existing region,
source-cache reentrancy, and layer-effect checks remain regression coverage.

Add an ignored synthetic profiling test reporting cold plan/composite time,
warm time, and per-layer build intervals. Compare sequential layer preparation
against concurrent preparation using the per-call selector on the same build
and pool, leaving each effect's internal parallelism enabled in both cases.
Run the ignored profile in isolation from other test cases for timing stability.
Repeat with a locally supplied workload as a manual acceptance check; disclose
only generic workload characteristics, timings, worker count, and build overlap
in recorded results.
Keep input names, paths, and layer names out of committed artifacts and PR text.
The original intermediate objective was about 2x for two heavy layers. The
expanded absolute latency goal below supersedes that completion criterion;
report measured ratios without imposing a flaky general-CI threshold. Record
whether #1178 is present in both baselines.

## Risks / Trade-offs

- Concurrent scratch allocations increase peak memory -> Retain existing cache
  limits and record peak memory during profiling; do not add a new memory-budget
  policy in this change. The shared pool bounds running work, not total scratch
  allocations suspended in nested joins.
- Existing per-effect parallel work competes for the same workers -> Measure
  the incremental improvement with identical intra-layer behavior in each run.
- A global test hook could contaminate unrelated tests -> Filter by fixture
  keys, use RAII cleanup, bounded synchronization, and the existing test locks.
- Shared source keys may cause duplicate worker builds -> Preserve existing
  non-blocking cache semantics, check shared-key plan completion and publication
  explicitly, and run its deadlock/reentrancy regression tests.

## Migration Plan

No data migration. After tests and implementation are written, compile once
with at most 80% of available CPUs and run the focused Rust suites, profiling,
and repository gates. Rollback selects the existing sequential plan builder at
the banded call site. Validate and archive this change in the shipping PR.

## Expanded latency goal

The initial cross-layer result is an intermediate baseline, not completion.
The user now requires the supplied workload's cold CPU composite in the
100–1000 ms range. Integrate the source and scalar-oracle tests from PR #1221
(commit ce6ee48f), available locally, rather than duplicating its exact SIMD
Gaussian and parallel bevel geometry changes. PR #1221 has merged into master
as c9a53cd0; the shipping branch builds on that merged dependency.

Use stage timings to distinguish matte/mask construction, distance fields,
Gaussian rounding, shading, pass emission, and band replay. Preserve kernel tap
order, f64 accumulation and intermediate f32 rounding. Existing scalar-oracle
tests from the dependency remain authoritative. Do not change support,
resolution, floating-point accuracy, or output tolerances to meet timing.

Cold acceptance measures the first and median repeated full composite after
decode and a source-cache purge, on this workstation at a stated worker count.
Decode, disk I/O, and reference comparisons stay outside the timed interval.
Accept only after both first and median cold results are at most 1000 ms and
match the original CPU output byte for byte. General CI does not enforce a
hardware-sensitive time limit; an opt-in workstation acceptance check does.
Retain generic timings and memory evidence without input names or paths.

### Measured residuals after kernel integration

The first combined profile leaves about 1 s in pass construction and 0.7 s in
band replay. Before the bevel distance stage, matte/glow processing still costs
about 0.6 s. Resolve raster/vector mask gates once per matte instead of walking
the mask configuration per pixel. Obtain a mutable plane slice once for matte
quantization, avoiding repeated copy-on-write bookkeeping for each sample.
Fill independent constant-alpha rows directly for glow and bevel emission.

Replay effects through row slices and the existing blend arithmetic, dispatching
common constant blend modes once per pass. Keep the former per-pixel replay as a
test oracle, including Dissolve, per-pixel colors, partial alpha, effect opacity,
clipping, and knockout coverage. No arithmetic simplification is allowed.

### Production acceptance seam

Unit-test mask sampling increments a shared atomic per pixel. That instrumentation
can dominate parallel renders even in a release-mode unit binary, so unit timings
are diagnostic, not production acceptance. The ignored `effect_cold_latency`
integration test calls the non-test production library, reads the local input and
saved original scalar buffer before timing, and asserts first/median cold latency
within 1000 ms with complete byte equality. The unit profiling harness remains
useful for stage discovery. When a reference path is supplied, it must already
contain an independently generated original CPU buffer. Neither profiling
harness may create or replace that reference through the current renderer.

### Feather-cache completion under concurrent preparation

Parallel node resolution also reaches plain layers' `BlendCtx` mask gates.
Unlike the earlier sequential traversal, duplicated plain layers can resolve
the same cold feather key concurrently. Feather construction launches nested
Rayon work, so waiting on an in-flight `OnceLock` can block a worker on its own
initializer or on a stolen half needed by that initializer (#1044).

Store resolved `Arc<Feathered>` planes, perform lookups and publication under
the existing cache mutex, and build misses outside every cache lock and
thread-local borrow. Recheck before publication and return the first resident
publication; duplicate in-flight builds are allowed. Retain the existing keys,
entry capacity, generation invalidation, and immutable coverage bytes. Scalar
sampling must release its `LOCAL` borrow before a miss builds nested Rayon
work, because that work can reenter the scalar sampler on the same thread.

Bounded cache tests force same-thread queued lookup and cross-thread stolen-half
lookup while a build is in flight. A separate full-composite test covers cold
shared raster/vector feather masks on plain layers with one and multiple workers
and compares exact output with sequential preparation and a single canvas.

### Cache-local Gaussian tiles

Native CPU sampling of the production library identifies the u8 Gaussian row
accumulator as the largest residual cost. Its vertical pass streams full-width
input rows for each output row, exceeding private cache capacity at large kernel
support. Process sixteen output rows and 128 columns per tile: each output still
adds taps in exactly the old order, while adjacent output rows reuse the input
working set in private cache. Tile sizes are internal constants, not settings.
Test widths/heights on both sides of tile boundaries and kernel support larger
than the image against the existing independent scalar Gaussian.
