# Design

## Context

See `proposal.md` for the measured costs. The constraints that shape the
approach:

- `pictura_shell` is STATIC and every one of the 122 `tst_*` executables links
  it plus `pictura_app` and `pictura_assets`
  (`crates/pictura-app/cpp/tests/CMakeLists.txt`).
- All five packagers already configure with `-DBUILD_TESTING=OFF`
  (`flatpak/io.github.KookaStudio.KookaPictura.yml:43`,
  `snap/snapcraft.yaml:55`, and the three `.github/actions/*-bundle/action.yml`
  files). `BUILD_TESTING` therefore already separates a development build from a
  shipped one.
- Corrosion hardcodes the Cargo target directory to
  `${CMAKE_BINARY_DIR}/.../cargo/build` (`Corrosion.cmake:751,841`), so a fresh
  CMake build directory always recompiles the Rust crate. A sandbox rebuild is
  not cheap.
- 109 of the 122 Qt Test suites isolate `XDG_STATE_HOME` through
  `pictura::test::ScopedStateHome`, the remaining 13 are pure unit tests with no
  main window, and no suite uses `QSettings`, so suites are safe to run
  concurrently in separate processes. Twenty-odd suites use timed waits, so the
  parallel level needs measurement rather than a saturation default.
- `qt-build-utils` emits `cargo::rerun-if-env-changed=QMAKE` unconditionally
  (`installation/qmake.rs:96`), and CMake exports `QMAKE` to the Cargo build
  (`CxxQt.cmake:108`). A direct Cargo run without `QMAKE` changes the
  fingerprint.
- `check-local-build-caching.sh` self-skips without `qmake6`
  (`scripts/check-local-build-caching.sh:26-29`), and the guards workflow
  installs no Qt, so that CI job's run is a no-op today.

## Goals / Non-Goals

**Goals:**

- Cut the clean-build link and compile cost without changing release artifacts
  or application behavior.
- Make the local verify loop stop paying twice for work it already did.
- Keep every change independently revertible so a regression can be bisected to
  one lever.
- Leave a before/after measurement table for the four acceptance scenarios.

**Non-Goals:**

- Precompiled headers (sccache PCH support is unreliable) and unity builds.
- Nightly-only compiler features, Cranelift, `-Zthreads`, or `cargo-hakari`.
- Changing the default `CMAKE_BUILD_TYPE`.
- Changing release packaging or the shipped binary's shape.
- Adding a new runtime dependency. Mold stays optional.

## Decisions

### 1. `pictura_shell` is SHARED only for development builds

`add_library(pictura_shell SHARED ...)` when `BUILD_TESTING AND NOT WIN32`,
STATIC otherwise. In the shared configuration the shell links `pictura_app`,
`pictura_assets`, and lcms2 PRIVATE, publishes the cxx-qt generated include
directory (`CXX_QT_EXPORT_DIR/crates/pictura_app/include`) and
`crates/pictura-app/cpp` as PUBLIC includes, and each test links only the `.so`
plus `Qt6::Test`. Keeping `pictura_app` PUBLIC would re-add the Rust archive and
the cxx-qt `initializers.o` object to every executable, which is exactly the
duplication this change removes; keeping it PRIVATE without republishing the
export directory would lose the generated headers. The cxx bridge C++ compiled
by `cc` gets `-fPIC`.

**Why:** one link per build instead of 122 links of two large archives. It also
makes the link-pool, split-DWARF, and per-test-debug tuning mostly unnecessary.
The link-line check must prove the test executable names `libpictura_shell.so`
and not `libpictura_app.a`, `initializers.o`, or the qrc object.

**Install:** `cmake --install` installs only `pictura`. The STATIC configuration
is the only installable one; installing a shared-shell development build is
documented as unsupported and verified by installing a `BUILD_TESTING=OFF`
build into a temporary prefix and running it.

**Alternatives considered:**

- Always shared: breaks packaging, requires shipping a `.so`, and needs symbol
  exports on Windows. Rejected.
- OBJECT library: still copies objects into every test and relinks everything.
  Rejected.
- Keep static and only tune the linker: leaves the 18 GB and 1157 CPU-s of
  links in place. Rejected.

**Gate:** a phase-0 spike must link `tst_smoke` and `pictura` and run both. It
checks position independence, the duplicate `decode_image`/`render_text`/
`encode_image` copies that the Rust staticlib also contains, and the cxx-qt
`initializers.o` object library. If the spike fails, the fallback is to keep the
shell static and drop the items that only existed to amortize the static links.

### 2. The cache proof rebuilds one probe object, not the world

`check-local-build-caching.sh --rebuild` keeps a mode that removes a single
object owned by the shell target and rebuilds `pictura_shell`; the hit counter
must increase. `verify-full.sh` calls that mode instead of deleting every `*.o`
under `build/`.

**Why:** the requirement is "unchanged sources reuse cached objects". One object
proves it. Deleting 250 test objects forces the next build to redo test
compilation and every test link.

**Alternatives considered:**

