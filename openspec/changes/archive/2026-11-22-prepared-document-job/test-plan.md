# Test Plan: prepared-document-job

All checks are Rust unit tests. The lifecycle owner's checks live in the new
`cxxqt_object/prepared_job.rs` module, where the two inline
`#[cfg(test)] mod tests` blocks move from the adapters and exercise the owner
directly with a fake plan. The document-replacement check lives beside Auto-Align
in `cxxqt_object/auto_align/imp.rs`. The existing `tst_auto_align` and
`tst_auto_blend` suites are run unchanged as integration checks in `tasks.md`.

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| `ppj_one_run` | Start a request, attempt a second start while the first runs, then wait for the first worker to finish and attempt a start again before landing. | Both attempts are refused while the run is held (running, then finished but not yet landed), and landing the first still yields its plan at the revision captured at its start. | `pictura-app::a_second_start_is_refused_until_the_first_lands` |
| `ppj_poll` | Start a request; poll running and progress; land; poll again. | Running reflects the worker, progress matches the value the plan wrote, and both report idle/zero after landing. | `pictura-app::a_run_reports_running_and_progress` |
| `ppj_restart` | Start, land, then start again. | The new start is accepted; landing it at the old revision reports stale and landing it at the new revision yields its plan. | `pictura-app::a_second_start_is_refused_until_the_first_lands` |
| `ppj_once` | Start a worker that returns a plan; land, poll and stop, then land again. | The first landing yields the plan, polling and stopping do not resurrect it, and the second landing yields nothing. | `pictura-app::a_finished_plan_lands_once` |
| `ppj_before_poll` | Start a worker that finishes before any poll, then land without polling. | Landing joins the finished worker and yields its plan. | `pictura-app::a_finished_plan_lands_once` |
| `ppj_stale` | Start at revision N, land at revision N+1. | Landing reports stale and yields no plan; the same test asserts a matching revision yields the plan and a worker returning no plan yields nothing. | `pictura-app::a_stale_plan_is_dropped` |
| `ppj_cancel` | Start at revision N with a holding delay, request a stop, change the revision to N+1, let the fake plan return a plan and record that it ran, then land; then start a worker that returns a plan, wait for it to finish, request a stop, and land. | Both landings report cancelled and yield no plan: the first proves the worker ran, was joined, and the stop wins over stale; the second proves a stop that arrives after the worker finished still commits nothing. | `pictura-app::a_cancelled_plan_lands_nothing` |
| `ppj_panic` | Start a worker whose plan call panics; land. | Landing reports failed, yields no plan, and the panic does not propagate to the caller. | `pictura-app::a_worker_panic_lands_nothing` |
| `ppj_drop` | Hold a run, clone its cancellation flag, drop the owner. | The cancellation flag is set and dropping returned without joining the worker. | `pictura-app::dropping_a_run_asks_its_worker_to_stop` |
| `ppj_discard` | Hold a run, discard it, start another. | The discard asked the first worker to stop without joining, the new start is accepted, landing the discarded run yields nothing, and landing the new run at its revision yields its plan. | `pictura-app::discarding_a_run_allows_a_new_start` |
| `ppj_replaced` | Hold a run with a delay, replace the document it was prepared from, then land and start again. | The held run is discarded, landing yields nothing, and the new start is accepted and lands its plan. | `pictura-app::replacing_the_document_discards_its_run` |
