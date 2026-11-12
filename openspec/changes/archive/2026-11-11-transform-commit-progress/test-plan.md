# Test plan

This plan covers the added `document/long-operation-progress` and the modified
`tools/move-commit-responsiveness` for `transform-commit-progress` (#1278, PR 1).

**Fixtures.** Engine tests use small seeded layers (e.g. 257 × 131, so row
counts are odd). The app and Qt tests reuse `tst_large_document`'s 4200² fixture,
which is the smallest size over the 4 MP job threshold. They hold the job with
the `transform_set_delay` test seam, so the window and Cancel are deterministic.
Cancel is triggered through the `jobProgress` dialog's Cancel button, or Esc.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lop_quick_job` | Run `runCancellableJob` with a job that finishes at once; sample `QApplication::overrideCursor()` inside `running()` | Wait cursor set while running, no `jobProgress` window shown, cursor restored | `tst_busy_job::quickJobShowsOnlyTheWaitCursor` |
| `lop_window_delay` | Run a job held for 600 ms; check the window before and after 250 ms; count a 10 ms timer | Hidden before 250 ms, visible with a Cancel button after, the timer keeps firing | `tst_busy_job::slowJobShowsTheProgressWindow` |
| `lop_esc_cancels` | Run a held job; once the window is visible, send Esc to it | The cancel callback runs and the job ends | `tst_busy_job::escCancelsTheJob` |
| `lop_progress_monotonic` | Transform a 257 × 131 layer under a job token, with a sampler thread recording `permille()` | Values never decrease; final value is 1000 | `pictura-core::job::progress_never_decreases`, `pictura-render::transform_progress_rises_to_completion` |
| `lop_engine_cancel` | Transform under a token with cancel already set; also cancel mid-way from a second thread | `job::run` returns `Cancelled`; the input document's layer pixels and rect are unchanged | `pictura-core::job::a_cancelled_run_reports_cancelled`, `pictura-render::a_cancelled_transform_leaves_the_document` |
| `lop_cancel_unchanged` | Commit a large transform, cancel the held job | Composite pixel-identical to before; history count and index unchanged | `pictura-app::transform_commit_cancel_changes_nothing`, `tst_large_document::cancellingATransformResumesTheSession` |
| `lop_large_document` | Open a 17000 × 17000 document, rotate a full-canvas layer 7°, commit; watch the window; Cancel once, then commit again | Canvas repaints, bar advances, Cancel returns within 1 s; second commit lands as one history state | manual: a 17000² document needs about 1.2 GB and minutes of CPU per run, too large and slow for CI |
| `mcr_transform_deferred` | Commit a 4200² rotated layer with the job held | `jobProgress` appears, value rises, landing adds one `"Free Transform"` state, handles gone, image stays until `moveCommitted` | `tst_large_document::aTransformCommitLandsOffThread`, `pictura-app::transform_commit_lands_one_history_state` |
| `mcr_transform_cancel` | Commit the same transform, press Cancel while held | Session active with the same quad, composite identical, history count unchanged | `tst_large_document::cancellingATransformResumesTheSession`, `pictura-app::transform_commit_cancel_resumes_the_session` |
| `mcr_transform_lifted_job` | Select a 4200² region, Free Transform, commit then cancel; commit again and let it land | Lifted session resumes on cancel; second commit adds one state and merges the pixels | `pictura-app::lifted_transform_commit_runs_as_a_job` |
