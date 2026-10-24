# Test plan

Every automated check is a Qt Test case in `tst_document_float`, run on the
offscreen platform. Nothing is added to the self-test. Each test function
builds its own frame, so the suite stays sharded.

The offscreen platform has no system move: `QWindow::startSystemMove()`
returns false there. `DocumentFloatWindow::setSystemMoveForTest(true)` makes a
float act as it does on Wayland, so the cases that use it cover the path where
the compositor refuses the system move. A real compositor's move can't be
driven offscreen, so it is a manual row.

The Ctrl+drag cases set `setWindowDockDragForTest`, which receives the drag's
MIME data in place of the blocking `QDrag::exec`. They then send a synthetic
`QDragEnterEvent`, `QDragMoveEvent`, and `QDropEvent` with that MIME data to
the drop target.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| `dtw_tear_out` | Two tabs; drag the second tab 120 px below the bar | Second floats and is active; first tabbed | `tst_document_float::tearOutFloats` |
| `dtw_tear_out_last` | One tab; drag it below the bar | Document floats; tab count 0 | `tst_document_float::tearOutLastTab` |
| `dtw_tear_reorder` | Drag a tab sideways within the bar | Order changed; no float | `tst_document_float::sidewaysDragReorders` |
| `dtw_dock_back` | Drag a float header over the tab bar and release | Tab at drop index; active | `tst_document_float::dragHeaderDocks` |
| `dtw_wayland_system_move` | On a Wayland session, float a document; drag its header past the frame's edge, also onto a second monitor; then tear a tab out | The window follows the pointer outside the frame and stays floating; the torn-out float appears where KWin places it, visible and active | manual: a real compositor's interactive move can't run on the offscreen platform |
| `dtw_wayland_tear_out` | System move forced, in-frame hosting off; drag the second tab below the bar | A top-level float exists for it, it is active, the first stays tabbed | `tst_document_float::tearOutUnderWayland` |
| `dtw_ctrl_drag_docks` | Three docs, third floats; Ctrl+drag its header (hook records the MIME); enter, move, drop on tab 1 | Hook got the private type; drop accepted; third doc at tab 1; three tabs; third active | `tst_document_float::ctrlDragDocksAtDrop` |
| `dtw_ctrl_drag_refused` | Docking off; float a doc; enter and drop its MIME on the tab bar | Enter and drop not accepted; window still floats | `tst_document_float::ctrlDragRefusedWithoutDocking` |
| `dtw_ctrl_drag_elsewhere` | One tabbed, one floating; enter, move, and drop the float's MIME on the tabbed canvas | Drop not accepted; same float, same geometry | `tst_document_float::ctrlDragElsewhereKeepsFloat` |
| `dtw_ctrl_drag_keeps_file_drops` | One doc floats; drop a PNG's file URL on the tab bar | Drop accepted; a second document opens; the float is unchanged | `tst_document_float::fileDropOnStripOpensFile` |
| `dtw_header_double_click_docks` | Docking off; one tabbed, one floating; double-click the float's header | Float gone; the doc is tab 1 (last); it is active | `tst_document_float::headerDoubleClickDocks` |
| `dtw_float_header_label` | Float a doc; edit it | Header text == label followed by the modified marker | `tst_document_float::floatHeaderShowsMarker` |
| `dtw_float_hosting` | Float with child mode forced, then without | Child overlay of the frame vs. a `Qt::Tool` top-level parented to the frame, on every platform | `tst_document_float::floatHostingFollowsPlatform` |
| `dtw_float_toplevel_wayland` | System move forced, in-frame hosting off; float a doc; drag its header over the tab bar | Top-level `Qt::Tool`, transient for the frame; the refused system move falls back to the hand drag, which docks it | `tst_document_float::floatTopLevelUnderWayland` |
| `dtw_float_close` | Only doc floats, clean; header close | Float and doc gone; document count 0; double-click-to-open armed | `tst_document_float::floatCloseClosesDocument` |
