# Design

## Context

After `wayland-floating-document-windows`, every float is a frameless
`Qt::Tool` top-level transient for the frame. On Wayland a plain header drag
calls `QWindow::startSystemMove()` and can't dock; a tear-out floats the window
where the compositor maps it and stops. Ctrl+drag carries the window as
drag-and-drop under `application/x-pictura-document-window`, and the frame's
`windowDropFilter` on `documentTabBar` and `documentTabs` docks it when it is
dropped on the strip's band. X11 and offscreen move floats by hand
(`beginDrag` / `dragTo` / `endDrag`), and a release over the strip docks
(`onFloatDragReleased`).

QtWayland (`qwaylanddatadevice.cpp`, `startDrag`) attaches a top-level to a
drag through `xdg_toplevel_drag_v1` when the compositor offers the manager and
the drag's MIME data has `application/x-qt-mainwindowdrag-window` (a
`QDataStream` of `qintptr`, the `QWindow*`) and
`application/x-qt-mainwindowdrag-position` (a `QDataStream` of `QPoint`, the
grab offset in that window). `QDockWidget` uses the same keys
(`qmainwindowlayout.cpp`, `performPlatformWidgetDrag`). The compositor then
moves the window with the pointer for the whole drag, and the drag's enter,
move, and drop still go to the surfaces under the pointer.

## Goals / Non-Goals

**Goals:**

- On Wayland a header drag and a tear-out move the window with the pointer,
  and a drop on the strip docks it.
- A float can be docked by drag when no document is tabbed.
- One dock line shows where a drop or release docks, for every drag that can
  dock.

**Non-Goals:**

- Floating panels (`PanelFloat`) and their own drop indicators.
- Saving float positions (UI-15).
- Docking by dropping on another float or on a tile pane.

## Decisions

### F1. The window drag

`startWindowDockDrag(window, carryWindow, grabOffset)` builds the drag's MIME
data: the private type, as before, and with `carryWindow` the two main-window
keys, serialized exactly as `QDockWidget` does, naming the float's
`windowHandle()` and `grabOffset`. The frame is the drag source; QtWayland
takes the drag's origin and serial from the last input, so a press on the
float's header or on the tab strip both start it.

`QDrag::exec` still runs on the next event-loop turn, so the header's or the
tab strip's mouse handler has returned first. It runs only while
`QGuiApplication::mouseButtons()` still has the left button: a press already
released can't start a drag. A carried window is the drag's image, so the drag
sets no pixmap then; Ctrl+drag keeps the header's grab as its pixmap.

Test seam: `setWindowDockDragForTest(hook)` receives the MIME data in place of
the deferred `exec`, as before, now with the carried window's keys.

### F2. Which platform carries the window

`DocumentFloatWindow::movesByWindowDrag()` replaces `movesBySystem()`: true when
`QGuiApplication::platformName()` starts with `wayland`, or while
`setWindowDragForTest(true)` forces it. X11 keeps the hand drag: there a client
moves its own top-level, and the release it receives docks. `startSystemMove()`
and its fallback are removed; nothing else used them.

ponytail: a Wayland compositor without `xdg_toplevel_drag_v1` runs the drag as
a plain drag-and-drop. The window doesn't follow the pointer and stays where it
was, while a drop on the zone still docks and a header double-click still
docks. There is no system-move fallback for that case.

### F3. The header drag

A press starts the hand drag (`beginDrag`), as before. When the pointer passes
the start-drag distance and the float is a top-level that moves by window
drag, `dragTo` ends the float's own drag and calls `onWindowDragRequested` with
the grab offset (the press in the float's coordinates). The frame starts the
carried drag. A child-hosted float (the test hook) and every other platform
keep the hand drag. Ctrl+drag is unchanged: it never carries the window.

### F4. The tear-out

When the tear-out floats a tab and the new float moves by window drag, the
filter starts the carried drag for it at once and claims the event. The float
was shown by `floatWindow`, so its top-level role exists before `exec`. The
grab offset is the centre of the header, `(width / 2, kHeaderHeight / 2)`, the
point today's placement puts under the pointer. The deferred `exec` (F1)
lets the filter return first, and the button is still down. X11 keeps
`tearFloat_` and the hand drag.

### F5. The dock zone

`dockZoneGlobalRect()` replaces `tabStripGlobalRect()`. It is the band along
the top of the tab widget, which is the document area's page while the strip
shows: as high as the tab bar while the bar shows tabs, otherwise
`max(tab bar sizeHint height, DocumentFloatWindow::kHeaderHeight)`, the band a
tab bar would take. It is empty while docking is off, while tiled
(`tileHost_`), and while the tab widget is hidden (a full-screen mode lending
the document area to a float's page). The hand drag's release
(`onFloatDragReleased`) and the drop filter both test it, and
`dockIndexAt(globalPos)` picks the insertion index: the tab under the pointer,
or the end.

### F6. The dock line

The frame owns one indicator, `documentDockIndicator`, a child of the frame
created on first use and transparent for mouse events. `updateDockIndicator`
shows it while a point is in the zone and hides it outside:

- with tabs, a 2 px vertical line at the insertion point (the left edge of the
  tab the drop inserts before, or the right edge of the last tab), as high as
  the zone;
- with no tabs, a 2 px horizontal line along the bottom of the band.

Its colour comes from the theme: `QWidget#documentDockIndicator` joins the rule
that paints `panelDropIndicator` and the other insertion lines `${accent}`
(`#2a7fff`).

The drop filter updates it on enter and move, and hides it on leave and drop.
The hand drag reports each move through the float's `onDragMoved`, and
`onFloatDragReleased` hides it on release. The deferred `exec` hides it when
the drag returns, which covers a cancel.

### F7. Catching the drop

The frame's `windowDropFilter` stays on `documentTabs` and `documentTabBar`.
Both accept drops; the tab widget fills the document area while it is the
area's page, so a drag over the zone reaches one of them even when the bar has
no tabs: the empty bar and the tab widget's blank page don't accept drops, and
Qt hands the event to the tab widget. Whenever the zone is active the tab
widget is that page, so no wider filter is needed.

The filter claims only events whose MIME data has the private type, and
`DragLeave`, which carries no data, only hides the line and passes on.
`FileDropRouter` claims only layer drags and local file URLs, and passes the
private type through, so file and layer drops are unchanged.

`DragEnter` is accepted for a known float whenever the zone is active, even
outside it: the tab widget then keeps receiving the moves while the pointer
travels to the band. `DragMove` and `Drop` are accepted only inside the zone.

### F8. Docking after a platform drop

A drop must not delete the float that the running platform drag carries. The
drop filter records the pending dock (window and index) and queues
`applyPendingDock()`; while `exec` runs it does nothing, and the code after
`exec` returns applies it. Without an `exec` (the test seam) the queued call
docks. `dockWindow` hides and deletes the float and activates the docked tab,
so no float is left behind. A detached window clears the pending dock.

## Risks / Trade-offs

- [The compositor lacks `xdg_toplevel_drag_v1`] → The ponytail in F2: the
  window stays put during the drag; dropping on the zone and the header
  double-click still dock.
- [KWin delivers the drag to the dragged window's own surface] → The float's
  canvas refuses the private type, so the drop is refused there and the window
  stays where it was.
- [The deferred `exec` after a tear-out] → QtWayland takes the last input
  serial, which is the tab strip's press. Offscreen can't run a platform drag,
  so the pointer-follow on KWin is a manual row.
- [`DragEnter` accepted outside the zone] → The drag cursor may show "allowed"
  between entering the tab widget and the first move; the move then refuses
  outside the zone.
