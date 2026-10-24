# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `vp_square_on` | Add a 400 px square centred in an 800×600 document | Valid; three inner grid lines each way at x = 300, 400, 500 | `pictura-filters::a_square_on_rectangle_is_valid_with_an_even_grid` |
| `vp_red` | Ask the validity of a crossed, a concave and a collinear quad | Each red | `pictura-filters::crossed_concave_and_degenerate_quads_are_red` |
| `vp_yellow` | Add a convex quad whose vanishing points both lie up-right of the centre | Yellow; no grid | `pictura-filters::a_quad_no_camera_sees_as_a_rectangle_is_yellow` |
| `vp_camera` | Add the image of a tilted 3D rectangle through a known camera (f = 1.7) | Focal length recovered within 1e-6; every grid line runs through a vanishing point | `pictura-filters::a_projected_rectangle_recovers_its_camera_and_grid_converges` |
| `vp_edge_resize` | Drag a tilted plane's edge node outward | Still valid; more lines; every earlier line unchanged or lengthened along itself | `pictura-filters::resizing_an_edge_leaves_the_grid_on_the_surface` |
| `vp_hits` | Hit-test corners, edge nodes, an overlap and ground with two planes | Active plane's nodes first; overlap picks the active plane; ground none | `pictura-filters::hits_prefer_nodes_then_the_newest_plane` |
| `vp_editor` | Create two overlapping planes, cycle twice, delete, undo, redo, undo to empty | Alternating cycle; delete removes the active plane; undo/redo step to empty | `pictura-filters::the_editor_creates_cycles_deletes_and_undoes` |
| `vp_tear_off` | Tear a plane off edge 1 and drag it out | Valid, 90°, perpendicular normals, shares the parent's corners 1 and 2 | `pictura-filters::a_torn_off_plane_is_perpendicular_and_shares_the_edge` |
| `vp_tear_off_undo` | Ctrl-drag an edge node through several moves, undo; Alt-drag a root; tear off a red plane | New plane active; one undo removes it; the two gestures refused | `pictura-filters::a_tear_off_drag_is_one_undo_step` |
| `vp_angle` | Set 45°, 0°; aim the far edge node at its 60° position | Normals at 45°, then parallel; aim returns 60° ± 0.1° | `pictura-filters::the_angle_turns_a_related_plane_about_its_hinge` |
| `vp_angle_lock` | Tear a grandchild off a related plane | Root and middle plane cannot turn; grandchild can | `pictura-filters::a_plane_with_related_planes_cannot_turn` |
| `vp_follow` | Move the parent's corner, then cross the parent | Related plane stays on the hinge; then red with its outline frozen | `pictura-filters::related_planes_follow_their_parent` |
| `vp_slide` | Drag a related plane's inside | It moves; hinge corners stay on the parent edge line | `pictura-filters::a_slid_related_plane_stays_on_its_hinge` |
| `vp_grid_continues` | Turn a related plane to 0° | Each parent line ending on the hinge continues into the related plane | `pictura-filters::grid_lines_continue_across_a_flat_hinge` |
| `vp_remove` | Remove the first of two families | The other family stays, re-indexed and valid | `pictura-filters::removing_a_plane_removes_its_related_planes` |
| `vp_render` | Render a square-on grid into an offset opaque layer and a clear one | Lines and outline darken by half on each side of a boundary; cells unchanged; clear layer gets black, half-opaque lines | `pictura-filters::render_grids_draws_black_lines_into_the_layer` |
| `vp_render_empty` | Apply an empty edit, then one edit twice | Empty is bit-identical; two runs identical | `pictura-filters::an_empty_edit_changes_nothing_and_runs_are_deterministic` |
| `vp_kept_edit` | Map ids 0, none, kept, evicted, unknown, wrong arity | 0 and none empty; kept maps; the rest refused | `pictura-app::vanishing_point_names_a_kept_edit_and_refuses_one_it_no_longer_has`, `pictura-app::kept_edits_are_found_by_id_and_zero_draws_nothing` |
| `vp_menu` | Check the row on 8-bit, then after 16 Bits/Channel | Alt+Ctrl+V, one slot; enabled then disabled | `tst_vanishing_point::menuFollowsDepth` |
| `vp_cancel` | Draw a plane, Render Grids on, Cancel; reopen | Image and history unchanged; no planes | `tst_vanishing_point::cancelChangesNothing` |
| `vp_ok_render` | Draw a plane, Render Grids on, OK | One "Vanishing Point" state; change inside the plane only; Last Filter `vanishing-point` | `tst_vanishing_point::okRendersGridsAsOneState` |
| `vp_ok_planes` | Draw a plane, OK with Render Grids off; reopen; new document | No state; image unchanged; reopens with the plane and Render Grids off; new document has none | `tst_vanishing_point::okWithoutRenderKeepsPlanesOnly` |
| `vp_selection` | Render with a selection over half the plane | Change inside the selection only | `tst_vanishing_point::renderIsClippedToTheSelection` |
| `vp_last_settings` | Commit two planes with Render Grids, delete them in a later session, run Last Filter Settings | Two planes, Render Grids on | `tst_vanishing_point::lastSettingsRestoresPlanes` |
| `vp_last_depth` | Commit, convert to 16 Bits/Channel, run Last Filter Settings and Last Filter; check every mode's gate | No dialog, no state, refusal names 8-bit; only 8-bit RGB and Grayscale pass | `tst_vanishing_point::lastFilterRefusesOffEightBit`, `pictura-app::commits_are_refused_off_eight_bit_rgb_and_gray` |
| `vp_controls` | Open on a document with no planes | Title, four tools without Measure, Grid Size 100 in 1–1000, Angle disabled, Show Edges on, Render Grids off, Create Plane selected | `tst_vanishing_point::opensWithControls` |
| `vp_create` | Click three corners, Backspace, two more; separately Esc after two | One valid plane; Edit Plane selected; Esc leaves nothing pending | `tst_vanishing_point::fourClicksCreateAPlane` |
| `vp_drag` | Drag a corner, then the inside | Corner follows; move shifts all corners by the drag | `tst_vanishing_point::dragReshapesAndMoves` |
| `vp_tear_off_ui` | Ctrl-drag an edge node, set Angle 45, select the parent | Second plane active, Angle enabled at 90 then 45; disabled on the parent | `tst_vanishing_point::ctrlDragTearsOffAndAngleTurns` |
| `vp_cycle_ui` | Ctrl-click an overlap of two planes, press Delete | Active plane changes; Delete removes it | `tst_vanishing_point::ctrlClickCyclesAndDeleteRemoves` |
| `vp_show_edges` | Press Ctrl+H twice | Planes hidden, then shown | `tst_vanishing_point::showEdgesHidesOverlays` |
| `vp_undo_ui` | Make a plane, Ctrl+Z, Ctrl+Shift+Z | Plane gone, then back | `tst_vanishing_point::undoRedoPlaneEdits` |
| `vp_grid_size` | Grid Size 100 → 50 | More grid lines on the active plane | `tst_vanishing_point::gridSizeChangesTheGrid` |
| `vp_tool_keys` | Press C, H, Z, V | Each selects its tool | `tst_vanishing_point::toolShortcuts` |
| Visual match to CS6 | Draw the same planes on the same photos in CS6 and Pictura, including torn-off planes at 90° and other angles | Comparable validity colours, grid spacing and hinge geometry | manual: no CS6 oracle; the plane solver is an approximation of a closed algorithm |
| Interactive feel | Drag corners, edges and tear-offs on a 24 MP photo | Grid and nodes follow the pointer without lag | manual: responsiveness on real hardware is not observable offscreen |
| Plane colours on screen | Draw valid, red and yellow planes over light and dark photos | Blue, red and yellow outlines and grids read clearly | manual: the suite checks each plane's validity code, not how the colours read |
