# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-core`: `job::progress_never_decreases`, `job::a_cancelled_run_reports_cancelled`.
- [x] 1.2 `pictura-render`: `transform_progress_rises_to_completion`, `a_cancelled_transform_leaves_the_document`.
- [x] 1.3 `pictura-app`: `transform_commit_lands_one_history_state`, `transform_commit_cancel_changes_nothing`, `transform_commit_cancel_resumes_the_session`, `lifted_transform_commit_runs_as_a_job`.
- [x] 1.4 Qt: new `tst_busy_job` (`quickJobShowsOnlyTheWaitCursor`, `slowJobShowsTheProgressWindow`, `escCancelsTheJob`); `tst_large_document::aTransformCommitLandsOffThread` adapted, `cancellingATransformResumesTheSession` added.

## 2. Engine

- [x] 2.1 `pictura-core::job`: `JobProgress` (stages: `set_stages`, `begin_stage`, `advance`), `Cancelled`, `run`, `current`, `scope`.
- [x] 2.2 `resample_plane` (transform), `transform_native` and `warp_planes`: `begin_stage` per layer, `advance` per output row or band, skip rows once cancelled and refuse before `write_layer`.

## 3. App

- [x] 3.1 `TransformCommitJob` replaces `DeferredTransform` / `TransformJob` / `build_transform`; lifted commits use it too.
- [x] 3.2 Bridge: `transform_commit_running`, `transform_commit_progress`, `transform_commit_cancel`, `transform_commit_finish`.
- [x] 3.3 `settle_pending_composite` joins and lands a pending job.
- [x] 3.4 `runCancellableJob` sets the wait cursor; `runTransformCommit` helper; route the four C++ commit sites through it.

## 4. Close-out

- [x] 4.1 Docs: `docs/dev/STATE.md`; ROADMAP unchanged (no roadmap item for #1278).
- [x] 4.2 `openspec validate --all --strict`, `scripts/check-test-plan.py`, one build, `verify-fast`.
- [x] 4.3 Archive the change in the PR.
