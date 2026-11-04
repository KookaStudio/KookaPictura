# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `mq_bounds_once` | Select 4×4 at (2,2), refresh the Info panel, move the cursor five times; select 6×3 and refresh | One bounds compute across the moves, W/H `4 × 4`; one more compute, W/H `6 × 3` | `tst_info_panel::cursorMovesReuseTheSelectionBounds` |
| `mq_caches_follow_revision` | Read bounds and count twice; replace the selection; preview a move by (3,2); clear | One compute per revision; new bounds and count; offset bounds; none and 0 | `pictura-app::selection_bounds_and_count_follow_the_revision` |
| `mq_contour_window` | Trace pseudo-random masks, an edge-touching mask, Select All and an empty mask within their bounds and whole | Same drawn edge sets | `pictura-select::tracing_within_the_bounds_matches_the_whole_mask` |
| `mq_shape_masks` | Build rects and ellipses inside, across each edge, outside, and with zero or negative size | Every pixel equals the per-pixel definition | `pictura-select::rects_and_ellipses_match_their_per_pixel_shapes` |
| `mq_cursor_cached` | Request `cursor.moveSelection` twice at one hotspot and once at another | Same pixmap cache key; the other hotspot differs | `tst_icon_assets::cursorIsBuiltOnce` |
| `mq_large_manual` | Real display, 16507×16196 document, `PICTURA_PAINT_TIMING=1`: choose Marquee, drag and release a rectangle, hover in and out | No full-mask stall on switch, release or hover; ants and W/H match | manual: the full-size document does not fit the test runners' memory and the stall is felt on a real display; `pictura-app::marquee_selection_profile` times the stages at a divisor |
