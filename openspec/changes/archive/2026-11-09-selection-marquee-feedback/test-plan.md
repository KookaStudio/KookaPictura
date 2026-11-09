# Test plan

One row per scenario in the change's delta specs. `manual:` rows name the
check and why it is not automated.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| Drag a rectangular selection | Drag the Rectangular Marquee and release | The selection covers the dragged rectangle | `tst_selection_preview::liveOutlineMatchesTheCommittedOne` |
| Drag an elliptical selection | Drag the Elliptical Marquee past the canvas edge | The ellipse is inscribed in the drag rectangle and closed along the border | `tst_selection_preview::ellipticalMarqueePastTheEdgeClosesAlongTheBorder` |
| Tool-time feather softens the boundary | Rasterise a feather > 0 | Boundary coverage is strictly between 0 and 255 | `pictura-select::feather_makes_ramp` |
| Fixed ratio constrains the geometry | Drag with Fixed Ratio 2:1 | W/H is 2 within 1 px | `pictura-select::marquee_rect_fixed_ratio_landscape` |
| lst_fixed_ratio_fits_drag | Drag 40x10 with Fixed Ratio 2:1 | Selection is 20x10 | `pictura-select::marquee_rect_fixed_ratio_fits_drag` |
| lst_fixed_ratio_narrow | Drag 10x40 with Fixed Ratio 2:1 | Selection is 10x5 | `pictura-select::marquee_rect_fixed_ratio_narrow` |
| lst_fixed_ratio_one_axis | Drag 0x40 with Fixed Ratio 2:1 | Selection is 80x40 | `pictura-select::marquee_rect_fixed_ratio_one_axis` |
| lst_fixed_ratio_reverse | Drag (50,50)->(10,10) with Fixed Ratio 2:1 | Selection is (10,30,40,20) | `pictura-select::marquee_rect_fixed_ratio_reverse` |
| Fixed size ignores the drag extent | Drag with Fixed Size 100x50 | Size stays 100x50; top-left follows the pointer | `pictura-select::marquee_rect_fixed_size_follows_the_pointer` |
| lst_fixed_size_follows | Fixed Size 100x50 on a large document, release at (120,80) | Selection is 100x50 at (120,80) | `tst_selection_preview::fixedSizeFollowsTheCursor` |
| lst_fixed_size_clip | Fixed Size 100x50 on 64x64, release at (20,20) | Selection is clipped to the canvas | `tst_selection_preview::fixedSizeClipsAtTheCanvasEdge` |
| lst_elliptical_fixed_ratio | Elliptical Fixed Ratio 2:1, drag 40x10 | Ellipse inscribed in 20x10 | `tst_selection_preview::ellipticalStyleIsIndependent` |
| lst_elliptical_fixed_size | Elliptical Fixed Size 100x50 on a large document | Ellipse inscribed in 100x50 at the release point | `tst_selection_preview::ellipticalStyleIsIndependent` |
| Anti-alias is honest about the ceiling | Inspect the Elliptical Marquee options bar | Anti-alias is visible, disabled, and carries a reason | manual: options-bar inspection; the disabled state has no automated hook |
| Add extends an existing selection | Add mode, overlapping drag | Union of the old and new coverage | `pictura-select::combine_with_each_mode` |
| Subtract removes coverage | Subtract mode, overlapping drag | New coverage is excluded | `pictura-select::combine_with_each_mode` |
| Intersect keeps the overlap | Intersect mode, overlapping drag | Only the overlap remains | `pictura-select::combine_with_each_mode` |
| New replaces the selection | New mode, drag | Previous selection is discarded | `pictura-select::combine_with_each_mode` |
| Shift adds mid-gesture | Shift-drag with an existing selection | The committed result is a union | manual: modifier-to-mode mapping is covered by the self-test `tsc_selection_alt_pivot` |
| Alt subtracts mid-gesture | Alt held at press with an existing selection | The committed result excludes the drag | `tst_selection_alt::firstAltSubtractsAndDoesNotMirror` |
| Shift+Alt intersects | Shift+Alt-drag with an existing selection | The committed result is the overlap | manual: modifier-to-mode mapping is covered by the self-test `tsc_selection_alt_pivot` |
| The quick mode is locked for the gesture | Start with Shift, release before mouse-up | The result stays Add | manual: the self-test `tsc_selection_alt_pivot` checks the locked mode |
| No existing selection ignores the combine modifier | Alt-drag with no selection | From-centre geometry, committed New | `tst_selection_alt::altWithNoSelectionDrawsFromTheCentre` |
| lst_combine_cursor | Hover a selection with Shift or Alt | The combine cursor is shown | manual: the self-test `tsc_cursor_modifiers` checks the cursor id |
| lst_alt_no_pretoggle | Hold Alt over the canvas without a drag | The locked quick mode is unchanged | manual: the self-test `tsc_selection_alt_pivot` checks the pre-toggle rule |
| lst_alt_drag_mirror_combine | Start a drag with no Alt, press Alt mid-drag | Mirror geometry, combine mode unchanged | `tst_selection_alt::altPressedDuringDragMirrors` |
| Shift makes a square | Shift-drag the Rectangular Marquee | The box is square | `pictura-select::marquee_rect_shift_squares` |
| Shift makes a circle | Shift-drag the Elliptical Marquee | The ellipse is a circle in a square box | manual: the ellipse rasteriser is unchanged; the square box is covered by `pictura-select::marquee_rect_shift_squares` |
| Alt draws from the centre | Alt-drag with no selection | The rect is centred on the press point | `pictura-select::marquee_rect_mirror` |
| lst_marquee_dragmods | Start with Shift, release before mouse-up | The captured square constraint stays | manual: Qt drag check; no existing test releases Shift mid-drag |
| The size readout tracks the drag | Drag the marquee | The `W x H` readout follows and clears on release | manual: visual readout check |
| lst_alt_press_subtract | Alt held at press with an existing selection | Subtract, no mirror | `tst_selection_alt::firstAltSubtractsAndDoesNotMirror` |
| lst_alt_drag_mirror | Start with no Alt, press Alt mid-drag | Mirror geometry | `tst_selection_alt::altPressedDuringDragMirrors` |
| lst_fixed_ratio_modifiers | Fixed Ratio with Shift or Alt | Ratio kept; no square/mirror/centre | `pictura-select::marquee_rect_fixed_ratio_ignores_modifiers` |
| lst_fixed_size_modifiers | Fixed Size with Shift or Alt | Entered size at the release point; no square/mirror/centre | `pictura-select::marquee_rect_fixed_size_ignores_modifiers` |
| lst_elliptical_style_independent | Set the rectangle to Fixed Size, select the ellipse | The ellipse stays Normal | `tst_selection_preview::ellipticalStyleIsIndependent` |
| lst_rect_style_independent | Set the ellipse to Fixed Ratio, select the rectangle | The rectangle stays Normal | `tst_selection_preview::ellipticalStyleIsIndependent` |
| lst_elliptical_fixed_style | Set the ellipse to Fixed Ratio then Fixed Size | Each elliptical drag uses its own style; the rectangle is unchanged | `tst_selection_preview::ellipticalStyleIsIndependent` |
| lst_polygon_close_zoom | 200% zoom, three vertices, close click 4 px from the first | The outline stays open | `tst_selection_preview::polygonalCloseRadiusScalesWithZoom` |
| lst_polygon_close_zoom_hit | 200% zoom, three vertices, close click 3 px from the first | The outline closes and commits | `tst_selection_preview::polygonalCloseRadiusScalesWithZoom` |
| lst_polygon_double_click_zoom | 200% zoom, three vertices, double-click 3 px from the last | The outline closes and commits | `tst_selection_preview::polygonalDoubleClickRadiusScalesWithZoom` |
| lst_polygon_double_click_zoom_miss | 200% zoom, three vertices, double-click 4 px from the last | The outline stays open | `tst_selection_preview::polygonalDoubleClickRadiusScalesWithZoom` |
| uistalt_first_subtract | Alt held at the press with an existing selection | Subtract, no mirror | `tst_selection_alt::firstAltSubtractsAndDoesNotMirror` |
| uistalt_drag_mirror | Alt first pressed during the drag | Mirror, no subtract, cursor unchanged | `tst_selection_alt::altPressedDuringDragMirrors` |
| uistalt_second_mirror | Release and press Alt again after an Alt-at-press subtract | Mirror | `tst_selection_alt::secondAltMirrorsAboutTheAnchor` |
| uistalt_no_selection_centre | Alt at press with no selection | From-centre geometry, committed New | `tst_selection_alt::altWithNoSelectionDrawsFromTheCentre` |
| uistalt_marquee_options_persist | Set the rectangle Fixed Size and the ellipse Fixed Ratio, save and reload | Both restore their own options | `tst_tool_options_persistence::ellipticalOptionsRoundTrip` |
