# Test plan

This change has no spec scenarios (`skip_specs: true`). The rows below map the
issue's acceptance criteria and the design risks to runnable checks.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `lop_quick_job` — a quick job shows only the wait cursor | Run the quick-job case with its clock started on the first poll and the zero-delay probe armed | The cursor was observed while the job ran, no progress window appeared, and the cursor was restored | `tst_busy_job::quickJobShowsOnlyTheWaitCursor` |
| Dialog construction is slower than the job window | Run the case under `scripts/cpu-budget.py exec` on a loaded host so `QProgressDialog` construction exceeds 60 ms | The case still observes the wait cursor and shows no window, within design D3's ~190 ms setup-skew margin | manual: reproduced only under host load; the case returns true from its first poll, so construction never enters its clock |
| The other job behaviors are unchanged | Run the whole suite | Slow job shows the window, Esc cancels, and a negative progress value shows a busy bar | `tst_busy_job::slowJobShowsTheProgressWindow`, `tst_busy_job::escCancelsTheJob`, `tst_busy_job::aNegativeProgressShowsABusyBar` |
| Poll scheduling is starved | Run the single case 20 times under the 80% budget and a few times with `PICTURA_CPU_BUDGET_PERCENT=5` | 20 of 20 pass and the stressed runs pass | manual: `QT_QPA_PLATFORM=offscreen scripts/cpu-budget.py exec -- build/crates/pictura-app/cpp/tests/tst_busy_job quickJobShowsOnlyTheWaitCursor` |
| Workspace gates | Run fmt, clippy, the unified test report, OpenSpec validation, and the fast gate | All pass | manual: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `bash scripts/verify-fast.sh`; `openspec validate --all --strict` |
