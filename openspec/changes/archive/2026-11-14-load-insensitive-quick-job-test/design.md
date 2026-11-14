# Design

## Context

See `proposal.md` — Why.

`runCancellableJob` (`crates/pictura-app/cpp/busy_job.cpp`) runs `start()`,
constructs the `QProgressDialog`, starts its own 250 ms window clock, and only
then evaluates `running()` for the first time at line 118. If that first poll
returns true it sets the override cursor at line 119 and enters the event loop.
The test's clock therefore starts before dialog construction while the runner's
own clock starts after it.

`tst_busy_job.cpp` is the only direct test of `runCancellableJob`'s
cursor/window timing; `tst_document_jobs`, `tst_auto_align`, and
`tst_auto_blend` exercise it indirectly through their command paths. Its other
three cases also start their clocks in `start()` but allow 700 ms, 600 ms, and
10 s (`tst_busy_job.cpp:102`, `:159`, `:133`), so their margins absorb dialog
construction; only the 60 ms quick-job case does not.

## Goals / Non-Goals

**Goals:**

- Make `quickJobShowsOnlyTheWaitCursor` independent of dialog construction time
  and poll scheduling under host load.
- Keep all three existing assertions and the 60 ms quick-job deadline.
- Keep the fix to one test file; the job loop runs about 60 ms and dialog
  construction stays outside the test clock.

**Non-Goals:**

- Changing `runCancellableJob` or its cursor behavior. The issue confirms the
  logic is correct; setting the cursor before the first `running()` poll would
  change behavior for jobs that complete synchronously.
- Widening the other three cases, which have ample headroom.
- A general timing-test policy for the repository; this is one test's clock
  origin.

## Decisions

### D1. The test clock starts on the first `running()` poll

The first `running()` call comes after the dialog is constructed and after the
runner starts its own 250 ms clock at `busy_job.cpp:99`; only the poll setup
sits between. Timing from the first poll therefore measures the job loop, and
the clock trails the runner's by that small setup delta rather than by dialog
construction. A `started` flag starts the clock once, on the first poll.

Alternative considered: widen the deadline from 60 ms to about 200 ms while
leaving the clock in `start()`. Rejected: it still fails whenever construction
takes longer than the new deadline, so it narrows the flake rather than
removing it, and it slows the case by 140 ms.

### D2. Latch the cursor witness with a zero-delay probe

The first `running()` call returns before `setOverrideCursor` runs, so the
earliest normal cursor sample is the next 16 ms poll. On a loaded host that
poll may not be delivered within the 60 ms job window. Scheduling
`QTimer::singleShot(0, ...)` from the first poll puts a probe on the event
queue before the 16 ms poll; it fires when `loop.exec()` starts, after line 119
has set the cursor, and is due earlier than every poll tick. The existing
per-poll sample stays as a second witness.

Alternative considered: rely on the first 16 ms poll and keep the 60 ms
deadline after D1. Rejected: it leaves exactly the scheduling dependency this
change is meant to remove.

### D3. Keep the 60 ms deadline

Trailing is the risky direction for the no-window assertion. After D1 the test
trails the runner by the poll-setup delta, so the runner reaches 250 ms while
the test is still under 60 ms only if that delta is about 190 ms. Widening the
deadline to 200 ms would shrink that margin to about 50 ms and make the
no-window assertion easier to trip. The 60 ms deadline also keeps the case
fast.

### D4. The probe is scoped to the job loop

The probe captures the test locals by reference and is armed with a local
`QObject probeContext` as its context, so a skipped loop cancels it before the
test locals die. In the intended flow the first poll returns true by
construction, so it fires during `loop.exec()` while `runCancellableJob` is
still on the stack. The loop can only quit from a poll callback, which is due
16 ms later than the probe.

## Risks / Trade-offs

- A future change that starts the job loop without setting the cursor would
  still fail the probe, which is the intended behavior.
- The probe is one extra timer in a test; it adds no production code and no
  measurable runtime.
- The other cases in this suite remain theoretically load-sensitive at their
  current margins. They are not part of issue #1313 and are left unchanged.

## Migration Plan

None. Revert the test-file diff and restore the change folder from
`openspec/changes/archive/`.

## Open Questions

None.
