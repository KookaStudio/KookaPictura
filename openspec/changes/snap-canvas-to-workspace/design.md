# Design

## Context

See `proposal.md` for motivation. The pan clamp lives in one helper,
`offsetRangeFor(imageSize, zoom, viewport, freePan, margin)` in `canvas_range.h`.
It already has a per-axis constrained mode: when `freePan` is false and an axis
fits, that axis collapses to the centred offset. It is consumed by
`ImageView::clampOffset()` (`image_view_document.cpp`) and by `CanvasScrollBars`
(`canvas_scrollbars.cpp`, three call sites).

The View menu is a registry-backed command tree (`command_tree.cpp`) with
checkable items wired in `frame_menus.cpp` through a handler plus a
`setCheckedProvider`. Window-level toggles (`rulersVisible_`, `freePan_`) follow a
"flip a bool, re-apply to every canvas, `saveSession()`" pattern
(`frame_guides.cpp`, `frame_menus.cpp`). `SessionState` (`session.h/.cpp`) is an
opaque JSON store with per-field defaults, so unknown keys and older stores
round-trip.

## Goals / Non-Goals

**Goals:**
- Express the document-level "whole document fits" decision in the one shared
  helper, so the canvas and the scrollbars cannot disagree about the lock.
- Leave `freePan` semantics untouched: Snap is an additional, stricter override
  that only fires when the whole scaled document fits.
- Hide the scrollbars exactly while they would offer no pan range, and restore
  them the moment the document overflows.

**Non-Goals:**
- Replacing or renaming Free Pan, or changing its default.
- A per-axis lock (Free Pan already covers the per-axis case).
- Hiding the bars for the per-axis free-pan constraint: only the workspace snap
  lock hides them.
- Changing the cxx-qt bridge or any engine crate.

## Decisions

**1. A `snapToWorkspace` parameter on the shared helper, applied after the
free-pan branch.** The helper computes `w = imageWidth * zoom` and
`h = imageHeight * zoom`. When `snapToWorkspace` is true and `w > 0 && h > 0 &&
w <= viewport.width() && h <= viewport.height()`, both axes collapse to
`(viewport - scaled) / 2`; otherwise the existing free-pan policy runs unchanged.
Document-level, not per-axis: the issue says panning and the bars return as soon
as the document exceeds the workspace, which is an all-or-nothing decision.
*Alternative:* reuse `freePan = false` for the snap lock — rejected: that would
make Snap a per-axis constraint that pins a fitting axis of a document that
overflows the other axis, contradicting "once the scaled document exceeds the
workspace, panning returns as usual".

**2. A per-canvas `snapToWorkspace_` flag with a derived locked query.**
`ImageView::setSnapToWorkspace(bool)` re-clamps, `update()`s, and emits
`viewChanged()` (the same contract as `setFreePan`), because `viewChanged` is not
connected to `update()` and the canvas pixels would otherwise stay stale.
`workspaceSnapLocked()` recomputes the fit from the current zoom, document size,
and widget size; it is the single source both the pan gate and the scrollbars
read, so they cannot disagree.

**3. Pan rejection at the press and at the drag.** While locked, `mousePressEvent`
does not begin a pan for the middle button or the pan-enabled left button, and
`spacePan_` still cannot move the canvas because `clampOffset()` pins the offset.
The clamp is the real guard; the press gate only avoids a hand cursor that could
not move anything.

**4. The scrollbars hide only under the lock.** `CanvasScrollBars::syncFromView`
shows the bars and their corner unless `view_->workspaceSnapLocked()`; then it
hides them. The bars stay a pure projection otherwise, and the offset remains the
single source of truth. Hiding a bar changes the canvas size through the grid
layout, which re-runs `clampOffset()`; because the freed space only makes the
document fit better, the locked state is stable.

**5. Menu placement beside Free Pan.** The checkable item is inserted directly
after `View > Free Pan`, grouping the two pan-behaviour toggles. Command id
`view.snapCanvasToWorkspace`; label `Snap Canvas To Workspace`.

**6. Persist with a false default.** `SessionState::snapCanvasToWorkspace =
false`, loaded with a `false` default and written on save; `addDocument` pushes
the window flag onto each new canvas, exactly as `freePan_` is applied today.

## Risks / Trade-offs

- **Overlap with Free Pan** → the two toggles are independent and both default to
  today's behaviour; Snap is the stricter, document-level lock and is documented
  as such. No existing Free Pan scenario changes.
- **Scrollbar hide/show churn on zoom** → the locked state flips exactly at the
  fit boundary; the layout settle re-runs the clamp and converges because hiding
  the bars only enlarges the viewport.
- **Zero-size document** → the lock is guarded on `w > 0 && h > 0`, matching the
  free-pan collapse guard.
- **`frame.cpp` size headroom** → the file is near its 1200-LOC cap; the
  `addDocument` change is a single line, alongside the existing `setFreePan` call.

## Migration Plan

Additive and default-off. A store written by an older build has no
`snapCanvasToWorkspace` key, so it loads `false` (today's behaviour) and is
rewritten with the key on the next save. No data migration; rollback is reverting
the commit.
