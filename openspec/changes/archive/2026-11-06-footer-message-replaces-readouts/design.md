# Design

## Context

See `proposal.md - Why`. The footer is a `NotificationStatusBar` (a
`QStatusBar` subclass) built in `PicturaMainWindow::buildStatusBar()`. Every
readout — zoom, resolution/size, view-options, GPU, action, and the tool-hint
bar — is added with `QStatusBar::addWidget` (the normal, left area); only the
Help ▸ Search hint is permanent (right area). Temporary messages arrive through
`QStatusBar::showMessage()` from about twenty call sites.

Qt 6.11's `QStatusBar::hideOrShow()` hides the normal widgets when a message
starts, but any later `QWidget::setVisible(true)` on one of them shows it again.
`updateStatus()` is that later call. Qt's own `QStatusBar::paintEvent` then
paints the message text across `QStatusBarPrivate::messageRect()`, which is
clamped only by the first *permanent* widget, so the text lands on the readouts.

The subclass's `messageRect()` clamps to every visible child found recursively,
so a readout's internal line edit (whose `x` is relative to the readout, not the
bar) collapses the pill to a zero-width rect. The subclass also calls the base
`paintEvent`, which paints the message text a second time.

## Goals / Non-Goals

**Goals:**

- A temporary message replaces the footer readouts instead of overprinting them,
  including when a readout refresh happens while the message is shown.
- The banner is the single message painter: themed panel, then the pill and its
  text once.
- The banner rect is measured against the bar's direct children, so nested
  widgets cannot shrink it.

**Non-Goals:**

- Moving the readouts to the permanent (right) area; the CS6-style footer keeps
  the readouts left-aligned.
- Changing the pill colours, the message timeouts, or any `showMessage` call
  site.
- Changing the readout text formats (including the known 72-ppi placeholder).

## Decisions

### Hide the readouts while a message is shown

Chosen: `updateStatus()` computes `showReadouts = hasDoc && currentMessage()
.isEmpty()` and uses it for the zoom, size, backend, view-options, action, and
hint visibility, including the two static group separators (they are normal
widgets too). `buildStatusBar()` connects `QStatusBar::messageChanged` to
`updateStatus()`, so clearing the message restores the readouts and showing one
hides them even if `updateStatus()` runs later. The static rules are reached by
their `statusSeparator` object name, the same way `applyStatusBarScale` finds
them, so `frame.h` stays at its 1200-line cap.

Qt's `hideOrShow()` already hides normal widgets; the gate is needed because
`updateStatus()` would otherwise re-show them. Because the gate also hides them
explicitly, Qt's `hideOrShow()` does not re-show them on `clearMessage()`; the
`messageChanged` connection does, after the message is gone. The explicit hide
also matters for the queued `_q_showIfNotHidden` call Qt posts to a child: a
widget hidden without `WA_WState_ExplicitShowHide` (as `hideOrShow()` leaves it)
is shown again when that queued call lands, which is how the two separators
survived the first cut of this fix.

Alternative considered: `addPermanentWidget` for every readout. Rejected: it
satisfies the geometry check by moving zoom/size/GPU to the right of the stretch,
next to Search, which abandons the documented left-aligned footer. Hiding keeps
the layout and matches CS6, where a status message replaces the readouts.

### Paint the message once, in the subclass

Chosen: `NotificationStatusBar::paintEvent` no longer calls
`QStatusBar::paintEvent`. It draws `PE_PanelStatusBar` with the widget's style
option (the theme's `QStatusBar { background: ... }` rule is drawn there) and
then the pill and text. The base painter's per-item `PE_FrameStatusBarItem`
frames are skipped; the theme's `QStatusBar::item { border: 0; }` makes them
invisible anyway.

Alternative considered: keep the base painter and clear the message inside
`paintEvent` so the base paints nothing. Rejected: clearing and restoring
`showMessage` inside a paint would restart the timeout timer and emit signals.

### Measure the banner against direct children

Chosen: `messageRect()` iterates
`findChildren<QWidget*>(Qt::FindDirectChildrenOnly)`. The readouts are direct
children of the bar, so their `x` is in bar coordinates. While a message shows,
they are hidden, leaving the permanent Search hint as the right clamp, exactly
like Qt's own message rect.

### Tests

- `tst_notification_status_bar::messageReplacesReadouts` builds a real
  `PicturaMainWindow`, opens a document, shows a message, and asserts every
  readout is hidden and that the banner rect intersects no visible direct child
  (the permanent Search hint included); clearing the message brings the
  readouts back.
- `tst_notification_status_bar::messageSurvivesAStatusRefresh` shows a message
  and then changes the canvas zoom, which calls `updateStatus()`; the readouts
  stay hidden and the message is unchanged.
- The existing `temporaryMessageHasYellowBackground` still pins the pill.
- Hidden widgets keep their last geometry in Qt, so the intersection assertion
  is made against the visible children; the hidden readouts are asserted
  separately, which is the stronger statement (they are not drawn at all).

## Risks / Trade-offs

- [A readout is hidden while a message shows, so its value is momentarily not
  readable] -> Intended: the message owns the footer, and the value returns when
  the message clears (messages are transient, 4-10 s).
- [Skipping the base paint drops per-item frames if a future theme gives
  `QStatusBar::item` a border] -> The theme sets it to 0; a themed border would
  need to move into the subclass. Noted in the code comment.
- [A nested child of the permanent Search hint could still clamp the banner] ->
  `FindDirectChildrenOnly` excludes it; the Search hint itself is the direct
  child and is the intended clamp.
