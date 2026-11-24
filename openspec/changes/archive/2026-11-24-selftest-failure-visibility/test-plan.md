# Test Plan

Every scenario in `specs/` is mapped by its `[tag]`, or by its title when
untagged. Automated coverage names a Qt Test case, a Python self-check function,
or the guard's tree scan. Runtime captures and CI-only behavior stay manual.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A passing check emits a suite and pass token | Point a fresh report at a temp sink, begin `x`, pass `x=1`, read the sink | `SUITE core x` precedes `PASS core x x=1` | `tst_selftest_report::passTokenCarriesSuiteAndName` |
| A skipped check emits a skip token with a reason | Begin `y`, skip `no adapter`, read the sink | `SKIP core y no adapter` and skipped count 1 | `tst_selftest_report::skipTokenCarriesReason` |
| A failing check emits a fail token and keeps its exit code | Begin `z`, fail with 42, read return and sink | `FAIL core z 42 boom` and return 42 | `tst_selftest_report::failTokenCarriesCode` |
| `tr_failure_code_nonzero` | Run built-binary failures with codes 256 and 512, read status and tokens | Status is non-zero, tokens keep 256 and 512 | manual: needs injected failures in a built binary; token behavior is covered by `tst_selftest_report::failTokenCarriesCode` |
| The summary aggregates the tokens | Emit one pass, skip, and fail, then finish | One summary reports 1/1/1 | `tst_selftest_report::summaryAggregatesTokens` |
| `tr_summary_on_failure` | Return a failure code from a real guarded helper | One summary includes the failure | `tst_selftest_report::summaryEmittedOnEarlyReturn` |
| `tr_state_home_summary` | Run a built binary with an unusable state directory | One `state_home_temp_dir` FAIL and one final summary | manual: needs a built binary and an unusable filesystem path |
| `tr_profile_summary` | Run a built binary with a profile that cannot open | One `profile_open` FAIL with code 555 and one final summary | `tst_selftest_report::failRunEmitsOneSummary`, plus manual: built-binary profile failure |
| `tr_fail_names_its_check` | Begin `previous`, pass, begin `own`, fail | FAIL names `own`; the tree scan rejects code/check mismatches | `tst_selftest_report::failTokenNamesItsOwnCheck`, `check-selftest-codes::check_sources` |
| `tr_suite_transition` | Begin `alpha_check` in `alpha`, then `beta_check` in `beta` | Two SUITE tokens surround their matching PASS tokens | `tst_selftest_report::suiteTransitionEmitsSuiteTokens` |
| The self-test source stays within its size budget | Run the file-size guard after source edits | Passes with `selftest.cpp` at or below 6439 | manual: the file-size gate is a shell script outside the plan's test-reference format |
| `ca_unique` | Run the guard self-test with duplicate and raw-code fixtures | Both are rejected | `check-selftest-codes::self_check` |
| `ca_retired` | Run the guard self-test with an active retired code | The retired allocation is rejected | `check-selftest-codes::self_check` |
| `ca_append` | Inspect the registry and run its consistency case | Code 555 exists and earlier values are unchanged | `tst_selftest_report::registryAndSuiteMetadataAreConsistent` |
| `ca_shared` | Run a fixture where two declared checks share one code, then use it from an undeclared check | Declared owners pass and the undeclared owner is rejected | `check-selftest-codes::self_check` |
| `dc_registered` | Run the guard over an unregistered check fixture and the tree | The fixture and any unknown source check are rejected | `check-selftest-codes::self_check`, `check-selftest-codes::check_sources` |
| `dc_suite` | Read suite metadata and drive a suite transition | Registered suite drives tokens; legacy checks resolve to `core` | `tst_selftest_report::registryAndSuiteMetadataAreConsistent`, `tst_selftest_report::suiteTransitionEmitsSuiteTokens` |
| `fx_cleanup` | Open a document inside `SelfTestRun`, change tool, destroy the run | Run-created document closes and previous tool returns | `tst_selftest_report::sharedRunFixtureCleansDocumentsAndRestoresTool` |
| `fx_paint_reuse` | Create a seeded `DocumentFixture`, then destroy it | Seed PNG is removed and its document closes | `tst_selftest_report::documentFixtureRemovesSeedAndClosesDocument` |
| `tr_truncated_selftest_fails` | Feed the reporter a capture with PASS but no SUMMARY | Capture is named truncated and exit is non-zero | `report_tests::run_self_check` |
| `tr_partial_summary` | Feed the reporter a SUMMARY with one count | Invalid summary is named and exit is non-zero | `report_tests::run_self_check` |
| `tr_trailing_tokens` | Feed the reporter a PASS after SUMMARY | Trailing token is named and exit is non-zero | `report_tests::run_self_check` |
| `tr_summary_counts` | Feed the reporter mismatched SUMMARY and token counts | Both counts are named and exit is non-zero | `report_tests::run_self_check` |
| `tr_summary_only_failure` | Feed the reporter only a failed SUMMARY | Capture's failed count is named and exit is non-zero | `report_tests::run_self_check` |
| `tr_merged_fail_not_hidden` | Merge a FAIL capture with a later PASS capture | Merged check stays failed and exit is non-zero | `report_tests::run_self_check` |
| `tr_per_capture_summary` | Merge two complete captures with different counts | Both capture summaries and the merged subtotal render | `report_tests::run_self_check` |
| `sc_code_nonzero` | Run guard fixtures with raw, duplicate, retired, and unknown codes | Every planted violation is reported | `check-selftest-codes::self_check` |
| `sc_fail_names_check` | Run a later-check code before its begin, then after its pass | First is rejected, second is accepted | `check-selftest-codes::self_check` |
| `sc_skip_truncation` | Run a skip-then-return fixture with a later begin | The truncation is reported | `check-selftest-codes::self_check` |
| `sc_code_table` | Run paired `names[i]` and `codes[i]` fixtures | Each code is checked against its matching name | `check-selftest-codes::self_check` |
| `sc_guard_self_test` | Run `check-selftest-codes.py --self-test` | Planted violations fail and clean fixtures pass | `check-selftest-codes::self_check` |
| Green token bytes stay stable | Capture both green runs before and after the change, compare machine tokens | Suite, name, pass detail, and order are byte-identical | manual: needs the pre-change binary and two built-binary captures |
| Failure identity stays stable | Compare ordered baseline/current code and message inventories | All 729 direct sites keep raw code and message; only owner names change | `check-selftest-codes::check_sources`, plus manual: review the source inventory |
| Docs-only change | Push only docs, specs, markdown, or Serena paths | Builds skip and guards still run the registry guard | manual: requires CI; workflow wiring is reviewed in the diff |
| Code change | Push a path outside the ignored set | Build jobs run as before | manual: requires CI; path filter is unchanged |
| Guards fail | Make a listed guard exit non-zero | Guards workflow fails | manual: requires CI; step failure propagation is reviewed in the diff |
| The fast full gate runs the worktree-hook check | Run `verify-fast.sh` | Worktree-hook check runs and can fail the gate | manual: gate invocation is the artifact |
| The guards workflow runs the archive check | Run the guards workflow | Archive check runs and fails on an unarchived change | manual: requires CI |
| `vh_docs_only_guard` | Run `verify-fast.sh` on a docs-only diff with a planted violation | Tests skip but the registry guard still fails the gate | manual: needs a docs-only tree and planted violation |

