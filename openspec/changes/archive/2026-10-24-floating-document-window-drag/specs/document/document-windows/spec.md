## MODIFIED Requirements

### Requirement: Tear a tab out and dock it back

Dragging a document tab out of the tab strip, past a short distance
perpendicular to the strip, SHALL float that window under the pointer and let
the drag continue as a window move. Dragging along the strip SHALL keep
reordering tabs. Dragging a floating document window by its header and releasing
or dropping it on the dock zone SHALL dock it as a tab at the drop position,
provided `Enable Floating Document Window Docking` is on. Tearing out SHALL also
work for the last remaining tab. A loading tab SHALL NOT tear out.

The dock zone SHALL be the tab strip's band. While the strip has no tabs, or its
bar is hidden, the dock zone SHALL be the band along the top of the document
area where the tab bar would be, as high as the tab bar's preferred height and
at least as high as a floating window's header. The dock zone SHALL be inactive
while the document windows are tiled and while
`Enable Floating Document Window Docking` is off.

While a drag that can dock a floating window is over the dock zone, the
application SHALL show a blue placement line in the drop indicator accent: a
vertical line at the insertion point between tabs while the strip has tabs, or
a horizontal line along the band while it has none. The line SHALL hide when
the drag leaves the zone, drops, is released, or is cancelled.

Where the compositor owns top-level placement (Wayland), a plain header drag of
a floating window SHALL start a drag-and-drop that carries the window and asks
the compositor to move the window with the pointer for the rest of the drag, so
that the window can leave the application frame. Dropping it on the dock zone
SHALL dock it at the drop position; a drop anywhere else, or outside the
application, SHALL leave the window floating where the compositor left it. A
tab torn out on Wayland SHALL float, become active, and continue as the same
window drag, the pointer holding the centre of the new window's header, so that
the window follows the pointer and a drop on the dock zone docks it again. A
compositor that can't move a window during a drag-and-drop leaves the window in
place while the drag runs; dropping on the dock zone still docks it.

On every platform, dragging a floating window's header with Ctrl held SHALL
start a drag-and-drop that carries the window. Dropping it on the dock zone
SHALL dock the window as a tab at the drop position, or at the end of the strip
when the drop is past the last tab, and activate it. The zone SHALL refuse the
drop while `Enable Floating Document Window Docking` is off. A drop anywhere
else SHALL leave the window floating where it was. File drops on the tab strip
SHALL keep opening the dropped files.

A drop that docks SHALL leave no floating window behind, and the docked tab
SHALL become active.

Double-clicking a floating window's header SHALL dock it as the last tab and
activate it, whether or not docking is enabled.

#### Scenario: Tear out [dtw_tear_out]
- **WHEN** two documents are tabbed and the user drags the second tab well below
  the tab strip and releases
- **THEN** the second document is in a floating document window, the first stays
  tabbed, and the floating window is active

#### Scenario: Tear out the last tab [dtw_tear_out_last]
- **WHEN** one document is tabbed and the user drags its tab well below the strip
- **THEN** the document floats and the tab strip has no tabs

#### Scenario: Reordering is unchanged [dtw_tear_reorder]
- **WHEN** the user drags a tab sideways within the strip
- **THEN** the tabs are reordered and nothing floats

#### Scenario: Dock back [dtw_dock_back]
- **WHEN** the header drag moves the window by hand and the user drags a
  floating window's header over the tab strip and releases
- **THEN** the document becomes a tab at the drop position and becomes active

#### Scenario: A header drag on Wayland leaves the frame [dtw_wayland_system_move]
- **WHEN** the platform is Wayland with a compositor that moves a window during
  a drag-and-drop, and the user drags a floating window's header past the edge
  of the application frame and drops it there, then tears a tab out and drops
  it back on the strip
- **THEN** each window follows the pointer for the whole drag, the
  header-dragged window stays floating where it was dropped, and the torn-out
  window becomes a tab again

#### Scenario: Tear out on Wayland [dtw_wayland_tear_out]
- **WHEN** the platform is Wayland and the user drags a tab well below the strip
- **THEN** the document floats in a visible top-level window that is active, the
  first document stays tabbed, and a drag-and-drop that carries the new window
  starts

#### Scenario: A Wayland header drag carries the window [dtw_wayland_header_drag_mime]
- **WHEN** the platform is Wayland and the user drags a floating window's header
  past the drag distance without Ctrl
- **THEN** a drag-and-drop starts whose data carries the document window, the
  floating window's native window, and the press position in that window, and
  the application does not move the window itself

