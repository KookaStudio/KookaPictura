# Tasks

Work the groups in order. Each group lands its own tests and documentation and
ends with a measurement. The measurement builds are the acceptance exception to
the single-compile rule: the issue requires before/after timings. The full
verification gates can only run after the change is archived (group 6), because
`check-openspec-archived.sh` fails while the change directory is open.

## 1. Baseline and spike

- [x] 1.1 Write `test-plan.md` in this change folder with one row per scenario in every delta spec, and verify `python3 scripts/check-test-plan.py` reports no missing row
- [ ] 1.2 Record the before numbers on this machine: clean configure and build (wall, `build/.ninja_log` CPU-s, `du -sh build`), a rebuild after touching one shell `.cpp`, a rebuild after touching one QObject header, `verify-fast.sh`, and `verify-full.sh`; verify every row is captured with command and duration
- [x] 1.3 Spike the shared shell locally (throwaway edit): `pictura_shell` SHARED when `BUILD_TESTING`, `-fPIC` on the cxx-qt `cc` bridge, `pictura_app` and `pictura_assets` linked PRIVATE, the cxx-qt export include directory republished PUBLIC, and tests linked to the shell plus `Qt6::Test` only; verify `tst_smoke` links, loads, and passes, the test link line does not name `libpictura_app.a`, `initializers.o`, or the qrc object, and `./build/pictura --headless --self-test` still passes; record go/no-go
- [ ] 1.4 If the spike fails, drop the shared-shell tasks and the link-pool task, record the failure in `design.md`, and verify the remaining task groups still stand on their own

## 2. Verify loop

- [x] 2.1 Clear stale `build/qt-test-results/*.xml` before CTest in `scripts/test-report.sh`; verify a deleted suite's old JUnit file is not reported
- [x] 2.2 In `scripts/test-report.sh` and `.github/workflows/ci.yml`, run CTest with `-j "${CTEST_PARALLEL_LEVEL:-4}"` and echo the chosen level; verify every suite still writes its own JUnit file, the report counts are unchanged, and a failure still fails the report
- [x] 2.3 Time `test-report.sh` at `-j 2`, `-j 4`, and `-j 6`, twice each; verify the chosen level is stable and record peak memory before fixing the default
- [x] 2.4 In `scripts/check-local-build-caching.sh`, replace the whole-tree `*.o` deletion with a one-probe-object rebuild against the directory passed in; verify `sccache --show-stats` hits increase, the probe object belongs to the rebuilt target, and a test object created beforehand still exists afterwards
- [x] 2.5 In `scripts/verify-full.sh`, call the probe mode with the real `build/` directory instead of deleting every object; verify no Qt Test object or test executable was removed
- [x] 2.6 In `scripts/check-local-build-caching.sh`, pass one persistent `-DFETCHCONTENT_BASE_DIR` to all three configures and build only in the inspected directory; verify a run fetches cxx-qt-cmake and Corrosion once (configure log or directory contents) and no build output is written to the shared base
- [x] 2.7 In `scripts/verify-fast.sh`, run the independent script checks concurrently with captured logs and aggregated failure reporting; keep any check that writes the repo-root `compile_commands.json` away from a check that reads it; verify a deliberately failing check fails the gate after the others finish and prints its log
- [x] 2.8 Update the verification and caching guidance in `DEVELOPING.md` and `AGENTS.md` (parallel CTest and its override, stale JUnit clearing, probe proof, FetchContent cache, concurrent checks); verify the documented commands match the scripts
- [x] 2.9 Run `test-report.sh`, `test-report.sh -j` overrides, the caching check in both modes, and the relevant guard scripts; verify they pass and record the after numbers for the verify rows

## 3. Linker and shell

