# Proposal

## Why

Auto-Align and Auto-Blend each own a near-identical background-plan protocol: a
worker handle, cancellation and progress atomics, a captured content revision,
cancellation on drop, polling, joining, stale-result rejection, and one-shot
landing. The two copies must stay correct independently, so a change to one
leaves the other able to apply a plan the document no longer supports (#1399).

## What Changes

- Add one `PreparedJob<P>` owner in
  `crates/pictura-app/src/cxxqt_object/prepared_job.rs` for a plan computed on a
  worker from a document snapshot: start, running, progress, stop, discard, and
  a one-shot `land` returning `Landed::Empty | Ready | Stale | Cancelled |
  Failed`.
- Hold one `PreparedJob<Plan>` in each of `AutoAlignJob` and `AutoBlendJob`.
  Their plan options, apply/edit block, post-commit messages, and FFI surface
  stay where they are.
- Delete the two `Run` types, their `Drop` impls, both `landed` functions, and
  the duplicated start/running/progress/cancel bodies, plus the two duplicate
  copies of the Rust tests.
- Refuse a landed plan whose run was asked to stop, so a cancellation that
  arrives after the engine's last check cannot commit. The observable rule is
  already "cancelling SHALL commit nothing"; this makes it hold at the seam.
- Keep one shared test module that defines each preserved Rust check
  (`a_stale_plan_is_dropped`, `dropping_a_run_asks_its_worker_to_stop`) once,
  and keep every `tst_auto_align` / `tst_auto_blend` check.

No algorithm, eligibility, dialog, selection, or pixel behavior changes, and
history is unchanged except for one intended rule: a stop that arrives after the
worker returns its plan now commits nothing. No dependency is added. The
whole-document `DocumentJob` and the filter worker are untouched.

## Capabilities

### New Capabilities

- `document/prepared-plan-job`: the shared lifecycle for a plan computed on a
  worker from a document snapshot — one owner for the worker, cancel/progress
  state, captured revision, one-shot joining and stale/cancelled/failed
  rejection.

The user-visible cancel and history guarantees stay owned by
`document/long-operation-progress`, `document/auto-align-layers`, and
`document/auto-blend-layers`; this capability owns only the shared seam they
call. The change is named after issue #1399; the capability is named
`prepared-plan-job` because the seam carries a plan, not a whole-document
mutation.

### Modified Capabilities

<!-- none: the Auto-Align and Auto-Blend requirements keep their observable
     behavior; only how the app reaches them changes -->

## Impact

- **Code**: new `crates/pictura-app/src/cxxqt_object/prepared_job.rs` plus
  `mod prepared_job;` in `cxxqt_object.rs`; `auto_align/imp.rs` and
  `auto_blend/imp.rs` shrink to their plan options and apply/edit halves;
  `auto_align.rs`, `auto_blend.rs`, `state.rs`, and
  `document_session/session.rs` keep their current names (session close
  delegates to `discard`).
- **Tests**: the two inline `#[cfg(test)] mod tests` blocks move into the new
  module with the existing named checks plus panic, cancel, a refused start
  while a run is held, drop, one-shot landing, and completion-before-first-poll
  checks. `tst_auto_align` and `tst_auto_blend` are unchanged.
- **Dependencies**: none.
