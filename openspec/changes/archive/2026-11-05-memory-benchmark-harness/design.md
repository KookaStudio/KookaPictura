# Design

## Two layers of measurement

| Layer | Measures | Where | Runs in |
|---|---|---|---|
| Process | peak resident memory (`VmHWM`), resident size after, wall time, OOM under a limit | `scripts/profile-tools.py --peak [--memory-max]` | by hand; the 17000² baseline |
| Engine | heap peak through `PeakAlloc` | `pictura-app` `tests_profiles/memory.rs` | `cargo nextest` (4250² guards); `--run-ignored only` (17000²) |

The process layer is the truth for a user: it includes the Qt side, the GPU
driver, and freed memory the allocator kept. The engine layer is
deterministic enough to gate CI, and it does not need the app or a display.

## Scenarios

The catalogue lives in `scripts/memory_scenarios.py`. Each M id names the
steps of its **setup** (prerequisites, not measured) and its **measured** steps.
Every scenario runs in a fresh app process.

| ID | Setup | Measured |
|---|---|---|
| M1 | none | `document new` W×H white, `background_layer` (File ▸ New's locked Background) |
| M2 | M1 | `layer_op add` (Layer 1, active) |
| M3 | M1, M2, Background active, marquee 1000² at centre | `run_command edit.fill` foreground |
| M4 | M1, M2, marquee | Paint Bucket click at centre on Layer 1 |
| M5 | M1, M2 | Paint Bucket click on Layer 1, no selection |
| M6 | M1, M2 | Brush, 300 px hard round, a 4000 px horizontal stroke on Layer 1 |
| M7 | M1, M2, then M3, M4, M5 and M6 measured steps (each from its own setup state) | undo ×3, redo ×3 |
| M8 | M1, M2, M6 | `history snapshot`, then `history restore` |
| M9 | M1, M2, M4, deselect | Move tool drag of Layer 1 by 500 px |
| M10 | M1, M2, M4, deselect | `free_transform` Layer 1, 150 %, 15°, commit |
| M11 | M1, M2, M4 | `document save_as` PSD |
| M12 | generated fixture | `document open` of the 4-layer PSD |
| M13 | M1 | zoom fit → 100 % → pan across (scroll) → fit |

M3 fills the Background, so Layer 1 is still empty for M4, M5 and M6 (owner
decision). M9 and M10 act on Layer 1's 1000² square with no selection (owner
decision). Sizes in the catalogue are fractions of the document side, except
the fixed 1000² marquee, 300 px brush and 4000 px stroke. These scale by the
divisor (`--size 4250x4250` uses 250², 75 px and 1000 px), so a reduced run
means the same thing.

M7's setup runs the M3–M6 gestures in one app: the fill on Background in the
marquee, a bucket in the marquee on Layer 1, a deselect, a whole-layer bucket
on Layer 1, and the stroke. M5 then floods the rest of Layer 1 around the
square. That is not identical to M5 alone, but it is the history M7 needs.

## Peak measurement (process)

Before the measured steps: `status` to settle, then idle (`wait_idle`). Then
write `5` to `/proc/<pid>/clear_refs`, which resets `VmHWM` to the current
`VmRSS`, and read `VmRSS` as `rss_before`. After the measured steps: `status`,
then `wait_idle`, then `settle_workers`. `settle_workers` polls `VmRSS` every
50 ms until it moves less than 1 MiB over 500 ms, because deferred worker
composites and pyramid builds still allocate after the GUI is idle. Then read
`VmHWM` as `peak_mb` and `VmRSS` as `rss_mb`. `peak_added_mb = peak_mb -
rss_before_mb`. `total_ms` runs from the first measured request to the settle.

## Memory limit and outcomes

With `--memory-max N` the app is launched as `systemd-run --user --scope
--quiet -p MemoryMax=N -p MemorySwapMax=0 -- <app …>`. A `--scope` unit runs
the command in the caller's process, so the `Popen` pid is the app's. Each row
carries `outcome`:

- `ok` — every step ran.
- `oom` — the app died by SIGKILL (`returncode == -9`, or `137` through a
  shell). `oom_phase` is `setup` or `scenario`.
