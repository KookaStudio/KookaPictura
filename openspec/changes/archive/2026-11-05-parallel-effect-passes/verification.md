# Verification

## Behavior

Banded plan preparation collects sibling nodes in document order while resolving
them on the existing Rayon pool. Descendants of visible groups participate;
hidden subtrees do no work. Other plan builders remain sequential. The band loop,
effect kernels, cache keys, and cache publication semantics are unchanged.

Tests observe actual effect-pass build entry/exit and band entry. Controlled
fixtures use a bounded rendezvous; measured profiles do not synchronize builds.
Shared-key fixtures initialize lazy plane identities before cloning and verify
key equality, so they exercise real cache sharing rather than two fresh keys.
No golden baseline or dependency changed.

## Initial scheduling-only measurements (superseded acceptance)

Three samples per preparation mode, same optimized test build, CPU output, same
intra-layer settings, cache purged before each cold sample. The private serial
bevel blur is present in both baselines; #1178 is not included.

| Workload | Workers | Sequential cold median | Concurrent cold median | Speedup | Sequential warm median | Concurrent warm median |
|---|---|---|---|---|---|---|
| Generated two-bevel 1200x800 | 2 | 692.30 ms | 653.68 ms | 1.06x | 34.36 ms | 55.33 ms |
| Supplied 4934x1500 | 2 | 14109.62 ms | 8347.92 ms | 1.69x | 712.00 ms | 726.49 ms |
| Supplied 4934x1500 | 12 | 12876.85 ms | 7741.08 ms | 1.66x | 502.09 ms | 586.86 ms |

All sampled cold/warm outputs agree. The twelve-worker comparison also asserts
byte equality across preparation modes within the same profiling process. The
supplied-workload output hash is `b91db08a24c24f3c` in every sample.

For the first supplied-workload two-worker sample, sequential builds occupy
0.08-6717.85 ms and 6717.87-13386.25 ms; concurrent builds occupy 0.08-7557.88 ms
and 0.10-7647.09 ms. The first band begins only after preparation. The measured
overlap satisfies the cross-layer objective without test synchronization.

The approximate 2x cold target is not reached. In the two-worker case, plan time
falls from 13387.11 ms to 7648.15 ms, while about 700 ms of band work remains.
Concurrent individual builds also take longer than sequential ones. Shared CPU
and memory contention is a plausible cause of that increase, not an isolated
measurement of it. Smaller generated workloads show much less gain. Warm
composites do not gain from this change and the measurements above include warm
regressions; no warm-speedup claim is made.

Peak process RSS measured in separate two-worker processes reaches 600640 KiB
sequential and 782600 KiB concurrent (about 587 and 764 MiB). With twelve workers
in one process, the high-water mark reaches 1569392 KiB across both modes; that
cumulative value cannot isolate the memory increase of the parallel mode.
RSS includes allocator retention, test snapshots, and output buffers, not only
effect scratch. The cache retention budget is unchanged.

## Reviewer checklist

- [ ] Repeat the supplied-workload check locally, keeping worker count and
  intra-layer settings equal: confirm unsynchronized build overlap, exact
  output, cold/warm timings, and peak-memory trade-offs. This input is not a
  committed CI fixture; record only generic workload characteristics and results.

## Expanded goal: production acceptance

The user expanded completion to 100–1000 ms cold compositing. The initial
7.7-second scheduling-only result is not accepted as the endpoint.

The implementation combines the exact SIMD Gaussian and parallel bevel geometry
from PR #1221 (#1178, source commit e87f54c7, archive ce6ee48f) with cross-layer
preparation, bulk matte conversion, resolved mask gates, parallel glow/bevel
emission, and row-specialized pass replay. A native CPU profile then identified
the u8 Gaussian accumulator as the largest residual (about 36% of sampled CPU
time). Cache-local vertical tiles reduce its streaming work without changing
any pixel's ordered tap sum. Scalar tests cover tile boundaries and oversized
kernels. No approximation, resolution reduction, dependency, or golden change.

Unit-test render binaries increment a shared atomic for every masked sample.
Their timings include substantial instrumentation contention, even in release
mode, so they are diagnostic rather than production acceptance. The new ignored
`effect_cold_latency` integration test calls the production library without
those counters. It reads the local input and saved original scalar reference
before timing, purges the source cache before each cold composite, asserts the
first and median cold result at most 1000 ms, and compares the full buffer byte
for byte against the original reference outside timing.

First production acceptance, 16 workers, optimized Cargo test profile:

| Sample | Cold | Warm | Output |
|---|---|---|---|
| First | 808.291 ms | 176.855 ms | Exact original bytes |
| Second | 810.329 ms | 164.801 ms | Exact original bytes |
| Third | 808.317 ms | 187.251 ms | Exact original bytes |

Median cold: 808.317 ms. Hash remains `b91db08a24c24f3c`. Peak process RSS
reaches 1413836 KiB across the three samples (about 1.35 GiB), including allocator
retention, reference/output buffers, and concurrent intermediate fields. This
change meets the latency target with a memory trade-off; it does not claim
Photoshop algorithm or timing parity.

