# Test plan

This plan covers `document/transform-preview-memory` and
`tools/projective-mode-switch` for `transform-preview-memory` (#1297).

**Fixtures.** The preview threshold is pinned per thread with
`set_transform_preview_pixels_for_test` (a negative value restores 16 Mpx), so
a 40 × 30 layer stands in for a 5000 × 4000 one. Gate tests pin the governor's
host reading (`set_available_memory_for_test`). Mode-switch unit tests build a
`TransformSession` directly, as the skew and distort tests do.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `tpm_reduced` | Threshold 1000 px; build the preview of a plain 60 × 40 layer at (3, 5) on a 100² document | Level 1; image 29 × 19 at (4, 6) (the layer aligned inside itself to the 2-px grid) in the layer colour; base patch 31 × 21 at (2, 4) in the base colour | `pictura-app::a_large_plain_layer_previews_at_a_reduced_level` |
| `tpm_small_exact` | Threshold 2400 px; preview a plain 60 × 40 layer; then a 60 × 40 layer with a layer mask under threshold 1000 | Level 0 and a 60 × 40 image both times | `pictura-app::a_small_or_styled_layer_previews_at_full_size` |
| `tpm_draw_scaled` | Threshold 300 px; Free Transform a black 20 × 20 mark on a 40 × 30 layer; grab the canvas; repeat at the default threshold | Preview scale 2 then 1; the mark's interior is black and the canvas outside it white in both grabs | `tst_transform_overlay::reducedPreviewDrawsAtDocumentSize` |
| `tpm_warp_scaled` | Warp a 40 × 40 red square through a Distort matrix from the full image and from its 20 × 20 half with `sourceScale` 2 | The two outputs are the same image at the same origin, and the stretched corner is opaque | `tst_transform_distort::projectivePreviewSamplesAReducedImage` |
| `tpm_warp_level` | Warp the 12 × 9 mark with the Arc preset at the default threshold, then at a 50-px one | Preview scale 1, then 2; the reduced warped image is half the full one within the 2-px alignment | `tst_transform_warp::warpPreviewKeepsTheReducedLevel` |
| `tpm_bytes` | Threshold 1000 px on the 100² document: bytes of the 60 × 40 plain layer, of a 30 × 30 one, and of the 60 × 40 one lifted | `20·600`, `20·900`, and `20·2400 + 12·10000` | `pictura-app::preview_bytes_follow_the_level` |
| `tpm_gate` | Pin no room; begin Free Transform on a layer; read the status bar; unpin and begin again | No session; history count unchanged; memory refusal recorded; transform memory message shown | `tst_memory_governor::transformBeginRefusedBeyondTheRoom` |
| `pms_keep` | Free session with a move, 150 % width, 30° rotation; switch to Skew (unit) and choose Skew from the menu (Qt) | Mode `skew`; quad corners and reference point position unchanged; similarity reset | `pictura-app::switching_to_skew_keeps_the_box_and_the_reference_point`, `tst_transform_commands::choosingSkewMidSessionKeepsTheTransform` |
| `pms_commit` | Free session 150 % wide; dispatch Distort; commit | Layer rect 150 % wide; one history state | `tst_transform_commands::distortMidSessionCommitsThePendingScale` |
| `pms_type` | Session holding editable type; try Distort and Perspective (unit); with a Free session open on a type layer, refresh the registry (Qt) | Refused; mode stays Free; Distort disabled | `pictura-app::projective_switch_refusals`, `tst_transform_distort::distortCommandDisabledMidSessionOnType` |
| `pms_warp` | Session with a stored warp, and one editing a warp; switch to Skew | Refused | `pictura-app::projective_switch_refusals` |
| 17000² rotate then Skew | On a desktop session with about 8 GB of RAM: 17000² document, fill a 12000² square on a layer, move it, Free Transform 110 % and commit, rotate 30° and commit, then `Edit > Transform > Skew` and drag a corner, at fit-on-screen and at 100 % | No OOM kill; the preview draws (coarser at 100 %); the commit completes or is refused with the memory message | manual: needs a 289 MP document on a real display, where the canvas paints the projective preview (the offscreen QPA never paints it), and several GB of free RAM |
