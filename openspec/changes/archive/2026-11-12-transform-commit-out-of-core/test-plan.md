# Test plan

This plan covers `document/out-of-core-transform` and
`compositing/tiled-region-composite` for `transform-commit-out-of-core` (#1290,
phases 3–4).

**Fixtures.** Engine tests use seeded layers. The dense resampler stays as a
`#[cfg(test)]` oracle (`dense_resample`), and so does the flat banded path.
Gate tests pin the governor's host reading (`memory::set_host_for_test`) and the
scratch volumes' available bytes (`memory_gate::set_scratch_available_for_test`).
The spill test configures a temporary scratch volume and runs
`eviction::run_pass`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `oct_resample_exact` | For scale 0.5, scale 1.5 + 17°, scale 0.02 and a perspective quad, each interpolation: transform a seeded masked layer; compare with `dense_resample` | Channels and mask tiled; bytes equal | `pictura-render::tiled_resample_matches_the_dense_resampler` |
| `oct_resample_deep` | Scale a 16-bit layer with a retained store by 1.5, rotate 10° | Retained planes tiled and equal the typed oracle | `pictura-render::deep_planes_resample_through_tiles` |
| `oct_uniform_tiles` | Scale a solid 2048² layer by 1.5 | `allocated_bytes` < ¼ of the dense size per channel | `pictura-render::a_solid_layer_scales_into_uniform_tiles` |
| `oct_output_spills` | Rotate a 1024² layer 10° with a temp scratch volume; run an eviction pass that spills | Some tiles spilled; layer reads back equal to the oracle | `pictura-render::transformed_tiles_spill_and_read_back` |
| `oct_cancel` | Transform under an already-cancelled job | Cancelled; rect and pixels unchanged | `pictura-render::a_cancelled_transform_leaves_the_document` |
| `tcm_gate_move` | Commit cost of a 10 × 10 rect moved by (5, 0) and (5.5, 0) | Spillable `4·150` and `4·110 + 4·160` | `pictura-app::a_whole_pixel_move_spills_only_the_composite` |
| `oct_gate_spills` | Gate a 17000² full-canvas ×1.5 commit with 1.75 GiB room, 64 GiB scratch | Admitted | `pictura-app::a_large_resize_is_admitted_when_scratch_holds_it` |
| `oct_gate_scratch_full` | Same with no scratch available; build the transform refusal message | Refused; scratch-full recorded; message names the scratch disks | `pictura-app::full_scratch_disks_refuse_the_transform`, `tst_memory_governor::transformRefusalNamesFullScratchDisks` |
| `oct_gate_working` | Gate with room below the working set, ample scratch | Memory refusal; scratch-full not recorded | `pictura-app::a_working_set_beyond_ram_is_a_memory_refusal` |
| `trc_exact` | Composite the banded fixture whole (over the test threshold) and a sub-region (under it) | Whole region tiled and equal to the one-canvas composite; sub-region flat | `pictura-render::a_tiled_region_equals_the_flat_region` |
| `trc_patch_tiled` | Patch a tiled 512² region into a tiled composite | Pixels equal; `materializations()` unchanged | `pictura-render::patching_a_tiled_region_does_not_materialize` |
| `trc_srgb_lazy` | Land a whole-document region of an Adobe RGB document over the sync tile limit through the deferred landing | Region equals the composite; tiles invalidated; display conversion count unchanged | `pictura-app::a_deferred_whole_landing_skips_the_display_conversion` |
| 17000² resize under 2 GiB | Open a 17000 × 17000 single-layer document under `systemd-run --user -p MemoryMax=2G`; Free Transform ×0.5, ×1.0 with 15° rotation, ×1.5; commit each | No refusal, no OOM kill, progress window advances; result matches the same commit on an unconstrained run (compare saved PSDs) | manual: needs a 289 MP document, a cgroup limit and several GB of free scratch disk, too large and host-dependent for CI |
| 17000² perspective and distort under 4 GiB | Same document under `MemoryMax=4G`; Perspective and Distort commits | No refusal, no OOM kill | manual: same reason |
| Full scratch disk | Point the only scratch volume at a nearly full filesystem; commit a 17000² ×1.5 | Refused with the scratch-disk message; document and history unchanged | manual: needs a real nearly full filesystem |