- The issue's sandbox suggestion: a fresh CMake directory recompiles Rust
  (Corrosion target directory), roughly 200 s, and doubles the C++ shell work on
  a cold cache. Rejected.
- Remove the proof from the local gate and rely on CI: the guards job has no
  Qt, so the check self-skips there. Rejected as coverage loss.

### 3. CTest runs with a bounded parallel level

`ctest ... -j "${CTEST_PARALLEL_LEVEL:-4}"` in `test-report.sh` and in the CI
headless job.

**Why:** four is enough to hide process startup and test time without launching
122 offscreen QApplications at once. Every suite is its own process; 109 use
`ScopedStateHome` and the other 13 are pure unit tests. Timed waits are common,
so the level is measured at 2, 4, and 6 with two runs each before choosing, and
CI may settle on a lower level than a developer machine.

**Alternatives considered:** `-j 16` (memory spikes, timing flakes), per-test
`PROCESSORS` properties (more config for no extra control).

### 4. Mold applies to C++ links only

On ELF platforms, CMake probes for `mold` with `find_program` and confirms the
compiler accepts `-fuse-ld=mold` with `CheckLinkerFlag`; it then probes `lld` the
same way and falls back to the compiler default if neither is usable. macOS and
Windows keep their native linker. `.cargo/config.toml` keeps lld. CI installs
mold. The explicit `-DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld` is removed from
`verify-full.sh`, `check-local-build-caching.sh`, and `ci.yml`; otherwise the
explicit-choice rule would make mold unreachable in exactly the paths the spec
says must exercise it.

**Why:** `-fuse-ld=mold` has no automatic fallback, needs GCC 12.1 or newer, and
is not a macOS or MSVC linker. Pinning it in Cargo config would break
`cargo test` on a machine without mold. The Rust test binaries are a small
fraction of the C++ link cost.

**Alternatives considered:** `CMAKE_LINKER_TYPE MOLD` (needs CMake 3.29 while
the project floor is 3.24); pinning mold in both configs (no fallback); probing
only the executable (a GCC 11 machine would configure and then fail to link).

### 5. The Rust build keeps the standard release profile

The issue proposed a custom `app` profile (`inherits = "release"`,
`opt-level = 2`, `debug = "line-tables-only"`) passed through Corrosion's
`PROFILE`. It was measured on this machine and rejected: the `_cargo-build_pictura_app`
edge took **249 s** with the `app` profile and **182 s** with `--release` on a
cold build. The line-table debug info costs more than the opt-level drop saves,
so the development CMake build uses the standard release profile and packaging
is unchanged.

**Alternatives considered:** keeping `opt-level = 2` with `debug = false`
(a second rebuild to test a profile whose release baseline is already faster);
editing `[profile.release]` (changes shipped artifacts).

### 6. Heavy sources compile at `-O1`; Qt Test units at `-g1`

In optimized development configurations only, `command_tree.cpp` and
`filter_commands.cpp` get `-O1` (GCC/Clang/AppleClang), and test translation
units get `-g1`. Release and Debug keep the normal settings.

**Why:** 99 s and 37 s for menu construction code that is not hot. Splitting
`command_tree.cpp` is the alternative but is a large review surface for a file
that is already past the 800-line target; keep the move as a fallback if `-O1`
does not pay.

### 7. No split DWARF

Use `-g1` rather than `-gsplit-dwarf`. **Why:** sccache's split-DWARF handling
is the known risk, and the shared shell already removes the link pressure split
DWARF would relieve.

### 8. Explicit moc includes

Each QObject header with a same-named `.cpp` gets
`#include "moc_<header>.cpp"` at the end of that `.cpp`; `acr/acr_panels.h`,
which has no same-named `.cpp`, gets its include in `acr/acr_basic_panel.cpp`.
No new guard: a guard that understands AUTOMOC's naming would cost more than the
one-line convention, which is documented instead.

**Why:** `mocs_compilation.cpp` currently recompiles for any moc change. With
explicit includes, a header edit recompiles its owning translation unit, which
already depends on that header.

**Alternative considered:** leave it; a 13.7 s central unit is small but it is
paid on every header edit.

### 9. Link job pool

`CMAKE_JOB_POOLS link=4` and `CMAKE_JOB_POOL_LINK link` when Ninja is used,
placed before the first `add_library`/`add_executable` because CMake reads the
variable when it creates each target.

**Why:** keeps the initial parallel compile from firing many large links at
once. This is the first item to drop if the shared shell makes it flat; the
tasks measure it.

### 10. QMAKE is exported by the verify scripts

`scripts/qmake-env.sh` reads `QMAKE_EXECUTABLE` from `build/CMakeCache.txt` when
a build tree exists and exports it before any Cargo step; with no build tree it
resolves `qmake6` then `qmake` from `PATH`, matching the configure's order. It
is sourced by `verify-fast.sh` and `test-report.sh`.

**Why:** stabilizes the cxx-qt build-script fingerprint across
`verify-full` → `verify-fast`. A direct `cargo test` still bounces once after a
CMake build; full stabilization would require changing cxx-qt's environment
contract, which is out of scope.

