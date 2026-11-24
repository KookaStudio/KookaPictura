# Proposal

## Why

Two avoidable costs run on every edit-loop iteration, and both are I/O the gate
does not need.

`scripts/check-file-size.sh` counted lines with one `wc -l < "$file"` fork per
file across 2,506 crate sources: measured 4.2-5.9 s idle on this machine, and the
issue reports up to 19 s on a loaded box. `find ... | xargs wc -l` does the same
count in about 20 ms.

Four checks then walked the whole tree regardless of what changed:

| check | files scanned | wall time, idle |
|---|---|---|
| `check-file-size.sh` | 2,506 | 4.2-5.9 s |
| `check-milestone-names.py` | 2,506 (29.8 MB) | 3.5 s |
| `check-msvc-string-limits.py` | 1,091 C++ | 3.6 s |
| `check-colour-literals.py` | 1,091 C++ | 0.07 s |

`scripts/verify-fast.sh` already computed the changed file set for its docs-only
fast path, and `scripts/check-test-plan.py` was the only check that used a
changed-set idea. The data existed; nothing else consumed it. Issue #1568.

## What Changes

- Add `scripts/changed_files.py`: the one shared changed-path helper (merge base
  against the default branch plus `git status --porcelain --untracked-files=all`),
  moved out of `verify-fast.sh`, with a library entry point, a CLI and a
  `--self-test`.
- Batch the line count in `check-file-size.sh` into one `wc -l` over the path
  list, and scope it to the changed set. The caps stay 1200 code / 1400 test and
  no allowlist entry is added.
- Give `check-milestone-names.py`, `check-msvc-string-limits.py` and
  `check-colour-literals.py` a `--changed FILE` mode; `--full` stays the default
  and is what CI runs.
- Every scoped check falls back to a whole-tree scan when the changed set is
  empty, unreadable, or touches something that can affect files the set does not
  name (a check's own allowlist). An empty set never means "nothing to check".
- `scripts/verify-fast.sh` computes the changed list once with the helper and
  passes it to the file-local checks.
- `scripts/guard.sh` gains `--changed FILE` and forwards it to the three
  file-local checks; without the flag its output and exit status are unchanged,
  and CI keeps calling it with no flag.
- Each check's `--self-test` (added where missing) proves the scoped path: a
  changed set that excludes a violating file passes in scoped mode, the same set
  including it fails, an empty set still fails, and `--full` always fails.

Not breaking: no cap, no allowlist entry, no CI invocation, and no `guard.sh`
behaviour changes. No new dependency.

## Capabilities

### New Capabilities

None. The change adds one capability-free helper script.

### Modified Capabilities

- `verification/verification-harness`: gains an ADDED requirement, "Fast-gate
  checks scan the changed set", covering the shared changed-set helper, the
  `--changed` / `--full` contract, the empty-set fallback, the batched line
  count, and the unchanged whole-tree behaviour of the guards workflow.

## Impact

- New: `scripts/changed_files.py`.
- Edited: `scripts/check-file-size.sh`, `scripts/check-milestone-names.py`,
  `scripts/check-msvc-string-limits.py`, `scripts/check-colour-literals.py`,
  `scripts/verify-fast.sh`, `scripts/guard.sh`.
- No Rust, no C++, no CMake, no new dependency, no spec-cap or allowlist change.
- CI (`.github/workflows/guards.yml`) is untouched and keeps the whole-tree scan,
  because it invokes `check-file-size.sh` and `guard.sh` with no arguments.

## Release valve

None needed: all four checks classify a file on its own path and text, so each is
soundly scoped. No check was left unscoped. The one whole-tree-by-nature input —
a check's allowlist — is handled by falling back to `--full` when that file is in
the changed set, rather than by pretending the set is complete. (Recorded here so
the reader can see the decision was made, not skipped.)