#### Scenario: A torn-out window docks again by drop [dtw_wayland_tear_out_drag]
- **WHEN** the platform is Wayland, the user tears the second of two tabs out,
  and the drag that carries it drops on the first tab
- **THEN** the document becomes the tab at index 0, no floating window is left,
  and it is active

#### Scenario: The dock line between tabs [dtw_dock_indicator]
- **WHEN** two documents are tabbed, a third floats, and a drag carrying the
  float moves over the second tab
- **THEN** a 2 px vertical dock line shows at the second tab's left edge, as
  high as the strip

#### Scenario: The dock line without tabs [dtw_dock_indicator_no_tabs]
- **WHEN** every document floats and a drag carrying one of them moves over the
  top band of the document area
- **THEN** a 2 px horizontal dock line shows along that band

#### Scenario: Dropping into the empty strip docks [dtw_dock_drop_no_tabs]
- **WHEN** every document floats and a drag carrying one of them drops on the
  top band of the document area
- **THEN** the drop is accepted, the document becomes tab 0 and is active, no
  floating window is left for it, and the dock line hides

#### Scenario: The dock line hides when the drag leaves [dtw_dock_indicator_hidden_on_leave]
- **WHEN** a drag carrying a floating window shows the dock line and then leaves
  the strip
- **THEN** the dock line hides

#### Scenario: The hand drag shows the dock line [dtw_hand_drag_indicator]
- **WHEN** the header drag moves a floating window by hand and the pointer is
  over the tab strip, and the user then releases
- **THEN** the dock line shows while the pointer is over the strip, hides on the
  release, and the window docks

#### Scenario: No dock zone while tiled or with docking off [dtw_dock_zone_inactive_tiled_or_off]
- **WHEN** docking is off, or the windows are tiled, and a drag carrying a
  floating window moves over the strip
- **THEN** the drag is refused and no dock line shows

#### Scenario: Ctrl+drag docks at the drop position [dtw_ctrl_drag_docks]
- **WHEN** two documents are tabbed, a third floats, and the user Ctrl+drags the
  float's header and drops it on the second tab
- **THEN** the third document becomes the tab at index 1, the strip has three
  tabs, and the third document is active

#### Scenario: Ctrl+drag is refused while docking is off [dtw_ctrl_drag_refused]
- **WHEN** docking is disabled and the user Ctrl+drags a floating window's
  header onto the tab strip
- **THEN** the strip refuses the drop and the window stays floating

#### Scenario: A Ctrl+drag dropped elsewhere leaves the window floating [dtw_ctrl_drag_elsewhere]
- **WHEN** the user Ctrl+drags a floating window's header and drops it on a
  tabbed document's canvas
- **THEN** the drop is refused and the window stays floating where it was

#### Scenario: File drops on the strip still open files [dtw_ctrl_drag_keeps_file_drops]
- **WHEN** a document floats and the user drops an image file on the tab strip
- **THEN** the file opens as a new document and the floating window is unchanged

#### Scenario: Double-clicking the header docks [dtw_header_double_click_docks]
- **WHEN** docking is disabled, a document floats beside a tabbed one, and the
  user double-clicks the float's header
- **THEN** the document becomes the last tab and is active

### Requirement: Floating document window

A floating document window SHALL show a header carrying the document window's
full label, elided only to the header's width, followed by the modified marker
while the document is modified, and a close control. The close control SHALL
follow the tab close rules. The window SHALL stay above the application frame
and hide with it. A floating window's header SHALL stay inside the document
area when the window is placed by a command, so that no command produces an
unreachable window.

On every platform, a floating document window SHALL be a frameless top-level
window that is transient for the application frame. Where the compositor owns
top-level placement (Wayland), the compositor SHALL place the window and move
it when its header is dragged.

#### Scenario: Header label follows the document [dtw_float_header_label]
- **WHEN** a floating document is modified
- **THEN** its header shows the label followed by the modified marker

#### Scenario: Hosting follows the platform [dtw_float_hosting]
- **WHEN** a document is floated with in-frame hosting forced for testing, and
  again with the platform's own hosting
- **THEN** the window is a child overlay of the frame in the first case and a
  top-level tool window parented to the frame in the second, whatever the
  platform

#### Scenario: A float on Wayland is a top-level window [dtw_float_toplevel_wayland]
- **WHEN** the platform reports Wayland and a document is floated
- **THEN** the window is a top-level tool window transient for the frame, and a
  header drag that drops on the tab strip docks it

#### Scenario: Closing the last window of a floating document [dtw_float_close]
- **WHEN** an unmodified document's only window floats and the user uses its
  header close control
- **THEN** the floating window and the document close, and the empty workspace
  shows when no other document is open
