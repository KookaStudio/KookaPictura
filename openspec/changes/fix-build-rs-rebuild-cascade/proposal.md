# Proposal

## Why

The local edit loop pays for work it did not ask for. Touching one shell
`.cpp` reruns the cxx-qt build script, which rewrites every generated header
with a fresh timestamp, which makes the *next* build recompile all 267 shell
translation units — a second, unrequested full C++ rebuild after every edit.
The same rewrite also means the first build after a real bridge change
compiles the C++ against the *previous* header, so a shared struct whose layout
changed is silently mismatched across the boundary.

The report is not honest either: `scripts/test-report.sh` printed "0 failed"
while 14 CTest suites were "Not Run", because each suite writes its own JUnit
file and a suite that never starts writes none. The local gates will happily
test a build tree that predates the sources under test.

## What Changes

**Build script change tracking**

- Stop the cxx-qt build script from rerunning when an unrelated file under
  `crates/pictura-app/` changes. The default `crate_include_root` makes cargo
  watch the whole crate directory, so every C++ source is an input.
- Track the hand-written bridge headers and sources the build script compiles
  explicitly, because the C compiler helper emits no change-tracking directive
  of its own.
- Write the generated headers only when their content changed, so a no-op
  rerun leaves their timestamps alone.

**The C++ rebuild lands in the same build**

- Detect a bridge change before the build decides what to compile, and rebuild
  the C++ that depends on the generated headers in that same build, so no
  second build is needed and the C++ never compiles against a stale header.
- Measure the heaviest C++ translation units and the clean build's critical
  path, and keep only the changes that measurably pay.

**One QMAKE for every cargo invocation**

- `qt-build-utils` tracks `QMAKE` unconditionally, and the CMake build exports
  an absolute path that a bare `cargo` does not, so alternating the two gates
  rebuilds `cxx-qt-lib`, `cxx-qt` and `pictura_app` for nothing. Every cargo
  invocation has to see the same value, and the helper that half-solved it
  goes away.

**Honest reports and gates**

- A registered CTest suite that produced no report is a failure, not a
  silently missing line.
- The local gates detect a build tree that predates the sources under test.
- The two app self-test invocations run concurrently with the other layers.
- The build-caching check configures the project once in its default mode and
  keeps the three-case variant for CI.
- The three slowest Qt Test suites stop bounding the CTest run: they are
  registered in several entries each, and no suite times out at any
  parallelism.

**Docs**: `DEVELOPING.md`, `AGENTS.md` and `.serena/memories/tech_stack.md`
describe a lighter `app` Cargo profile that was measured and rejected in #771
and does not exist. They are corrected to what the build actually does.

## Capabilities

### New Capabilities

None. Every behaviour below extends an existing capability.

### Modified Capabilities

- `meta/build-performance`: the build script tracks only the bridge sources;
  generated headers keep their timestamp when their content is unchanged; a
  bridge change rebuilds the C++ in the same build; the heaviest C++
  translation units are measured; and every cargo invocation sees the same
  `QMAKE`.
- `verification/test-reporting`: a registered suite that produced no report
  fails the run, and the test layers run concurrently.
- `verification/verification-harness`: the local gates detect a stale build
  tree and a missing suite executable.
- `meta/local-build-caching`: the default mode of the caching check configures
  the project once instead of once per case.
- `verification/qt-test-harness`: the Qt Test layer's critical path is not a
  single suite, and the suites wait for the state they observe.

## Impact

- `crates/pictura-app/build.rs` — change tracking and write-if-changed for the
  generated headers.
- `CMakeLists.txt` — the bridge stamp and the per-source dependency on it.
- `.cargo/config.toml`, `scripts/qmake-env.sh` (removed),
  `scripts/verify-fast.sh`, `scripts/test-report.sh`,
  `scripts/report_tests.py`, `scripts/check-local-build-caching.sh`.
- `crates/pictura-app/cpp/tests/tst_floating_workspaces.cpp`,
  `tst_workspaces.cpp`, `tst_import_pdf.cpp`, and the tests'
  `CMakeLists.txt`.
- `DEVELOPING.md`, `AGENTS.md`, `.serena/memories/tech_stack.md`.
- No new runtime dependency and no new build dependency. The C++ compiler,
  Cargo and the Qt version are unchanged; only the change-tracking,
  scheduling and reporting change.
- The application binary is byte-for-byte unaffected: no source, no
  dependency, no link flag and no runtime behaviour changes.
