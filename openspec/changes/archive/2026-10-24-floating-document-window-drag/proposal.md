# Proposal

## Why

Smoke testing on KDE Plasma 6 (KWin, Wayland) with Qt 6.11 found two problems
with the floating document windows of `wayland-floating-document-windows`:

1. A tab dragged out of the strip becomes a float that appears where KWin maps
   it, in the middle of the main window. The drag stops there; the window does
   not follow the pointer.
2. When every document floats the tab bar has no tabs and takes no space. A
   plain header drag on Wayland is the compositor's move, which can't dock, so
   there is no drag that puts a float back as a tab, and nothing shows where a
   docked window would go.

QtWayland already supports `xdg_toplevel_drag_v1`, which KWin advertises. A
drag-and-drop whose `QMimeData` names a top-level window (the keys
`QDockWidget` uses on Wayland) has that window attached to the drag: the
compositor moves the window with the pointer for the whole drag, while the
drag's enter, move, and drop events still reach the surfaces under the
pointer. That gives Wayland both a window that follows the pointer and a drop
the application sees.

## What Changes

- On Wayland a plain header drag of a top-level float starts a drag-and-drop
  that carries the window under the private document-window MIME type and the
  two Qt main-window-drag keys (the float's `QWindow` and the press position in
  it). The window follows the pointer; a drop on the dock zone docks it, a drop
  anywhere else leaves it where the compositor left it. This replaces the
  `startSystemMove()` path, which is removed.
- A tab torn out on Wayland starts the same drag at once, with the pointer at
  the centre of the new float's header, so the torn-out window follows the
  pointer and dropping it back on the strip docks it again. X11 keeps the
  hand-moved tear-out.
- The dock zone is the tab strip's band, or, while the strip has no tabs or is
  hidden, the band a tab bar would take at the top of the document area. It is
  inactive while tiled and while `Enable Floating Document Window Docking` is
  off.
- While a document-window drag is over the zone, the frame shows a 2 px
  `#2a7fff` dock line: vertical at the insertion point between tabs, or
  horizontal along the band when there are no tabs. It hides on leave, drop,
  release, and cancel. This applies to the Wayland drag, Ctrl+drag on every
  platform, and the hand-moved header drag (X11, offscreen).
- A drop that docks records the window and index and docks once the platform
  drag has finished, so the float a running drag carries is never deleted
  under it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/document-windows`: "Tear a tab out and dock it back" (the Wayland
  window drag for header drags and tear-outs, the dock zone without tabs, and
  the dock line) and "Floating document window" (the Wayland scenario's header
  drag now docks through the window drag).

## Impact

- `crates/pictura-app/cpp`:
  - `document_float_window.{h,cpp}`: the Wayland header drag request, the
    hand drag's move report, and the renamed test seam
    (`movesByWindowDrag()` / `setWindowDragForTest()`); `startSystemMove()` is
    removed.
  - `frame.h`, `frame_document_menus.cpp`, `frame_document_windows.cpp`: the
    carried-window MIME keys, the tear-out drag, the dock zone, the dock line,
    and the deferred dock.
  - `theme.cpp`: the dock line joins the accent indicator rule.
  - `tests/tst_document_float.cpp`: the new and rewritten cases.
- `docs/dev/doc-code-parity-gaps.md` (UI-14) and `docs/dev/STATE.md`.