### 11. FetchContent base is persistent

All throwaway configures in `check-local-build-caching.sh` share
`-DFETCHCONTENT_BASE_DIR=${XDG_CACHE_HOME:-$HOME/.cache}/kooka-pictura/fetchcontent`.
Only the real build tree is built; the shared base is configure-time content.

**Why:** three configures currently clone cxx-qt-cmake and Corrosion three
times. A user-level cache clones once per machine. It lives outside the repo so
worktrees share it and no artifact is committed.

**Trade-off:** `FETCHCONTENT_BASE_DIR` also holds the dependencies' build
directories. The check takes a pid lock next to the base; a concurrent run that
does not get the lock falls back to a per-run sandbox base, so two runs never
write the shared base at once. The opt-in `PICTURA_VERIFY_LOCK` still serializes
whole verification runs. The check builds only in the inspected tree, never in
the shared base.

### 12. Fast-gate checks run concurrently with a portable runner

A small start/wait/aggregate helper in `verify-fast.sh` runs the independent
script checks in the background, captures each log in a temp directory, waits for
all, and reports every non-zero status. No `wait -n` (bash 3.2 on macOS).

**Why:** the scripts are independent and I/O-bound. Cargo steps stay sequential
because they share the build directory.

## Measurement protocol

On the same machine, warm and cold, before and after:

| Scenario | Command | Record |
|---|---|---|
| Clean build | remove `build/`, configure, `cmake --build build --parallel` | wall time, `build/.ninja_log` CPU-s per edge class, `du -sh build` |
| One shell `.cpp` edit | `touch crates/pictura-app/cpp/command_tree.cpp`, rebuild | wall time, objects rebuilt |
| One QObject header edit | touch a header, rebuild | whether `mocs_compilation.cpp` recompiled |
| `verify-fast.sh` | `/usr/bin/time` after a warm build | wall time |
| `verify-full.sh` | `/usr/bin/time` after a warm build | wall time |

The first clean build after the change must also confirm that packaging
(`-DBUILD_TESTING=OFF`) still links a static shell and needs no extra shared
object.

## Risks / Trade-offs

- [Shared shell link failure: non-PIC bridge objects or duplicate symbols] →
  phase-0 spike; `-fPIC` covers the closure and `CXXFLAGS=-fPIC` covers the
  separate cxx-qt `initializers.o`; link `pictura_app`/assets PRIVATE; assert
  the test link line names only the `.so`; fall back to a static shell.
- [Dev binary needs the `.so` at runtime] → build-tree RUNPATH; release and
  packaging stay static. Dev installs are unsupported and documented.
- [Installing a shared-shell build produces a broken binary] → install only from
  `-DBUILD_TESTING=OFF`; the verification installs into a temp prefix and runs
  the self-test.
- [`ctest -j 4` surfaces timing flakes] → measure at 2, 4, and 6 with two runs;
  lower the level before touching a test.
- [`-O1` on `command_tree.cpp` changes menu-construction codegen] →
  `tst_command_tree` covers the tree, and `tst_filter_menu`/`tst_filter_gallery`
  cover `filter_commands.cpp`; split the file if a behavior check fails.
- [Mold is missing, too old for the compiler, or on macOS/Windows] → probe the
  compiler flag, probe lld, keep the native linker for Apple/MSVC and the
  compiler default when nothing is usable; no Cargo changes.
- [`-g1` breaks sccache or a backtrace expectation] → measure; drop the item
  with the number rather than keep dead config. The raw flags are
  GCC/Clang/AppleClang only.
- [110-file moc diff obscures review] → one mechanical commit separated from the
  build changes.
- [Two verification runs share the FetchContent base] → the opt-in verify lock
  serializes them; the check does not build in that base.
- [Final gates run before archiving and fail on the open change] → archive
  before the final `verify-fast`/`verify-full`; the individual group checks run
  before archive.

## Migration Plan

1. Phase 0 records the baseline and runs the shared-shell spike.
2. Phases 2–5 land one lever group at a time, each with its own measurement and
   the checks it touches. The full gates run after the change is archived; an
   open change directory makes `check-openspec-archived.sh` fail by design.
3. Rollback is per lever: reverting a phase restores the previous CMake option
   or script call. The shared shell is the only change that requires reverting a
   spec delta, and it is isolated in phase 3.
4. Archive the change in the same PR that ships the code, then run the final
   `verify-fast`/`verify-full`.

## Open Questions

These are measurable during implementation and do not change the specs, the
approach, or the task order:

- Whether `ctest -j 4` is the right ceiling, or 2 or 6 is better on this machine
  and in CI.
- Whether `-O1` alone is enough for `command_tree.cpp`, or the file split is
  needed.
- Whether the link job pool and `[profile.test.package."*"] debug = false` pay
  for themselves after the shared shell.
- Whether the shared `FETCHCONTENT_BASE_DIR` needs its own lock when
  `PICTURA_VERIFY_LOCK` is off; the fallback is a per-run base.
