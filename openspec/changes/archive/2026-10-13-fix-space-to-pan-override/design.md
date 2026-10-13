# Design

## Context

See `proposal.md - Why`. The transient Space hand mode lives on `ImageView` as
`spacePan_`. `ImageView::setSpacePan` applies it once by calling `setCursor` and
`setPanEnabled(true)`; nothing re-asserts it afterwards, while the tool
controller keeps reapplying tool cursors and pan policy on every pointer move
and tool-state refresh:

- `ToolController::refreshCursor` (`tools_marquee.cpp:37`) is the funnel for
  tool cursors and is reached on every move via
  `handleMoved -> updateSelectionHover` (`tools_selection_move.cpp:87`), plus
  out-of-band refreshes (`applyToolPolicy`, the region-blit path at
  `frame.cpp:440`, modifier edges, `bindCanvas`).
- Hover handlers set the cursor directly, bypassing the funnel:
  `tools_guides.cpp:106`, `tool_slice.cpp:273`, `tool_perspectivecrop.cpp:356`.
- `applyToolPolicy` (`tools.cpp:680`) recomputes
  `setPanEnabled(!session && active_ == Hand)`, so any tool-state refresh during
  the hold disables panning and the press falls through to the tool
  (`image_view.cpp:814`).

The press is only the moment the override is consulted, so an update that lands
between the keydown and the press drops the pan. Key and mouse arrive on separate
Qt queues, so a press that precedes the keydown is owned by the tool and never
pans.

Independently, the frame's Space *press* is guarded with `!isAutoRepeat()`
(`frame.cpp:1049`) but its *release* is not (`frame.cpp:1115`). On X11 a held key
emits synthetic release/press pairs, so the release runs `setSpacePan(false)` and
the follow-up press is ignored by the press guard — the override is cleared for
the rest of the hold. No cursor guard can fix this, because `spacePan_` is
already false when the next cursor update runs. `LiquifyCanvas` guards both
edges (`liquify_canvas.cpp:426,437`).

## Goals / Non-Goals

**Goals:**

- The Space override is authoritative for the whole hold: hand cursor and
  pan-enabled state survive every cursor/pan update.
- Releasing Space restores the previous tool's cursor and pan-enable state.
- No flicker on hover; no dropped pan caused by a mid-hold update.

**Non-Goals:**

- Converting an already-running tool drag into a pan (choice A). A drag in
  progress when Space is pressed keeps running; this change does not abandon it.
- Blocking Space during a live type/text session beyond what the frame already
  does for transform sessions (`frame.cpp:1051`).
- Any Rust, codec, or document-state change.

## Decisions

### The override is consulted at apply time, not stored as a value to overwrite

Chosen: `spacePan_` is checked at the top of each path that applies a tool
cursor or pan policy, and that path returns the override instead of the tool
state. This is the issue's stated shape ("a single top-level condition checked
before cursor-mode updates are applied").

Alternative considered: store a separate "tool cursor" alongside an "effective
cursor" and recompute on each change. Rejected — `QWidget::setCursor` is not
virtual and tool/hover handlers call it directly in several places, so a
shadow value would still need every direct caller rerouted, a larger diff for
no behavioural gain.

Concretely:

1. `ToolController::refreshCursor` — immediately after the `if (!canvas_) return;`
   (`tools_marquee.cpp:41`) and *before* the `if (canvasSampler_)` early return
   (line 42), if `canvas_->spacePanForTest()` set the hand cursor (closed while a
   pan is in progress, open otherwise) and return. Placing it after the sampler
   check would let the Eyedropper/sampler path keep clobbering the hand cursor.
2. `ImageView::setPanEnabled` — ignore a disable while `spacePan_` is set, so
   `applyToolPolicy` cannot turn panning off mid-hold.
3. `ImageView::mousePressEvent` — let `spacePan_` bypass the `panEnabled_` gate,
   so the press pans even if some update raced ahead.
4. `ImageView::mouseMoveEvent` — while `spacePan_` is set and no mouse button is
   down, do not emit `mouseMoved`. This suppresses hover-driven tool updates and
   the direct hover `setCursor` calls, killing the flicker without freezing a
   button-held tool drag (choice A). `updateGuideHover`'s `SplitH`/`SplitV` is
   reached only through `handleMoved` (`tools.cpp:803`), i.e. through this same
   suppressed signal, so the no-button hover case is covered; only a button-held
   drag can still show a split cursor.
5. `PicturaMainWindow::keyReleaseEvent` — guard the Space case with
   `!event->isAutoRepeat()` (`frame.cpp:1115`), matching the press handler, so an
   X11 auto-repeat release cannot end the override mid-hold.
6. `ImageView::mouseReleaseEvent` — on a middle-button release while `spacePan_`
   is set, re-assert `Qt::OpenHandCursor` instead of `unsetCursor()`
   (`image_view.cpp:869`), matching the left-button path at line 865.
7. `PicturaMainWindow` focus-change handler (`frame_build.cpp`) — if the canvas
   loses focus or the window deactivates while `spacePan_` is set, end the
   override and restore the tool. This closes the hole the `setPanEnabled` guard
   would otherwise open: a missed Space key-up used to be harmless (the tool
   refresh disabled panning), but the guard keeps panning enabled, so a stuck
   override would route every later press to pan.
8. `ToolController::beginTransformImpl` (`tool_transform.cpp:68`) — end any
   transient Space pan before `setPanEnabled(false)`, so the modal transform
   session's pan-disable is not swallowed by the override guard.
9. `ImageView::setSpacePan(true)` — show `ClosedHandCursor` if a pan is already
   in progress (a middle-button drag) rather than always the open hand.

### Cursor state stays owned by `ImageView`

The guard needs the hand shape and whether a pan is in progress, both private to
`ImageView`. Add a small accessor (`isPanning()`) or a `reassertSpacePanCursor()`
helper rather than exposing `panning_` broadly. `spacePanForTest()` is already
used in production (`frame.cpp:1116`, `tool_hand_zoom.cpp:55`); a cleaner
`spacePan()` accessor is optional cleanup, not required by this change.

## Risks / Trade-offs

- [Hover tool previews freeze while Space is held] → Intended: Space owns the
  pointer; the hand cursor is the only affordance and brush rings are already
  hidden while panning (`image_view.cpp:643`).
- [A press that arrives before the Space keydown is still owned by the tool] →
  Accepted; same race exists in CS6 and fixing it needs event reordering, which
  is out of scope.
- [X11 key auto-repeat emits a synthetic release that clears the override] →
  Guard the Space release with `!isAutoRepeat()`; cover it with a test that
  sends a release with `autorep = true` and asserts the override survives.
- [A button-held drag near a guide can still show `SplitH`/`SplitV` mid-hold] →
  Accepted; the no-button hover case is covered by the move suppression, and the
  next guard-driven refresh restores the hand.
- [A missed Space key-up leaves the override stuck, now that the pan guard keeps
  panning enabled] → End the override on canvas focus loss / window deactivate
  (decision 7).
- [A transform session starts while Space is held and its pan-disable is
  swallowed by the guard] → End the override at transform start (decision 8).
- [New guard branches in hot paths (`mouseMoveEvent`, `refreshCursor`)] → They
  are single boolean checks on an existing member, cheaper than the cursor
  churn they replace.
