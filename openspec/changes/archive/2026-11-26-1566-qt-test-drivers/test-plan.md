# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Configure builds the suites | Configure the test layer, inspect the completeness guard and included-source list, and compare generated driver targets and sources with the CMake registry. | Every standalone suite appears once, every source belongs to one subsystem driver, missing or duplicate registry entries fail configuration, included implementation sources are excluded, and no per-suite executable remains. | manual: target and source ownership are CMake configuration facts, not observable inside one Qt Test process. |
| CTest runs each suite offscreen | Run two entries that share a driver in parallel, each with state writes, and inspect their CTest commands. | Each entry starts one top-level driver process, selects a different suite, and uses a separate state home. | manual: process lifetime and parallel entry startup require CTest and OS observation. |
| `qth_driver_selection` | Register a probe under an explicit fake source path, dispatch to it with extra Qt Test arguments, and inspect the probe's received arguments, null terminator, and return value. | Only the probe runs, `argv[0]` is the suite name, later arguments are unchanged, `argv[argc]` is null, and the probe's return value propagates. | `tst_test_support::driverDispatchesSelectedSuite` |
| `qth_driver_invalid` | Call the dispatcher with no selector, an incomplete selector, a wrong-position selector, an empty name, a `.cpp` name, an unknown suite, and two registrations with the same name. | Each call returns nonzero and neither probe starts. | `tst_test_support::driverRejectsInvalidSelection` |
| CTest emits per-executable JUnit | Run one unsharded entry and one sharded entry, then list and parse their XML reports. | Files keep the `qt-test-results/<entry>.xml` names and contain the selected suite's cases. | manual: CTest owns the output path and Qt Test writes the XML. |
| Tests are optional | Configure a separate build with `BUILD_TESTING=OFF` and inspect its targets and CTest list. | No driver target or Qt Test registration exists. | manual: a second configuration is outside the Qt process; the `if(BUILD_TESTING)` branch gates `include(CTest)` and the whole driver block, so the off configuration emits neither. |
| CTest does not wrap nextest | Inspect every generated CTest command. | No command invokes `cargo nextest`. | manual: generated CTest commands are configuration output. |
| Suites do not share state | Run two state-writing suites in parallel through CTest and repeat with reversed order. | Neither suite observes the other's state, and results do not depend on order. | manual: cross-process isolation needs the CTest scheduler. |
| Each run starts clean | Run a state-writing suite twice and inspect its temporary state root and persisted values. | The second run starts from a fresh temporary directory. | manual: repeated external process startup is outside a single Qt Test case. |
| A pre-existing state home is restored | Set `XDG_STATE_HOME`, construct and destroy `ScopedStateHome`, then repeat with the variable unset. | The prior value is restored in the first case and the variable stays unset in the second. | `tst_smoke::stateHomeIsolationRestoresPriorValue` |
| The seeded suites run | Run the four named CTest entries and inspect their JUnit reports. | `tst_smoke`, `tst_command_tree`, `tst_layers_panel`, and `tst_edit_clipboard` all execute through their drivers. | manual: registration and cross-suite execution are CTest facts. |
| The migrated checks leave the self-test | Search the self-test sources and token registry for the seeded check names. | None of the named checks remains in the self-test. | manual: this is a source and registry invariant across two test layers. |
| `qth_critical_path` | Run the full Qt Test layer at its default parallel level and read CTest's critical-path output. | At least two suite names determine the critical path. | manual: CTest computes the critical path after the run. |
| `qth_suite_timeout` | Run the full Qt Test layer and inspect every entry result. | No entry is reported as timed out. | manual: timeout status is produced by CTest. |
| `qth_longest_entry` | Compare each entry's CTest duration with total wall time. | No entry exceeds one quarter of total wall time unless its shared-state reason is registered. | manual: the bound is a whole-run timing property. |
| `qth_long_suites_sharded` | Run all four shares of each long suite, list the functions in each JUnit report, and inspect test properties. | The union contains every function exactly once, Quick Mask shares use 300 seconds, and Document Navigate shares use 120 seconds. | `tst_test_support::testFunctionsExcludeLifecycleAndInvalidSlots`, `tst_test_support::shardAppendsItsShare`, `tst_test_support::emptyShareRunsNothing`; manual: run every real share and compare the JUnit function union and CTest properties. |

## Verification results (2026-11-26)

- Configure+build on this branch: cold 7m40.1s (766 CXX, 17 links, 16 AUTOMOC), against
  clean `origin/master` 8m35.0s (1024 CXX, 275 links, 274 AUTOMOC).
- Test-layer full rebuild: 38.5s branch vs 41.9s master; single-suite warm edit 1.8s vs 1.9s.
- `tst_quick_mask`: 45.3s as one master entry, 14.3-16.0s per share on four shares.
  `tst_document_navigate` four shares 4.6-6.7s. Function union per suite is complete;
  only `initTestCase`/`cleanupTestCase` repeat per share, as designed.
- CTest JSON diff: 312 -> 318 entries, two removed names, eight added, no other property,
  JUnit path, environment, or timeout change.
- Focused suites: `tst_test_support`, `tst_smoke`, `tst_command_tree`, `tst_layers_panel`,
  `tst_edit_clipboard`, `tst_structured_logging`, `tst_preferences_reset`, `tst_recovery`,
  `tst_control_metrics_off`, `tst_performance_preferences`, `tst_debug_toolset` all pass.
- Full layer: six failures (`tst_new_document`, `tst_crop_options`, `tst_move_feedback`,
  `tst_smart_guides`, `tst_present_cache`, `tst_type_tools`) reproduce on clean
  `origin/master`; one `tst_client_registration` SEGFAULT did not reproduce in
  24 isolated or 3 repeated parallel runs.
