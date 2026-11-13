# Test plan

This plan covers `document/out-of-core-warp`, and the modified
`interop/agentic-control` and `verification/memory-benchmarks` requirements, for
`warp-commit-out-of-core` (#1292).

**Fixtures.** Engine tests use seeded layers. The dense rasterizer stays as a
`#[cfg(test)]` oracle (`warp_oracle::dense_warp_planes`), unchanged from before
this change. Under `cfg(test)` the window budget is small, so every test warp
also splits its blocks. Gate tests pin the host's available memory and the
scratch volumes' available bytes. The spill test configures a temporary scratch
volume and runs `eviction::run_pass`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ocw_warp_exact` | Warp a seeded 8-bit layer with a mask through an arc, a twist, a folding mesh and a shrinking mesh, with each interpolation | Channels and mask tiled; equal to the dense rasterizer byte for byte | `pictura-render::tiled_warp_matches_the_dense_rasterizer` |
| `ocw_warp_deep` | Warp 16-bit and 32-bit layers with a retained store and a mask through an arc | Every retained plane tiled and byte-identical to the dense rasterizer | `pictura-render::deep_stores_warp_through_tiles` |
| `ocw_uniform_tiles` | Warp a 2048² solid layer through a flag warp | Each channel allocates under a quarter of the dense plane | `pictura-render::a_solid_layer_warps_into_uniform_tiles` |
| `ocw_output_spills` | Warp a 1024² seeded layer with a scratch volume, then run eviction passes | Some tiles spilled; the layer reads back equal to the dense rasterizer | `pictura-render::warped_tiles_spill_and_read_back` |
| `ocw_cancel` | Warp under an already-cancelled job | No change; rect and pixels unchanged | `pictura-render::a_cancelled_warp_changes_nothing` |
| `ocw_memory_max` | `python3 scripts/profile-tools.py --scenarios M14 --memory-max 4G` at 17000² | `outcome: ok`, no OOM kill or refusal | manual: needs a 17000² document under a systemd cgroup memory limit and several minutes of a real app; run for this change and recorded in `docs/dev/STATE.md` |
| `ocw_gate_spills` | Gate a 17000² full-canvas warp's commit cost at 1.75 GiB room and 64 GiB scratch | Working set includes the strip bytes; admitted | `pictura-app::a_large_warp_is_admitted_when_scratch_holds_it` |
| `mb_free_transform` | `layer_op free_transform` at 150 %, 15° on a painted square | Bounds grow; one more history state | `tst_control_editing::freeTransformScalesAndRotates` |
| `mb_free_transform_busy` | Open a Free Transform, then call `free_transform` | `refused`; the session is still open | `tst_control_editing::freeTransformIsRefusedDuringASession` |
| `mb_free_transform_warp` | `layer_op free_transform` with `warp` `arc`, `bend` 50; then `bend` 150; then `warp` `nope` | Top edge rises, one more history state, no open session; `invalid_param`; `refused` | `tst_control_editing::freeTransformWarpsTheLayer` |
| `mb_catalogue` | `profile-tools.py --self-check` with a built app | Every M1–M14 row `outcome: ok` at 512² | `profile-tools::self_check_catalogue` |
