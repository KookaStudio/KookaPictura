# Design

## Context

See `proposal.md` for the problem and scope. The Qt Test layer registers one
CTest entry per suite (several per sharded suite) and writes one JUnit report
per entry. `scripts/report_tests.py` keeps status and failure data from those
reports and drops every `time` attribute. Shard membership is the literal list
`PICTURA_QT_TEST_SHARDS` in `crates/pictura-app/cpp/tests/CMakeLists.txt`, split
in declaration order by `shardedMain`, and the fixed-wait policy is a Rust
`#[test]` in `crates/pictura-app/src/build_config.rs`.

#1566 introduces the registry-driver model: one process and one CTest entry per
suite, dispatching by suite name. This change builds on it instead of starting a
competing sharding track.

## Goals / Non-Goals

**Goals:**

- Retain per-entry and per-function Qt timing, and publish the raw artifacts.
- Derive shard membership and count from that evidence.
- Keep explicit test-function selection, empty-shard detection, and the existing
  `shardedMain` algorithm.
- Move the fixed-wait policy to a guard that runs on C++-only changes.
- Remove the Rust scanner once the guard runs everywhere.

**Non-Goals:**

- Change suite bodies, assertions, state isolation, timeouts, or JUnit names.
- Add a scheduler module, a dependency, or a generated C++ registry.
- Group suites into one process (that is #1566's registry, not this change).
- Run builds, tests, or measurements in this task.

## Decisions

### 1. The reporter keeps the duration it already parses

`parse_qt_junit` already walks every `testcase`; `junit_timings` returns
`{suite: seconds}` and `[(suite, case, seconds)]` from the same XML. The Qt
`Layer` carries the suite totals in `times`, and `render` prints a suite's
seconds beside its counts when present. A missing or malformed `time` reads as
0.0, so a supplier without durations cannot fail the report.

The alternative — a second XML walker in a separate script — duplicates the
parser and drifts. Reusing the existing parse keeps one reader of the format.

### 2. One timing store, two consumers

`scripts/qt_test_timings.py collect` merges the JUnit reports and CTest's
`build/ctest-junit.xml` (wall time per entry) into
`build/qt-test-timings.json`. The store has `suites` (seconds, per-function
seconds, run count, evidence string) and `entries` (CTest wall time per entry).
CI uploads `build/qt-test-results/*.xml`, `build/ctest-junit.xml`, and
the store as one artifact, so the raw evidence survives the run.

`.ci/qt-test-timings.json` is the committed seed carrying the recorded
`tst_quick_mask` (112 s) and `tst_document_navigate` (21 s) runs under a 389 s
layer total. Those runs recorded only a suite total, so the seed's `functions`
is empty and both suites keep the declaration-order split until a CI run
refreshes the store with per-function times; a CI run overwrites it from fresh
data.

The alternative — parsing the artifacts only inside CI — loses the evidence once
the run is gone and cannot gate a local regenerate.

### 3. The plan is data the CMake registry reads

`qt_test_timings.py plan` writes `cmake/qt-test-shards.json`: per suite, a
`count` and, when per-function evidence exists, the exact `shards` (lists of
function names), plus an `evidence` string naming the seconds, share count, and
largest share. The algorithm sorts functions longest-first and fills the
emptiest bin, so shares stay balanced; the count is the fewest shares that keep an
entry under a quarter of the layer's measured total, capped by the function count
so no share is empty.

The test `CMakeLists.txt` reads the plan with `string(JSON ...)`. A suite in the
plan registers `count` entries named `<suite>_<index>`, each with
`PICTURA_TEST_SHARD=<index>/<count>`; a measured share passes its function names
on the command line, and `shardedMain` keeps named functions as given. A share
without named functions falls back to the declaration-order split, which is the
existing behaviour. A missing plan warns and registers every suite in one entry
rather than failing configuration, so a fresh clone without the plan still builds.

`qt_test_timings.py verify` rejects a plan whose share is empty, whose functions
repeat, or which omits a function the suite source declares; CI runs it every
build. `carry_forward` keeps a baseline entry when a partial store has no fresh
measurement, so regenerating from a short run cannot silently un-shard a suite.

The alternative — a hand list of functions per share — rots the moment a suite
gains a function and is exactly the declaration-order weakness the issue names.

### 4. Suite isolation is per process and per entry, not per file

The "one executable per `tst_*.cpp`" wording guards suite isolation: a crash
names one suite, each suite owns its JUnit file, and each suite keeps its
timeout. The registry-driver model (#1566) keeps one process per CTest entry and
one entry per suite, so every guarantee holds while the executable count is per
subsystem. This change does not group suites; it changes which entries a
sharded suite has and how a share's functions are chosen.

### 5. The fixed-wait policy runs where C++ is checked

`scripts/check-qt-test-waits.py` carries the exact rule the Rust test had: a
`qWait(` call whose argument is not `0` must trail a `//` comment or sit under a
comment-only line. It reads only `crates/pictura-app/cpp/tests/*.{cpp,h}` and
supports `--changed` for scoped local scans.

`scripts/guard.sh` runs it (self-test then the scoped scan), so
`scripts/verify-fast.sh` and `.github/workflows/guards.yml` cover it for every
pull request, including a C++-only one. The Qt-aware CI job also runs it
alongside the shard-plan verification. `qt_test_waits_are_justified` is deleted
only after all three paths are wired, which the task order enforces.

The alternative — keep the Rust test and add a second copy — leaves two rules to
drift, and the issue asks for the move, not a duplicate.

### 6. The longest entry is reported, not assumed

`qt_test_timings.py report` prints each CTest entry's seconds against a quarter
of the layer total, marking `over` or `within`. CI writes it after every run, so
the `qt-test-harness` longest-entry rule is visible in the log and the raw
artifact carries the same numbers. The plan's `evidence` string records the
largest predicted share, so the shard decision and the observed entry time can be
compared.

## Risks / Trade-offs

- `cmake/qt-test-shards.json` is data, so a suite rename or removal leaves a
  stale plan entry. Mitigation: `verify` uses the suite source to reject unknown
  functions, and CMake configuration fails when a plan entry names no registered
  suite, so a rename cannot silently drop sharding.
- The committed seed has no per-function splits, so those suites keep the
  declaration-order split until a CI run refreshes the store. Mitigation: the
  plan's `evidence` says so, and the next CI capture fills the functions in.
- Moving the wait check to Python changes the failure text. Mitigation: the
  `--self-check` asserts both arms (a justified comment passes, a bare non-zero
  wait fails) and the guard runs the self-test before the scan.
- A C++-only pull request now pays one extra Python scan. Mitigation: the check
  scans only the Qt test directory and honours `--changed`.
- CI artifact size grows by the JUnit XML set. Mitigation: the files are already
  produced per suite and are small relative to the build artifacts uploaded.
- No build, test, or measurement has run in this change. The plan and store are
  review-clean only, and the first CI run is what proves the CMake JSON parse and
  the artifact paths.

## Migration Plan

1. Add the timing store and the guard with their self-checks.
2. Wire `scripts/report_tests.py`, `scripts/test-report.sh`, `scripts/guard.sh`,
   and `.github/workflows/ci.yml`.
3. Add `cmake/qt-test-shards.json` and read it from the test CMake registry.
4. Delete `qt_test_waits_are_justified` after the guard runs in all three paths.
5. Roll back by reverting the change as one unit; the plan is data, so reverting
   it restores the single-entry registration.

## Open Questions

None for implementation. The measured before/after shard wall time is a follow-up
observation, not a claim this change makes.
