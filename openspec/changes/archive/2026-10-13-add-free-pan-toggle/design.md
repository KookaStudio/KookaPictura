# Design

## Context

See `proposal.md` for motivation. The current pan clamp lives in one helper,
`offsetRangeFor(imageSize, zoom, viewport, margin)` in `canvas_range.h`. For each
axis it keeps at least one display inch (`kCanvasRevealMarginPx = 96`) visible;
when the scaled axis is smaller than that margin, `visibleX = min(margin, w)`
instead keeps the whole extent visible, so a small axis is allowed to slide from
`0` to `viewport - w` (a corner, not the centre). It also has a second,
independent centring branch when the computed `min` exceeds `max`. It is
consumed by two independent callers:

- `ImageView::clampOffset()` (`image_view_document.cpp`), reached by the pan,
  zoom, centre, fit, actual-pixels, initial-view, and resize paths. Note that the
  document-size replacement paths (`replaceDocument`, `replaceImage`) set
  `docSize_` directly and do **not** clamp today — this change must close that
  gap or the constraint is not applied after an Image/Canvas Size or crop.
- `CanvasScrollBars` (`canvas_scrollbars.cpp`), which recomputes the range for
  its thumb in `syncFromView` and in the two `valueChanged` handlers.

The View menu is a registry-backed command tree in `command_tree.cpp`; checkable
items are wired in `frame_menus.cpp` with a handler plus a
`setCheckedProvider`, and window-level toggles (`rulersVisible_`, `guidesVisible_`)
follow a "flip a bool, re-apply, `saveSession()`" pattern (`frame_guides.cpp`).
`SessionState` (`session.h/.cpp`) is an opaque JSON store with per-field
defaults, so unknown keys and older stores round-trip.

## Goals / Non-Goals

**Goals:**
- The clamp policy for "does the document fit this axis" is expressed in the one
  shared helper, so the canvas and the scrollbars cannot disagree.
- With Free Pan on (default) the shared helper returns the same values as
  before; the only deliberate call-site change is the new clamp on the
  document-size replacement paths.
- The constrained policy applies uniformly to every mutation point and to newly
  opened documents without a per-call-site special case.

**Non-Goals:**
- Adding an overscroll/pasteboard mode, a third pan state, or a configurable
  reveal margin.
- Changing the cxx-qt bridge or any engine crate: this is app-side view state.
- Making Fit On Screen ignore the constraint (Fit always centres; that already
  satisfies the constrained policy).

## Decisions

**1. Add a `freePan` parameter to `offsetRangeFor`, defaulting to `true`.**
The clamp point is shared, so the policy belongs on the shared seam. When
`freePan` is false, for each axis: if `scaled <= viewport` the range collapses to
`(viewport - scaled) / 2` (min == max, centred); otherwise the reveal-margin
values are unchanged. This deliberately supersedes both existing small-axis
behaviours (the `min(margin, w)` whole-extent slide and the `min > max`
inversion branch) for the constrained case. The collapse SHALL be guarded on a
non-empty document (`scaled > 0`), because the scrollbars call `offsetRangeFor`
with a zero size before the canvas has a document, and collapsing that to
`viewport / 2` would yield a negative bar value. Per-axis means a wide-but-short
image is centred horizontally while still panning vertically, which matches
Photoshop and needs no extra branch.
*Alternatives:* guard only `ImageView::clampOffset` — rejected: the scrollbar
recomputes the constrained range independently, so a free-panned offset lands
outside the thumb range and the next `valueChanged` clamps the image straight
back, making the toggle look dead. A per-axis transform in the view only —
rejected for the same split-brain reason.

**2. Positive, checked-by-default label "Free Pan".**
A checkable View item reads "checked = the named thing is active". The required
default is free pan, so the positive noun keeps the checkbox honest and matches
the terse siblings (`Rulers`, `Snap`, `Extras`). The inverse family
(`Constrain Pan`, `Pin When Fitted`) was considered; it is coherent but ships
unchecked and names behaviour that is invisible while the image fits, which
invites a "broken toggle" report. Command id `view.freePan`.
*Alternative:* `Pan Anywhere` — rejected: it overpromises, since the reveal
margin still limits panning at high zoom.

**3. Window-level flag mirrored onto each canvas.**
`PicturaMainWindow::freePan_` is the durable toggle (loaded in `loadGuideSession`
alongside the other view toggles, saved in `saveSession`), and `addDocument`
pushes it onto each new canvas's `ImageView::setFreePan`, exactly as
`gpuCompute_`/`colorPolicy_` are applied today. `setFreePan` runs
`clampOffset()`, then `update()`, then emits `viewChanged`: the `update()` is
required because `viewChanged` only reaches the scrollbars and the adjustment
preview (it is not connected to `ImageView::update()`), so without it the canvas
pixels stay stale until an incidental expose. The document-size replacement paths
gain a clamp: `replaceDocument` clamps after it sets the new size, and
`replaceImage` clamps only for a non-empty image, because `replaceDocument` calls
`replaceImage(QImage())` first and clamping that empty intermediate would reset
the offset to zero and lose the view (a regression `tst_transform_overlay`
surfaced indirectly).
*Alternative:* per-document persistence — deferred; the issue asks for a
session-persisted menu toggle, not per-tab state.

**4. Menu placement after the zoom group.**
The item is inserted after `Print Size` / before the separator that opens the
Screen Mode group, grouping it with viewport navigation.

## Risks / Trade-offs

- **Scrollbar divergence** → thread the flag through all three
  `offsetRangeFor` call sites in `canvas_scrollbars.cpp`, not just the view; the
  delta spec makes the bars' projection a requirement so a regression is caught.
- **Over-clamping an adjusted view on resize** → the new collapse only fires when
  Free Pan is off and the axis fits; the existing resize-keeps-centre path and
  its test run under the default (on), unchanged.
- **Invisible constraint while the image fits** → the toggle is a deliberate,
  discoverable menu state; a status/tool-hint cue is out of scope and can be
  added later without changing the specs.
- **Polarity confusion from `panEnabled_`** → that existing flag means "may a
  press start a pan" (tool/transform gating) and is unrelated; the new field is
  named `freePan_` to avoid overloading it.
- **Zero-size document** → the scrollbars call `offsetRangeFor` with a zero image
  size before `setDocument`; the collapse is guarded on `scaled > 0` and covered
  by a test case.
- **`frame.cpp` size headroom** → the file is near its 1200-LOC cap; keep the
  `addDocument` change minimal and split rather than allowlist if it crosses.

## Migration Plan

Additive and default-on. `SessionState::freePan = true` is the field default, so
a store written by an older build loads the current free-pan behaviour and is
rewritten with the new key on the next save. No rollback beyond reverting the
commit; no data migration.
