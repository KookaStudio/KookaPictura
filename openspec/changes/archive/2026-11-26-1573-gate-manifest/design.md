# Design

## Context

See `proposal.md` for the drift this change removes. The current shell runner
already owns changed-path detection, CPU budgeting, build locking, sequential
Cargo steps, and parallel script checks. Guards CI runs the same script family
as separate steps. Several checks are composed inside `guard.sh`, and the
build-lock check is already in guards CI despite an older spec sentence saying
otherwise for its introducing change.

The manifest must preserve those boundaries. It must not move Cargo commands
into the parallel batch, invent a second scheduler for #1599, or require a TOML
package. Python 3.11's standard `tomllib` is already used by repository checks.

## Goals / Non-Goals

**Goals:**

- One data file selects commands for `verify-fast.sh` and guards CI.
- Existing commands, arguments, self-tests, scoped scans, and failure identity
  stay intact.
- The drift check proves both tree coverage and placement invariants.
- A new standalone check needs one manifest row, not runner edits.

**Non-Goals:**

- Replacing the separate lint, test, GPU, Windows, or Qt-headless workflows.
- Changing any check's implementation or test threshold.
- Generating the manifest from the runners. That would preserve the drift.
- Adding a dependency or a general-purpose workflow engine.

## Decisions

### Manifest shape

Use `[[check]]` tables in `scripts/checks.toml`. Required fields are `id`,
`scope`, `cost`, `phase`, `command`, `local_fast`, `local_full`, and
`ci_guards`. `docs_only` and `ci_build` are explicit optional placements. A
`scripts` array names every script behind a composed command, so the four
file-local checks and doc-claim checks remain represented without duplicate
execution.

Scopes are `guards`, `build`, and `manual`. Build entries such as fmt, clippy,
tests, and build-tree checks point at their existing CI build-job coverage.
Manual tooling has no automatic placement. Cost is `cheap`, `moderate`, or
`expensive`; it documents review decisions but does not silently change
placement.

The manifest is sorted by ID. The drift check compares discovered
`scripts/check-*.py` and `scripts/check-*.sh` paths with the union of all
`script` and `scripts` paths, in the style of
`bridge_sources_match_the_tree`. It also rejects stale paths, duplicate IDs,
invalid fields, missing required CI IDs, guards entries absent from local full,
and local entries without guards/build-CI placement.

### One runner, two execution styles

`scripts/gate_manifest.py` loads and validates the TOML with `tomllib`, selects
one placement and phase, and executes commands from the repository root. Local
parallel runs use a bounded thread pool and preserve manifest-order reporting.
Each subprocess gets `GATE_PLACEMENT`, `GATE_CHANGED_FILE`, and the inherited CPU
budget. Guards CI runs with one worker and stops at the first failure, matching
the old workflow's step behavior.

Commands remain shell strings because the existing checks already use shell
composition, pipelines, and self-test chaining. The manifest is repository
data, not untrusted input, and both runners execute it with the same privileges
as the commands they replace.

A generated GitHub matrix was considered. It would expose one CI step per
check, but it would also require JSON job outputs and a second scheduling
layer. A single manifest-driven step keeps the mandatory set in one place and
preserves serial failure behavior with less machinery.

### Preserve the shell runner's boundaries

`verify-fast.sh` keeps changed-path detection, the docs-only branch, CPU-budget
reexec, build-lock handling, and cleanup. It asks the runner for sequential
commands first, then parallel commands. The sequential phase contains fmt,
clippy, build-tree, and tests in their current order. The parallel phase
contains the existing independent checks.

The docs-only branch asks for `docs-only` commands only. This preserves the
existing subset while putting its membership in the manifest.

`verify-full.sh` exports `PICTURA_FULL_GATES=1` before its nested fast run. The
cache check keeps its current default local-fast mode, uses `--full` for CI and
full-local mode, and keeps its separate `--rebuild` proof in `verify-full.sh`.
The build-tree check keeps `--warn` locally and strict mode in full-local mode.
A fresh CI Qt build supplies the build-job staleness guarantee.

### Mandatory set and extension

The drift check holds a small set of existing mandatory CI IDs as a ratchet.
New IDs need no test edit. Every `ci_guards = true` row must also have
`local_full = true`. A local row must have `ci_guards = true` or name one of the
fixed existing build-CI equivalents. A new script check cannot claim
`ci_build = true` to escape guards CI. Manual rows have no automatic placement.

This covers #1571 and #1599 without naming them in code. #1571 can keep its
reporting changes in the existing test command. #1599 can add its fixed-wait
check as one normal row, or compose it in an existing guard row, without a
special case in either runner.

## Risks / Trade-offs

- A malformed command can fail only at execution time → The drift check validates
  structure and paths before either runner starts a batch.
- One CI step gives less granular check-run reporting → Each command prints its
  manifest ID and the workflow names the first failing ID in its output.
- Parallel #1571 or #1599 branches may edit the same runners → Rebase before
  implementation review and reconcile their new checks as ordinary rows.
- The unarchived change intentionally fails the archive guard in a PR → Archive
  is a later integration step; this task leaves the change open and unverified.
- Existing specs contain stale placement wording → This delta updates the
  docs-only and local-runner requirements. The build-lock requirement's
  historical "for this change" sentence does not govern this change.

## Migration Plan

1. Add the manifest, runner, and drift check without changing check scripts.
2. Point both runners at the manifest and preserve phase and environment
   boundaries.
3. Run the drift self-test and both runner selection paths after the no-check
   implementation restriction is lifted.
4. Rebase against #1571/#1599 if they land first and add their checks as normal
   rows.
5. Archive the OpenSpec change only when the shipping PR is ready.

Rollback is a single revert: the previous shell and workflow command lists are
self-contained and need no data migration.
