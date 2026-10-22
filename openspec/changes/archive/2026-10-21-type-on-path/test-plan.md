# Test plan: type on a path

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| path_click | Create a horizontal two-anchor Work Path, select Horizontal Type, click its middle, type `abc` | Session active, caret stops on the path, preview has ink, no history state before commit | `tst_type_on_path::clickOnOpenPathStartsOnPath` |
| path_start_click | Lay `abcd` out on a horizontal path with `start` at half its length | First pen is at the half-way point; later pens advance in anchor order | `pictura-render::path_text_starts_at_click_arc` |
| path_hit | Probe a closed path 1 px from its edge and at its centre | Edge probe returns on-path with the nearest arc; centre returns area | `pictura-render::path_hit_prefers_edge_then_inside` |
| path_overflow | Type a long word on a short path | Overlay overflow is true, marker is at the path end, committed rect stays on the path ink | `tst_type_on_path::overflowPlusAtPathEnd` |
| path_closed_wrap | Lay text longer than the remaining arc but shorter than the circumference on a closed path | Glyphs wrap past the start and overflow is false; text longer than the circumference overflows | `pictura-render::path_text_wraps_closed_path` |
| area_click | Create a closed rectangular Work Path, click its centre, type wide text, commit | One area type layer, text wraps in the path box, no ink outside the path | `tst_type_on_path::clickInsideClosedPathMakesAreaType` |
| area_clip | Render a word in a narrow triangular closed path | Every painted pixel is inside the triangle; a rectangle paints excluded corners | `pictura-render::area_text_clips_to_closed_path` |
| path_visible | Draw a Work Path, select Horizontal Type, hover its middle | Path anchors stay visible and the pointer zone is the type-on-path cursor | `tst_type_on_path::clickOnOpenPathStartsOnPath` |
| path_transform | Commit text on a path, move the layer, then scale it with Free Transform | Stored path points follow the move; the layer stays on-path type and the size grows | `tst_type_on_path::moveAndTransformOnPath` |
| path_commit | Type on a path, commit, click the layer, type more, commit | One state per commit, layer stays a path type layer, text changes | `tst_type_on_path::commitAndReopenOnPath` |
| path_mask | Commit Horizontal Type Mask on a path | Selection contains path glyphs and no off-path pixels | `tst_type_on_path::typeMaskFollowsPath` |
| path_psd | Write a document with one on-path and one area type layer, read it back; then read the authored PSD with ag-psd | Kind, closed flag, control points, and start offset match; ag-psd sees the same text path | `pictura-codec::text_path_round_trips`, `pictura-codec::text_path_survives_psd`, `pictura-codec::ag_psd_reads_authored_text_path`, `tst_type_on_path::saveAndReopenKeepsPath` |
| path_psd_read | Decode a `Txt2` `TextFrameSet` with an on-path and an area entry | On-path start fraction/control points and area closed points decode | `pictura-codec::reads_text_frame_set_entries` |
