# Design: shared-mask-ops

## Context

See `proposal.md` for motivation and `specs/imaging/mask-ops/spec.md` for the
contract. The current implementations that shape the approach:

- `pictura-select`'s morphology treats out-of-canvas samples as **zero**
  (`lib.rs:360-401`), while its Smooth/median path clamps; `pictura-filters`,
  `pictura-paint` and `pictura-render` treat them as **clamp**.
  `tools/selection-model` pins the zero behaviour for Contract and Border, and
  `imaging/other-filters` pins the clamp behaviour for Maximum and Minimum.
- The selection median is O(r²) per pixel (`lib.rs:565`), while
  `pictura-filters::noise::window_median` is already a per-row sliding histogram
  with the same clamp semantics (`noise.rs:59`). The window is odd, so its
  `total / 2` and `(2r+1)² / 2` median thresholds are the same.
- `pictura-render`'s spread and choke work on an `f32` matte, with clamp border
  and `-∞` / `+∞` scan accumulators (`layer_effects/mod.rs:690-762`).
- `pictura-select` and `pictura-paint` both flood with a 4-neighbour queue that
  grows one entry per visited pixel (`lib.rs:437,498`, `bucket.rs:32`).
- `pictura-core` currently has **no dependencies**; `rayon` is already used by
  filters, paint and render.

## Goals / Non-Goals

**Goals:**

- One implementation per algorithm, with the border policy carried by the caller
  rather than assumed.
- Exact equivalence to the current naive forms, proven by randomised tests that
  keep the naive versions as the oracle.
- Cost independent of the radius for morphology and median, and independent of
  the reference-colour count for Similar.
- No duplicated row-parallel scaffolding at call sites.

**Non-Goals:**

- No GPU acceleration. CPU output stays the oracle.
- No change to Smooth's median semantics (no mean-and-threshold shortcut).
- No distance-transform or Euclidean structuring element; the square footprint
  and the documented border policies stay as they are.
- No new crate and no new lockfile entry beyond `rayon` in `pictura-core`.
- No `docs/` change.

## Decisions

### The module lives in `pictura-core`, not in a new crate

`pictura-select`, `pictura-filters`, `pictura-paint` and `pictura-render`
all need at least one of these kernels; only core is below all four. A new crate
would add a manifest, a version line and a `deny.toml` review for one module;
putting the module in `pictura-select` would force filters and paint to depend
on the selection crate.

Core gains `rayon = "1"`. That is not a new crate in the workspace graph — three
engine crates already depend on it — but it is the first dependency core has, so
the module is written as leaf kernels with no knowledge of documents, layers or
Qt.

### Morphology: monotonic deque, generic sample, explicit border

The public shape is one pair per direction over a flat plane:

```
dilate<T: Copy + PartialOrd>(src, w, h, r, border) -> Vec<T>
erode<T: Copy + PartialOrd>(src, w, h, r, border) -> Vec<T>
```

The policy is `Border::Zero(T)` (the callers pass `0u8` / `0.0f32`) or
`Border::Clamp`.

Both are separable: a horizontal pass over rows, then a vertical pass over
columns. Each 1-D pass uses a monotonic deque of indices, so a line of `n`
samples costs `O(n)` and the whole plane is `O(1)` per sample per axis. The
deque carries `(index, value)` and drops dominated entries; the border extension
is applied when a deque entry leaves the window, not inside the hot loop.

Alternatives:
- *Naive square scan*: the current implementation; correct but O(r) per sample
  per axis. Kept as the test oracle.
- *Van Herk/Gil-Werman*: also O(1) per sample, more branches and prefix/suffix
  scratch. The deque is shorter to prove correct.
- *Two border variants as two functions*: doubles the surface for no benefit;
  the policy is a value-carrying enum.

The generic `PartialOrd` bound serves both the u8 selection plane and the f32
layer matte. Matte samples are finite (coverage and offset fields are clamped
before they reach the filter), so no NaN-aware total order is needed; the
comparison is written as `if v > best { best = v }` so a NaN would keep the
previous value rather than panic.

`pictura-render` keeps its `-∞` / `+∞` wrappers only long enough to drop them:
clamp border guarantees every window is non-empty, so the accumulators are dead
code once the deque does the windowing.

### Median: column bands

The per-row histogram touches 201 strided rows to remove and 201 to add for
every output pixel — roughly 400 cache lines per pixel, which is where the
5–10 s goes, not the arithmetic.

The kernel turns the slide around:

- Keep one 256-bin `u32` histogram per column, over the `2r+1`-tall clamped
  column segment at that x, built by counting clamped row multiplicities. `u32`
  bins keep the window count exact past radius 127 (a `u16` histogram would
  overflow once `(2r+1)² > 65535`).
