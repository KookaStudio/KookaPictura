# Test Plan

This change is build and verification configuration. Almost every scenario is
proven by inspecting the generated build files, watching the build-script
fingerprint, or timing a run, so their Coverage cell is a manual check with
the exact command. The rows that name a Qt Test case name the existing case
that must stay green while the build's shape changes. The manual rows are the
reviewer checklist in the PR.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `bp_qmake_stable` | Run `verify-full.sh`, then `verify-fast.sh`, with `CARGO_LOG=cargo::core::compiler::fingerprint=info` | Both cargo invocations observe the same `QMAKE`; the cxx-qt build script does not rerun because of it | manual: read the fingerprint log for a `QMAKE` change, and print the value each flow sees |
| `bp_qmake_change` | Build with `QMAKE` naming a different qmake than the previous run | The cxx-qt build script reruns | manual: `QMAKE=/usr/bin/qmake6 cargo build -p pictura_app` and read the fingerprint log |
| `bp_bridge_tracking` | Touch one shell `.cpp` that no bridge includes; record the build-script fingerprint directory's mtime; build | The build script did not rerun; the generated headers keep their timestamps | manual: `stat` the fingerprint file under `target/**/build/pictura_app-*/` and the exported header before and after the build |
| `bp_bridge_header_tracking` | Edit `crates/pictura-app/cpp/decode_image.h`; build | The build script reruns and the Rust static library is rebuilt | manual: the fingerprint log shows a `pictura_app` rebuild after the edit |
| `bp_header_stable_mtime` | Record the exported header's mtime; force a build-script rerun (touch a bridge `.rs` whose content does not change the generated interface); re-record | The header's mtime is unchanged | manual: `stat` the exported `cxxqt_object.cxxqt.h` before and after |
| `bp_header_changed_mtime` | Edit a bridge `.rs` so the generated interface changes; build | The exported header's mtime advances | manual: `stat` the exported header before and after the build |
| `bp_bridge_same_build` | Edit a bridge `.rs`; run one `cmake --build build`; count the recompiled objects; run it again | The C++ that includes the generated header is recompiled in the first build; the second build recompiles none | manual: read the Ninja rebuild list (`ninja -d explain`, or the `build/.ninja_log` timestamps) for both builds |
| `bp_bridge_no_change` | Build twice with no source change | No C++ translation unit is recompiled in either build | manual: read the Ninja rebuild list for both builds |
| `bp_heavy_tu_cheapened` | Measure the per-object CPU time of the heaviest shell translation units; decide on a split | The heaviest source is split along a concern seam as a pure move, or the measurement is recorded as not worth it | manual: read `build/.ninja_log`, sort by duration, and record the numbers in `design.md` |
| `bp_test_tu_cost` | Measure the total C++ CPU time of the Qt Test translation units and the per-suite moc cost | The per-suite moc cost is reduced, or the measurement is recorded as not worth changing | manual: read `build/.ninja_log` for the `tst_*` objects and the `mocs_compilation.cpp` objects |
| `tr_missing_suite_fails` | Remove one `tst_*` executable from the build tree; run `scripts/test-report.sh never` | The report names the suite as having produced no report and the script exits non-zero | manual: `rm build/crates/pictura-app/cpp/tests/tst_smoke` then run the script |
| `tr_crashed_suite_fails` | Make one suite exit before it writes its JUnit report; run the report | The report names that suite and the script exits non-zero | manual: a scratch edit to a suite, then revert it |
| `tr_layers_concurrent` | Run `scripts/test-report.sh never` with a built application and built suites; compare the wall time with the sum of the layers' individual durations | The self-test layer overlaps the Qt Test layer, and every layer still reports | manual: time the run and compare it with the per-layer durations the script prints |
| `vh_stale_tree` | Make a tracked source under the application crate newer than `build/pictura`; run the full gate | The gate names the source and fails | manual: `touch crates/pictura-app/cpp/frame.cpp` then run the gate |
| `vh_missing_suite_binary` | Remove one `tst_*` executable; run the full gate | The gate names the suite and fails | manual: remove the executable, then run the gate |
| `lbc_fetchcontent_shared` | Run the caching check in its default mode; inspect the shared FetchContent base | cxx-qt-cmake and Corrosion are fetched once for the whole run | manual: list the `-src` directories under `${XDG_CACHE_HOME:-$HOME/.cache}/kooka-pictura/fetchcontent` |
| `lbc_fetchcontent_reuse` | Run the check twice and compare the base directory and the elapsed time | The second run does not clone again | manual: compare the `-src` mtimes under the shared base and the elapsed time |
| `lbc_default_one_configure` | Run the caching check in its default mode and count the configures | The project is configured exactly once and the launcher rule is still asserted | manual: the script prints the configure count; assert it is 1 |
| `lbc_full_variant` | Run the caching check in its full variant | The default, explicit-override, and absent-launcher rules are all asserted | manual: the script prints three launcher results |
| `qth_critical_path` | Run `ctest --test-dir build -R '^tst_'` and read the reported critical path | The critical path names at least two suites | manual: read CTest's "Critical path" line |
| `qth_suite_timeout` | Run the whole CTest layer | No suite is reported as timed out, and the split suites still pass | `tst_workspaces::applyReGroupsPanel`, `tst_floating_workspaces::flyoutHostsWholeGroup`, `tst_import_pdf::genericPdfShowsDialog` |
