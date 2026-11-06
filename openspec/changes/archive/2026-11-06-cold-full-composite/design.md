# Design

## Context

See proposal.md for the reported latency and the diagnostic-only scope.

- The branch now includes the upstream cold-composite work: #1178 (cold
  layer-effect composites), #1179 (cold styled composites), and #1180 (reuse
  ready effect passes for short regions). Those changes added
  `tests/effect_cold_latency.rs`, `tests/bevel_cold_composite_profile.rs`, and
  `src/tests/effect_preparation/{probe,profile}.rs`.
- The existing profiles require a locally supplied PSD or cover one generated
  bevel scene. They do not reproduce the reported 4934x1500 nested/clipped/
  smart-object/type stack, do not separate application opening from the first
  edit, and do not implement the specified comparator.
- `PictureViewRust::composite_into_doc` records full-composite render time;
  presentation reset follows separately. A banded CPU composite builds one
  `CompositePlan` before parallel execution. Sources, effect passes, and mask
  gates can perform cached first-use work. `content_route` uses pixels for an
  unfiltered smart object with a proxy; embedded decoding is a candidate
  category, not an established cause.
- `VectorMask::coverage_sampler` initializes a smooth row cache; feather and
  effect caches have separate lifetimes. A source-cache purge is not a fresh
  document/process boundary.
- Application open is more than PSD decoding: `decode_psd_with` calls
  `store_opened_composite`, builds target/history, and `apply_opened` replaces
  the view's document. The existing `open_psd_matches_sync` test demonstrates
  generated PSD -> decode -> default Rust view -> apply without a GUI suite.

## Goals / Non-Goals

**Goals:** provide reproducible first-render and actual first-edit evidence,
phase attribution, an honest backend report, and a specified comparison rule.

**Non-Goals:** optimize production algorithms, select an unmeasured cause,
warm caches at open, add a compositor/cache architecture, change pixel quality,
or claim the reported issue fixed.

## Decisions

### 1. A complete diagnostic change precedes optimization

Write all diagnostic code, fixtures, and tests; then build once at the end,
with at most 80% of available CPUs. Run the new executable to collect attribution
only after that build. Compiler-error fixes follow the normal rebuild rule.
This prerequisite has no algorithm-selection dependency and needs no compatible
pre-existing executable. Profiling may use `perf` after the diagnostic build;
missing profiler privileges must be reported, not treated as attribution.

Record measured results and the still-unresolved questions in the completed
change. A later, explicitly proposed change selects and implements the fix,
using this already-built diagnostic harness and baseline. It must preserve the
same harness/protocol when measuring the candidate, including any necessary
profiling-only patch on its baseline revision. This change's completion means
that diagnostics work, not that cold rendering became faster.

Alternative rejected: forbid compiling the new timers until an optimization
chosen from those same timers has been implemented. That dependency is cyclic.

### 2. Keep engine-cold and application-first-edit protocols distinct

Both protocols launch an exact child test per sample/backend in a fresh process,
with a recursion guard, explicit fixed thread count, and no hidden warm-up.
Generate test inputs before launching timed children. Record opaque input id,
protocol version, edit recipe and direction, dimensions, machine identity, build
configuration, thread count, requested/executed backend and fallback reason,
metrics mode, revision, and raw timings. Never emit input or layer names, paths,
document pixels, or document metadata unrelated to comparison.

The engine protocol times decoding separately, then one first full composite,
repeated full renders, and a bounded region. No query may prepare that child's
sources, masks, effects, or GPU before the first full measurement. GPU device
initialization is timed inside the first-full span and labelled as a sub-phase.
Digests run outside timers through a deterministic digest hook. This protocol
isolates kernels; its decode time is not application opening.

The application protocol lives in `pictura-app` Rust tests/profile code. It uses
production `decode_psd`/`apply_opened` and records the complete decode/apply
opening interval, including opening composite when performed, target/history,
and document replacement/display preparation. Do not purge or reset caches
between opening and edits. An opening render is part of opening, not the
first-edit sample. The opening-composite assertion uses an 8-bit generated
fixture with no retained source planes, or reports that the production path
skipped opening compositing.