- Parallelism is over column strips with a `±r` halo. Each strip owns the band
  state for columns `max(x0-r, 0) .. min(x1+r, w)` and runs the full row loop;
  no state is shared and no reduction crosses threads, so the result is
  scheduling independent.
- Advancing one output row updates each band by one sample in and one out, and
  carries the strip's `x0` window histogram with the same deltas weighted by
  each column's clamped multiplicity, so no row rebuilds the 256-bin window.
- Sliding one pixel right is `window += band[x+r] - band[x-r]`: 256 contiguous
  adds and 256 contiguous subtracts over a 1 KiB block, auto-vectorised.
- The median is the first bin where the running total exceeds `(2r+1)² / 2`.
  A prefix scan is the first implementation; a walking median pointer is only
  added if the profile asks for it.

The strip state is about 6–9 MB at 6000 columns with `u32` bins and the `±r`
halos (r = 10..100); it is transient and smaller than the 24 MB mask.

Alternatives:
- *Keep the row slider and just parallelise rows*: portable and short, but it
  keeps the O(r) per pixel and the cache-line traffic; it is 5–10× short of the
  target at r=100.
- *Box mean + threshold*: only equals a median on binary windows; changes soft
  coverage and would need a spec amendment. Rejected.
- *Binary fast path with a distance transform*: large win on binary masks, no
  win on feathered ones. Deferred; see Open Questions.

The shared median replaces both copies. The kernel applies no cap; each caller
keeps its existing bound: `Selection::smooth` relies on its 100 clamp, and the
filter keeps its `radius > 100` error and then applies its existing
`min(radius, max(w, h))` before calling. The differential test must include
`r > max(w, h)`.

### Similar: a dilated 2^24-bit colour cube

References are the distinct colours of the already-selected pixels. Exact
membership is a bitset; the tolerance is a Chebyshev box, which is separable, so
the cube is dilated by `t` along each RGB axis after the references are marked.
The result is exactly "some reference within `t` on every channel".

- Layout: one bit per RGB colour, `2^24` bits = 2 MiB, four `u64` words per
  `(r, g)` row. Index is `(r << 16) | (g << 8) | b`.
- `b` dilation is an OR of shifted 256-bit rows; `g` and `r` dilation OR whole
  rows across the axis. For `t = 100` each axis costs about 53 M 64-bit
  operations (about 160 M total), so the 100 ms target is unproven until
  profiled.
- Query is `bits[c >> 6] & (1 << (c & 63))`.

Alternatives:
- *Dense `u8` presence cube* (16 MiB) reusing the morphology primitive: simpler
  code, but about 3× the traffic and 8× the memory.
- *Per-pixel scan of the reference set*: the current behaviour; rejected.

The matcher lives in `pictura-select` (`src/matcher.rs`), not in core: only
Similar uses it. The issue's direction put it in the shared module, but YAGNI —
the growth step is moving the file if a second crate ever needs it.

### Flood: row spans, predicate on the edge

```
flood(w, h, seeds: &[u8], matches: impl Fn(usize, usize) -> bool) -> Option<Vec<u8>>
```

`seeds` is the initial selected set: every non-zero sample is a source, and the
returned 0/255 mask includes the seeds. `matches(from, to)` is evaluated for
each neighbour edge, so the same kernel serves Magic Wand (fixed reference
colour, RGB), Grow (the popped pixel's colour) and the Paint Bucket (fixed
reference, RGBA, alpha included). Magic Wand and the bucket pass a one-pixel
seed mask and use the result directly; Grow keeps its existing mask and raises
to 255 only the reached pixels that were 0, so intermediate seed coverage is
preserved. The frontier is a stack of `(y, x_start, x_end)` spans; each span is
expanded left/right, then one row above and below is scanned for new spans.
Marking happens when a span is pushed, so no pixel is visited twice and the
stack is bounded by the number of runs, not the number of pixels. A `#[cfg(test)]`
peak-frontier counter (test-only hook) makes the frontier bound assertable. A
seed mask with no in-plane source returns `None`; callers keep their existing
error mapping.

Alternatives:
- *Keep `VecDeque<usize>` and only swap the bucket*: three queue
  implementations remain, and a uniform 24 MP fill still risks a 192 MB queue.
- *Span fill only in the bucket*: rejected because Magic Wand and Grow have the
  same frontier shape and the same predicate structure.

### Call-site map

| Caller | Kernel | Border |
|---|---|---|
| `Selection::{expand,contract}` | `dilate`/`erode` u8 | zero |
| `Selection::border` | two morphs | zero |
| `Selection::smooth`, `refine` | `median` u8 | clamp |
| `filters::maximum/minimum` | `dilate`/`erode` over planes | clamp |
| `filters::noise::median` | `median` per plane | clamp |
| `paint::bucket::flood` | `flood` RGBA | — |
| `paint::bucket::stroke_selection` | `dilate`/`erode` u8 | clamp |
| `layer_effects::{dilate_matte,erode_matte}` | `dilate`/`erode` f32 | clamp |
| `select::{magic_wand,grow}` | `flood` RGB | — |
| `select::similar` | matcher | — |

