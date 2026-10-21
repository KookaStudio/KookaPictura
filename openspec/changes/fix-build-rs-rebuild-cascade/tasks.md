# Tasks

Work the groups in order. Each group lands its own checks and documentation and
ends with a measurement. The measurement builds are the acceptance exception to
the single-compile rule: the issue requires before/after numbers. The full
verification gates can only run after the change is archived (group 8),
because `check-openspec-archived.sh` fails while the change directory is open.

## 1. Baseline and spike

- [x] 1.1 Write `test-plan.md` in this change folder with one row per scenario in every delta spec, and verify `python3 scripts/check-test-plan.py` reports no missing row
- [x] 1.2 Record the before numbers on this machine with command and duration for each row: a clean configure and build (wall, `build/.ninja_log` CPU-s, `du -sh build`), a rebuild after touching one shell `.cpp` (both the first and the second build), a rebuild after touching a non-bridge `.rs`, a rebuild after editing a bridge `.rs` (both builds), `ctest -R '^tst_'`, `verify-fast.sh`, and `verify-full.sh`; verify every row has a number
- [x] 1.3 Record the CTest critical path and the three slowest suites' individual durations; verify the critical path currently names one suite, so the "not bounded by its slowest suite" requirement has a before value
- [x] 1.4 Spike the QMAKE mechanism: add `QMAKE = { value = "qmake6", force = true }` to `.cargo/config.toml`, build `pictura_app` with `QMAKE` unset, then with `QMAKE=/usr/bin/qmake6`, then unset again, with `CARGO_LOG=cargo::core::compiler::fingerprint=info`; verify the third build does not recompile `cxx-qt-lib`, `cxx-qt` or `pictura_app`; record go/no-go in `design.md` and, if it is a no-go, keep the fallback there
- [x] 1.5 Verify the exported header tree's layout in a built tree (`$CXX_QT_EXPORT_DIR/crates/pictura_app/include`), so the write-if-changed snapshot has a known shape; record the file count and the total size

## 2. Build script change tracking

- [x] 2.1 In `crates/pictura-app/build.rs`, call `.crate_include_root(None)` and emit `cargo::rerun-if-changed` for the three hand-written bridge edges (`cpp/decode_image.{h,cpp}`, `cpp/render_text.{h,cpp}`, `cpp/encode_image.{h,cpp}`); verify the build script no longer reruns when a shell `.cpp` changes, and that editing `cpp/decode_image.h` still reruns it
- [x] 2.2 In `crates/pictura-app/build.rs`, snapshot the exported header tree before `CxxQtBuilder::build()` and restore the modification time of every file whose bytes are unchanged; verify the exported header's mtime is unchanged after a no-op rerun and advances after a rerun that changes the generated interface
- [x] 2.3 Verify the whole application still builds, that `pictura_app` links into the shell, and that the self-test and the Qt suites pass; record the after numbers for the leaf-`.cpp` and non-bridge-`.rs` rows

## 3. The bridge stamp

- [x] 3.1 In `CMakeLists.txt`, collect the bridge Rust sources into one list, add them to `CMAKE_CONFIGURE_DEPENDS`, and write a content hash of them to `${CMAKE_BINARY_DIR}/pictura_bridge.stamp` at configure time, rewriting the stamp only when the hash changes; verify a re-configure with no bridge change leaves the stamp's mtime alone
- [x] 3.2 In `CMakeLists.txt`, add the stamp to the shell sources and the Qt Test sources through `OBJECT_DEPENDS`; verify `build.ninja` carries the stamp as an input of the compile edges
- [x] 3.3 Edit a bridge `.rs` and build once; verify the C++ that includes the generated header is recompiled in that build and that a second build recompiles none of it; record the after numbers for the bridge rows
- [x] 3.4 Verify a leaf `.cpp` edit, a non-bridge `.rs` edit, and an unchanged rebuild still recompile nothing beyond what changed; record the numbers

## 4. QMAKE

- [x] 4.1 Act on the spike's outcome: if `config.toml`'s `[env]` reaches the fingerprint, keep `QMAKE = { value = "qmake6", force = true }` in `.cargo/config.toml`; otherwise make `CMakeLists.txt` honour `$ENV{QMAKE}` when it is set, keep the `find_program` fallback for the configure-time check, and document the one-time export in `DEVELOPING.md`; verify the mechanism the spike chose
- [x] 4.2 Delete `scripts/qmake-env.sh` and its two `source` lines in `scripts/verify-fast.sh` and `scripts/test-report.sh`; verify no script exports `QMAKE` any more and that alternating `verify-full.sh` and `verify-fast.sh` does not rebuild `pictura_app` because of `QMAKE`
- [x] 4.3 Verify that a genuinely different `QMAKE` still reruns the cxx-qt build script, and record the fingerprint-log evidence for both rows

## 5. Report and gate honesty

