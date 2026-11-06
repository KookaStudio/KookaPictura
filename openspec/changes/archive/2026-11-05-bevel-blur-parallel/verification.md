# Verification

## Correctness and static checks

- The six new field, SIMD-row, and composite regression tests pass. Generated complete
  composites match the former radius-taking convolution across supported
  styles, techniques, masks, softening, and worker counts.
- `cargo nextest run -p pictura-filters -p pictura-render`: 2,537 passed,
  58 skipped (the ignored profiling/device tests). Nextest used eight test
  processes and two default Rayon workers; the new tests construct their own
  one-worker and four-worker pools.
- Clippy with `--all-targets -- -D warnings` passes for both affected crates.
- Workspace formatting, test-plan coverage, file-size, self-test budget,
  repository guard, and `git diff --check` pass.
- `openspec validate --all --strict`: 379 passed, zero failed.

The initial shared-target compile resolved an obsolete dependency without the
new public helper. A fresh isolated target compiled successfully. A concurrent
in-process `cargo test` run hit three global-counter assertions; the new render
checks now use the existing serial-test lock, and the standard process-isolated
nextest run passes. Nextest's first report write was denied through the shared
target symlink; the approved rerun completed successfully with its JUnit report.

No golden baseline or dependency changed. No app/Qt source changed; GUI tests
and an app build were not run.

## Cold CPU composite measurements

Both revisions use the same profiling harness, Cargo `test` profile (workspace
engine crates at opt-level 2, dependency crates at opt-level 0), and
`RAYON_NUM_THREADS=16` on the same machine. Baseline source is the pre-change
HEAD snapshot. Every sample runs in a fresh process, reads and decodes before
starting the composite timer, and measures the first full CPU composite.
Every baseline and changed output hash matches. Input identifiers are omitted.

| Revision | Run | Composite seconds | Under four seconds |
|---|---|---:|---|
| Baseline | 1 | 12.458415138 | No |
| Baseline | 2 | 12.225225164 | No |
| Baseline | 3 | 12.284792280 | No |
| Changed | 1 | 3.670372718 | Yes |
| Changed | 2 | 3.615283447 | Yes |
| Changed | 3 | 5.901282870 | No |
| Changed, variance investigation | 4 | 5.421399884 | No |
| Changed, variance investigation | 5 | 3.828123191 | Yes |
| Changed, variance investigation | 6 | 3.566463692 | Yes |

For runs 4-6, process CPU time was 32.999 s, 32.948 s, and 31.740 s;
process wall time was 5.558 s, 3.960 s, and 3.697 s. System CPU pressure
`some` accumulated 1.782 s, 0.650 s, and 0.490 s over those process lifetimes.
The slow run coincides with greater scheduling pressure, but these measurements
do not establish consistent compliance with the four-second requirement.
The initial three changed runs have a 3.670 s median versus the baseline's
12.285 s (about 3.35 times faster); all six changed runs are retained above.

## Further optimization and final acceptance

Following the user's request for further speed, the blur now accumulates a
whole row one tap at a time, with padded input and reusable double-precision
scratch storage. Its multiply/add order remains unchanged. Independent
Euclidean distance and shading passes now run in parallel, with direct
bitwise distance-field and full-composite comparisons against the former
serial algorithms. Gaussian row sums select AVX2 on supported x86 CPUs and
retain a portable fallback. This also accelerates the existing single-precision
Gaussian helper used by Inner Glow. Direct tests cover both paths and vector
tails; existing filter digests remain unchanged. No FMA is introduced.

The following checkpoints are retained, including slower outliers. Stage probes
were temporary and are removed from the final code. The app's `dev-app` profile
optimizes dependencies as well as workspace code; rows labeled `test` use the
same settings as the original baseline above. Every row is a fresh-process
cold CPU composite; every output hash still matches.

| Checkpoint | Profile | Composite seconds |
|---|---|---:|
| Vectorizable blur, diagnostic | test | 2.229674294 |
| Parallel distance/shading, diagnostic | test | 1.46428866 |
| Parallel geometry, optimized check 1 | dev-app | 5.155540808 |
| Parallel geometry, optimized check 2 | dev-app | 1.373018997 |
| Parallel geometry, optimized check 3 | dev-app | 1.427889387 |
| Parallel geometry, interleaved check | test | 1.446637089 |
| Composite-stage diagnostic | dev-app | 3.673931534 |
| Effect-stage diagnostic | dev-app | 4.639333413 |
| Parallel geometry, clean check 1 | test | 5.582186079 |
| Parallel geometry, clean check 2 | test | 3.54661461 |
| Parallel geometry, clean check 3 | test | 2.452273205 |
| Final AVX2 acceptance 1 | test | 2.799011166 |
| Final AVX2 acceptance 2 | test | 3.244027364 |
| Final AVX2 acceptance 3 | test | 1.660238198 |
| Final AVX2 acceptance 1 | dev-app | 1.229553418 |
| Final AVX2 acceptance 2 | dev-app | 1.284100902 |
| Final AVX2 acceptance 3 | dev-app | 1.362431088 |

Final acceptance passes in both profiles: all three final `test` runs are
under four seconds (1.660-3.244 s), and all three final `dev-app` runs are
under four seconds (1.230-1.362 s, median 1.284 s). The latter are the optimized
app-profile measurements, not a claim of sub-second Photoshop parity.
Earlier outliers coincide with broader timing variability: some decodes took
0.23-0.29 s versus roughly 0.08 s in the final optimized runs. Scheduling
pressure was observed, but it is not asserted as the sole cause.

Stage measurements after row-wise vectorization showed about 0.33 s per
beveled layer in matte/distance/height and 0.15 s in shading. Parallelizing
those passes reduced them to roughly 0.09 s and 0.03 s respectively in the
next diagnostic. Inner Glow and the remaining composite work are now a
substantial part of the total. Effect scheduling and cache changes remain
outside this change.

The final standard nextest run reports 2,537 passed and 58 skipped. Clippy
for both affected crates and all targets passes with warnings denied. All
six spec scenarios have implemented coverage. No temporary timing probes,
local input identifiers, dependency changes, or golden changes are included.
