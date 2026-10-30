# Design

## Root cause

`layer_source::lookup` cloned the entry's `Arc<OnceLock<Artifact>>` under the
map lock, released the map lock, then called `cell.get_or_init(build)`. The map
lock was safe; the once-lock was not. `OnceLock` blocks every other caller, and
deadlocks a re-entrant caller on the same thread, for as long as `build` runs.

Since #1033, `build_layer_passes` runs rayon kernels (`apply_mask` uses
`par_chunks_mut`). A rayon worker that joins and finds its other half stolen
waits in `wait_until_cold`, which runs any queued job. The stack dump in #1044
shows this on one thread:

1. a `composite_banded` band job calls `effect_passes`, misses, and enters
   `OnceLock::initialize` -> `build_layer_passes` -> `build_inner_glow` ->
   `apply_mask` (parallel);
2. while joining inside `apply_mask`, the worker runs a stolen
   `composite_pixels` row job; while that joins, it runs a stolen
   `composite_banded` band job;
3. that band calls `effect_passes` for the same layer, the same key, and blocks
   in `Once::call` on the cell its own thread is initialising.

The other workers' band jobs block on the same cell. The pool is wedged.

A cross-thread variant has the same shape: a worker that runs half of the
build's kernel can, while joining inside it, run a band job that blocks on the
cell. The build then waits for that half, which cannot finish. So any wait for
an in-flight build on a pool worker can deadlock, not just a re-entrant one.

## Options considered

1. **Initialise outside the parallel region.** Walk the layer tree before the
   bands fan out and build every source. A separate walk must mirror
   visibility, clipping, knockout `skip_effects`, and the content routes. If it
   drifts, it silently reintroduces the deadlock. It also does not protect any
   other parallel caller of the cache.
2. **Make the build sequential inside the pool.** No rayon switch for this
   exists. Running builds on a dedicated pool either serialises every build or
   moves the same blocking-wait problem into that pool, because a nested
   composite inside a build looks up nested keys there.
3. **Wait cooperatively.** Mark the key in flight and have a second caller
   loop on `rayon::yield_now` until the value appears. In the same-thread
   case the waiter sits on top of the build it waits for, so it spins forever;
   the wait itself is the bug, however it is spelled.
4. **Build outside the once-lock, publish with `OnceLock::set`.** Every miss
   builds and the first `set` wins. This was the first draft, and it fails
   twice. It drops the dedupe the cache promised plain concurrent callers
   (`parallel_build_of_one_key_runs_once`, eight OS threads, one build). And
   `set` is not a non-blocking publish: it waits while another caller is inside
   `get_or_init`, so keeping `get_or_init` for callers outside the pool would
   put the pool-side `set` back into a wait on a running build.
5. **A slot that knows who may wait.** Replace the `OnceLock` with a small
   `Slot` (`Mutex<Empty | Building | Ready>` plus a `Condvar`). A lookup
   outside the pool claims an empty slot or waits for a building one. A lookup
   on a rayon worker (`rayon::current_thread_index()` is `Some`) never waits:
   it builds its own copy, claiming the slot only if it is empty. Whoever
   publishes first wins; later finishers return the published value.

## Decision

Option 5 in `lookup`, because it removes blocking-in-pool for every cache kind
(embedded, filtered, text, effects, passes) in one place and keeps the
build-once guarantee where waiting is safe. A thread outside the pool holds no
rayon jobs, so nothing the builder needs can be queued behind it; a worker
can hold such jobs, so it must not wait. The mutex is held only to read or
write the state, never across a build. A build that panics drops its `Claim`,
which resets the slot to empty and wakes waiters, so one of them builds it
(the `OnceLock` poisoning behaviour this replaces did the same).

The duplicate cost matters for banded composites, where every band hits the same
keys at almost the same moment. A cold banded composite would otherwise build
each whole-layer pass once per concurrently running band. That breaks the
banded contract ("one whole-layer pass is built and replayed into every band",
`banded_sub_region_reuses_one_pass`), and on a heavy style it multiplies CPU
time and transient memory by the thread count. So `composite_banded` composites
band 0 on the calling thread before fanning out the rest. This is option 1, but
with no mirror to drift: band 0 runs the real band code, and
`composite_layer_inner` requests effect passes for every visible layer before
any per-band intersection, so band 0 requests exactly the keys every band
requests. If band 0 is not enough, the result is extra work, never a deadlock.

Band 0's own kernels are still parallel, so serialising one 64-row band costs
little.

## Tests

The two deadlock shapes are forced, not left to timing, and each runs under a 30 s
watchdog on a spawned thread so a regression fails instead of hanging:

- `same_key_lookup_stolen_mid_build_does_not_deadlock`: a one-thread pool;
  the build calls `rayon::yield_now`, which runs the queued same-key lookup on
  top of it (the re-entrant shape in the #1044 dump).
- `same_key_lookup_in_a_stolen_half_of_the_build_does_not_deadlock`: a
  two-thread pool; the build `join`s a half that spins until the other half
  is done, so the idle worker must steal the other half, which looks up the
  same key (the cross-thread shape).

Both were run against the unfixed `lookup` and failed on their 30 s watchdogs.
`panicking_build_releases_its_claim` covers the claim reset, and the existing
`parallel_build_of_one_key_runs_once` keeps the outside-pool dedupe honest.

## Ceilings

- `ponytail:` a smart-filter or type source looked up only by bands below the
  first can still be built by several bands at once on a cold composite. That
  costs time, not correctness. Warm those too if profiling shows it.
- A crafted cyclic embedded payload still waits on its own build outside the
  pool, as it did on the once-lock, and recurses without bound on a worker.
  The `ponytail:` in `decode_embedded` names the depth guard as the ceiling.