Deleted by the migration: `select::median`, `select::morph`,
`filters::noise::window_median`, `paint::bucket::{dilate,erode}`,
`render::layer_effects::{axis_max,axis_min}`, and the bucket's DFS body.

### Testing and profiling

- The naive forms stay in `#[cfg(test)]` next to the kernels and are the oracle
  for randomised equivalence tests: fixed-seed xorshift planes, sizes including
  0, 1, `w < r`, `h < r`, and both border policies. No new test dependency.
- `similar` is compared against the per-colour scan on a synthetic image with
  thousands of distinct colours, and the matcher against a brute-force box
  membership test.
- The flood is compared against a naive `VecDeque` breadth-first fill.
- The existing selection and filter oracle suites are unchanged. The
  layer-effect gate is the retained pre-migration spread/choke scan plus the
  exact-pixel effect tests.
- A new `#[ignore]` profile in `crates/pictura-select/tests/profile.rs` builds a
  6000×4000 mask and prints Similar with 10 and 10 000 reference colours; Smooth
  at radius 10 and 100; Expand at radius 1 and 100; plus the three 6000×4000
  targets (Similar 10 000 < 100 ms, Smooth r=100 < 1 s, Expand r=100 < 300 ms).
  It reports rather than asserts wall-clock time, matching
  `selection_move_perf.rs`; the spec's targets are reviewed from its output.
- The 6000×4000 profile on the 16-core dev box after the strip rewrite printed
  Similar 10 000 colours ~52–57 ms, Smooth r=10 ~495 ms, Smooth r=100 ~478 ms,
  Expand r=100 ~55 ms.

## Risks / Trade-offs

- **The column-band median is the riskiest new code** → the per-strip band
  state and the `x0`-window carry are covered by the sorted-window differential
  and the r=100 profile; re-profile the strip layout if it changes.
- **f32 morphology must reproduce the effect bytes exactly** → the deque is
  comparison-for-comparison the same max/min as the scan; clamping makes every
  window non-empty, so the infinities never leak. The pre-migration spread/choke
  digests plus the exact-pixel effect tests are the gate.
- **Nested parallelism** → the kernels are leaves; call sites drop their own
  `par_chunks` loops. Render's effects are not invoked inside another parallel
  iterator over the same plane.
- **The bit-cube dilation is fiddly across word boundaries** → test the matcher
  against brute force for every tolerance in 0..=255 on a reduced palette, and
  at 0/1/32/100/255 on the full cube.
- **The Similar 100 ms target is met on the dev box (~52–62 ms)** → the
  three-axis cube dilation is about 160 M 64-bit operations at `t = 100`;
  re-profile and parallelise the cube build or cheapen the per-axis dilation
  if the cube layout changes.
- **~2 MiB cube plus a ~6–9 MB median strip state per call** → both are transient
  and smaller than the 24 MB document mask; no caching is introduced.
- **Similar's fixed cube build costs a tiny-image run tens of milliseconds** →
  accepted: it is imperceptible at that size and the target is large documents;
  a small-image fallback can be added if a profile asks for it.
- **The morph peak is two planes** → `tmp` is dropped before the output is
  allocated, so the horizontal buffer plus the collected vertical parts/output
  never exceed two planes.
- **Profile targets are hardware-dependent** → the harness prints rather than
  asserts, so a slow CI box does not turn a perf change into a flaky failure.
- **#969 touches the render crate from the same base commit** → the render
  change is one wrapper swap in `layer_effects/mod.rs`; whoever lands second
  rebases that file.

## Migration Plan

1. Add `pictura-core::mask_ops` with morphology, median (column bands),
   flood and their differential tests. No call sites change yet.
2. Swap the select modify ops and Refine Edge's Smooth; run the selection oracle
   and the existing select tests.
3. Swap filters Maximum/Minimum/Median; run the filter digests and ImageMagick
   oracles.
4. Swap paint's flood and stroke band; run the paint tests.
5. Swap render's spread/choke; run the retained pre-migration scan and effect
   tests.
6. Swap `select::{magic_wand,grow}` to the shared flood and `similar` to the
   matcher; run the selection suite.
7. Delete the duplicated bodies, add the full-document profile, and verify the
   column-band median against its target.

Each step is independently shippable and reversible by restoring the old body;
there is no data migration.

## Open Questions

- If the column-band profile still misses on a realistic feathered mask, add the
  soft-distance binary fast path from the exploration. It is an accelerator
  behind the same exact kernel and does not change the spec, so it can be
  deferred until the numbers say otherwise.
