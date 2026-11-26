# Tasks

## 1. Proposal review

- [x] 1.1 Review the proposal and resolve legitimate findings before implementation; verify the final proposal, design, specs, tasks, and test-plan agree on placements and failure behavior.

## 2. Drift coverage first

- [x] 2.1 Add the manifest drift self-tests for unlisted scripts, stale paths, malformed and duplicate entries, mandatory CI placement, runner parity, local-only rejection, docs-only selection, and serial/parallel failure capture; verify the planted cases by source inspection and defer execution under the no-check constraint.
- [x] 2.2 Add selection tests proving a new row reaches local full and CI, full-local deferral is explicit, and existing worktree/archive/build-lock checks stay placed; verify the real manifest satisfies those assertions by source inspection and defer execution under the no-check constraint.

## 3. Manifest and runner

- [x] 3.1 Add `scripts/checks.toml` with one entry per existing check, composed guard scripts, scope, cost, phase, command, and placements; verify the drift self-test's discovered and listed sets match by source inspection.
- [x] 3.2 Add the standard-library manifest loader, validator, selector, and bounded local/serial CI executor; verify its command environment preserves changed-path scoping and full-local behavior by source inspection.
- [x] 3.3 Add `scripts/check-gate-manifest.py` with `--self-test` and tree validation; verify it is itself represented in the manifest and stays below the file-size cap by source inspection.

## 4. Wire both runners

- [x] 4.1 Replace the hand-written script-check list in `scripts/verify-fast.sh` with sequential and parallel manifest selections while retaining docs-only detection, CPU budgeting, locking, and cleanup; verify the diff contains no direct check-script dispatch outside the manifest runner.
- [x] 4.2 Replace the guards workflow's check steps with one manifest-driven CI selection while retaining checkout, OpenSpec installation, `GUARD_BASE`, and the test-plan fetch; verify every previous guards command appears in exactly one manifest entry.
- [x] 4.3 Export `PICTURA_FULL_GATES=1` from `scripts/verify-full.sh` before the nested fast gate; verify the cache and build-tree rows retain fast, full, and CI behavior by source inspection.

## 5. Review and deferred verification

- [x] 5.1 Review the complete diff for manifest completeness, mandatory CI coverage, runner parity, drift teeth, #1571/#1599 extension fit, and accidental local-only checks; verify all legitimate findings are fixed.
- [x] 5.2 Record in the final handoff that builds, tests, lint, format, OpenSpec validation, and repository check scripts were not run under the task constraint; name the drift self-test, runner selection paths, and normal aggregate gates as follow-ups.
