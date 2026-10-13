# Proposal

## Why

When the image is zoomed out enough to fit inside the workspace, the canvas can
be dragged into the surrounding backdrop, so the document drifts off-centre and
becomes hard to relocate. Photoshop instead pins a fitting image to the centre
and only allows free panning once the document overflows the viewport. Kooka
Pictura always free-pans and offers no way to switch to the constrained
behaviour, so users who want Photoshop-style framing cannot get it.

## What Changes

- Add a checkable **View > Free Pan** menu item (command id `view.freePan`),
  **checked by default** so the current free-pan behaviour is unchanged for
  existing users.
- When **Free Pan is checked** (default): panning behaves exactly as today — the
  image may be panned within the shared reveal margin even when it fits.
- When **Free Pan is unchecked**: on any axis where the scaled document fits
  inside the viewport, the pan range collapses to the centred position, so the
  image stays pinned/centred (Photoshop behaviour). Axes where the document
  overflows keep today's reveal-margin clamp, so panning while zoomed in past the
  viewport is unaffected.
- The workspace scrollbars honour the same range, so a constrained axis collapses
  its scrollbar range to the centred value rather than offering a range the
  canvas will refuse.
- The toggle state persists across sessions through the existing session store.
- Toggling applies immediately to every open document's canvas.

Not a breaking change: the default preserves today's behaviour. `docs/` is
explicitly out of scope for this change; the capability specs are the contract.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities

- `document/command-registry`: add the `View > Free Pan` command declaration, its
  checkable dispatch, default-checked state, and session persistence to the
  command table.
- `tools/canvas-tools`: add the per-axis free-pan clamp policy the toggle enables
  (centre a fitting axis, leave an overflowing axis on the reveal margin).
- `document/canvas-scrollbars`: modify the always-visible requirement so its
  pannable clause is scoped to Free Pan on, and add the requirement that the bars
  project the constrained (free-pan) range.

## Impact

- App C++ (the only affected layer; no engine crate or Qt bridge change):
  - `crates/pictura-app/cpp/canvas_range.h` — `offsetRangeFor` gains a `freePan`
    parameter.
  - `crates/pictura-app/cpp/image_view.{h,cpp}` and `image_view_document.cpp` —
    per-canvas `freePan_` flag, `setFreePan` (re-clamp + `update()` + emit), and
    `clampOffset` threading, including the document-size replacement paths.
  - `crates/pictura-app/cpp/canvas_scrollbars.cpp` — read the canvas's free-pan
    state at the three `offsetRangeFor` call sites.
  - `crates/pictura-app/cpp/frame.h` — the window-level `freePan_` flag.
  - `crates/pictura-app/cpp/frame.cpp` — apply the flag to each canvas in
    `addDocument`.
  - `crates/pictura-app/cpp/frame_guides.cpp` — load `freePan_` in
    `loadGuideSession`, following the existing view-toggle pattern.
  - `crates/pictura-app/cpp/frame_menus.cpp` — the `ViewFreePan` handler and
    checked provider.
  - `crates/pictura-app/cpp/frame_session.cpp` — persist the flag.
  - `crates/pictura-app/cpp/commands.h` / `command_tree.cpp` — the command id and
    the checkable View-menu entry.
  - `crates/pictura-app/cpp/session.{h,cpp}` — the persisted `freePan` field.
  - `crates/pictura-app/cpp/tests/tst_canvas_pan.cpp` — Qt Test cases for the
    constrained clamp and the scrollbar projection.
  - `crates/pictura-app/cpp/tests/tst_command_tree.cpp` — the command-table entry.
  - `crates/pictura-app/cpp/tests/tst_session_store.cpp` — the persistence
    round-trip.
- No new dependencies.
