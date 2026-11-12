# Test plan

This plan covers the modified `tools/move-commit-responsiveness` for
`transform-cancel-ends-session` (#1290, phase 5 review). Fixtures are those of
`transform-commit-progress` (a 4200² document held by `transform_set_delay`) and
`transform-spill-progress` (a 1000² document with the room pinned).

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mcr_transform_deferred` | Commit a 4200² rotated layer with the job held | `jobProgress` appears, value rises, one `"Free Transform"` state lands | `tst_large_document::aTransformCommitLandsOffThread` |
| `mcr_transform_cancel` | Commit a 4200² transform with the job held; press Cancel | No session; handles and transform image gone; display and history unchanged | `tst_large_document::cancellingATransformEndsIt` |
| `mcr_transform_lifted_job` | Lift a selection; run its job, cancel; run it again and land | Cancelled job lands nothing, composite and selection unchanged; second adds one state and merges | `pictura-app::lifted_transform_commit_runs_as_a_job` |
| `mcr_transform_spill_job` | Pin room to the working set plus 32 MiB and 64 GiB of scratch; commit a 1000² layer scaled to about 3000² with the job held; Cancel | `jobProgress` appears; no session afterwards; display and history unchanged | `pictura-app::a_commit_beyond_the_room_expects_to_spill`, `tst_memory_governor::aSpillingTransformRunsAsAJob` |
| `mcr_transform_fits_sync` | Unpin; start the same transform again and commit with a 600 ms job delay | No `jobProgress` window; history gains one state on return | `tst_memory_governor::aSpillingTransformRunsAsAJob` |
| `mcr_transform_spill_progress` | Open a 17000 × 17000 single-layer document under `systemd-run --user -p MemoryMax=2G`; Free Transform ×1.5; commit; watch the window; Cancel once, then transform and commit again | Window shows; value keeps rising while tiles spill; Cancel shows the original layer with no handles and no history state; second commit lands one state | manual: needs a 289 MP document, a cgroup limit and several GB of free scratch disk, too large and host-dependent for CI |
