# Proposal

## Why

Issue #1198: the left side of the footer is garbled. The zoom readout, the
document size/resolution text, and the GPU readout are drawn on top of a
temporary status message. Two defects combine:

- `QStatusBar::hideOrShow()` does hide the normal (`addWidget`) readouts while
  a temporary message is shown, but `PicturaMainWindow::updateStatus()` calls
  `setVisible(true)` on them again on the next refresh. The open flow triggers
  exactly that: `showMessage(openNotice)` is followed by
  `activateDocWindow()` → `updateStatus()`. The readouts return under the
  message.
- `QStatusBar::paintEvent` paints the temporary message across the full footer
  (clamped only by the first permanent widget) while
  `NotificationStatusBar::paintEvent` paints the same message again on its
  yellow pill. Its `messageRect()` clamps to *any* visible child, including
  nested children of the readouts, so the pill shrinks or collapses while the
  base text still spans the bar.

## What Changes

- The footer readouts stay left-aligned but are hidden while a temporary status
  message is displayed, and are restored when the message clears. A readout
  refresh during a message (zoom change, document switch, transform change)
  cannot bring them back over it.
- `NotificationStatusBar` becomes the single message painter: it paints the
  themed status-bar panel and then the message once, on the pill, and no longer
  calls `QStatusBar::paintEvent`.
- `NotificationStatusBar::messageRect()` measures direct children only, so a
  nested child of a readout (for example the zoom field's line edit) can no
  longer shrink the banner.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/application-shell`: the capability gains a "Temporary status messages in
  the status bar" requirement (readouts hide while a message shows, a refresh
  cannot bring them back, and the banner is the only message painter) with two
  scenarios.

## Impact

- `crates/pictura-app/cpp/notification_status_bar.{h,cpp}` (public
  `messageRect()`, single-painter `paintEvent`, direct-children rect).
- `crates/pictura-app/cpp/frame_build.cpp` (connect `messageChanged` to
  `updateStatus`).
- `crates/pictura-app/cpp/frame.cpp` (`updateStatus` gates every readout and
  the two static group separators on the current temporary message).
- `crates/pictura-app/cpp/tests/tst_notification_status_bar.cpp` (two Qt tests
  on a real `PicturaMainWindow` footer).
- No engine, document, codec, dependency, or `docs/` change.
