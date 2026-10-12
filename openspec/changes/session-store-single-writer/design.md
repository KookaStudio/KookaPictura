# Design: session-store-single-writer

## Context

`$XDG_STATE_HOME/kooka-pictura/state.json` is load-modify-written by
`PicturaMainWindow::saveSession()` (`frame_session.cpp:111`), which
`closeEvent` (`frame.cpp:1009`) and `aboutToQuit` (`main.cpp:187`) both call.
Every other preference writer does a fresh load immediately before its write, so
the loss window is the frame's: it loads at startup and writes much later, and
an instance that started earlier can still be running when a newer one writes.
The workspace store (`workspaces/`) is snapshotted from the same save path.

## Goals / Non-Goals

**Goals:**
- A launch cannot overwrite a store another launch wrote after it started.
- A self-test can never write the real store, even if its temp dir fails.
- No new dependency, no lock file, no merge machinery.

**Non-Goals:**
- A single-instance lock or a takeover after the other instance exits.
- Per-key merging of concurrent edits.
- Profile separation (`--profile NAME`), which the issue marks optional.

## Decisions

- **Compare-and-swap on a writer token, not file mtime.** `saveSession` reads
  the existing object anyway for the unknown-key-preserving rewrite, so it reads
  `writerToken` (a `QUuid` created once per process) and `writtenAt`
  (epoch ms) from the same parse. Refusal is
  `!writer.isEmpty() && writer != instanceToken() && writtenAt >= instanceStartMs()`:
  a store written by a different launch at or after this launch's start is
  newer than anything this instance loaded. `instanceToken`/`instanceStartMs`
  are function-local statics, so a direct save from a preference dialog shares
  the process identity and is never refused by its own earlier write. File mtime
  was rejected: it has clock-resolution and touch-rewrite ambiguity and no
  identity.
- **The launch start is the process start.** The frame loads the session in its
  constructor, within milliseconds of the statics' first use, so a store written
  after process start is by construction after this instance loaded. This avoids
  threading a baseline revision from the frame into the free function.
- **Refusal is a `false` return.** `saveSession` already returns `bool`; the
  frame captures it and skips `snapshotActiveWorkspace()`, so the stale instance
  leaves the workspace store alone too. Other call sites already ignore the
  return, matching their current best-effort behaviour.
- **A missing token is adopted.** Stores from older builds and test fixtures
  carry no token; refusing them would strand every existing profile. The first
  write stamps the token.
- **The self-test abort takes the next free code.** Exit code 199 is the next
  unused self-test identity (highest was 198) and the token line follows the
  `FAIL <suite> <name> <code> <message>` protocol so the unified reporter can
  parse it. `main.cpp` already creates the temp dir before the frame; the check
  simply returns instead of resetting the optional.

## Risks / Trade-offs

- [A refused instance never writes again, even after the other exits] → Accepted
  by the issue ("the later write is refused"); a restart adopts the store. A
  lock/heartbeat is the upgrade path if this bites.
- [Clock skew across two launches] → Same machine, same wall clock; the
  comparison is only used to order two writes, and equal timestamps refuse
  (data-loss-safe).
- [Existing tests write `state.json` directly] → They write no token, so the
  adoption path keeps them green; `tst_session_store` covers the refusal and
  adoption explicitly.

## Migration Plan

None: the store stays schema version 9; the two new keys are additive and
unknown-key preservation already covers older builds reading them back.

## Open Questions

None.
