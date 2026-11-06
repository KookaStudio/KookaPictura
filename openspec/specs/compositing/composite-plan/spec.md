# compositing/composite-plan Specification

## Purpose
Resolves a layer stack's per-layer composite state once for a composite call and
reuses it for every band, row, and region, so banded compositing does not repeat
metadata work per band while producing identical pixels.

## Requirements

### Requirement: One plan serves every band and region of a composite

The compositor SHALL resolve each visible layer's composite state once per
composite call and SHALL reuse that resolved state for every band, row range, and
region of that call. Every entry point — whole-canvas, banded, sub-region,
native-depth, and render-session — SHALL perform one resolution per layer per
composite call, with the entry point's clip and effect region as inputs. A band
MUST NOT repeat route selection, adjustment decoding, effect decoding, pass
building, source building, mask resolution, native-plane lookup, or content-key
computation.

#### Scenario: A banded composite resolves each layer once [cp_once]

- **WHEN** a document taller than two bands with a pixel layer, an adjustment
  layer, a proxy-less type layer, a smart-filtered layer, and a styled layer is
  composited
- **THEN** every layer's route, adjustment, effects, passes, source, mask gates,
  and native planes are resolved once for the composite, and every band reuses
  them

#### Scenario: Banded and whole composites stay identical [cp_identity]

- **WHEN** the same stack is composited in bands and on one canvas
- **THEN** the two outputs are byte-identical

### Requirement: Content resolves at most once per layer per composite

For one layer in one composite, the compositor SHALL decode its adjustment or
lookup table at most once, decode its effect descriptor at most once, build or
fetch its effect passes at most once, and build or fetch each cached source
raster (type, smart source, or smart-filtered) at most once, regardless of the
number of bands or regions. A region-sized generated fill buffer (a gradient or
pattern fill over a band) MAY be built per band.

#### Scenario: An adjustment layer decodes once per composite [cp_adjust_once]

- **WHEN** a document with a Curves layer and a Color Lookup layer is composited
  in bands
- **THEN** each adjustment decodes exactly once for that composite, including the
  embedded `.CUBE` parse

#### Scenario: A source layer resolves once per composite [cp_source_once]

- **WHEN** a document with a proxy-less type layer, a smart-filtered layer, and a
  styled layer is composited in bands
- **THEN** each source raster and effect pass is fetched or built once for that
  composite

#### Scenario: A hidden layer resolves nothing [cp_hidden]

- **WHEN** a layer is not visible
- **THEN** the plan resolves no route, source, effect, or mask for it and the
  composite is unchanged

### Requirement: Content keys come from plane identity, not bytes

The compositor SHALL derive a layer's content key from plane stamps or idents and
stable model values. Computing a key MUST NOT read image-plane bytes and MUST NOT
materialize a fractured plane into one contiguous allocation. The smart-filter
`options` plane contributes its ident, and a smart-object payload without a
cached digest MAY be hashed once per composite.

#### Scenario: A fractured plane is not materialized [cp_no_materialize]

- **WHEN** a banded composite runs over a document whose channel plane has
  diverged bands and whose layer is styled but carries no adjustment
- **THEN** the plane-materialization counter does not increase across the band
  loop after the plan build

#### Scenario: An edited layer still rebuilds [cp_invalidate]

- **WHEN** a layer's channel, mask, adjustment, effect, payload, or type model
  changes and the document is composited again
- **THEN** the new content resolves and the output equals a purge-then-composite

### Requirement: Synthetic composite layers reuse resolved content

