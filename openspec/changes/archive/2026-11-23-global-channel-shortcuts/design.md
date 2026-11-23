# Design

## Problem

`ChannelsPanel` builds each row with CS6's `Ctrl+N` label but keeps it as a
disabled `QLabel`; no shortcut, action, or key handler uses it. `Ctrl+1` is
`View > 100%` and `Ctrl+2`…`Ctrl+5` are otherwise unbound, so the channel labels
promise a binding that does not exist.

## Fix

`PicturaMainWindow::dispatchCommandKey` already owns document-wide keys that
have no menu action (`Ctrl+\` for the mask channel and the `Ctrl+=` zoom
fallback). It receives keys from the canvas because an ignored `ImageView` key
event propagates to the frame, which is how `Ctrl+=` reaches the zoom handler in
tests. The channel keys join that path rather than adding four `QShortcut`
objects, so they share one dispatch point and do not fire from modal dialogs or
while a text field consumes the chord.

`ChannelsPanel::toggleChannelForShortcut(int digit)` finds the row whose
published shortcut ends in that digit and calls the existing `toggle(index)`
path. The row label stays the single source of truth for which key maps to which
row; the frame only passes the digit. `toggle()` already implements the eye
semantics: the composite row restores all channels, an individual row flips its
bit, and the last visible channel stays visible.

The frame accepts `Ctrl+2`…`Ctrl+5` with no Alt or Shift, and returns the key
unconsumed when no document is open or no row matches. Alpha rows (`Ctrl+6`…)
are outside this change; issue #508 owns the legacy/custom channel-shortcut
scheme.
