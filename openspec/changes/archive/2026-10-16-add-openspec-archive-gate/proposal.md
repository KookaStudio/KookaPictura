# Proposal

## Why

`openspec/changes/` has accumulated the whole shipped backlog: over a hundred
changes are complete but unarchived, so their deltas were never merged into
`openspec/specs/` and the capability corpus is stale. Nothing forces timely
archiving, so the pile grows with every feature. Issue #487.

## What Changes

- Archive the entire completed backlog so `openspec/specs/` reflects the shipped
  state, and give every newly created capability a real `## Purpose`.
- Add `scripts/check-openspec-archived.sh` — a check that fails when any
  directory exists under `openspec/changes/` other than `archive/` — with a
  self-test.
- Wire the check into the guards CI workflow and into both paths of
  `scripts/verify-fast.sh`.
- Document the zero-open policy: a change is authored, implemented, and archived
  within the pull request that ships it.

Not breaking.

## Capabilities

### New Capabilities
- `meta/openspec-archive`: The zero-open archive policy and the guard that
  enforces it — no non-archived change survives a pull request, checked by
  `scripts/check-openspec-archived.sh` in the guards workflow and `verify-fast.sh`,
  and documented in `DEVELOPING.md` and `AGENTS.md`.

### Modified Capabilities
- `verification/verification-harness`: The guards workflow and the
  `verify-fast.sh` full gate gain `scripts/check-openspec-archived.sh` alongside
  the existing script checks.

## Impact

- New files: `scripts/check-openspec-archived.sh`.
- Edited: `.github/workflows/guards.yml` (one step), `scripts/verify-fast.sh`
  (both the docs-only path and the full gate), `DEVELOPING.md` and `AGENTS.md`
  (policy text — root files, no `TASK-ALLOWS-DOCS`).
- Non-code: every completed change moves under `openspec/changes/archive/` and
  its deltas merge into `openspec/specs/` (~100 new main specs, each given a real
  Purpose).
- No Rust/C++ code, no new dependencies.
- The release PR is covered because `RELEASE_PLEASE_TOKEN` is set, so the guards
  workflow fires on it as on any pull request.
