# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tr_layers_concurrent` | Run the real executor with fake `pictura` processes that record start/end events and emit separate token streams. | Both self-tests overlap each other, keep separate captures, and either failure reaches the report. | `test_report_execution::selftests_run_concurrently` |
| `qe_overlap_opt_in` | Run a staged fake build with `PICTURA_QT_SELFTEST_OVERLAP=1` and a deliberately longer build tail. | Both self-tests start before the tail ends; CTest starts after the tail and both self-tests end. | `test_report_execution::overlap_is_opt_in` |
| `qe_overlap_default` | Run the same staged fake build without the overlap variable. | Both self-tests end before the all-target build starts. | `test_report_execution::overlap_is_opt_in` |
| `qe_ci_shared` | Inspect the `qt-headless` job and run the fake CI adapter arguments. | One shared-script step owns configure, staged build, self-tests, CTest, and reporting. | `test_report_execution::ci_adapter_uses_shared_executor` |
| `qe_local_adapter` | Inspect `verify-full.sh` and `verify-fast.sh`. | The full gate forwards configure/build to `test-report.sh` and has no duplicate CMake build. | `test_report_execution::local_adapter_uses_shared_executor` |
| `qe_nextest_separate` | Run the default fake executor and inspect event order. | The fake nextest run finishes before configure and every Qt stage. | `test_report_execution::nextest_is_separate` |
| `qe_pictura_first` | Run a staged fake build and inspect event order. | The `pictura` target ends before either self-test starts. | `test_report_execution::pictura_links_before_selftests` |
| `qe_ctest_after_build` | Hold the fake all-target build open and inspect CTest events. | CTest starts only after the all-target build and both self-tests finish. | `test_report_execution::ctest_waits_for_build` |
| `qe_single_ninja` | Make the fake build writer reject a second concurrent owner and run overlapped mode. | No overlap violation is recorded and the run succeeds. | `test_report_execution::one_ninja_writer` |
| `qe_failure_identity` | Make bare and document self-tests exit 7 and 9 with distinct FAIL tokens. | The reporter receives both captures and the executor returns 7. | `test_report_execution::failure_identity_and_separate_captures` |
| `qe_build_failure` | Fail the fake `pictura` build, then the all-target build in a separate case. | Dependent stages do not start, available captures and missing reports are reported, and the build status wins. | `test_report_execution::build_failure_stops_later_stages` |
| `qe_signal_cleanup` | Start long-running fake children, signal the executor, and poll their PIDs. | Children are terminated and reaped, temporary captures are removed, and the executor exits non-zero. | `test_report_execution::signal_cleanup_reaps_children` |
| `qe_stale_cleanup` | Seed an old Qt JUnit file before a fake run that produces different reports. | The old file is absent before execution and absent from the report. | `test_report_execution::stale_junit_cleanup` |
| `qe_missing_report` | Register two fake suites and make one exit without JUnit. | The run fails and names the suite that produced no report. | `test_report_execution::missing_report_is_named` |
| `qe_timings` | Run a complete staged fake execution and inspect stdout. | Configure, `pictura`, build tail, both self-tests, CTest, and reporting durations are present. | `test_report_execution::stage_timings_are_recorded` |
| `qe_overlap_measurement` | Compare sequential and overlapped staged runs on four vCPUs after implementation. | Adapters enable overlap only after a net wall-time win without new flakes or contention regressions. | manual: requires the #1570 four-vCPU measurement; intentionally not run in this change |
| `qe_ci_tiff` | Inspect workflow step order around TIFF extraction and shared execution. | TIFF extraction and `LD_LIBRARY_PATH` setup precede the shared executor. | `test_report_execution::ci_adapter_uses_shared_executor` |
| `qe_ci_cpu` | Inspect the shared execution step's environment and run the fake CI adapter. | CI requests the full runner CPU budget and preserves `CTEST_PARALLEL_LEVEL` override behavior. | `test_report_execution::ci_adapter_uses_shared_executor` |
| `qe_ci_orchestration` | Inspect all `qt-headless` steps. | No separate self-test, CTest, or Qt reporter step duplicates the executor. | `test_report_execution::ci_adapter_uses_shared_executor` |
