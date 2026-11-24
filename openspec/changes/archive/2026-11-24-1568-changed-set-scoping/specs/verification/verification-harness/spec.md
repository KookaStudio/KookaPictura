# verification/verification-harness

## ADDED Requirements

### Requirement: Fast-gate checks scan the changed set

The repository SHALL provide one shared changed-path helper,
`scripts/changed_files.py`, that computes the changed set as the files that
differ from the merge base with the default branch (`master`, `main`,
`kooka/master`, `kooka/main`, or their `origin/` forms) plus the paths reported
by `git status --porcelain --untracked-files=all`. `scripts/verify-fast.sh` SHALL
use that helper for its docs-only detection and SHALL pass the resulting list to
the file-local checks it runs.

The file-local checks — `scripts/check-file-size.sh`,
`scripts/check-milestone-names.py`, `scripts/check-msvc-string-limits.py`, and
`scripts/check-colour-literals.py` — SHALL each accept a changed-path list and
SHALL scan only the files in that list, while `--full` SHALL scan the whole tree.
`--full` SHALL be the default and CI SHALL keep using it. A scoped scan SHALL
fall back to the whole-tree scan when the changed set is empty, when a changed
path cannot be represented in the list (a newline in a name), or when the change
touches a file that can affect a file the set does not name (a check's own
allowlist). A changed-path list that cannot be read SHALL be a usage error and
SHALL exit non-zero rather than fall back, so a mistyped path cannot silently
widen a run. `scripts/guard.sh` SHALL forward an opt-in changed-path list to the
file-local checks and SHALL otherwise behave as before, so its output and exit
status are unchanged for a whole-tree run.

#### Scenario: The changed set is computed once and shared [vh_changed_set_shared]

- **WHEN** `scripts/verify-fast.sh` starts and `scripts/changed_files.py --self-test` passes
- **THEN** the gate obtains the changed paths from the shared helper and hands the same list to every file-local check it runs

#### Scenario: A scoped scan inspects only the changed files [vh_changed_set_scoped]

- **WHEN** a file-local check runs with a changed-path list that names one clean file
- **THEN** it reports no violation in scoped mode, while a `--full` run over the same tree reports the violation in the unchanged file

#### Scenario: A changed violation still fails a scoped run [vh_changed_set_violation]

- **WHEN** a file-local check runs with a changed-path list that names a file holding a violation
- **THEN** the scoped run fails and names that file

#### Scenario: An empty changed set scans everything [vh_changed_set_empty]

- **WHEN** a file-local check runs with an empty changed-path list, or the list cannot be read
- **THEN** an empty list scans the whole tree and still fails on a violation, and an unreadable list exits as a usage error

#### Scenario: A check allowlist change widens the scan [vh_changed_set_allowlist]

- **WHEN** a file-local check's allowlist is itself in the changed set
- **THEN** the check scans the whole tree rather than the changed set

#### Scenario: The line count is batched [vh_file_size_batched]

- **WHEN** `scripts/check-file-size.sh` scans the tree
- **THEN** it counts every file's lines in one `wc -l` batch, applies the 1200-line code and 1400-line test caps and the allowlist ceilings from that batch, and prints the same violations and summary as before

#### Scenario: The guard stays whole-tree in CI [vh_guard_unchanged]

- **WHEN** `scripts/guard.sh` runs without a changed-path list, as the guards workflow invokes it
- **THEN** it scans the whole tree and its output and exit status are unchanged

#### Scenario: The self-tests prove the scoped path [vh_changed_set_self_test]

- **WHEN** any of the four checks runs with `--self-test`
- **THEN** it asserts that a changed set excluding a violating file passes in scoped mode, that the same set including it fails, that an empty set still fails, and that a `--full` run fails

### Requirement: The file-size check counts lines in one batch

`scripts/check-file-size.sh` SHALL count lines with one batched `wc -l` over its
path list rather than one `wc` invocation per file. It SHALL apply the 1200-line
code cap, the 1400-line test cap, and the allowlist ceilings from that batch, print
the same per-file violations and summary line, and keep the file-size allowlist's
lower-only ceiling. No entry SHALL be added to
`scripts/file-size-allowlist.txt` by this change.

#### Scenario: A batched count keeps the caps and the allowlist [vh_file_size_caps]

- **WHEN** `scripts/check-file-size.sh` scans a tree holding a small clean file, a 1250-line code file allowlisted at 1200, and a 1250-line file under `crates/tests/` allowlisted at 1300
- **THEN** it reports the code file growing past its allowlisted ceiling, passes the 1250-line test file under the 1400-line test budget, and passes the clean file

#### Scenario: The cap values do not move [vh_file_size_caps_unchanged]

- **WHEN** `scripts/check-file-size.sh` is inspected after the change
- **THEN** the code budget is 1200 and the test budget is 1400, and `scripts/file-size-allowlist.txt` has no new entry
