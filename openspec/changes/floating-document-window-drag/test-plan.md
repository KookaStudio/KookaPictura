# Test plan

Every automated check is a Qt Test case in `tst_document_float`, run on the
offscreen platform. Nothing is added to the self-test. Each test function
builds its own frame, so the suite stays sharded.

`DocumentFloatWindow::setWindowDragForTest(true)` makes a top-level float act
as it does on Wayland: a plain header drag and a tear-out start the carried
window drag. `setWindowDockDragForTest` receives that drag's MIME data in place
of the blocking `QDrag::exec`. The cases then send synthetic
`QDragEnterEvent`, `QDragMoveEvent`, `QDragLeaveEvent`, and `QDropEvent` with
that data to the tab bar or the tab widget, and find the dock line by its
object name, `documentDockIndicator`. Offscreen can't run a platform drag, so
the compositor moving the window with the pointer is a manual row.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dtw_tear_out` | Two tabs; drag the second tab 120 px below the bar | Second floats and is active; first tabbed | `tst_document_float::tearOutFloats` |
| `dtw_tear_out_last` | One tab; drag it below the bar | Document floats; tab count 0 | `tst_document_float::tearOutLastTab` |
| `dtw_tear_reorder` | Drag a tab sideways within the bar | Order changed; no float | `tst_document_float::sidewaysDragReorders` |
| `dtw_dock_back` | Drag a float header over the tab bar and release | Tab at drop index; active | `tst_document_float::dragHeaderDocks` |
| `dtw_wayland_system_move` | On KDE Plasma 6 (KWin, Wayland), float a document; drag its header past the frame's edge, onto a second monitor, and drop it; tear a tab out, keep dragging, and drop it on the strip | Each window follows the pointer for the whole drag; the header-dragged one stays where it was dropped; the torn-out one becomes a tab again and is active; the blue line shows over the strip while dragging | manual: the compositor moving a window during a drag-and-drop (`xdg_toplevel_drag_v1`) can't run on the offscreen platform |
| `dtw_wayland_tear_out` | Window drag forced, in-frame hosting off, hook set; drag the second tab below the bar | A visible top-level float for it, active; the first stays tabbed; the hook got the private type, the float's `windowHandle()`, and the header centre as the grab offset | `tst_document_float::tearOutUnderWayland` |
| `dtw_wayland_header_drag_mime` | Window drag forced, in-frame hosting off, hook set; float a doc; plain-drag its header 40 px | Hook got the private type; the window key decodes to `windowHandle()`; the position key equals the press point in the float; the float did not move | `tst_document_float::headerDragCarriesWindow` |
| `dtw_wayland_tear_out_drag` | As `dtw_wayland_tear_out`; drop the hooked MIME on tab 0 | Drop accepted; doc 1 is tab 0; no float; it is active | `tst_document_float::tearOutDragDocksOnStrip` |
| `dtw_dock_indicator` | Three docs, third floats; Ctrl+drag MIME; enter and move over tab 1's centre | Dock line visible, 2 px wide, as high as the zone, its left edge at tab 1's left edge less 1 | `tst_document_float::dockIndicatorBetweenTabs` |
| `dtw_dock_indicator_no_tabs` | Two docs, both float; enter and move the MIME of one over the top band of the tab widget | Tab count 0; dock line visible, 2 px high, as wide as the document area, inside the top band | `tst_document_float::dockIndicatorWithoutTabs` |
| `dtw_dock_drop_no_tabs` | As above; drop in the band | Drop accepted; doc is tab 0, active; no float for it; dock line hidden | `tst_document_float::dockDropWithoutTabs` |
| `dtw_dock_indicator_hidden_on_leave` | Three docs, third floats; enter and move over the bar, then send `DragLeave` | Line visible after the move, hidden after the leave; window still floats | `tst_document_float::dockIndicatorHidesOnLeave` |
| `dtw_hand_drag_indicator` | In-frame hosting forced, window drag off; three docs, third floats; press its header and move over tab 1 without releasing; release | Line visible before the release; hidden after it; doc 2 is tab 1 | `tst_document_float::handDragShowsIndicator` |
| `dtw_dock_zone_inactive_tiled_or_off` | Docking off: enter and move a float's MIME over tab 0. Docking on, three docs tiled, one floated: move its MIME over the tab widget | Neither move accepted; the line never shows; the window still floats | `tst_document_float::dockZoneInactiveTiledOrOff` |
| `dtw_ctrl_drag_docks` | Three docs, third floats; Ctrl+drag its header (hook records the MIME); enter, move, drop on tab 1 | Hook got the private type and no carried window; drop accepted; third doc at tab 1; three tabs; third active | `tst_document_float::ctrlDragDocksAtDrop` |
| `dtw_ctrl_drag_refused` | Docking off; float a doc; enter and drop its MIME on the tab bar | Enter and drop not accepted; window still floats | `tst_document_float::ctrlDragRefusedWithoutDocking` |
| `dtw_ctrl_drag_elsewhere` | One tabbed, one floating; enter, move, and drop the float's MIME on the tabbed canvas | Drop not accepted; same float, same geometry | `tst_document_float::ctrlDragElsewhereKeepsFloat` |
| `dtw_ctrl_drag_keeps_file_drops` | One doc floats; drop a PNG's file URL on the tab bar | Drop accepted; a second document opens; the float is unchanged | `tst_document_float::fileDropOnStripOpensFile` |
| `dtw_header_double_click_docks` | Docking off; one tabbed, one floating; double-click the float's header | Float gone; the doc is tab 1 (last); it is active | `tst_document_float::headerDoubleClickDocks` |
| `dtw_float_header_label` | Float a doc; edit it | Header text == label followed by the modified marker | `tst_document_float::floatHeaderShowsMarker` |
| `dtw_float_hosting` | Float with child mode forced, then without | Child overlay of the frame vs. a `Qt::Tool` top-level parented to the frame, on every platform | `tst_document_float::floatHostingFollowsPlatform` |
| `dtw_float_toplevel_wayland` | Window drag forced, in-frame hosting off; float a doc; drag its header (hook records the MIME); drop the MIME on tab 1 | Top-level `Qt::Tool`, transient for the frame; the drag carries the window; the drop docks it at tab 1, active | `tst_document_float::floatTopLevelUnderWayland` |
| `dtw_float_close` | Only doc floats, clean; header close | Float and doc gone; document count 0; double-click-to-open armed | `tst_document_float::floatCloseClosesDocument` |
