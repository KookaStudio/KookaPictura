# Proposal

## Why

Issue #1290, phase 5. Phases 3–4 let a Free Transform commit spill its output
tiles to the scratch disk instead of refusing. But a commit goes to the
cancellable job (#1278) only when its source layer rect reaches 4 MP. A smaller
layer scaled up to a large output, or a commit made when the governor has little
room, can still spill. It then runs on the GUI thread with no progress window and
no Cancel while the evictor writes tiles to disk.

## What Changes

- **A commit expected to spill runs as a job.** After the gate admits a commit,
  the commit runs as the cancellable job when its source rect reaches 4 MP, as it
  does today, or when its working bytes plus its spillable bytes exceed the
  governor's room. The job is the existing #1278 job. Its progress counts the
  resampled output rows, which keep advancing while the strips' tiles are
  written and spilled, and its Cancel leaves the document and history unchanged.
- Any other commit stays synchronous.

Not in this change: Move tool commits (their own deferred composite), warp
(#1292), Image Size (#1293), Canvas Size and Rotate Canvas (#1294).

## Capabilities

- **MODIFIED** `tools/move-commit-responsiveness`: "A large transform commit
  leaves the GUI thread" also sends a commit expected to spill to the job.

## Impact

- `pictura-app`: `memory_gate::expects_spill`; `commit_transform` checks it
  next to `DEFERRED_TRANSFORM_PIXELS`.
