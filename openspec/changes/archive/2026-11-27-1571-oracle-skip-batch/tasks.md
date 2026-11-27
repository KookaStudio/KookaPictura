# Tasks

## 1. Tests first

- [x] 1.1 Add typed-skip, capability-family, Python/Node batch-order, and Python/Node worker-status tests; verify each test names its planned guarantee and checks worker PID reuse.
- [x] 1.2 Add the present-wrong-answer integration test; verify its exact `should_panic(expected = "oracle mismatch")` contract can only pass on the comparison assertion.
- [x] 1.3 Extend the reporter self-check for binary-qualified skip conversion, reason round-tripping, malformed, duplicate, unmatched, ambiguous, and failure-preserving tokens; verify each fixture reaches `build_layers` and `exit_code`.

## 2. Shared oracle seam and transport

- [x] 2.1 Implement `pictura-testkit::oracle::require`, typed skips, caller-aware reporting, and the builder-compatible command API; verify the new unit tests cover the public behavior.
- [x] 2.2 Add framed Python and Node workers with per-request output capture and cached imports; verify the order and status tests exercise both framing and worker reuse.
- [x] 2.3 Wire `PICTURA_ORACLE_SKIPS` through `scripts/test-report.sh` and the CI nextest job, and merge tokens into nextest results in `scripts/report_tests.py`; verify the reporter self-check fixtures cover counts, names, and errors.

## 3. Oracle migration

- [x] 3.1 Replace targeted availability helpers and early returns across codec, render, color, adjust, filters, ops, and select; verify no targeted helper or ad hoc tool skip remains and all assertions are unchanged.
- [x] 3.2 Route Python and Node oracle commands through the batch command API; verify command arguments, cwd, status, stdout, and stderr remain represented at every call site.
- [x] 3.3 Review the complete diff for batch correlation, skip identity, comparison semantics, and file-size limits; verify findings are fixed without new allowlist entries.

## 4. Integration evidence

- [ ] 4.1 Run formatting, lint, type checks, tests, OpenSpec validation, and repository gates when the no-check restriction is lifted; verify all planned coverage executes.
- [x] 4.2 Record any remaining limitations and keep the change uncommitted and unarchived; verify `git status` shows only intended source and OpenSpec files.
