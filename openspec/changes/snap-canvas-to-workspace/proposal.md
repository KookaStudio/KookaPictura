# Proposal

## Why

When a document is zoomed out enough to fit inside the workspace, the canvas can
drift off-centre or be dragged into the surrounding backdrop. Issue #122 asks for
a checkable **View > Snap Canvas To Workspace** option that, when on, locks a
fitting canvas to the workspace centre: the canvas is pinned, panning is refused,
and the workspace scrollbars (which offer no pan range while the document fits)
are hidden, so the workspace reads as a single centred sheet. Once the document
is zoomed past the workspace the lock releases and panning and the bars return.

This is distinct from **View > Free Pan** (#414), which is per-axis and stays on
by default: Free Pan controls whether an axis that fits may be panned off-centre
at all, while Snap Canvas To Workspace is an explicit, document-level lock that
also hides the now-empty scrollbars. Snap defaults off, so today's behaviour is
unchanged for every existing user.

## What Changes

- Add a checkable **View > Snap Canvas To Workspace** menu item (command id
  `view.snapCanvasToWorkspace`), **unchecked by default**.
- When **checked** and the scaled document fits the viewport on both axes: the
  canvas offset is forced to the workspace centre, a pan cannot move it, and both
  workspace scrollbars are hidden. When the scaled document overflows either
  axis, the flag has no effect: the existing free-pan policy governs, panning
  works, and the scrollbars are shown.
- The toggle state persists across sessions through the existing session store.
- Toggling applies immediately to every open document's canvas, and the flag is
  applied to each newly opened document.
- `docs/dev/canvas-view-spec.md` gains the new mode and clarifies that wheel
  zoom-in stays cursor-anchored in both modes.

Not a breaking change: the default preserves today's behaviour. The capability
specs are the contract.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities

- `document/command-registry`: add the `View > Snap Canvas To Workspace` command
  declaration, its checkable dispatch, default-unchecked state, and session
  persistence to the command table.
- `tools/canvas-tools`: add the workspace-snap clamp policy (pin both axes to the
  centre while the whole document fits, otherwise defer to the free-pan policy).
- `document/canvas-scrollbars`: modify the always-visible requirement so the bars
  hide while the workspace snap lock is active, and reappear once the document
  overflows or the flag is cleared.

## Impact

- App C++ (the only affected layer; no engine crate or Qt bridge change):
  - `crates/pictura-app/cpp/canvas_range.h` — `offsetRangeFor` gains a
    `snapToWorkspace` parameter.
  - `crates/pictura-app/cpp/image_view.{h,cpp}` and `image_view_document.cpp` —
    per-canvas `snapToWorkspace_` flag, `setSnapToWorkspace` (re-clamp +
    `update()` + emit), a `workspaceSnapLocked()` query, and pan rejection while
    locked.
  - `crates/pictura-app/cpp/canvas_scrollbars.{h,cpp}` — hide/show the bars and
    their corner from the canvas's locked state.
  - `crates/pictura-app/cpp/frame.h` — the window-level flag.
  - `crates/pictura-app/cpp/frame.cpp` — apply the flag to each canvas in
    `addDocument`.
  - `crates/pictura-app/cpp/frame_guides.cpp` — load the flag in
    `loadGuideSession`, following the existing view-toggle pattern.
  - `crates/pictura-app/cpp/frame_menus.cpp` — the handler and checked provider.
  - `crates/pictura-app/cpp/frame_session.cpp` — persist the flag.
  - `crates/pictura-app/cpp/commands.h` / `command_tree.cpp` — the command id and
    the checkable View-menu entry.
  - `crates/pictura-app/cpp/session.{h,cpp}` — the persisted field.
  - `crates/pictura-app/cpp/tests/tst_canvas_pan.cpp` — Qt Test cases for the
    lock, pan rejection, and scrollbar hiding.
  - `crates/pictura-app/cpp/tests/tst_command_tree.cpp` — the command-table
    entry.
  - `crates/pictura-app/cpp/tests/tst_session_store.cpp` — the persistence
    round-trip.
- `docs/dev/canvas-view-spec.md` (TASK-ALLOWS-DOCS).
- No new dependencies.