- `refused` — the live app answered a step with an error. At 17000² on the
  reference machine the M12 open is refused by the PSD reader's image budget
  (#844: three quarters of `MemAvailable` for one document's planes).
- `crash` — any other exit, or no answer.

The harness itself never fails on an `oom`. Its exit status is non-zero only
for a usage error or a `--self-check` failure.

## Fixtures

`scripts/gen-large-fixtures.py --size N --out path.psd` writes an 8-bit RGB PSD
with a white Background and four "dense painted" layers. Each layer is a seeded
set of filled discs and gradient bands, which RLE (PackBits) compresses: pure
noise would not, and classic PSD section lengths are 32-bit. The output depends
only on `--size` and `--seed` (default 1210), so the bytes are stable. It uses
the standard library only. The harness generates the M12 fixture into its
temporary directory, or reuses a `--fixture-dir` cache keyed by size and seed.

## PeakAlloc

`pictura_testkit::PeakAlloc<A = System>` wraps a `GlobalAlloc`. It keeps
`current` (`AtomicUsize`, add on alloc, subtract on dealloc, the difference on
realloc) and `peak` (`fetch_max` after each increase). `reset()` sets `peak =
current`. All atomics are `Relaxed`: the counters are statistics, and
`fetch_max` keeps the peak monotone without ordering against other memory.

`pictura-app` installs it as `#[global_allocator]` under `#[cfg(test)]` only.
The shipped staticlib keeps the system allocator.

## Heap guards

`crates/pictura-app/src/cxxqt_object/tests_profiles/memory.rs` holds the
guards; `memory/scenarios.rs` holds one step function per gesture. It sits
under `tests_profiles/` because `cxxqt_object.rs` is at its size ceiling. Each
step repeats the bridge's sequence on `PictureViewRust` with engine calls (the
Qt object cannot be built in a Rust test), in the style of the profiles. M11
and M12 call the save and open jobs' own Rust functions. The move composite is
landed inline, as `move_gesture_profile` does. M13 is view-only and has no
engine path. The M12 guard writes its fixture with the codec (the same seeded
discs as the script), because a Rust test does not call Python.

`measure(side, run)` resets the peak after setup, runs the measured part in a
four-thread rayon pool, and returns `peak - current_at_reset`. Four threads fix
the number of per-thread buffers, so the peak does not depend on the machine's
core count.

- `<scenario>_heap_peak` runs at 4250² and asserts `peak <= baseline * 1.10`.
  `BASELINE` is a table of bytes in `memory.rs`, recorded on `master` with
  this harness. A nextest test group runs two guards at a time.
- `<scenario>_heap_peak_full` is `#[ignore]`d and prints the peak at 17000².

The peak counter is process-wide, so a guard is only exact when it runs alone
in its process. Under `cargo nextest` each test is its own process. Under plain
`cargo test` (threads share a process) the guards print a skip and pass, the
oracle tests' self-skip convention. Nextest sets `NEXTEST=1`.

ponytail: the guards repeat the bridge's sequence by hand, so a bridge change
that adds an allocation the guard does not repeat goes unseen here. The
process benchmark is the end-to-end check.

## `free_transform` control op

`layer_op` gains op `free_transform` with `index`, `scale` (percent, both
axes) and `angle` (degrees). It uses the transform tool's own path, reached
with `findChild<ToolController*>()` as `tool_options` is (`frame.h` is at its
size cap): `beginFreeTransform`, `setTransformWidth`, `setTransformHeight`,
`setTransformAngle`, then `commitFreeTransform`. The reply does not wait: a
deferred commit lands later, and `status` settles it. A session that is
already open is refused, not taken over.

## Self-check

`profile-tools.py --self-check` runs its pure checks: OOM classification, the
`systemd-run` command line, the compare table with a peak and an outcome, and
the JSON schema. With a built app it also runs every M scenario at 512² (M12
with a 512² fixture) and validates each row: `outcome == ok`, `peak_mb >=
rss_mb_before`, and `peak_added_mb >= 0`. `verify-full.sh` runs it after the
build. `gen-large-fixtures.py --self-check` generates 64² twice, compares the
SHA-256, and parses its own header, layer count and RLE row counts back.

## Limit guard

With swap off inside the scope, the kernel makes room for a limited app by
swapping out the rest of the machine. A first 17000² run under 6.3G on the
7.3 GiB reference laptop froze its desktop and needed a reboot. The profiler
therefore refuses a `--memory-max` above `MemAvailable` less 1.5 GiB unless
`--force` is given, and gives a memory run without `--memory-max` that amount
as its limit: an unlimited 17000² run is the same hazard.

## Baseline

The baseline is run on the reference machine (Apple M1, 4 Firestorm + 4
Icestorm cores, 7.3 GiB, Asahi Fedora kernel 6.18) with other programs closed,
at 17000² under `MemoryMax=4G`, each M in its own scope. Runs under 6G and
just below the machine's RAM (the issue asks for 8G) do not fit this machine
and are left to a follow-up issue on a machine with at least 16 GB.

The JSON records `meta.machine` (CPU model, cores, MemTotal, kernel, Qt
version, git commit) and the run under `runs."4G"`, so `--compare-run 4G`
selects it.
