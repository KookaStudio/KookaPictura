# Proposal

## Why

`pictura` is a single native process (a Rust `staticlib` linked into the C++ Qt
binary), so external samplers attach with no application code. Yet no documented
recipe produces a release-mode build in which both Rust and C++ frames carry
source-line attribution: the workspace has no `[profile.release]`, the packaging
configure compiles C++ at `-O3` without `-g`, and `DEVELOPING.md` covers only the
in-app timing harness. A plain release binary already resolves function names
from `.symtab`; what it lacks is the line-level attribution and inlined-frame
information that make a sampling report actionable. Worse, the one sentence the
documentation does offer about C++ profiling points readers at that packaging
configure — the only configure that also drops the Rust line tables the
development profile carries. `docs/01-architecture/performance-targets.md`
(ARCH-013) requires in-app timers — `QElapsedTimer` and the Rust monotonic
timers (`docs/01-architecture/performance-targets.md:134-139`) — to validate its
budgets, and the remaining frame-budget and scheduler instrumentation is tracked
by #941 and #944; none of those helps find where a sampled stall comes from. A
documented sampling workflow is the missing piece (#661).

## What Changes

- `DEVELOPING.md` gains a sampling-profiling recipe: a profiling configure
  (`BUILD_TESTING` off, `RelWithDebInfo`) whose build carries Cargo's
  `CARGO_PROFILE_RELEASE_DEBUG` environment override, so Rust and C++ carry line
  tables without editing `Cargo.toml`; the `perf` and `samply` record commands;
  and the `perf_event_open` prerequisite, split between user-space and kernel
  frames.
- The recipe states the budget split: the profiling **build** runs under
  `scripts/cpu-budget.py` like any other heavy command, while the **recording**
  runs on every CPU because the budget's affinity slice changes the thread pool
  and cache behaviour being measured.
- The existing note that sends C++ profiling at a packaging configure is
  corrected to name the debug-info override and link the recipe.
- `.env.example` gains a commented `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`
  entry under its external-build-knobs section, since that file lists every
  environment variable the project uses.
- Explicitly out of scope: any change to Cargo profiles, CMake flags, or shipped
  artifacts; full-inline fidelity (`CARGO_PROFILE_RELEASE_DEBUG=2` is documented
  as the escalation, not the default); frame-pointer builds; further in-app
  instrumentation (#941, #944; the #651 metrics half already shipped); and
  shipping debug info in release artifacts, which is a crash-reporting decision
  rather than a profiling one.

## Capabilities

### New Capabilities

- `meta/profiling-workflow`: the repository's contract for sampling a
  release-mode build — which configure and override make Rust and C++ frames
  carry source-line attribution, that shipped artifacts keep their current
  profile settings, and that `DEVELOPING.md` documents the runnable recipe and
  its prerequisites.

### Modified Capabilities

None. The build itself does not change: `meta/build-performance` continues to
pin the release profile for packaging and the development profile for optimized
development configures, and `meta/cpu-budget` continues to govern heavy commands
and gates.

## Impact

`DEVELOPING.md` and a commented `.env.example` entry only. No source,
`Cargo.toml`, `CMakeLists.txt`, workflow, dependency, or artifact change; the
profiling build is a local, opt-in configure whose debug information is supplied
per invocation from the environment.
