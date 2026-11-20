# Test plan

One row per scenario in the change's delta specs. The first cell is the
scenario's tag.

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| `ft_axis_aligned_hard_edge` | Scale a solid 64 × 64 layer 2× with no rotation through the engine | No alpha sample is fractional; the transformed edge stays fully opaque | `pictura-render::an_axis_aligned_scale_keeps_a_hard_edge` |
| `ft_preview_edge_rotated` | Rotate a solid layer 30° through the engine | At least one transformed edge sample has fractional alpha | `pictura-render::a_rotated_edge_is_antialiased`, `pictura-render::a_quad_rotated_edge_is_antialiased` |
| `ft_preview_edge` | New 100 × 100 white document; fill a black square; drag its bottom edge to a half-pixel position; hide the transform box; check the preview edge band; commit and check the same band | Every pixel in the band is black or white, never gray; the preview and commit both keep the hard edge | `tst_transform_controls::previewEdgeMatchesTheCommittedResample` |

Manual checks: none.