- [x] 5.1 In `scripts/report_tests.py`, accept the list of suites CTest registered and fail, naming each suite, when a registered suite produced no report; add a self-test case for it; verify `python3 scripts/report_tests.py --self-test` passes and that removing a `tst_*` executable makes the report name it and exit non-zero
- [x] 5.2 In `scripts/test-report.sh`, pass the registered suite list to the reporter and clear `build/qt-test-results/*.xml` before the CTest run as before; verify a stale JUnit file is still not counted and a missing suite now is
- [x] 5.3 In `scripts/test-report.sh`, run the two application self-test invocations concurrently with each other and with the CTest layer, each keeping its own capture; verify both captures reach the reporter, a failure in either still fails the report, and the wall time is below the sum of the layers' durations
- [x] 5.4 Add a stale-build-tree check to `scripts/verify-fast.sh` and `scripts/verify-full.sh`: fail when the application binary predates a tracked source under the application crate or when a registered suite has no executable, and name the file or suite; verify both cases fail the gate and that a current tree passes
- [x] 5.5 In `scripts/check-local-build-caching.sh`, make the default mode configure the project once and keep the other two launcher cases in a `--full` variant; verify the default mode configures exactly once and still asserts the sccache launcher, that the full variant asserts all three rules, and that the check no longer writes the repository-root `compile_commands.json` more than once
- [x] 5.6 Profile `scripts/check-build-lock.sh` and record where its time goes; if a fix is warranted, apply it and verify the check still passes and still proves every assertion

## 6. Clean build and heavy translation units

- [x] 6.1 Measure the clean build with the Rust edge on the critical path: the wall time is 58.5 s over 1225 edges, the cargo edge is 39 s of it and no shell translation unit runs inside that window; the 123 translation units that include no generated header cost 55 s of CPU; verify the numbers come from `build/.ninja_log`
- [x] 6.2 Measure the shell split before shipping it: it moves 55 s of CPU into the cargo edge's window and shortens the clean build by about 5 s (58.5 s to 54 s), but deciding which translation units include a generated header needs a transitive include-graph walk in CMake, and a translation unit on the wrong side compiles against a stale generated header -- the bug this change removes; record the numbers and the decision in `design.md` and drop the requirement from the delta
- [x] 6.3 Record the clean build's after numbers in `design.md`; the split is not shipped, so there is no overlap to verify
- [x] 6.4 Measure the heaviest shell translation units: `frame_keys.cpp` at 0.51 s and the heaviest Qt Test unit at 0.86 s; neither dominates a 58.5 s build, so neither is split; record the numbers in `design.md`
- [x] 6.5 Measure the Qt Test translation units: 86.4 s of CPU over 252 units, with the per-suite moc cost at 4.2 s over 129 units and the 138 test links at 0.1 s each; none of it is worth reducing; record the numbers in `design.md`
- [x] 6.6 Measure split debug information against `-g`: one shell translation unit compiles in 1.67 s with `-g` and 1.78 s with `-gsplit-dwarf` (the object shrinks from 458 KiB to 64 KiB), the links gain nothing and sccache does not cache the `.dwo` file; the Qt Test targets already use `-g1`; record the numbers in `design.md`

## 7. The Qt Test layer's critical path

- [x] 7.1 Measure each suite's individual duration from the CTest log and identify what dominates the three slowest: window and panel construction, PDF parsing, or event pumping; record the numbers
- [x] 7.2 Split the suites that bound the critical path into more registered entries, so the layer's critical path names at least two suites; verify the split suites still pass, that no suite is reported as timed out, and that the CTest wall time falls
- [x] 7.3 Verify `CTEST_PARALLEL_LEVEL` still overrides the level and that the default level is unchanged; record the CTest wall time and the critical path after the split

## 8. Integration and acceptance

- [x] 8.1 Update `DEVELOPING.md`, `AGENTS.md`, and `.serena/memories/tech_stack.md`: the build's change tracking and the bridge stamp, the single `QMAKE` value, the stale-tree check, the report's missing-suite rule, the concurrent layers, the caching check's single configure, and the correction of the `app` profile that #771 rejected; verify the documented commands match the scripts and that no document still describes the `app` profile
- [x] 8.2 Run `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`; verify both pass with no project warning
- [ ] 8.3 Run `openspec validate --all --strict` and `openspec archive fix-build-rs-rebuild-cascade`; verify only `archive/` remains under `openspec/changes/` and `check-openspec-archived.sh` passes
- [ ] 8.4 Run `verify-full.sh` after the archive and verify it is green, that the packaging configure is still static, and that the self-test and every Qt suite pass
- [ ] 8.5 Put the before/after table (clean build, leaf `.cpp` first and second build, non-bridge `.rs`, bridge `.rs` first and second build, clippy without `QMAKE`, `ctest -R '^tst_'`, `verify-fast.sh`, `verify-full.sh`) in the PR body, and list the manual rows of `test-plan.md` as the reviewer checklist; verify every row has a number for both sides
