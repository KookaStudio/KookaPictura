# Test plan

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `vp_menu` | Check the row on 8-bit, then after 16 Bits/Channel | Alt+Ctrl+V, one slot; enabled then disabled | `tst_vanishing_point::menuFollowsDepth` |
| `vp_cancel` | Draw a plane, Render Grids on, Cancel; reopen | Image and history unchanged; no planes | `tst_vanishing_point::cancelChangesNothing` |
| `vp_ok_render` | Draw a plane, Render Grids on, OK | One "Vanishing Point" state; change inside the plane only | `tst_vanishing_point::okRendersGridsAsOneState` |
| `vp_ok_planes` | Draw a plane, OK with Render Grids off; reopen; new document | No state; reopens with the plane; new document has none | `tst_vanishing_point::okWithoutRenderKeepsPlanesOnly` |
| `vp_selection` | Render with a selection over half the plane | Change inside the selection only | `tst_vanishing_point::renderIsClippedToTheSelection` |
| `vp_last_settings` | Commit two planes with Render Grids, delete them later, run Last Filter Settings | Two planes, Render Grids on | `tst_vanishing_point::lastSettingsRestoresPlanes` |
| `vp_last_depth` | Commit, convert to 16 Bits/Channel, run Last Filter Settings and Last Filter | No dialog, no state, refusal names 8-bit | `tst_vanishing_point::lastFilterRefusesOffEightBit`, `pictura-app::commits_are_refused_off_eight_bit_rgb_and_gray` |
| `vp_ok_pixels` | Paint a Brush stroke in a plane, OK; separately paint and Cancel | One state, change inside the plane only; Cancel changes nothing | `tst_vanishing_point_retouch::okAppliesAStrokeAsOneState` |
| `vp_controls` | Open on a document with no planes | Nine tools in order without Measure, Grid Size 100 in 1–1000, Angle disabled, Show Edges on, Render Grids off, Create Plane selected | `tst_vanishing_point::opensWithControls` |
| `vp_create` | Click three corners, Backspace, two more; separately Esc after two | One valid plane; Edit Plane selected | `tst_vanishing_point::fourClicksCreateAPlane` |
| `vp_drag` | Drag a corner, then the inside | Corner follows; move shifts all corners | `tst_vanishing_point::dragReshapesAndMoves` |
| `vp_tear_off_ui` | Ctrl-drag an edge node, set Angle 45, select the parent | Second plane at 90 then 45; Angle disabled on the parent | `tst_vanishing_point::ctrlDragTearsOffAndAngleTurns` |
| `vp_cycle_ui` | Ctrl-click an overlap, press Delete | Active plane changes; Delete removes it | `tst_vanishing_point::ctrlClickCyclesAndDeleteRemoves` |
| `vp_show_edges` | Press Ctrl+H twice | Planes hidden, then shown | `tst_vanishing_point::showEdgesHidesOverlays` |
| `vp_undo_ui` | Make a plane, Ctrl+Z, Ctrl+Shift+Z | Plane gone, then back | `tst_vanishing_point::undoRedoPlaneEdits` |
| `vp_grid_size` | Grid Size 100 → 50 | More grid lines | `tst_vanishing_point::gridSizeChangesTheGrid` |
| `vp_tool_keys` | Press C, M, S, B, T, I, H, Z, V | Each selects its tool | `tst_vanishing_point::toolShortcuts` |
| `vp_flat` | Map points on a 90° torn-off plane into flat space and back | Distance past the hinge kept; round trip within 1e-6 px | `pictura-filters::a_hinge_unfolds_into_the_flat_space` |
| `vp_inside` | Paint off a plane; drop an oversized float | Nothing outside the plane changes | `pictura-filters::edits_stay_inside_planes` |
| `vp_paste` | Paste 40×30 and 1000×1000 into a square-on plane; paste with no plane | Centred at one texel per unit; large one shrunk to 80 %; no-plane paste refused | `pictura-filters::paste_places_a_float_in_the_active_plane` |
| `vp_float_perspective` | Drop a solid float on a square-on plane; place one on a tilted plane | Coverage within 2 % of the texel area; tilted footprint edges run through the vanishing points | `pictura-filters::a_float_follows_the_planes_perspective` |
| `vp_float_hinge` | Drag a float onto the plane torn off its edge | Covers both planes, continuous at the hinge | `pictura-filters::a_dragged_float_wraps_across_a_hinge` |
| `vp_transform` | Scale a float by 2, rotate it, flip it | Fourfold footprint; rotated and flipped texels land where expected | `pictura-filters::transform_scales_rotates_and_flips` |
| `vp_drop` | Drop white over black at 50 % opacity; another with 8 px feather | Half cover; full inside, fading over the edge | `pictura-filters::dropping_uses_opacity_and_feather` |
| `vp_heal` | Drop flat red onto flat grey with Heal Off, Luminance, On | Red; red hue at grey's lightness; grey | `pictura-filters::heal_modes_blend_as_stated` |
| `vp_marquee` | Draw a marquee, Alt-drag it; draw another, drag (Destination), Ctrl-drag | Copy floats, source untouched; marquee moves only; marquee filled from under the pointer | `pictura-filters::the_marquee_copies_moves_and_fills` |
| `vp_marquee_plane` | Double-click inside a plane | The marquee is the plane's rect | `pictura-filters::a_double_click_selects_the_whole_plane` |
| `vp_stamp` | Set a source, paint aligned twice, then unaligned | Texture copied at the offset; unaligned restarts at the source | `pictura-filters::the_stamp_clones_in_perspective` |
| `vp_brush` | One dab on a square-on plane and one on a receding floor | Round, then flatter vertically | `pictura-filters::brush_dabs_foreshorten` |
| `vp_pixel_undo` | Paint, undo, redo | Bit-identical restore; stroke back | `pictura-filters::pixel_edits_undo_and_redo` |
| `vp_patch` | Replay a session's edit on the original pixels, with and without grids | Equals the working copy; grids over the pixels | `pictura-filters::the_commit_replays_the_sessions_pixels` |
| `vp_eyedropper` | Sample a painted pixel and a backdrop pixel | The colours the preview shows | `pictura-filters::the_eyedropper_picks_from_the_preview` |
| `vp_paste_ui` | Ctrl+V with no plane; with a plane; Ctrl+T | Refusal on the help line; a floating selection in the preview; Transform selected | `tst_vanishing_point_retouch::ctrlVPastesIntoThePlane` |
| `vp_paste_commit` | Paste, drag the float, OK | One state; pasted colour inside the plane only, where it was dragged | `tst_vanishing_point_retouch::pastedFloatCommitsOnOk` |
| `vp_options` | Select each tool | Its options with the stated ranges and defaults | `tst_vanishing_point_retouch::optionsFollowTheTool` |
| `vp_stamp_ui` | Stamp with no source; Alt-click a source and paint; Eyedropper the result | Help line asks for a source; painted area takes the source colour; Brush Color becomes it | `tst_vanishing_point_retouch::stampAndEyedropperWork` |
| `vp_brush_keys` | Press `]` then `[` with the Brush | Diameter 100 → 150 → 100 | `tst_vanishing_point_retouch::bracketsChangeDiameter` |
| Visual match to CS6 | Clone, paste and paint the same edits on the same photos in CS6 and Pictura | Comparable perspective, foreshortening and heal results | manual: no CS6 oracle; the heal and stroke blends approximate closed algorithms |
| Interactive feel | Drag a large float and paint long strokes on a 24 MP photo | The preview follows the pointer | manual: responsiveness on real hardware is not observable offscreen |
| System clipboard paste | Copy an image in another application, then Ctrl+V in the dialog | The other application's image floats in the plane | manual: the offscreen platform has no other application to own the clipboard |
