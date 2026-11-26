# Proposal

## Why

`scripts/verify-fast.sh` and `.github/workflows/guards.yml` maintain separate
check lists. They have drifted: icon provenance and Flatpak cargo-source checks
run locally but not in guards CI, so a local failure does not block a pull
request.

## What Changes

- Add `scripts/checks.toml` as the checked-in inventory of script checks, their
  scope, cost, execution phase, and local/docs-only/CI placement.
- Make `scripts/verify-fast.sh` and the guards workflow select commands from
  that manifest through one standard-library Python runner.
- Keep the current guards CI set mandatory and add the missing icon-provenance
  and Flatpak cargo-source checks. Shell seams stay mandatory.
- Keep format, clippy, and tests sequential. Run independent checks concurrently
  only in the local gate, bounded by `PICTURA_JOBS`.
- Let `verify-full.sh` opt into expensive full-mode checks with
  `PICTURA_FULL_GATES=1`; a check may move from local fast to local full only
  when it remains in CI and local full.
- Add a drift check that rejects unlisted `scripts/check-*` scripts, stale
  paths, duplicate IDs, missing mandatory CI entries, and local-only placement.
- Preserve each check's arguments, self-tests, changed-path scoping, and failure
  behavior. Add no dependency.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `verification/verification-harness`: define the shared gate manifest, runner
  parity, mandatory CI placement, drift detection, and explicit full-local
  opt-in. Update the docs-only and local-runner requirements to match the
  manifest-driven guard set.

## Impact

- `scripts/checks.toml`, a small manifest runner and drift check, both verification
  runners, and `verify-full.sh` change.
- Guards CI replaces its hand-written check steps with one manifest-driven step.
  The separate lint/test/Qt jobs and their commands stay unchanged.
- Existing checks keep their implementations and self-tests. No dependency or
  product behavior changes.
