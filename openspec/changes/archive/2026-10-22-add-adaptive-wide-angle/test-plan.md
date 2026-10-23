# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `awa_fisheye_disc` | Apply Fisheye 11.74 mm, crop 0.20, Scale 50 % to an opaque layer | An opaque disc of radius ≈ F about the centre; transparent outside | `pictura-filters::short_fisheye_leaves_a_disc` |
| `awa_perspective_corners` | Apply Perspective at a short focal length, Scale 150 % | Corners clear, edge midpoints covered, centre colour within 2 | `pictura-filters::perspective_pulls_corners_in` |
| `awa_spherical_identity` | Apply Full Spherical, Scale 100 %, no constraints, to a 2:1 layer | Every sample unchanged | `pictura-filters::full_spherical_is_identity_without_constraints` |
| `awa_scale` | Apply Full Spherical at Scale 50 % | About a quarter of the layer stays opaque, centred | `pictura-filters::scale_shrinks_about_the_centre` |
| `awa_background_fill` | Apply a blank-leaving correction to a three-plane buffer | Blank pixels are white | `pictura-filters::blank_areas_are_white_without_alpha` |
| `awa_deterministic` | Apply the same constrained parameters twice | Byte-identical outputs | `pictura-filters::constrained_runs_are_deterministic` |
| `awa_line_straight` | Add an Unfixed line off-centre on a fisheye layer | Trace bows > 8 px before, ≤ 1 px from its chord after | `pictura-filters::a_line_constraint_straightens_its_trace` |
| `awa_line_orientation` | Add a slanted Horizontal line; separately a slanted Vertical line | Samples share one row / one column within 1 px | `pictura-filters::oriented_lines_level_their_trace` |
| `awa_fixed_angle` | Fix a line at 30° | Output endpoints at 30° ± 0.5° | `pictura-filters::a_fixed_line_takes_its_angle` |
| `awa_polygon` | Add a four-corner polygon on a fisheye layer | Each edge within 1 px of its chord | `pictura-filters::a_polygon_straightens_its_edges` |
| `awa_invalid` | Add a zero-length line | Reported invalid; output equals the unconstrained output | `pictura-filters::an_invalid_constraint_changes_nothing` |
| `awa_menu` | Open an 8-bit RGB document, then convert a new one to 16 Bits/Channel | Enabled, then disabled | `tst_adaptive_wide_angle::menuFollowsDepth` |
| `awa_controls` | Open the workspace on an image without EXIF | Title, Correction items, ranges, As Shot disabled, view toggles, `--` camera line | `tst_adaptive_wide_angle::opensWithControls` |
| `awa_exif` | Open on `metadata.psd` (EXIF 50 mm, Make, Model) | 15.31, 3.27, As Shot enabled and checked, camera line names the camera | `tst_adaptive_wide_angle::exifPrefillsLens` |
| `awa_spherical_gate` | Open on a 3:2 layer, then a 2:1 layer | Full Spherical disabled, then enabled | `tst_adaptive_wide_angle::fullSphericalNeedsTwoToOne` |
| `awa_draw_line` | Drag with Constraint, then Shift-drag mostly horizontally | Two constraints: cyan unfixed, yellow horizontal; preview changed | `tst_adaptive_wide_angle::dragDrawsConstraints` |
| `awa_context_menu` | Pick Vertical, Arbitrary, Delete from a constraint's menu | Magenta, green, removed | `tst_adaptive_wide_angle::contextMenuOrientsAndDeletes` |
| `awa_polygon_tool` | Click four corners, press E, click a new fourth, click the first | One closed four-corner polygon | `tst_adaptive_wide_angle::polygonClicksClose` |
| `awa_preview_toggle` | Turn Preview off with a blank-leaving correction, then on | Unchanged layer shown, then the correction again | `tst_adaptive_wide_angle::previewToggleShowsSource` |
| `awa_show_mesh` | Turn Show Mesh on | A non-empty mesh path that follows the correction | `tst_adaptive_wide_angle::showMeshDrawsGrid` |
| `awa_move` | Drag 20 px with Move at 100 % | Offset slots record 20 px of the layer size | `tst_adaptive_wide_angle::moveToolOffsets` |
| `awa_tool_keys` | Press Y, M, H, Z, C | Each selects its tool | `tst_adaptive_wide_angle::toolShortcuts` |
| `awa_commit` | Draw and Cancel; draw and OK | Cancel changes nothing; OK adds one `adaptive-wide-angle` state and changes pixels | `tst_adaptive_wide_angle::cancelAndOk` |
| `awa_last_settings` | Commit Perspective, 120, one constraint; reopen with the slots | Perspective, 120, one constraint | `tst_adaptive_wide_angle::lastSettingsRestoresConstraints` |
| `awa_detail` | Hover two parts of the preview | The Detail picture differs | `tst_adaptive_wide_angle::detailFollowsPointer` |
| Visual match to CS6 | Correct the same fisheye and wide-angle photos in CS6 and Pictura with matching settings and constraints | Comparable straightening and blank-area shapes | manual: no CS6 oracle; the projection and solver are approximations of a closed algorithm |
| Interactive feel | Draw, drag handles and move sliders on a 24 MP photo | Re-solve and preview stay responsive | manual: responsiveness on real hardware is not observable offscreen |
| Constraint colours on screen | Draw unfixed, horizontal, vertical, fixed constraints | Cyan, yellow, magenta, green read clearly over the image | manual: the suite checks each constraint's state, not how the colours read |
