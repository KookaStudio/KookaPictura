# Design

## Context

See `proposal.md` — Why. Today `auto_align/imp.rs:22-143` and
`auto_blend/imp.rs:24-141` each define `Run`, its `Drop` cancellation,
`AutoAlignJob`/`AutoBlendJob`, and a `landed` function. Their `start`,
`running`, `progress`, `cancel`, and `finish` bodies are the same protocol over
`plan_auto_align` / `plan_auto_blend`, which already share the
`(&AtomicBool, &AtomicU32)` cancellation and progress shape
(`pictura-render` `document_ops/layer_ops/auto_{align,blend}/mod.rs`).

The C++ shell already shares its polling loop (`runCancellableJob`,
`cpp/busy_job.h`); the duplication that remains is Rust-side. Two neighbouring
owners are explicitly different: `impl_core/document_job.rs` mutates the live
document and lands by swapping it in, and `filter_tools/jobs.rs` filters a
layer; neither is a prepared plan. The document replacement path already calls
`clear_document_scoped_state()`, which calls each adapter's `cancel()` and
therefore discards a held run before the new document is installed.

## Goals / Non-Goals

**Goals:**
- One owner for the worker handle, cancel/progress state, captured revision,
  delay seam, polling, joining, and stale/cancelled/failed rejection.
- The plan's computation and application remain in the two adapters.
- Identical observable behavior for both commands, minus the cancellation race
  closed by the specs.

**Non-Goals:**
- No universal job framework, no trait over plan types, no change to
  `DocumentJob`, the filter worker, or the C++ orchestration.
- No new cancellation or progress API in `pictura-render`; engines keep their
  `AtomicBool`/`AtomicU32` parameters.
- No change to dialogs, eligibility, selection, history labels, or messages.

## Decisions

### Generic owner with a plan closure, not a trait

`PreparedJob<P>` holds `run: Option<Run<P>>` and the test delay. `Run<P>` holds
`worker: Option<JoinHandle<Option<P>>>`, `cancel: Arc<AtomicBool>`,
`progress: Arc<AtomicU32>`, and `revision: u64`; its `Drop` sets the cancel
flag. `start` takes the snapshot `Document`, the captured revision, and
`FnOnce(&Document, &AtomicBool, &AtomicU32) -> Option<P>`; `None` is a cancelled
plan. `land(revision)` takes the run and yields
`Landed::{Empty, Ready(P), Stale, Cancelled, Failed}`.

Alternatives rejected:
- A `PreparedPlan` trait with `plan`/`apply`/`changed` methods adds a second
  implementation seam for two plans and drags application into the shared type,
  which the issue forbids ("do not build a universal job framework").
- Extending `DocumentJob` would import its whole-document ownership and
  source-state model into read/plan/apply work.
- Free functions cannot own a one-shot handle across calls without a state
  struct anyway; `PreparedJob` is that struct.

### Apply, edit decision, and messages stay in the adapters

Align must reject a plan with no lens correction and only `Identity` warps
*before* `view.edit` (`apply_auto_align` returns the reference layer's Identity
path, so an empty path list is not "nothing changed"). Blend must compute
`masks_only` before applying to report "masks only" after a successful commit.
Both clear link sets and return `Edit::Structural { damage: None }` inside
`view.edit`, but their applicability tests and post-commit messages differ.
Sharing that block would need one flag per difference; the owner stops at
`Landed<P>`.

### Cancellation is refused at landing

`land` takes the held run, takes its worker handle, joins it once, and only then
classifies the outcome, in this order: a join panic is `Failed`; a requested
stop is `Cancelled` even when the worker returned a plan; a worker that returned
no plan is `Cancelled`; a revision mismatch is `Stale`; otherwise
`Ready(plan)`. Without the stop check, a cancel that arrives after the engine's
last internal check but before it returns would commit. The spec already says a
cancel commits nothing; this enforces it at the seam. `Ok(Err(cancelled))`
already wins over a stale revision today, so the engine-cancel arm is
preserved. The existing Qt cancel tests assert "nothing committed", not "engine
returned cancellation", so they still pass.

### Panic payloads are forgotten

`JoinHandle::join` returns the panic payload; `land` reports `Failed` and calls
`std::mem::forget(payload)`, matching `impl_core/document_job.rs:399-402`. The
payload is not inspected and its destructor cannot panic the GUI thread.

### The test delay belongs to the owner

The delay loop before the plan call is identical in both adapters and exists
only so Qt tests can act mid-run. It moves into `start` with a
`set_delay_for_test(ms)` setter, and `discard` preserves the configured delay.
The loop runs inside the spawned worker body, checks the stop flag about every
millisecond, and never blocks `start`. The adapter structs keep their existing
FFI-facing names and delegate `set_delay_for_test`, `running`, and `progress`;
the FFI `auto_*_cancel` calls the owner's `stop()`, and the adapter's
session-close `cancel()` calls `discard()`.

### One-shot landing is structural

The handle is `Option<JoinHandle<...>>` only so `land` can take it out (a type
with `Drop` cannot be destructured); a second `land` finds no run and yields
`Empty`. No flag or generation counter is needed.

## Risks / Trade-offs

- **`land` joins on the GUI thread** → unchanged from today; the shell lands
  only after `running` is false, and both adapters already join there.
- **A panicking worker prints the default panic message in test output** →
  accepted; the panic is caught by `join` and the test asserts `Failed`.
- **`mem::forget` leaks one payload on a panic** → deliberate, matching
  `document_job`; a panicking destructor must not unwind the GUI thread.
- **Generic bounds make the plan closure `Send + 'static`** → both plan
  options are plain data; no new constraint in practice.
- **A caller that ignores `Landed` variants could apply on `Stale`** → not
  possible at the seam: no plan is yielded for stale, cancelled, or failed.

## Migration Plan

1. Add `prepared_job.rs` with the owner, `Landed`, and its unit tests (the named
   checks move here, plus the new panic/cancel/once/drop checks).
2. Switch Auto-Align and Auto-Blend to hold `PreparedJob<Plan>`; delete their
   `Run`, `Drop`, `landed`, and duplicated method bodies. Both adapters stay
   green independently.
3. Run the existing Qt suites unchanged, then the full gates.
4. Archive the change in the same PR that ships it.

Rollback is the revert of one PR; no document format, persisted state, or
dependency changes.

## Open Questions

- None blocking. Thread-spawn failure (`std::thread::spawn` panics) is left as
  today and is a candidate for a later `Builder` change.
