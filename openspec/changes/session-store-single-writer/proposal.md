# Proposal: session-store-single-writer

## Why

Issue #332: the XDG session store is a single shared profile and `saveSession()`
writes the whole store with no check that another instance has written since this
one loaded it, so whichever instance quits last wins. During a dev loop (app
open, rebuild, launch a second window to check, close the stale one) that
silently overwrites the arranged workspace with the stale instance's layout,
which then looks like the app reset the workspace. The self-test also has a
fallback that would write the real store when its temp dir cannot be created,
and it leaves a second temp state dir under `/tmp` behind.

## What Changes

- **Stale-instance guard.** Every write stamps the store with a per-launch
  `writerToken` and `writtenAt`; `saveSession` refuses when the store's token is
  another launch's and its write time is not older than this launch's start. A
  refused frame save also skips the active-workspace snapshot, so the workspace
  store cannot be clobbered either. A store with no token (an older build, a
  test fixture) is adopted and stamped with this launch's token.
- **Self-test hardening.** A self-test run whose state temp dir cannot be
  created fails (exit 199, the next free self-test code) instead of falling back
  to the real store, and the redundant second `XDG_STATE_HOME` redirect in
  `selftest.cpp` is dropped so no `/tmp/kooka-pictura-selftest-*` litter remains.

## Capabilities

### Modified Capabilities

- `document/workspace-persistence`: the session state store gains the writer
  token, the refusal rule, and the self-test isolation contract.

## Impact

- `crates/pictura-app/cpp/session.cpp`, `frame_session.cpp`, `main.cpp`,
  `selftest.cpp`; new `tst_session_store` Qt Test. No new dependency.
- Ceilings (`ponytail:`): a refused instance stays read-only for the rest of its
  run (no lock, merge, or takeover when the other instance exits); the token is
  per launch, not per window.
