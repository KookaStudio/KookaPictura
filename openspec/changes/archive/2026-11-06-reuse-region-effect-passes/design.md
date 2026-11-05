# Design

## Context

See proposal.md for the reported timing discontinuity. `composite_region.rs`
sets `effect_region = Some(clip)` for short sub-regions; banding starts above
128 rows and builds its plan with no effect region. `effect_passes_with` in
`layer_source/mod.rs` bypasses caching for every region request today.

`LayerPasses` is already shared through an `Arc`. CPU replay in
`layer_effects/planes.rs` and GPU replay in `gpu/effect.rs` already intersect
document-space pass rectangles with the output region. The current cache has
Empty, Building, and Ready slots; ordinary `lookup` can initiate a build or
wait outside rayon. The new path must inspect only ready content.

The existing capability explicitly requires bounded builds on every short
region. The delta refines this to require them only when reusable passes are
unavailable, while preserving their support calculations and output equality.

## Goals / Non-Goals

**Goals:** Keep warm region cost proportional to replayed pixels, preserve
bounded cold-region work, and avoid cache pollution or new build waits.

**Non-Goals:** Change effect algorithms, the banding threshold, cache budgets,
GPU quantization, cold-build parallelism, or gesture APIs. No new dependencies.

## Decisions

### Probe ready passes using the existing whole-layer key

After the no-passes and pattern-driven checks, a region request computes the
same `Kind::Passes` key as a full request. Add a private ready-only cache probe.
Under the cache mutex, locate the slot and clone its `Arc`, then release that
mutex before inspecting slot state. Return a cloned artifact only for Ready;
Empty and Building are misses. Touch the retained entry's LRU position on a
successful hit and record a source-cache hit; record a miss on unavailable
content. Do not use `Slot::resolve`, insert a slot, publish a transient
artifact, or change resident-byte accounting. Never hold the cache and slot
mutexes together; this avoids introducing a lock-order dependency.

The ordinary whole-layer lookup retains its existing deduplication and rayon
semantics. The probe is deliberately separate: using ordinary lookup with a
region build closure could publish incomplete passes under a whole-layer key.
Waiting for an in-flight pass would also make a small region pay cold full
build latency. A concurrent eviction may leave the probe holding an already
cloned artifact safely; it must not resurrect the evicted entry.

### Reuse the shared pass directly

Return the cached `Arc<LayerPasses>` and let existing CPU/GPU replay clip it.
Copying cropped alpha/color planes adds allocations without reducing the
existing replay loops. A miss falls through to the current transient builder
with its region and resolved fill inputs intact. Pattern-driven passes skip
the ready probe because their key excludes document pattern contents.

### Invalidate by content rather than adding a stroke flag

Proposed design assumption: “active strokes rebuild” refers to content that
the stroke changes, rather than every styled layer in the document. Channel
and raster-mask writes reset `Plane::ident`; the existing pass key includes
those idents, mask settings, effect descriptors, geometry, and vector-mask
identity. The next present therefore misses the pre-edit pass and rebuilds
only the grown region. An unchanged layer can still reuse its warm pass.
This preserves freshness without threading gesture state through compositor
entry points. Tests must warm the old content before changing it so they can
detect accidental stale reuse.

### Verify routing separately from timing

Use `Arc` identity and per-layer test counters to prove warm reuse, and direct
bounded-pass inspection plus full-slice comparisons for misses. Extend test
instrumentation at `build_layer_passes` so the per-content-key unit-test count
includes every build, including transient builds that bypass `lookup`. Assert
that this count does not increase for warmed short-region requests. The current
`pass_build_count` observes lookup builds only and is insufficient alone.
Do not rely solely on global counters during parallel tests. Hold the
existing cache test guard in cache-sensitive tests. Cover ready probes of
absent, Empty, Building, evicted, and over-budget entries; coordinate the
Building case explicitly rather than timing a race.

