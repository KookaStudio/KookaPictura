# Test plan

This plan covers the modified `tools/move-commit-responsiveness` for
`transform-spill-progress` (#1290, phase 5). The scenarios it keeps unchanged
(`mcr_transform_deferred`, `mcr_transform_cancel`, `mcr_transform_lifted_job`)
keep their existing tests.

**Fixtures.** The gate's room and the scratch disks' free bytes are pinned on
the GUI thread (`set_available_memory_for_test`,
`set_scratch_available_for_test`). The Qt test uses a 1000² document, under the
4 MP job threshold, with its layer's corner dragged out by 2000 px (an output of
about 3000², about 69 MiB spillable). It holds the job with
`transform_set_delay`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mcr_transform_deferred` | Commit a 4200² rotated layer with the job held | `jobProgress` appears, value rises, one `"Free Transform"` state lands | `tst_large_document::aTransformCommitLandsOffThread` |
| `mcr_transform_cancel` | Commit the same transform, press Cancel while held | Session resumes; composite and history unchanged | `tst_large_document::cancellingATransformResumesTheSession` |
| `mcr_transform_lifted_job` | Lift a selection, commit, cancel, commit again | Lifted session resumes; second commit adds one state | `pictura-app::lifted_transform_commit_runs_as_a_job` |
| `mcr_transform_spill_job` | Pin room to the working set plus 32 MiB and 64 GiB of scratch; commit the 1000² layer scaled to about 3000² with the job held; Cancel from the window | Gate admits; `expects_spill` is true; `jobProgress` appears; Cancel resumes the session; display and history unchanged | `pictura-app::a_commit_beyond_the_room_expects_to_spill`, `tst_memory_governor::aSpillingTransformRunsAsAJob` |
| `mcr_transform_fits_sync` | Unpin; commit the same session with the job delay still set | No `jobProgress` window; history gains one state on return | `pictura-app::a_commit_beyond_the_room_expects_to_spill`, `tst_memory_governor::aSpillingTransformRunsAsAJob` |
| `mcr_transform_spill_progress` | Open a 17000 × 17000 single-layer document under `systemd-run --user -p MemoryMax=2G`; Free Transform ×1.5; commit; watch the window; Cancel once, then commit again | Window shows; value keeps rising while tiles spill; Cancel restores the document with no history state; second commit lands one state | manual: needs a 289 MP document, a cgroup limit and several GB of free scratch disk, too large and host-dependent for CI |
