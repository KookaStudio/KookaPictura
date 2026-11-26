# Tasks

## 1. Reporter regression coverage

- [x] 1.1 Extend `scripts/report_tests.py::run_self_check` without removing assertions. Assert that one complete stream keeps its SUMMARY total and unique entries, and that a two-shared-plus-`fresh_white`-plus-36-document fixture has the same 39 unique keys as the legacy bare/document merge. Verify by inspecting the new assertions; runtime execution is task 6.3.

## 2. Combined self-test mode

- [x] 2.1 Rename the loaded-document boolean to `documentMode` in `main.cpp`, `selftest.h`, and `selftest.cpp`. For self-test runs, create the scratch document before the run and defer the PSD open to `runSelfTest`. Verify by inspecting the diff for only the mode handoff, no ST macro, code, check-body, or return-path changes, and `selftest.cpp` remaining at 6439 lines.
- [x] 2.2 Start `documentMode` false, run the existing `fresh_white` block, open `psdPath` in-process, close the scratch document after a successful open, update the active view and by-value image, set the same flag from the open result, and then run the existing document block when true. Verify by inspecting the two conditionals and the open handoff.
- [ ] 2.3 Preserve direct bare-mode behavior and the existing failed-open fallback. Verify later by running both direct modes and comparing their check-name sets, SUMMARY counts, and representative failure identities.

## 3. One local capture

- [x] 3.1 Change `scripts/test-report.sh` to launch one combined self-test with `two_layers.psd`, pass one `--selftest` capture, and keep it concurrent with CTest while nextest remains sequential. Verify by inspecting the script; runtime observation is task 6.3.

## 4. Single-stream reporter handling

- [x] 4.1 Move the single-capture SUMMARY override into `merge_selftest_streams` and remove the duplicate override from `build_layers`. Preserve multi-stream unique totals and FAIL-wins behavior. Verify by inspecting the implementation and existing assertions; runtime execution is task 6.3.
- [x] 4.2 Confirm the existing multi-stream, per-capture SUMMARY, truncation, and FAIL-wins assertions remain unchanged. Verify by diffing `run_self_check`.

## 5. Linux headless CI

- [x] 5.1 Remove the bare self-test step from `.github/workflows/ci.yml` and keep one combined invocation with `two_layers.psd`. Verify by inspecting the workflow; CI runtime remains deferred.

## 6. Review and deferred verification

- [x] 6.1 Run a proposal review, using a nested reviewer when the task launcher is available, and fix all legitimate findings before implementation. Verify the 503-check arithmetic and agreement across the proposal, design, deltas, tasks, and test plan.
- [x] 6.2 Run a change review against the actual diff, using a nested reviewer when the task launcher is available. Verify unique-count preservation, mode conditionals, SUMMARY handling, failure identity, and the no-ST-macro/no-exit-code constraint.
- [ ] 6.3 In a later checked phase, run the reporter self-check, build the app, run bare and combined modes, and run the unified report. Verify one combined SUMMARY with 503 unique checks and unchanged failure identities.
- [ ] 6.4 In a later sequential phase, execute the overlap measurement plan and record the result before any nextest overlap change.
