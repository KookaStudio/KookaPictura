# Test plan

This plan covers the added `document/transform-commit-memory` for
`transform-commit-peak-memory` (#1290, phases 1–2).

**Fixtures.** Engine tests use a seeded 300 × 200 layer, tiled with
`Plane::tile` so that plane sharing can be observed with `Plane::shares`. The
layer carries a seeded mask over a rect offset from the layer rect. The
allocation tests arm `memory::fail_next_plane_for_test`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tcm_move_shares_tiles` | Move the tiled, masked layer by (37, −21) with Bilinear | Every channel `shares` its source plane; layer and mask rects shifted by (37, −21); pixels equal | `pictura-render::an_integer_move_shares_the_source_planes` |
| `tcm_move_matches_resample` | For each interpolation except Smoother, run `transform_layer_with_map` with `pure_translate` true and then false on clones | Same rect; channel and mask bytes identical | `pictura-render::an_integer_move_matches_the_resampled_result` |
| `tcm_session_move_exact` | `apply_session_transform` with identity linear part, pivot (0.1, 29.3), offset (7, −3) on a 40 × 30 layer | Rect moved by (7, −3); every channel `shares` its source plane | `pictura-app::a_move_about_an_off_centre_pivot_keeps_the_planes` |
| `tcm_smoother_resamples` | Move by (4, 9) with Bicubic Smoother | Channels do not share planes with the source | `pictura-render::a_smoother_move_still_resamples` |
| `tcm_resample_fallible` | Arm the failpoint, rotate the layer 10° | Returns false; rect and pixels unchanged; `memory::last_refusal()` is set | `pictura-render::a_failed_resample_allocation_refuses_the_transform` |
| `tcm_move_no_alloc` | Arm the failpoint, move by (3, 3) | Succeeds; the next `try_plane` still fails | `pictura-render::an_integer_move_allocates_no_resample_buffer` |
| `tcm_gate_move` | `commit_bytes` of a 10 × 10 rect moved by (5, 0) and (5.5, 0) | `12·150` and `4·110 + 12·160` | `pictura-app::commit_bytes_charges_a_move_only_for_the_composite` |
| Large-document move on a limited host | Open a 17000 × 17000 document under `systemd-run --user -p MemoryMax=4G`, Free Transform the full-canvas layer, drag to move by whole pixels, commit | No refusal, no OOM kill; the moved layer is correct | manual: needs a 289 MP document and a cgroup memory limit, too large and host-dependent for CI |
