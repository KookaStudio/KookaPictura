# Proposal

## Why

On Wayland a floating document window is an overlay inside the application
frame (design D3 of `document-tabs-windows`). That choice assumed a client
can't move a top-level window on Wayland, because the compositor ignores
`move()`. The user found that Wayland does support movable sub-windows through
`QWindow::startSystemMove()`, which the Filter Gallery's dialog chrome already
uses (`dialog_chrome.cpp`, `DialogTitleBar::mousePressEvent`). The in-frame
overlay keeps every floating document inside the frame, so a document can't be
moved to a second monitor or beside the frame. Drag-to-dock also misbehaved
there. `docs/02-ui-ux/application-frame.md` already describes floating
documents on Wayland as compositor-managed top-levels (parity gap UI-14).

## What Changes

- On every platform a floating document window is a frameless top-level window
  that is transient for the frame. Wayland no longer uses an in-frame overlay.
  A test hook still forces in-frame hosting, so the offscreen geometry tests
  keep working. Floating panels (`PanelFloat`) are unchanged.
- A plain header drag on Wayland hands the move to the compositor with
  `startSystemMove()`, so the window can leave the frame. When the compositor
  refuses, and on every other platform, the header drag moves the window by
  hand and a release over the tab strip docks it, as before.
- Tearing a tab out on Wayland tries a system move on the new float while the
  button is still down. When the compositor refuses, the float appears where
  the compositor places it.
- Ctrl+drag on a floating window's header starts a drag-and-drop that carries
  the window. Dropping it on the document tab strip docks the window at the
  drop position, unless `Enable Floating Document Window Docking` is off. File
  drops on the strip keep opening files.
- Double-clicking a floating window's header docks it at the end of the tab
  strip and activates it, whatever the docking preference says.
- Cascade on Wayland still floats every window and gives them one size. The
  compositor owns their positions, as the Cascade requirement already allows.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/document-windows`: the "Floating document window" hosting rule
  (top-level everywhere) and "Tear a tab out and dock it back" (the system move
  on Wayland, Ctrl+drag docking, and header double-click docking).

## Impact

- `crates/pictura-app/cpp`:
  - `document_float_window.{h,cpp}`: hosting, the system move, Ctrl+drag, and
    the header double-click
  - `frame_document_windows.cpp`, `frame_document_menus.cpp`,
    `frame_build.cpp`, `frame.h`: the dock-drag MIME type, the strip drop, and
    the tear-out's system move
  - `panels/panel_column.h`, `panels/panel_float.cpp`: a read accessor for the
    existing child-overlay test hook. Panel hosting does not change.
  - `tests/tst_document_float.cpp`: the new cases
- `docs/dev/doc-code-parity-gaps.md` (UI-14) and `docs/dev/STATE.md`.
