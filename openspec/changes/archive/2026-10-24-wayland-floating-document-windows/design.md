# Design

## Context

`DocumentFloatWindow` follows `PanelFloat::overlayUsesTopLevel()`. On Wayland
that returns false, so the float is a child widget of the frame, clamped to the
frame's rect and moved by hand. Elsewhere it is a frameless `Qt::Tool`
top-level window parented to the frame, also moved by hand. A header release
over the tab strip docks it (`onFloatDragReleased`). The tab strip's tear-out
filter (`tearOutFilter`) floats a tab and hands the rest of the press to the
new float's `beginDrag` / `dragTo` / `endDrag`.

On Wayland a client can't place its own top-level window, but it can ask the
compositor to move one interactively with `QWindow::startSystemMove()`. The
dialog chrome (`DialogTitleBar::mousePressEvent`) already does this for every
framed dialog, including the Filter Gallery.

## Goals / Non-Goals

**Goals:**

- A floating document can leave the frame on Wayland.
- Docking stays possible on Wayland.
- X11 and the offscreen test platform keep today's hand-moved drag, and with it
  drop-to-dock.

**Non-Goals:**

- Floating panels (`PanelFloat`) keep their in-frame overlay on Wayland.
- Saving float positions (UI-15).

## Decisions

### W1. Hosting

A floating document window is a frameless `Qt::Tool` top-level window parented
to the frame on every platform. Qt makes a parented top-level transient for its
parent's window, so it stays above the frame and hides with it.

The in-frame hosting stays, for tests only. The existing
`PanelFloat::setForceChildOverlayForTest` hook already drives the four document
suites (`tst_document_float`, `tst_document_arrange`, `tst_document_windows`,
`tst_document_navigate`), so the float reuses it instead of adding a second
hook that every suite would have to set. `PanelFloat` gains a read accessor,
`childOverlayForcedForTest()`, and `DocumentFloatWindow` hosts as a child only
when it is true. `PanelFloat::overlayUsesTopLevel()` and the panels' own
hosting are unchanged.

### W2. Which platform moves by system

`DocumentFloatWindow::movesBySystem()` is true when
`QGuiApplication::platformName()` contains `wayland`, the same check as
`overlayUsesTopLevel()`, or when `setSystemMoveForTest(true)` forces it.

X11 also implements `startSystemMove()` (through `_NET_WM_MOVERESIZE`), but
there the window manager then owns the pointer until release. The client gets
no motion or release events, so drop-to-dock would stop working. X11 already
lets a client move its own top-level, so it keeps the hand-moved drag. Only
Wayland, where the hand-moved drag can't move a top-level at all, uses the
system move.

### W3. Plain header drag

A press starts the hand drag as before (`beginDrag`). When the pointer passes
the start-drag distance, `dragTo` first calls `startSystemMove()` on the
float's `QWindow` when `movesBySystem()`. If it returns true, the compositor
owns the move. The float ends its own drag and reports no release, because none
will arrive. If it returns false, because the compositor refused or the
platform has no system move (offscreen), the hand drag continues and a release
over the strip docks as before.

The system move starts at the drag threshold, not on the press, so a press
without motion and a double-click stay with the header.

### W4. Wayland limits

During a system move the client receives no motion or release events, and on
Wayland it never learns a top-level window's position. So:

- A plain header drag can't dock by drop on Wayland. Ctrl+drag (W5) and the
  header double-click (W6) are the Wayland ways to dock, and they work on every
  platform.
- Cascade, Float in Window, Float All, and the resize-to-fit placement set
  positions that the compositor may ignore. The Cascade requirement already
  allows this: the windows still float and share a size.
- `clampFloatGeometry` and the default placement still run, but on Wayland the
  compositor decides where the window lands.

### W5. Ctrl+drag carries the window as drag-and-drop

A press with Ctrl on the header arms a dock drag instead of a move. Past the
start-drag distance the header asks the frame to start it
(`onDockDragRequested`). The frame builds a `QMimeData` with the private type
`application/x-pictura-document-window` whose payload is the `DocWindow`'s
address. The frame compares the address with its registry and never
dereferences it until the lookup vouches for it, the same way the layer drag
tags its document (`kLayerSourceMimeType`).

The frame runs `QDrag::exec` on the next event-loop turn, with the frame as the
drag source. A drop that docks the window deletes the float, and that must not
happen while the header's own mouse handler is still on the stack under the
drag's nested loop.

A drag-and-drop is a real Wayland data-device drag. The compositor reports the
drop target to the client, so docking works where a system move can't.

Test seam: `setWindowDockDragForTest(hook)` on the frame. While a hook is set,
the frame passes the MIME data to the hook instead of running the blocking
`QDrag::exec`.

### W6. Header double-click docks

A left double-click on the header docks the window at the end of the strip
through `dockWindow(window, -1)`, which also activates it. It ignores the
docking preference, because it is an explicit command, like Consolidate.

### W7. The strip drop and file drops

Two event filters see drag events on `documentTabBar` and `documentTabs`:
`FileDropRouter` and the frame. Qt calls the filter installed last first, and
the router was installed last. The router claims an event only for a layer drag
(`kLayerMimeType` with a known source) or for local file URLs. For any other
MIME data it returns false, so a dock drag passes through it untouched. The
frame's `windowDropFilter` claims only events whose MIME data has the private
type and leaves file and layer drags alone. Neither filter touches the other's
payload.

`windowDropFilter`:

- `DragEnter`: accept when the window is a known float, docking is on, and the
  strip is visible. Accepting on enter lets the tab widget keep receiving moves
  until the pointer reaches the strip's band.
- `DragMove`: accept only inside the strip's band (the rect
  `onFloatDragReleased` already uses, now `tabStripGlobalRect()`).
- `Drop`: the same check, then `dockWindow(window, tabAt(pos))`. `tabAt` gives
  -1 past the last tab, which docks at the end.
- Otherwise ignore the event and claim it. A refused drop, or a drop anywhere
  else, leaves the window floating where it was.

### W8. Tear-out on Wayland

On Wayland the tear-out floats the window and stops there: it neither continues
the drag by hand (the client can't move a top-level) nor calls
`startSystemMove()`. A system move asked for right after `show()`, before the
compositor has mapped the surface and with a press serial that belongs to the
tab strip, made the torn-out window vanish on KWin during smoke testing. The
float appears where the compositor places it and is active; a header drag moves
it from there. X11 keeps the hand drag that continues the tear-out.

## Risks / Trade-offs

- [A tear-out on Wayland can't follow the pointer] → The float appears where the
  compositor places it and is active; the user moves it by its header.
- [A Wayland user expects a plain header drag over the strip to dock] → The
  parity register (UI-14) records it. Ctrl+drag and double-click dock.
- [`QDrag::exec` started from a timer instead of a mouse handler] → The button
  is still down on the next event-loop turn. Qt's X11 and Wayland drags take
  the pointer state then. Not verified offscreen, so it is in the manual
  checklist.
