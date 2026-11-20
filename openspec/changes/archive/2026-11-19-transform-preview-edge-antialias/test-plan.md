# Test plan

One row per scenario in the change's delta specs. The first cell is the
scenario's tag.

| Scenario | Steps | Expected | Coverage |
| --- | --- | --- | --- |
| `ft_preview_edge` | New 100 × 100 white document; fill a black square at (20, 20, 40, 40); begin Free Transform; drag the bottom edge from y=60 to y=87.5; hide the transform box and sample the edge row; commit and sample the same row | The preview edge is an intermediate gray, not white or black, and matches the committed edge within 32 levels | `tst_transform_controls::previewEdgeMatchesTheCommittedResample` |

Manual checks: none.
