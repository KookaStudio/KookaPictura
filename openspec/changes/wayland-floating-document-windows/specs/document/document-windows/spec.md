## MODIFIED Requirements

### Requirement: Tear a tab out and dock it back

Dragging a document tab out of the tab strip, past a short distance
perpendicular to the strip, SHALL float that window under the pointer and let
the drag continue as a window move. Dragging along the strip SHALL keep
reordering tabs. Dragging a floating document window by its header and releasing
over the tab strip SHALL dock it as a tab at the drop position, provided
`Enable Floating Document Window Docking` is on. Tearing out SHALL also work for
the last remaining tab. A loading tab SHALL NOT tear out.

Where the compositor owns top-level placement (Wayland), a plain header drag
SHALL hand the move to the compositor's interactive move, so that the window can
leave the application frame. Such a drag SHALL NOT dock, because the
application learns neither the pointer's path nor the release. When the
compositor refuses the interactive move, the header drag SHALL move the window
by hand and a release over the tab strip SHALL dock it, as on other platforms.
A tab torn out on Wayland SHALL request the compositor's interactive move for
the new floating window while the button is still down. When the compositor
refuses, the window SHALL float where the compositor places it. Either way the
torn-out window SHALL float and become active.

On every platform, dragging a floating window's header with Ctrl held SHALL
start a drag-and-drop that carries the window. Dropping it on the tab strip
SHALL dock the window as a tab at the drop position, or at the end of the strip
when the drop is past the last tab, and activate it. The strip SHALL refuse the
drop while `Enable Floating Document Window Docking` is off. A drop anywhere
else SHALL leave the window floating where it was. File drops on the tab strip
SHALL keep opening the dropped files.

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
- **WHEN** the platform is Wayland and the user drags a floating window's header
  past the edge of the application frame
- **THEN** the compositor moves the window outside the frame and the window
  stays floating

#### Scenario: Tear out on Wayland [dtw_wayland_tear_out]
- **WHEN** the platform is Wayland and the user drags a tab well below the strip
- **THEN** the document floats in a top-level window that is active, moved by
  the compositor when it accepts the interactive move and otherwise placed by
  the compositor

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
- **THEN** the window is a top-level tool window transient for the frame, and
  when the compositor refuses the interactive move a header drag over the tab
  strip still docks it

#### Scenario: Closing the last window of a floating document [dtw_float_close]
- **WHEN** an unmodified document's only window floats and the user uses its
  header close control
- **THEN** the floating window and the document close, and the empty workspace
  shows when no other document is open
