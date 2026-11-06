# Proposal

## Why

Issue #1210, step 1 of the bounded-memory plan (#1209). Every later step claims
to save memory on large documents, and none of those claims can be checked
without a repeatable before/after measurement. `scripts/profile-tools.py` and
the `tests_profiles/*` profiles measure time and the resident size *after* a
scenario. Nothing measures the **peak** a scenario reaches. Nothing checks
whether the app survives under a hard memory limit, and nothing stops a later
change from raising the heap peak again.

## What Changes

- **Peak resident memory per scenario.** `profile-tools.py --peak` resets the
  app's peak (`5` → `/proc/<pid>/clear_refs`) before each scenario. It reports
  the peak (`VmHWM`), the peak the scenario added over the resident size at
  its start, the resident size after it, and its wall time.
- **A hard memory limit.** `--memory-max <size>` runs the app in
  `systemd-run --user --scope -p MemoryMax=<size> -p MemorySwapMax=0`. An app
  killed by SIGKILL is recorded as an `oom` outcome, naming the phase it died
  in (setup or scenario). It is not counted as a harness failure.
- **The memory scenarios M1–M13** (17000×17000 RGB 8-bit, white Background).
  Each runs in a fresh app process. Its prerequisites run first and are not
  measured, so an OOM in one scenario does not hide the others.
- **`--compare`** prints the peak and the outcome before → after, next to the
  existing times.
- **`PeakAlloc`** in `pictura-testkit` is a std-only counting `GlobalAlloc`
  wrapper (current, peak, reset). The `pictura-app` unit-test binary installs
  it. A heap-peak guard per scenario at 4250² fails when the peak exceeds the
  recorded baseline by more than 10 %. An `#[ignore]`d test per scenario
  reports the peak at full size.
- **`scripts/gen-large-fixtures.py`** writes the M12 PSD deterministically,
  RLE-compressed, at any size. Fixtures are generated, never committed.
- **A committed baseline**, `docs/dev/benchmarks/memory-baseline.json`: M1–M13
  at 17000² on the reference machine (7.3 GiB) under `MemoryMax=4G`. Runs under
  larger limits froze the reference laptop and move to a follow-up issue on a
  machine with at least 16 GB.
- **A limit guard.** `--memory-max` above the memory available now less 1.5 GiB
  is refused unless `--force` is given: with swap off in the scope, the kernel
  swaps out the rest of the machine to make room.
- **A `free_transform` control op** (begin, numeric scale/rotate, commit) for
  M10. The control server had no way to drive Free Transform.
- `check-test-plan.py` resolves `<script>::<fn>` coverage references against
  `scripts/<script>.py`, so the harness's own checks count as automated.

## Capabilities

### New Capabilities

- `verification/memory-benchmarks`

The issue names `tooling/memory-benchmarks`. `tooling` is not a domain, and
`verification/` already holds `tool-profiler`.

### Modified Capabilities

- `interop/agentic-control`: adds the `free_transform` `layer_op`, and
  `background_layer` on `document new` so the benchmark's document is File ▸
  New's locked Background (the control path made an unlocked `Layer 0` with an
  alpha plane, 289 MB more at 17000²).

## Impact

`pictura-testkit` (`PeakAlloc`), `pictura-app` (test-only global allocator,
`tests_profiles/memory` guards, the `free_transform` control op), `scripts/`
(`profile-tools.py`, `memory_scenarios.py`, `gen-large-fixtures.py`,
`check-test-plan.py`, `verify-full.sh`), `docs/dev/benchmarks/`.