Add an ignored profile to `tests/source_cache_profile.rs` using a fixed generated
4934x1500 8-bit RGB document: an opaque white background and two opaque Normal
pixel layers, red and blue, each with a full-canvas rect and a vector mask.
Use straight polygon masks with document-space vertices
`[(300,250), (2300,250), (1300,1250)]` and
`[(2600,250), (4600,750), (2600,1250), (3100,750)]`, respectively. Each styled
layer enables Smooth bevel (size 250, depth 1000, soften 0) and inner glow;
fix all remaining parameters in the fixture source, with no pattern effects,
smart objects, type layers, random values, or external assets. This is a
performance fixture for the supported model, not a Photoshop oracle.

Measure adjacent document-space regions `(0,512,4934,128)` and
`(0,640,4934,128)` against their union `(0,512,4934,256)`, where each tuple is
`(left,top,width,height)`. Summing the two short-region timings gives exactly
the same covered pixels and effects as the banded reference; no area or
equal-content-density approximation is needed. Execute all composites inside
one dedicated rayon pool created with `num_threads(1)`, so the reference gains
no parallelism from the machine's CPU count. Build the fixture and take the
cold short-region timing before the full warm-up. Verify whole-layer passes
fit the existing cache budget and stay retained, then warm the full composite.

Run three untimed rounds, followed by nine measured rounds. In each round,
time the two short composites separately and add their durations, and time the
single union composite. Alternate pair-first and union-first order by round.
Take the median of each workload's nine samples and assert
`median(t_top128 + t_bottom128) / median(t_union256) <= 4.0`. Four allows the
extra plan/allocation overhead of two calls while rejecting a large cost cliff;
the same-pixel, single-worker reference makes this bound independent of core
count. Compare each result to the warmed full-composite slice outside timing.
Report both medians, ratio, cold short time and warm full time. Optional
multi-worker timings are diagnostics and do not change this acceptance test.

The integration profile cannot read private `cfg(test)` unit-test build counts.
For this pixel-only fixture, assert the expected public `source_hits` deltas
for warmed short requests: one decoded-effect hit and one ready-pass hit per
styled layer, four hits per call with the current routing. Also assert zero
warm `source_misses` and `effect_decodes`. Document that these are reuse
signals; the unit tests above supply direct zero-build evidence. Reconcile
the documented expected hit delta if routing changes, preserving the explicit
pass-hit contribution. Run the profile alone to isolate its process-global
metrics:

`cargo test -p pictura-render --test source_cache_profile warm_bevel_region_replay_profile -- --ignored --exact --nocapture --test-threads=1`

The wall-time ratio assertion belongs only to this explicitly invoked ignored
profile. Ordinary CI tests assert routing, freshness, bounds and exact pixels.

## Risks / Trade-offs

- A whole pass larger than the cache budget cannot benefit -> preserve bounded
  transient fallback and include eviction/over-budget probe coverage.
- A test could accidentally enable pattern overlay and never exercise reuse
  -> warm fixtures explicitly exclude pattern-driven effects; pattern tests
  are separate and mutate the referenced pattern.
- Global cache purges or metrics can race -> use the existing guard and
  artifact identity or per-key instrumentation.
- CPU and GPU have different existing tolerances -> CPU byte equality remains
  the oracle; retain existing GPU parity tests without widening tolerances.
- CPU-count-dependent comparisons can obscure the regression -> the acceptance
  comparison uses the same pixels and one rayon worker for both workloads;
  optional parallel measurements remain diagnostic.
- Public metrics do not count transient effect builds -> use fixture-specific
  cache-hit signals in the isolated integration profile and direct all-build
  instrumentation in the warmed-reuse unit tests.

## Migration Plan

No data migration or public API change is required. Land the cache probe,
region selection change, and regression tests together. Run the planned tests
after implementation is complete, then the strict OpenSpec and repository
gates. Archive the change in the shipping PR. Rollback restores unconditional
transient short-region builds without changing persisted files.
