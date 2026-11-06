## ADDED Requirements

### Requirement: Peak resident memory per scenario

With `--peak`, `scripts/profile-tools.py` SHALL reset the app's peak resident
size before each scenario's measured steps by writing `5` to
`/proc/<pid>/clear_refs`. After those steps and the app's deferred work have
settled, it SHALL report the peak (`VmHWM`) as `peak_mb`, the peak less the
resident size at the reset as `peak_added_mb`, the resident size as `rss_mb`,
and the wall time as `total_ms`.

#### Scenario: A scenario reports its peak [mb_peak]

- **WHEN** the profiler runs a memory scenario with `--peak`
- **THEN** its row holds `peak_mb`, `peak_added_mb`, `rss_mb` and `total_ms`,
  with `peak_mb` at or above the resident size at the reset and
  `peak_added_mb` at or above zero

### Requirement: Running under a hard memory limit

With `--memory-max <size>`, the profiler SHALL launch the app as `systemd-run
--user --scope -p MemoryMax=<size> -p MemorySwapMax=0`. An app that dies by
SIGKILL SHALL give the row `outcome: oom` with `oom_phase` set to `setup` or
`scenario`. A live app that answers a step with an error (an open over the
image budget, for example) SHALL give `outcome: refused` with the app's
message. Any other death, or no answer, SHALL give `outcome: crash`. None of
these SHALL stop the remaining scenarios or fail the profiler. Because swap is off inside the
scope, the kernel makes room for the app by swapping out the rest of the
machine; the profiler SHALL refuse a limit above the memory available now less
1.5 GiB unless `--force` is given, and memory scenarios run without
`--memory-max` SHALL get that amount as their limit unless `--force` is given.

#### Scenario: The limit wraps the app [mb_limit_command]

- **WHEN** the profiler builds the app's command line with `--memory-max 4G`
- **THEN** it is prefixed by `systemd-run --user --scope` with `MemoryMax=4G`
  and `MemorySwapMax=0`, and without `--memory-max` it is the app alone

#### Scenario: A limit the machine cannot spare is refused [mb_limit_guard]

- **WHEN** `--memory-max` exceeds the memory available now (`MemAvailable`)
  less 1.5 GiB and `--force` is not given
- **THEN** the profiler exits with a message naming both amounts before it
  launches anything, and a memory run without `--memory-max` is limited to
  the available memory less 1.5 GiB

#### Scenario: A killed app is an OOM, not a harness failure [mb_oom_outcome]

- **WHEN** the app's exit status is SIGKILL (`-9` or `137`) during setup or
  during the measured steps, another status, or the live app answers a step
  with an error
- **THEN** the row is `oom` with that phase, `crash`, or `refused`, and the
  profiler goes on to the next scenario

### Requirement: The memory scenario catalogue

The profiler SHALL provide scenarios M1–M13 as listed in this change's
design. They default to 17000×17000 RGB 8-bit with a white Background, and
`--size` scales every geometry with the document. Each SHALL run in a fresh app
process, with its prerequisites run before the peak reset and not measured.

#### Scenario: Every scenario runs at a small size [mb_catalogue]

- **WHEN** `profile-tools.py --self-check` runs with a built app
- **THEN** every scenario M1–M13 runs at 512×512 with `outcome: ok` and a
  valid row

### Requirement: Before and after memory comparison

`--save` SHALL write the memory fields and each row's outcome to the JSON, and
`--compare` SHALL print, per scenario, the peak and the outcome before → after
next to the existing times. No separate tool SHALL be added.

#### Scenario: Compare shows the peak change [mb_compare]

- **WHEN** two saved runs differ in a scenario's peak and another's outcome
- **THEN** the compare table shows that peak before → after with the ratio, and
  that outcome before → after

### Requirement: A counting allocator for heap peaks

`pictura-testkit` SHALL provide `PeakAlloc`, a standard-library `GlobalAlloc`
wrapper that counts the bytes currently allocated and their peak, with a reset
that sets the peak to the current count. It SHALL add no dependency.

#### Scenario: Allocations are accounted [mb_peak_alloc]

- **WHEN** blocks are allocated, grown and shrunk by realloc, and freed, with a
  reset between
- **THEN** `current` follows every change, `peak` is the largest `current`
  since the last reset, and a reset sets `peak` to `current`

### Requirement: Heap peak guards

`pictura-app` SHALL install `PeakAlloc` in its unit-test binary only. For each
scenario M1–M12 it SHALL have a test that repeats the bridge's work at
4250×4250 and fails when the heap peak exceeds the recorded baseline by more
than 10 %, and an ignored test that reports the peak at 17000×17000. Under a
runner that shares one process between tests, the guards SHALL skip.

#### Scenario: A guard holds the heap peak [mb_heap_guard]

- **WHEN** `cargo nextest` runs the memory guards
- **THEN** each of M1–M12 measures its heap peak at 4250² and passes at or
  below its baseline × 1.10

### Requirement: Generated large fixtures

Large fixtures SHALL be produced by `scripts/gen-large-fixtures.py` and SHALL
NOT be committed. For a size and a seed, the generator SHALL write identical
bytes on every run: an 8-bit RGB PSD with a white Background and four densely
painted layers, with RLE-compressed channels.

#### Scenario: The fixture is deterministic and readable [mb_fixture]

- **WHEN** the generator writes the same size and seed twice
- **THEN** both files are identical, and the PSD header, the five layers and
  their RLE row tables parse back

### Requirement: A committed memory baseline

`docs/dev/benchmarks/memory-baseline.json` SHALL hold the M1–M13 results at
17000² on a named reference machine (CPU, cores, RAM, kernel, commit), run
under `MemoryMax=4G`, recording which scenarios were OOM-killed or refused and
the peak of each that completed. Runs under larger limits need a machine with more memory
than the reference laptop and are tracked separately.

#### Scenario: The baseline records the reference machine [mb_baseline]

- **WHEN** the baseline file is read
- **THEN** it names the machine and holds, for every M1–M13 of the `4G` run,
  an outcome and, where the scenario completed, a peak