- [x] 3.1 In `CMakeLists.txt`, probe mold only on ELF platforms with `CheckLinkerFlag`, probe lld the same way, fall back to the compiler default when neither is usable, and keep the native linker on macOS and Windows; verify a configure with mold links with it and a configure without mold still succeeds
- [x] 3.2 Remove the explicit `-DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld` from `scripts/verify-full.sh`, `scripts/check-local-build-caching.sh`, and `.github/workflows/ci.yml`, install mold in CI, and update `scripts/install-deps-fedora.sh`, `AGENTS.md`, and `DEVELOPING.md`; also clear a cached `CMAKE_EXE_LINKER_FLAGS` in an existing `build/` (fresh directory or `-U`) so the probe is not masked; verify the CI headless configure selects mold and the documented command matches
- [x] 3.3 Build `pictura_shell` SHARED when `BUILD_TESTING AND NOT WIN32`, link `pictura_app` and `pictura_assets` PRIVATE, republish the cxx-qt export include directory and `crates/pictura-app/cpp` PUBLIC, add `-fPIC` to the cxx-qt `cc` bridge, and link each test and `pictura` against the shell only (`Qt6::Test` for tests); verify all Qt suites and the self-test pass and no executable link line repeats the Rust archive, `initializers.o`, or the qrc object
- [x] 3.4 Add the Ninja link job pool (`CMAKE_JOB_POOLS link=4`, `CMAKE_JOB_POOL_LINK`) before the first target is created; verify `build.ninja` references the pool and the build succeeds
- [x] 3.5 Configure packaging (`-DBUILD_TESTING=OFF -DCMAKE_BUILD_TYPE=Release`), install it into a temporary prefix, and run the installed `pictura --headless --self-test`; verify the shell is static and the installed binary needs no separate shell library (check with `ldd` on Linux)
- [x] 3.6 Update the build shape, linker, and install guidance in `DEVELOPING.md` and `AGENTS.md`; verify `cmake --install` is documented as a static-configuration operation
- [x] 3.7 Re-measure the clean build, the one-shell-source rebuild, and disk usage; verify the after numbers are recorded and the link pool is kept only if it helps

## 4. Compile settings and moc

- [x] 4.1 Compile `command_tree.cpp` and `filter_commands.cpp` at `-O1` only for GCC/Clang/AppleClang in optimized development configurations; verify the compile command carries `-O1` and `tst_command_tree`, `tst_filter_menu`, and `tst_filter_gallery` pass
- [x] 4.2 Compile Qt Test translation units at `-g1` only for GCC/Clang/AppleClang; verify the compile command carries `-g1`, the flag is absent for MSVC, and sccache still reports hits on a rebuild
- [x] 4.3 Append `#include "moc_<header>.cpp"` to each matching QObject `.cpp` and to `acr/acr_basic_panel.cpp` for `acr/acr_panels.h`; verify all 110 Q_OBJECT headers are covered, `mocs_compilation.cpp` contains no moc include, and existing suites pass
- [x] 4.4 Record the moc-include convention in `AGENTS.md`; verify a new-file reader can follow it
- [x] 4.5 Re-measure the header-edit rebuild (a broad header such as `frame.h` still recompiles its consumers; `mocs_compilation` must not) and record whether `command_tree.cpp` still needs splitting; if `-O1` did not pay, open the split as a follow-up rather than expanding this change

## 5. Rust build and QMAKE

- [x] 5.1 Measure the proposed `app` profile against the standard release profile on a cold build; the Rust edge took 249 s with `app` and 182 s with `--release`, so keep the release profile and remove the custom profile
- [x] 5.2 Add the exact `[profile.test.package."*"] debug = false` setting; verify the self-test, nextest, and Qt suites pass, and record the link-time delta; drop the setting if it does not pay
- [x] 5.3 Diagnose the `pictura_app` rerun with `CARGO_LOG=cargo::core::compiler::fingerprint=info`, then add `scripts/qmake-env.sh` to read `QMAKE_EXECUTABLE` from `build/CMakeCache.txt` (falling back to `qmake6` then `qmake` from `PATH`), sourced by `verify-fast.sh` and `test-report.sh`; verify alternating `verify-full` and `verify-fast` no longer reruns the cxx-qt build script
- [x] 5.4 Verify the existing `RUSTC_WRAPPER=sccache` entries in `.env.example` and `DEVELOPING.md` are accurate and the example file stays a no-op as shipped; make no edit if they are already correct
- [x] 5.5 Add `CTEST_PARALLEL_LEVEL` and the FetchContent cache path to `.env.example` if they are user-facing; verify the file still enables nothing as shipped
- [x] 5.6 Record the Rust build measurement (release 182 s vs `app` 249 s) and confirm the release profile is kept

## 6. Integration and acceptance

- [x] 6.1 Run `openspec validate --all --strict` and `openspec archive speed-up-cmake-and-verify`; verify only `archive/` remains under `openspec/changes/` and `check-openspec-archived.sh` passes
- [x] 6.2 Run `verify-full.sh` after the archive and verify it is green; verify the packaging configure is static and no warning comes from project code
- [x] 6.3 Put the before/after table (clean build, one-shell-source rebuild, header edit, `verify-fast.sh`, `verify-full.sh`) in the PR body; verify every row has a number for both sides