## Verification record

Verified on 2026-11-24 under `scripts/with-build-lock.sh` unless noted.

- `cargo fmt --all` passed.
- The wrapped CMake configure and full build passed after adding the generated
  view bridge include to `selftest_fixture.h` and the registry include to
  `tst_selftest_report.cpp`.
- `python3 scripts/check-selftest-codes.py` and its `--self-test` passed. The
  self-check runs 13 fixtures, including indexed name/code tables.
- `bash scripts/check-selftest-budget.sh` passed at 503/503.
- Offscreen `tst_selftest_report` passed 11 cases in 2.17 seconds.
- `python3 scripts/report_tests.py --self-check` passed.
- The branch headless self-test passed 467 checks with 0 failures and 0 skips in
  37.396 seconds. A clean `origin/master` worktree at `5361a2e16` passed the same
  467 checks in 37.482 seconds. Both exited 0 and emitted one final summary.
- Raw stderr differed only in the random `QTemporaryDir` socket path. After
  replacing that path with one placeholder, both 77,929-byte streams had SHA-256
  `2bfa2e4fd0500dcb865c65b2ead6f59c48d0fb8fcebc7ccf164ceecdd59f8b95` and
  were byte-identical. No speedup is claimed from these single runs.
- No Rust tests changed: report parsing lives in `scripts/report_tests.py` and
  is covered by its Python self-check.
- The final aggregate passed fmt, clippy, all 6,820 Rust tests, both self-test
  captures, and 4,444 Qt cases. It reported 14 Qt skips and one failure,
  `PresentCacheTest::frameBoundedPaintMatchesCpu(gpu)`, which reproduces on
  clean `origin/master`. Every post-test guard and strict OpenSpec validation
  passed except `check-milestone-names.py --self-test`, whose scoped-scan
  assertion also reproduces on clean `origin/master`.
- CI-only workflow scenarios remain pending. The built-binary failure injections
  for codes 256/512, unusable state home, and profile-open failure also remain
  manual.
