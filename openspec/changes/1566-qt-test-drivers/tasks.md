# Tasks

## 1. Plan and dispatch coverage

- [x] 1.1 Review the proposal, design, specs, and test plan with a read-only reviewer and fix every legitimate finding. Verify that all four artifacts exist and every scenario has a plan row.
- [x] 1.2 Add `tst_test_support::driverDispatchesSelectedSuite` and `tst_test_support::driverRejectsInvalidSelection`. Verify forwarding, suite-name `argv[0]`, null termination, return codes, path normalization, missing or incomplete selectors, wrong selector position, empty names, `.cpp` names, unknown suites, and duplicate registrations without adding a CTest entry.

## 2. Registry and suite entry points

- [x] 2.1 Add the suite registry, entry macros, and `runSuiteDriver` to `qt_test_support.h/.cpp`. Use a function-local static registry behind an accessor. Verify that dispatch requires exactly one matching registration, duplicates fail before a suite starts, and no scheduler or child-process mechanism is introduced.
- [x] 2.2 Route every `QTEST_MAIN` and `QTEST_GUILESS_MAIN` through its entry macro and wrap the 16 remaining custom mains with a derived `picturaQtTest_<TestClass>` namespace and registration call. Verify that every suite body, assertion, and moc include remains unchanged.
- [x] 2.3 Keep the 12 existing sharded custom mains and the custom `shardedMain` bodies in `tst_raw_open` and `tst_theme_apply`. Route `tst_test_support` through the sharded macro. Verify that only `tst_quick_mask` and `tst_document_navigate` newly change from `QTEST_MAIN` to the sharded entry macro.
- [x] 2.4 Add `--suite <name>` to the self-spawn commands in structured logging, preferences reset, recovery, and control metrics off. Verify that all four call sites are test code, all four suites are unsharded, and child-mode flags or named-function arguments follow the selector unchanged.
- [x] 2.5 Audit every proposed driver source set for duplicate test-class names, global-scope helper collisions, startup hooks, and static initializers. Verify that no `Q_COREAPP_STARTUP_FUNCTION`, `Q_IMPORT_PLUGIN`, or `Q_GLOBAL_STATIC` appears in the suite sources and move any colliding helper into an anonymous namespace.

## 3. CMake subsystem drivers

- [x] 3.1 Replace the flat executable loop with the 14-subsystem registry and derived suite-to-driver map. Add a configure-time guard that compares standalone suite sources with the registry, excludes the explicit included-source list, and rejects missing, extra, or duplicate entries. Verify that all 270 fetched-master suites appear exactly once.
- [x] 3.2 Move the executable-path compile definitions, `pictura` dependencies, and self-test harness sources to the owning driver or source. Set `SKIP_PRECOMPILE_HEADERS` on the three sources with executable-path definitions. Verify that structured logging, UI localization, preferences store, update preferences, and self-test report keep their special setup.
- [x] 3.3 Keep one `add_test` per existing suite share, invoke the mapped driver with `--suite`, and preserve each entry's JUnit path, environment, and timeout. Verify by static review that consolidation alone leaves all 310 existing names unchanged; defer the generated JSON comparison to task 5.3.
- [x] 3.4 Keep `pictura_test_pch` and its `REUSE_FROM` wiring unchanged, with AUTOMOC off on the PCH and support libraries. Verify that grouped suite sources still include their own moc files and no source-specific main rename is used.

## 4. Long-suite sharding

- [x] 4.1 Add `tst_quick_mask` and `tst_document_navigate` to `PICTURA_QT_TEST_SHARDS`. Verify that both keep their existing reset bodies, each has four shares, and the existing `shardedMain` selection remains the only shard algorithm.
- [x] 4.2 Apply Quick Mask's 300-second timeout to every share and keep Document Navigate at 120 seconds. Verify by static review that the two old unsharded names are replaced by eight share names and the total rises from 310 to 316; defer generated-property verification to task 5.3.

## 5. Change review and handoff

- [x] 5.1 Review the actual diff with a read-only reviewer and fix every legitimate finding. Verify registry completeness, CTest and JUnit stability, shard union, timeout preservation, AUTOMOC reduction, source registration, global-helper isolation, and unchanged assertions.
- [x] 5.2 Audit scripts, CI, and contributor commands for direct per-suite executable references. Verify that `check-build-tree.sh` still checks `command[0]` against the suite named by the CTest entry, `check-local-build-caching.sh` still matches `tst_driver_*` objects, and no script or CI command invokes a removed `tst_*` suite binary directly.
- [x] 5.3 Diff `ctest --show-only=json-v1` output against a clean `origin/master` worktree. Verified: 312 baseline entries, 318 branch entries, exactly two removed names (`tst_quick_mask`, `tst_document_navigate`) and eight added (`*_0..3` each); every other entry keeps its name, JUnit path, environment, and timeout, and the only command change is `<driver> --suite <name>` in place of the per-suite binary. Recorded measurements: cold configure+build 8m35.0s → 7m40.1s, test-layer full rebuild 41.9s → 38.5s, warm single-suite edit 1.8s → 1.9s, AUTOMOC steps 274 → 16 (272 → 14 for test targets), link steps 275 → 17 (272 → 14), Quick Mask 45.3s in one entry → 14.3–16.0s per share. Full Qt Test layer: 6 baseline failures reproduce identically on clean `origin/master`; one `tst_client_registration` SEGFAULT occurred once in a parallel run and did not reproduce in 24 isolated or 3 repeated parallel runs, and its stack is the unchanged `client_registration_box.cpp` destructor path.
- [x] 5.4 Commit the change, archive it, and open the pull request.
