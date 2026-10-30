# Proposal

## Why

`pictura-render::huge_inner_glow_is_bounded` deadlocked under
`cargo nextest run --workspace` (#1044): every rayon worker sat in a futex wait
for hours. The layer-source cache (#991) built each artifact inside
`OnceLock::get_or_init`, and since the effect-matte kernels were parallelised
(#1033) the build itself runs rayon work. A worker joining inside that build
steals queued work, including another band of the same banded composite. That
band looks up the same effect-pass key, so it blocks on the once-lock whose
initialiser sits lower on its own stack. Nothing can finish: the builder is
blocked on itself, and every other band is blocked on the builder.

## What Changes

- The cache cell is no longer a `OnceLock`. A lookup on a rayon worker never
  waits for an in-flight build of its key: it builds its own copy, and the
  first artifact published is the one every caller returns. A lookup on a
  thread outside the pool still waits, so plain concurrent callers keep
  building once. A build that panics releases its claim.
- The banded compositor composites its first band before fanning the others
  out. Every band asks for every visible layer's whole-layer effect passes, so
  the first band builds each pass once and the parallel bands replay it from the
  cache, which keeps the "one whole-layer pass per banded composite" contract
  without the lookup dedupe that caused the deadlock.
- A test that a panicked build releases its claim, and two deterministic
  regression tests drive the steal patterns under a
  watchdog: a same-thread re-entry on a one-thread pool, and a cross-thread
  steal of half the build on a two-thread pool.

No breaking changes. No new dependencies. Output pixels are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/layer-source-cache`: adds a requirement that a cache lookup on
  the worker pool never blocks on an in-flight build, that lookups outside the
  pool still build once, and that a banded composite still builds each
  whole-layer pass once.

## Impact

- Render: `layer_source::lookup`, `composite_region::composite_banded`.
- Tests: `pictura-render` layer-source cache suite gains three regression tests; the
  existing banded and huge-glow tests are unchanged.
