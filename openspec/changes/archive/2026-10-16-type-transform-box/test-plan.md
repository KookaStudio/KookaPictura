# Test plan

One row per spec scenario. Coverage names the automated test, or
`manual: <reason>` when it cannot be automated. Checked by
`scripts/check-test-plan.py`.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `ttb_frame_math` | Quarter-turn rotate; corner scale; edge resize; pivot drag | 90°, pivot fixed; opposite corner fixed, axes stretched; one edge moved; pivot moved by the drag | `tst_paragraph_type::frameGestureMath` |
| `ttb_rotate` | Type in a box; hover outside a corner; drag about the pivot; release; commit | Rotate zone; angle readout, cleared on release; box turned; no history before commit; layer rotated | `tst_paragraph_type::boxRotatesWithReadout` |
| `ttb_resize_readout` | Drag a paragraph box corner | Handle zone; W over H readout | `tst_paragraph_type::resizeShowsSizeReadout` |
| `ttb_pivot` | Drag the reference point by (10, 6) | Pivot zone; moved by (10, 6); X over Y readout | `tst_paragraph_type::pivotDrags` |
| `ttb_ctrl_scale` | Point type; hold Ctrl; drag the bottom-right handle out; commit | Box only with Ctrl; layer scaled up | `tst_paragraph_type::ctrlScalesPointType` |
| On-screen box and cursors | On a real display: hover each handle, outside a corner, the reference point; rotate and resize | Resize, rotate, and four-way cursors; the ring-and-cross reference point; readouts follow the pointer | manual: cursor shapes and on-screen drawing need a real display; zones are automated above |
