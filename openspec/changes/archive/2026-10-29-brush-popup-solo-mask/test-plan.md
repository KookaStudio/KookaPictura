# Test plan

One row per spec scenario in the delta. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `cp_mask_thumb_bounded` | Sample the mask thumbnail of a 4096² document at 24 px, counting `mask_sample` calls | One sample per thumbnail pixel (≤ 24×24), never a document scan | `pictura-app::mask_thumbnail_samples_touch_only_the_grid` |
| `cp_mask_thumb_unchanged` | Thumbnail a 6×4 document whose mask rectangle is 2×2, comparing against the document-plane nearest-neighbour downsample | Identical pixels, including the off-rect `default_color` | `pictura-app::mask_thumbnail_samples_match_the_full_plane_downsample` |