The production check runs after decoding a fresh document. Subsequent samples
purge the source cache but can reuse document-owned vector coverage tables, as
in the ordinary application. The first sample is therefore reported separately.

### Fresh-process repeats

| Fresh process | First cold | Median of three cold samples | Warm range |
|---|---|---|---|
| 1 | 808.291 ms | 808.317 ms | 164.801–187.251 ms |
| 2 | 990.004 ms | 990.004 ms | 173.093–188.845 ms |
| 3 | 888.365 ms | 855.721 ms | 167.329–194.078 ms |

All three first/median acceptance assertions pass, with complete byte equality
to the saved original scalar image in all nine samples. One later sample in
process 2 is 1022.062 ms; the target is not a hard real-time guarantee. Peak RSS
across these fresh-process checks reaches 1528056 KiB (about 1.46 GiB), including
retained allocations and reference/output buffers. Hardware/workload scheduling
variability remains visible, and early pre-tiling experiments exceeded budget;
those misses drove the native profiling and exact cache-local convolution fix.

Final generated 1200x800 diagnostic profile (16 workers) also passes with
unchanged output hash; this small workload is well below the workstation goal.
Its per-call sequential/parallel preparation comparison leaves intra-layer
kernels enabled in both cases. Unit profiling timings remain diagnostic because
of the per-pixel counter overhead described above.

## Final automated verification

- Renderer unit tests: 1499 passed, 17 intentionally ignored profiling/GPU tests.
- Filter unit tests: 794 passed, 1 intentionally ignored profile.
- Renderer document/adjustment/clipping/knockout/merge oracles: 32 passed.
- Filter oracle/ported/properties/scenarios integration suites: 168 passed.
- Total non-ignored regression checks: 2493 passed.
- Explicit production acceptance: three fresh processes, three cold/warm samples
  each, all first/median assertions and original-buffer comparisons pass.
- Clippy for all targets of both affected crates with warnings denied, formatting,
  file-size, test-plan, and repository guards pass. No dependencies or golden
  baselines changed. The masked full-composite fixture shares the existing
  counter-test lock so it cannot contaminate global mask-sample assertions.
- Native CPU sampling used gprofng outside the repository. No throwaway native
  profile or private input was added to the repository.

Final diagnostic pass-build intervals overlap (both begin at about 0.16 ms and
finish at about 767.82 ms). Per-layer pre-distance work, including earlier glow
processing, is about 311 ms; distance about 119 ms; field blur about 244 ms;
shading about 70 ms; emission about 23 ms. These are stage diagnostics in the
instrumented unit binary, not production acceptance latency. The saved original
buffer still matches exactly. Production latency evidence is the fresh-process
integration table above.

## Review corrections (2026-11-06)

The same review agent implemented both findings. Feather-cache misses now build
outside shared locks and scalar sampling's thread-local borrow, publish resolved
planes, and reuse the first resident publication. Cold concurrent misses may
duplicate work rather than blocking a Rayon worker. Cache hits retain the direct
sample path without adding an Arc clone per pixel. Both profiling harnesses
require an existing independent reference; neither generates or overwrites it.

Four added regression checks cover forced same-thread scalar reentry, lookup
from a stolen Rayon half, exact plain-layer composites sharing raster/vector
feather masks on one and four workers, and missing/existing reference handling.
The shared-mask fixture exercises both direct and box-stack feather kernels.

After all source edits, renderer test targets compiled successfully with 12 of
16 CPUs. Clippy for all renderer targets with warnings denied, formatting,
diff whitespace, file-size, repository guards, zero-open guard, and strict
OpenSpec validation (378 specs) pass. Both archived test plans were checked
explicitly; the ordinary test-plan guard excludes archives.

The first full unit-binary run with 12 outer test threads passed all four new
checks but failed the existing
`layer_source::tests::every_source_kind_hits_on_the_second_composite`: its
global pass-cache count was 2 rather than 1. Its local test mutex does not
isolate other tests' cache insertions. Without changing its assertions, the
complete rerun with outer execution serialized passed **1503 tests**, with
17 intentionally ignored profiles/GPU tests. The new regressions still create
dedicated concurrent Rayon pools internally. All five renderer oracle suites
also pass, **32 checks** in total. No dependencies or golden bytes changed.

Production acceptance used the existing independent reference and 16 workers:

| Attempt | First cold | Median cold | Cold samples | Warm range | Result |
|---|---:|---:|---|---:|---|
| Initial | 1095.010 ms | 1398.814 ms | 1095.010 / 2210.423 / 1398.814 ms | 216.100–527.384 ms | Latency assertion failed |
| Quieter rerun | 974.663 ms | 855.696 ms | 974.663 / 855.696 / 843.951 ms | 172.083–193.981 ms | Passed |

All cold and warm buffers match exactly; hash remains `b91db08a24c24f3c`.
Rerun peak process RSS was 1443296 KiB. Host activity was higher around the
initial attempt, but the cause of timing variation was not established.
Anonymous input inspection confirms that this workload has no nonzero feather
parameters, so duplicated feather builds do not explain these timings.
