# Tasks

## 1. Test coverage

- [x] 1.1 Write `test-plan.md` with one row for every scenario in the three spec deltas.
- [x] 1.2 Add `scripts/classify-ci-paths.py --self-check` covering C++-only, Rust, mixed, docs-only, render, MCP/script, workflow, empty, malformed, and run-all fallback inputs. Execution is deferred under the issue constraints.
- [x] 1.3 Extend `scripts/check-view-oracle.py --self-check` with mixed CMake logs and malformed JSON-looking lines. Execution is deferred under the issue constraints.

## 2. Reuse the Corrosion build report

- [x] 2.1 Add an opt-in CMake flag that appends `--message-format=json` to the existing Corrosion Cargo command, and confirm the default remains off.
- [x] 2.2 Make the oracle checker extract Cargo JSON records from a mixed build log while retaining the unique successful artifact, profile, feature, and file checks.
- [x] 2.3 Enable the flag in `qt-headless`, capture the normal build log, run the oracle audit after the build, and remove both oracle-only Cargo builds and their report upload from `test`.

## 3. Classify paths and fail closed

- [x] 3.1 Add the standard-library classifier CLI and its self-check, including JSON file-list parsing and run-all output publication before a non-zero classifier failure.
- [x] 3.2 Replace `cpp_only` with `rust`, `cpp`, `render`, and `docs` outputs, make the first paths-filter attempt continue-on-error, retry it once on a failed outcome, and confirm the classifier reads the successful JSON file list.
- [x] 3.3 Gate `lint` and `test` on `rust`, `qt-headless` on `cpp`, and both render jobs on `render`, using single-line `always()` conditions that skip only on exact `false`.

## 4. Preserve settled coverage

- [x] 4.1 Remove the Windows default library and backend CPU test commands, and confirm both Clippy steps and the DX12 WARP/FXC smoke remain.
- [x] 4.2 Confirm `lint` and `test` remain separate, `cargo nextest run -p pictura-render` remains in `test`, and packaging workflows remain Release-PR-gated.

## 5. Review and handoff

- [x] 5.1 Run the proposal review and resolve every legitimate finding in the change artifacts.
- [x] 5.2 Review the actual diff for fail-closed behavior, exact job gating, artifact reuse, and settled constraints, then resolve every legitimate finding.
- [x] 5.3 Leave the worktree review-clean, unverified, uncommitted, unarchived, and without a pull request.
