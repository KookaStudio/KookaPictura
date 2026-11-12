# Proposal

## Why

Issue #1278: on large documents, several operations still block the GUI thread
for seconds, and the window looks frozen while they run. Free Transform commit is
the worst case. A transform of 4 MP or more already resamples on a worker, but
there is no window, no progress and no way to cancel. Any reader that settles the
pending transform, such as the next menu command or tool press, then finishes the
resample synchronously on the GUI thread.

This change is the first of two pull requests for #1278. It adds the shared
long-operation job contract and moves Free Transform commit onto it. The
whole-image operations follow in the second pull request (Image Size, Canvas
Size, Rotate/Flip, mode and bit-depth conversion, Merge Down/Visible, Flatten).

## What Changes

- **Engine progress and cancellation.** A shared `JobProgress` token in
  `pictura-core` carries a cancel flag and a monotonic 0–1000 progress value. The
  transform and warp resamplers check the flag and report progress at output-row
  boundaries. A cancelled resample returns no result and leaves its document
  untouched.
- **Cancellable job window.** `runCancellableJob` sets the wait cursor at once,
  as `runBusyJob` already does. Its window still appears after 250 ms. Cancel and
  Esc set the cancel flag.
- **Free Transform commit as a job.** A commit of 4 MP or more, including a
  lifted selection and a warp, runs as a cancellable job with real row progress.
  The handles stay on screen until the job lands. Cancel returns to the live
  transform session, with the document and history unchanged. A completed job
  records the same single `"Free Transform"` state as today. Smaller commits stay
  synchronous.

## Capabilities

- **ADDED** `document/long-operation-progress`: the shared job window, progress,
  and cancel contract.
- **MODIFIED** `tools/move-commit-responsiveness`: "A large transform commit
  leaves the GUI thread" becomes a modal cancellable job.

## Impact

- `pictura-core`: new `job` module.
- `pictura-render`: the transform, quad, and warp entry points take a job token.
- `pictura-app`: the transform commit job replaces `DeferredTransform` and
  `TransformJob`. `busy_job.cpp` gains the wait cursor, and every C++
  `commit_transform` call site goes through one helper.