When the compositor composites a derived variant of a layer (a clipping base at
full opacity, its coverage pass, or an adjustment's gated blend), it SHALL reuse
that layer's resolved content — its source, effects, and passes — and SHALL
apply only the attribute overrides (opacity, fill, blend mode, mask presence,
Blend If presence). The group arm (pass-through versus isolated) MAY be
re-decided when an override changes blend mode, opacity, or mask presence, so a
variant reproduces the current synthetic clone's routing. It MUST NOT deep-copy
the layer model or re-resolve its content for the variant.

#### Scenario: A clipping group reuses its base's content [cp_clip_reuse]

- **WHEN** a clipping group's base is an adjustment or type layer and the
  document is composited in bands
- **THEN** the group's coverage and opaque composites reuse the base's resolved
  content and decode it once, the proxy composite reuses the group canvas, and
  the output stays byte-identical to the pre-change result

### Requirement: The plan changes no output

Every composite and saved file MUST stay byte-identical to the pre-change result.
Banded, whole, region, parallel, and serial execution MUST remain byte-identical
to each other.

#### Scenario: Existing oracles are unchanged [cp_oracles]

- **WHEN** the document, clipping, knockout, adjustment, merge, native-depth, and
  reference-fixture oracle suites run
- **THEN** every output buffer and encoded file is byte-identical to its recorded
  result

### Requirement: Banded preparation permits concurrent layer-effect builds

Before rendering any band, a banded CPU composite SHALL prepare the whole-layer
effect passes of its visible layers, including descendants of visible groups,
making independent layer builds eligible for concurrent execution when multiple
workers are available. Actual build intervals need not overlap for arbitrary
workloads or worker schedules. Preparation SHALL preserve the document's layer
and effect stacking order and SHALL resolve each layer once for that composite.
Hidden layers and descendants of hidden groups MUST NOT have their effects
built. A single-worker
execution SHALL complete with the same output.

#### Scenario: Two independent styled layers build concurrently [pep_overlap]

- **WHEN** a controlled cold banded test contains two visible styled layers with
  distinct effect-pass cache keys, executes on a dedicated two-worker pool, and
  synchronizes actual build entry with a bounded rendezvous
- **THEN** the effect-pass build intervals overlap before the first band renders
- **AND** each layer's whole-layer passes are built once for that composite

#### Scenario: Visible group descendants participate [pep_nested]

- **WHEN** a controlled cold banded test contains two styled descendants of
  visible groups with distinct effect-pass cache keys, executes on a dedicated
  two-worker pool, and synchronizes actual build entry with a bounded rendezvous
- **THEN** their effect-pass build intervals overlap and each layer resolves once
- **AND** the composite equals the single-canvas composite byte for byte

#### Scenario: Hidden subtrees do no effect work [pep_hidden]

- **WHEN** a banded composite contains a hidden styled layer and a hidden group
  with a styled child whose own visibility is enabled
- **THEN** no effect passes are built for either hidden layer or the child
- **AND** the pixels equal the composite with those hidden subtrees removed

#### Scenario: One worker completes exactly [pep_one_worker]

- **WHEN** a cold banded composite with multiple styled layers runs with only
  one worker
- **THEN** it completes without waiting for another worker and equals the
  single-canvas composite byte for byte

### Requirement: Parallel preparation preserves effect reuse and exact pixels

A banded composite SHALL retain each layer's resolved whole-layer effect passes
through the lifetime of that composite and replay them without rebuilding or
looking them up again per band, even when the shared cache cannot retain them.
An unchanged subsequent composite SHALL reuse passes still resident in the
shared cache. Each visible layer SHALL resolve its passes at most once for that
composite; this does not require one physical build per layer when cache keys
are shared. Shared-key lookups SHALL preserve the existing non-waiting worker
behavior, allowing duplicate in-flight builds and returning the first published
artifact while that cache entry remains resident. An edited layer SHALL rebuild
its invalidated passes through the existing content identity rules. Full and
tall sub-region banded output SHALL remain byte-identical to the corresponding
single-canvas output, including
layer order, blend modes, masks, and clipping. Non-banded entry points SHALL
retain their existing layer-resolution scheduling and region-pass behavior.

#### Scenario: Cold and warm banded composites reuse passes [pep_reuse]

- **WHEN** a multi-layer styled document whose layers have distinct effect-pass
  cache keys is composited in bands cold and then unchanged with its passes
  resident in the cache
- **THEN** each distinct pass key builds once for the first composite and zero
  times for the second, and the outputs are byte-identical

#### Scenario: Evicted passes remain available to every band [pep_eviction]

- **WHEN** a cold banded composite has styled layers with distinct effect-pass
  cache keys and passes exceeding the shared cache's retention budget
- **THEN** each distinct pass key builds once for that composite and every band
  reuses them, producing the same pixels as a single-canvas composite

#### Scenario: Shared-key layers complete with exact output [pep_shared_key]

- **WHEN** two visible styled layers share a cold effect-pass cache key and a
  banded composite prepares them on a multi-worker pool with sufficient budget
  to retain that entry
- **THEN** the composite completes without workers waiting on an in-flight build,
  each layer resolves once, and both receive the same published pass artifact
- **AND** either a cache hit or duplicate in-flight builds is accepted, no band
  performs another lookup or build, and the pixels equal a single-canvas composite

#### Scenario: A changed style rebuilds only the changed layer [pep_invalidate]

- **WHEN** one of two styled layers with distinct effect-pass cache keys changes
  its effect parameters after a warm banded composite while the unchanged
  layer's passes remain resident
- **THEN** only the edited layer's effect passes rebuild
- **AND** the result equals a purge-then-composite of the edited document

#### Scenario: Full and tall sub-region output remain exact [pep_exact]

- **WHEN** a document with multiple styled layers, raster and vector masks,
  clipping, and groups is composited in bands over its full extent and an offset
  sub-region taller than two bands
- **THEN** both outputs equal the matching pixels of the single-canvas composite
- **AND** parallel and serial preparation produce identical bytes

#### Scenario: Non-banded preparation keeps its scheduling [pep_scope]

- **WHEN** the renderer uses an ordinary single-canvas, short-region,
  native-depth, or render-session plan entry point without dispatching to banding
- **THEN** layer resolution uses its existing sequential traversal
- **AND** a non-banded sub-region still builds transient region-sized effect passes

### Requirement: Feather preparation never waits on an in-flight worker build

Feather-cache construction SHALL execute outside cache locks and thread-local
borrows, and lookups MUST NOT wait on an in-flight feather build during nested
Rayon execution. Concurrent misses MAY construct duplicate planes but SHALL
return the first published plane while it remains resident. Existing mask
coverage, cache identities, and cache entry limits SHALL remain unchanged.

#### Scenario: Plain layers share cold feathered masks safely [pep_feather_shared]

- **WHEN** visible plain pixel layers share a cold raster or vector feather-mask
  key and a banded plan prepares them on one or multiple workers
- **THEN** the composite completes within the regression check's bounded wait
- **AND** its bytes equal sequential preparation and a single-canvas composite

#### Scenario: Scalar feather sampling reenters its own build [pep_feather_reentrant]

- **WHEN** a Rayon worker initializing a cold feather key executes a queued
  same-key scalar lookup before its outer initializer finishes
- **THEN** both lookups complete without waiting or a thread-local borrow panic
- **AND** the first published coverage remains authoritative while resident

#### Scenario: A stolen half looks up an in-flight feather key [pep_feather_stolen]

- **WHEN** a feather build waits for a stolen Rayon half whose worker looks up
  that same in-flight key
- **THEN** both lookups complete and share the first published resident plane

### Requirement: Cold styled compositing meets the workstation latency target

The supplied profiling workload's cold CPU composite SHALL complete within
1000 ms on the target workstation while remaining byte-identical to the original
CPU output. Profiling SHALL report first-frame and repeated cold timing, warm
timing, remaining stage costs, worker count, and peak memory. Cold timing SHALL
begin after decode and a source-cache purge and end when the exact composite is
ready; it MUST NOT substitute reduced-resolution or approximate pixels. Recorded
artifacts SHALL omit the input name, path, and layer names. A supplied original
reference SHALL already exist and MUST NOT be generated or overwritten through
the current renderer by either profiling harness.

#### Scenario: The supplied workload meets the cold goal [pep_cold_goal]

- **WHEN** the supplied workload is decoded and its exact CPU composite is timed
  with a cold source cache on the target workstation
- **THEN** the first and median repeated cold composite each take at most 1000 ms
- **AND** all bytes equal the original CPU reference

#### Scenario: Profiling requires an existing independent reference [pep_reference_required]

- **WHEN** a profiling run is given a path for its original CPU reference
- **THEN** it reads the existing independently generated bytes without modification
- **AND** an absent reference fails rather than creating a baseline through the
  current renderer
