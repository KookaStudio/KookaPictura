# Test Plan

This change is build and verification configuration. Most scenarios are proven
by inspecting generated build files, reading the Cargo/CMake invocation, or
timing a run, so their Coverage cell is a manual check with the exact command.
The behavior-preservation rows name the existing suite that must stay green.
Manual rows are the reviewer checklist in the PR.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `bp_mold` | Configure on an ELF host with mold installed; run `ninja -t commands pictura` and read the link command | The link command carries `-fuse-ld=mold` | manual: run `ninja -t commands pictura` in the build tree and search the link line |
| `bp_mold_fallback` | Configure in a fresh directory with `mold` hidden from `PATH`; link `pictura` | Configuration succeeds; the link uses lld when usable, otherwise the compiler default | manual: use a masked `PATH` and a fresh `mktemp -d` build directory |
| `bp_mold_override` | Configure with an explicit linker selection while mold is installed | The explicit selection wins | manual: read `CMakeCache.txt` and the link line |
| `bp_mold_platform` | Inspect the macOS and Windows packaging configure commands | No `-fuse-ld` flag is added; the platform linker is used | manual: read `.github/actions/macos-bundle/action.yml` and `windows-bundle/action.yml` |
| `bp_cheap_tu` | Read `compile_commands.json` for `command_tree.cpp` and `filter_commands.cpp` in an optimized development build | Both carry `-O1`; a Debug build does not | manual: inspect the two entries with GCC, Clang, and AppleClang; run `tst_command_tree`, `tst_filter_menu`, and `tst_filter_gallery` |
| `bp_test_debug` | Read `compile_commands.json` for a `tst_*.cpp`; check MSVC separately | GCC/Clang/AppleClang carry `-g1`; MSVC is unchanged | manual: inspect the `tst_smoke.cpp` entry and the MSVC configure |
| `bp_link_pool` | Inspect `build.ninja` | Link edges reference a pool of depth 4 | manual: search `build.ninja` for `pool link` |
| `bp_moc_includes` | Enumerate all 110 `Q_OBJECT` headers and their include sites | Each matching `.cpp` includes its `moc_<header>.cpp`; `acr/acr_panels.h` is included from `acr_basic_panel.cpp`; `mocs_compilation.cpp` has no moc include | manual: a Python enumeration over `crates/pictura-app/cpp` plus `grep` for `moc_` in the generated file |
| `bp_moc_incremental` | Touch one widely included QObject header (for example `frame.h`); rebuild | The moc-owning translation unit recompiles; `mocs_compilation.cpp` does not | manual: read the Ninja rebuild list; expect the header's consumers too, but no central moc unit |
| `bp_qmake_stable` | Run `verify-full.sh`, then `verify-fast.sh` with `CARGO_LOG=cargo::core::compiler::fingerprint=info` | The cxx-qt build script does not rerun | manual: assert no `pictura_app` rebuild line caused by `QMAKE` |
| `bp_qmake_change` | Set `QMAKE` to a different qmake, then build | The cxx-qt build script reruns | manual: `QMAKE=/path/to/other/qmake cargo build -p pictura_app` |
| `lbc_rebuild_hits` | Run `scripts/check-local-build-caching.sh --rebuild --dir build` on a warm cache | The hit counter increases | manual: the script prints `(before -> after)` and exits 0 |
| `lbc_probe_only` | Count objects and executable `tst_*` files under `build/`; run `--rebuild --dir build`; count again | Only the probe object was removed; test outputs remain | manual: a Python count of `build/**/*.o` and executable `build/**/tst_*` before and after |
| `lbc_fetchcontent_shared` | Run the default-mode caching check; inspect the shared FetchContent base | cxx-qt-cmake and Corrosion are fetched once for all three configures | manual: list `-src` directories under `${XDG_CACHE_HOME:-$HOME/.cache}/kooka-pictura/fetchcontent` |
| `lbc_fetchcontent_reuse` | Run the check twice and compare the base directory and elapsed time | The second run does not clone again | manual: compare `-src` mtimes under the shared base and the elapsed time |
| The executable keeps only main and the harness | Inspect the CMake targets | `pictura` holds main and self-test sources; the shell holds the rest | manual: inspect `CMakeLists.txt` and the generated target sources |
| A test executable inherits the cxx-qt surface | Run a suite that includes the generated headers | It links and runs | `tst_smoke::frameOpensOffscreen` |
| The self-test behavior is unchanged | Run the binary with and without a fixture | Same tokens and exit code | manual: `./build/pictura --headless --self-test` and with `two_layers.psd` |
| `qth_shell_shared` | Build with `BUILD_TESTING=ON`; read the `tst_smoke` link line; run the suite | The shell is a `.so`; the test links the shell and `Qt6::Test` without the Rust archive, `initializers.o`, or the qrc object; the suite passes | manual: inspect the link line with `ninja -t commands tst_smoke` and run `ctest` |
| `qth_shell_static_release` | Configure with `BUILD_TESTING=OFF -DCMAKE_BUILD_TYPE=Release`; install to a temp prefix; run the installed binary | The shell is static, the installed binary needs no shell library, and the self-test passes | manual: `cmake --install` into `mktemp -d`, then `ldd` and `--headless --self-test` |
| `qth_shell_windows` | Inspect the Windows configure path | The shell target is STATIC | manual: review the `NOT WIN32` condition; Windows development tests are not built in CI |
| Every layer is summarised | Run `scripts/test-report.sh never` | Per-suite lines, subtotals, TOTAL | manual: run the script and compare the layer list |
| A failure is listed and fails the run | Temporarily fail one Qt case in a scratch copy; run the report | FAILURES names it and the script exits non-zero | manual: scratch run, then revert |
| Colour output is selectable | Run the reporter with `--color never` and `--color always` | No escapes, then escapes | manual: `python3 scripts/report_tests.py --junit ... --color never` |
| The Qt Test layer is folded in | Run `test-report.sh` with built tests | `tst_*` JUnit files are parsed and counted | manual: run the script and read the Qt Test subtotal |
| `tr_qt_parallel` | Run `test-report.sh` at `-j 2`, `-j 4`, and `-j 6`, twice each | Suites run concurrently, each writes JUnit, and a failure still fails the run | manual: time each run and record peak memory |
| `tr_stale_junit` | Leave a JUnit file for a suite that no longer exists; run the report | The stale file is removed and does not appear | manual: create a fake XML in `build/qt-test-results/`, run, and check the report |
| Push or pull request | Open the PR | The expected jobs run | manual: CI run on the PR |
| A feature-branch push is covered once | Push twice to the branch | One run per pull-request synchronize | manual: CI run list |
| Tests run under nextest | Inspect `ci.yml` | It runs `cargo nextest run --workspace` | manual: read the workflow |
| Lint and test run in parallel | Open the PR | `lint` and `test` jobs overlap | manual: CI run timeline |
| The local fast gate uses nextest | Run `verify-fast.sh` | It runs nextest then doctests | manual: read the script output |
| The JUnit artifact is uploaded | Open a CI run | `nextest-junit` artifact exists | manual: CI artifacts |
| The Qt app self-tests headlessly | Open a CI run | Both self-test steps pass | manual: CI run log |
| External oracles are exercised | Open a CI run | The oracles job runs with the external tools | manual: CI oracle log |
| Compiled artifacts are reused | Run CI twice | The second run restores caches | manual: CI cache log |
| A runaway job is bounded | Inspect the workflows | Every job has `timeout-minutes` | manual: read the workflows |
| `vh_ci_ctest_parallel` | Open the CI headless job log; the task echoes the level | CTest runs with the bounded level and all JUnit files exist | manual: CI run log |
| `vh_ci_mold` | Open the CI headless configure log | mold is installed and the link uses it | manual: CI run log |
| `vh_fast_gate_preserves` | Count test objects and binaries, run `verify-full.sh`, count again | Counts are unchanged | manual: a Python count before and after |
| `vh_cache_probe` | Run the full gate on a warm cache | Hits increase after the probe | manual: read the probe output |
| `vh_cache_probe_fails` | Remove the launcher from a scratch build and run the probe | The probe fails | manual: scratch build with no launcher |
| `vh_parallel_checks` | Run `verify-fast.sh` | All checks run and the gate is green | manual: read the script output |
| `vh_parallel_failure` | Make one check fail in a scratch copy; run `verify-fast.sh` | The gate fails after the others finish and prints that log | manual: scratch copy, then revert |