Then mutate a visible contributing layer (a generated fixture uses an opacity
change) and invoke the production edit transaction with `Edit::Full`. Assert
that rendered pixels change and exactly one full composite occurred; a neutral
insert/move is invalid. The measured recipe is fixed-direction: an unmeasured
reset to the starting state, then a measured edit to the alternate state.
Direction is recorded; opposite directions are never combined into one median.
The first-edit boundary includes the composite store and presentation reset
(`reset_pyramid`/present), or reports presentation reset as a separate metric.
Measure a warm bounded region through the application refresh boundary. Compare
identical resulting states.

Generated nested/clipped/masked/styled documents are the default, with hidden
type and proxy/filtered source variants. A small fixture supports default tests;
large profiles are ignored with an invocation reason. The application profile
may accept an explicit local input through `PICTURA_PROFILE_PSD` and a compatible
edit target chosen locally, without storing either in reports. A set-but-missing
or incompatible input fails with a redacted diagnostic; it is not skipped or
silently replaced by a generated sample. Normal automated checks require no
external data.

Alternative rejected: report construction or raw decode as application opening,
or use an engine-only scene to prove a first-edit application regression fixed.

### 3. Attribute coarse phases without changing rendering

Reuse the render metrics opt-in gate. Children that need attribution set
`PICTURA_METRICS=1` in their spawn environment; `set_for_this_thread` remains a
unit-test helper for the app phase recorder. Measure plan construction,
embedded-source decode, effect construction, raster/vector mask preparation, and
composite execution/output conversion. Plan time is inclusive; nested spans are
labelled and never summed twice. Phases not executed are absent, distinct from
measured zero. Aggregate worker-thread measurements explicitly; do not rely on
app thread-local accumulation to see worker spans. Avoid per-pixel timing and
always-on detailed timing state. Report tests cover disabled metrics and exact
output with metrics on/off. Existing cache and render algorithms remain intact.

### 4. The comparator has explicit, conservative outcomes

The normative comparison rule is in the spec. Use integer nanoseconds and
doubled medians to avoid NaN/rounding-dependent comparisons. For even sample
counts, `median2` is the sum of the two middle values; for odd counts it is twice
the middle value. A noisy batch is `20 * range > median2`; equality is not
noisy. Improvement and regression compare doubled medians against `2 * V`.
Never discard outliers or silently retry until a run passes. The 10%
range/median noise limit is a fixed diagnostic policy, not a universal
performance target or a claim of statistical significance.

Application comparisons use cold end-to-end time (opening wall + first-edit
wall, metrics off) as the primary; controls are repeated full-edit median and
warm-region time. Engine comparisons use cold end-to-end time (decode + first
full, metrics off) as the primary; controls are repeated-full median and region.
Opening, first-edit, decode, and first-full times are reported separately.
Phase breakdown is supporting evidence and is not a substitute for the
end-to-end metrics-off primary. This primary cannot be gamed by moving work
between opening and the first edit.

Select one protocol, input, backend, and metrics mode per comparison. Actual GPU
execution and CPU fallback are separate cohorts even when both requested GPU.
Cohort identity is: protocol/version, input id, edit recipe/direction, machine
(`PICTURA_PROFILE_MACHINE` plus `OS`/`ARCH`), build (`cfg!(debug_assertions)`
plus features), thread count, requested/executed backend, and metrics mode.
Revision is reported but may differ; it is `option_env!("CARGO_PKG_VERSION")`
plus an optional git revision when available.

Default tests use fabricated reports to exercise every comparator outcome.
Ignored profiles collect real samples. This diagnostic change can validate a
report or produce an inconclusive comparison without demonstrating improvement.
A later optimization cannot treat invalid, insufficient, noisy, or inconclusive
results as a passed performance gate.

### 5. One report schema, one shared module, one executable

A public `pictura_render::cold_profile` module owns the report type, line parser,
cohort validation, comparator, outcome enum, and exit-code mapping. Both crates
use it; the app profile adds no parallel schema.

Reports are line-oriented `key=value` text, one `metric=<integer nanoseconds>`
line per metric and one `cohort.key=value` line per identity field, plus raw
sample lines and digests. The writer reads an output path from
`PICTURA_PROFILE_REPORT`. No JSON dependency is added.

`crates/pictura-render/src/bin/cold_compare.rs` reads two report files and exits
with the spec codes: 0 improved, 2 regressed, 3 inconclusive, 4 insufficient,
5 invalid, 1 usage/I/O. An integration test spawns it through
`env!("CARGO_BIN_EXE_...")` and asserts each code.

