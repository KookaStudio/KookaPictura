# Proposal: initial-view-zoom-readout

## Why

Issue #303 (with #324 as its new-document face). Two symptoms. First, a new or
opened document lands top-left at a tiny zoom (8.75 % in the report): the
automatic initial fit runs when the document is attached, but a canvas that has
never been shown gets its laid-out geometry silently — Qt delivers no resize
event for it — so the fit is computed against the pre-layout grid minimum
(320x240) and then kept: the fit marks the view user-adjusted, so the later
real-size resize only clamps the offset and never re-fits. Second, the
status-bar zoom field is fed the raw percent, so a fractional zoom renders as
e.g. "52.3438" — wider than the value box reserves for its integer range, so
the readout clips — and it disagrees with the integer percent the file-tab
title and the Navigator readout show.

## What Changes

- `ImageView` gains a pending-settle flag: `setImage`/`setDocument` set it, and
  while it is pending every resize re-fits, the show pass re-fits at the real
  geometry (once synchronously, once queued for multi-pass layouts), and the
  first user pan/zoom/100 % choice ends the settle early so an explicit view is
  never snapped back.
- `updateStatus` feeds the status-bar zoom field the integer percent (`qRound`),
  the same format the tab title uses, so the readouts agree and every displayed
  value fits the width the field reserves for its integer range.
- The zoom/pan view-state family moves from `image_view.cpp` to
  `image_view_document.cpp` (pure moves) so the translation unit stays under
  the file-size cap.

## Capabilities

### Modified Capabilities

- `ui/document-canvas`: an ADDED requirement pins the initial document view —
  centred at 100 % when the document fits the viewport, fit-to-screen and
  centred when it does not — and that an explicit user view survives resizes.

## Impact

`crates/pictura-app/cpp/image_view*.cpp/.h` and `frame.cpp`'s status sync only.
No bridge, document, or engine change; scrollbars and the status-bar zoom field
follow the existing signals. The status-bar readout change conforms to the
existing `ui/application-shell` magnification requirement ("the same value in
the same format"); no requirement text changes.

