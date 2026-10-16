# Proposal

## Why

The **windows** release job fails on every Release PR: compiling
`crates/pictura-app/cpp/theme.cpp` aborts with
`error C2026: string too big, trailing characters truncated`. The stylesheet is a
single `QStringLiteral(R"(...)")` literal that has grown to ~19,670 bytes, and
MSVC caps a string literal at 16,380 single-byte characters — before adjacent
literals are concatenated. GCC/Clang accept the longer literal, so the breakage
only surfaces on Windows, and because `windows.yml` only runs the build on the
release-please PR (`windows.yml:41`), feature CI never caught it (#474).

The literal first crossed the cap in `00fc89d6` (#460) and grew further in
`d6f8fed3` (#470); the failure surfaced once a later merge to `master` refreshed
the Release PR's CI. The immediate fix is to deliver the stylesheet as several
literals. The durable fix is to stop this class of breakage from reaching the
Release PR at all.

## What Changes

- Split the `theme.cpp` stylesheet literal into a small number of
  `QStringLiteral` pieces joined with `+`, each comfortably under MSVC's
  per-literal cap. The assembled stylesheet text is byte-identical.
- Add `scripts/check-msvc-string-limits.py`, a guard that fails when any single
  C++ string-literal token under `crates/pictura-app/cpp` exceeds 16,000 bytes,
  and wire it into `scripts/guard.sh` so it runs on every fast gate and in the
  `guards` CI job. This reproduces MSVC's per-literal limit on Linux, so future
  growth is caught before a Windows runner is needed.
- **Temporarily** let `windows.yml` run on ordinary pull requests (add the C++
  and Windows packaging paths to its `paths:` filter and drop the
  release-please-only job `if:`) so the fix is validated on this PR. This is
  scaffolding and is reverted within this change: the final diff leaves
  `.github/workflows/windows.yml` unchanged.
- Record the fix and the guard in `docs/dev/STATE.md`.

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `meta/windows-release-build`: add a requirement that the C++ shell stays
  within MSVC's per-literal string-literal limit — the stylesheet is delivered
  as multiple literals, and a literal over the limit fails the repository guard.

## Impact

- `crates/pictura-app/cpp/theme.cpp` — split the one oversized stylesheet
  literal; no behaviour change.
- `scripts/check-msvc-string-limits.py` (new) and `scripts/guard.sh` — the
  literal-limit guard, standard-library Python only, no new dependency.
- `.github/workflows/windows.yml` — temporary PR gate/paths relaxation used to
  validate the fix, reverted within this change (no net diff).
- `docs/dev/STATE.md` — record the outcome and the guard (commit carries
  `TASK-ALLOWS-DOCS`).
- Follow-up (not in this change): the same release-only gate on `appimage.yml`
  remains a latent copy of the feature-CI blind spot.