### 6. Fresh-child orchestration is explicit

The parent test spawns `std::env::current_exe()` with
`--exact <child_test> --ignored --nocapture`, sets `PICTURA_COLD_CHILD=1` as the
recursion guard, sets `RAYON_NUM_THREADS` before rayon initialises in the child,
and sets `PICTURA_PROFILE_REPORT` to the child's output file. A bounded timeout
kills a hung child and reports a generic failure. A child-failure/timeout test
covers the non-zero path. Generated inputs are written before the child is
spawned; the page-cache warmth this implies is recorded as a reproducibility
limitation, not treated as a cold-filesystem claim.

The app profile lives in a new
`crates/pictura-app/src/cxxqt_object/tests_profiles/cold.rs` declared by
`tests_profiles.rs`; it does not grow `cxxqt_object.rs`, `impl_core.rs`, or
`helpers.rs`.

## Risks / Trade-offs

- **Instrumentation perturbs timing** -> use coarse boundaries and retain
  end-to-end metrics-off samples separately from attribution samples.
- **Opening warms some caches** -> preserve that behavior in the application
  protocol, and label the engine's stricter cold protocol separately.
- **A synthetic scene misses the issue** -> record that limitation. The follow-on
  optimization must show matching attributed work on a synthetic reproducer and
  the locally exercised application sequence before claiming the issue fixed.
- **Upstream optimizations may already have moved the cold cost** -> the
  upstream verification already records 808-990 ms cold on the supplied
  workload; re-measure before selecting a cause and record the post-rebase
  baseline.
- **Noise prevents comparison** -> return inconclusive and retain raw samples;
  repeat only as an explicitly labelled new batch, with no outlier removal.
- **Privacy leaks through errors** -> report opaque ids and generic errors;
  test successful, missing-input, incompatible-input, and child-failure paths.

## Measured Baseline

Collected after the rebase, from the upstream verification record and the newly
built diagnostic profiles. Numbers are anonymous: generated inputs, no asset
names or paths.

- Upstream #1179 verification on the supplied 4934x1500 workload, 16 workers:
  first-frame and median cold times 808-990 ms across three fresh processes,
  warm 165-194 ms, byte-identical to the original scalar buffer. The reported
  15-second symptom is therefore already largely mitigated; the residual cold
  cost is the subject of this diagnostic and a possible follow-on.
- Engine profile, debug build, 4 workers, generated 1200x800 nested/clipped/
  masked/styled/smart-object/hidden scene: cold end-to-end primary (decode +
  first full) 56.4-63.1 ms across three fresh processes. Recorded phases:
  `render_plan`, `render_execute`, `embedded_decode`, `effect_decode`,
  `effect_build`, `mask_prepare`.
- Application profile, debug build, CPU, generated 1200x800 scene: first-edit
  full composite 66.5-87.8 ms; warm repeated full edits in the same range.
  Opening wall time and the opening-composite phase are recorded in each report.
- The reported 4934x1500 document is not in the repository. The generated
  scene is smaller and does not reproduce its exact layer stack, so these
  numbers neither confirm nor deny that #1174 is fully fixed. They establish
  the post-rebase baseline and the protocol; the locally exercised document is
  the remaining evidence the follow-on must collect.
- The page cache is warm for generated inputs written before child launch; this
  is a process/cache-cold measurement, not a cold-filesystem claim.

## Migration Plan and Handoff

No persisted data migration. After all diagnostic code/tests are finished, build
with `bash scripts/verify-full.sh` (or an explicit `cmake --build build`) at most
80% of available CPUs via `CMAKE_BUILD_PARALLEL_LEVEL`/`CARGO_BUILD_JOBS`, and use
`scripts/check-build-tree.sh` as the executable-current check. Run the focused
checks, existing exactness oracles, and both profiles. Record anonymous
attribution and commands that require no embedded local path.

Archive this diagnostic change after its gates pass; keep #1174 open. Update its
milestone record as diagnostics complete, with optimization outstanding. A
follow-on proposal is required to select a proven mechanism and its exact
regression test, define its scoped fix, then build that complete change last.
Revert diagnostic instrumentation if it changes rendering or retained state when
disabled; no optimization rollback is involved in this prerequisite.
